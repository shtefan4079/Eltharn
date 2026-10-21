use crate::render::baked_model::Vertex;
use crate::render::state::level::sky_render_state::SkyRenderState;
use crate::render::texture::TextureAtlas;
use el_core::ResourceLocation;
use el_math::JavaRandom;
use wgpu::util::DeviceExt;
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct SkyCameraUniform {
    pub(crate) view_proj: [[f32; 4]; 4],
    pub(crate) lightmap_factors: [f32; 4],
}
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyUniforms {
    model_matrix: [[f32; 4]; 4],
    color_modifier: [f32; 4],
    fog_color: [f32; 4],
    fog_params: [f32; 4],
    _padding: [[f32; 4]; 9],
}
#[inline]
fn java_no_depth_state_with_attached_target(
    format: wgpu::TextureFormat,
) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format,
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Always,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}
pub struct SkyRenderer {
    pub star_buffer: wgpu::Buffer,
    pub star_index_count: u32,
    pub sun_buffer: wgpu::Buffer,
    pub moon_buffer: wgpu::Buffer,
    pub top_sky_buffer: wgpu::Buffer,
    pub bottom_sky_buffer: wgpu::Buffer,
    pub sunrise_buffer: wgpu::Buffer,
    pub sky_index_buffer: wgpu::Buffer,
    pub sunrise_index_buffer: wgpu::Buffer,
    pub star_index_buffer: wgpu::Buffer,
    pub celestial_index_buffer: wgpu::Buffer,
    sky_pipeline: wgpu::RenderPipeline,
    sunrise_pipeline: wgpu::RenderPipeline,
    stars_pipeline: wgpu::RenderPipeline,
    celestial_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pub sky_camera_buffer: wgpu::Buffer,
    pub sky_camera_bind_group: wgpu::BindGroup,
    pub(crate) celestial_buffers_updated: bool,
}
impl SkyRenderer {
    pub fn new(
        device: &wgpu::Device,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        let (star_vertices, star_indices) = Self::build_stars();
        let star_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stars Vertex Buffer"),
            contents: bytemuck::cast_slice(&star_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let sun_vertices = Self::build_sun_quad_default();
        let sun_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sun Vertex Buffer"),
            contents: bytemuck::cast_slice(&sun_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let moon_vertices = Self::build_moon_phases_default();
        let moon_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Moon Vertex Buffer"),
            contents: bytemuck::cast_slice(&moon_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let (top_vertices, top_indices) = Self::build_sky_disc(16.0);
        let top_sky_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Top Sky Vertex Buffer"),
            contents: bytemuck::cast_slice(&top_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let (bottom_vertices, _) = Self::build_sky_disc(-16.0);
        let bottom_sky_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Bottom Sky Vertex Buffer"),
            contents: bytemuck::cast_slice(&bottom_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let (sunrise_vertices, sunrise_indices) = Self::build_sunrise_fan();
        let sunrise_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sunrise Vertex Buffer"),
            contents: bytemuck::cast_slice(&sunrise_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let sky_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sky Disc Index Buffer"),
            contents: bytemuck::cast_slice(&top_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let sunrise_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sunrise Index Buffer"),
            contents: bytemuck::cast_slice(&sunrise_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let star_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stars Index Buffer"),
            contents: bytemuck::cast_slice(&star_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let celestial_indices = vec![0u32, 1, 2, 0, 2, 3];
        let celestial_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Celestial Index Buffer"),
            contents: bytemuck::cast_slice(&celestial_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sky Uniform Buffer"),
            size: (std::mem::size_of::<SkyUniforms>() * 6) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sky Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(
                        std::mem::size_of::<SkyUniforms>() as u64
                    ),
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sky Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(std::mem::size_of::<SkyUniforms>() as u64),
                }),
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sky Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sky_shader.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sky Pipeline Layout"),
            bind_group_layouts: &[
                camera_bind_group_layout,
                texture_bind_group_layout,
                &bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3, 1 => Float32x2, 2 => Float32x3, 3 => Float32x4, 4 =>
                Float32x2
            ],
        };
        let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sky Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertex_layout.clone()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_untextured",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(java_no_depth_state_with_attached_target(depth_format)),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let sunrise_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sunrise Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertex_layout.clone()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_untextured",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(java_no_depth_state_with_attached_target(depth_format)),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let stars_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Stars Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertex_layout.clone()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_untextured",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::Zero,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(java_no_depth_state_with_attached_target(depth_format)),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let celestial_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Celestial Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertex_layout.clone()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_textured",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::Zero,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(java_no_depth_state_with_attached_target(depth_format)),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let sky_camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sky Camera Buffer"),
            size: std::mem::size_of::<SkyCameraUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sky_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sky Camera Bind Group"),
            layout: camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sky_camera_buffer.as_entire_binding(),
            }],
        });
        Self {
            star_buffer,
            star_index_count: star_indices.len() as u32,
            sun_buffer,
            moon_buffer,
            top_sky_buffer,
            bottom_sky_buffer,
            sunrise_buffer,
            sky_index_buffer,
            sunrise_index_buffer,
            star_index_buffer,
            celestial_index_buffer,
            sky_pipeline,
            sunrise_pipeline,
            stars_pipeline,
            celestial_pipeline,
            uniform_buffer,
            bind_group,
            sky_camera_buffer,
            sky_camera_bind_group,
            celestial_buffers_updated: false,
        }
    }
    pub fn prepare_uniforms(
        &self,
        queue: &wgpu::Queue,
        translation_free_view_proj: glam::Mat4,
        state: &SkyRenderState,
    ) {
        let camera_uniform = SkyCameraUniform {
            view_proj: translation_free_view_proj.to_cols_array_2d(),
            lightmap_factors: [1.0, 1.0, 0.0, 0.0],
        };
        queue.write_buffer(
            &self.sky_camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );
        let base_rot = glam::Mat4::from_rotation_y(-90.0f32.to_radians());
        let rot = base_rot * glam::Mat4::from_rotation_x(state.sun_angle);
        let moon_rot = base_rot * glam::Mat4::from_rotation_x(state.moon_angle);
        let star_rot = base_rot * glam::Mat4::from_rotation_x(state.star_angle);
        let sunrise_color = state.sunrise_and_sunset_color;
        let ss_alpha = sunrise_color[3];
        let sunrise_angle = if state.sun_angle.sin() < 0.0 {
            180.0f32.to_radians()
        } else {
            0.0
        };
        let sunrise_rot = glam::Mat4::from_rotation_x(90.0f32.to_radians())
            * glam::Mat4::from_rotation_z(sunrise_angle + 90.0f32.to_radians())
            * glam::Mat4::from_scale(glam::Vec3::new(1.0, 1.0, ss_alpha));
        let fog_color = [
            state.fog_color[0],
            state.fog_color[1],
            state.fog_color[2],
            1.0,
        ];
        let sky_fog_params = [0.0, state.view_distance_blocks, 1.0, 0.0];
        let no_fog_params = [0.0, 0.0, 0.0, 0.0];
        let uniforms = [
            SkyUniforms {
                model_matrix: glam::Mat4::IDENTITY.to_cols_array_2d(),
                color_modifier: [
                    state.sky_color[0],
                    state.sky_color[1],
                    state.sky_color[2],
                    1.0,
                ],
                fog_color,
                fog_params: sky_fog_params,
                _padding: [[0.0; 4]; 9],
            },
            SkyUniforms {
                model_matrix: sunrise_rot.to_cols_array_2d(),
                color_modifier: sunrise_color,
                fog_color,
                fog_params: no_fog_params,
                _padding: [[0.0; 4]; 9],
            },
            SkyUniforms {
                model_matrix: rot.to_cols_array_2d(),
                color_modifier: [1.0, 1.0, 1.0, state.rain_brightness],
                fog_color,
                fog_params: no_fog_params,
                _padding: [[0.0; 4]; 9],
            },
            SkyUniforms {
                model_matrix: moon_rot.to_cols_array_2d(),
                color_modifier: [1.0, 1.0, 1.0, state.rain_brightness],
                fog_color,
                fog_params: no_fog_params,
                _padding: [[0.0; 4]; 9],
            },
            SkyUniforms {
                model_matrix: star_rot.to_cols_array_2d(),
                color_modifier: [
                    state.star_brightness,
                    state.star_brightness,
                    state.star_brightness,
                    state.star_brightness,
                ],
                fog_color,
                fog_params: no_fog_params,
                _padding: [[0.0; 4]; 9],
            },
            SkyUniforms {
                model_matrix: glam::Mat4::from_translation(glam::Vec3::new(0.0, 12.0, 0.0))
                    .to_cols_array_2d(),
                color_modifier: [0.0, 0.0, 0.0, 1.0],
                fog_color,
                fog_params: sky_fog_params,
                _padding: [[0.0; 4]; 9],
            },
        ];
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&uniforms));
    }
    pub fn draw_pass<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        atlas_bind_group: &'a wgpu::BindGroup,
        state: &SkyRenderState,
    ) {
        let stride = std::mem::size_of::<SkyUniforms>() as u32;
        render_pass.set_pipeline(&self.sky_pipeline);
        render_pass.set_bind_group(0, &self.sky_camera_bind_group, &[]);
        render_pass.set_bind_group(1, atlas_bind_group, &[]);
        render_pass.set_bind_group(2, &self.bind_group, &[0 * stride]);
        render_pass.set_vertex_buffer(0, self.top_sky_buffer.slice(..));
        render_pass.set_index_buffer(self.sky_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..24, 0, 0..1);
        if state.is_sunrise_visible() {
            render_pass.set_pipeline(&self.sunrise_pipeline);
            render_pass.set_bind_group(2, &self.bind_group, &[1 * stride]);
            render_pass.set_vertex_buffer(0, self.sunrise_buffer.slice(..));
            render_pass.set_index_buffer(
                self.sunrise_index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..48, 0, 0..1);
        }
        render_pass.set_pipeline(&self.celestial_pipeline);
        render_pass.set_bind_group(2, &self.bind_group, &[2 * stride]);
        render_pass.set_vertex_buffer(0, self.sun_buffer.slice(..));
        render_pass.set_index_buffer(
            self.celestial_index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..6, 0, 0..1);
        let phase = state.moon_phase % 8;
        render_pass.set_bind_group(2, &self.bind_group, &[3 * stride]);
        render_pass.set_vertex_buffer(
            0,
            self.moon_buffer
                .slice((phase * 4 * std::mem::size_of::<Vertex>() as u32) as wgpu::BufferAddress..),
        );
        render_pass.draw_indexed(0..6, 0, 0..1);
        if state.are_stars_visible() {
            render_pass.set_pipeline(&self.stars_pipeline);
            render_pass.set_bind_group(2, &self.bind_group, &[4 * stride]);
            render_pass.set_vertex_buffer(0, self.star_buffer.slice(..));
            render_pass
                .set_index_buffer(self.star_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.star_index_count, 0, 0..1);
        }
        if state.should_render_dark_disc {
            render_pass.set_pipeline(&self.sky_pipeline);
            render_pass.set_bind_group(2, &self.bind_group, &[5 * stride]);
            render_pass.set_vertex_buffer(0, self.bottom_sky_buffer.slice(..));
            render_pass
                .set_index_buffer(self.sky_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..24, 0, 0..1);
        }
    }
    fn build_stars() -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut rng = JavaRandom::new(10842);
        let uv_rect = crate::render::texture::SpriteUvRect {
            u0: 0.0,
            v0: 0.0,
            u1: 0.0,
            v1: 0.0,
        };
        let color = [1.0, 1.0, 1.0, 1.0];
        let light = [1.0, 1.0];
        for _ in 0..1500 {
            let x = rng.next_float() * 2.0 - 1.0;
            let y = rng.next_float() * 2.0 - 1.0;
            let z = rng.next_float() * 2.0 - 1.0;
            let star_size = 0.15 + rng.next_float() * 0.1;
            let len_sq = x * x + y * y + z * z;
            if len_sq > 0.01 && len_sq < 1.0 {
                let center = glam::Vec3::new(x, y, z).normalize() * 100.0;
                let z_rot = rng.next_double() as f32 * std::f32::consts::PI * 2.0;
                let z_axis = -center.normalize();
                let up_ref = glam::Vec3::new(0.0, 1.0, 0.0);
                let x_axis = if z_axis.y.abs() > 0.99 {
                    glam::Vec3::new(1.0, 0.0, 0.0).cross(z_axis).normalize()
                } else {
                    up_ref.cross(z_axis).normalize()
                };
                let y_axis = z_axis.cross(x_axis).normalize();
                let cos_z = (-z_rot).cos();
                let sin_z = (-z_rot).sin();
                let x_rot = x_axis * cos_z + y_axis * sin_z;
                let y_rot = -x_axis * sin_z + y_axis * cos_z;
                let base = vertices.len() as u32;
                let v0 = center + (x_rot * star_size) - (y_rot * star_size);
                let v1 = center + (x_rot * star_size) + (y_rot * star_size);
                let v2 = center - (x_rot * star_size) + (y_rot * star_size);
                let v3 = center - (x_rot * star_size) - (y_rot * star_size);
                let normal = [-center.x, -center.y, -center.z];
                vertices.push(Vertex {
                    position: v0.to_array(),
                    tex_coords: [uv_rect.u0, uv_rect.v1],
                    normal,
                    color,
                    light,
                });
                vertices.push(Vertex {
                    position: v1.to_array(),
                    tex_coords: [uv_rect.u1, uv_rect.v1],
                    normal,
                    color,
                    light,
                });
                vertices.push(Vertex {
                    position: v2.to_array(),
                    tex_coords: [uv_rect.u1, uv_rect.v0],
                    normal,
                    color,
                    light,
                });
                vertices.push(Vertex {
                    position: v3.to_array(),
                    tex_coords: [uv_rect.u0, uv_rect.v0],
                    normal,
                    color,
                    light,
                });
                indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
        (vertices, indices)
    }
    fn build_celestial_quad(
        sprite: &crate::render::texture::SpriteUvRect,
        size: f32,
    ) -> Vec<Vertex> {
        let color = [1.0, 1.0, 1.0, 1.0];
        let light = [1.0, 1.0];
        vec![
            Vertex {
                position: [-size, 100.0, -size],
                tex_coords: [sprite.u0, sprite.v0],
                normal: [0.0, -1.0, 0.0],
                color,
                light,
            },
            Vertex {
                position: [size, 100.0, -size],
                tex_coords: [sprite.u1, sprite.v0],
                normal: [0.0, -1.0, 0.0],
                color,
                light,
            },
            Vertex {
                position: [size, 100.0, size],
                tex_coords: [sprite.u1, sprite.v1],
                normal: [0.0, -1.0, 0.0],
                color,
                light,
            },
            Vertex {
                position: [-size, 100.0, size],
                tex_coords: [sprite.u0, sprite.v1],
                normal: [0.0, -1.0, 0.0],
                color,
                light,
            },
        ]
    }
    fn build_sun_quad_default() -> Vec<Vertex> {
        let sprite = crate::render::texture::SpriteUvRect {
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        };
        Self::build_celestial_quad(&sprite, 30.0)
    }
    pub fn update_celestial_buffers(&mut self, device: &wgpu::Device, atlas: &TextureAtlas) {
        log::debug!(
            "[SKY_DEBUG] update_celestial_buffers called, atlas has {} sprites",
            atlas.sprites.len()
        );
        let sun_vertices = {
            let sun_loc = ResourceLocation::from_string("minecraft:environment/celestial/sun");
            let sprite = atlas
                .sprites
                .get(&sun_loc)
                .copied()
                .unwrap_or_else(|| {
                    log::debug!(
                        "[SKY_DEBUG] WARNING: Sun texture not found in atlas! Using fallback UV (0,0)-(1,1)"
                    );
                    crate::render::texture::SpriteUvRect {
                        u0: 0.0,
                        v0: 0.0,
                        u1: 1.0,
                        v1: 1.0,
                    }
                });
            log::debug!(
                "[SKY_DEBUG] Sun sprite UV: u0={:.4}, v0={:.4}, u1={:.4}, v1={:.4}",
                sprite.u0,
                sprite.v0,
                sprite.u1,
                sprite.v1
            );
            Self::build_celestial_quad(&sprite, 30.0)
        };
        self.sun_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sun Vertex Buffer"),
            contents: bytemuck::cast_slice(&sun_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let moon_phase_names = [
            "full_moon",
            "waning_gibbous",
            "third_quarter",
            "waning_crescent",
            "new_moon",
            "waxing_crescent",
            "first_quarter",
            "waxing_gibbous",
        ];
        let has_all_moon_sprites = moon_phase_names.iter().all(|name| {
            atlas
                .sprites
                .contains_key(&ResourceLocation::from_string(&format!(
                    "minecraft:environment/celestial/moon/{}",
                    name
                )))
        });
        let moon_vertices = if has_all_moon_sprites {
            let mut vertices = Vec::new();
            for name in &moon_phase_names {
                let loc = ResourceLocation::from_string(&format!(
                    "minecraft:environment/celestial/moon/{}",
                    name
                ));
                let phase_rect = atlas.sprites.get(&loc).copied().unwrap_or(
                    crate::render::texture::SpriteUvRect {
                        u0: 0.0,
                        v0: 0.0,
                        u1: 1.0,
                        v1: 1.0,
                    },
                );
                let size = 20.0;
                let color = [1.0, 1.0, 1.0, 1.0];
                let light = [1.0, 1.0];
                vertices.push(Vertex {
                    position: [-size, 100.0, -size],
                    tex_coords: [phase_rect.u1, phase_rect.v1],
                    normal: [0.0, -1.0, 0.0],
                    color,
                    light,
                });
                vertices.push(Vertex {
                    position: [size, 100.0, -size],
                    tex_coords: [phase_rect.u0, phase_rect.v1],
                    normal: [0.0, -1.0, 0.0],
                    color,
                    light,
                });
                vertices.push(Vertex {
                    position: [size, 100.0, size],
                    tex_coords: [phase_rect.u0, phase_rect.v0],
                    normal: [0.0, -1.0, 0.0],
                    color,
                    light,
                });
                vertices.push(Vertex {
                    position: [-size, 100.0, size],
                    tex_coords: [phase_rect.u1, phase_rect.v0],
                    normal: [0.0, -1.0, 0.0],
                    color,
                    light,
                });
            }
            vertices
        } else {
            Self::build_moon_phases_default()
        };
        self.moon_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Moon Vertex Buffer"),
            contents: bytemuck::cast_slice(&moon_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.celestial_buffers_updated = true;
    }
    fn build_moon_phases_default() -> Vec<Vertex> {
        let mut vertices = Vec::new();
        let sprite = crate::render::texture::SpriteUvRect {
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        };
        let width = (sprite.u1 - sprite.u0) / 4.0;
        let height = (sprite.v1 - sprite.v0) / 2.0;
        for phase in 0..8 {
            let px = phase % 4;
            let py = phase / 4;
            let phase_rect = crate::render::texture::SpriteUvRect {
                u0: sprite.u0 + px as f32 * width,
                v0: sprite.v0 + py as f32 * height,
                u1: sprite.u0 + (px + 1) as f32 * width,
                v1: sprite.v0 + (py + 1) as f32 * height,
            };
            let size = 20.0;
            let color = [1.0, 1.0, 1.0, 1.0];
            let light = [1.0, 1.0];
            vertices.push(Vertex {
                position: [-size, 100.0, -size],
                tex_coords: [phase_rect.u1, phase_rect.v1],
                normal: [0.0, -1.0, 0.0],
                color,
                light,
            });
            vertices.push(Vertex {
                position: [size, 100.0, -size],
                tex_coords: [phase_rect.u0, phase_rect.v1],
                normal: [0.0, -1.0, 0.0],
                color,
                light,
            });
            vertices.push(Vertex {
                position: [size, 100.0, size],
                tex_coords: [phase_rect.u0, phase_rect.v0],
                normal: [0.0, -1.0, 0.0],
                color,
                light,
            });
            vertices.push(Vertex {
                position: [-size, 100.0, size],
                tex_coords: [phase_rect.u1, phase_rect.v0],
                normal: [0.0, -1.0, 0.0],
                color,
                light,
            });
        }
        vertices
    }
    fn build_sky_disc(yy: f32) -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let x = yy.signum() * 512.0;
        let color = [1.0, 1.0, 1.0, 1.0];
        let light = [1.0, 1.0];
        let uv_rect = [0.0, 0.0];
        let normal = [0.0, -yy.signum(), 0.0];
        vertices.push(Vertex {
            position: [0.0, yy, 0.0],
            tex_coords: uv_rect,
            normal,
            color,
            light,
        });
        for i in (-180..=180).step_by(45) {
            let angle = (i as f32).to_radians();
            vertices.push(Vertex {
                position: [x * angle.cos(), yy, 512.0 * angle.sin()],
                tex_coords: uv_rect,
                normal,
                color,
                light,
            });
        }
        for i in 1..=8 {
            indices.extend_from_slice(&[0, i, i + 1]);
        }
        (vertices, indices)
    }
    fn build_sunrise_fan() -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let light = [1.0, 1.0];
        let uv = [0.0, 0.0];
        let normal = [0.0, 1.0, 0.0];
        vertices.push(Vertex {
            position: [0.0, 100.0, 0.0],
            tex_coords: uv,
            normal,
            color: [1.0, 1.0, 1.0, 1.0],
            light,
        });
        for i in 0..=16 {
            let angle = (i as f32) * std::f32::consts::TAU / 16.0;
            vertices.push(Vertex {
                position: [
                    angle.sin() * 120.0,
                    angle.cos() * 120.0,
                    -angle.cos() * 40.0,
                ],
                tex_coords: uv,
                normal,
                color: [1.0, 1.0, 1.0, 0.0],
                light,
            });
        }
        for i in 1..=16 {
            indices.extend_from_slice(&[0, i, i + 1]);
        }
        (vertices, indices)
    }
}
