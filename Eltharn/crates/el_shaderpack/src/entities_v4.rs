use std::ops::Range;

pub const MAX_ENTITY_DRAWS: usize = 4096;
pub const MAX_ENTITY_MESH_BYTES: usize = 8 * 1024 * 1024;
const STRIDE: u64 = 256;
const UNIFORM_BYTES: u64 = 192;
const ARENA_ALLOCATION_INCREMENT: u64 = 262_144;
pub const RESERVATION: u64 = MAX_ENTITY_DRAWS as u64 * STRIDE * 3
    + (MAX_ENTITY_MESH_BYTES as u64 + 2 * ARENA_ALLOCATION_INCREMENT) * 2 * 2 * 2;

#[derive(Clone, Copy, Debug)]
pub struct EntityMatrices {
    pub camera_clip: [[f32; 4]; 4],
    pub light_clip: [[f32; 4]; 4],
    pub world_normal: [[f32; 4]; 4],
}

pub struct EntityDraw<'a> {
    pub vertices: &'a wgpu::Buffer,
    pub vertex_range: Range<u64>,
    pub indices: &'a wgpu::Buffer,
    pub index_range: Range<u64>,
    pub index_count: u32,
    pub texture: &'a wgpu::BindGroup,
    pub cull: bool,
    pub geometry: bool,
    pub shadow: bool,
}

pub fn check_device(limits: &wgpu::Limits) -> Result<(), String> {
    check_device_with_light(limits, false)
}

pub fn check_device_with_light(limits: &wgpu::Limits, light: bool) -> Result<(), String> {
    if limits.min_uniform_buffer_offset_alignment == 0
        || STRIDE % u64::from(limits.min_uniform_buffer_offset_alignment) != 0
        || limits.max_buffer_size < STRIDE * MAX_ENTITY_DRAWS as u64
        || limits.max_buffer_size < MAX_ENTITY_MESH_BYTES as u64 + ARENA_ALLOCATION_INCREMENT
        || limits.max_uniform_buffer_binding_size
            < (UNIFORM_BYTES + if light { 16 } else { 0 }) as u32
        || limits.max_dynamic_uniform_buffers_per_pipeline_layout < 1
        || limits.max_bind_groups < 2
        || limits.max_vertex_buffers < 1
        || limits.max_vertex_attributes < 5
        || limits.max_vertex_buffer_array_stride < 56
    {
        return Err("device cannot bind bounded cutout entity replay".into());
    }
    Ok(())
}

pub struct EntityReplay {
    geometry: [wgpu::RenderPipeline; 2],
    shadow: [wgpu::RenderPipeline; 2],
    slots: Vec<(wgpu::Buffer, wgpu::BindGroup)>,
    active: usize,
    count: usize,
    staging: Vec<u8>,
    light: bool,
}

