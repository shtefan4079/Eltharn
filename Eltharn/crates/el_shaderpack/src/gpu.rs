use crate::{model::*, strict::ensure};
use std::collections::{BTreeMap, BTreeSet};
use wgpu::util::DeviceExt;
mod cache;
mod pipelines;
mod targets;
pub use cache::{CacheStats, PipelineCache, PipelineCacheKey, ProgramFingerprint};
type Result<T> = std::result::Result<T, String>;
pub struct Runtime {
    id: String,
    content_id: String,
    profile: Profile,
    variant: Variant,
    rejected: Vec<String>,
    pipes: pipelines::Pipelines,
    targets: targets::Targets,
    samplers: BTreeMap<String, wgpu::Sampler>,
    params: Option<wgpu::Buffer>,
    camera: Option<wgpu::Buffer>,
    environment: Option<wgpu::Buffer>,
    format: wgpu::TextureFormat,
    size: [u32; 2],
    dimension_limit: u32,
    budget: u64,
    bytes: u64,
    cache_key: PipelineCacheKey,
    generation: u64,
    encoded_frames: u64,
    depth_captures: u64,
    frame_started: bool,
    depth_valid: bool,
}
fn byte_cost(variant: &Variant, profile: Profile, size: [u32; 2]) -> Result<u64> {
    let textures = variant.textures.len() as u64 + 2 + u64::from(profile.has_depth());
    u64::from(size[0])
        .checked_mul(u64::from(size[1]))
        .and_then(|n| n.checked_mul(4))
        .and_then(|n| n.checked_mul(textures))
        .and_then(|n| n.checked_add(if variant.params.is_some() { 1024 } else { 0 }))
        .and_then(|n| n.checked_add(if profile.has_depth() { 176 } else { 0 }))
        .and_then(|n| n.checked_add(if profile.has_environment() { 80 } else { 0 }))
        .ok_or_else(|| "GPU texture budget arithmetic overflow".into())
}
fn dimensions(size: [u32; 2], limit: u32) -> Result<()> {
    ensure(
        size.iter().all(|n| *n > 0 && *n <= limit),
        "viewport exceeds effective GPU limits",
    )
}
fn select_variant(
    pack: &ValidatedPack,
    device: &wgpu::Device,
    adapter: &wgpu::Adapter,
    size: [u32; 2],
    scene_format: wgpu::TextureFormat,
    budget: u64,
) -> Result<(Variant, u64, Vec<String>, u32)> {
    use wgpu::{TextureFormat as F, TextureUsages as U};
    if !matches!(
        scene_format,
        F::Rgba8Unorm | F::Bgra8Unorm | F::Rgba8UnormSrgb | F::Bgra8UnormSrgb
    ) {
        return Err(format!(
            "shaderpack GPU preparation: pack '{}': unsupported scene format",
            pack.id
        ));
    }
    let dimension_limit = device
        .limits()
        .max_texture_dimension_2d
        .min(adapter.limits().max_texture_dimension_2d);
    dimensions(size, dimension_limit).map_err(|error| {
        format!(
            "shaderpack GPU preparation: pack '{}' size {}x{}: {error}",
            pack.id, size[0], size[1]
        )
    })?;
    let support = |format, usage, filter: bool| {
        let f = adapter.get_texture_format_features(format);
        f.allowed_usages.contains(usage)
            && (!filter
                || f.flags
                    .contains(wgpu::TextureFormatFeatureFlags::FILTERABLE))
    };
    ensure(
        support(
            F::Rgba8Unorm,
            U::RENDER_ATTACHMENT | U::TEXTURE_BINDING,
            true,
        ),
        "RGBA8 sample/filter/render unavailable",
    )?;
    ensure(
        support(
            scene_format,
            U::RENDER_ATTACHMENT | U::TEXTURE_BINDING | U::COPY_SRC,
            false,
        ),
        "scene output format unsupported",
    )?;
    if pack.profile.has_depth() {
        ensure(
            support(
                F::Depth32Float,
                U::RENDER_ATTACHMENT | U::TEXTURE_BINDING,
                false,
            ),
            "Depth32 snapshot unavailable",
        )?;
    }
    let mut capabilities = BTreeSet::from(["filterable_rgba8", "sampler_linear"]);
    match pack.profile {
        Profile::Color => {
            capabilities.insert("post_color_v1");
        }
        Profile::ColorDepth => {
            if device.limits().max_uniform_buffer_binding_size >= 176
                && device.limits().max_uniform_buffers_per_shader_stage >= 1
                && device.limits().max_sampled_textures_per_shader_stage >= 2
            {
                capabilities.extend(["post_color_depth_v1", "depth32float", "camera_uniform_v1"]);
            }
        }
        Profile::ColorDepthEnvironment => {
            if device.limits().max_uniform_buffer_binding_size >= 176
                && device.limits().max_uniform_buffers_per_shader_stage >= 2
                && device.limits().max_sampled_textures_per_shader_stage >= 2
            {
                capabilities.extend([
                    "post_color_depth_environment_v1",
                    "depth32float",
                    "camera_uniform_v1",
                    "environment_uniform_v1",
                ]);
            }
        }
    }
    let uniform_need =
        u32::from(pack.profile.has_depth()) + u32::from(pack.profile.has_environment()) + 1;
    if device.limits().max_uniform_buffer_binding_size >= 1024
        && device.limits().max_uniform_buffers_per_shader_stage >= uniform_need
    {
        capabilities.insert("params_uniform_v1");
    }
    let mut rejected = Vec::new();
    let mut chosen = None;
    for variant in &pack.variants {
        let cost = byte_cost(variant, pack.profile, size)?;
        let missing: Vec<_> = variant
            .capabilities
            .iter()
            .filter(|c| !capabilities.contains(c.as_str()))
            .collect();
        if !missing.is_empty() {
            rejected.push(format!("{}: missing {missing:?}", variant.id));
        } else if variant.min_texture_dimension > dimension_limit {
            rejected.push(format!("{}: minimum GPU dimension", variant.id));
        } else if cost > budget {
            rejected.push(format!("{}: {cost} bytes exceed {budget}", variant.id));
        } else {
            chosen = Some((variant.clone(), cost));
            break;
        }
    }
    let (variant, bytes) =
        chosen.ok_or_else(|| format!("no supported variant: {}", rejected.join("; ")))?;
    Ok((variant, bytes, rejected, dimension_limit))
}
impl Runtime {
    pub fn requires_environment(&self) -> bool {
        self.profile.has_environment()
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn prepare(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        adapter: &wgpu::Adapter,
        pack: &ValidatedPack,
        size: [u32; 2],
        scene_format: wgpu::TextureFormat,
        budget: u64,
        scene: &wgpu::TextureView,
        world_depth: &wgpu::TextureView,
    ) -> Result<Self> {
        let (variant, bytes, rejected, dimension_limit) =
            select_variant(pack, device, adapter, size, scene_format, budget)?;
        let cache_key = PipelineCacheKey::for_pack(pack, &variant, scene_format)?;
        Self::finish_prepare(
            device,
            pack,
            variant,
            bytes,
            rejected,
            cache_key,
            scene_format,
            size,
            dimension_limit,
            budget,
            scene,
            world_depth,
            None,
        )
        .await
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn prepare_with_cache(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        adapter: &wgpu::Adapter,
        pack: &ValidatedPack,
        size: [u32; 2],
        scene_format: wgpu::TextureFormat,
        budget: u64,
        scene: &wgpu::TextureView,
        world_depth: &wgpu::TextureView,
        cache: &mut PipelineCache,
    ) -> Result<Self> {
        let (variant, bytes, rejected, dimension_limit) =
            select_variant(pack, device, adapter, size, scene_format, budget)?;
        let cache_key = PipelineCacheKey::for_pack(pack, &variant, scene_format)?;
        let cached = cache.take(&cache_key);
        Self::finish_prepare(
            device,
            pack,
            variant,
            bytes,
            rejected,
            cache_key,
            scene_format,
            size,
            dimension_limit,
            budget,
            scene,
            world_depth,
            cached,
        )
        .await
    }
    pub fn retire_into_cache(self, cache: &mut PipelineCache) {
        cache.store(self.cache_key.clone(), self.pipes);
    }
    pub fn cache_key(&self) -> &PipelineCacheKey {
        &self.cache_key
    }
    #[allow(clippy::too_many_arguments)]
    async fn finish_prepare(
        device: &wgpu::Device,
        pack: &ValidatedPack,
        variant: Variant,
        bytes: u64,
        rejected: Vec<String>,
        cache_key: PipelineCacheKey,
        scene_format: wgpu::TextureFormat,
        size: [u32; 2],
        dimension_limit: u32,
        budget: u64,
        scene: &wgpu::TextureView,
        world_depth: &wgpu::TextureView,
        cached: Option<pipelines::Pipelines>,
    ) -> Result<Self> {
        ensure(
            !variant.passes.is_empty()
                && variant.passes.len() <= 32
                && variant.textures.len() <= 64,
            "invalid admitted graph bounds",
        )?;
        ensure(
            variant.params.as_ref().is_none_or(|v| v.len() == 1024),
            "invalid admitted param bytes",
        )?;
        device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let pipes =
            cached.unwrap_or_else(|| pipelines::create(device, pack, &variant, scene_format));
        let samplers = variant
            .samplers
            .iter()
            .map(|id| {
                (
                    id.clone(),
                    device.create_sampler(&wgpu::SamplerDescriptor {
                        label: Some(id),
                        mag_filter: wgpu::FilterMode::Linear,
                        min_filter: wgpu::FilterMode::Linear,
                        mipmap_filter: wgpu::FilterMode::Nearest,
                        address_mode_u: wgpu::AddressMode::ClampToEdge,
                        address_mode_v: wgpu::AddressMode::ClampToEdge,
                        address_mode_w: wgpu::AddressMode::ClampToEdge,
                        ..Default::default()
                    }),
                )
            })
            .collect();
        let params = variant.params.as_ref().map(|b| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shaderpack params"),
                contents: b,
                usage: wgpu::BufferUsages::UNIFORM,
            })
        });
        let camera = (pack.profile.has_depth()).then(|| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("shaderpack world camera"),
                size: 176,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        let environment = (pack.profile.has_environment()).then(|| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("shaderpack environment"),
                size: 80,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        let targets = match targets::create(
            device,
            &pipes,
            &variant,
            scene_format,
            size,
            scene,
            world_depth,
            &samplers,
            params.as_ref(),
            camera.as_ref(),
            environment.as_ref(),
        ) {
            Ok(targets) => targets,
            Err(error) => {
                let validation = device.pop_error_scope().await;
                let oom = device.pop_error_scope().await;
                let _ = (validation, oom);
                return Err(format!(
                    "shaderpack GPU preparation: pack '{}' variant '{}': {error}",
                    pack.id, variant.id
                ));
            }
        };
        let validation = device.pop_error_scope().await;
        let oom = device.pop_error_scope().await;
        if let Some(error) = validation.or(oom) {
            return Err(format!(
                "shaderpack GPU preparation: pack '{}' variant '{}': {error}",
                pack.id, variant.id
            ));
        }
        Ok(Self {
            id: pack.id.clone(),
            content_id: pack.content_id.clone(),
            profile: pack.profile,
            variant,
            rejected,
            pipes,
            targets,
            samplers,
            params,
            camera,
            environment,
            format: scene_format,
            size,
            dimension_limit,
            budget,
            bytes,
            cache_key,
            generation: 1,
            encoded_frames: 0,
            depth_captures: 0,
            frame_started: false,
            depth_valid: false,
        })
    }
    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        scene: &wgpu::TextureView,
        world_depth: &wgpu::TextureView,
        size: [u32; 2],
    ) -> Result<()> {
        dimensions(size, self.dimension_limit)?;
        let bytes = byte_cost(&self.variant, self.profile, size)?;
        ensure(
            bytes <= self.budget,
            "pinned shaderpack exceeds resize budget",
        )?;
        let targets = targets::create(
            device,
            &self.pipes,
            &self.variant,
            self.format,
            size,
            scene,
            world_depth,
            &self.samplers,
            self.params.as_ref(),
            self.camera.as_ref(),
            self.environment.as_ref(),
        )
        .map_err(|error| {
            format!(
                "shaderpack resize {}x{}: variant '{}': {error}",
                size[0], size[1], self.variant.id
            )
        })?;
        self.targets = targets;
        self.size = size;
        self.bytes = bytes;
        self.generation += 1;
        self.frame_started = false;
        self.depth_valid = false;
        Ok(())
    }
    pub fn begin_frame(&mut self, queue: &wgpu::Queue, frame: &FrameData) -> Result<()> {
        self.frame_started = false;
        self.depth_valid = false;
        ensure(
            !self.profile.has_environment(),
            "environment profile requires begin_frame_with_environment",
        )?;
        if let Some(camera) = &self.camera {
            let viewport = [
                self.size[0] as f32,
                self.size[1] as f32,
                1.0 / self.size[0] as f32,
                1.0 / self.size[1] as f32,
            ];
            ensure(
                frame.viewport == viewport,
                "camera viewport differs from pinned targets",
            )?;
            queue.write_buffer(camera, 0, &frame.bytes()?);
        }
        self.frame_started = true;
        self.depth_valid = self.profile == Profile::Color;
        Ok(())
    }
    pub fn begin_frame_with_environment(
        &mut self,
        queue: &wgpu::Queue,
        frame: &FrameData,
        environment: [u8; 80],
    ) -> Result<()> {
        self.frame_started = false;
        self.depth_valid = false;
        ensure(
            self.profile.has_environment(),
            "begin_frame_with_environment requires the environment profile",
        )?;
        crate::model::validate_environment_bytes(&environment)?;
        let camera = self
            .camera
            .as_ref()
            .ok_or_else(|| "environment profile has no camera buffer".to_string())?;
        let environment_buffer = self
            .environment
            .as_ref()
            .ok_or_else(|| "environment profile has no environment buffer".to_string())?;
        let viewport = [
            self.size[0] as f32,
            self.size[1] as f32,
            1.0 / self.size[0] as f32,
            1.0 / self.size[1] as f32,
        ];
        ensure(
            frame.viewport == viewport,
            "camera viewport differs from pinned targets",
        )?;
        queue.write_buffer(camera, 0, &frame.bytes()?);
        queue.write_buffer(environment_buffer, 0, &environment);
        self.frame_started = true;
        self.depth_valid = false;
        Ok(())
    }
    pub fn capture_depth(&mut self, encoder: &mut wgpu::CommandEncoder) -> Result<()> {
        if self.profile == Profile::Color {
            return Ok(());
        }
        ensure(
            self.frame_started && !self.depth_valid,
            "world depth capture must occur once in an active frame",
        )?;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shaderpack world depth snapshot"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.targets.depth.as_ref().expect("depth profile").view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(self.pipes.depth.as_ref().expect("depth profile"));
        pass.set_bind_group(
            0,
            self.targets.capture.as_ref().expect("depth profile"),
            &[],
        );
        pass.draw(0..3, 0..1);
        drop(pass);
        self.depth_valid = true;
        self.depth_captures += 1;
        Ok(())
    }
    pub fn encode(&mut self, encoder: &mut wgpu::CommandEncoder) -> Result<()> {
        ensure(
            self.frame_started && self.depth_valid,
            "shader graph requires current frame color/depth",
        )?;
        draw(
            encoder,
            "shaderpack decode",
            &self.targets.input.view,
            &self.pipes.decode,
            &self.targets.decode,
        );
        for (i, p) in self.variant.passes.iter().enumerate() {
            draw(
                encoder,
                &p.id,
                &self.targets.textures[&p.output].view,
                &self.pipes.pack[&p.program],
                &self.targets.passes[i],
            );
        }
        draw(
            encoder,
            "shaderpack encode",
            &self.targets.output.view,
            &self.pipes.encode,
            &self.targets.encode,
        );
        self.frame_started = false;
        self.depth_valid = false;
        self.encoded_frames += 1;
        Ok(())
    }
    pub fn output_view(&self) -> &wgpu::TextureView {
        &self.targets.output.view
    }
    pub fn output_texture(&self) -> &wgpu::Texture {
        &self.targets.output.texture
    }
    pub fn encoded_frames(&self) -> u64 {
        self.encoded_frames
    }
    pub fn stats(&self) -> serde_json::Value {
        let depth = usize::from(self.profile.has_depth());
        let env_buffers = usize::from(self.profile.has_environment());
        let lifetimes: BTreeMap<&str, [usize; 2]> = self
            .variant
            .texture_lifetimes
            .iter()
            .map(|(name, life)| (name.as_str(), [life.first_write_step, life.last_use_step]))
            .collect();
        serde_json::json!(
            { "id" : self.id, "content_id" : self.content_id, "profile" : self.profile
            .name(), "variant" : self.variant.id, "rejected_variants" : self.rejected,
            "pipeline_cache_key" : self.cache_key.digest_hex(), "shader_programs" : self
            .pipes.pack.len(), "internal_programs" : 1 + depth, "pipelines" : self.pipes
            .pack.len() + 2 + depth, "textures" : self.variant.textures.len() + 2 +
            depth, "samplers" : self.samplers.len(), "buffers" : usize::from(self.params
            .is_some()) + depth + env_buffers, "bindgroups" : self.variant.passes.len() +
            2 + depth, "logical_bytes" : self.bytes, "budget_bytes" : self.budget,
            "dimensions" : self.size, "generation" : self.generation, "encoded_frames" :
            self.encoded_frames, "depth_captures" : self.depth_captures, "pass_order" :
            self.variant.passes.iter().map(| pass | pass.id.as_str()).collect::< Vec < _
            >> (), "texture_lifetimes" : lifetimes }
        )
    }
}
fn draw(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    group: &wgpu::BindGroup,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
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
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, group, &[]);
    pass.draw(0..3, 0..1);
}
