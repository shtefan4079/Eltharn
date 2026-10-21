use super::*;
use crate::render::state::level::LevelRenderState;
use crate::render::terrain_gpu_arena::{
    TerrainOpaqueBatch, TerrainOpaqueDraw, TerrainOpaqueIndirectUpload,
};
pub(crate) fn compare_translucent_far_to_near(
    left_distance_squared: f32,
    right_distance_squared: f32,
) -> std::cmp::Ordering {
    right_distance_squared.total_cmp(&left_distance_squared)
}
pub(crate) fn begin_remaining_world_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    scene: &'a wgpu::TextureView,
    depth: &'a wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("World layers after terrain lighting"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: scene,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    })
}
pub(in crate::client::state) struct WorldPassInputs<'a> {
    pub camera: &'a Camera,
    pub aspect: f32,
    pub sky_color: [f32; 3],
    pub level_frame: &'a LevelRenderState,
    pub maps: &'a HashMap<u32, crate::render::map::MapData>,
    pub map_entities: &'a HashMap<i32, RemoteEntityRenderState>,
    pub map_level: Option<&'a ClientLevel>,
    pub cloud_range: i32,
    pub cloud_fog_end: f32,
    pub render_clouds: &'a str,
    pub remote_world: bool,
    pub opaque_terrain_uses_multi_draw: bool,
    pub opaque_terrain_draws: &'a Vec<(RenderSectionKey, TerrainOpaqueDraw)>,
    pub opaque_terrain_batches: &'a Vec<TerrainOpaqueBatch>,
    pub opaque_terrain_indirect_uploads: &'a Vec<TerrainOpaqueIndirectUpload>,
    pub prepared_moving_block_draws: &'a Vec<PreparedMovingBlockDraw>,
    pub pretranslucent_remote_entity_draws: &'a Vec<PreparedRemoteEntityDraw>,
    pub fishing_line_draws: &'a Vec<PreparedRemoteLineDraw>,
    pub breaking_overlay_stages: &'a Vec<usize>,
    pub outline_after_terrain: bool,
    pub outline_index_count: u32,
    pub outline_secondary_index_count: u32,
}
pub(in crate::client::state) struct WorldTranslucentTailInputs<'a> {
    pub camera_position: glam::Vec3,
    pub improved_transparency: bool,
    pub remote_world: bool,
    pub visible_remote_sections: &'a RemoteVisibleSections,
    pub terrain_frustum: &'a crate::render::chunk::Frustum,
    pub outline_after_terrain: bool,
    pub outline_index_count: u32,
    pub outline_secondary_index_count: u32,
    pub weather_counts: Option<(u32, u32)>,
}
impl ClientRenderer {
    pub(in crate::client::state) fn encode_world_pass(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        input: WorldPassInputs<'_>,
        particle_engine: &mut ParticleEngine,
        profile_encode_commands: bool,
        encode_counters: &mut RenderEncodeCounters,
        drawn_block_trace_sections: &mut Option<Vec<RenderSectionKey>>,
        after_opaque_terrain: Option<&mut dyn FnMut(&ClientRenderer, &mut wgpu::CommandEncoder)>,
    ) {
        let WorldPassInputs {
            camera,
            aspect,
            sky_color,
            level_frame,
            maps,
            map_entities,
            map_level,
            cloud_range,
            cloud_fog_end,
            render_clouds,
            remote_world,
            opaque_terrain_uses_multi_draw,
            opaque_terrain_draws,
            opaque_terrain_batches,
            opaque_terrain_indirect_uploads,
            prepared_moving_block_draws,
            pretranslucent_remote_entity_draws,
            fishing_line_draws,
            breaking_overlay_stages,
            outline_after_terrain,
            outline_index_count,
            outline_secondary_index_count,
        } = input;
        if self.missing_entity_texture.is_none() {
            self.missing_entity_texture = Some(item_frame::missing_texture_binding(
                device,
                queue,
                &self.texture_bind_group_layout,
            ));
        }
        if !maps.is_empty() && self.map_renderer.is_none() {
            self.map_renderer = Some(crate::render::map::MapRenderer::new(
                device,
                self.scene_texture.format(),
                &self.camera_bind_group_layout,
                &self.texture_bind_group_layout,
                std::mem::size_of::<CameraUniform>(),
            ));
        }
        if let Some(map_renderer) = &mut self.map_renderer {
            let mut map_camera = self.camera_uniform;
            map_camera.update_translation_free_view_proj(camera, aspect);
            map_camera.lightmap_factors[2..4].copy_from_slice(&[-1.0, -1.0]);
            map_camera.fog_camera_position = [0.0; 4];
            map_renderer.prepare(
                device,
                queue,
                &self.texture_bind_group_layout,
                maps,
                map_entities,
                map_level,
                camera.position.as_dvec3(),
                level_frame.partial_tick,
                bytemuck::bytes_of(&map_camera),
            );
        }
        let scene_view = &self.scene_texture_view;
        let atlas = self.block_atlas.as_ref().expect("prepared atlas");
        let moving_block_atlas_bind_group = self
            .moving_block_atlas_bind_group
            .as_ref()
            .expect("prepared moving-block atlas");
        let pretranslucent_entity_arena_buffers = self.pretranslucent_entity_mesh_arena.uploaded();
        let pretranslucent_item_arena_buffers = self.pretranslucent_item_mesh_arena.uploaded();
        {
            let mut render_pass = CountedRenderPass::new(
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: scene_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: sky_color[0] as f64,
                                g: sky_color[1] as f64,
                                b: sky_color[2] as f64,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_texture,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(JAVA_REVERSE_DEPTH_CLEAR_VALUE),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                }),
                profile_encode_commands.then_some(&mut encode_counters.world),
            );
            let translation_free_view_proj = camera.render_view_rotation_projection_matrix(aspect);
            if let Some(sky) = &self.sky_renderer {
                let sky_draw_started_at = Instant::now();
                sky.draw_pass(
                    &mut render_pass,
                    &atlas.bind_group,
                    &level_frame.sky_render_state,
                );
                self.render_profiler
                    .observe("SkyRenderer::draw_pass", sky_draw_started_at.elapsed());
            }
            let configured_cloud_range = cloud_range;
            if let Some(cloud) = &mut self.cloud_renderer {
                let render_clouds_opt = render_clouds;
                if render_clouds_opt != "false" {
                    let camera_pos = camera.position;
                    let fancy = render_clouds_opt == "true";
                    let cloud_range = configured_cloud_range;
                    let cloud_draw_started_at = Instant::now();
                    cloud.draw_pass(
                        device,
                        queue,
                        &mut render_pass,
                        &atlas.bind_group,
                        translation_free_view_proj,
                        camera_pos,
                        level_frame.game_time,
                        level_frame.partial_tick,
                        fancy,
                        level_frame.cloud_color,
                        level_frame.cloud_height,
                        cloud_range,
                        cloud_fog_end,
                    );
                    self.render_profiler
                        .observe("CloudRenderer::draw_pass", cloud_draw_started_at.elapsed());
                }
            }
            let opaque_terrain_encode_started_at = Instant::now();
            render_pass.set_pipeline(&self.terrain_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &atlas.bind_group, &[]);
            if remote_world {
                if opaque_terrain_uses_multi_draw {
                    for (batch, upload) in opaque_terrain_batches
                        .iter()
                        .zip(opaque_terrain_indirect_uploads.iter().copied())
                    {
                        let Some((vertex_buffer, index_buffer)) =
                            self.remote_terrain_gpu_arena.buffers(batch.bank)
                        else {
                            debug_assert!(false, "terrain batch references a missing bank");
                            continue;
                        };
                        let Some(indirect_buffer) =
                            self.remote_terrain_gpu_arena.indirect_buffer(upload)
                        else {
                            debug_assert!(false, "terrain indirect batch was not uploaded");
                            continue;
                        };
                        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                        render_pass
                            .set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                        render_pass.multi_draw_indexed_indirect(
                            indirect_buffer,
                            0,
                            upload.draw_count,
                        );
                    }
                    if let Some(sections) = drawn_block_trace_sections.as_mut() {
                        sections.extend(opaque_terrain_draws.iter().map(|(key, _)| *key));
                    }
                } else {
                    let mut current_bound_bank = None;
                    for (key, draw) in opaque_terrain_draws {
                        if current_bound_bank != Some(draw.bank) {
                            let Some((vertex_buffer, index_buffer)) =
                                self.remote_terrain_gpu_arena.buffers(draw.bank)
                            else {
                                debug_assert!(false, "terrain section references a missing bank");
                                continue;
                            };
                            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                            render_pass.set_index_buffer(
                                index_buffer.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );
                            current_bound_bank = Some(draw.bank);
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (vertex_buffer, _) = self
                                .remote_terrain_gpu_arena
                                .buffers(draw.bank)
                                .expect("bound terrain bank disappeared");
                            let offset = u64::try_from(draw.base_vertex)
                                .expect("negative terrain base vertex")
                                * std::mem::size_of::<Vertex>() as u64;
                            render_pass.set_vertex_buffer(0, vertex_buffer.slice(offset..));
                            render_pass.draw_indexed(
                                draw.first_index..draw.first_index + draw.index_count,
                                0,
                                0..1,
                            );
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        render_pass.draw_indexed(
                            draw.first_index..draw.first_index + draw.index_count,
                            draw.base_vertex,
                            0..1,
                        );
                        if let Some(sections) = drawn_block_trace_sections.as_mut() {
                            sections.push(*key);
                        }
                    }
                }
            } else if self.num_indices > 0 {
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
            }
            self.render_profiler.observe(
                "State::render::opaque_terrain_encode",
                opaque_terrain_encode_started_at.elapsed(),
            );
            if let Some(after_opaque_terrain) = after_opaque_terrain {
                drop(render_pass);
                after_opaque_terrain(self, encoder);
                render_pass = CountedRenderPass::new(
                    begin_remaining_world_pass(encoder, scene_view, &self.depth_texture),
                    profile_encode_commands.then_some(&mut encode_counters.world),
                );
                render_pass.set_pipeline(&self.terrain_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &atlas.bind_group, &[]);
            }
            for draw in prepared_moving_block_draws
                .iter()
                .filter(|draw| !draw.translucent_phase)
            {
                render_pass.set_bind_group(
                    0,
                    self.pretranslucent_entity_camera_arena
                        .bind_group(draw.camera_bind_group_index),
                    &[],
                );
                render_pass.set_bind_group(1, &moving_block_atlas_bind_group, &[]);
                if let Some(layer) = draw.solid.as_ref() {
                    let binding = pretranslucent_entity_arena_buffers
                        .expect("moving-block draw requires an uploaded frame arena")
                        .resolve(layer.frame_mesh);
                    render_pass.set_pipeline(&self.moving_block_solid_pipeline);
                    render_pass.set_vertex_buffer(0, binding.vertex_buffer().slice(..));
                    render_pass.set_index_buffer(
                        binding.index_buffer().slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    let (first_index, index_count, base_vertex) =
                        binding.draw_indexed_args::<Vertex>();
                    render_pass.draw_indexed(
                        first_index..first_index + index_count,
                        base_vertex,
                        0..1,
                    );
                }
                if let Some(layer) = draw.cutout.as_ref() {
                    let binding = pretranslucent_entity_arena_buffers
                        .expect("moving-block draw requires an uploaded frame arena")
                        .resolve(layer.frame_mesh);
                    render_pass.set_pipeline(&self.moving_block_cutout_pipeline);
                    render_pass.set_vertex_buffer(0, binding.vertex_buffer().slice(..));
                    render_pass.set_index_buffer(
                        binding.index_buffer().slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    let (first_index, index_count, base_vertex) =
                        binding.draw_indexed_args::<Vertex>();
                    render_pass.draw_indexed(
                        first_index..first_index + index_count,
                        base_vertex,
                        0..1,
                    );
                }
                if let Some(layer) = draw.translucent.as_ref() {
                    let binding = pretranslucent_entity_arena_buffers
                        .expect("moving-block draw requires an uploaded frame arena")
                        .resolve(layer.frame_mesh);
                    render_pass.set_pipeline(&self.moving_block_translucent_pipeline);
                    render_pass.set_vertex_buffer(0, binding.vertex_buffer().slice(..));
                    render_pass.set_index_buffer(
                        binding.index_buffer().slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    let (first_index, index_count, base_vertex) =
                        binding.draw_indexed_args::<Vertex>();
                    render_pass.draw_indexed(
                        first_index..first_index + index_count,
                        base_vertex,
                        0..1,
                    );
                }
            }
            #[cfg(feature = "extensions")]
            if let Some(gpu) = &self.mod_content {
                for (mesh, pipeline) in [
                    (&gpu.solid, &self.moving_block_solid_pipeline),
                    (&gpu.cutout, &self.moving_block_cutout_pipeline),
                    (&gpu.translucent, &self.moving_block_translucent_pipeline),
                ] {
                    if let (Some(vertices), Some(indices)) =
                        (&mesh.vertex_buffer, &mesh.index_buffer)
                        && mesh.index_count > 0
                    {
                        render_pass.set_pipeline(pipeline);
                        render_pass.set_bind_group(0, &gpu.camera_bind_group, &[]);
                        render_pass.set_bind_group(1, &gpu.atlas.bind_group, &[]);
                        render_pass.set_vertex_buffer(0, vertices.slice(..));
                        render_pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                        render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                    }
                }
            }
            if let (Some(vertex_buffer), Some(index_buffer)) = (
                self.block_entity_gpu_mesh.vertex_buffer.as_ref(),
                self.block_entity_gpu_mesh.index_buffer.as_ref(),
            ) && self.block_entity_gpu_mesh.index_count > 0
            {
                render_pass.set_pipeline(&self.block_entity_render_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &atlas.bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.block_entity_gpu_mesh.index_count, 0, 0..1);
            }
            for (texture_key, gpu_mesh) in &self.block_entity_dynamic_player_skull_meshes {
                if gpu_mesh.index_count == 0 {
                    continue;
                }
                let Some(texture) = self.remote_entity_textures.get(texture_key) else {
                    continue;
                };
                let (Some(vertex_buffer), Some(index_buffer)) = (
                    gpu_mesh.vertex_buffer.as_ref(),
                    gpu_mesh.index_buffer.as_ref(),
                ) else {
                    continue;
                };
                render_pass.set_pipeline(&self.inventory_preview_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, texture, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
            }
            if let (Some(vertex_buffer), Some(index_buffer)) = (
                self.block_entity_pattern_gpu_mesh.vertex_buffer.as_ref(),
                self.block_entity_pattern_gpu_mesh.index_buffer.as_ref(),
            ) && self.block_entity_pattern_gpu_mesh.index_count > 0
            {
                render_pass.set_pipeline(&self.banner_pattern_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &atlas.bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(
                    0..self.block_entity_pattern_gpu_mesh.index_count,
                    0,
                    0..1,
                );
            }
            if let Some(map_renderer) = &self.map_renderer {
                map_renderer.draw(&mut render_pass);
            }
            let camera_right = camera.render_right();
            let camera_up = camera_right
                .cross(camera.render_forward())
                .normalize_or_zero();
            let particle_render_started_at = Instant::now();
            particle_engine.render(
                device,
                queue,
                &mut render_pass,
                &self.camera_bind_group,
                &atlas.bind_group,
                &self.opaque_particle_pipeline,
                &self.particle_pipeline,
                camera_right,
                camera_up,
                level_frame.partial_tick,
            );
            self.render_profiler.observe(
                "ParticleEngine::render",
                particle_render_started_at.elapsed(),
            );
            if !pretranslucent_remote_entity_draws.is_empty() {
                let mut active_frame_mesh_arena = None::<bool>;
                for draw in pretranslucent_remote_entity_draws {
                    let entity_tex = if draw.uses_block_atlas {
                        &atlas.bind_group
                    } else {
                        self.remote_entity_textures
                            .get(draw.texture_path)
                            .unwrap_or_else(|| {
                                self.missing_entity_texture
                                    .as_ref()
                                    .expect("missing image prepared")
                            })
                    };
                    render_pass.set_pipeline(if draw.uses_experience_orb_pipeline {
                        &self.experience_orb_pipeline
                    } else if draw.uses_wind_charge_pipeline {
                        &self.wind_charge_pipeline
                    } else if draw.uses_dragon_fireball_pipeline {
                        &self.dragon_fireball_pipeline
                    } else if draw.uses_entity_cutout_cull_pipeline {
                        if draw.uses_block_atlas {
                            &self.block_entity_render_pipeline
                        } else {
                            &self.entity_direct_cutout_cull_pipeline
                        }
                    } else if draw.uses_entity_cutout_pipeline {
                        &self.dragon_fireball_pipeline
                    } else {
                        &self.entity_direct_cutout_cull_pipeline
                    });
                    render_pass.set_bind_group(
                        0,
                        self.pretranslucent_entity_camera_arena
                            .bind_group(draw.camera_bind_group_index),
                        &[],
                    );
                    render_pass.set_bind_group(1, entity_tex, &[]);
                    let PreparedRemoteEntityMesh::Frame(frame_mesh) = &draw.mesh;
                    let binding = pretranslucent_entity_arena_buffers
                        .expect("entity draw requires an uploaded frame arena")
                        .resolve(*frame_mesh);
                    if active_frame_mesh_arena != Some(true) {
                        render_pass.set_vertex_buffer(0, binding.vertex_buffer().slice(..));
                        render_pass.set_index_buffer(
                            binding.index_buffer().slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        active_frame_mesh_arena = Some(true);
                    }
                    let (first_index, index_count, base_vertex) =
                        binding.draw_indexed_args::<Vertex>();
                    render_pass.draw_indexed(
                        first_index..first_index + index_count,
                        base_vertex,
                        0..1,
                    );
                    for layer in &draw.extra_layers {
                        let Some(layer_tex) = self.remote_entity_textures.get(layer.texture_path)
                        else {
                            continue;
                        };
                        let binding = pretranslucent_entity_arena_buffers
                            .expect("entity feature draw requires an uploaded frame arena")
                            .resolve(layer.frame_mesh);
                        render_pass.set_bind_group(1, layer_tex, &[]);
                        if active_frame_mesh_arena != Some(true) {
                            render_pass.set_vertex_buffer(0, binding.vertex_buffer().slice(..));
                            render_pass.set_index_buffer(
                                binding.index_buffer().slice(..),
                                wgpu::IndexFormat::Uint32,
                            );
                            active_frame_mesh_arena = Some(true);
                        }
                        let (first_index, index_count, base_vertex) =
                            binding.draw_indexed_args::<Vertex>();
                        render_pass.draw_indexed(
                            first_index..first_index + index_count,
                            base_vertex,
                            0..1,
                        );
                    }
                    if !draw.held_items.is_empty() {
                        render_pass.set_pipeline(&self.item_pipeline);
                        render_pass.set_bind_group(1, &atlas.bind_group, &[]);
                        for item in &draw.held_items {
                            let binding = pretranslucent_item_arena_buffers
                                .expect("held-item draw requires an uploaded frame arena")
                                .resolve(item.frame_mesh);
                            render_pass.set_pipeline(&self.item_pipeline);
                            render_pass.set_bind_group(
                                0,
                                self.pretranslucent_entity_camera_arena
                                    .bind_group(item.camera_bind_group_index),
                                &[],
                            );
                            if active_frame_mesh_arena != Some(false) {
                                render_pass.set_vertex_buffer(0, binding.vertex_buffer().slice(..));
                                render_pass.set_index_buffer(
                                    binding.index_buffer().slice(..),
                                    wgpu::IndexFormat::Uint32,
                                );
                                active_frame_mesh_arena = Some(false);
                            }
                            let (first_index, index_count, base_vertex) =
                                binding.draw_indexed_args::<Vertex>();
                            render_pass.draw_indexed(
                                first_index..first_index + index_count,
                                base_vertex,
                                0..1,
                            );
                            if item.has_foil {
                                render_pass.set_pipeline(&self.item_glint_pipeline);
                                render_pass.set_bind_group(1, &atlas.bind_group, &[]);
                                render_pass.set_bind_group(2, &self.item_glint_bind_group, &[]);
                                render_pass.set_bind_group(
                                    3,
                                    &self.item_glint_uniform_bind_group,
                                    &[],
                                );
                                render_pass.draw_indexed(
                                    first_index..first_index + index_count,
                                    base_vertex,
                                    0..1,
                                );
                            }
                        }
                    }
                }
            }
            if !fishing_line_draws.is_empty() {
                render_pass.set_pipeline(&self.outline_primary_pipeline);
                render_pass.set_bind_group(0, &self.outline_camera_bind_group, &[]);
                for draw in fishing_line_draws {
                    let binding = pretranslucent_entity_arena_buffers
                        .expect("fishing-line draw requires an uploaded frame arena")
                        .resolve(draw.frame_mesh);
                    render_pass.set_vertex_buffer(0, binding.vertex_buffer().slice(..));
                    render_pass.set_index_buffer(
                        binding.index_buffer().slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    let (first_index, index_count, base_vertex) =
                        binding.draw_indexed_args::<LineVertex>();
                    render_pass.draw_indexed(
                        first_index..first_index + index_count,
                        base_vertex,
                        0..1,
                    );
                }
            }
            if !outline_after_terrain {
                if let (Some(vertex_buffer), Some(index_buffer)) = (
                    self.block_outline_gpu_mesh.vertex_buffer.as_ref(),
                    self.block_outline_gpu_mesh.index_buffer.as_ref(),
                ) && outline_index_count > 0
                {
                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    let (secondary_range, primary_range) = block_outline_draw_ranges(
                        outline_index_count,
                        outline_secondary_index_count,
                    );
                    if let Some(secondary_range) = secondary_range {
                        render_pass.set_pipeline(&self.outline_secondary_pipeline);
                        render_pass.set_bind_group(0, &self.outline_camera_bind_group, &[]);
                        render_pass.draw_indexed(secondary_range, 0, 0..1);
                    }
                    if !primary_range.is_empty() {
                        render_pass.set_pipeline(&self.outline_primary_pipeline);
                        render_pass.set_bind_group(0, &self.outline_camera_bind_group, &[]);
                        render_pass.draw_indexed(primary_range, 0, 0..1);
                    }
                }
            }
            let mut translucent_moving_block_draws = prepared_moving_block_draws
                .iter()
                .filter(|draw| draw.translucent_phase)
                .collect::<Vec<_>>();
            translucent_moving_block_draws.sort_unstable_by(|left, right| {
                compare_translucent_far_to_near(
                    left.translucent_sort_distance_squared,
                    right.translucent_sort_distance_squared,
                )
            });
            for draw in translucent_moving_block_draws {
                render_pass.set_bind_group(
                    0,
                    self.pretranslucent_entity_camera_arena
                        .bind_group(draw.camera_bind_group_index),
                    &[],
                );
                render_pass.set_bind_group(1, &moving_block_atlas_bind_group, &[]);
                if let Some(layer) = draw.solid.as_ref() {
                    let binding = pretranslucent_entity_arena_buffers
                        .expect("moving-block draw requires an uploaded frame arena")
                        .resolve(layer.frame_mesh);
                    render_pass.set_pipeline(&self.moving_block_solid_pipeline);
                    render_pass.set_vertex_buffer(0, binding.vertex_buffer().slice(..));
                    render_pass.set_index_buffer(
                        binding.index_buffer().slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    let (first_index, index_count, base_vertex) =
                        binding.draw_indexed_args::<Vertex>();
                    render_pass.draw_indexed(
                        first_index..first_index + index_count,
                        base_vertex,
                        0..1,
                    );
                }
                if let Some(layer) = draw.cutout.as_ref() {
                    let binding = pretranslucent_entity_arena_buffers
                        .expect("moving-block draw requires an uploaded frame arena")
                        .resolve(layer.frame_mesh);
                    render_pass.set_pipeline(&self.moving_block_cutout_pipeline);
                    render_pass.set_vertex_buffer(0, binding.vertex_buffer().slice(..));
                    render_pass.set_index_buffer(
                        binding.index_buffer().slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    let (first_index, index_count, base_vertex) =
                        binding.draw_indexed_args::<Vertex>();
                    render_pass.draw_indexed(
                        first_index..first_index + index_count,
                        base_vertex,
                        0..1,
                    );
                }
                if let Some(layer) = draw.translucent.as_ref() {
                    let binding = pretranslucent_entity_arena_buffers
                        .expect("moving-block draw requires an uploaded frame arena")
                        .resolve(layer.frame_mesh);
                    render_pass.set_pipeline(&self.moving_block_translucent_pipeline);
                    render_pass.set_vertex_buffer(0, binding.vertex_buffer().slice(..));
                    render_pass.set_index_buffer(
                        binding.index_buffer().slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    let (first_index, index_count, base_vertex) =
                        binding.draw_indexed_args::<Vertex>();
                    render_pass.draw_indexed(
                        first_index..first_index + index_count,
                        base_vertex,
                        0..1,
                    );
                }
            }
            if let Some(crumbling_textures) = self.crumbling_textures.as_ref() {
                render_pass.set_pipeline(&self.crumbling_pipeline);
                render_pass.set_bind_group(0, &self.crumbling_camera_bind_group, &[]);
                for &stage in breaking_overlay_stages {
                    let gpu_mesh = &self.block_breaking_overlay_gpu_meshes[stage];
                    let (Some(vertex_buffer), Some(index_buffer)) = (
                        gpu_mesh.vertex_buffer.as_ref(),
                        gpu_mesh.index_buffer.as_ref(),
                    ) else {
                        continue;
                    };
                    render_pass.set_bind_group(1, &crumbling_textures.stages[stage], &[]);
                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
                }
            }
        }
    }
    pub(in crate::client::state) fn encode_world_translucent_tail(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        input: WorldTranslucentTailInputs<'_>,
        profile_encode_commands: bool,
        encode_counters: &mut RenderEncodeCounters,
        drawn_block_trace_sections: &mut Option<Vec<RenderSectionKey>>,
    ) {
        let WorldTranslucentTailInputs {
            camera_position,
            improved_transparency,
            remote_world,
            visible_remote_sections,
            terrain_frustum,
            outline_after_terrain,
            outline_index_count,
            outline_secondary_index_count,
            weather_counts,
        } = input;
        let scene_view = &self.scene_texture_view;
        let atlas = self.block_atlas.as_ref().expect("prepared atlas");
        let mut render_pass = CountedRenderPass::new(
            begin_remaining_world_pass(encoder, scene_view, &self.depth_texture),
            profile_encode_commands.then_some(&mut encode_counters.world),
        );
        let translucent_terrain_encode_started_at = Instant::now();
        render_pass.set_pipeline(&self.translucent_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &atlas.bind_group, &[]);
        if remote_world {
            let mut translucent_sections = visible_remote_sections
                .keys()
                .iter()
                .filter_map(|key| {
                    let mesh = self.remote_section_meshes.get(key)?;
                    (mesh.translucent_indices > 0
                        && terrain_frustum.contains_section(
                            key.chunk_x,
                            key.section_y,
                            key.chunk_z,
                        ))
                    .then_some((
                        *key,
                        mesh.allocation,
                        mesh.translucent_start,
                        mesh.translucent_indices,
                    ))
                })
                .map(
                    |(key, allocation, translucent_start, translucent_indices)| {
                        let center = glam::Vec3::new(
                            key.chunk_x as f32 * 16.0 + 8.0,
                            key.section_y as f32 * 16.0 + 8.0,
                            key.chunk_z as f32 * 16.0 + 8.0,
                        );
                        (
                            center.distance_squared(camera_position),
                            key,
                            allocation,
                            translucent_start,
                            translucent_indices,
                        )
                    },
                )
                .collect::<Vec<_>>();
            translucent_sections.sort_unstable_by(
                |(left_distance, _, _, _, _), (right_distance, _, _, _, _)| {
                    compare_translucent_far_to_near(*left_distance, *right_distance)
                },
            );
            let mut current_bound_bank = None;
            for (_, key, allocation, translucent_start, translucent_indices) in translucent_sections
            {
                let bank = allocation.bank();
                if current_bound_bank != Some(bank) {
                    let Some((vertex_buffer, index_buffer)) =
                        self.remote_terrain_gpu_arena.buffers(bank)
                    else {
                        debug_assert!(false, "terrain section references a missing bank");
                        continue;
                    };
                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    current_bound_bank = Some(bank);
                }
                let Ok(draw) = self.remote_terrain_gpu_arena.draw_for_index_range(
                    allocation,
                    translucent_start,
                    translucent_indices,
                ) else {
                    debug_assert!(false, "terrain translucent draw range overflowed");
                    continue;
                };
                #[cfg(target_arch = "wasm32")]
                {
                    let (vertex_buffer, _) = self
                        .remote_terrain_gpu_arena
                        .buffers(draw.bank)
                        .expect("bound terrain bank disappeared");
                    let offset = u64::try_from(draw.base_vertex)
                        .expect("negative terrain base vertex")
                        * std::mem::size_of::<Vertex>() as u64;
                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(offset..));
                    render_pass.draw_indexed(
                        draw.first_index..draw.first_index + draw.index_count,
                        0,
                        0..1,
                    );
                }
                #[cfg(not(target_arch = "wasm32"))]
                render_pass.draw_indexed(
                    draw.first_index..draw.first_index + draw.index_count,
                    draw.base_vertex,
                    0..1,
                );
                if let Some(sections) = drawn_block_trace_sections.as_mut() {
                    sections.push(key);
                }
            }
        } else if self.translucent_num_indices > 0 {
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(
                self.translucent_index_start
                    ..self.translucent_index_start + self.translucent_num_indices,
                0,
                0..1,
            );
        }
        self.render_profiler.observe(
            "State::render::translucent_terrain_encode",
            translucent_terrain_encode_started_at.elapsed(),
        );
        if let Some((rain_indices, snow_indices)) = weather_counts
            && let (Some(vertex_buffer), Some(index_buffer)) = (
                self.weather_gpu_mesh.vertex_buffer.as_ref(),
                self.weather_gpu_mesh.index_buffer.as_ref(),
            )
            && let Some(weather_textures) = self.weather_textures.as_ref()
        {
            render_pass.set_pipeline(
                if weather_depth_write_enabled_like_java(improved_transparency) {
                    &self.weather_depth_write_pipeline
                } else {
                    &self.weather_no_depth_write_pipeline
                },
            );
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            if rain_indices > 0 {
                render_pass.set_bind_group(1, &weather_textures.rain, &[]);
                render_pass.draw_indexed(0..rain_indices, 0, 0..1);
            }
            if snow_indices > 0 {
                render_pass.set_bind_group(1, &weather_textures.snow, &[]);
                render_pass.draw_indexed(rain_indices..rain_indices + snow_indices, 0, 0..1);
            }
        }
        if outline_after_terrain {
            if let (Some(vertex_buffer), Some(index_buffer)) = (
                self.block_outline_gpu_mesh.vertex_buffer.as_ref(),
                self.block_outline_gpu_mesh.index_buffer.as_ref(),
            ) && outline_index_count > 0
            {
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                let (secondary_range, primary_range) =
                    block_outline_draw_ranges(outline_index_count, outline_secondary_index_count);
                if let Some(secondary_range) = secondary_range {
                    render_pass.set_pipeline(&self.outline_secondary_pipeline);
                    render_pass.set_bind_group(0, &self.outline_camera_bind_group, &[]);
                    render_pass.draw_indexed(secondary_range, 0, 0..1);
                }
                if !primary_range.is_empty() {
                    render_pass.set_pipeline(&self.outline_primary_pipeline);
                    render_pass.set_bind_group(0, &self.outline_camera_bind_group, &[]);
                    render_pass.draw_indexed(primary_range, 0, 0..1);
                }
            }
        }
    }
}
