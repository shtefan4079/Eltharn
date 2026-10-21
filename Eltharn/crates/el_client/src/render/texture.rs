use crate::resources::manager::{ClientResourceManager, ResourceError};
use el_core::resource_location::ResourceLocation;
use image::{DynamicImage, GenericImage, GenericImageView, RgbaImage};
use std::collections::{HashMap, VecDeque};
use wgpu::Device;
const MAX_ATLAS_ROW_WIDTH: u32 = 2048;
const MAX_ATLAS_MIP_LEVEL: u32 = 4;
pub const ATLAS_SPRITE_PADDING: u32 = 1 << MAX_ATLAS_MIP_LEVEL;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureAtlasOptions {
    pub max_mip_level: u32,
    pub apply_vanilla_cutout_mip_preprocess: bool,
    pub mag_filter: wgpu::FilterMode,
    pub min_filter: wgpu::FilterMode,
    pub mipmap_filter: wgpu::FilterMode,
}
impl TextureAtlasOptions {
    pub const TERRAIN: Self = Self {
        max_mip_level: MAX_ATLAS_MIP_LEVEL,
        apply_vanilla_cutout_mip_preprocess: true,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Linear,
    };
    pub const GUI: Self = Self {
        max_mip_level: 0,
        apply_vanilla_cutout_mip_preprocess: false,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
    };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NineSliceInfo {
    pub width: u32,
    pub height: u32,
    pub border_left: u32,
    pub border_top: u32,
    pub border_right: u32,
    pub border_bottom: u32,
    pub stretch_inner: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileInfo {
    pub width: u32,
    pub height: u32,
}
#[derive(serde::Deserialize)]
struct McmetaGuiContent {
    scaling: Option<McmetaScaling>,
}
#[derive(Clone, serde::Deserialize)]
#[serde(untagged)]
enum McmetaBorder {
    Uniform(u32),
    Detailed {
        left: u32,
        top: u32,
        right: u32,
        bottom: u32,
    },
}
#[derive(Clone, serde::Deserialize)]
struct McmetaScaling {
    #[serde(rename = "type")]
    scaling_type: String,
    width: Option<u32>,
    height: Option<u32>,
    border: Option<McmetaBorder>,
    #[serde(default)]
    stretch_inner: bool,
}
impl McmetaScaling {
    fn sprite_size(&self) -> Option<(u32, u32)> {
        match (self.width, self.height) {
            (Some(width), Some(height)) if width > 0 && height > 0 => Some((width, height)),
            _ => None,
        }
    }
    fn into_nine_slice_info(self) -> Option<NineSliceInfo> {
        if self.scaling_type != "nine_slice" {
            return None;
        }
        let (width, height) = self.sprite_size()?;
        let (border_left, border_top, border_right, border_bottom) = match self.border? {
            McmetaBorder::Uniform(value) => (value, value, value, value),
            McmetaBorder::Detailed {
                left,
                top,
                right,
                bottom,
            } => (left, top, right, bottom),
        };
        Some(NineSliceInfo {
            width,
            height,
            border_left,
            border_top,
            border_right,
            border_bottom,
            stretch_inner: self.stretch_inner,
        })
    }
    fn into_tile_info(self) -> Option<TileInfo> {
        if self.scaling_type != "tile" {
            return None;
        }
        let (width, height) = self.sprite_size()?;
        Some(TileInfo { width, height })
    }
}
pub struct TextureAtlas {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
    pub sprites: HashMap<ResourceLocation, SpriteUvRect>,
    pub sprite_dimensions: HashMap<ResourceLocation, (u32, u32)>,
    pub sprite_transparency: HashMap<ResourceLocation, SpriteTransparency>,
    pub nine_slices: HashMap<ResourceLocation, NineSliceInfo>,
    pub sprite_tiles: HashMap<ResourceLocation, TileInfo>,
    animations: Vec<AtlasAnimation>,
    pending_animation_uploads: Vec<PendingAnimationUpload>,
    pending_animation_bytes: Vec<u8>,
    animation_staging_buffer: Option<wgpu::Buffer>,
    animation_staging_capacity: u64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpriteTransparency {
    #[default]
    Opaque,
    Cutout,
    Translucent,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum MipmapStrategy {
    #[default]
    Auto,
    Mean,
    Cutout,
    StrictCutout,
    DarkCutout,
}
struct AtlasAnimation {
    placement: SpritePlacement,
    frames: Vec<PreparedAnimationFrame>,
    sequence: Vec<AnimationSequenceFrame>,
    total_time: u64,
    current_frame: usize,
    current_prepared_frame: usize,
}
struct PendingAnimationUpload {
    mip_level: u32,
    origin: wgpu::Origin3d,
    width: u32,
    height: u32,
    offset: u64,
    bytes_per_row: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AnimationSequenceFrame {
    prepared_frame: usize,
    time: u32,
}
struct PendingAnimationFrame {
    index: u32,
    image: RgbaImage,
    time: u32,
}
struct PreparedAnimationFrame {
    padded_mips: Vec<RgbaImage>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteUvRect {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SpritePlacement {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}
impl TextureAtlas {
    pub fn from_resource_ids(
        device: &Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        resource_manager: &ClientResourceManager,
        texture_ids: &[ResourceLocation],
    ) -> Result<Self, ResourceError> {
        Self::from_resource_id_map(
            device,
            queue,
            bind_group_layout,
            resource_manager,
            texture_ids,
            texture_ids,
        )
    }
    pub fn from_resource_id_map(
        device: &Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        resource_manager: &ClientResourceManager,
        sprite_ids: &[ResourceLocation],
        texture_ids: &[ResourceLocation],
    ) -> Result<Self, ResourceError> {
        Self::from_resource_id_map_with_options(
            device,
            queue,
            bind_group_layout,
            resource_manager,
            sprite_ids,
            texture_ids,
            TextureAtlasOptions::TERRAIN,
        )
    }
    pub fn from_resource_id_map_with_options(
        device: &Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        resource_manager: &ClientResourceManager,
        sprite_ids: &[ResourceLocation],
        texture_ids: &[ResourceLocation],
        options: TextureAtlasOptions,
    ) -> Result<Self, ResourceError> {
        assert_eq!(
            sprite_ids.len(),
            texture_ids.len(),
            "sprite id and texture id lists must have the same length"
        );
        let mut sprite_ids = sprite_ids.to_vec();
        let mut texture_ids = texture_ids.to_vec();
        let missing_id = missing_texture_id();
        if !sprite_ids.iter().any(|id| id == &missing_id) {
            sprite_ids.push(missing_id.clone());
            texture_ids.push(missing_id);
        }
        let mut images = Vec::with_capacity(texture_ids.len());
        let mut sprite_dimensions = HashMap::new();
        let mut nine_slices = HashMap::new();
        let mut sprite_tiles = HashMap::new();
        let mut pending_animations: Vec<Option<Vec<PendingAnimationFrame>>> =
            Vec::with_capacity(texture_ids.len());
        let mut mipmap_strategies = Vec::with_capacity(texture_ids.len());
        let mut alpha_cutoff_biases = Vec::with_capacity(texture_ids.len());
        for (sprite_id, texture_id) in sprite_ids.iter().zip(texture_ids.iter()) {
            let is_missing_texture = *texture_id == missing_texture_id();
            let mut img = if is_missing_texture {
                DynamicImage::ImageRgba8(generate_missing_texture_image())
            } else {
                resource_manager.read_image(texture_id)?
            };
            let mut animation = None;
            let mut mipmap_strategy = MipmapStrategy::Auto;
            let mut alpha_cutoff_bias = 0.0_f32;
            if !is_missing_texture {
                if let Ok(Some(bytes)) = resource_manager.read_texture_metadata(texture_id) {
                    if let Ok(content) = String::from_utf8(bytes) {
                        #[derive(serde::Deserialize)]
                        struct McmetaGui {
                            gui: Option<McmetaGuiContent>,
                            animation: Option<McmetaAnimation>,
                            texture: Option<McmetaTexture>,
                        }
                        #[derive(serde::Deserialize)]
                        struct McmetaTexture {
                            mipmap_strategy: Option<String>,
                            alpha_cutoff_bias: Option<f32>,
                        }
                        #[derive(serde::Deserialize)]
                        struct McmetaAnimation {
                            frametime: Option<u32>,
                            width: Option<u32>,
                            height: Option<u32>,
                            frames: Option<Vec<McmetaFrame>>,
                        }
                        #[derive(serde::Deserialize)]
                        #[serde(untagged)]
                        enum McmetaFrame {
                            Index(u32),
                            Detailed { index: u32, time: Option<u32> },
                        }
                        if let Ok(meta) = serde_json::from_str::<McmetaGui>(&content) {
                            if let Some(texture) = meta.texture {
                                mipmap_strategy = match texture.mipmap_strategy.as_deref() {
                                    Some("mean") => MipmapStrategy::Mean,
                                    Some("cutout") => MipmapStrategy::Cutout,
                                    Some("strict_cutout") => MipmapStrategy::StrictCutout,
                                    Some("dark_cutout") => MipmapStrategy::DarkCutout,
                                    _ => MipmapStrategy::Auto,
                                };
                                alpha_cutoff_bias = texture.alpha_cutoff_bias.unwrap_or(0.0);
                            }
                            if let Some(anim) = meta.animation {
                                let frame_width = anim.width.unwrap_or(img.width());
                                let frame_height = anim.height.unwrap_or(frame_width);
                                if frame_width > 0
                                    && frame_height > 0
                                    && img.width() >= frame_width
                                    && img.height() >= frame_height
                                {
                                    let columns = img.width() / frame_width;
                                    let rows = img.height() / frame_height;
                                    let frame_count = columns * rows;
                                    let selected = anim.frames.unwrap_or_else(|| {
                                        (0..frame_count).map(McmetaFrame::Index).collect()
                                    });
                                    let mut frames = Vec::new();
                                    let default_frame_time = anim.frametime.unwrap_or(1).max(1);
                                    for frame in selected {
                                        let (index, custom_time) = match frame {
                                            McmetaFrame::Index(index) => (index, None),
                                            McmetaFrame::Detailed { index, time } => (index, time),
                                        };
                                        if index >= frame_count {
                                            continue;
                                        }
                                        let time = custom_time.unwrap_or(default_frame_time).max(1);
                                        let x = index % columns * frame_width;
                                        let y = index / columns * frame_height;
                                        frames.push(PendingAnimationFrame {
                                            index,
                                            image: img
                                                .view(x, y, frame_width, frame_height)
                                                .to_image(),
                                            time,
                                        });
                                    }
                                    if !frames.is_empty() {
                                        img = DynamicImage::ImageRgba8(frames[0].image.clone());
                                        animation = Some(frames);
                                    }
                                }
                            }
                            if let Some(gui) = meta.gui {
                                if let Some(scaling) = gui.scaling {
                                    if let Some(nine_slice) = scaling.clone().into_nine_slice_info()
                                    {
                                        nine_slices.insert(sprite_id.clone(), nine_slice);
                                    } else if let Some(tile) = scaling.into_tile_info() {
                                        sprite_tiles.insert(sprite_id.clone(), tile);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            sprite_dimensions.insert(sprite_id.clone(), (img.width(), img.height()));
            images.push(img);
            pending_animations.push(animation);
            mipmap_strategies.push(mipmap_strategy);
            alpha_cutoff_biases.push(alpha_cutoff_bias);
        }
        Ok(Self::from_images(
            device,
            queue,
            bind_group_layout,
            &images,
            &sprite_ids,
            sprite_dimensions,
            nine_slices,
            sprite_tiles,
            pending_animations,
            mipmap_strategies,
            alpha_cutoff_biases,
            options,
        ))
    }
    fn from_images(
        device: &Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        images: &[DynamicImage],
        texture_ids: &[ResourceLocation],
        sprite_dimensions: HashMap<ResourceLocation, (u32, u32)>,
        nine_slices: HashMap<ResourceLocation, NineSliceInfo>,
        sprite_tiles: HashMap<ResourceLocation, TileInfo>,
        pending_animations: Vec<Option<Vec<PendingAnimationFrame>>>,
        mipmap_strategies: Vec<MipmapStrategy>,
        alpha_cutoff_biases: Vec<f32>,
        options: TextureAtlasOptions,
    ) -> Self {
        debug_assert!(
            options.max_mip_level <= MAX_ATLAS_MIP_LEVEL,
            "atlas max mip level {} exceeds the {MAX_ATLAS_MIP_LEVEL}-level padding budget",
            options.max_mip_level
        );
        let options = TextureAtlasOptions {
            max_mip_level: options.max_mip_level.min(MAX_ATLAS_MIP_LEVEL),
            ..options
        };
        let max_atlas_height = device.limits().max_texture_dimension_2d;
        let (atlas_width, atlas_height, placements) = pack_sprite_images(images, max_atlas_height);
        for (index, placement) in placements.iter().enumerate() {
            if placement.is_none() {
                let id = texture_ids
                    .get(index)
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| format!("image #{index}"));
                log::warn!(
                    "Skipping atlas sprite {id}: it does not fit in the {MAX_ATLAS_ROW_WIDTH}x{max_atlas_height} atlas budget"
                );
            }
        }
        let transparency_by_image = images.iter().map(classify_transparency).collect::<Vec<_>>();
        let sprite_transparency: HashMap<ResourceLocation, SpriteTransparency> = texture_ids
            .iter()
            .zip(transparency_by_image.iter().copied())
            .map(|(id, transparency)| (id.clone(), transparency))
            .collect();
        let atlas_mips = build_atlas_mip_levels(
            images,
            &placements,
            atlas_width,
            atlas_height,
            &mipmap_strategies,
            &alpha_cutoff_biases,
            &transparency_by_image,
            texture_ids,
            options,
        );
        let size = wgpu::Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Texture Atlas"),
            size,
            mip_level_count: options.max_mip_level + 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (mip_level, atlas_image) in atlas_mips.iter().enumerate() {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: mip_level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                atlas_image,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * atlas_image.width()),
                    rows_per_image: Some(atlas_image.height()),
                },
                wgpu::Extent3d {
                    width: atlas_image.width(),
                    height: atlas_image.height(),
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: options.mag_filter,
            min_filter: options.min_filter,
            mipmap_filter: options.mipmap_filter,
            lod_min_clamp: 0.0,
            lod_max_clamp: options.max_mip_level as f32,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("Atlas Bind Group"),
        });
        let animations = pending_animations
            .into_iter()
            .zip(placements.iter().copied())
            .zip(mipmap_strategies)
            .zip(alpha_cutoff_biases)
            .zip(texture_ids.iter())
            .filter_map(
                |((((animation, placement), mipmap_strategy), alpha_cutoff_bias), texture_id)| {
                    let pending_frames = animation?;
                    let placement = placement?;
                    let transparency = sprite_transparency
                        .get(texture_id)
                        .copied()
                        .unwrap_or_default();
                    let solidify_cutout_mips = options.apply_vanilla_cutout_mip_preprocess
                        && should_solidify_cutout_mips(Some(texture_id));
                    let mut unique_frames = HashMap::<u32, usize>::new();
                    let mut frames = Vec::new();
                    let mut sequence = Vec::with_capacity(pending_frames.len());
                    for pending in pending_frames {
                        let prepared_frame =
                            if let Some(&prepared_frame) = unique_frames.get(&pending.index) {
                                prepared_frame
                            } else {
                                let prepared_frame = frames.len();
                                frames.push(prepare_animation_frame(
                                    &pending.image,
                                    mipmap_strategy,
                                    transparency,
                                    alpha_cutoff_bias,
                                    solidify_cutout_mips,
                                    options.max_mip_level,
                                ));
                                unique_frames.insert(pending.index, prepared_frame);
                                prepared_frame
                            };
                        sequence.push(AnimationSequenceFrame {
                            prepared_frame,
                            time: pending.time,
                        });
                    }
                    let Some(total_time) = sequence.iter().try_fold(0_u64, |total, frame| {
                        total.checked_add(u64::from(frame.time))
                    }) else {
                        log::warn!(
                            "Disabling texture animation {texture_id}: total duration overflows u64"
                        );
                        return None;
                    };
                    Some(AtlasAnimation {
                        placement,
                        frames,
                        sequence,
                        total_time,
                        current_frame: 0,
                        current_prepared_frame: 0,
                    })
                },
            )
            .collect();
        Self {
            texture,
            view,
            sampler,
            bind_group,
            sprites: sprite_rects(texture_ids, &placements, atlas_width, atlas_height),
            sprite_dimensions,
            sprite_transparency,
            nine_slices,
            sprite_tiles,
            animations,
            pending_animation_uploads: Vec::new(),
            pending_animation_bytes: Vec::new(),
            animation_staging_buffer: None,
            animation_staging_capacity: 0,
        }
    }
    pub fn tick_animations(&mut self, game_tick: u64) {
        let mut pending_animation_uploads = std::mem::take(&mut self.pending_animation_uploads);
        pending_animation_uploads.clear();
        let mut pending_animation_bytes = std::mem::take(&mut self.pending_animation_bytes);
        pending_animation_bytes.clear();
        for animation in &mut self.animations {
            let Some(next_frame) =
                select_animation_frame(&animation.sequence, animation.total_time, game_tick)
            else {
                debug_assert!(
                    false,
                    "texture animation sequence duration does not cover its total"
                );
                static WARNED: std::sync::Once = std::sync::Once::new();
                WARNED
                    .call_once(|| {
                        log::warn!(
                            "Texture animation sequence duration does not cover its total; skipping the animation"
                        );
                    });
                continue;
            };
            let next_prepared_frame = animation.sequence[next_frame].prepared_frame;
            if next_frame == animation.current_frame {
                continue;
            }
            animation.current_frame = next_frame;
            if next_prepared_frame == animation.current_prepared_frame {
                continue;
            }
            animation.current_prepared_frame = next_prepared_frame;
            let frame = &animation.frames[next_prepared_frame];
            for (level, padded_frame) in frame.padded_mips.iter().enumerate() {
                let level = level as u32;
                let padding = (ATLAS_SPRITE_PADDING >> level).max(1);
                let width = padded_frame.width();
                let height = padded_frame.height();
                let row_bytes = usize::try_from(width)
                    .expect("animation width exceeds usize")
                    .checked_mul(4)
                    .expect("animation row size overflow");
                let bytes_per_row =
                    align_up(row_bytes, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize);
                let offset = align_up(
                    pending_animation_bytes.len(),
                    wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize,
                );
                pending_animation_bytes.resize(offset, 0);
                let raw = padded_frame.as_raw();
                for row in 0..usize::try_from(height).expect("animation height exceeds usize") {
                    let row_start = row
                        .checked_mul(row_bytes)
                        .expect("animation row offset overflow");
                    pending_animation_bytes
                        .extend_from_slice(&raw[row_start..row_start + row_bytes]);
                    pending_animation_bytes.resize(
                        pending_animation_bytes
                            .len()
                            .checked_add(bytes_per_row - row_bytes)
                            .expect("animation staging size overflow"),
                        0,
                    );
                }
                pending_animation_uploads.push(PendingAnimationUpload {
                    mip_level: level,
                    origin: wgpu::Origin3d {
                        x: (animation.placement.x >> level).saturating_sub(padding),
                        y: (animation.placement.y >> level).saturating_sub(padding),
                        z: 0,
                    },
                    width,
                    height,
                    offset: u64::try_from(offset).expect("animation staging offset exceeds u64"),
                    bytes_per_row: u32::try_from(bytes_per_row)
                        .expect("animation row stride exceeds u32"),
                });
            }
        }
        self.pending_animation_uploads = pending_animation_uploads;
        self.pending_animation_bytes = pending_animation_bytes;
    }
    pub fn encode_animation_updates(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        if self.pending_animation_uploads.is_empty() {
            return;
        }
        let required = u64::try_from(self.pending_animation_bytes.len())
            .expect("animation staging bytes exceed u64");
        if self.animation_staging_buffer.is_none() || required > self.animation_staging_capacity {
            let capacity = required
                .next_power_of_two()
                .max(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u64);
            self.animation_staging_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Texture Atlas Animation Staging"),
                size: capacity,
                usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.animation_staging_capacity = capacity;
        }
        let staging = self
            .animation_staging_buffer
            .as_ref()
            .expect("animation staging buffer was just allocated");
        queue.write_buffer(staging, 0, &self.pending_animation_bytes);
        for upload in &self.pending_animation_uploads {
            encoder.copy_buffer_to_texture(
                wgpu::ImageCopyBuffer {
                    buffer: staging,
                    layout: wgpu::ImageDataLayout {
                        offset: upload.offset,
                        bytes_per_row: Some(upload.bytes_per_row),
                        rows_per_image: Some(upload.height),
                    },
                },
                wgpu::ImageCopyTexture {
                    texture: &self.texture,
                    mip_level: upload.mip_level,
                    origin: upload.origin,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: upload.width,
                    height: upload.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        self.pending_animation_uploads.clear();
        self.pending_animation_bytes.clear();
    }
    pub fn sprite_uv(&self, id: &ResourceLocation, uv: [f32; 2]) -> Option<[f32; 2]> {
        self.sprites
            .get(id)
            .map(|rect| rect.map_model_uv(uv[0], uv[1]))
    }
    pub fn sprite_rect_or_missing(&self, id: &ResourceLocation) -> SpriteUvRect {
        self.sprites
            .get(id)
            .or_else(|| self.sprites.get(&missing_texture_id()))
            .copied()
            .unwrap_or(SpriteUvRect {
                u0: 0.0,
                v0: 0.0,
                u1: 1.0,
                v1: 1.0,
            })
    }
    pub fn sprite_dimensions_or_missing(&self, id: &ResourceLocation) -> (u32, u32) {
        self.sprite_dimensions
            .get(id)
            .or_else(|| self.sprite_dimensions.get(&missing_texture_id()))
            .copied()
            .unwrap_or((16, 16))
    }
    pub fn sprite_transparency_or_missing(&self, id: &ResourceLocation) -> SpriteTransparency {
        self.sprite_transparency
            .get(id)
            .or_else(|| self.sprite_transparency.get(&missing_texture_id()))
            .copied()
            .unwrap_or(SpriteTransparency::Opaque)
    }
    pub fn contains_sprite(&self, id: &ResourceLocation) -> bool {
        self.sprites.contains_key(id)
    }
}
fn select_animation_frame(
    sequence: &[AnimationSequenceFrame],
    total_time: u64,
    game_tick: u64,
) -> Option<usize> {
    if sequence.is_empty() || total_time == 0 {
        return None;
    }
    let position = game_tick % total_time;
    let mut elapsed = 0_u64;
    sequence.iter().position(|frame| {
        elapsed = elapsed.saturating_add(u64::from(frame.time));
        position < elapsed
    })
}
fn align_up(value: usize, alignment: usize) -> usize {
    debug_assert!(alignment > 0);
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value
            .checked_add(alignment - remainder)
            .expect("texture animation staging alignment overflow")
    }
}
fn prepare_animation_frame(
    frame: &RgbaImage,
    mipmap_strategy: MipmapStrategy,
    transparency: SpriteTransparency,
    alpha_cutoff_bias: f32,
    solidify_cutout_mips: bool,
    max_mip_level: u32,
) -> PreparedAnimationFrame {
    let padded_mips = generate_sprite_mips(
        frame,
        mipmap_strategy,
        transparency,
        alpha_cutoff_bias,
        solidify_cutout_mips,
        max_mip_level,
    )
    .into_iter()
    .enumerate()
    .map(|(level, mip)| {
        let padding = (ATLAS_SPRITE_PADDING >> level as u32).max(1);
        let padded_width = mip.width() + padding * 2;
        let padded_height = mip.height() + padding * 2;
        let mut padded_frame = RgbaImage::new(padded_width, padded_height);
        copy_sprite_with_padding(
            &mut padded_frame,
            &mip,
            SpritePlacement {
                x: padding,
                y: padding,
                width: mip.width(),
                height: mip.height(),
            },
            padded_width,
            padded_height,
            padding,
        );
        padded_frame
    })
    .collect();
    PreparedAnimationFrame { padded_mips }
}
pub fn missing_texture_id() -> ResourceLocation {
    ResourceLocation::new("minecraft", "missingno")
}
pub(crate) fn generate_missing_texture_image() -> RgbaImage {
    let mut image = RgbaImage::new(16, 16);
    let pink = image::Rgba([248, 0, 248, 255]);
    let black = image::Rgba([0, 0, 0, 255]);
    for y in 0..16 {
        for x in 0..16 {
            let pixel = if (y < 8) ^ (x < 8) { pink } else { black };
            image.put_pixel(x, y, pixel);
        }
    }
    image
}
fn classify_transparency(image: &DynamicImage) -> SpriteTransparency {
    let mut has_transparent = false;
    let mut has_translucent = false;
    let pixels = image.to_rgba8();
    for pixel in pixels.pixels() {
        let a = pixel[3];
        if a == 0 {
            has_transparent = true;
        } else if a != 255 {
            has_translucent = true;
        }
    }
    if has_translucent {
        SpriteTransparency::Translucent
    } else if has_transparent {
        SpriteTransparency::Cutout
    } else {
        SpriteTransparency::Opaque
    }
}
fn build_atlas_mip_levels(
    images: &[DynamicImage],
    placements: &[Option<SpritePlacement>],
    atlas_width: u32,
    atlas_height: u32,
    strategies: &[MipmapStrategy],
    alpha_cutoff_biases: &[f32],
    transparencies: &[SpriteTransparency],
    texture_ids: &[ResourceLocation],
    options: TextureAtlasOptions,
) -> Vec<RgbaImage> {
    let mut atlas_mips = (0..=options.max_mip_level)
        .map(|level| {
            RgbaImage::new(
                (atlas_width >> level).max(1),
                (atlas_height >> level).max(1),
            )
        })
        .collect::<Vec<_>>();
    for (index, (image, placement)) in images.iter().zip(placements).enumerate() {
        let Some(placement) = placement else {
            continue;
        };
        let strategy = strategies
            .get(index)
            .copied()
            .unwrap_or(MipmapStrategy::Auto);
        let transparency = transparencies.get(index).copied().unwrap_or_default();
        let alpha_cutoff_bias = alpha_cutoff_biases.get(index).copied().unwrap_or(0.0);
        let sprite_mips = generate_sprite_mips(
            &image.to_rgba8(),
            strategy,
            transparency,
            alpha_cutoff_bias,
            options.apply_vanilla_cutout_mip_preprocess
                && should_solidify_cutout_mips(texture_ids.get(index)),
            options.max_mip_level,
        );
        for (level, sprite) in sprite_mips.iter().enumerate() {
            let level = level as u32;
            let padding = (ATLAS_SPRITE_PADDING >> level).max(1);
            let mip_placement = SpritePlacement {
                x: placement.x >> level,
                y: placement.y >> level,
                width: sprite.width(),
                height: sprite.height(),
            };
            let atlas = &mut atlas_mips[level as usize];
            let atlas_width = atlas.width();
            let atlas_height = atlas.height();
            copy_sprite_with_padding(
                atlas,
                sprite,
                mip_placement,
                atlas_width,
                atlas_height,
                padding,
            );
        }
    }
    atlas_mips
}
fn generate_sprite_mips(
    base: &RgbaImage,
    strategy: MipmapStrategy,
    transparency: SpriteTransparency,
    alpha_cutoff_bias: f32,
    solidify_cutout_mips: bool,
    max_mip_level: u32,
) -> Vec<RgbaImage> {
    let strategy = match strategy {
        MipmapStrategy::Auto if transparency != SpriteTransparency::Opaque => {
            MipmapStrategy::Cutout
        }
        MipmapStrategy::Auto => MipmapStrategy::Mean,
        strategy => strategy,
    };
    let is_cutout = matches!(
        strategy,
        MipmapStrategy::Cutout | MipmapStrategy::StrictCutout | MipmapStrategy::DarkCutout
    );
    let alpha_ref = if strategy == MipmapStrategy::StrictCutout {
        0.3
    } else {
        0.5
    };
    let mut base = base.clone();
    if solidify_cutout_mips {
        match strategy {
            MipmapStrategy::Cutout | MipmapStrategy::StrictCutout => {
                solidify_transparent_pixels(&mut base);
            }
            MipmapStrategy::DarkCutout => {
                fill_empty_areas_with_dark_color(&mut base);
            }
            MipmapStrategy::Auto | MipmapStrategy::Mean => {}
        }
    }
    let original_coverage = is_cutout.then(|| alpha_test_coverage(&base, alpha_ref, 1.0));
    let mut levels = vec![base];
    for _ in 1..=max_mip_level {
        let previous = levels.last().expect("base mip exists");
        let width = (previous.width() / 2).max(1);
        let height = (previous.height() / 2).max(1);
        let mut next = RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let pixels = [
                    previous.get_pixel(
                        (x * 2).min(previous.width() - 1),
                        (y * 2).min(previous.height() - 1),
                    ),
                    previous.get_pixel(
                        (x * 2 + 1).min(previous.width() - 1),
                        (y * 2).min(previous.height() - 1),
                    ),
                    previous.get_pixel(
                        (x * 2).min(previous.width() - 1),
                        (y * 2 + 1).min(previous.height() - 1),
                    ),
                    previous.get_pixel(
                        (x * 2 + 1).min(previous.width() - 1),
                        (y * 2 + 1).min(previous.height() - 1),
                    ),
                ];
                let pixel = if strategy == MipmapStrategy::DarkCutout {
                    dark_cutout_mean(pixels)
                } else {
                    linear_mean(pixels)
                };
                next.put_pixel(x, y, pixel);
            }
        }
        if let Some(coverage) = original_coverage {
            scale_alpha_to_coverage(&mut next, coverage, alpha_ref, alpha_cutoff_bias);
        }
        levels.push(next);
    }
    levels
}
fn should_solidify_cutout_mips(texture_id: Option<&ResourceLocation>) -> bool {
    texture_id
        .map(|id| !id.path.starts_with("item/"))
        .unwrap_or(true)
}
fn solidify_transparent_pixels(image: &mut RgbaImage) {
    let width = image.width();
    let height = image.height();
    let pixel_count = width.saturating_mul(height) as usize;
    let mut nearest_color = vec![[0_u8; 4]; pixel_count];
    let mut distances = vec![u32::MAX; pixel_count];
    let mut queue = VecDeque::new();
    for y in 0..height {
        for x in 0..width {
            let pixel = *image.get_pixel(x, y);
            if pixel[3] != 0 {
                let index = packed_pixel_index(x, y, width);
                distances[index] = 0;
                nearest_color[index] = pixel.0;
                queue.push_back((x, y));
            }
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let index = packed_pixel_index(x, y, width);
        let next_distance = distances[index].saturating_add(1);
        for (neighbor_x, neighbor_y) in [
            (x.checked_add(1), Some(y)),
            (x.checked_sub(1), Some(y)),
            (Some(x), y.checked_add(1)),
            (Some(x), y.checked_sub(1)),
        ] {
            let (Some(neighbor_x), Some(neighbor_y)) = (neighbor_x, neighbor_y) else {
                continue;
            };
            if neighbor_x >= width || neighbor_y >= height {
                continue;
            }
            let neighbor_index = packed_pixel_index(neighbor_x, neighbor_y, width);
            if distances[neighbor_index] <= next_distance {
                continue;
            }
            distances[neighbor_index] = next_distance;
            nearest_color[neighbor_index] = nearest_color[index];
            queue.push_back((neighbor_x, neighbor_y));
        }
    }
    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel_mut(x, y);
            if pixel[3] == 0 {
                let mut color = nearest_color[packed_pixel_index(x, y, width)];
                color[3] = 0;
                *pixel = image::Rgba(color);
            }
        }
    }
}
fn fill_empty_areas_with_dark_color(image: &mut RgbaImage) {
    let darkest = image
        .pixels()
        .filter(|pixel| pixel[3] != 0)
        .min_by_key(|pixel| u32::from(pixel[0]) + u32::from(pixel[1]) + u32::from(pixel[2]))
        .map(|pixel| {
            [
                (u16::from(pixel[0]) * 3 / 4) as u8,
                (u16::from(pixel[1]) * 3 / 4) as u8,
                (u16::from(pixel[2]) * 3 / 4) as u8,
                0,
            ]
        })
        .unwrap_or([191, 191, 191, 0]);
    for pixel in image.pixels_mut() {
        if pixel[3] == 0 {
            *pixel = image::Rgba(darkest);
        }
    }
}
fn packed_pixel_index(x: u32, y: u32, width: u32) -> usize {
    (x + y * width) as usize
}
#[inline]
fn java_srgb_to_linear(channel: u8) -> u16 {
    let x = f32::from(channel) / 255.0;
    let linear = if x >= 0.04045 {
        ((x + 0.055) / 1.055).powf(2.4)
    } else {
        x / 12.92
    };
    (linear * 1023.0).round() as u16
}
#[inline]
fn java_linear_to_srgb(linear: u16) -> u8 {
    let x = f32::from(linear.min(1023)) / 1023.0;
    let srgb = if x >= 0.0031308 {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    } else {
        12.92 * x
    };
    (srgb * 255.0).round() as u8
}
fn linear_mean(pixels: [&image::Rgba<u8>; 4]) -> image::Rgba<u8> {
    let mut linear = [0_u32; 3];
    for pixel in pixels {
        linear[0] += u32::from(java_srgb_to_linear(pixel[0]));
        linear[1] += u32::from(java_srgb_to_linear(pixel[1]));
        linear[2] += u32::from(java_srgb_to_linear(pixel[2]));
    }
    image::Rgba([
        java_linear_to_srgb((linear[0] / 4) as u16),
        java_linear_to_srgb((linear[1] / 4) as u16),
        java_linear_to_srgb((linear[2] / 4) as u16),
        ((u32::from(pixels[0][3])
            + u32::from(pixels[1][3])
            + u32::from(pixels[2][3])
            + u32::from(pixels[3][3]))
            / 4) as u8,
    ])
}
fn dark_cutout_mean(pixels: [&image::Rgba<u8>; 4]) -> image::Rgba<u8> {
    let mut linear = [0_u32; 4];
    for pixel in pixels {
        if pixel[3] == 0 {
            continue;
        }
        for channel in 0..4 {
            linear[channel] += u32::from(java_srgb_to_linear(pixel[channel]));
        }
    }
    image::Rgba([
        java_linear_to_srgb((linear[0] / 4) as u16),
        java_linear_to_srgb((linear[1] / 4) as u16),
        java_linear_to_srgb((linear[2] / 4) as u16),
        java_linear_to_srgb((linear[3] / 4) as u16),
    ])
}
fn alpha_test_coverage(image: &RgbaImage, alpha_ref: f32, alpha_scale: f32) -> f32 {
    let width = image.width();
    let height = image.height();
    if width <= 1 || height <= 1 {
        let total = width.saturating_mul(height).max(1);
        let covered = image
            .pixels()
            .filter(|pixel| (f32::from(pixel[3]) / 255.0 * alpha_scale).clamp(0.0, 1.0) > alpha_ref)
            .count() as u32;
        return covered as f32 / total as f32;
    }
    let mut coverage = 0.0_f32;
    for y in 0..height - 1 {
        for x in 0..width - 1 {
            let alpha00 =
                (f32::from(image.get_pixel(x, y)[3]) / 255.0 * alpha_scale).clamp(0.0, 1.0);
            let alpha10 =
                (f32::from(image.get_pixel(x + 1, y)[3]) / 255.0 * alpha_scale).clamp(0.0, 1.0);
            let alpha01 =
                (f32::from(image.get_pixel(x, y + 1)[3]) / 255.0 * alpha_scale).clamp(0.0, 1.0);
            let alpha11 =
                (f32::from(image.get_pixel(x + 1, y + 1)[3]) / 255.0 * alpha_scale).clamp(0.0, 1.0);
            let mut texel_coverage = 0.0_f32;
            for subsample_y in 0..4 {
                let fy = (subsample_y as f32 + 0.5) / 4.0;
                for subsample_x in 0..4 {
                    let fx = (subsample_x as f32 + 0.5) / 4.0;
                    let alpha = alpha00 * (1.0 - fx) * (1.0 - fy)
                        + alpha10 * fx * (1.0 - fy)
                        + alpha01 * (1.0 - fx) * fy
                        + alpha11 * fx * fy;
                    if alpha > alpha_ref {
                        texel_coverage += 1.0;
                    }
                }
            }
            coverage += texel_coverage / 16.0;
        }
    }
    coverage / ((width - 1) * (height - 1)) as f32
}
fn scale_alpha_to_coverage(
    image: &mut RgbaImage,
    desired: f32,
    alpha_ref: f32,
    alpha_cutoff_bias: f32,
) {
    let mut min_alpha_scale = 0.0_f32;
    let mut max_alpha_scale = 4.0_f32;
    let mut alpha_scale = 1.0_f32;
    let mut best_alpha_scale = 1.0_f32;
    let mut best_error = f32::MAX;
    for _ in 0..5 {
        let current_coverage = alpha_test_coverage(image, alpha_ref, alpha_scale);
        let error = (current_coverage - desired).abs();
        if error < best_error {
            best_error = error;
            best_alpha_scale = alpha_scale;
        }
        if current_coverage < desired {
            min_alpha_scale = alpha_scale;
        } else if current_coverage > desired {
            max_alpha_scale = alpha_scale;
        } else {
            break;
        }
        alpha_scale = (min_alpha_scale + max_alpha_scale) * 0.5;
    }
    for pixel in image.pixels_mut() {
        let alpha = (f32::from(pixel[3]) / 255.0 * best_alpha_scale + alpha_cutoff_bias + 0.025)
            .clamp(0.0, 1.0);
        pixel[3] = (alpha * 255.0).round() as u8;
    }
}
impl SpriteUvRect {
    pub fn inset_by_texels(self, width: u32, height: u32, inset_texels: f32) -> Self {
        fn inset_axis(start: f32, end: f32, texels: u32, inset_texels: f32) -> (f32, f32) {
            if texels == 0 || inset_texels <= 0.0 {
                return (start, end);
            }
            let span = end - start;
            let inset = (span.abs() / texels as f32 * inset_texels).min(span.abs() * 0.5);
            if span >= 0.0 {
                (start + inset, end - inset)
            } else {
                (start - inset, end + inset)
            }
        }
        let (u0, u1) = inset_axis(self.u0, self.u1, width, inset_texels);
        let (v0, v1) = inset_axis(self.v0, self.v1, height, inset_texels);
        Self { u0, v0, u1, v1 }
    }
    pub fn map_model_uv(self, u: f32, v: f32) -> [f32; 2] {
        [
            self.u0 + (self.u1 - self.u0) * (u / 16.0),
            self.v0 + (self.v1 - self.v0) * (v / 16.0),
        ]
    }
    pub fn map_unit_uv(self, u: f32, v: f32) -> [f32; 2] {
        [
            self.u0 + (self.u1 - self.u0) * u,
            self.v0 + (self.v1 - self.v0) * v,
        ]
    }
}
fn sprite_rects(
    texture_ids: &[ResourceLocation],
    placements: &[Option<SpritePlacement>],
    atlas_width: u32,
    atlas_height: u32,
) -> HashMap<ResourceLocation, SpriteUvRect> {
    let mut sprites = HashMap::new();
    for (id, placement) in texture_ids.iter().zip(placements) {
        let Some(placement) = placement else {
            continue;
        };
        sprites.insert(
            id.clone(),
            SpriteUvRect {
                u0: placement.x as f32 / atlas_width as f32,
                v0: placement.y as f32 / atlas_height as f32,
                u1: (placement.x + placement.width) as f32 / atlas_width as f32,
                v1: (placement.y + placement.height) as f32 / atlas_height as f32,
            },
        );
    }
    sprites
}
fn pack_sprite_images(
    images: &[DynamicImage],
    max_height: u32,
) -> (u32, u32, Vec<Option<SpritePlacement>>) {
    pack_sprite_images_with_max_size(images, MAX_ATLAS_ROW_WIDTH, max_height)
}
fn pack_sprite_images_with_max_size(
    images: &[DynamicImage],
    max_row_width: u32,
    max_height: u32,
) -> (u32, u32, Vec<Option<SpritePlacement>>) {
    if images.is_empty() {
        return (1, 1, Vec::new());
    }
    let mut placements = Vec::with_capacity(images.len());
    let mut x = 0;
    let mut y = 0;
    let mut row_height = 0;
    let mut atlas_width = 0;
    let mut atlas_height = 0;
    let max_row_width = align_to_mip_grid(max_row_width.max(1));
    let max_height = align_to_mip_grid(max_height.max(1));
    for image in images {
        let width = image.width().max(1);
        let height = image.height().max(1);
        let slot_width = align_to_mip_grid(width + ATLAS_SPRITE_PADDING * 2);
        let slot_height = align_to_mip_grid(height + ATLAS_SPRITE_PADDING * 2);
        if slot_width > max_row_width || slot_height > max_height {
            placements.push(None);
            continue;
        }
        if x > 0 && x + slot_width > max_row_width {
            y += row_height;
            x = 0;
            row_height = 0;
        }
        if y + slot_height > max_height {
            placements.push(None);
            continue;
        }
        placements.push(Some(SpritePlacement {
            x: x + ATLAS_SPRITE_PADDING,
            y: y + ATLAS_SPRITE_PADDING,
            width,
            height,
        }));
        x += slot_width;
        row_height = row_height.max(slot_height);
        atlas_width = atlas_width.max(x);
        atlas_height = atlas_height.max(y + row_height);
    }
    (
        align_to_mip_grid(atlas_width.max(1)),
        align_to_mip_grid(atlas_height.max(1)),
        placements,
    )
}
fn align_to_mip_grid(value: u32) -> u32 {
    value.div_ceil(ATLAS_SPRITE_PADDING) * ATLAS_SPRITE_PADDING
}
#[allow(dead_code)]
fn copy_sprite_with_extruded_padding(
    atlas_image: &mut RgbaImage,
    sprite: &RgbaImage,
    placement: SpritePlacement,
    atlas_width: u32,
    atlas_height: u32,
) {
    copy_sprite_with_padding(
        atlas_image,
        sprite,
        placement,
        atlas_width,
        atlas_height,
        ATLAS_SPRITE_PADDING,
    );
}
fn copy_sprite_with_padding(
    atlas_image: &mut RgbaImage,
    sprite: &RgbaImage,
    placement: SpritePlacement,
    atlas_width: u32,
    atlas_height: u32,
    padding: u32,
) {
    let fits = placement.x.saturating_add(sprite.width()) <= atlas_width
        && placement.y.saturating_add(sprite.height()) <= atlas_height;
    debug_assert!(
        fits,
        "sprite placement {placement:?} does not fit in {atlas_width}x{atlas_height}"
    );
    if !fits {
        return;
    }
    if atlas_image
        .copy_from(sprite, placement.x, placement.y)
        .is_err()
    {
        debug_assert!(false, "copy_from rejected a validated sprite placement");
        return;
    }
    if placement.width == 0 || placement.height == 0 {
        return;
    }
    let x0 = placement.x;
    let y0 = placement.y;
    let x1 = placement.x + placement.width - 1;
    let y1 = placement.y + placement.height - 1;
    for y in 0..placement.height {
        let atlas_y = y0 + y;
        let left = *sprite.get_pixel(0, y);
        let right = *sprite.get_pixel(placement.width - 1, y);
        for pad in 1..=padding {
            if x0 >= pad {
                atlas_image.put_pixel(x0 - pad, atlas_y, left);
            }
            let target_x = x1 + pad;
            if target_x < atlas_width {
                atlas_image.put_pixel(target_x, atlas_y, right);
            }
        }
    }
    let padded_x_min = x0.saturating_sub(padding);
    let padded_x_max = (x1 + padding).min(atlas_width.saturating_sub(1));
    for atlas_x in padded_x_min..=padded_x_max {
        let sprite_x = atlas_x.saturating_sub(x0).min(placement.width - 1);
        let top = *sprite.get_pixel(sprite_x, 0);
        let bottom = *sprite.get_pixel(sprite_x, placement.height - 1);
        for pad in 1..=padding {
            if y0 >= pad {
                atlas_image.put_pixel(atlas_x, y0 - pad, top);
            }
            let target_y = y1 + pad;
            if target_y < atlas_height {
                atlas_image.put_pixel(atlas_x, target_y, bottom);
            }
        }
    }
}
