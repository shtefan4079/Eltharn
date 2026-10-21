use super::*;

pub(in crate::client::state) struct LocalAvatarPassInputs<'a> {
    pub player_texture: &'a wgpu::BindGroup,
    pub cape_texture: Option<&'a wgpu::BindGroup>,
    pub player_camera_uniform: CameraUniform,
    pub main_item_camera_uniform: Option<CameraUniform>,
    pub offhand_item_camera_uniform: Option<CameraUniform>,
    pub body_visible: bool,
    pub layers_visible: bool,
}

impl ClientRenderer {
    fn held_item_pipeline(&self, custom_head_texture: bool) -> &wgpu::RenderPipeline {
        match crate::render::armor_pass::item_pipeline_id(custom_head_texture) {
            crate::render::armor_pass::AvatarPipelineId::ItemPreview => {
                &self.inventory_preview_pipeline
            }
            _ => &self.item_pipeline,
        }
    }

    pub(in crate::client::state) fn encode_local_avatar(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        input: LocalAvatarPassInputs<'_>,
        counters: Option<&mut crate::render::profiler::RenderEncodeScopeCounters>,
    ) {
        let (Some(v_buf), Some(i_buf)) = (&self.player_vertex_buffer, &self.player_index_buffer)
        else {
            return;
        };
        let atlas = self.block_atlas.as_ref().expect("prepared atlas");
        let scene_view = &self.scene_texture_view;

        queue.write_buffer(
            &self.player_camera_buffer,
            0,
            bytemuck::cast_slice(&[input.player_camera_uniform]),
        );
        let main_item_draw_data = if let (Some(uniform), Some(vb), Some(ib)) = (
            input.main_item_camera_uniform,
            &self.third_person_main_item_vertex_buffer,
            &self.third_person_main_item_index_buffer,
        ) {
            queue.write_buffer(
                &self.held_item_camera_buffer,
                0,
                bytemuck::cast_slice(&[uniform]),
            );
            Some((vb, ib))
        } else {
            None
        };
        let offhand_item_draw_data = if let (Some(uniform), Some(vb), Some(ib)) = (
            input.offhand_item_camera_uniform,
            &self.third_person_offhand_item_vertex_buffer,
            &self.third_person_offhand_item_index_buffer,
        ) {
            queue.write_buffer(
                &self.off_hand_item_camera_buffer,
                0,
                bytemuck::cast_slice(&[uniform]),
            );
            Some((vb, ib))
        } else {
            None
        };

        let mut render_pass = CountedRenderPass::new(
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Player Render Pass"),
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

        render_pass.set_pipeline(&self.inventory_preview_pipeline);
        render_pass.set_bind_group(0, &self.player_camera_bind_group, &[]);
        render_pass.set_bind_group(1, input.player_texture, &[]);
        render_pass.set_vertex_buffer(0, v_buf.slice(..));
        render_pass.set_index_buffer(i_buf.slice(..), wgpu::IndexFormat::Uint32);
        if input.body_visible {
            render_pass.draw_indexed(0..self.player_num_indices, 0, 0..1);
        }
        if !input.layers_visible {
            return;
        }
        for armor in crate::render::model_frame::WORLD_ARMOR_DRAW_ORDER
            .iter()
            .filter_map(|&slot| self.local_armor_meshes[slot].as_ref())
            .flat_map(|armor| armor.layers.iter())
        {
            let Some(armor_texture) = self.remote_entity_textures.get(&armor.texture_key) else {
                continue;
            };
            let (Some(vertex_buffer), Some(index_buffer)) =
                (&armor.gpu_mesh.vertex_buffer, &armor.gpu_mesh.index_buffer)
            else {
                continue;
            };
            render_pass.set_pipeline(if armor.decal {
                &self.armor_decal_pipeline
            } else {
                &self.armor_pipeline
            });
            render_pass.set_bind_group(0, &self.player_camera_bind_group, &[]);
            render_pass.set_bind_group(1, armor_texture, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..armor.gpu_mesh.index_count, 0, 0..1);
            if armor.has_foil {
                render_pass.set_pipeline(&self.item_glint_pipeline);
                render_pass.set_bind_group(2, &self.armor_glint_bind_group, &[]);
                render_pass.set_bind_group(3, &self.armor_glint_uniform_bind_group, &[]);
                render_pass.draw_indexed(0..armor.gpu_mesh.index_count, 0, 0..1);
            }
        }

        if let Some((vb, ib)) = main_item_draw_data {
            let custom_head_texture = self
                .held_item_custom_texture_key
                .as_deref()
                .and_then(|key| self.remote_entity_textures.get(key))
                .is_some();
            render_pass.set_pipeline(self.held_item_pipeline(custom_head_texture));
            render_pass.set_bind_group(0, &self.held_item_camera_bind_group, &[]);
            if let Some(key) = self.held_item_custom_texture_key.as_deref()
                && let Some(texture) = self.remote_entity_textures.get(key)
            {
                render_pass.set_bind_group(1, texture, &[]);
            } else {
                render_pass.set_bind_group(1, &atlas.bind_group, &[]);
            }
            render_pass.set_vertex_buffer(0, vb.slice(..));
            render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.third_person_main_item_num_indices, 0, 0..1);
            if self.held_item_stack.as_ref().is_some_and(item_has_foil) {
                render_pass.set_pipeline(&self.item_glint_pipeline);
                render_pass.set_bind_group(2, &self.item_glint_bind_group, &[]);
                render_pass.set_bind_group(3, &self.item_glint_uniform_bind_group, &[]);
                render_pass.set_bind_group(0, &self.held_item_camera_bind_group, &[]);
                if let Some(key) = self.held_item_custom_texture_key.as_deref()
                    && let Some(texture) = self.remote_entity_textures.get(key)
                {
                    render_pass.set_bind_group(1, texture, &[]);
                } else {
                    render_pass.set_bind_group(1, &atlas.bind_group, &[]);
                }
                render_pass.draw_indexed(0..self.third_person_main_item_num_indices, 0, 0..1);
            }
        }

        if let Some((vb, ib)) = offhand_item_draw_data {
            let custom_head_texture = self
                .off_hand_item_custom_texture_key
                .as_deref()
                .and_then(|key| self.remote_entity_textures.get(key))
                .is_some();
            render_pass.set_pipeline(self.held_item_pipeline(custom_head_texture));
            render_pass.set_bind_group(0, &self.off_hand_item_camera_bind_group, &[]);
            if let Some(key) = self.off_hand_item_custom_texture_key.as_deref()
                && let Some(texture) = self.remote_entity_textures.get(key)
            {
                render_pass.set_bind_group(1, texture, &[]);
            } else {
                render_pass.set_bind_group(1, &atlas.bind_group, &[]);
            }
            render_pass.set_vertex_buffer(0, vb.slice(..));
            render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.third_person_offhand_item_num_indices, 0, 0..1);
            if self.off_hand_item_stack.as_ref().is_some_and(item_has_foil) {
                render_pass.set_pipeline(&self.item_glint_pipeline);
                render_pass.set_bind_group(2, &self.item_glint_bind_group, &[]);
                render_pass.set_bind_group(3, &self.item_glint_uniform_bind_group, &[]);
                render_pass.set_bind_group(0, &self.off_hand_item_camera_bind_group, &[]);
                if let Some(key) = self.off_hand_item_custom_texture_key.as_deref()
                    && let Some(texture) = self.remote_entity_textures.get(key)
                {
                    render_pass.set_bind_group(1, texture, &[]);
                } else {
                    render_pass.set_bind_group(1, &atlas.bind_group, &[]);
                }
                render_pass.draw_indexed(0..self.third_person_offhand_item_num_indices, 0, 0..1);
            }
        }

        if self.player_cape_num_indices > 0
            && let (Some(cape_v_buf), Some(cape_i_buf)) = (
                &self.player_cape_vertex_buffer,
                &self.player_cape_index_buffer,
            )
            && let Some(cape_tex) = input.cape_texture
        {
            render_pass.set_pipeline(&self.cape_pipeline);
            render_pass.set_bind_group(0, &self.player_camera_bind_group, &[]);
            render_pass.set_bind_group(1, cape_tex, &[]);
            render_pass.set_vertex_buffer(0, cape_v_buf.slice(..));
            render_pass.set_index_buffer(cape_i_buf.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.player_cape_num_indices, 0, 0..1);
        }

        drop(render_pass);
    }
}
