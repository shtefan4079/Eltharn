pub const LIGHT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;
pub const TERRAIN_SHADER: &str = include_str!("shaders/opaque_light.wgsl");

pub fn geometry_pipeline(
    device: &wgpu::Device,
    camera_layout: &wgpu::BindGroupLayout,
    texture_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("opaque effective block/sky light"),
        source: wgpu::ShaderSource::Wgsl(TERRAIN_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("opaque light MRT"),
        bind_group_layouts: &[camera_layout, texture_layout],
        push_constant_ranges: &[],
    });
    let target = |format| {
        Some(wgpu::ColorTargetState {
            format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("opaque light MRT"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: "vs_geometry",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 56, step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x2,2=>Float32x3,3=>Float32x4,4=>Float32x2],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader, entry_point: "fs_geometry_normal",
            targets: &[target(wgpu::TextureFormat::Rgba16Float), target(wgpu::TextureFormat::Rgba8Unorm), target(LIGHT_FORMAT)],
        }),
        primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::GreaterEqual,
            stencil: Default::default(), bias: Default::default(),
        }),
        multisample: Default::default(), multiview: None,
    })
}

pub(crate) fn probe(device: &wgpu::Device, entities: bool) -> Result<(), String> {
    let camera = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("opaque light MRT preflight camera"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: std::num::NonZeroU64::new(64),
            },
            count: None,
        }],
    });
    let texture = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("opaque light MRT preflight texture"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let _pipeline = geometry_pipeline(device, &camera, &texture);
    if entities {
        let _replay = crate::entities_v4::EntityReplay::new_with_light(device, &texture, true)?;
    }
    Ok(())
}