impl EntityReplay {
    pub fn new(
        device: &wgpu::Device,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, String> {
        Self::new_with_light(device, texture_layout, false)
    }

    pub fn light_enabled(&self) -> bool {
        self.light
    }

    pub fn new_with_light(
        device: &wgpu::Device,
        texture_layout: &wgpu::BindGroupLayout,
        light: bool,
    ) -> Result<Self, String> {
        check_device_with_light(&device.limits(), light)?;
        let uniform_bytes = UNIFORM_BYTES + if light { 16 } else { 0 };
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("entity replay matrices"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: std::num::NonZeroU64::new(uniform_bytes),
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cutout entity replay"),
            bind_group_layouts: &[&uniform_layout, texture_layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cutout entity replay"),
            source: wgpu::ShaderSource::Wgsl(shader_source(light).into()),
        });
        let mut targets = vec![
            Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
        ];
        if light {
            targets.push(Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rg16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }));
        }
        let pipeline = |shadow, cull| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(if shadow { "cutout entity sun caster" } else { "cutout entity material" }),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: if shadow { "vs_shadow" } else { "vs_geometry" },
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 56,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x2,2=>Float32x3,3=>Float32x4,4=>Float32x2],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: if shadow { "fs_shadow" } else { "fs_geometry" },
                    targets: if shadow { &[] } else { &targets },
                }),
                primitive: wgpu::PrimitiveState { cull_mode: if cull { Some(wgpu::Face::Back) } else { None }, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: shadow,
                    depth_compare: if shadow { wgpu::CompareFunction::LessEqual } else { wgpu::CompareFunction::GreaterEqual },
                    stencil: Default::default(), bias: Default::default(),
                }),
                multisample: Default::default(), multiview: None,
            })
        };
        let slots = (0..3)
            .map(|_| {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("bounded entity replay matrices"),
                    size: STRIDE * MAX_ENTITY_DRAWS as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("entity replay matrices"),
                    layout: &uniform_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &buffer,
                            offset: 0,
                            size: std::num::NonZeroU64::new(uniform_bytes),
                        }),
                    }],
                });
                (buffer, group)
            })
            .collect();
        Ok(Self {
            geometry: [pipeline(false, false), pipeline(false, true)],
            shadow: [pipeline(true, false), pipeline(true, true)],
            slots,
            active: 0,
            count: 0,
            staging: Vec::new(),
            light,
        })
    }

    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        matrices: &[EntityMatrices],
    ) -> Result<(), String> {
        self.prepare_inner(queue, matrices, None)
    }

    pub fn prepare_with_light(
        &mut self,
        queue: &wgpu::Queue,
        matrices: &[EntityMatrices],
        light_uv: &[[f32; 2]],
    ) -> Result<(), String> {
        self.prepare_inner(queue, matrices, Some(light_uv))
    }

    fn prepare_inner(
        &mut self,
        queue: &wgpu::Queue,
        matrices: &[EntityMatrices],
        light_uv: Option<&[[f32; 2]]>,
    ) -> Result<(), String> {
        self.count = 0;
        if self.light
            && light_uv.is_none_or(|uv| {
                uv.len() != matrices.len()
                    || uv
                        .iter()
                        .flatten()
                        .any(|v| !v.is_finite() || !(0.0..=15.0 / 16.0).contains(v))
            })
        {
            return Err("entity effective block/sky light missing or invalid".into());
        }
        if matrices.len() > MAX_ENTITY_DRAWS {
            return Err("cutout entity draw limit exceeded (4096)".into());
        }
        self.staging.clear();
        self.staging.resize(matrices.len() * STRIDE as usize, 0);
        for (index, m) in matrices.iter().enumerate() {
            let values = [m.camera_clip, m.light_clip, m.world_normal];
            for (component, value) in values.iter().flatten().flatten().enumerate() {
                if !value.is_finite() {
                    return Err("nonfinite entity replay matrix".into());
                }
                let offset = index * STRIDE as usize + component * 4;
                self.staging[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
            }
            if self.light {
                for (component, value) in light_uv.unwrap()[index].iter().enumerate() {
                    let start = index * STRIDE as usize + UNIFORM_BYTES as usize + component * 4;
                    self.staging[start..start + 4].copy_from_slice(&value.to_ne_bytes());
                }
            }
        }
        self.active = (self.active + 1) % self.slots.len();
        if !self.staging.is_empty() {
            queue.write_buffer(&self.slots[self.active].0, 0, &self.staging);
        }
        self.count = matrices.len();
        Ok(())
    }

    fn validate(&self, draws: &[EntityDraw<'_>]) -> Result<(), String> {
        if draws.len() != self.count {
            return Err("entity matrices/draw count mismatch".into());
        }
        for draw in draws {
            if draw.vertex_range.start >= draw.vertex_range.end
                || draw.vertex_range.end > draw.vertices.size()
                || draw.vertex_range.start % 56 != 0
                || (draw.vertex_range.end - draw.vertex_range.start) % 56 != 0
                || draw.index_range.start % 4 != 0
                || draw.index_range.end > draw.indices.size()
                || draw
                    .index_range
                    .start
                    .checked_add(u64::from(draw.index_count) * 4)
                    != Some(draw.index_range.end)
            {
                return Err("invalid entity replay arena range".into());
            }
        }
        Ok(())
    }

    fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        draws: &'a [EntityDraw<'a>],
        shadow: bool,
    ) {
        for (index, draw) in draws.iter().enumerate() {
            if !(if shadow { draw.shadow } else { draw.geometry }) {
                continue;
            }
            pass.set_pipeline(
                &(if shadow { &self.shadow } else { &self.geometry })[usize::from(draw.cull)],
            );
            pass.set_bind_group(
                0,
                &self.slots[self.active].1,
                &[(index as u32) * STRIDE as u32],
            );
            pass.set_bind_group(1, draw.texture, &[]);
            pass.set_vertex_buffer(0, draw.vertices.slice(draw.vertex_range.clone()));
            pass.set_index_buffer(
                draw.indices.slice(draw.index_range.clone()),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(0..draw.index_count, 0, 0..1);
        }
    }

    pub fn encode_geometry(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        normal: &wgpu::TextureView,
        albedo: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        draws: &[EntityDraw<'_>],
    ) -> Result<(), String> {
        self.encode_geometry_with_light(encoder, normal, albedo, None, depth, draws)
    }

    pub fn encode_geometry_with_light(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        normal: &wgpu::TextureView,
        albedo: &wgpu::TextureView,
        light: Option<&wgpu::TextureView>,
        depth: &wgpu::TextureView,
        draws: &[EntityDraw<'_>],
    ) -> Result<(), String> {
        if self.light && light.is_none() {
            return Err("entity block/sky light target absent".into());
        }
        self.validate(draws)?;
        let attachment = |view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })
        };
        let mut attachments = vec![attachment(normal), attachment(albedo)];
        if self.light {
            attachments.push(attachment(light.unwrap()));
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("cutout entity normal/albedo replay"),
            color_attachments: &attachments,
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
        });
        self.draw(&mut pass, draws, false);
        Ok(())
    }

    pub fn encode_shadow(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        depth: &wgpu::TextureView,
        draws: &[EntityDraw<'_>],
    ) -> Result<(), String> {
        self.validate(draws)?;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("cutout entity sun casters"),
            color_attachments: &[],
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
        });
        self.draw(&mut pass, draws, true);
        Ok(())
    }
}

