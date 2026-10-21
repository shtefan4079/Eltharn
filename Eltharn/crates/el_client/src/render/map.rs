use crate::gameplay::entity::remote::state::RemoteEntityRenderState;
use crate::render::baked_model::Vertex;
use crate::world::client_level::ClientLevel;
use el_core::item::{DataComponent, ItemStack};
use el_math::block_pos::BlockPos;
use el_nbt::tag::Tag;
use el_protocol_backend::BackendMapDecoration;
use glam::{DVec3, Mat4, Vec3};
use std::collections::HashMap;
pub const MAP_SIZE: usize = 128;
pub const MAP_PIXELS: usize = MAP_SIZE * MAP_SIZE;
#[derive(Debug, Clone)]
pub struct MapData {
    pub colors: [u8; MAP_PIXELS],
    pub revision: u64,
    pub scale: u8,
    pub locked: bool,
    pub tracking_position: bool,
    pub decorations: Vec<BackendMapDecoration>,
}
impl Default for MapData {
    fn default() -> Self {
        Self {
            colors: [0; MAP_PIXELS],
            revision: 0,
            scale: 0,
            locked: false,
            tracking_position: false,
            decorations: Vec::new(),
        }
    }
}
const MATERIAL_COLORS: [u32; 64] = [
    0, 8368696, 16247203, 13092807, 16711680, 10526975, 10987431, 31744, 16777215, 10791096,
    9923917, 7368816, 4210943, 9402184, 16776437, 14188339, 11685080, 6724056, 15066419, 8375321,
    15892389, 5000268, 10066329, 5013401, 8339378, 3361970, 6704179, 6717235, 10040115, 1644825,
    16445005, 6085589, 4882687, 55610, 8476209, 7340544, 13742497, 10441252, 9787244, 7367818,
    12223780, 6780213, 10505550, 3746083, 8874850, 5725276, 8014168, 4996700, 4993571, 5001770,
    9321518, 2430480, 12398641, 9715553, 6035741, 1474182, 3837580, 5647422, 1356933, 6579300,
    14200723, 8365974, 0, 0,
];
pub fn map_color_rgba(packed: u8) -> [u8; 4] {
    let rgb = MATERIAL_COLORS[usize::from(packed >> 2)];
    if rgb == 0 {
        return [0; 4];
    }
    let brightness = [180, 220, 255, 135][usize::from(packed & 3)];
    [
        (((rgb >> 16) & 255) * brightness / 255) as u8,
        (((rgb >> 8) & 255) * brightness / 255) as u8,
        ((rgb & 255) * brightness / 255) as u8,
        255,
    ]
}
pub fn map_id(stack: &ItemStack) -> Option<u32> {
    if stack.is_empty() {
        return None;
    }
    match stack.components.get("minecraft:map_id")? {
        DataComponent::Generic {
            value: Tag::Int(id),
            ..
        } => u32::try_from(*id).ok(),
        _ => None,
    }
}
fn frame_map_transform(spawn_data: i32, rotation: i32, invisible: bool) -> Mat4 {
    let direction = (spawn_data % 6).unsigned_abs();
    let (step, x_rotation, y_rotation) = match direction {
        0 => (-Vec3::Y, 90.0_f32, 180.0_f32),
        1 => (Vec3::Y, -90.0, 180.0),
        2 => (-Vec3::Z, 0.0, 0.0),
        3 => (Vec3::Z, 0.0, 180.0),
        4 => (-Vec3::X, 0.0, 90.0),
        _ => (Vec3::X, 0.0, -90.0),
    };
    Mat4::from_translation(step * 0.46875)
        * Mat4::from_rotation_x(x_rotation.to_radians())
        * Mat4::from_rotation_y(y_rotation.to_radians())
        * Mat4::from_translation(Vec3::Z * if invisible { 0.5 } else { 0.4375 })
        * Mat4::from_rotation_z(((rotation % 4) as f32 * 90.0 + 180.0).to_radians())
        * Mat4::from_scale(Vec3::splat(1.0 / 128.0))
        * Mat4::from_translation(Vec3::new(-64.0, -64.0, -1.0))
}
fn append_frame_quad(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    entity: &RemoteEntityRenderState,
    camera_origin: DVec3,
    partial_tick: f32,
    light: [f32; 2],
) {
    let relative = (entity.render_position_d(partial_tick) - camera_origin).as_vec3();
    let pose = Mat4::from_translation(relative)
        * frame_map_transform(
            entity.spawn_data,
            entity.item_frame_rotation,
            entity.shared_flags & 0x20 != 0,
        );
    let base = vertices.len() as u32;
    for (position, uv) in [
        ([0.0, 128.0, -0.01], [0.0, 1.0]),
        ([128.0, 128.0, -0.01], [1.0, 1.0]),
        ([128.0, 0.0, -0.01], [1.0, 0.0]),
        ([0.0, 0.0, -0.01], [0.0, 0.0]),
    ] {
        vertices.push(Vertex {
            position: pose.transform_point3(Vec3::from_array(position)).to_array(),
            tex_coords: uv,
            normal: [0.0, 0.0, -1.0],
            color: [1.0; 4],
            light,
        });
    }
    indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}
