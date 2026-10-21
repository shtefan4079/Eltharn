use crate::model::FrameData;

pub(crate) struct FrameGpu {
    pub layout: wgpu::BindGroupLayout,
    pub camera: wgpu::Buffer,
    pub environment: wgpu::Buffer,
    pub depth: wgpu::TextureView,
    _depth_texture: wgpu::Texture,
    copy_layout: wgpu::BindGroupLayout,
    copy_pipeline: wgpu::RenderPipeline,
    size: [u32; 2],
    pub ready: bool,
}

pub(crate) fn reservation(size: [u32; 2]) -> Option<u64> {
    u64::from(size[0])
        .checked_mul(u64::from(size[1]))?
        .checked_mul(16)?
        .checked_add(256)
}

pub(crate) fn light_reservation(size: [u32; 2]) -> Option<u64> {
    u64::from(size[0])
        .checked_mul(u64::from(size[1]))?
        .checked_mul(4)
}

fn texture_entry(binding: u32, sample_type: wgpu::TextureSampleType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

impl FrameGpu {
    pub fn new(
        device: &wgpu::Device,
        size: [u32; 2],
        params: bool,
        shadow: bool,
        history: bool,
        light: bool,
    ) -> Self {
        let mut entries = vec![
            texture_entry(0, wgpu::TextureSampleType::Float { filterable: true }),
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            texture_entry(3, wgpu::TextureSampleType::Depth),
            texture_entry(6, wgpu::TextureSampleType::Float { filterable: true }),
            texture_entry(7, wgpu::TextureSampleType::Float { filterable: true }),
        ];
        for (binding, bytes) in [(4, 176), (5, 80)] {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(bytes),
                },
                count: None,
            });
        }
        if params {
            entries.push(params_entry());
        }
        if shadow {
            entries.push(texture_entry(8, wgpu::TextureSampleType::Depth));
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 9,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(96),
                },
                count: None,
            });
        }
        if history {
            entries.push(texture_entry(
                10,
                wgpu::TextureSampleType::Float { filterable: true },
            ));
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 11,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(192),
                },
                count: None,
            });
        }
        if light {
            entries.push(texture_entry(
                12,
                wgpu::TextureSampleType::Float { filterable: true },
            ));
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("v4 frame inputs"),
            entries: &entries,
        });
        let uniform = |label, size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let camera = uniform("v4 world camera", 176);
        let environment = uniform("v4 environment", 80);
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("v4 world depth snapshot"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth = depth_texture.create_view(&Default::default());
        let copy_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("v4 depth copy"),
            entries: &[texture_entry(0, wgpu::TextureSampleType::Depth)],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("v4 depth copy"),
            bind_group_layouts: &[&copy_layout],
            push_constant_ranges: &[],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("v4 depth copy"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/depth_copy.wgsl").into()),
        });
        let copy_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("v4 depth copy"),
            layout: Some(&pipeline_layout),
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
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
        });
        Self {
            layout,
            camera,
            environment,
            depth,
            _depth_texture: depth_texture,
            copy_layout,
            copy_pipeline,
            size,
            ready: false,
        }
    }

    pub fn capture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        depth: &wgpu::TextureView,
        camera: &FrameData,
        environment: &[u8; 80],
    ) -> Result<(), String> {
        self.ready = false;
        let bytes = camera.bytes()?;
        crate::model::validate_environment_bytes(environment)?;
        if camera.viewport[0] != self.size[0] as f32 || camera.viewport[1] != self.size[1] as f32 {
            return Err("v4 world snapshot viewport mismatch".into());
        }
        queue.write_buffer(&self.camera, 0, &bytes);
        queue.write_buffer(&self.environment, 0, environment);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("v4 depth copy"),
            layout: &self.copy_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(depth),
            }],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("v4 snapshot world depth before hand"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.copy_pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
        self.ready = true;
        Ok(())
    }
}

pub(crate) fn params_entry() -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 2,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: std::num::NonZeroU64::new(1024),
        },
        count: None,
    }
}
