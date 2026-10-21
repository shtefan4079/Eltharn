use crate::model::{ValidatedPack, Variant};
use std::collections::BTreeMap;

#[derive(Debug)]
pub(crate) struct Pipelines {
    pub pack: BTreeMap<String, wgpu::RenderPipeline>,
    pub decode: wgpu::RenderPipeline,
    pub encode: wgpu::RenderPipeline,
    pub depth: Option<wgpu::RenderPipeline>,
    pub pack_layout: wgpu::BindGroupLayout,
    pub codec_layout: wgpu::BindGroupLayout,
    pub depth_layout: Option<wgpu::BindGroupLayout>,
}

const CODEC_SRC: &str = include_str!("../shaders/codec.wgsl");
const DEPTH_SRC: &str = include_str!("../shaders/depth_copy.wgsl");

fn primitive() -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        polygon_mode: wgpu::PolygonMode::Fill,
        unclipped_depth: false,
        conservative: false,
    }
}

fn no_msaa() -> wgpu::MultisampleState {
    wgpu::MultisampleState {
        count: 1,
        mask: !0,
        alpha_to_coverage_enabled: false,
    }
}

fn color_target(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    }
}

fn nonzero(v: u64) -> Option<std::num::NonZeroU64> {
    match std::num::NonZeroU64::new(v) {
        Some(n) => Some(n),
        None => None,
    }
}

pub(super) fn create(
    device: &wgpu::Device,
    pack: &ValidatedPack,
    variant: &Variant,
    scene_format: wgpu::TextureFormat,
) -> Pipelines {
    let frag = wgpu::ShaderStages::FRAGMENT;
    let mut entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();
    entries.push(wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: frag,
        ty: wgpu::BindingType::Texture {
            multisampled: false,
            view_dimension: wgpu::TextureViewDimension::D2,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
        },
        count: None,
    });
    entries.push(wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: frag,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    });
    if variant.params.is_some() {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: frag,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: nonzero(1024),
            },
            count: None,
        });
    }
    let want_depth = pack.profile.has_depth();
    let want_env = pack.profile.has_environment();
    if want_depth {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 3,
            visibility: frag,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Depth,
            },
            count: None,
        });
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 4,
            visibility: frag,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: nonzero(176),
            },
            count: None,
        });
        if want_env {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility: frag,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: nonzero(80),
                },
                count: None,
            });
        }
    }
    let pack_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("eltharn_pack"),
        entries: &entries,
    });
    let pack_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("eltharn_pack_pl"),
        bind_group_layouts: &[&pack_layout],
        push_constant_ranges: &[],
    });
    let mut pack_map: BTreeMap<String, wgpu::RenderPipeline> = BTreeMap::new();
    let pack_targets = [Some(color_target(wgpu::TextureFormat::Rgba8Unorm))];
    for pass in &variant.passes {
        if pack_map.contains_key(&pass.program) {
            continue;
        }
        let prog = &pack.programs[&pass.program];
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&pass.program),
            source: wgpu::ShaderSource::Wgsl(prog.source.clone().into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&pass.program),
            layout: Some(&pack_pl),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: &prog.vertex_entry,
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: &prog.fragment_entry,
                targets: &pack_targets,
            }),
            primitive: primitive(),
            depth_stencil: None,
            multisample: no_msaa(),
            multiview: None,
        });
        pack_map.insert(pass.program.clone(), pipeline);
    }
    let codec_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("eltharn_codec"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: frag,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
            },
            count: None,
        }],
    });
    let codec_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("eltharn_codec_pl"),
        bind_group_layouts: &[&codec_layout],
        push_constant_ranges: &[],
    });
    let codec_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("eltharn_codec"),
        source: wgpu::ShaderSource::Wgsl(CODEC_SRC.into()),
    });
    let decode_targets = [Some(color_target(wgpu::TextureFormat::Rgba8Unorm))];
    let decode = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("eltharn_decode"),
        layout: Some(&codec_pl),
        vertex: wgpu::VertexState {
            module: &codec_module,
            entry_point: "vs_full",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &codec_module,
            entry_point: "fs_decode",
            targets: &decode_targets,
        }),
        primitive: primitive(),
        depth_stencil: None,
        multisample: no_msaa(),
        multiview: None,
    });
    let encode_targets = [Some(color_target(scene_format))];
    let encode = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("eltharn_encode"),
        layout: Some(&codec_pl),
        vertex: wgpu::VertexState {
            module: &codec_module,
            entry_point: "vs_full",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &codec_module,
            entry_point: "fs_encode",
            targets: &encode_targets,
        }),
        primitive: primitive(),
        depth_stencil: None,
        multisample: no_msaa(),
        multiview: None,
    });
    let mut depth_layout: Option<wgpu::BindGroupLayout> = None;
    let mut depth: Option<wgpu::RenderPipeline> = None;
    if want_depth {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("eltharn_depth"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: frag,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Depth,
                },
                count: None,
            }],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("eltharn_depth_pl"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("eltharn_depth"),
            source: wgpu::ShaderSource::Wgsl(DEPTH_SRC.into()),
        });
        let pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("eltharn_depth"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: "vs_full",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: "fs_copy",
                targets: &[],
            }),
            primitive: primitive(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: no_msaa(),
            multiview: None,
        });
        depth_layout = Some(layout);
        depth = Some(pipe);
    }
    Pipelines {
        pack: pack_map,
        decode,
        encode,
        depth,
        pack_layout,
        codec_layout,
        depth_layout,
    }
}