fn frame_light(
    entity: &RemoteEntityRenderState,
    level: Option<&ClientLevel>,
    partial_tick: f32,
) -> [f32; 2] {
    if entity.kind.trim_start_matches("minecraft:") == "glow_item_frame" {
        return [210.0 / 256.0, 240.0 / 256.0];
    }
    let Some(level) = level else {
        return [15.0 / 16.0; 2];
    };
    let position = entity.render_position_d(partial_tick);
    let pos = BlockPos::new(
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    );
    [
        f32::from(if entity.shared_flag(0) {
            15
        } else {
            level.brightness(false, pos)
        }) / 16.0,
        f32::from(level.brightness(true, pos)) / 16.0,
    ]
}
struct MapTexture {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    revision: Option<u64>,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    vertex_capacity: u64,
    index_capacity: u64,
}
pub struct MapRenderer {
    textures: HashMap<u32, MapTexture>,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
}
impl MapRenderer {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        camera_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
        camera_uniform_size: usize,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Vanilla map text shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Map text pipeline layout"),
            bind_group_layouts: &[camera_layout, texture_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Vanilla map text pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3, 1 => Float32x2, 2 => Float32x3, 3 =>
                        Float32x4, 4 => Float32x2
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_map",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::GreaterEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
        });
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Map camera uniform"),
            size: camera_uniform_size as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Map camera binding"),
            layout: camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Map dynamic texture nearest sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Self {
            textures: HashMap::new(),
            pipeline,
            sampler,
            camera_buffer,
            camera_bind_group,
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout,
        maps: &HashMap<u32, MapData>,
        entities: &HashMap<i32, RemoteEntityRenderState>,
        level: Option<&ClientLevel>,
        camera_origin: DVec3,
        partial_tick: f32,
        camera_uniform: &[u8],
    ) {
        self.textures.retain(|id, _| maps.contains_key(id));
        for cached in self.textures.values_mut() {
            cached.vertices.clear();
            cached.indices.clear();
        }
        for entity in entities.values() {
            if !matches!(
                entity.kind.trim_start_matches("minecraft:"),
                "item_frame" | "glow_item_frame"
            ) {
                continue;
            }
            let Some(id) = entity.item_stack.as_ref().and_then(map_id) else {
                continue;
            };
            let Some(data) = maps.get(&id) else { continue };
            let cached = self.textures.entry(id).or_insert_with(|| {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Vanilla map dynamic texture"),
                    size: wgpu::Extent3d {
                        width: 128,
                        height: 128,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                let view = texture.create_view(&Default::default());
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Map dynamic texture binding"),
                    layout: texture_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                });
                MapTexture {
                    texture,
                    bind_group,
                    revision: None,
                    vertices: Vec::new(),
                    indices: Vec::new(),
                    vertex_buffer: None,
                    index_buffer: None,
                    vertex_capacity: 0,
                    index_capacity: 0,
                }
            });
            if cached.revision != Some(data.revision) {
                let rgba: Vec<u8> = data
                    .colors
                    .iter()
                    .flat_map(|&color| map_color_rgba(color))
                    .collect();
                queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture: &cached.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &rgba,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(512),
                        rows_per_image: Some(128),
                    },
                    wgpu::Extent3d {
                        width: 128,
                        height: 128,
                        depth_or_array_layers: 1,
                    },
                );
                cached.revision = Some(data.revision);
            }
            append_frame_quad(
                &mut cached.vertices,
                &mut cached.indices,
                entity,
                camera_origin,
                partial_tick,
                frame_light(entity, level, partial_tick),
            );
        }
        queue.write_buffer(&self.camera_buffer, 0, camera_uniform);
        for cached in self
            .textures
            .values_mut()
            .filter(|map| !map.indices.is_empty())
        {
            upload_buffer(
                device,
                queue,
                &mut cached.vertex_buffer,
                &mut cached.vertex_capacity,
                bytemuck::cast_slice(&cached.vertices),
                wgpu::BufferUsages::VERTEX,
            );
            upload_buffer(
                device,
                queue,
                &mut cached.index_buffer,
                &mut cached.index_capacity,
                bytemuck::cast_slice(&cached.indices),
                wgpu::BufferUsages::INDEX,
            );
        }
    }
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        for cached in self.textures.values().filter(|map| !map.indices.is_empty()) {
            let (Some(vertices), Some(indices)) = (&cached.vertex_buffer, &cached.index_buffer)
            else {
                continue;
            };
            pass.set_bind_group(1, &cached.bind_group, &[]);
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..cached.indices.len() as u32, 0, 0..1);
        }
    }
}
fn upload_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &mut Option<wgpu::Buffer>,
    capacity: &mut u64,
    bytes: &[u8],
    usage: wgpu::BufferUsages,
) {
    if bytes.len() as u64 > *capacity {
        *capacity = (bytes.len() as u64).next_power_of_two().max(256);
        *buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Map frame geometry"),
            size: *capacity,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }
    if let Some(buffer) = buffer {
        queue.write_buffer(buffer, 0, bytes)
    }
}
