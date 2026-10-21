use glam::{Mat4, Vec3};
pub const TERRAIN_VERTEX_STRIDE: u64 = 56;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowConfig {
    pub resolution: u32,
    pub distance: f32,
    pub depth_bias: f32,
}
impl ShadowConfig {
    pub fn validate(self) -> Result<(), String> {
        if !(256..=4096).contains(&self.resolution)
            || !self.resolution.is_power_of_two()
            || !self.distance.is_finite()
            || !(16.0..=256.0).contains(&self.distance)
            || !self.depth_bias.is_finite()
            || !(0.0..=0.02).contains(&self.depth_bias)
        {
            return Err(
                "terrain shadow requires resolution power-of-two256..4096, distance16..256, depth_bias0..0.02"
                    .into(),
            );
        }
        Ok(())
    }
    pub fn bytes(self) -> u64 {
        u64::from(self.resolution).pow(2) * 4 + 96
    }
}
#[derive(Clone, Debug)]
pub struct ShadowFrame {
    pub view_projection: [[f32; 4]; 4],
    pub light_direction: [f32; 4],
    pub map: [f32; 4],
}
impl ShadowFrame {
    pub fn sun(camera: [f32; 3], sun_angle: f32, config: ShadowConfig) -> Result<Self, String> {
        config.validate()?;
        if camera.iter().any(|v| !v.is_finite()) || !sun_angle.is_finite() {
            return Err("nonfinite shadow camera/sun".into());
        }
        let direction = (Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_2)
            * Mat4::from_rotation_x(sun_angle))
        .transform_vector3(Vec3::Y)
        .normalize();
        let up = if direction.dot(Vec3::Y).abs() > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        let rotation = Mat4::look_to_rh(Vec3::ZERO, -direction, up);
        let mut light_center = rotation.transform_point3(Vec3::from_array(camera));
        let texel = 2.0 * config.distance / config.resolution as f32;
        light_center.x = (light_center.x / texel).round() * texel;
        light_center.y = (light_center.y / texel).round() * texel;
        let center = rotation.inverse().transform_point3(light_center);
        let view = Mat4::look_to_rh(center + direction * (2.0 * config.distance), -direction, up);
        let d = config.distance;
        let projection = Mat4::orthographic_rh(-d, d, -d, d, 0.0, 4.0 * d);
        Ok(Self {
            view_projection: (projection * view).to_cols_array_2d(),
            light_direction: [
                direction.x,
                direction.y,
                direction.z,
                if direction.y > 0.0 { 1.0 } else { 0.0 },
            ],
            map: [
                config.resolution as f32,
                1.0 / config.resolution as f32,
                config.depth_bias,
                d,
            ],
        })
    }
    pub fn intersects_aabb(&self, min: [f32; 3], max: [f32; 3]) -> bool {
        let rows = Mat4::from_cols_array_2d(&self.view_projection).transpose();
        let planes = [
            rows.w_axis + rows.x_axis,
            rows.w_axis - rows.x_axis,
            rows.w_axis + rows.y_axis,
            rows.w_axis - rows.y_axis,
            rows.z_axis,
            rows.w_axis - rows.z_axis,
        ];
        planes.into_iter().all(|p| {
            let point = Vec3::new(
                if p.x >= 0.0 { max[0] } else { min[0] },
                if p.y >= 0.0 { max[1] } else { min[1] },
                if p.z >= 0.0 { max[2] } else { min[2] },
            );
            p.truncate().dot(point) + p.w >= 0.0
        })
    }
    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        let values = self
            .view_projection
            .iter()
            .flatten()
            .chain(self.light_direction.iter())
            .chain(self.map.iter());
        let mut bytes = Vec::with_capacity(96);
        for value in values {
            if !value.is_finite() {
                return Err("nonfinite shadow uniform".into());
            }
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        Ok(bytes)
    }
}
#[cfg(feature = "gpu")]
pub struct TerrainShadowDraw<'a> {
    pub vertices: &'a wgpu::Buffer,
    pub indices: &'a wgpu::Buffer,
    pub first_index: u32,
    pub index_count: u32,
    pub base_vertex: i32,
}
#[cfg(feature = "gpu")]
pub(crate) struct ShadowGpu {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub uniform: wgpu::Buffer,
    config: ShadowConfig,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    pub ready: bool,
}
#[cfg(feature = "gpu")]
impl ShadowGpu {
    pub fn new(device: &wgpu::Device, config: ShadowConfig) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("terrain sun shadow"),
            size: wgpu::Extent3d {
                width: config.resolution,
                height: config.resolution,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terrain shadow matrix"),
            size: 96,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terrain shadow caster"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(96),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("terrain shadow caster"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terrain shadow caster"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/terrain_shadow.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terrain shadow caster"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: TERRAIN_VERTEX_STRIDE,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 12,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 32,
                            shader_location: 3,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs",
                targets: &[],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
        });
        Self {
            texture,
            view,
            uniform,
            config,
            layout,
            pipeline,
            ready: false,
        }
    }
    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &ShadowFrame,
        atlas: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        draws: &[TerrainShadowDraw<'_>],
    ) -> Result<(), String> {
        self.ready = false;
        if frame.map
            != [
                self.config.resolution as f32,
                1.0 / self.config.resolution as f32,
                self.config.depth_bias,
                self.config.distance,
            ]
        {
            return Err("shadow frame/config mismatch".into());
        }
        for draw in draws {
            if draw.base_vertex < 0
                || draw.first_index.checked_add(draw.index_count).is_none()
                || u64::from(draw.first_index + draw.index_count) * 4 > draw.indices.size()
                || (draw.base_vertex as u64) * TERRAIN_VERTEX_STRIDE >= draw.vertices.size()
            {
                return Err("invalid shadow draw range".into());
            }
        }
        queue.write_buffer(&self.uniform, 0, &frame.bytes()?);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain shadow caster"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(atlas),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("terrain sun shadow"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        for draw in draws {
            pass.set_vertex_buffer(
                0,
                draw.vertices
                    .slice((draw.base_vertex as u64) * TERRAIN_VERTEX_STRIDE..),
            );
            pass.set_index_buffer(draw.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(
                draw.first_index..draw.first_index + draw.index_count,
                0,
                0..1,
            );
        }
        self.ready = true;
        Ok(())
    }
}
