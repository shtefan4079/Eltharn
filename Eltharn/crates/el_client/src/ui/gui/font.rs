use crate::resources::manager::{ClientResourceManager, ResourceError};
use ab_glyph::{Font, FontArc, FontVec, PxScale, ScaleFont};
use el_core::resource_location::ResourceLocation;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use wgpu::{BindGroup, Device, Queue, TextureView};
#[derive(Deserialize, Debug)]
struct FontConfig {
    providers: Vec<FontProvider>,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FontProviderFilter {
    #[serde(default)]
    uniform: Option<bool>,
    #[serde(default, rename = "jp")]
    japanese_variants: Option<bool>,
}
impl FontProviderFilter {
    fn matches(&self, options: FontOptions) -> bool {
        self.uniform
            .is_none_or(|required| required == options.uniform)
            && self
                .japanese_variants
                .is_none_or(|required| required == options.japanese_variants)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FontOptions {
    pub uniform: bool,
    pub japanese_variants: bool,
}
#[derive(Clone, Debug, Default)]
struct SerdeSkipList(pub HashSet<char>);
impl<'de> Deserialize<'de> for SerdeSkipList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Str(String),
            List(Vec<String>),
        }
        let helper = Option::<Helper>::deserialize(deserializer)?;
        let mut set = HashSet::new();
        match helper {
            Some(Helper::Str(s)) => {
                set.extend(s.chars());
            }
            Some(Helper::List(list)) => {
                for s in list {
                    set.extend(s.chars());
                }
            }
            None => {}
        }
        Ok(SerdeSkipList(set))
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Shift(pub f32, pub f32);
impl<'de> Deserialize<'de> for Shift {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let list = Option::<Vec<f32>>::deserialize(deserializer)?;
        if let Some(v) = list {
            if v.len() >= 2 {
                return Ok(Shift(v[0], v[1]));
            }
        }
        Ok(Shift(0.0, 0.0))
    }
}
#[derive(Clone, Debug, Deserialize)]
pub struct UnihexOverrideRange {
    #[serde(deserialize_with = "deserialize_codepoint")]
    pub from: u32,
    #[serde(deserialize_with = "deserialize_codepoint")]
    pub to: u32,
    #[serde(default)]
    pub left: i32,
    #[serde(default)]
    pub right: i32,
}
fn deserialize_codepoint<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum CpHelper {
        Int(u32),
        Str(String),
    }
    match CpHelper::deserialize(deserializer)? {
        CpHelper::Int(i) => Ok(i),
        CpHelper::Str(s) => {
            if let Some(ch) = s.chars().next() {
                Ok(ch as u32)
            } else {
                Err(serde::de::Error::custom("empty codepoint string"))
            }
        }
    }
}
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum FontProvider {
    #[serde(rename = "reference")]
    Reference {
        id: String,
        #[serde(default)]
        filter: FontProviderFilter,
    },
    #[serde(rename = "bitmap")]
    Bitmap {
        file: String,
        #[serde(default = "default_ascent")]
        ascent: i32,
        #[serde(default = "default_height")]
        height: i32,
        chars: Vec<String>,
        #[serde(default)]
        filter: FontProviderFilter,
    },
    #[serde(rename = "space")]
    Space {
        advances: HashMap<String, f32>,
        #[serde(default)]
        filter: FontProviderFilter,
    },
    #[serde(rename = "unihex")]
    Unihex {
        hex_file: String,
        #[serde(default)]
        size_overrides: Vec<UnihexOverrideRange>,
        #[serde(default)]
        filter: FontProviderFilter,
    },
    #[serde(rename = "ttf", alias = "truetype")]
    TrueType {
        file: String,
        #[serde(default = "default_ttf_size")]
        size: f32,
        #[serde(default = "default_ttf_oversample")]
        oversample: f32,
        #[serde(default)]
        shift: Shift,
        #[serde(default)]
        skip: SerdeSkipList,
        #[serde(default)]
        filter: FontProviderFilter,
    },
    #[serde(rename = "legacy_unicode")]
    LegacyUnicode {
        #[serde(rename = "sizes")]
        _sizes: String,
        template: String,
        #[serde(default)]
        filter: FontProviderFilter,
    },
    #[serde(other)]
    Unknown,
}
fn default_ascent() -> i32 {
    7
}
fn default_height() -> i32 {
    8
}
fn default_ttf_size() -> f32 {
    11.0
}
fn default_ttf_oversample() -> f32 {
    1.0
}
#[derive(Clone, Debug)]
pub struct Glyph {
    pub texture_index: usize,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub left: f32,
    pub width: f32,
    pub height: f32,
    pub ascent: f32,
    pub advance: f32,
}
pub struct FontAtlasManager {
    sheet_width: u32,
    sheet_height: u32,
    sheets: Vec<Vec<u8>>,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    pub base_texture_index: usize,
    pub allocated_chars: HashSet<char>,
}
impl FontAtlasManager {
    pub fn new(base_texture_index: usize, sheet_width: u32, sheet_height: u32) -> Self {
        Self {
            sheet_width,
            sheet_height,
            sheets: Vec::new(),
            cursor_x: 1,
            cursor_y: 1,
            row_height: 0,
            base_texture_index,
            allocated_chars: HashSet::new(),
        }
    }
    fn ensure_sheet(&mut self) -> usize {
        if self.sheets.is_empty() {
            self.sheets.push(vec![
                0u8;
                (self.sheet_width * self.sheet_height * 4) as usize
            ]);
            self.cursor_x = 1;
            self.cursor_y = 1;
            self.row_height = 0;
        }
        self.sheets.len() - 1
    }
    pub fn allocate_glyph(
        &mut self,
        glyph_w: u32,
        glyph_h: u32,
        rgba_pixels: &[u8],
    ) -> (usize, f32, f32, f32, f32) {
        let padding = 1;
        let padded_w = glyph_w + padding;
        let padded_h = glyph_h + padding;
        self.ensure_sheet();
        if self.cursor_x + padded_w + padding > self.sheet_width {
            self.cursor_x = 1;
            self.cursor_y += self.row_height + padding;
            self.row_height = 0;
        }
        if self.cursor_y + padded_h + padding > self.sheet_height {
            self.sheets.push(vec![
                0u8;
                (self.sheet_width * self.sheet_height * 4) as usize
            ]);
            self.cursor_x = 1;
            self.cursor_y = 1;
            self.row_height = 0;
        }
        let sheet_idx = self.sheets.len() - 1;
        let gx = self.cursor_x;
        let gy = self.cursor_y;
        if glyph_h > self.row_height {
            self.row_height = glyph_h;
        }
        self.cursor_x += padded_w;
        let sheet = &mut self.sheets[sheet_idx];
        for y in 0..glyph_h {
            for x in 0..glyph_w {
                let src_offset = ((y * glyph_w + x) * 4) as usize;
                let dst_offset = (((gy + y) * self.sheet_width + (gx + x)) * 4) as usize;
                if src_offset + 3 < rgba_pixels.len() && dst_offset + 3 < sheet.len() {
                    sheet[dst_offset] = rgba_pixels[src_offset];
                    sheet[dst_offset + 1] = rgba_pixels[src_offset + 1];
                    sheet[dst_offset + 2] = rgba_pixels[src_offset + 2];
                    sheet[dst_offset + 3] = rgba_pixels[src_offset + 3];
                }
            }
        }
        let global_sheet_idx = sheet_idx;
        let u0 = gx as f32 / self.sheet_width as f32;
        let v0 = gy as f32 / self.sheet_height as f32;
        let u1 = (gx + glyph_w) as f32 / self.sheet_width as f32;
        let v1 = (gy + glyph_h) as f32 / self.sheet_height as f32;
        (global_sheet_idx, u0, v0, u1, v1)
    }
    pub fn allocate_glyph_for_char(
        &mut self,
        ch: char,
        glyph_w: u32,
        glyph_h: u32,
        rgba_pixels: &[u8],
    ) -> (usize, f32, f32, f32, f32) {
        self.allocated_chars.insert(ch);
        self.allocate_glyph(glyph_w, glyph_h, rgba_pixels)
    }
    pub fn upload_to_gpu(
        self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        texture_views: &mut Vec<TextureView>,
        bind_groups: &mut Vec<BindGroup>,
        glyphs: &mut HashMap<char, Glyph>,
    ) {
        let base_offset = texture_views.len();
        for ch in &self.allocated_chars {
            if let Some(glyph) = glyphs.get_mut(ch) {
                glyph.texture_index += base_offset;
            }
        }
        for (i, sheet_pixels) in self.sheets.into_iter().enumerate() {
            let size = wgpu::Extent3d {
                width: self.sheet_width,
                height: self.sheet_height,
                depth_or_array_layers: 1,
            };
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("Font Atlas Sheet {}", base_offset + i)),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &sheet_pixels,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * self.sheet_width),
                    rows_per_image: Some(self.sheet_height),
                },
                size,
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: texture_bind_group_layout,
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
                label: Some(&format!("Font Atlas Sheet Bind Group {}", base_offset + i)),
            });
            texture_views.push(view);
            bind_groups.push(bind_group);
        }
    }
}
pub struct FontRenderer {
    pub glyphs: HashMap<char, Glyph>,
    pub missing_glyph: Glyph,
    pub texture_views: Vec<TextureView>,
    pub bind_groups: Vec<BindGroup>,
    ascii_glyphs: [Option<Glyph>; 128],
    glyph_candidates_by_width: HashMap<i32, Vec<char>>,
}
impl FontRenderer {
    pub fn new(
        glyphs: HashMap<char, Glyph>,
        missing_glyph: Glyph,
        texture_views: Vec<TextureView>,
        bind_groups: Vec<BindGroup>,
    ) -> Self {
        let ascii_glyphs = Self::build_ascii_table(&glyphs);
        let glyph_candidates_by_width = Self::compute_candidates_by_width(&glyphs);
        Self {
            glyphs,
            missing_glyph,
            texture_views,
            bind_groups,
            ascii_glyphs,
            glyph_candidates_by_width,
        }
    }
    pub fn rebuild_cache(&mut self) {
        self.ascii_glyphs = Self::build_ascii_table(&self.glyphs);
        self.glyph_candidates_by_width = Self::compute_candidates_by_width(&self.glyphs);
    }
    fn build_ascii_table(glyphs: &HashMap<char, Glyph>) -> [Option<Glyph>; 128] {
        let mut table: [Option<Glyph>; 128] = std::array::from_fn(|_| None);
        for cp in 0..128u8 {
            let ch = cp as char;
            if let Some(glyph) = glyphs.get(&ch) {
                table[cp as usize] = Some(glyph.clone());
            }
        }
        table
    }
    fn compute_candidates_by_width(glyphs: &HashMap<char, Glyph>) -> HashMap<i32, Vec<char>> {
        let mut by_width: HashMap<i32, Vec<char>> = HashMap::new();
        for (&ch, glyph) in glyphs {
            if ch != ' ' {
                let width = glyph.advance.ceil() as i32;
                by_width.entry(width).or_default().push(ch);
            }
        }
        for list in by_width.values_mut() {
            list.sort_unstable();
        }
        by_width
    }
    pub fn load(
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        resource_manager: &ClientResourceManager,
        options: FontOptions,
    ) -> Result<Self, ResourceError> {
        let mut glyphs = HashMap::new();
        let mut texture_views = Vec::new();
        let mut bind_groups = Vec::new();
        let mut loaded_files = HashMap::new();
        let mut loading_stack = Vec::new();
        let mut atlas = FontAtlasManager::new(0, 2048, 2048);
        let default_id = ResourceLocation::from_string("minecraft:default");
        let _ = Self::load_font_recursive(
            device,
            queue,
            texture_bind_group_layout,
            resource_manager,
            options,
            &default_id,
            &mut glyphs,
            &mut texture_views,
            &mut bind_groups,
            &mut loaded_files,
            &mut loading_stack,
            &mut atlas,
        );
        if let Ok(namespaces) = resource_manager.namespaces() {
            for ns in namespaces {
                if ns != "minecraft" {
                    let ns_default_id = ResourceLocation::new(&ns, "default");
                    let _ = Self::load_font_recursive(
                        device,
                        queue,
                        texture_bind_group_layout,
                        resource_manager,
                        options,
                        &ns_default_id,
                        &mut glyphs,
                        &mut texture_views,
                        &mut bind_groups,
                        &mut loaded_files,
                        &mut loading_stack,
                        &mut atlas,
                    );
                }
            }
        }
        if let Some(space_glyph) = glyphs.get_mut(&' ') {
            if space_glyph.advance <= 0.0 {
                space_glyph.advance = 4.0;
            }
        } else {
            glyphs.insert(
                ' ',
                Glyph {
                    texture_index: 0,
                    u0: 0.0,
                    v0: 0.0,
                    u1: 0.0,
                    v1: 0.0,
                    left: 0.0,
                    width: 0.0,
                    height: 8.0,
                    ascent: 7.0,
                    advance: 4.0,
                },
            );
        }
        Self::load_system_fallback_glyphs(&mut glyphs, &mut atlas);
        atlas.upload_to_gpu(
            device,
            queue,
            texture_bind_group_layout,
            &mut texture_views,
            &mut bind_groups,
            &mut glyphs,
        );
        let missing_pixels: [u8; 5 * 8 * 4] = {
            let mut pixels = [0u8; 5 * 8 * 4];
            for y in 0..8usize {
                for x in 0..5usize {
                    let edge = x == 0 || x == 4 || y == 0 || y == 7;
                    let offset = (y * 5 + x) * 4;
                    pixels[offset] = 255;
                    pixels[offset + 1] = 255;
                    pixels[offset + 2] = 255;
                    pixels[offset + 3] = if edge { 255 } else { 0 };
                }
            }
            pixels
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Font Missing Glyph"),
            size: wgpu::Extent3d {
                width: 5,
                height: 8,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &missing_pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(5 * 4),
                rows_per_image: Some(8),
            },
            wgpu::Extent3d {
                width: 5,
                height: 8,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: texture_bind_group_layout,
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
            label: Some("Font Missing Glyph Bind Group"),
        });
        let missing_texture_index = texture_views.len();
        texture_views.push(view);
        bind_groups.push(bind_group);
        let missing_glyph = Glyph {
            texture_index: missing_texture_index,
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
            left: 0.0,
            width: 5.0,
            height: 8.0,
            ascent: 7.0,
            advance: 6.0,
        };
        log::info!("Loaded font renderer with {} glyphs", glyphs.len());
        Ok(Self::new(glyphs, missing_glyph, texture_views, bind_groups))
    }
    pub fn measure_width(&self, text: &str, scale: f32) -> f32 {
        let mut width = 0.0;
        for ch in text.chars() {
            width += self.glyph(ch).advance * scale;
        }
        width
    }
    #[inline(always)]
    pub fn glyph(&self, ch: char) -> &Glyph {
        let cp = ch as u32;
        if cp < 128 {
            if let Some(glyph) = &self.ascii_glyphs[cp as usize] {
                return glyph;
            }
        }
        self.glyphs.get(&ch).unwrap_or(&self.missing_glyph)
    }
    pub fn candidates_by_width(&self, target_width: i32) -> Option<&[char]> {
        self.glyph_candidates_by_width
            .get(&target_width)
            .map(|v| v.as_slice())
    }
    pub fn random_glyph(&self, target_width: i32, seed: u64) -> char {
        if let Some(candidates) = self.candidates_by_width(target_width) {
            if !candidates.is_empty() {
                let mut mixed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
                mixed ^= mixed >> 30;
                mixed = mixed.wrapping_mul(0xBF58_476D_1CE4_E5B9);
                mixed ^= mixed >> 27;
                return candidates[(mixed as usize) % candidates.len()];
            }
        }
        '?'
    }
    pub fn obfuscated_char(&self, ch: char, seed: u64) -> char {
        if ch == ' ' {
            return ' ';
        }
        let target_width = self.glyph(ch).advance.ceil() as i32;
        let substituted = self.random_glyph(target_width, seed);
        if substituted == '?' { ch } else { substituted }
    }
    pub fn vanilla_ascent(&self) -> f32 {
        self.glyph('A').ascent.max(0.0)
    }
    #[allow(clippy::too_many_arguments)]
    fn load_font_recursive(
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        resource_manager: &ClientResourceManager,
        options: FontOptions,
        font_id: &ResourceLocation,
        glyphs: &mut HashMap<char, Glyph>,
        texture_views: &mut Vec<TextureView>,
        bind_groups: &mut Vec<BindGroup>,
        loaded_files: &mut HashMap<String, usize>,
        loading_stack: &mut Vec<ResourceLocation>,
        atlas: &mut FontAtlasManager,
    ) -> Result<(), ResourceError> {
        if loading_stack.contains(font_id) {
            let mut chain = loading_stack
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            chain.push(font_id.to_string());
            return Err(ResourceError::Parse {
                id: font_id.clone(),
                message: format!("cyclic font provider reference: {}", chain.join(" -> ")),
            });
        }
        loading_stack.push(font_id.clone());
        let result = Self::load_font_recursive_inner(
            device,
            queue,
            texture_bind_group_layout,
            resource_manager,
            options,
            font_id,
            glyphs,
            texture_views,
            bind_groups,
            loaded_files,
            loading_stack,
            atlas,
        );
        loading_stack.pop();
        result
    }
    #[allow(clippy::too_many_arguments)]
    fn load_font_recursive_inner(
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        resource_manager: &ClientResourceManager,
        options: FontOptions,
        font_id: &ResourceLocation,
        glyphs: &mut HashMap<char, Glyph>,
        texture_views: &mut Vec<TextureView>,
        bind_groups: &mut Vec<BindGroup>,
        loaded_files: &mut HashMap<String, usize>,
        loading_stack: &mut Vec<ResourceLocation>,
        atlas: &mut FontAtlasManager,
    ) -> Result<(), ResourceError> {
        let definitions = resource_manager.read_font_stack(font_id)?;
        let mut parsed_any = false;
        let mut first_parse_error = None;
        for json_str in definitions.into_iter().rev() {
            let config: FontConfig = match serde_json::from_str(&json_str) {
                Ok(config) => {
                    parsed_any = true;
                    config
                }
                Err(error) => {
                    if first_parse_error.is_none() {
                        first_parse_error = Some(error);
                    }
                    log::warn!("Failed to parse font definition {}", font_id);
                    continue;
                }
            };
            for provider in config.providers {
                match provider {
                    FontProvider::Reference { id, filter } => {
                        if !filter.matches(options) {
                            continue;
                        }
                        let ref_id = ResourceLocation::from_string(&id);
                        let _ = Self::load_font_recursive(
                            device,
                            queue,
                            texture_bind_group_layout,
                            resource_manager,
                            options,
                            &ref_id,
                            glyphs,
                            texture_views,
                            bind_groups,
                            loaded_files,
                            loading_stack,
                            atlas,
                        );
                    }
                    FontProvider::Space { advances, filter } => {
                        if !filter.matches(options) {
                            continue;
                        }
                        for (ch_str, advance) in advances {
                            if let Some(ch) = ch_str.chars().next() {
                                glyphs.insert(
                                    ch,
                                    Glyph {
                                        texture_index: 0,
                                        u0: 0.0,
                                        v0: 0.0,
                                        u1: 0.0,
                                        v1: 0.0,
                                        left: 0.0,
                                        width: 0.0,
                                        height: 8.0,
                                        ascent: 7.0,
                                        advance,
                                    },
                                );
                            }
                        }
                    }
                    FontProvider::Bitmap {
                        file,
                        ascent,
                        height,
                        chars,
                        filter,
                    } => {
                        if !filter.matches(options) {
                            continue;
                        }
                        let clean_file = file.strip_suffix(".png").unwrap_or(&file);
                        let (namespace, path) = if let Some((ns, p)) = clean_file.split_once(':') {
                            (ns, p)
                        } else {
                            ("minecraft", clean_file)
                        };
                        let clean_path = path.strip_prefix("textures/").unwrap_or(path);
                        let file_id = ResourceLocation::new(namespace, clean_path);
                        let img_opt = if let Some(&idx) = loaded_files.get(&file) {
                            resource_manager
                                .read_image(&file_id)
                                .or_else(|_| {
                                    let raw_path = format!("textures/{clean_path}.png");
                                    let raw_id = ResourceLocation::new(namespace, &raw_path);
                                    let bytes = resource_manager.read_resource_bytes(&raw_id)?;
                                    image::load_from_memory(&bytes).map_err(|e| {
                                        ResourceError::Image {
                                            path: std::path::PathBuf::from(clean_file),
                                            source: e,
                                        }
                                    })
                                })
                                .ok()
                                .map(|img| (idx, img.to_rgba8()))
                        } else {
                            let img_result = resource_manager.read_image(&file_id).or_else(|_| {
                                let raw_path = format!("textures/{clean_path}.png");
                                let raw_id = ResourceLocation::new(namespace, &raw_path);
                                let bytes = resource_manager.read_resource_bytes(&raw_id)?;
                                image::load_from_memory(&bytes).map_err(|e| ResourceError::Image {
                                    path: std::path::PathBuf::from(clean_file),
                                    source: e,
                                })
                            });
                            match img_result {
                                Ok(img) => {
                                    let rgba = img.to_rgba8();
                                    let (width, height_img) = rgba.dimensions();
                                    let size = wgpu::Extent3d {
                                        width,
                                        height: height_img,
                                        depth_or_array_layers: 1,
                                    };
                                    let texture = device.create_texture(&wgpu::TextureDescriptor {
                                        label: Some(&format!("Font Texture: {}", file)),
                                        size,
                                        mip_level_count: 1,
                                        sample_count: 1,
                                        dimension: wgpu::TextureDimension::D2,
                                        format: wgpu::TextureFormat::Rgba8Unorm,
                                        usage: wgpu::TextureUsages::TEXTURE_BINDING
                                            | wgpu::TextureUsages::COPY_DST,
                                        view_formats: &[],
                                    });
                                    queue.write_texture(
                                        wgpu::ImageCopyTexture {
                                            texture: &texture,
                                            mip_level: 0,
                                            origin: wgpu::Origin3d::ZERO,
                                            aspect: wgpu::TextureAspect::All,
                                        },
                                        &rgba,
                                        wgpu::ImageDataLayout {
                                            offset: 0,
                                            bytes_per_row: Some(4 * width),
                                            rows_per_image: Some(height_img),
                                        },
                                        size,
                                    );
                                    let view = texture
                                        .create_view(&wgpu::TextureViewDescriptor::default());
                                    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                                        address_mode_u: wgpu::AddressMode::ClampToEdge,
                                        address_mode_v: wgpu::AddressMode::ClampToEdge,
                                        address_mode_w: wgpu::AddressMode::ClampToEdge,
                                        mag_filter: wgpu::FilterMode::Nearest,
                                        min_filter: wgpu::FilterMode::Nearest,
                                        mipmap_filter: wgpu::FilterMode::Nearest,
                                        ..Default::default()
                                    });
                                    let bind_group =
                                        device.create_bind_group(&wgpu::BindGroupDescriptor {
                                            layout: texture_bind_group_layout,
                                            entries: &[
                                                wgpu::BindGroupEntry {
                                                    binding: 0,
                                                    resource: wgpu::BindingResource::TextureView(
                                                        &view,
                                                    ),
                                                },
                                                wgpu::BindGroupEntry {
                                                    binding: 1,
                                                    resource: wgpu::BindingResource::Sampler(
                                                        &sampler,
                                                    ),
                                                },
                                            ],
                                            label: Some(&format!("Font Bind Group: {}", file)),
                                        });
                                    let idx = texture_views.len();
                                    texture_views.push(view);
                                    bind_groups.push(bind_group);
                                    loaded_files.insert(file.clone(), idx);
                                    Some((idx, rgba))
                                }
                                Err(e) => {
                                    log::warn!(
                                        "Failed to load font bitmap texture '{}': {}",
                                        file,
                                        e
                                    );
                                    None
                                }
                            }
                        };
                        if let Some((tex_idx, rgba)) = img_opt {
                            let (tex_w, tex_h) = rgba.dimensions();
                            let rows = chars.len();
                            if rows == 0 {
                                continue;
                            }
                            let cols = chars[0].chars().count();
                            if cols == 0 {
                                continue;
                            }
                            let cell_w = tex_w / cols as u32;
                            let cell_h = tex_h / rows as u32;
                            if cell_w == 0 || cell_h == 0 {
                                continue;
                            }
                            for (row_idx, row_str) in chars.iter().enumerate() {
                                let chars_in_row: Vec<char> = row_str.chars().collect();
                                for (col_idx, &ch) in chars_in_row.iter().enumerate() {
                                    if ch == '\0' || glyphs.contains_key(&ch) {
                                        continue;
                                    }
                                    let cell_x = col_idx as u32 * cell_w;
                                    let cell_y = row_idx as u32 * cell_h;
                                    let actual_w = get_actual_glyph_width(
                                        &rgba, cell_x, cell_y, cell_w, cell_h,
                                    );
                                    if actual_w == 0 && ch != ' ' {
                                        continue;
                                    }
                                    let glyph_w = actual_w as f32;
                                    let glyph_h = height as f32;
                                    let cell_h_f32 = cell_h as f32;
                                    let scaled_width = glyph_w * (glyph_h / cell_h_f32);
                                    let left = 0.0;
                                    let advance = if ch == ' ' {
                                        4.0
                                    } else if glyph_w > 0.0 {
                                        ((0.5 + glyph_w * (glyph_h / cell_h_f32)) as i32 + 1) as f32
                                    } else {
                                        0.0
                                    };
                                    let u_nudge = 0.01_f32.min(glyph_w * 0.49);
                                    let v_nudge = 0.01_f32.min(cell_h_f32 * 0.49);
                                    let u0 = (cell_x as f32 + u_nudge) / tex_w as f32;
                                    let u1 = (cell_x as f32 + (glyph_w - u_nudge).max(0.0))
                                        / tex_w as f32;
                                    let v0 = (cell_y as f32 + v_nudge) / tex_h as f32;
                                    let v1 = (cell_y as f32 + (cell_h_f32 - v_nudge).max(0.0))
                                        / tex_h as f32;
                                    glyphs.insert(
                                        ch,
                                        Glyph {
                                            texture_index: tex_idx,
                                            u0,
                                            v0,
                                            u1,
                                            v1,
                                            left,
                                            width: scaled_width,
                                            height: glyph_h,
                                            ascent: ascent as f32,
                                            advance,
                                        },
                                    );
                                }
                            }
                        }
                    }
                    FontProvider::Unihex {
                        hex_file,
                        size_overrides,
                        filter,
                    } => {
                        if !filter.matches(options) {
                            continue;
                        }
                        let (namespace, path) = if let Some((ns, p)) = hex_file.split_once(':') {
                            (ns, p)
                        } else {
                            ("minecraft", hex_file.as_str())
                        };
                        let font_path = if path.starts_with("font/") {
                            path.to_string()
                        } else {
                            format!("font/{path}")
                        };
                        let file_id = ResourceLocation::new(namespace, &font_path);
                        let bytes_res =
                            resource_manager.read_resource_bytes(&file_id).or_else(|_| {
                                let direct_id = ResourceLocation::new(namespace, path);
                                resource_manager.read_resource_bytes(&direct_id)
                            });
                        if let Ok(bytes) = bytes_res {
                            Self::parse_unihex_bytes(&bytes, &size_overrides, glyphs, atlas);
                        } else {
                            log::warn!("Could not load Unihex font file: {}", hex_file);
                        }
                    }
                    FontProvider::TrueType {
                        file,
                        size,
                        oversample,
                        shift,
                        skip,
                        filter,
                    } => {
                        if !filter.matches(options) {
                            continue;
                        }
                        let (namespace, path) = if let Some((ns, p)) = file.split_once(':') {
                            (ns, p)
                        } else {
                            ("minecraft", file.as_str())
                        };
                        let font_path = if path.starts_with("font/") {
                            path.to_string()
                        } else {
                            format!("font/{path}")
                        };
                        let file_id = ResourceLocation::new(namespace, &font_path);
                        let bytes_res =
                            resource_manager.read_resource_bytes(&file_id).or_else(|_| {
                                let direct_id = ResourceLocation::new(namespace, path);
                                resource_manager.read_resource_bytes(&direct_id)
                            });
                        if let Ok(bytes) = bytes_res {
                            Self::parse_ttf_bytes(
                                &bytes, size, oversample, shift, &skip.0, glyphs, atlas,
                            );
                        } else {
                            log::warn!("Could not load TrueType font file: {}", file);
                        }
                    }
                    FontProvider::LegacyUnicode {
                        _sizes: _,
                        template,
                        filter,
                    } => {
                        if !filter.matches(options) {
                            continue;
                        }
                        Self::load_legacy_unicode(resource_manager, &template, glyphs, atlas);
                    }
                    FontProvider::Unknown => {}
                }
            }
        }
        if !parsed_any {
            let message = first_parse_error
                .map(|error| format!("Failed to parse font JSON: {:?}", error))
                .unwrap_or_else(|| "Font definition is empty".to_string());
            return Err(ResourceError::Parse {
                id: font_id.clone(),
                message,
            });
        }
        Ok(())
    }
    pub fn parse_unihex_bytes(
        data: &[u8],
        size_overrides: &[UnihexOverrideRange],
        glyphs: &mut HashMap<char, Glyph>,
        atlas: &mut FontAtlasManager,
    ) {
        if data.starts_with(b"PK\x03\x04") || data.starts_with(b"PK\x05\x06") {
            let cursor = std::io::Cursor::new(data);
            if let Ok(mut archive) = zip::ZipArchive::new(cursor) {
                for i in 0..archive.len() {
                    if let Ok(mut file) = archive.by_index(i) {
                        if file.name().ends_with(".hex") {
                            let mut buf = String::new();
                            if file.read_to_string(&mut buf).is_ok() {
                                Self::parse_unihex_text(&buf, size_overrides, glyphs, atlas);
                            }
                        }
                    }
                }
            }
        } else if let Ok(text) = std::str::from_utf8(data) {
            Self::parse_unihex_text(text, size_overrides, glyphs, atlas);
        }
    }
    pub fn parse_unihex_text(
        text: &str,
        size_overrides: &[UnihexOverrideRange],
        glyphs: &mut HashMap<char, Glyph>,
        atlas: &mut FontAtlasManager,
    ) {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((cp_str, hex_data)) = line.split_once(':') else {
                continue;
            };
            let Ok(cp) = u32::from_str_radix(cp_str.trim(), 16) else {
                continue;
            };
            let Some(ch) = char::from_u32(cp) else {
                continue;
            };
            if glyphs.contains_key(&ch) {
                continue;
            }
            let hex_data = hex_data.trim();
            let hex_len = hex_data.len();
            let digits_per_row = match hex_len {
                32 => 2,
                64 => 4,
                96 => 6,
                128 => 8,
                _ => continue,
            };
            let bit_width = digits_per_row * 4;
            let mut row_masks = [0u32; 16];
            let mut combined_mask = 0u32;
            for row in 0..16 {
                let start = row * digits_per_row;
                let end = start + digits_per_row;
                if end <= hex_data.len() {
                    if let Ok(val) = u32::from_str_radix(&hex_data[start..end], 16) {
                        let shifted = val << (32 - bit_width);
                        row_masks[row] = shifted;
                        combined_mask |= shifted;
                    }
                }
            }
            let (left, right) =
                if let Some(ovr) = size_overrides.iter().find(|o| o.from <= cp && cp <= o.to) {
                    (ovr.left, ovr.right)
                } else if combined_mask == 0 {
                    (0, bit_width as i32 - 1)
                } else {
                    let l = combined_mask.leading_zeros() as i32;
                    let r = 31 - combined_mask.trailing_zeros() as i32;
                    (l, r)
                };
            let glyph_w = (right - left + 1).max(1) as u32;
            let mut rgba_pixels = vec![0u8; (glyph_w * 16 * 4) as usize];
            for y in 0..16u32 {
                let mask = row_masks[y as usize];
                for x in 0..glyph_w {
                    let bit_idx = 31 - (left + x as i32);
                    let is_set = if (0..32).contains(&bit_idx) {
                        (mask >> bit_idx) & 1 != 0
                    } else {
                        false
                    };
                    let offset = ((y * glyph_w + x) * 4) as usize;
                    if is_set {
                        rgba_pixels[offset] = 255;
                        rgba_pixels[offset + 1] = 255;
                        rgba_pixels[offset + 2] = 255;
                        rgba_pixels[offset + 3] = 255;
                    }
                }
            }
            let (sheet_idx, u0, v0, u1, v1) =
                atlas.allocate_glyph_for_char(ch, glyph_w, 16, &rgba_pixels);
            let advance = (glyph_w as f32) / 2.0 + 1.0;
            let scaled_width = glyph_w as f32 / 2.0;
            glyphs.insert(
                ch,
                Glyph {
                    texture_index: sheet_idx,
                    u0,
                    v0,
                    u1,
                    v1,
                    left: 0.0,
                    width: scaled_width,
                    height: 8.0,
                    ascent: 7.0,
                    advance,
                },
            );
        }
    }
    pub fn parse_ttf_bytes(
        data: &[u8],
        size: f32,
        oversample: f32,
        shift: Shift,
        skip: &HashSet<char>,
        glyphs: &mut HashMap<char, Glyph>,
        atlas: &mut FontAtlasManager,
    ) {
        let font_res = FontArc::try_from_vec(data.to_vec())
            .or_else(|_| FontVec::try_from_vec_and_index(data.to_vec(), 0).map(FontArc::new));
        let Ok(font) = font_res else {
            return;
        };
        let pixel_size = size * oversample;
        let scale = PxScale::from(pixel_size);
        let scaled_font = font.as_scaled(scale);
        let common_ranges: &[std::ops::RangeInclusive<u32>] = &[
            0x0020..=0x007E,
            0x00A0..=0x024F,
            0x0250..=0x02AF,
            0x02B0..=0x02FF,
            0x0300..=0x036F,
            0x0370..=0x03FF,
            0x0400..=0x052F,
            0x0530..=0x058F,
            0x0590..=0x06FF,
            0x0750..=0x077F,
            0x08A0..=0x08FF,
            0x0900..=0x097F,
            0x0E00..=0x0E7F,
            0x10A0..=0x10FF,
            0x1100..=0x11FF,
            0x16A0..=0x16FF,
            0x1E00..=0x1EFF,
            0x1F00..=0x1FFF,
            0x2000..=0x2BFF,
            0x2C00..=0x2C5F,
            0x2C60..=0x2C7F,
            0x2DE0..=0x2DFF,
            0x3000..=0x303F,
            0x3040..=0x309F,
            0x30A0..=0x30FF,
            0x3100..=0x312F,
            0x3130..=0x318F,
            0x3400..=0x4DBF,
            0x4E00..=0x9FFF,
            0xA640..=0xA69F,
            0xA720..=0xA7FF,
            0xAC00..=0xD7AF,
            0xE000..=0xF8FF,
            0xF900..=0xFAFF,
            0xFB00..=0xFB4F,
            0xFB50..=0xFDFF,
            0xFE70..=0xFEFF,
            0xFF00..=0xFFEF,
            0x1F000..=0x1F9FF,
            0xF0000..=0xFFFFD,
            0x100000..=0x10FFFD,
        ];
        for range in common_ranges {
            for cp in range.clone() {
                let Some(ch) = char::from_u32(cp) else {
                    continue;
                };
                if skip.contains(&ch) || glyphs.contains_key(&ch) {
                    continue;
                }
                let glyph_id = font.glyph_id(ch);
                if glyph_id.0 == 0 {
                    continue;
                }
                let glyph = glyph_id
                    .with_scale_and_position(scale, ab_glyph::point(0.0, scaled_font.ascent()));
                let advance = scaled_font.h_advance(glyph_id) / oversample;
                if let Some(outlined) = font.outline_glyph(glyph) {
                    let bounds = outlined.px_bounds();
                    let w = bounds.width().ceil().max(1.0) as u32;
                    let h = bounds.height().ceil().max(1.0) as u32;
                    let mut rgba = vec![0u8; (w * h * 4) as usize];
                    outlined.draw(|gx, gy, coverage| {
                        if gx < w && gy < h {
                            let a = (coverage * 255.0).round().clamp(0.0, 255.0) as u8;
                            let idx = ((gy * w + gx) * 4) as usize;
                            rgba[idx] = 255;
                            rgba[idx + 1] = 255;
                            rgba[idx + 2] = 255;
                            rgba[idx + 3] = a;
                        }
                    });
                    let (sheet_idx, u0, v0, u1, v1) =
                        atlas.allocate_glyph_for_char(ch, w, h, &rgba);
                    let left = bounds.min.x / oversample + shift.0;
                    let ascent = (scaled_font.ascent() - bounds.min.y) / oversample + shift.1;
                    let glyph_w = w as f32 / oversample;
                    let glyph_h = h as f32 / oversample;
                    glyphs.insert(
                        ch,
                        Glyph {
                            texture_index: sheet_idx,
                            u0,
                            v0,
                            u1,
                            v1,
                            left,
                            width: glyph_w,
                            height: glyph_h,
                            ascent,
                            advance,
                        },
                    );
                } else if advance > 0.0 {
                    glyphs.insert(
                        ch,
                        Glyph {
                            texture_index: 0,
                            u0: 0.0,
                            v0: 0.0,
                            u1: 0.0,
                            v1: 0.0,
                            left: 0.0,
                            width: 0.0,
                            height: 8.0,
                            ascent: 7.0,
                            advance,
                        },
                    );
                }
            }
        }
    }
    pub fn load_legacy_unicode(
        resource_manager: &ClientResourceManager,
        template: &str,
        glyphs: &mut HashMap<char, Glyph>,
        atlas: &mut FontAtlasManager,
    ) {
        for page in 0..=255u32 {
            let page_hex = format!("{:02x}", page);
            let page_path = template.replace("%s", &page_hex);
            let clean_path = if page_path.ends_with(".png") {
                &page_path[..page_path.len() - 4]
            } else {
                &page_path
            };
            let id = ResourceLocation::from_string(clean_path);
            let Ok(img) = resource_manager.read_image(&id) else {
                continue;
            };
            let rgba = img.to_rgba8();
            let (tex_w, tex_h) = rgba.dimensions();
            let cell_w = tex_w / 16;
            let cell_h = tex_h / 16;
            for row in 0..16u32 {
                for col in 0..16u32 {
                    let cp = (page << 8) | (row * 16 + col);
                    let Some(ch) = char::from_u32(cp) else {
                        continue;
                    };
                    if glyphs.contains_key(&ch) {
                        continue;
                    }
                    let cell_x = col * cell_w;
                    let cell_y = row * cell_h;
                    let actual_w = get_actual_glyph_width(&rgba, cell_x, cell_y, cell_w, cell_h);
                    if actual_w == 0 {
                        continue;
                    }
                    let mut char_pixels = vec![0u8; (actual_w * cell_h * 4) as usize];
                    for y in 0..cell_h {
                        for x in 0..actual_w {
                            let src_px = rgba.get_pixel(cell_x + x, cell_y + y);
                            let dst_idx = ((y * actual_w + x) * 4) as usize;
                            char_pixels[dst_idx..dst_idx + 4].copy_from_slice(&src_px.0);
                        }
                    }
                    let (sheet_idx, u0, v0, u1, v1) =
                        atlas.allocate_glyph_for_char(ch, actual_w, cell_h, &char_pixels);
                    let cell_h_f = cell_h as f32;
                    let scaled_w = (actual_w as f32) * (8.0 / cell_h_f);
                    let advance = (0.5 + (actual_w as f32) * (8.0 / cell_h_f)) as i32 as f32 + 1.0;
                    glyphs.insert(
                        ch,
                        Glyph {
                            texture_index: sheet_idx,
                            u0,
                            v0,
                            u1,
                            v1,
                            left: 0.0,
                            width: scaled_w,
                            height: 8.0,
                            ascent: 7.0,
                            advance,
                        },
                    );
                }
            }
        }
    }
    pub fn load_system_fallback_glyphs(
        glyphs: &mut HashMap<char, Glyph>,
        atlas: &mut FontAtlasManager,
    ) {
        const SYSTEM_FONT_CANDIDATES: &[&str] = &[
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSerifCJK-Regular.ttc",
            "/usr/share/fonts/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
            "/usr/share/fonts/wqy-microhei/wqy-microhei.ttc",
            "/usr/share/fonts/wqy-zenhei/wqy-zenhei.ttc",
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/noto/NotoSansArabic-Regular.ttf",
            "/usr/share/fonts/noto/NotoSansDevanagari-Regular.ttf",
            "/usr/share/fonts/noto/NotoSansThai-Regular.ttf",
            "/usr/share/fonts/noto/NotoSansHebrew-Regular.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
            "/system/fonts/NotoSansCJK-Regular.ttc",
            "/system/fonts/DroidSansFallback.ttf",
            "/system/fonts/NotoSansSC-Regular.otf",
            "/system/fonts/NotoSansJP-Regular.otf",
            "/system/fonts/NotoSansKR-Regular.otf",
            "/system/fonts/NotoColorEmoji.ttf",
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\msyh.ttf",
            "C:\\Windows\\Fonts\\msgothic.ttc",
            "C:\\Windows\\Fonts\\malgun.ttf",
            "C:\\Windows\\Fonts\\arialuni.ttf",
            "C:\\Windows\\Fonts\\seguisym.ttf",
            "C:\\Windows\\Fonts\\simsun.ttc",
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/AppleSDGothicNeo.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ];
        let mut font_arcs: Vec<FontArc> = Vec::new();
        for path in SYSTEM_FONT_CANDIDATES {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(font) = FontArc::try_from_vec(bytes.clone()) {
                    font_arcs.push(font);
                } else {
                    for idx in 0..8 {
                        if let Ok(font_vec) = FontVec::try_from_vec_and_index(bytes.clone(), idx) {
                            font_arcs.push(FontArc::new(font_vec));
                            break;
                        }
                    }
                }
            }
        }
        let unicode_blocks: &[std::ops::RangeInclusive<u32>] = &[
            0x00A0..=0x024F,
            0x0250..=0x02AF,
            0x0370..=0x03FF,
            0x0400..=0x052F,
            0x0590..=0x05FF,
            0x0600..=0x06FF,
            0x0750..=0x077F,
            0x08A0..=0x08FF,
            0x0900..=0x097F,
            0x0E00..=0x0E7F,
            0x10A0..=0x10FF,
            0x1100..=0x11FF,
            0x16A0..=0x16FF,
            0x1E00..=0x1EFF,
            0x2000..=0x2BFF,
            0x2C00..=0x2C7F,
            0x2DE0..=0x2DFF,
            0x3000..=0x303F,
            0x3040..=0x309F,
            0x30A0..=0x30FF,
            0x3100..=0x318F,
            0x3400..=0x4DBF,
            0x4E00..=0x9FFF,
            0xA640..=0xA69F,
            0xA720..=0xA7FF,
            0xAC00..=0xD7AF,
            0xE000..=0xF8FF,
            0xF900..=0xFAFF,
            0xFB50..=0xFDFF,
            0xFE70..=0xFEFF,
            0xFF00..=0xFFEF,
            0x1F000..=0x1F9FF,
            0xF0000..=0xFFFFD,
            0x100000..=0x10FFFD,
        ];
        let target_font_size = 16.0;
        let scale = PxScale::from(target_font_size);
        for font in &font_arcs {
            let scaled_font = font.as_scaled(scale);
            let font_ascent = scaled_font.ascent();
            for block in unicode_blocks {
                for cp in block.clone() {
                    let Some(ch) = char::from_u32(cp) else {
                        continue;
                    };
                    if glyphs.contains_key(&ch) {
                        continue;
                    }
                    let glyph_id = font.glyph_id(ch);
                    if glyph_id.0 == 0 {
                        continue;
                    }
                    let glyph =
                        glyph_id.with_scale_and_position(scale, ab_glyph::point(0.0, font_ascent));
                    let advance_raw = scaled_font.h_advance(glyph_id);
                    if let Some(outlined) = font.outline_glyph(glyph) {
                        let bounds = outlined.px_bounds();
                        let w = bounds.width().ceil().max(1.0) as u32;
                        let h = bounds.height().ceil().max(1.0) as u32;
                        let mut rgba = vec![0u8; (w * h * 4) as usize];
                        outlined.draw(|x, y, coverage| {
                            if x < w && y < h {
                                let alpha = (coverage * 255.0).round().clamp(0.0, 255.0) as u8;
                                let idx = ((y * w + x) * 4) as usize;
                                rgba[idx] = 255;
                                rgba[idx + 1] = 255;
                                rgba[idx + 2] = 255;
                                rgba[idx + 3] = alpha;
                            }
                        });
                        let (sheet_idx, u0, v0, u1, v1) =
                            atlas.allocate_glyph_for_char(ch, w, h, &rgba);
                        let glyph_w = w as f32 / 2.0;
                        let glyph_h = h as f32 / 2.0;
                        let left = (bounds.min.x / 2.0).max(0.0);
                        let is_zero_width = matches!(
                            cp, 0x200B..= 0x200F | 0x2028..= 0x202E | 0x2060..= 0x206F |
                            0xFEFF
                        );
                        let advance = if is_zero_width {
                            0.0
                        } else {
                            (advance_raw / 2.0).round().max(1.0)
                        };
                        let ascent = 7.0;
                        glyphs.insert(
                            ch,
                            Glyph {
                                texture_index: sheet_idx,
                                u0,
                                v0,
                                u1,
                                v1,
                                left,
                                width: glyph_w,
                                height: glyph_h,
                                ascent,
                                advance,
                            },
                        );
                    } else if advance_raw > 0.0 {
                        let is_zero_width = matches!(
                            cp, 0x200B..= 0x200F | 0x2028..= 0x202E | 0x2060..= 0x206F |
                            0xFEFF
                        );
                        let advance = if is_zero_width {
                            0.0
                        } else {
                            (advance_raw / 2.0).round().max(1.0)
                        };
                        glyphs.insert(
                            ch,
                            Glyph {
                                texture_index: 0,
                                u0: 0.0,
                                v0: 0.0,
                                u1: 0.0,
                                v1: 0.0,
                                left: 0.0,
                                width: 0.0,
                                height: 8.0,
                                ascent: 7.0,
                                advance,
                            },
                        );
                    }
                }
            }
        }
    }
}
fn get_actual_glyph_width(
    image: &image::RgbaImage,
    cell_x: u32,
    cell_y: u32,
    cell_width: u32,
    cell_height: u32,
) -> u32 {
    for x in (0..cell_width).rev() {
        for y in 0..cell_height {
            let px = image.get_pixel(cell_x + x, cell_y + y);
            let is_active = px[3] != 0;
            if is_active {
                return x + 1;
            }
        }
    }
    0
}
impl el_gui::FontWidthMeasurer for FontRenderer {
    #[inline]
    fn measure_width(&self, text: &str, scale: f32) -> f32 {
        if text.contains('\u{1b}') || text.contains('§') {
            crate::ui::chat::chat_component::formatted_chat_width(self, text, scale)
        } else {
            self.measure_width(text, scale)
        }
    }
    #[inline]
    fn measure_char(&self, ch: char, scale: f32) -> f32 {
        self.glyph(ch).advance * scale
    }
}
