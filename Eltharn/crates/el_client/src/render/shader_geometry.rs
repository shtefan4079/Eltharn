use super::*;
use crate::render::terrain_gpu_arena::{
    TerrainOpaqueBatch, TerrainOpaqueDraw, TerrainOpaqueIndirectUpload,
};
#[cfg(feature = "shaderpacks")]
const _: () = {
    use crate::render::baked_model::Vertex;
    assert!(
        std::mem::size_of::<Vertex>() as u64 == el_shaderpack::shadow_v4::TERRAIN_VERTEX_STRIDE
    );
    assert!(std::mem::offset_of!(Vertex, position) == 0);
    assert!(std::mem::offset_of!(Vertex, tex_coords) == 12);
    assert!(std::mem::offset_of!(Vertex, normal) == 20);
    assert!(std::mem::offset_of!(Vertex, color) == 32);
    assert!(std::mem::offset_of!(Vertex, light) == 48);
};
pub(crate) const GEOMETRY_NORMAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub(crate) const GEOMETRY_NORMAL_CLEAR: wgpu::Color = wgpu::Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};
const GEOMETRY_WGSL: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) color: vec4<f32>,
    @location(4) light: vec2<f32>,
};

struct GeometryVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_geometry(model: VertexInput) -> GeometryVertexOutput {
    var out: GeometryVertexOutput;
    out.tex_coords = model.tex_coords;
    out.normal = model.normal;
    out.color = model.color;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct GeometryFragment {
    @location(0) normal: vec4<f32>,
    @location(1) albedo: vec4<f32>,
};

fn sample_terrain_nearest(uv_in: vec2<f32>) -> vec4<f32> {
    let pixel_size = 1.0 / vec2<f32>(textureDimensions(t_diffuse, 0));
    let du = dpdx(uv_in);
    let dv = dpdy(uv_in);
    let texel_screen_size = sqrt(du * du + dv * dv);
    let uv_texel_coords = uv_in / pixel_size;
    let texel_center = round(uv_texel_coords) - vec2<f32>(0.5);
    var texel_offset = uv_texel_coords - texel_center;
    texel_offset = (texel_offset - vec2<f32>(0.5)) * pixel_size / texel_screen_size + vec2<f32>(0.5);
    texel_offset = clamp(texel_offset, vec2<f32>(0.0), vec2<f32>(1.0));
    return textureSampleGrad(t_diffuse, s_diffuse, (texel_center + texel_offset) * pixel_size, du, dv);
}

@fragment
fn fs_geometry_normal(in: GeometryVertexOutput) -> GeometryFragment {
    let sampled = sample_terrain_nearest(in.tex_coords) * in.color;
    if (sampled.a < 0.5) {
        discard;
    }
    var output: GeometryFragment;
    output.albedo = vec4<f32>(sampled.rgb, 1.0);
    output.normal = vec4<f32>(0.5, 1.0, 0.5, 1.0);
    if (dot(in.normal, in.normal) >= 1e-6) {
        output.normal = vec4<f32>(normalize(in.normal) * 0.5 + vec3<f32>(0.5), 1.0);
    }
    return output;
}
"#;
pub(in crate::client::state) struct ShaderGeometry {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    _albedo_texture: wgpu::Texture,
    albedo_view: wgpu::TextureView,
    light: Option<(wgpu::Texture, wgpu::TextureView)>,
    pipeline: wgpu::RenderPipeline,
    width: u32,
    height: u32,
}
impl ShaderGeometry {
    #[cfg(feature = "shaderpacks")]
    pub(super) fn views(&self) -> (&wgpu::TextureView, &wgpu::TextureView) {
        (&self.view, &self.albedo_view)
    }
    #[cfg(feature = "shaderpacks")]
    pub(super) fn light_view(&self) -> Option<&wgpu::TextureView> {
        self.light.as_ref().map(|(_, view)| view)
    }
    fn new(
        device: &wgpu::Device,
        camera_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
        depth_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        light_enabled: bool,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shader Geometry Normal Texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: GEOMETRY_NORMAL_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let albedo_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shader Opaque Tinted Albedo"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let albedo_view = albedo_texture.create_view(&Default::default());
        let light = light_enabled.then(|| {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Shader Opaque Effective Block/Sky UV2"),
                size: wgpu::Extent3d {
                    width: width.max(1),
                    height: height.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rg16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            (texture, view)
        });
        let stock_pipeline = || {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader Geometry"),
                source: wgpu::ShaderSource::Wgsl(GEOMETRY_WGSL.into()),
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Shader Geometry Pipeline Layout"),
                bind_group_layouts: &[camera_layout, texture_layout],
                push_constant_ranges: &[],
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Shader Geometry Opaque Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_geometry",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x3, 1 => Float32x2, 2 => Float32x3, 3 =>
                            Float32x4, 4 => Float32x2
                        ],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_geometry_normal",
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            format: GEOMETRY_NORMAL_FORMAT,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                    ],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: depth_format,
                    depth_write_enabled: false,
                    depth_compare: JAVA_DEFAULT_DEPTH_COMPARE,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            })
        };
        #[cfg(feature = "shaderpacks")]
        let pipeline = if light_enabled {
            debug_assert_eq!(depth_format, wgpu::TextureFormat::Depth32Float);
            el_shaderpack::light_v4::geometry_pipeline(device, camera_layout, texture_layout)
        } else {
            stock_pipeline()
        };
        #[cfg(not(feature = "shaderpacks"))]
        let pipeline = {
            debug_assert!(!light_enabled);
            stock_pipeline()
        };
        Self {
            texture,
            view,
            _albedo_texture: albedo_texture,
            albedo_view,
            light,
            pipeline,
            width: width.max(1),
            height: height.max(1),
        }
    }
    fn matches_size(&self, width: u32, height: u32) -> bool {
        self.width == width.max(1) && self.height == height.max(1)
    }
}
pub(in crate::client::state) struct GeometryOpaquePackets<'a> {
    pub remote_world: bool,
    pub opaque_terrain_uses_multi_draw: bool,
    pub opaque_terrain_draws: &'a Vec<(RenderSectionKey, TerrainOpaqueDraw)>,
    pub opaque_terrain_batches: &'a Vec<TerrainOpaqueBatch>,
    pub opaque_terrain_indirect_uploads: &'a Vec<TerrainOpaqueIndirectUpload>,
}
impl ClientRenderer {
    #[cfg(feature = "shaderpacks")]
    pub(in crate::client::state) fn shader_geometry_views(
        &self,
    ) -> Option<(&wgpu::TextureView, &wgpu::TextureView)> {
        self.shader_geometry
            .as_ref()
            .map(|geometry| (&geometry.view, &geometry.albedo_view))
    }
    #[cfg(feature = "shaderpacks")]
    pub(in crate::client::state) fn shader_geometry_light_view(
        &self,
    ) -> Option<&wgpu::TextureView> {
        self.shader_geometry
            .as_ref()?
            .light
            .as_ref()
            .map(|(_, view)| view)
    }
    pub(in crate::client::state) fn set_shader_geometry_enabled(
        &mut self,
        device: &wgpu::Device,
        enabled: bool,
        width: u32,
        height: u32,
        light_enabled: bool,
    ) {
        if !enabled {
            self.shader_geometry = None;
            #[cfg(feature = "shaderpacks")]
            {
                self.shader_entities = None;
            }
            return;
        }
        let needs_rebuild = self
            .shader_geometry
            .as_ref()
            .map(|geometry| {
                !geometry.matches_size(width, height) || geometry.light.is_some() != light_enabled
            })
            .unwrap_or(true);
        if needs_rebuild {
            self.shader_geometry = Some(ShaderGeometry::new(
                device,
                &self.camera_bind_group_layout,
                &self.texture_bind_group_layout,
                wgpu::TextureFormat::Depth32Float,
                width,
                height,
                light_enabled,
            ));
        }
    }
    pub(in crate::client::state) fn resize_shader_geometry(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) {
        if let Some(geometry) = self.shader_geometry.as_ref() {
            let light_enabled = geometry.light.is_some();
            self.set_shader_geometry_enabled(device, true, width, height, light_enabled);
        }
    }
    pub(in crate::client::state) fn encode_shader_geometry(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        packets: GeometryOpaquePackets<'_>,
    ) {
        self.encode_shader_geometry_with_mods(encoder, packets, true);
    }
    #[cfg(feature = "shaderpacks")]
    pub(in crate::client::state) fn encode_terrain_lighting_geometry(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        packets: GeometryOpaquePackets<'_>,
    ) {
        self.encode_shader_geometry_with_mods(encoder, packets, false);
    }
    fn encode_shader_geometry_with_mods(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        packets: GeometryOpaquePackets<'_>,
        include_mods: bool,
    ) {
        #[cfg(not(feature = "extensions"))]
        let _ = include_mods;
        let Some(geometry) = self.shader_geometry.as_ref() else {
            return;
        };
        let GeometryOpaquePackets {
            remote_world,
            opaque_terrain_uses_multi_draw,
            opaque_terrain_draws,
            opaque_terrain_batches,
            opaque_terrain_indirect_uploads,
        } = packets;
        let mut attachments = vec![
            Some(wgpu::RenderPassColorAttachment {
                view: &geometry.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(GEOMETRY_NORMAL_CLEAR),
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &geometry.albedo_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(GEOMETRY_NORMAL_CLEAR),
                    store: wgpu::StoreOp::Store,
                },
            }),
        ];
        if let Some((_, view)) = &geometry.light {
            attachments.push(Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(GEOMETRY_NORMAL_CLEAR),
                    store: wgpu::StoreOp::Store,
                },
            }));
        }
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Shader Geometry Opaque Pass"),
            color_attachments: &attachments,
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
        });
        let Some(atlas) = self.block_atlas.as_ref() else {
            return;
        };
        render_pass.set_pipeline(&geometry.pipeline);
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
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.multi_draw_indexed_indirect(indirect_buffer, 0, upload.draw_count);
                }
            } else {
                let mut current_bound_bank = None;
                for (_, draw) in opaque_terrain_draws {
                    if current_bound_bank != Some(draw.bank) {
                        let Some((vertex_buffer, index_buffer)) =
                            self.remote_terrain_gpu_arena.buffers(draw.bank)
                        else {
                            debug_assert!(false, "terrain section references a missing bank");
                            continue;
                        };
                        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                        render_pass
                            .set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
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
                }
            }
        } else if self.num_indices > 0 {
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }
        #[cfg(feature = "extensions")]
        if include_mods && let Some(gpu) = &self.mod_content {
            for mesh in [&gpu.solid, &gpu.cutout] {
                if let (Some(vertices), Some(indices)) = (&mesh.vertex_buffer, &mesh.index_buffer)
                    && mesh.index_count > 0
                {
                    render_pass.set_bind_group(0, &gpu.camera_bind_group, &[]);
                    render_pass.set_bind_group(1, &gpu.atlas.bind_group, &[]);
                    render_pass.set_vertex_buffer(0, vertices.slice(..));
                    render_pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                }
            }
        }
    }
}
