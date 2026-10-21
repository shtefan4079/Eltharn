use super::*;

pub(in crate::client::state) struct WorldNameTagPassInputs<'a> {
    pub mesh: &'a NameTagMesh,
    pub font_bind_groups: &'a [wgpu::BindGroup],
}

impl ClientRenderer {
    pub(in crate::client::state) fn encode_world_name_tags(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        input: WorldNameTagPassInputs<'_>,
        query_started: bool,
        counters: Option<&mut crate::render::profiler::RenderEncodeScopeCounters>,
    ) {
        let WorldNameTagPassInputs {
            mesh: name_tag_mesh,
            font_bind_groups,
        } = input;
        let scene_view = &self.scene_texture_view;
        let has_name_tag_geometry = !name_tag_mesh.normal_text.indices.is_empty()
            || !name_tag_mesh.normal_background.indices.is_empty()
            || !name_tag_mesh.normal_effects.indices.is_empty()
            || !name_tag_mesh.see_through_text.indices.is_empty()
            || !name_tag_mesh.see_through_background.indices.is_empty()
            || !name_tag_mesh.see_through_effects.indices.is_empty();
        if has_name_tag_geometry {
            self.world_name_tag_mesh_arena.begin_frame();
            let mut append_name_tag_layer = |vertices: &[NameTagVertex], indices: &[u32]| {
                (!vertices.is_empty() && !indices.is_empty()).then(|| {
                    self.world_name_tag_mesh_arena
                        .append_indexed(vertices, indices)
                        .expect("non-empty name-tag geometry must enter the frame arena")
                })
            };
            let see_through_background_slice = append_name_tag_layer(
                &name_tag_mesh.see_through_background.vertices,
                &name_tag_mesh.see_through_background.indices,
            );
            let see_through_text_slice = append_name_tag_layer(
                &name_tag_mesh.see_through_text.vertices,
                &name_tag_mesh.see_through_text.indices,
            );
            let see_through_effect_slice = append_name_tag_layer(
                &name_tag_mesh.see_through_effects.vertices,
                &name_tag_mesh.see_through_effects.indices,
            );
            let normal_background_slice = append_name_tag_layer(
                &name_tag_mesh.normal_background.vertices,
                &name_tag_mesh.normal_background.indices,
            );
            let normal_text_slice = append_name_tag_layer(
                &name_tag_mesh.normal_text.vertices,
                &name_tag_mesh.normal_text.indices,
            );
            let normal_effect_slice = append_name_tag_layer(
                &name_tag_mesh.normal_effects.vertices,
                &name_tag_mesh.normal_effects.indices,
            );
            self.world_name_tag_mesh_arena.upload(device, queue);
            let name_tag_arena = self
                .world_name_tag_mesh_arena
                .uploaded()
                .expect("name-tag geometry requires an uploaded frame arena");

            if query_started && let Some(gpu_timer) = self.gpu_timer.as_mut() {
                gpu_timer.begin_pass(encoder, GpuTimedPass::WorldNameTags);
            }
            let mut name_tag_pass = CountedRenderPass::new(
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("World Name Tag Render Pass"),
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

            let has_see_through = see_through_background_slice.is_some()
                || see_through_text_slice.is_some()
                || see_through_effect_slice.is_some();
            if has_see_through {
                name_tag_pass.set_pipeline(&self.name_tag_see_through_pipeline);
                name_tag_pass.set_bind_group(0, &self.camera_bind_group, &[]);

                if let Some(slice) = see_through_background_slice {
                    let binding = name_tag_arena.resolve(slice);
                    name_tag_pass.set_bind_group(1, &self.name_tag_background_bind_group, &[]);
                    name_tag_pass.set_vertex_buffer(
                        0,
                        binding.vertex_buffer().slice(binding.vertex_range()),
                    );
                    name_tag_pass.set_index_buffer(
                        binding.index_buffer().slice(binding.index_range()),
                        wgpu::IndexFormat::Uint32,
                    );
                    name_tag_pass.draw_indexed(0..binding.index_count(), 0, 0..1);
                }

                if let Some(slice) = see_through_text_slice {
                    let binding = name_tag_arena.resolve(slice);
                    name_tag_pass.set_vertex_buffer(
                        0,
                        binding.vertex_buffer().slice(binding.vertex_range()),
                    );
                    name_tag_pass.set_index_buffer(
                        binding.index_buffer().slice(binding.index_range()),
                        wgpu::IndexFormat::Uint32,
                    );
                    let mut current_texture_index = None;
                    for batch in &name_tag_mesh.see_through_text.batches {
                        if current_texture_index != Some(batch.texture_index) {
                            let Some(bind_group) = font_bind_groups.get(batch.texture_index) else {
                                continue;
                            };
                            name_tag_pass.set_bind_group(1, bind_group, &[]);
                            current_texture_index = Some(batch.texture_index);
                        }
                        name_tag_pass.draw_indexed(
                            batch.start_index..batch.start_index + batch.index_count,
                            0,
                            0..1,
                        );
                    }
                }

                if let Some(slice) = see_through_effect_slice {
                    let binding = name_tag_arena.resolve(slice);
                    name_tag_pass.set_bind_group(1, &self.name_tag_background_bind_group, &[]);
                    name_tag_pass.set_vertex_buffer(
                        0,
                        binding.vertex_buffer().slice(binding.vertex_range()),
                    );
                    name_tag_pass.set_index_buffer(
                        binding.index_buffer().slice(binding.index_range()),
                        wgpu::IndexFormat::Uint32,
                    );
                    name_tag_pass.draw_indexed(0..binding.index_count(), 0, 0..1);
                }
            }

            let has_normal = normal_background_slice.is_some()
                || normal_text_slice.is_some()
                || normal_effect_slice.is_some();
            if has_normal {
                name_tag_pass.set_pipeline(&self.name_tag_pipeline);
                name_tag_pass.set_bind_group(0, &self.camera_bind_group, &[]);

                if let Some(slice) = normal_background_slice {
                    let binding = name_tag_arena.resolve(slice);
                    name_tag_pass.set_bind_group(1, &self.name_tag_background_bind_group, &[]);
                    name_tag_pass.set_vertex_buffer(
                        0,
                        binding.vertex_buffer().slice(binding.vertex_range()),
                    );
                    name_tag_pass.set_index_buffer(
                        binding.index_buffer().slice(binding.index_range()),
                        wgpu::IndexFormat::Uint32,
                    );
                    name_tag_pass.draw_indexed(0..binding.index_count(), 0, 0..1);
                }

                if let Some(slice) = normal_text_slice {
                    let binding = name_tag_arena.resolve(slice);
                    name_tag_pass.set_vertex_buffer(
                        0,
                        binding.vertex_buffer().slice(binding.vertex_range()),
                    );
                    name_tag_pass.set_index_buffer(
                        binding.index_buffer().slice(binding.index_range()),
                        wgpu::IndexFormat::Uint32,
                    );
                    let mut current_texture_index = None;
                    for batch in &name_tag_mesh.normal_text.batches {
                        if current_texture_index != Some(batch.texture_index) {
                            let Some(bind_group) = font_bind_groups.get(batch.texture_index) else {
                                continue;
                            };
                            name_tag_pass.set_bind_group(1, bind_group, &[]);
                            current_texture_index = Some(batch.texture_index);
                        }
                        name_tag_pass.draw_indexed(
                            batch.start_index..batch.start_index + batch.index_count,
                            0,
                            0..1,
                        );
                    }
                }

                if let Some(slice) = normal_effect_slice {
                    let binding = name_tag_arena.resolve(slice);
                    name_tag_pass.set_bind_group(1, &self.name_tag_background_bind_group, &[]);
                    name_tag_pass.set_vertex_buffer(
                        0,
                        binding.vertex_buffer().slice(binding.vertex_range()),
                    );
                    name_tag_pass.set_index_buffer(
                        binding.index_buffer().slice(binding.index_range()),
                        wgpu::IndexFormat::Uint32,
                    );
                    name_tag_pass.draw_indexed(0..binding.index_count(), 0, 0..1);
                }
            }
            drop(name_tag_pass);
            if query_started && let Some(gpu_timer) = self.gpu_timer.as_mut() {
                gpu_timer.end_pass(encoder, GpuTimedPass::WorldNameTags);
            }
        }
    }
}
