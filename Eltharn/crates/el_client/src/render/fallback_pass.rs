use super::*;
use crate::render::armor_pass::{
    AvatarArmorDraw, AvatarItemDraw, AvatarMeshDraw, AvatarPassDraws, AvatarPassPipelines,
    encode_avatar_layers,
};
impl ClientRenderer {
    pub(in crate::client::state) fn encode_fallback_entities(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        fallback_draws: Vec<FallbackDrawCommand>,
        query_started: bool,
        counters: Option<&mut crate::render::profiler::RenderEncodeScopeCounters>,
    ) {
        if self.missing_entity_texture.is_none() {
            self.missing_entity_texture = Some(item_frame::missing_texture_binding(
                device,
                queue,
                &self.texture_bind_group_layout,
            ));
        }
        if self.item_frame_pipeline.is_none()
            && fallback_draws
                .iter()
                .any(|draw| matches!(draw, FallbackDrawCommand::ItemFrame { .. }))
        {
            self.item_frame_pipeline = Some(item_frame::frame_pipeline(
                device,
                &self.camera_bind_group_layout,
                &self.texture_bind_group_layout,
                self.scene_texture.format(),
                false,
            ));
        }
        if self.frame_item_pipeline.is_none()
            && fallback_draws.iter().any(|draw| {
                matches!(
                    draw,
                    FallbackDrawCommand::ItemFrame { .. } | FallbackDrawCommand::DroppedItem { .. }
                )
            })
        {
            self.frame_item_pipeline = Some(item_frame::frame_pipeline(
                device,
                &self.camera_bind_group_layout,
                &self.texture_bind_group_layout,
                self.scene_texture.format(),
                true,
            ));
        }
        if self.frame_head_pipeline.is_none()
            && fallback_draws.iter().any(|draw| {
                matches!(
                    draw,
                    FallbackDrawCommand::ItemFrame {
                        item: Some((_, _, _, Some(_))),
                        ..
                    }
                )
            })
        {
            self.frame_head_pipeline = Some(item_frame::head_pipeline(
                device,
                &self.camera_bind_group_layout,
                &self.texture_bind_group_layout,
                self.scene_texture.format(),
            ));
        }
        let atlas = self.block_atlas.as_ref().expect("prepared atlas");
        let scene_view = &self.scene_texture_view;
        self.fallback_entity_camera_arena.upload(queue);
        self.fallback_entity_mesh_arena.upload(device, queue);
        let fallback_entity_arena_stats = self.fallback_entity_mesh_arena.stats();
        debug_assert_eq!(
            fallback_entity_arena_stats.queue_write_count(),
            if fallback_entity_arena_stats.append_count() > 0 {
                2
            } else {
                0
            }
        );
        let fallback_entity_mesh_buffers = self.fallback_entity_mesh_arena.uploaded();
        if query_started && let Some(gpu_timer) = self.gpu_timer.as_mut() {
            gpu_timer.begin_pass(encoder, GpuTimedPass::FallbackEntities);
        }
        let mut fallback_entities_pass = CountedRenderPass::new(
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Fallback Entities Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: scene_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            }),
            counters,
        );
        for command in fallback_draws {
            match command {
                FallbackDrawCommand::ItemFrame { frame, item } => {
                    fallback_entities_pass.set_bind_group(1, &atlas.bind_group, &[]);
                    if let Some((mesh, camera)) = frame {
                        let binding = fallback_entity_mesh_buffers
                            .expect("frame mesh uploaded")
                            .resolve(mesh);
                        item_frame::draw_mesh(
                            &mut fallback_entities_pass,
                            binding,
                            self.item_frame_pipeline
                                .as_ref()
                                .expect("prepared frame pipeline"),
                            &atlas.bind_group,
                            self.fallback_entity_camera_arena.bind_group(camera),
                        );
                    }
                    if let Some((mesh, camera, foil, texture)) = item {
                        let binding = fallback_entity_mesh_buffers
                            .expect("frame item mesh uploaded")
                            .resolve(mesh);
                        let index_count = binding.index_count();
                        let item_texture = texture
                            .as_ref()
                            .map(|key| {
                                self.remote_entity_textures.get(key).unwrap_or_else(|| {
                                    self.missing_entity_texture
                                        .as_ref()
                                        .expect("missing image prepared")
                                })
                            })
                            .unwrap_or(&atlas.bind_group);
                        item_frame::draw_mesh(
                            &mut fallback_entities_pass,
                            binding,
                            if texture.is_some() {
                                self.frame_head_pipeline
                                    .as_ref()
                                    .expect("prepared world head pipeline")
                            } else {
                                self.frame_item_pipeline
                                    .as_ref()
                                    .expect("prepared FIXED item pipeline")
                            },
                            item_texture,
                            self.fallback_entity_camera_arena.bind_group(camera),
                        );
                        if foil {
                            fallback_entities_pass.set_pipeline(&self.item_glint_pipeline);
                            fallback_entities_pass.set_bind_group(
                                2,
                                &self.item_glint_bind_group,
                                &[],
                            );
                            fallback_entities_pass.set_bind_group(
                                3,
                                &self.item_glint_uniform_bind_group,
                                &[],
                            );
                            fallback_entities_pass.draw_indexed(0..index_count, 0, 0..1);
                        }
                    }
                }
                FallbackDrawCommand::DroppedItem {
                    mesh_key,
                    camera_bind_group_indices,
                    has_foil,
                } => {
                    let Some(mesh) = self.dropped_item_meshes.get(&mesh_key) else {
                        continue;
                    };
                    let (bg_indices, bg_count) = camera_bind_group_indices;
                    for &bg_idx in &bg_indices[..bg_count] {
                        dropped_item::draw(
                            &mut fallback_entities_pass,
                            &mesh.vertex_buffer,
                            &mesh.index_buffer,
                            mesh.num_indices,
                            has_foil,
                            dropped_item::ItemDrawBindings {
                                base_pipeline: self
                                    .frame_item_pipeline
                                    .as_ref()
                                    .expect("prepared item material pipeline"),
                                foil_pipeline: &self.item_glint_pipeline,
                                atlas: &atlas.bind_group,
                                camera: self.fallback_entity_camera_arena.bind_group(bg_idx),
                                foil_texture: &self.item_glint_bind_group,
                                foil_uniform: &self.item_glint_uniform_bind_group,
                            },
                        );
                    }
                }
                FallbackDrawCommand::RemotePlayer {
                    entity_id,
                    camera_bind_group_index,
                    texture,
                    body_mesh,
                    layers_visible,
                    armor_meshes,
                    held_item_cameras,
                } => {
                    let Some(mesh) = self.remote_player_model_meshes.get(&entity_id) else {
                        continue;
                    };
                    let player_texture = texture.and_then(|texture| match texture {
                        FallbackPlayerTextureSource::Remote(key) => {
                            self.remote_entity_textures.get(key.as_ref())
                        }
                        FallbackPlayerTextureSource::Local => self.player_texture.as_ref(),
                    });
                    let player_camera_bind_group = self
                        .fallback_entity_camera_arena
                        .bind_group(camera_bind_group_index);
                    let armor_slot_slices =
                        std::array::from_fn(|index| armor_meshes[index].first().copied());
                    debug_assert!(
                        fallback_entity_mesh_buffers.is_some()
                            || !remote_player_needs_shared_frame_buffers(
                                body_mesh,
                                armor_slot_slices
                            ),
                        "remote player frame slices require uploaded shared buffers"
                    );
                    let frame_mesh_buffers = fallback_entity_mesh_buffers;
                    let armor_textures_available = std::array::from_fn(|index| {
                        mesh.armor[index].as_ref().is_some_and(|armor| {
                            armor.layers.iter().any(|layer| {
                                self.remote_entity_textures.contains_key(&layer.texture_key)
                            })
                        })
                    });
                    let frame_plan = plan_remote_player_frame(
                        body_mesh,
                        player_texture.is_some(),
                        armor_slot_slices,
                        armor_textures_available,
                        frame_mesh_buffers.is_some(),
                        false,
                        false,
                        false,
                    );
                    let body_draw = match (body_mesh, player_texture, frame_mesh_buffers) {
                        (Some(body_mesh), Some(player_texture), Some(frame_mesh_buffers)) => {
                            let body_binding = frame_mesh_buffers.resolve(body_mesh);
                            Some(AvatarMeshDraw {
                                vertex_buffer: body_binding.vertex_buffer(),
                                vertex_range: body_binding.vertex_range(),
                                index_buffer: body_binding.index_buffer(),
                                index_range: body_binding.index_range(),
                                index_count: body_binding.index_count(),
                                camera_bind_group: player_camera_bind_group,
                                texture_bind_group: player_texture,
                            })
                        }
                        _ => None,
                    };
                    debug_assert_eq!(
                        body_draw.is_some(),
                        frame_plan.body,
                        "body draw must follow the shared-frame plan"
                    );
                    let mut armor_draws: [Vec<AvatarArmorDraw>; 4] =
                        std::array::from_fn(|_| Vec::new());
                    for (index, (armor, frame_meshes)) in
                        mesh.armor.iter().zip(armor_meshes).enumerate()
                    {
                        if !frame_plan.armor[index] {
                            continue;
                        }
                        let Some(armor) = armor.as_ref() else {
                            continue;
                        };
                        let Some(frame_mesh_buffers) = frame_mesh_buffers else {
                            continue;
                        };
                        for (layer, frame_mesh) in armor.layers.iter().zip(frame_meshes) {
                            let Some(armor_texture) =
                                self.remote_entity_textures.get(&layer.texture_key)
                            else {
                                continue;
                            };
                            let binding = frame_mesh_buffers.resolve(frame_mesh);
                            armor_draws[index].push(AvatarArmorDraw {
                                mesh: AvatarMeshDraw {
                                    vertex_buffer: binding.vertex_buffer(),
                                    vertex_range: binding.vertex_range(),
                                    index_buffer: binding.index_buffer(),
                                    index_range: binding.index_range(),
                                    index_count: binding.index_count(),
                                    camera_bind_group: player_camera_bind_group,
                                    texture_bind_group: armor_texture,
                                },
                                has_foil: layer.has_foil,
                                decal: layer.decal,
                            });
                        }
                    }
                    let mut main_item_draw: Option<AvatarItemDraw> = None;
                    let mut off_item_draw: Option<AvatarItemDraw> = None;
                    for &(hand, item_bg_idx) in held_item_cameras
                        .iter()
                        .flatten()
                        .filter(|_| layers_visible)
                    {
                        let (item_mesh, hand_slot) = match hand {
                            crate::render::model::player::player_model::HandSide::Right => {
                                (mesh.main_hand_item.as_ref(), &mut main_item_draw)
                            }
                            crate::render::model::player::player_model::HandSide::Left => {
                                (mesh.off_hand_item.as_ref(), &mut off_item_draw)
                            }
                        };
                        let Some(item_mesh) = item_mesh else {
                            continue;
                        };
                        let custom_head_texture = item_mesh
                            .texture_key
                            .as_deref()
                            .and_then(|key| self.remote_entity_textures.get(key))
                            .is_some();
                        let texture_bind_group = item_mesh
                            .texture_key
                            .as_deref()
                            .and_then(|key| self.remote_entity_textures.get(key))
                            .unwrap_or(&atlas.bind_group);
                        *hand_slot = Some(AvatarItemDraw {
                            mesh: AvatarMeshDraw::whole_buffers(
                                &item_mesh.vertex_buffer,
                                &item_mesh.index_buffer,
                                item_mesh.num_indices,
                                self.fallback_entity_camera_arena.bind_group(item_bg_idx),
                                texture_bind_group,
                            ),
                            has_foil: item_has_foil(&item_mesh.stack),
                            use_preview_pipeline: custom_head_texture,
                        });
                    }
                    let cape_draw = if layers_visible && mesh.cape_num_indices > 0 {
                        match (
                            mesh.cape_vertex_buffer.as_ref(),
                            mesh.cape_index_buffer.as_ref(),
                            mesh.cape_texture_key
                                .as_deref()
                                .and_then(|key| self.remote_entity_textures.get(key)),
                        ) {
                            (Some(cape_v_buf), Some(cape_i_buf), Some(cape_tex)) => {
                                Some(AvatarMeshDraw::whole_buffers(
                                    cape_v_buf,
                                    cape_i_buf,
                                    mesh.cape_num_indices,
                                    player_camera_bind_group,
                                    cape_tex,
                                ))
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    let pipelines = AvatarPassPipelines {
                        body_pipeline: &self.inventory_preview_pipeline,
                        armor_pipeline: &self.armor_pipeline,
                        armor_decal_pipeline: &self.armor_decal_pipeline,
                        item_pipeline: &self.item_pipeline,
                        item_preview_pipeline: &self.inventory_preview_pipeline,
                        cape_pipeline: &self.cape_pipeline,
                        glint_pipeline: &self.item_glint_pipeline,
                        armor_glint_bind_group: &self.armor_glint_bind_group,
                        armor_glint_uniform_bind_group: &self.armor_glint_uniform_bind_group,
                        item_glint_bind_group: &self.item_glint_bind_group,
                        item_glint_uniform_bind_group: &self.item_glint_uniform_bind_group,
                    };
                    let draws = AvatarPassDraws {
                        body: body_draw,
                        armor: armor_draws,
                        main_item: main_item_draw,
                        off_item: off_item_draw,
                        cape: cape_draw,
                    };
                    encode_avatar_layers(&mut fallback_entities_pass, &pipelines, &draws);
                }
                FallbackDrawCommand::RemoteEntity {
                    camera_bind_group_index,
                    texture_path,
                    use_entity_cutout_pipeline,
                    dynamic_mesh,
                    wool_mesh,
                    armor_meshes,
                } => {
                    let camera_bind_group = self
                        .fallback_entity_camera_arena
                        .bind_group(camera_bind_group_index);
                    {
                        let entity_tex = self
                            .remote_entity_textures
                            .get(texture_path)
                            .unwrap_or_else(|| {
                                self.missing_entity_texture
                                    .as_ref()
                                    .expect("missing image prepared")
                            });
                        fallback_entities_pass.set_pipeline(if use_entity_cutout_pipeline {
                            &self.dragon_fireball_pipeline
                        } else {
                            &self.entity_direct_cutout_cull_pipeline
                        });
                        fallback_entities_pass.set_bind_group(0, camera_bind_group, &[]);
                        fallback_entities_pass.set_bind_group(1, entity_tex, &[]);
                        if let Some(frame_mesh) = dynamic_mesh {
                            fallback_entities_pass.set_pipeline(if use_entity_cutout_pipeline {
                                &self.dragon_fireball_pipeline
                            } else {
                                &self.entity_direct_cutout_cull_pipeline
                            });
                            fallback_entities_pass.set_bind_group(0, camera_bind_group, &[]);
                            fallback_entities_pass.set_bind_group(1, entity_tex, &[]);
                            let binding = fallback_entity_mesh_buffers
                                .expect("fallback entity frame mesh must be uploaded")
                                .resolve(frame_mesh);
                            fallback_entities_pass.set_vertex_buffer(
                                0,
                                binding.vertex_buffer().slice(binding.vertex_range()),
                            );
                            fallback_entities_pass.set_index_buffer(
                                binding.index_buffer().slice(binding.index_range()),
                                wgpu::IndexFormat::Uint32,
                            );
                            fallback_entities_pass.draw_indexed(0..binding.index_count(), 0, 0..1);
                        } else if dynamic_mesh.is_none()
                            && wool_mesh.is_none()
                            && armor_meshes.iter().all(Vec::is_empty)
                        {
                            let (Some(v_buf), Some(i_buf)) = (
                                &self.remote_entity_vertex_buffer,
                                &self.remote_entity_index_buffer,
                            ) else {
                                continue;
                            };
                            fallback_entities_pass.set_vertex_buffer(0, v_buf.slice(..));
                            fallback_entities_pass
                                .set_index_buffer(i_buf.slice(..), wgpu::IndexFormat::Uint32);
                            fallback_entities_pass.draw_indexed(
                                0..self.remote_entity_num_indices,
                                0,
                                0..1,
                            );
                        }
                    }
                    for slot in crate::render::model_frame::WORLD_ARMOR_DRAW_ORDER {
                        for (texture_key, frame_mesh, has_foil, decal) in &armor_meshes[slot] {
                            if let Some(armor_texture) =
                                self.remote_entity_textures.get(texture_key)
                            {
                                fallback_entities_pass.set_pipeline(if *decal {
                                    &self.armor_decal_pipeline
                                } else {
                                    &self.armor_pipeline
                                });
                                fallback_entities_pass.set_bind_group(0, camera_bind_group, &[]);
                                fallback_entities_pass.set_bind_group(1, armor_texture, &[]);
                                let binding = fallback_entity_mesh_buffers
                                    .expect("fallback entity armor frame mesh must be uploaded")
                                    .resolve(*frame_mesh);
                                fallback_entities_pass.set_vertex_buffer(
                                    0,
                                    binding.vertex_buffer().slice(binding.vertex_range()),
                                );
                                fallback_entities_pass.set_index_buffer(
                                    binding.index_buffer().slice(binding.index_range()),
                                    wgpu::IndexFormat::Uint32,
                                );
                                fallback_entities_pass.draw_indexed(
                                    0..binding.index_count(),
                                    0,
                                    0..1,
                                );
                                if *has_foil {
                                    fallback_entities_pass.set_pipeline(&self.item_glint_pipeline);
                                    fallback_entities_pass.set_bind_group(
                                        2,
                                        &self.armor_glint_bind_group,
                                        &[],
                                    );
                                    fallback_entities_pass.set_bind_group(
                                        3,
                                        &self.armor_glint_uniform_bind_group,
                                        &[],
                                    );
                                    fallback_entities_pass.draw_indexed(
                                        0..binding.index_count(),
                                        0,
                                        0..1,
                                    );
                                }
                            }
                        }
                    }
                    if let Some(frame_mesh) = wool_mesh
                        && let Some(wool_tex) =
                            self.remote_entity_textures.get("entity/sheep/sheep_wool")
                    {
                        let binding = fallback_entity_mesh_buffers
                            .expect("fallback sheep wool frame mesh must be uploaded")
                            .resolve(frame_mesh);
                        fallback_entities_pass.set_bind_group(1, wool_tex, &[]);
                        fallback_entities_pass.set_vertex_buffer(
                            0,
                            binding.vertex_buffer().slice(binding.vertex_range()),
                        );
                        fallback_entities_pass.set_index_buffer(
                            binding.index_buffer().slice(binding.index_range()),
                            wgpu::IndexFormat::Uint32,
                        );
                        fallback_entities_pass.draw_indexed(0..binding.index_count(), 0, 0..1);
                    }
                }
            }
        }
        drop(fallback_entities_pass);
        if query_started && let Some(gpu_timer) = self.gpu_timer.as_mut() {
            gpu_timer.end_pass(encoder, GpuTimedPass::FallbackEntities);
        }
    }
}
pub(crate) fn remote_player_needs_shared_frame_buffers(
    body_mesh: Option<FrameMeshSlice>,
    armor_meshes: [Option<FrameMeshSlice>; 4],
) -> bool {
    body_mesh.is_some() || armor_meshes.iter().any(|slice| slice.is_some())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RemotePlayerFramePlan {
    pub body: bool,
    pub armor: [bool; 4],
    pub main_item: bool,
    pub off_item: bool,
    pub cape: bool,
}
pub(crate) fn plan_remote_player_frame(
    body_mesh: Option<FrameMeshSlice>,
    body_texture_available: bool,
    armor_meshes: [Option<FrameMeshSlice>; 4],
    armor_textures_available: [bool; 4],
    shared_buffers_available: bool,
    main_item_mesh_present: bool,
    off_item_mesh_present: bool,
    cape_present: bool,
) -> RemotePlayerFramePlan {
    RemotePlayerFramePlan {
        body: body_mesh.is_some() && body_texture_available && shared_buffers_available,
        armor: std::array::from_fn(|index| {
            armor_meshes[index].is_some()
                && armor_textures_available[index]
                && shared_buffers_available
        }),
        main_item: main_item_mesh_present,
        off_item: off_item_mesh_present,
        cape: cape_present,
    }
}