const SHADER: &str = r#"
struct Matrices { camera_clip: mat4x4<f32>, light_clip: mat4x4<f32>, world_normal: mat4x4<f32> };
@group(0) @binding(0) var<uniform> m: Matrices;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var tex_sampler: sampler;
struct Vertex { @location(0) position: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) normal: vec3<f32>, @location(3) color: vec4<f32>, @location(4) light: vec2<f32> };
struct Varying { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) normal: vec3<f32>, @location(2) color: vec4<f32> };
@vertex fn vs_geometry(v: Vertex) -> Varying {
    return Varying(m.camera_clip * vec4<f32>(v.position, 1.), v.uv, (m.world_normal * vec4<f32>(v.normal, 0.)).xyz, v.color);
}
@vertex fn vs_shadow(v: Vertex) -> Varying {
    return Varying(m.light_clip * vec4<f32>(v.position, 1.), v.uv, v.normal, v.color);
}
struct Material { @location(0) normal: vec4<f32>, @location(1) albedo: vec4<f32> };
@fragment fn fs_geometry(v: Varying, @builtin(front_facing) front: bool) -> Material {
    let sampled = textureSample(tex, tex_sampler, v.uv);
    if sampled.a < 0.1 { discard; }
    let n = select(-v.normal, v.normal, front);
    var encoded = vec3<f32>(0.5, 1., 0.5);
    if dot(n, n) >= 1e-6 { encoded = normalize(n) * 0.5 + vec3<f32>(0.5); }
    return Material(vec4<f32>(encoded, 1.), vec4<f32>(sampled.rgb * v.color.rgb, 1.));
}
@fragment fn fs_shadow(v: Varying) { if textureSample(tex, tex_sampler, v.uv).a < 0.1 { discard; } }
"#;

fn shader_source(light: bool) -> String {
    if !light {
        return SHADER.into();
    }
    SHADER
        .replace(
            "world_normal: mat4x4<f32> };",
            "world_normal: mat4x4<f32>, light_uv: vec4<f32> };",
        )
        .replace(
            "@location(2) color: vec4<f32> };",
            "@location(2) color: vec4<f32>, @location(3) light_uv: vec2<f32> };",
        )
        .replace(
            "v.normal, 0.)).xyz, v.color)",
            "v.normal, 0.)).xyz, v.color, m.light_uv.xy)",
        )
        .replace(
            "v.uv, v.normal, v.color)",
            "v.uv, v.normal, v.color, m.light_uv.xy)",
        )
        .replace(
            "@location(1) albedo: vec4<f32> };",
            "@location(1) albedo: vec4<f32>, @location(2) light_uv: vec2<f32> };",
        )
        .replace(
            "vec4<f32>(sampled.rgb * v.color.rgb, 1.))",
            "vec4<f32>(sampled.rgb * v.color.rgb, 1.), v.light_uv)",
        )
}
