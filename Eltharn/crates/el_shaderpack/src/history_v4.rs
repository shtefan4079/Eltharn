use crate::graph_v4::HistoryConfig;

pub(crate) fn reservation(config: &HistoryConfig, size: [u32; 2]) -> Option<u64> {
    u64::from(size[0])
        .checked_mul(u64::from(size[1]))?
        .checked_mul(config.format.bytes_per_texel())?
        .checked_mul(2)?
        .checked_add(192)
}

pub(crate) struct HistoryGpu {
    pub config: HistoryConfig,
    textures: [wgpu::Texture; 2],
    views: [wgpu::TextureView; 2],
    pub uniform: wgpu::Buffer,
    size: [u32; 2],
    read: usize,
    valid: bool,
    index: u64,
    previous: [u8; 176],
    current: Option<[u8; 176]>,
    pending: bool,
    context: Option<(u64, u64)>,
}

impl HistoryGpu {
    pub fn new(device: &wgpu::Device, config: HistoryConfig, size: [u32; 2]) -> Self {
        let textures = std::array::from_fn(|_| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("previous submitted color"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: match config.format {
                    crate::model_v3::TextureFormatV3::Rgba16Float => {
                        wgpu::TextureFormat::Rgba16Float
                    }
                    _ => wgpu::TextureFormat::Rgba8Unorm,
                },
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
        });
        let views = std::array::from_fn(|i| textures[i].create_view(&Default::default()));
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("previous submitted camera"),
            size: 192,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            config,
            textures,
            views,
            uniform,
            size,
            read: 0,
            valid: false,
            index: 0,
            previous: [0; 176],
            current: None,
            pending: false,
            context: None,
        }
    }
    pub fn reset(&mut self) {
        self.valid = false;
        self.index = 0;
        self.current = None;
        self.pending = false;
    }
    pub fn observe(&mut self, world: Option<u64>, camera_cut: u64) {
        let next = world.map(|world| (world, camera_cut));
        if next.is_none() || self.context != next {
            self.reset();
            self.context = next;
        }
    }
    pub fn begin(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        camera: &crate::model::FrameData,
    ) -> Result<(), String> {
        self.pending = false;
        self.current = Some(
            camera
                .bytes()?
                .try_into()
                .map_err(|_| "history camera size")?,
        );
        if !self.valid {
            for view in &self.views {
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("clear invalid history"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }
        }
        let mut bytes = [0u8; 192];
        bytes[..176].copy_from_slice(if self.valid {
            &self.previous
        } else {
            self.current.as_ref().unwrap()
        });
        for (offset, value) in [
            (176, u32::from(self.valid)),
            (180, self.index as u32),
            (184, (self.index >> 32) as u32),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        queue.write_buffer(&self.uniform, 0, &bytes);
        Ok(())
    }
    pub fn view(&self) -> &wgpu::TextureView {
        &self.views[self.read]
    }
    pub fn copy_current(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::Texture,
    ) -> Result<(), String> {
        if self.current.is_none() || self.pending {
            return Err("history frame not captured/already encoded".into());
        }
        encoder.copy_texture_to_texture(
            source.as_image_copy(),
            self.textures[1 - self.read].as_image_copy(),
            wgpu::Extent3d {
                width: self.size[0],
                height: self.size[1],
                depth_or_array_layers: 1,
            },
        );
        self.pending = true;
        Ok(())
    }
    pub fn commit(&mut self) {
        if self.pending {
            self.previous = self.current.take().expect("history pending camera");
            self.read = 1 - self.read;
            self.valid = true;
            self.index = self.index.saturating_add(1);
            self.pending = false;
        }
    }
    pub fn abort(&mut self) {
        self.pending = false;
        self.current = None;
    }
    pub fn stats(&self) -> serde_json::Value {
        serde_json::json!({"source":self.config.source,"valid":self.valid,"submitted_frames":self.index,"pending":self.pending})
    }
}
