use super::*;

pub(super) struct WinPipelines {
    pub portal: RenderPipeline,
    pub vignette: RenderPipeline,
}

impl WinPipelines {
    pub fn new(
        device: &Device,
        target_format: wgpu::TextureFormat,
        uniforms: &wgpu::BindGroupLayout,
        textures: &wgpu::BindGroupLayout,
    ) -> Self {
        let portal_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Java End portal GUI shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../gui_end_portal.wgsl").into()),
        });
        let texture_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Java vignette GUI shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../gui.wgsl").into()),
        });
        let make = |label,
                    shader: &wgpu::ShaderModule,
                    layouts: &[&wgpu::BindGroupLayout],
                    blend| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: layouts,
                push_constant_ranges: &[],
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label), layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: shader, entry_point: "vs_main",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GuiVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader, entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState { format: target_format, blend, write_mask: wgpu::ColorWrites::ALL })],
                }),
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: false, depth_compare: wgpu::CompareFunction::Always,
                    stencil: Default::default(), bias: Default::default(),
                }),
                multisample: Default::default(), multiview: None,
            })
        };
        Self {
            portal: make(
                "End portal GUI pipeline",
                &portal_shader,
                &[uniforms, textures, textures],
                None,
            ),
            vignette: make(
                "Vignette GUI pipeline",
                &texture_shader,
                &[uniforms, textures],
                Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::Zero,
                        dst_factor: wgpu::BlendFactor::OneMinusSrc,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::Zero,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
            ),
        }
    }
}

impl GuiRenderer {
    pub(crate) fn draw_end_portal(&mut self, rect: [f32; 4], sky_idx: usize, portal_idx: usize) {
        let [x, y, w, h] = rect;
        let start_idx = self.indices.len() as u32;
        self.add_quad(
            x,
            y,
            w,
            h,
            0.0,
            0.0,
            1.0,
            1.0,
            [self.portal_game_time, 0.0, 0.0, 1.0],
        );
        let count = self.indices.len() as u32 - start_idx;
        self.batches.push(DrawBatch::EndPortal {
            sky_idx,
            portal_idx,
            start_idx,
            count,
        });
    }

    pub(crate) fn draw_vignette(&mut self, rect: [f32; 4], texture: usize, color: [f32; 4]) {
        let [x, y, w, h] = rect;
        let start_idx = self.indices.len() as u32;
        self.add_quad(x, y, w, h, 0.0, 0.0, 1.0, 1.0, color);
        let count = self.indices.len() as u32 - start_idx;
        self.batches.push(DrawBatch::Vignette {
            bind_group_idx: texture,
            start_idx,
            count,
        });
    }
}
