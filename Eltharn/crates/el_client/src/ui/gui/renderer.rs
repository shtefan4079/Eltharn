use crate::render::face_bakery::FaceBakery;
use crate::render::model::BlockModel;
use crate::render::texture::{
    NineSliceInfo, SpriteUvRect, TextureAtlas, TextureAtlasOptions, TileInfo,
};
use crate::resources::manager::{ClientResourceKind, ClientResourceManager, ResourceError};
use crate::ui::gui::font::FontRenderer;
use base64::Engine;
use el_core::item::{DataComponent, ItemStack};
use el_core::resource_location::ResourceLocation;
use el_math::Direction;
use el_nbt::tag::Tag;
use glam::{EulerRot, Mat4, Vec3};
use image::RgbaImage;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{self, Receiver, Sender};
use web_time::Instant;
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, Buffer, Device, Queue, RenderPipeline, TextureView};
#[cfg(feature = "extensions")]
mod canvas;
mod win_effects;
fn sprite_region_uvs(
    rect: SpriteUvRect,
    sprite_width: f32,
    sprite_height: f32,
    texture_x: f32,
    texture_y: f32,
    width: f32,
    height: f32,
) -> (f32, f32, f32, f32) {
    let u_span = rect.u1 - rect.u0;
    let v_span = rect.v1 - rect.v0;
    (
        rect.u0 + u_span * texture_x / sprite_width,
        rect.v0 + v_span * texture_y / sprite_height,
        rect.u0 + u_span * (texture_x + width) / sprite_width,
        rect.v0 + v_span * (texture_y + height) / sprite_height,
    )
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct SpriteQuad {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
}
fn push_stretched_sprite_quad(
    quads: &mut Vec<SpriteQuad>,
    rect: SpriteUvRect,
    virtual_size: [f32; 2],
    destination: [f32; 4],
    source: [f32; 4],
) {
    let [x, y, width, height] = destination;
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let [texture_x, texture_y, texture_width, texture_height] = source;
    let (u0, v0, u1, v1) = sprite_region_uvs(
        rect,
        virtual_size[0],
        virtual_size[1],
        texture_x,
        texture_y,
        texture_width,
        texture_height,
    );
    quads.push(SpriteQuad {
        x,
        y,
        width,
        height,
        u0,
        v0,
        u1,
        v1,
    });
}
fn push_tiled_sprite_quads(
    quads: &mut Vec<SpriteQuad>,
    rect: SpriteUvRect,
    virtual_size: [f32; 2],
    destination: [f32; 4],
    source: [f32; 4],
) {
    let [x, y, width, height] = destination;
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let [texture_x, texture_y, tile_width, tile_height] = source;
    if tile_width <= 0.0 || tile_height <= 0.0 {
        return;
    }
    let (u0, v0, tile_u1, tile_v1) = sprite_region_uvs(
        rect,
        virtual_size[0],
        virtual_size[1],
        texture_x,
        texture_y,
        tile_width,
        tile_height,
    );
    let columns = (width / tile_width).ceil() as u32;
    let rows = (height / tile_height).ceil() as u32;
    for column in 0..columns {
        let tile_x = column as f32 * tile_width;
        let remaining_width = width - tile_x;
        let (drawn_width, u1) = if tile_width <= remaining_width {
            (tile_width, tile_u1)
        } else {
            (
                remaining_width,
                u0 + (tile_u1 - u0) * (remaining_width / tile_width),
            )
        };
        for row in 0..rows {
            let tile_y = row as f32 * tile_height;
            let remaining_height = height - tile_y;
            let (drawn_height, v1) = if tile_height <= remaining_height {
                (tile_height, tile_v1)
            } else {
                (
                    remaining_height,
                    v0 + (tile_v1 - v0) * (remaining_height / tile_height),
                )
            };
            quads.push(SpriteQuad {
                x: x + tile_x,
                y: y + tile_y,
                width: drawn_width,
                height: drawn_height,
                u0,
                v0,
                u1,
                v1,
            });
        }
    }
}
fn push_nine_slice_inner_quads(
    quads: &mut Vec<SpriteQuad>,
    rect: SpriteUvRect,
    nine_slice: NineSliceInfo,
    destination: [f32; 4],
    source: [f32; 4],
) {
    let virtual_size = [nine_slice.width as f32, nine_slice.height as f32];
    if nine_slice.stretch_inner {
        push_stretched_sprite_quad(quads, rect, virtual_size, destination, source);
    } else {
        push_tiled_sprite_quads(quads, rect, virtual_size, destination, source);
    }
}
fn tile_quads(
    rect: SpriteUvRect,
    tile: TileInfo,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Vec<SpriteQuad> {
    let mut quads = Vec::new();
    push_tiled_sprite_quads(
        &mut quads,
        rect,
        [tile.width as f32, tile.height as f32],
        [x, y, width, height],
        [0.0, 0.0, tile.width as f32, tile.height as f32],
    );
    quads
}
#[cfg(feature = "extensions")]
fn tile_quad_count(tile: TileInfo, width: u32, height: u32) -> Option<u32> {
    if width == 0 || height == 0 || tile.width == 0 || tile.height == 0 {
        return None;
    }
    width
        .div_ceil(tile.width)
        .checked_mul(height.div_ceil(tile.height))
}
fn push_nine_slice_segment_quads(
    quads: &mut Vec<SpriteQuad>,
    rect: SpriteUvRect,
    nine_slice: NineSliceInfo,
    destination: [f32; 4],
    source: [f32; 4],
    inner: bool,
) {
    if inner {
        push_nine_slice_inner_quads(quads, rect, nine_slice, destination, source);
    } else {
        push_stretched_sprite_quad(
            quads,
            rect,
            [nine_slice.width as f32, nine_slice.height as f32],
            destination,
            source,
        );
    }
}
fn nine_slice_quads(
    rect: SpriteUvRect,
    nine_slice: NineSliceInfo,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Vec<SpriteQuad> {
    let mut quads = Vec::new();
    let sprite_width = nine_slice.width as f32;
    let sprite_height = nine_slice.height as f32;
    if width <= 0.0 || height <= 0.0 || sprite_width <= 0.0 || sprite_height <= 0.0 {
        return quads;
    }
    let border_left = (nine_slice.border_left as f32).min((width * 0.5).floor());
    let border_right = (nine_slice.border_right as f32).min((width * 0.5).floor());
    let border_top = (nine_slice.border_top as f32).min((height * 0.5).floor());
    let border_bottom = (nine_slice.border_bottom as f32).min((height * 0.5).floor());
    let virtual_size = [sprite_width, sprite_height];
    if width == sprite_width && height == sprite_height {
        push_stretched_sprite_quad(
            &mut quads,
            rect,
            virtual_size,
            [x, y, width, height],
            [0.0, 0.0, sprite_width, sprite_height],
        );
    } else if height == sprite_height {
        let destination_x = [x, x + border_left, x + width - border_right];
        let destination_width = [
            border_left,
            width - border_right - border_left,
            border_right,
        ];
        let source_x = [0.0, border_left, sprite_width - border_right];
        let source_width = [
            border_left,
            sprite_width - border_right - border_left,
            border_right,
        ];
        for column in 0..3 {
            push_nine_slice_segment_quads(
                &mut quads,
                rect,
                nine_slice,
                [destination_x[column], y, destination_width[column], height],
                [source_x[column], 0.0, source_width[column], sprite_height],
                column == 1,
            );
        }
    } else if width == sprite_width {
        let destination_y = [y, y + border_top, y + height - border_bottom];
        let destination_height = [
            border_top,
            height - border_bottom - border_top,
            border_bottom,
        ];
        let source_y = [0.0, border_top, sprite_height - border_bottom];
        let source_height = [
            border_top,
            sprite_height - border_bottom - border_top,
            border_bottom,
        ];
        for row in 0..3 {
            push_nine_slice_segment_quads(
                &mut quads,
                rect,
                nine_slice,
                [x, destination_y[row], width, destination_height[row]],
                [0.0, source_y[row], sprite_width, source_height[row]],
                row == 1,
            );
        }
    } else {
        let destination_x = [x, x + border_left, x + width - border_right];
        let destination_width = [
            border_left,
            width - border_right - border_left,
            border_right,
        ];
        let source_x = [0.0, border_left, sprite_width - border_right];
        let source_width = [
            border_left,
            sprite_width - border_right - border_left,
            border_right,
        ];
        let destination_y = [y, y + border_top, y + height - border_bottom];
        let destination_height = [
            border_top,
            height - border_bottom - border_top,
            border_bottom,
        ];
        let source_y = [0.0, border_top, sprite_height - border_bottom];
        let source_height = [
            border_top,
            sprite_height - border_bottom - border_top,
            border_bottom,
        ];
        for row in [0, 2, 1] {
            for column in 0..3 {
                push_nine_slice_segment_quads(
                    &mut quads,
                    rect,
                    nine_slice,
                    [
                        destination_x[column],
                        destination_y[row],
                        destination_width[column],
                        destination_height[row],
                    ],
                    [
                        source_x[column],
                        source_y[row],
                        source_width[column],
                        source_height[row],
                    ],
                    row == 1 || column == 1,
                );
            }
        }
    }
    quads
}
#[cfg(feature = "extensions")]
fn nine_slice_quad_count(info: NineSliceInfo, width: u32, height: u32) -> Option<u32> {
    if width == 0
        || height == 0
        || info.width == 0
        || info.height == 0
        || info.border_left.checked_add(info.border_right)? >= info.width
        || info.border_top.checked_add(info.border_bottom)? >= info.height
    {
        return None;
    }
    let left = info.border_left.min(width / 2);
    let right = info.border_right.min(width / 2);
    let top = info.border_top.min(height / 2);
    let bottom = info.border_bottom.min(height / 2);
    let count = |w: u32, h: u32, sw: u32, sh: u32, inner: bool| {
        if w == 0 || h == 0 {
            Some(0)
        } else if !inner || info.stretch_inner {
            Some(1)
        } else {
            w.div_ceil(sw).checked_mul(h.div_ceil(sh))
        }
    };
    if width == info.width && height == info.height {
        return Some(1);
    }
    if height == info.height {
        return count(left, height, left, info.height, false)?
            .checked_add(count(
                width - left - right,
                height,
                info.width - left - right,
                info.height,
                true,
            )?)?
            .checked_add(count(right, height, right, info.height, false)?);
    }
    if width == info.width {
        return count(width, top, info.width, top, false)?
            .checked_add(count(
                width,
                height - top - bottom,
                info.width,
                info.height - top - bottom,
                true,
            )?)?
            .checked_add(count(width, bottom, info.width, bottom, false)?);
    }
    let destination_widths = [left, width - left - right, right];
    let destination_heights = [top, height - top - bottom, bottom];
    let source_widths = [left, info.width - left - right, right];
    let source_heights = [top, info.height - top - bottom, bottom];
    let mut total = 0u32;
    for row in 0..3 {
        for column in 0..3 {
            total = total.checked_add(count(
                destination_widths[column],
                destination_heights[row],
                source_widths[column],
                source_heights[row],
                row == 1 || column == 1,
            )?)?;
        }
    }
    Some(total)
}
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GuiVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub color: [f32; 4],
}
fn clip_gui_triangle(triangle: [GuiVertex; 3], rect: [f32; 4]) -> Vec<GuiVertex> {
    let [left, top, right, bottom] = rect;
    if left >= right || top >= bottom {
        return Vec::new();
    }
    let mut polygon = triangle.to_vec();
    for (axis, edge, lower) in [
        (0, left, true),
        (0, right, false),
        (1, top, true),
        (1, bottom, false),
    ] {
        let Some(mut previous) = polygon.last().copied() else {
            break;
        };
        let inside = |vertex: GuiVertex| {
            if lower {
                vertex.position[axis] >= edge
            } else {
                vertex.position[axis] <= edge
            }
        };
        let mut previous_inside = inside(previous);
        let mut output = Vec::with_capacity(7);
        for current in polygon {
            let current_inside = inside(current);
            if previous_inside != current_inside {
                let ratio = (edge - previous.position[axis])
                    / (current.position[axis] - previous.position[axis]);
                let mut intersection = GuiVertex {
                    position: std::array::from_fn(|index| {
                        previous.position[index]
                            + ratio * (current.position[index] - previous.position[index])
                    }),
                    tex_coords: std::array::from_fn(|index| {
                        previous.tex_coords[index]
                            + ratio * (current.tex_coords[index] - previous.tex_coords[index])
                    }),
                    color: std::array::from_fn(|index| {
                        previous.color[index]
                            + ratio * (current.color[index] - previous.color[index])
                    }),
                };
                intersection.position[axis] = edge;
                output.push(intersection);
            }
            if current_inside {
                output.push(current);
            }
            previous = current;
            previous_inside = current_inside;
        }
        polygon = output;
    }
    polygon
}
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GuiUniforms {
    screen_size: [f32; 2],
    glint_time_millis: f32,
    glint_strength: f32,
}
#[derive(Debug, Clone, PartialEq)]
pub enum DrawBatch {
    EndPortal {
        sky_idx: usize,
        portal_idx: usize,
        start_idx: u32,
        count: u32,
    },
    Vignette {
        bind_group_idx: usize,
        start_idx: u32,
        count: u32,
    },
    Solid {
        start_idx: u32,
        count: u32,
    },
    Color {
        start_idx: u32,
        count: u32,
    },
    ColorInvertRect {
        start_idx: u32,
        count: u32,
    },
    ColorAdditiveRect {
        start_idx: u32,
        count: u32,
    },
    GuiAtlas {
        start_idx: u32,
        count: u32,
    },
    GuiGlintAtlas {
        start_idx: u32,
        count: u32,
    },
    Crosshair {
        start_idx: u32,
        count: u32,
    },
    FontSheet {
        sheet_idx: usize,
        start_idx: u32,
        count: u32,
    },
    CustomTexture {
        bind_group_idx: usize,
        start_idx: u32,
        count: u32,
    },
    ItemCutout {
        bind_group_idx: Option<usize>,
        start_idx: u32,
        count: u32,
    },
    GuiGlintCustomTexture {
        bind_group_idx: usize,
        start_idx: u32,
        count: u32,
    },
}
#[derive(Clone, Copy, Debug)]
pub struct GuiItemFoilMarker {
    index_start: usize,
    batch_start: usize,
    x: f32,
    y: f32,
    foil_uv_scale: f32,
    enabled: bool,
}
fn is_special_animated_foil_item(item: &ItemStack) -> bool {
    item.item.location.namespace == "minecraft"
        && matches!(
            item.item.location.path.as_str(),
            "clock" | "compass" | "recovery_compass"
        )
}
const GUI_BUFFER_SLOT_COUNT: usize = 3;
const GUI_INITIAL_VERTEX_CAPACITY: u64 = 262_144;
const GUI_INITIAL_INDEX_CAPACITY: u64 = 65_536;
const GUI_BUFFER_CAPACITY_INCREMENT: u64 = 262_144;
fn gui_buffer_capacity(current: u64, required: u64) -> u64 {
    if required <= current {
        return current;
    }
    let rounded = required.saturating_add(GUI_BUFFER_CAPACITY_INCREMENT - 1)
        / GUI_BUFFER_CAPACITY_INCREMENT
        * GUI_BUFFER_CAPACITY_INCREMENT;
    rounded.max(required)
}
#[derive(Default)]
struct GuiBufferSlot {
    vertex_buffer: Option<Buffer>,
    vertex_capacity: u64,
    index_buffer: Option<Buffer>,
    index_capacity: u64,
}
impl GuiBufferSlot {
    fn new(device: &Device, vertex_capacity: u64, index_capacity: u64) -> Self {
        let vertex_buffer = if vertex_capacity > 0 {
            Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("GUI Dynamic Vertex Buffer"),
                size: vertex_capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }))
        } else {
            None
        };
        let index_buffer = if index_capacity > 0 {
            Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("GUI Dynamic Index Buffer"),
                size: index_capacity,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }))
        } else {
            None
        };
        Self {
            vertex_buffer,
            vertex_capacity,
            index_buffer,
            index_capacity,
        }
    }
    fn upload(&mut self, device: &Device, queue: &Queue, vertex_bytes: &[u8], index_bytes: &[u8]) {
        let required_vertex_capacity = vertex_bytes.len() as u64;
        let vertex_capacity = gui_buffer_capacity(self.vertex_capacity, required_vertex_capacity);
        if required_vertex_capacity > 0
            && (self.vertex_buffer.is_none() || vertex_capacity != self.vertex_capacity)
        {
            self.vertex_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("GUI Dynamic Vertex Buffer"),
                size: vertex_capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.vertex_capacity = vertex_capacity;
        }
        if !vertex_bytes.is_empty()
            && let Some(vertex_buffer) = &self.vertex_buffer
        {
            queue.write_buffer(vertex_buffer, 0, vertex_bytes);
        }
        let required_index_capacity = index_bytes.len() as u64;
        let index_capacity = gui_buffer_capacity(self.index_capacity, required_index_capacity);
        if required_index_capacity > 0
            && (self.index_buffer.is_none() || index_capacity != self.index_capacity)
        {
            self.index_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("GUI Dynamic Index Buffer"),
                size: index_capacity,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.index_capacity = index_capacity;
        }
        if !index_bytes.is_empty()
            && let Some(index_buffer) = &self.index_buffer
        {
            queue.write_buffer(index_buffer, 0, index_bytes);
        }
    }
}
pub struct GuiRenderer {
    win_effects: win_effects::WinPipelines,
    pub(super) portal_game_time: f32,
    pipeline: RenderPipeline,
    item_cutout_pipeline: RenderPipeline,
    color_pipeline: RenderPipeline,
    color_unorm_pipeline: Option<RenderPipeline>,
    invert_pipeline: RenderPipeline,
    color_additive_pipeline: RenderPipeline,
    color_additive_unorm_pipeline: Option<RenderPipeline>,
    glint_pipeline: RenderPipeline,
    uniform_buffer: Buffer,
    uniform_bind_group: BindGroup,
    white_bind_group: BindGroup,
    _glint_texture_view: Option<TextureView>,
    glint_bind_group: Option<BindGroup>,
    glint_started_at: Instant,
    item_model_cooldown: f32,
    pub gui_atlas: TextureAtlas,
    pub item_flat_models: HashMap<ResourceLocation, GuiFlatItemModel>,
    pub item_block_models: HashMap<ResourceLocation, GuiBlockItemModel>,
    pub item_special_models: HashMap<ResourceLocation, GuiSpecialItemModel>,
    pub item_definitions: HashMap<ResourceLocation, serde_json::Value>,
    buffer_slots: [GuiBufferSlot; GUI_BUFFER_SLOT_COUNT],
    active_buffer_slot: Option<usize>,
    next_buffer_slot: usize,
    pub vertices: Vec<GuiVertex>,
    pub indices: Vec<u32>,
    pub batches: Vec<DrawBatch>,
    sealed_batches: usize,
    pub custom_textures: Vec<Option<(TextureView, BindGroup)>>,
    #[cfg(feature = "extensions")]
    mod_canvases: canvas::CanvasTextures,
    pub custom_texture_registry: std::collections::HashMap<ResourceLocation, usize>,
    dynamic_texture_attempted: HashSet<ResourceLocation>,
    player_head_download_tx: Sender<(ResourceLocation, String)>,
    player_head_download_rx: Receiver<(ResourceLocation, String, Option<RgbaImage>)>,
    player_profile_skin_urls: HashMap<uuid::Uuid, String>,
    pub active_scissor: Option<[f32; 4]>,
}
#[derive(Clone, Debug)]
pub struct GuiBlockItemModel {
    quads: Vec<GuiBlockItemQuad>,
}
#[derive(Clone, Debug)]
pub struct GuiFlatItemModel {
    layers: Vec<GuiFlatItemLayer>,
    pub gui_transform: Option<crate::render::model::ItemTransform>,
}
#[derive(Clone, Debug)]
struct GuiFlatItemLayer {
    texture: ResourceLocation,
    tint: GuiItemTintSource,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum GuiItemTintSource {
    Constant([f32; 4]),
    Potion { default_color: [f32; 4] },
}
impl GuiItemTintSource {
    fn default_color(self) -> [f32; 4] {
        match self {
            Self::Constant(color)
            | Self::Potion {
                default_color: color,
            } => color,
        }
    }
    fn resolve(self, item: &ItemStack) -> [f32; 4] {
        match self {
            Self::Constant(color) => color,
            Self::Potion { default_color } => {
                get_potion_color_or(item, default_color).unwrap_or(default_color)
            }
        }
    }
}
#[derive(Clone, Debug)]
pub struct GuiSpecialItemModel {
    kind: GuiSpecialItemKind,
    texture: ResourceLocation,
    crop_pixels: Option<[f32; 4]>,
    tint: [f32; 4],
    gui_transform: Mat4,
    banner_color: Option<String>,
    chest: GuiChestParams,
}
type GuiResolvedItemModel = (
    ResourceLocation,
    Mat4,
    Vec<[f32; 4]>,
    Option<GuiSpecialItemModel>,
);
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct GuiChestParams {
    openness: f32,
    chest_type: GuiChestType,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum GuiChestType {
    #[default]
    Single,
    Left,
    Right,
}
impl GuiChestType {
    fn from_serialized_name(name: &str) -> Option<Self> {
        match name {
            "single" => Some(Self::Single),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GuiSpecialItemKind {
    Shield,
    Banner,
    Chest,
    ShulkerBox,
    DecoratedPot,
    Conduit,
    Trident,
    Head,
    CopperGolemStatue,
}
#[derive(Clone, Debug)]
struct GuiBlockItemQuad {
    texture: ResourceLocation,
    positions: [[f32; 2]; 4],
    uvs: [[f32; 2]; 4],
    shade: f32,
    depth: f32,
    tint: [f32; 4],
}
#[derive(Clone, Debug)]
struct GuiModelPart {
    pose: GuiPartPose,
    cubes: Vec<GuiModelCube>,
    children: Vec<GuiModelPart>,
    color: [f32; 4],
}
#[derive(Clone, Copy, Debug)]
struct GuiPartPose {
    offset: Vec3,
    rotation: Vec3,
    scale: Vec3,
}
#[derive(Clone, Copy, Debug)]
struct GuiModelCube {
    tex_offs: [f32; 2],
    origin: Vec3,
    dimensions: Vec3,
    grow: Vec3,
    mirror: bool,
    tex_size: [f32; 2],
    visible_faces: [bool; 6],
}
#[derive(Clone, Copy, Debug)]
struct GuiModelVertex3 {
    pos: Vec3,
    uv: [f32; 2],
}
#[derive(Clone, Copy, Debug)]
struct GuiModelPolygon {
    vertices: [GuiModelVertex3; 4],
    normal: Vec3,
}
#[derive(Clone, Debug)]
struct GuiProjectedModelQuad {
    positions: [[f32; 2]; 4],
    uvs: [[f32; 2]; 4],
    normal: Vec3,
    depth: f32,
    color: [f32; 4],
}
impl GuiModelPart {
    fn new(
        pose: GuiPartPose,
        cubes: Vec<GuiModelCube>,
        children: Vec<GuiModelPart>,
        color: [f32; 4],
    ) -> Self {
        Self {
            pose,
            cubes,
            children,
            color,
        }
    }
}
impl GuiPartPose {
    fn zero() -> Self {
        Self {
            offset: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
    fn offset(x: f32, y: f32, z: f32) -> Self {
        Self {
            offset: Vec3::new(x, y, z),
            ..Self::zero()
        }
    }
    fn offset_and_rotation(x: f32, y: f32, z: f32, x_rot: f32, y_rot: f32, z_rot: f32) -> Self {
        Self {
            offset: Vec3::new(x, y, z),
            rotation: Vec3::new(x_rot, y_rot, z_rot),
            scale: Vec3::ONE,
        }
    }
    fn scaled(mut self, scale: f32) -> Self {
        self.scale = Vec3::splat(scale);
        self
    }
    fn matrix(self) -> Mat4 {
        Mat4::from_translation(self.offset / 16.0)
            * Mat4::from_euler(
                EulerRot::ZYX,
                self.rotation.z,
                self.rotation.y,
                self.rotation.x,
            )
            * Mat4::from_scale(self.scale)
    }
}
impl GuiModelCube {
    fn new(tex_offs: [f32; 2], origin: Vec3, dimensions: Vec3, tex_size: [f32; 2]) -> Self {
        Self {
            tex_offs,
            origin,
            dimensions,
            grow: Vec3::ZERO,
            mirror: false,
            tex_size,
            visible_faces: all_model_faces(),
        }
    }
    fn grown(mut self, grow: f32) -> Self {
        self.grow = Vec3::splat(grow);
        self
    }
    fn mirrored(mut self) -> Self {
        self.mirror = true;
        self
    }
    fn visible_faces(mut self, faces: [bool; 6]) -> Self {
        self.visible_faces = faces;
        self
    }
    fn polygons(self) -> Vec<GuiModelPolygon> {
        let mut min_x = self.origin.x - self.grow.x;
        let min_y = self.origin.y - self.grow.y;
        let min_z = self.origin.z - self.grow.z;
        let mut max_x = self.origin.x + self.dimensions.x + self.grow.x;
        let max_y = self.origin.y + self.dimensions.y + self.grow.y;
        let max_z = self.origin.z + self.dimensions.z + self.grow.z;
        if self.mirror {
            std::mem::swap(&mut min_x, &mut max_x);
        }
        let t0 = GuiModelVertex3::new(min_x, min_y, min_z);
        let t1 = GuiModelVertex3::new(max_x, min_y, min_z);
        let t2 = GuiModelVertex3::new(max_x, max_y, min_z);
        let t3 = GuiModelVertex3::new(min_x, max_y, min_z);
        let l0 = GuiModelVertex3::new(min_x, min_y, max_z);
        let l1 = GuiModelVertex3::new(max_x, min_y, max_z);
        let l2 = GuiModelVertex3::new(max_x, max_y, max_z);
        let l3 = GuiModelVertex3::new(min_x, max_y, max_z);
        let width = self.dimensions.x;
        let height = self.dimensions.y;
        let depth = self.dimensions.z;
        let u0 = self.tex_offs[0];
        let u1 = self.tex_offs[0] + depth;
        let u2 = self.tex_offs[0] + depth + width;
        let u22 = self.tex_offs[0] + depth + width + width;
        let u3 = self.tex_offs[0] + depth + width + depth;
        let u4 = self.tex_offs[0] + depth + width + depth + width;
        let v0 = self.tex_offs[1];
        let v1 = self.tex_offs[1] + depth;
        let v2 = self.tex_offs[1] + depth + height;
        let mut polygons = Vec::with_capacity(6);
        self.push_polygon(
            &mut polygons,
            Direction::Down,
            [l1, l0, t0, t1],
            [u1, v0, u2, v1],
        );
        self.push_polygon(
            &mut polygons,
            Direction::Up,
            [t2, t3, l3, l2],
            [u2, v1, u22, v0],
        );
        self.push_polygon(
            &mut polygons,
            Direction::West,
            [t0, l0, l3, t3],
            [u0, v1, u1, v2],
        );
        self.push_polygon(
            &mut polygons,
            Direction::North,
            [t1, t0, t3, t2],
            [u1, v1, u2, v2],
        );
        self.push_polygon(
            &mut polygons,
            Direction::East,
            [l1, t1, t2, l2],
            [u2, v1, u3, v2],
        );
        self.push_polygon(
            &mut polygons,
            Direction::South,
            [l0, l1, l2, l3],
            [u3, v1, u4, v2],
        );
        polygons
    }
    fn push_polygon(
        self,
        polygons: &mut Vec<GuiModelPolygon>,
        facing: Direction,
        vertices: [GuiModelVertex3; 4],
        uv_rect: [f32; 4],
    ) {
        if !self.visible_faces[direction_index(facing)] {
            return;
        }
        polygons.push(gui_model_polygon(
            vertices,
            uv_rect,
            self.tex_size,
            self.mirror,
            facing,
        ));
    }
}
impl GuiModelVertex3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            pos: Vec3::new(x, y, z),
            uv: [0.0, 0.0],
        }
    }
}
impl GuiRenderer {
    pub fn new(
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        resource_manager: &ClientResourceManager,
        target_format: wgpu::TextureFormat,
    ) -> Result<Self, ResourceError> {
        let uniforms = GuiUniforms {
            screen_size: [800.0, 600.0],
            glint_time_millis: 0.0,
            glint_strength: 0.75,
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GUI Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("gui_uniform_bind_group_layout"),
            });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("gui_uniform_bind_group"),
        });
        let white_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("White Fallback Texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
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
                texture: &white_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255, 255, 255, 255],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let white_view = white_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let white_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let white_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&white_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&white_sampler),
                },
            ],
            label: Some("White Bind Group"),
        });
        let glint_texture_id = ResourceLocation::new("minecraft", "misc/enchanted_glint_item");
        let (glint_texture_view, glint_bind_group) = match resource_manager
            .read_image(&glint_texture_id)
        {
            Ok(image) => {
                let rgba = image.to_rgba8();
                let (width, height) = rgba.dimensions();
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("GUI Enchanted Item Glint Texture"),
                    size: wgpu::Extent3d {
                        width,
                        height,
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
                    rgba.as_raw(),
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * width),
                        rows_per_image: Some(height),
                    },
                    wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                );
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                    address_mode_u: wgpu::AddressMode::Repeat,
                    address_mode_v: wgpu::AddressMode::Repeat,
                    address_mode_w: wgpu::AddressMode::Repeat,
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    mipmap_filter: wgpu::FilterMode::Linear,
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
                    label: Some("GUI Enchanted Item Glint Bind Group"),
                });
                (Some(view), Some(bind_group))
            }
            Err(error) => {
                log::warn!(
                    "[GUI] enchanted glint texture unavailable; foil rendering is disabled: {error}"
                );
                (None, None)
            }
        };
        log::info!("Scanning and loading GUI sprites...");
        let mut texture_ids = Vec::new();
        for namespace in resource_manager.namespaces()? {
            texture_ids.extend(resource_manager.list_texture_ids_in_directory(
                &namespace,
                "gui/sprites",
                "gui/sprites/",
            )?);
            texture_ids.extend(resource_manager.list_texture_ids_in_directory(
                &namespace,
                "mob_effect",
                "mob_effect/",
            )?);
        }
        if let Ok(item_texture_ids) =
            resource_manager.list_texture_ids_in_directory("minecraft", "item", "item/")
        {
            texture_ids.extend(item_texture_ids);
        }
        if let Ok(block_texture_ids) =
            resource_manager.list_texture_ids_in_directory("minecraft", "block", "block/")
        {
            texture_ids.extend(block_texture_ids);
        }
        for (source, prefix) in [
            ("entity/banner", "entity/banner/"),
            ("entity/chest", "entity/chest/"),
            ("entity/conduit", "entity/conduit/"),
            ("entity/copper_golem", "entity/copper_golem/"),
            ("entity/creeper", "entity/creeper/"),
            ("entity/decorated_pot", "entity/decorated_pot/"),
            ("entity/enderdragon", "entity/enderdragon/"),
            ("entity/piglin", "entity/piglin/"),
            ("entity/player/wide", "entity/player/wide/"),
            ("entity/shield", "entity/shield/"),
            ("entity/shulker", "entity/shulker/"),
            ("entity/skeleton", "entity/skeleton/"),
            ("entity/trident", "entity/trident/"),
            ("entity/zombie", "entity/zombie/"),
        ] {
            if let Ok(entity_texture_ids) =
                resource_manager.list_texture_ids_in_directory("minecraft", source, prefix)
            {
                texture_ids.extend(entity_texture_ids);
            }
        }
        #[cfg(feature = "extensions")]
        {
            if let Some(catalog) = crate::modding::client_content::catalog() {
                let loaded = crate::modding::resources::load_models(resource_manager, &catalog)
                    .map_err(|e| ResourceError::PreparedSnapshot {
                        operation: "load mod GUI assets",
                        message: e.to_string(),
                    })?;
                for tex in loaded.textures {
                    if !texture_ids.contains(&tex) {
                        texture_ids.push(tex);
                    }
                }
            }
        }
        let sprite_ids: Vec<ResourceLocation> = texture_ids
            .iter()
            .map(gui_sprite_id_from_texture_id)
            .collect();
        let gui_atlas = TextureAtlas::from_resource_id_map_with_options(
            device,
            queue,
            texture_bind_group_layout,
            resource_manager,
            &sprite_ids,
            &texture_ids,
            TextureAtlasOptions::GUI,
        )?;
        let (item_flat_models, item_block_models, item_special_models, item_definitions) =
            build_item_model_texture_index(resource_manager);
        #[cfg(feature = "extensions")]
        let (mut item_flat_models, mut item_block_models) = (item_flat_models, item_block_models);
        #[cfg(feature = "extensions")]
        {
            if let Some(catalog) = crate::modding::client_content::catalog() {
                for entry in catalog.entries() {
                    let el_mod_content::spec::ContentSpec::Item { id, model, .. } = &entry.spec
                    else {
                        continue;
                    };
                    let item_loc = ResourceLocation::from_string(id.as_str());
                    let model_loc = ResourceLocation::from_string(model.as_str());
                    let material = crate::modding::material::load(resource_manager, model)
                        .map_err(|message| ResourceError::PreparedSnapshot {
                            operation: "load mod item material",
                            message,
                        })?;
                    let tint = material.color.map(|v| v as f32 / 255.0);
                    if let Some(mut flat) =
                        flat_item_model_from_model(resource_manager, &model_loc, &[])
                    {
                        for layer in &mut flat.layers {
                            let original = layer.tint.default_color();
                            layer.tint = GuiItemTintSource::Constant(std::array::from_fn(|i| {
                                original[i] * tint[i]
                            }));
                        }
                        item_flat_models.insert(item_loc, flat);
                    } else if let Some(mut block) = gui_block_item_model_from_composite(
                        resource_manager,
                        &[(model_loc, Mat4::IDENTITY, Vec::new())],
                    ) {
                        for quad in &mut block.quads {
                            for (channel, multiplier) in quad.tint.iter_mut().zip(tint) {
                                *channel *= multiplier;
                            }
                        }
                        item_block_models.insert(item_loc, block);
                    } else {
                        return Err(ResourceError::PreparedSnapshot {
                            operation: "load mod GUI assets",
                            message: format!("missing mod item model for {}", item_loc),
                        });
                    }
                }
            }
        }
        log::info!("Packed GUI atlas with {} sprites", sprite_ids.len());
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GUI Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gui.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GUI Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, texture_bind_group_layout],
            push_constant_ranges: &[],
        });
        let win_effects = win_effects::WinPipelines::new(
            device,
            target_format,
            &uniform_bind_group_layout,
            texture_bind_group_layout,
        );
        let make_textured_pipeline = |label, fragment_entry| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GuiVertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x2, 1 => Float32x2, 2 => Float32x4
                        ],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: fragment_entry,
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Always,
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
        let pipeline = make_textured_pipeline("GUI Render Pipeline", "fs_main");
        let item_cutout_pipeline =
            make_textured_pipeline("GUI Item Cutout Pipeline", "fs_item_cutout");
        let color_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GUI Color Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gui_color.wgsl").into()),
        });
        let color_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("GUI Color Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });
        let make_color_pipeline = |format: wgpu::TextureFormat,
                                   label: &'static str,
                                   render_pass_has_depth: bool,
                                   blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&color_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &color_shader,
                    entry_point: "vs_main",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GuiVertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x2, 1 => Float32x2, 2 => Float32x4
                        ],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &color_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: render_pass_has_depth.then_some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Always,
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
        let color_additive_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let color_pipeline = make_color_pipeline(
            target_format,
            "GUI Color Pipeline",
            true,
            wgpu::BlendState::ALPHA_BLENDING,
        );
        let color_unorm_pipeline = target_format.is_srgb().then(|| {
            make_color_pipeline(
                target_format.remove_srgb_suffix(),
                "GUI Color UNORM Pipeline",
                false,
                wgpu::BlendState::ALPHA_BLENDING,
            )
        });
        let color_additive_pipeline = make_color_pipeline(
            target_format,
            "GUI Color Additive Pipeline",
            true,
            color_additive_blend,
        );
        let color_additive_unorm_pipeline = target_format.is_srgb().then(|| {
            make_color_pipeline(
                target_format.remove_srgb_suffix(),
                "GUI Color Additive UNORM Pipeline",
                false,
                color_additive_blend,
            )
        });
        let invert_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GUI Invert Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GuiVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2, 1 => Float32x2, 2 => Float32x4
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::OneMinusDst,
                            dst_factor: wgpu::BlendFactor::OneMinusSrc,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        let glint_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GUI Item Glint Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gui_glint.wgsl").into()),
        });
        let glint_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("GUI Item Glint Pipeline Layout"),
                bind_group_layouts: &[
                    &uniform_bind_group_layout,
                    texture_bind_group_layout,
                    texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let glint_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GUI Item Glint Pipeline"),
            layout: Some(&glint_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &glint_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GuiVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2, 1 => Float32x2, 2 => Float32x4
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &glint_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Src,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        let (download_tx, download_rx) = mpsc::channel::<(ResourceLocation, String)>();
        let (result_tx, result_rx) =
            mpsc::channel::<(ResourceLocation, String, Option<RgbaImage>)>();
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::{Arc, Mutex};
            let shared_rx = Arc::new(Mutex::new(download_rx));
            let worker_count = std::thread::available_parallelism()
                .map(|n| n.get().clamp(1, 8))
                .unwrap_or(4);
            for i in 0..worker_count {
                let shared_rx = Arc::clone(&shared_rx);
                let result_tx = result_tx.clone();
                let _ = std::thread::Builder::new()
                    .name(format!("eltharn-player-head-download-{i}"))
                    .spawn(move || {
                        loop {
                            let next = {
                                let rx = shared_rx
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                                rx.recv()
                            };
                            let Ok((id, url)) = next else { break };
                            let cache_path = {
                                use sha1::Digest;
                                let mut hasher = sha1::Sha1::new();
                                hasher.update(url.as_bytes());
                                let result = hasher.finalize();
                                let mut hash = String::with_capacity(40);
                                for b in result {
                                    use std::fmt::Write;
                                    let _ = write!(hash, "{b:02x}");
                                }
                                let prefix = if hash.len() >= 2 { &hash[0..2] } else { "xx" };
                                dirs::cache_dir()
                                    .unwrap_or_else(std::env::temp_dir)
                                    .join("eltharn")
                                    .join("assets")
                                    .join("skins")
                                    .join(prefix)
                                    .join(format!("{hash}.png"))
                            };
                            let image = if cache_path.is_file() {
                                std::fs::read(&cache_path)
                                    .ok()
                                    .and_then(|bytes| image::load_from_memory(&bytes).ok())
                                    .map(|image| image.to_rgba8())
                            } else {
                                reqwest::blocking::get(&url)
                                    .and_then(|response| response.error_for_status())
                                    .and_then(|response| response.bytes())
                                    .ok()
                                    .and_then(|bytes| {
                                        if let Some(parent) = cache_path.parent() {
                                            let _ = std::fs::create_dir_all(parent);
                                        }
                                        let temp_path = cache_path
                                            .with_extension(format!("tmp.{}", std::process::id()));
                                        if std::fs::write(&temp_path, &bytes).is_ok() {
                                            let _ = std::fs::rename(&temp_path, &cache_path);
                                        }
                                        image::load_from_memory(&bytes).ok()
                                    })
                                    .map(|image| image.to_rgba8())
                            };
                            if result_tx.send((id, url, image)).is_err() {
                                break;
                            }
                        }
                    });
            }
        }
        #[cfg(target_arch = "wasm32")]
        drop((download_rx, result_tx));
        Ok(Self {
            win_effects,
            portal_game_time: 0.0,
            pipeline,
            item_cutout_pipeline,
            color_pipeline,
            color_unorm_pipeline,
            invert_pipeline,
            color_additive_pipeline,
            color_additive_unorm_pipeline,
            glint_pipeline,
            uniform_buffer,
            uniform_bind_group,
            white_bind_group,
            _glint_texture_view: glint_texture_view,
            glint_bind_group,
            glint_started_at: Instant::now(),
            item_model_cooldown: 0.0,
            gui_atlas,
            item_flat_models,
            item_block_models,
            item_special_models,
            item_definitions,
            buffer_slots: std::array::from_fn(|_| {
                GuiBufferSlot::new(
                    device,
                    GUI_INITIAL_VERTEX_CAPACITY,
                    GUI_INITIAL_INDEX_CAPACITY,
                )
            }),
            active_buffer_slot: None,
            next_buffer_slot: 0,
            vertices: Vec::with_capacity(8192),
            indices: Vec::with_capacity(16384),
            batches: Vec::with_capacity(256),
            sealed_batches: 0,
            custom_textures: Vec::new(),
            #[cfg(feature = "extensions")]
            mod_canvases: canvas::CanvasTextures::default(),
            custom_texture_registry: std::collections::HashMap::new(),
            dynamic_texture_attempted: HashSet::new(),
            player_head_download_tx: download_tx,
            player_head_download_rx: result_rx,
            player_profile_skin_urls: HashMap::new(),
            active_scissor: None,
        })
    }
    pub fn load_custom_texture(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        resource_manager: &ClientResourceManager,
        id: &ResourceLocation,
    ) -> Result<usize, ResourceError> {
        let img = resource_manager.read_image(id)?;
        let rgba = img.to_rgba8();
        Ok(self.insert_custom_texture_rgba(device, queue, texture_bind_group_layout, id, &rgba))
    }
    pub fn register_dynamic_texture_rgba(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        id: &ResourceLocation,
        rgba: &RgbaImage,
    ) -> usize {
        let idx =
            self.insert_custom_texture_rgba(device, queue, texture_bind_group_layout, id, rgba);
        self.custom_texture_registry.insert(id.clone(), idx);
        idx
    }
    pub fn set_player_profile_skin_urls(&mut self, skins: &[(uuid::Uuid, Option<String>)]) {
        self.player_profile_skin_urls.clear();
        for (uuid, url) in skins {
            if let Some(url) = url.as_deref() {
                self.player_profile_skin_urls.insert(*uuid, url.to_owned());
            }
        }
    }
    fn resolved_player_head_skin_url(&self, item: &ItemStack) -> Option<String> {
        player_head_skin_url(item).or_else(|| {
            player_head_profile_uuid(item)
                .and_then(|uuid| self.player_profile_skin_urls.get(&uuid).cloned())
        })
    }
    pub fn prepare_player_head_texture(&mut self, item: &ItemStack) {
        if item.item.location.path != "player_head" {
            return;
        }
        let Some(url) = self.resolved_player_head_skin_url(item) else {
            return;
        };
        let id = if player_head_skin_url(item).is_some() {
            player_head_texture_id(&url)
        } else {
            player_skin_texture_id(&url)
        };
        if self.custom_texture_registry.contains_key(&id)
            || !self.dynamic_texture_attempted.insert(id.clone())
        {
            return;
        }
        if self
            .player_head_download_tx
            .send((id, url.clone()))
            .is_err()
        {
            log::warn!("[GUI] player-head download worker stopped URL={url}");
        }
    }
    pub fn prepare_player_head_default_texture(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        resource_manager: &ClientResourceManager,
        item: &ItemStack,
    ) {
        if item.item.location.path != "player_head"
            || self.resolved_player_head_skin_url(item).is_some()
        {
            return;
        }
        let uuid = player_head_profile_uuid(item).unwrap_or_else(uuid::Uuid::nil);
        self.prepare_default_player_skin_texture(
            device,
            queue,
            texture_bind_group_layout,
            resource_manager,
            uuid,
        );
    }
    pub fn prepare_player_skin_texture(&mut self, url: &str) {
        let id = player_skin_texture_id(url);
        if self.custom_texture_registry.contains_key(&id)
            || !self.dynamic_texture_attempted.insert(id.clone())
        {
            return;
        }
        if self
            .player_head_download_tx
            .send((id, url.to_owned()))
            .is_err()
        {
            log::warn!("[GUI] player-skin download worker stopped URL={url}");
        }
    }
    pub fn prepare_default_player_skin_texture(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        resource_manager: &ClientResourceManager,
        uuid: uuid::Uuid,
    ) {
        let path = default_player_skin_texture_path(uuid);
        let id = ResourceLocation::new("minecraft", &path);
        if self.custom_texture_registry.contains_key(&id)
            || !self.dynamic_texture_attempted.insert(id.clone())
        {
            return;
        }
        if let Ok(index) = self.load_custom_texture(
            device,
            queue,
            texture_bind_group_layout,
            resource_manager,
            &id,
        ) {
            self.custom_texture_registry.insert(id, index);
        }
    }
    pub fn poll_player_head_textures(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) {
        while let Ok((id, url, image)) = self.player_head_download_rx.try_recv() {
            let Some(image) = image else {
                log::warn!("[GUI] failed to download player-head skin URL={url}");
                continue;
            };
            let Some(image) = normalize_player_skin_image(image) else {
                log::warn!("[GUI] discarded player skin with unsupported dimensions URL={url}");
                continue;
            };
            self.register_dynamic_texture_rgba(
                device,
                queue,
                texture_bind_group_layout,
                &id,
                &image,
            );
        }
    }
    pub fn draw_player_face(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        uuid: uuid::Uuid,
        texture_url: Option<&str>,
        show_hat: bool,
        flip: bool,
        color: [f32; 4],
    ) -> bool {
        let id = texture_url.map(player_skin_texture_id).unwrap_or_else(|| {
            ResourceLocation::new("minecraft", &default_player_skin_texture_path(uuid))
        });
        let Some(&texture_idx) = self.custom_texture_registry.get(&id) else {
            return false;
        };
        let v0 = if flip { 16.0 / 64.0 } else { 8.0 / 64.0 };
        let v1 = if flip { 8.0 / 64.0 } else { 16.0 / 64.0 };
        self.draw_custom_texture_region(
            x,
            y,
            size,
            size,
            8.0 / 64.0,
            v0,
            16.0 / 64.0,
            v1,
            texture_idx,
            color,
        );
        if show_hat {
            self.draw_custom_texture_region(
                x,
                y,
                size,
                size,
                40.0 / 64.0,
                v0,
                48.0 / 64.0,
                v1,
                texture_idx,
                color,
            );
        }
        true
    }
    pub fn draw_player_head_item(
        &mut self,
        x: f32,
        y: f32,
        item: &ItemStack,
        color: [f32; 4],
    ) -> bool {
        if item.item.location.path != "player_head" {
            return false;
        }
        let id = if let Some(url) = self.resolved_player_head_skin_url(item) {
            if player_head_skin_url(item).is_some() {
                player_head_texture_id(&url)
            } else {
                player_skin_texture_id(&url)
            }
        } else {
            let uuid = player_head_profile_uuid(item).unwrap_or_else(uuid::Uuid::nil);
            ResourceLocation::new("minecraft", &default_player_skin_texture_path(uuid))
        };
        let Some(&texture_idx) = self.custom_texture_registry.get(&id) else {
            return false;
        };
        let Some(model) = self.item_special_models.get(&item.item.location).cloned() else {
            return false;
        };
        let parts = skull_gui_model_parts(
            &ResourceLocation::new("minecraft", "entity/player/wide/steve"),
            Some([8.0, 8.0, 8.0, 8.0]),
        );
        let tint = [
            color[0] * model.tint[0],
            color[1] * model.tint[1],
            color[2] * model.tint[2],
            color[3] * model.tint[3],
        ];
        let view_matrix = Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0)) * model.gui_transform;
        let indices_before = self.indices.len() as u32;
        let added = self.push_special_model_parts_custom(
            x,
            y,
            texture_idx,
            &parts,
            view_matrix,
            tint,
            true,
        );
        if added > 0 {
            self.append_item_cutout_batch(Some(texture_idx), indices_before, added);
            true
        } else {
            false
        }
    }
    fn insert_custom_texture_rgba(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        id: &ResourceLocation,
        rgba: &RgbaImage,
    ) -> usize {
        let (width, height) = rgba.dimensions();
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Custom GUI Texture: {}", id)),
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
            rgba.as_raw(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
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
            label: Some(&format!("Custom GUI Bind Group: {}", id)),
        });
        let idx = self.custom_textures.len();
        self.custom_textures.push(Some((view, bind_group)));
        idx
    }
    pub fn start_frame(&mut self, queue: &Queue, screen_width: f32, screen_height: f32) {
        self.start_frame_with_glint(queue, screen_width, screen_height, 0.5, 0.75);
    }
    pub fn start_frame_with_glint(
        &mut self,
        queue: &Queue,
        screen_width: f32,
        screen_height: f32,
        glint_speed: f32,
        glint_strength: f32,
    ) {
        self.vertices.clear();
        self.indices.clear();
        self.batches.clear();
        self.sealed_batches = 0;
        self.active_scissor = None;
        let uniforms = GuiUniforms {
            screen_size: [screen_width, screen_height],
            glint_time_millis: self.glint_started_at.elapsed().as_secs_f32()
                * 1000.0
                * glint_speed
                * 8.0,
            glint_strength,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }
    pub fn seal_batches(&mut self) {
        self.sealed_batches = self.batches.len();
    }
    #[inline]
    fn last_mergeable_batch_mut(&mut self) -> Option<&mut DrawBatch> {
        if self.batches.len() > self.sealed_batches {
            self.batches.last_mut()
        } else {
            None
        }
    }
    pub fn append_geometry(
        &mut self,
        vertices: &[GuiVertex],
        indices: &[u32],
        batches: &[DrawBatch],
    ) {
        if vertices.is_empty() || indices.is_empty() {
            return;
        }
        if self.vertices.is_empty() && self.indices.is_empty() {
            self.vertices.extend_from_slice(vertices);
            self.indices.extend_from_slice(indices);
            self.batches.extend_from_slice(batches);
            return;
        }
        let vertex_offset = self.vertices.len() as u32;
        let index_offset = self.indices.len() as u32;
        self.vertices.extend_from_slice(vertices);
        self.indices.reserve(indices.len());
        for &idx in indices {
            self.indices.push(idx + vertex_offset);
        }
        for batch in batches {
            let offset_batch = match *batch {
                DrawBatch::EndPortal {
                    sky_idx,
                    portal_idx,
                    start_idx,
                    count,
                } => DrawBatch::EndPortal {
                    sky_idx,
                    portal_idx,
                    start_idx: start_idx + index_offset,
                    count,
                },
                DrawBatch::Vignette {
                    bind_group_idx,
                    start_idx,
                    count,
                } => DrawBatch::Vignette {
                    bind_group_idx,
                    start_idx: start_idx + index_offset,
                    count,
                },
                DrawBatch::Solid { start_idx, count } => DrawBatch::Solid {
                    start_idx: start_idx + index_offset,
                    count,
                },
                DrawBatch::Color { start_idx, count } => DrawBatch::Color {
                    start_idx: start_idx + index_offset,
                    count,
                },
                DrawBatch::ColorInvertRect { start_idx, count } => DrawBatch::ColorInvertRect {
                    start_idx: start_idx + index_offset,
                    count,
                },
                DrawBatch::ColorAdditiveRect { start_idx, count } => DrawBatch::ColorAdditiveRect {
                    start_idx: start_idx + index_offset,
                    count,
                },
                DrawBatch::GuiAtlas { start_idx, count } => DrawBatch::GuiAtlas {
                    start_idx: start_idx + index_offset,
                    count,
                },
                DrawBatch::GuiGlintAtlas { start_idx, count } => DrawBatch::GuiGlintAtlas {
                    start_idx: start_idx + index_offset,
                    count,
                },
                DrawBatch::Crosshair { start_idx, count } => DrawBatch::Crosshair {
                    start_idx: start_idx + index_offset,
                    count,
                },
                DrawBatch::FontSheet {
                    sheet_idx,
                    start_idx,
                    count,
                } => DrawBatch::FontSheet {
                    sheet_idx,
                    start_idx: start_idx + index_offset,
                    count,
                },
                DrawBatch::CustomTexture {
                    bind_group_idx,
                    start_idx,
                    count,
                } => DrawBatch::CustomTexture {
                    bind_group_idx,
                    start_idx: start_idx + index_offset,
                    count,
                },
                DrawBatch::ItemCutout {
                    bind_group_idx,
                    start_idx,
                    count,
                } => DrawBatch::ItemCutout {
                    bind_group_idx,
                    start_idx: start_idx + index_offset,
                    count,
                },
                DrawBatch::GuiGlintCustomTexture {
                    bind_group_idx,
                    start_idx,
                    count,
                } => DrawBatch::GuiGlintCustomTexture {
                    bind_group_idx,
                    start_idx: start_idx + index_offset,
                    count,
                },
            };
            self.batches.push(offset_batch);
        }
    }
    pub fn begin_item_foil(&self, item: &ItemStack, x: f32, y: f32) -> GuiItemFoilMarker {
        let foil_uv_scale = is_special_animated_foil_item(item)
            .then_some(0.5)
            .unwrap_or(1.0);
        GuiItemFoilMarker {
            index_start: self.indices.len(),
            batch_start: self.batches.len(),
            x,
            y,
            foil_uv_scale,
            enabled: item.has_foil() && self.glint_bind_group.is_some(),
        }
    }
    pub fn finish_item_foil(&mut self, marker: GuiItemFoilMarker) {
        if !marker.enabled || marker.index_start >= self.indices.len() {
            return;
        }
        #[derive(Clone, Copy)]
        enum Source {
            Atlas,
            Custom(usize),
        }
        let index_end = self.indices.len();
        let existing_batch_count = self.batches.len();
        let mut source_ranges = Vec::<(Source, usize, usize)>::new();
        let scan_start = marker.batch_start.saturating_sub(1);
        for batch in self
            .batches
            .iter()
            .take(existing_batch_count)
            .skip(scan_start)
        {
            let (source, batch_start, batch_count) = match *batch {
                DrawBatch::GuiAtlas { start_idx, count } => {
                    (Source::Atlas, start_idx as usize, count as usize)
                }
                DrawBatch::CustomTexture {
                    bind_group_idx,
                    start_idx,
                    count,
                } => (
                    Source::Custom(bind_group_idx),
                    start_idx as usize,
                    count as usize,
                ),
                DrawBatch::ItemCutout {
                    bind_group_idx,
                    start_idx,
                    count,
                } => (
                    bind_group_idx.map_or(Source::Atlas, Source::Custom),
                    start_idx as usize,
                    count as usize,
                ),
                _ => continue,
            };
            let range_start = batch_start.max(marker.index_start);
            let range_end = batch_start.saturating_add(batch_count).min(index_end);
            if range_start < range_end {
                source_ranges.push((source, range_start, range_end));
            }
        }
        for (source, range_start, range_end) in source_ranges {
            let source_indices = self.indices[range_start..range_end].to_vec();
            let glint_start = self.indices.len() as u32;
            for source_index in source_indices {
                let source_vertex = self.vertices[source_index as usize];
                let glint_index = self.vertices.len() as u32;
                self.vertices.push(GuiVertex {
                    position: source_vertex.position,
                    tex_coords: source_vertex.tex_coords,
                    color: [
                        (source_vertex.position[0] - marker.x) / 16.0,
                        (source_vertex.position[1] - marker.y) / 16.0,
                        1.0,
                        marker.foil_uv_scale,
                    ],
                });
                self.indices.push(glint_index);
            }
            let count = (self.indices.len() as u32).saturating_sub(glint_start);
            if count == 0 {
                continue;
            }
            match source {
                Source::Atlas => self.append_gui_glint_atlas_batch(glint_start, count),
                Source::Custom(bind_group_idx) => {
                    self.append_gui_glint_custom_texture_batch(bind_group_idx, glint_start, count)
                }
            }
        }
    }
    fn add_quad(
        &mut self,
        mut x: f32,
        mut y: f32,
        mut w: f32,
        mut h: f32,
        mut uv_u0: f32,
        mut uv_v0: f32,
        mut uv_u1: f32,
        mut uv_v1: f32,
        color: [f32; 4],
    ) {
        if let Some([sx0, sy0, sx1, sy1]) = self.active_scissor {
            let x0 = x;
            let y0 = y;
            let x1 = x + w;
            let y1 = y + h;
            let nx0 = x0.max(sx0);
            let ny0 = y0.max(sy0);
            let nx1 = x1.min(sx1);
            let ny1 = y1.min(sy1);
            if nx0 >= nx1 || ny0 >= ny1 {
                x = 0.0;
                y = 0.0;
                w = 0.0;
                h = 0.0;
            } else {
                let left_clip = nx0 - x0;
                let right_clip = x1 - nx1;
                let top_clip = ny0 - y0;
                let bottom_clip = y1 - ny1;
                let u_width = uv_u1 - uv_u0;
                let v_height = uv_v1 - uv_v0;
                let nu0 = if w > 0.0 {
                    uv_u0 + (left_clip / w) * u_width
                } else {
                    uv_u0
                };
                let nu1 = if w > 0.0 {
                    uv_u1 - (right_clip / w) * u_width
                } else {
                    uv_u1
                };
                let nv0 = if h > 0.0 {
                    uv_v0 + (top_clip / h) * v_height
                } else {
                    uv_v0
                };
                let nv1 = if h > 0.0 {
                    uv_v1 - (bottom_clip / h) * v_height
                } else {
                    uv_v1
                };
                x = nx0;
                y = ny0;
                w = nx1 - nx0;
                h = ny1 - ny0;
                uv_u0 = nu0;
                uv_u1 = nu1;
                uv_v0 = nv0;
                uv_v1 = nv1;
            }
        }
        let start_idx = self.vertices.len() as u32;
        self.vertices.push(GuiVertex {
            position: [x, y],
            tex_coords: [uv_u0, uv_v0],
            color,
        });
        self.vertices.push(GuiVertex {
            position: [x + w, y],
            tex_coords: [uv_u1, uv_v0],
            color,
        });
        self.vertices.push(GuiVertex {
            position: [x + w, y + h],
            tex_coords: [uv_u1, uv_v1],
            color,
        });
        self.vertices.push(GuiVertex {
            position: [x, y + h],
            tex_coords: [uv_u0, uv_v1],
            color,
        });
        self.indices.push(start_idx);
        self.indices.push(start_idx + 1);
        self.indices.push(start_idx + 2);
        self.indices.push(start_idx);
        self.indices.push(start_idx + 2);
        self.indices.push(start_idx + 3);
    }
    pub fn add_custom_quad(
        &mut self,
        p0: [f32; 2],
        p1: [f32; 2],
        p2: [f32; 2],
        p3: [f32; 2],
        uv0: [f32; 2],
        uv1: [f32; 2],
        uv2: [f32; 2],
        uv3: [f32; 2],
        color: [f32; 4],
    ) {
        if let Some([sx0, sy0, sx1, sy1]) = self.active_scissor {
            let min_x = p0[0].min(p1[0]).min(p2[0]).min(p3[0]);
            let max_x = p0[0].max(p1[0]).max(p2[0]).max(p3[0]);
            let min_y = p0[1].min(p1[1]).min(p2[1]).min(p3[1]);
            let max_y = p0[1].max(p1[1]).max(p2[1]).max(p3[1]);
            if max_x <= sx0 || min_x >= sx1 || max_y <= sy0 || min_y >= sy1 {
                return;
            }
            if min_x < sx0 || max_x > sx1 || min_y < sy0 || max_y > sy1 {
                let points =
                    [(p0, uv0), (p1, uv1), (p2, uv2), (p3, uv3)].map(|(position, tex_coords)| {
                        GuiVertex {
                            position,
                            tex_coords,
                            color,
                        }
                    });
                for triangle in [
                    [points[0], points[1], points[2]],
                    [points[0], points[2], points[3]],
                ] {
                    let polygon = clip_gui_triangle(triangle, [sx0, sy0, sx1, sy1]);
                    if polygon.len() < 3 {
                        continue;
                    }
                    let start = self.vertices.len() as u32;
                    self.vertices.extend_from_slice(&polygon);
                    for index in 1..polygon.len() - 1 {
                        self.indices.extend_from_slice(&[
                            start,
                            start + index as u32,
                            start + index as u32 + 1,
                        ]);
                    }
                }
                return;
            }
        }
        let start_idx = self.vertices.len() as u32;
        self.vertices.push(GuiVertex {
            position: p0,
            tex_coords: uv0,
            color,
        });
        self.vertices.push(GuiVertex {
            position: p1,
            tex_coords: uv1,
            color,
        });
        self.vertices.push(GuiVertex {
            position: p2,
            tex_coords: uv2,
            color,
        });
        self.vertices.push(GuiVertex {
            position: p3,
            tex_coords: uv3,
            color,
        });
        self.indices.push(start_idx);
        self.indices.push(start_idx + 1);
        self.indices.push(start_idx + 2);
        self.indices.push(start_idx);
        self.indices.push(start_idx + 2);
        self.indices.push(start_idx + 3);
    }
    fn first_frame_rect(&self, sprite_id: &ResourceLocation, rect: SpriteUvRect) -> SpriteUvRect {
        let Some((width, height)) = self.gui_atlas.sprite_dimensions.get(sprite_id).copied() else {
            return rect;
        };
        if width == 0 || height <= width || height % width != 0 {
            return rect;
        }
        let frame_fraction = width as f32 / height as f32;
        SpriteUvRect {
            u0: rect.u0,
            v0: rect.v0,
            u1: rect.u1,
            v1: rect.v0 + (rect.v1 - rect.v0) * frame_fraction,
        }
    }
    pub fn draw_item_sprite(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        sprite_id: &ResourceLocation,
        color: [f32; 4],
    ) -> bool {
        let Some(rect) = self.gui_atlas.sprites.get(sprite_id).copied() else {
            return false;
        };
        let rect = self.first_frame_rect(sprite_id, rect);
        let indices_before = self.indices.len() as u32;
        self.add_quad(x, y, w, h, rect.u0, rect.v0, rect.u1, rect.v1, color);
        self.append_gui_atlas_batch(indices_before, 6);
        true
    }
    pub fn draw_atlas_sprite_region_pixels(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        sprite_id: &ResourceLocation,
        crop_pixels: Option<[f32; 4]>,
        color: [f32; 4],
    ) -> bool {
        let Some(rect) = self.gui_atlas.sprites.get(sprite_id).copied() else {
            return false;
        };
        let rect = self.first_frame_rect(sprite_id, rect);
        let (sprite_w, sprite_h) = self
            .gui_atlas
            .sprite_dimensions
            .get(sprite_id)
            .copied()
            .unwrap_or((16, 16));
        let [src_x, src_y, src_w, src_h] =
            crop_pixels.unwrap_or([0.0, 0.0, sprite_w as f32, sprite_h as f32]);
        let u0 = rect.u0 + (rect.u1 - rect.u0) * (src_x / sprite_w as f32);
        let v0 = rect.v0 + (rect.v1 - rect.v0) * (src_y / sprite_h as f32);
        let u1 = rect.u0 + (rect.u1 - rect.u0) * ((src_x + src_w) / sprite_w as f32);
        let v1 = rect.v0 + (rect.v1 - rect.v0) * ((src_y + src_h) / sprite_h as f32);
        let indices_before = self.indices.len() as u32;
        self.add_quad(x, y, w, h, u0, v0, u1, v1, color);
        self.append_gui_atlas_batch(indices_before, 6);
        true
    }
    #[inline]
    pub fn append_solid_batch(&mut self, indices_before: u32, added_indices: u32) {
        if added_indices == 0 {
            return;
        }
        if let Some(DrawBatch::Solid { count, .. }) = self.last_mergeable_batch_mut() {
            *count += added_indices;
        } else {
            self.batches.push(DrawBatch::Solid {
                start_idx: indices_before,
                count: added_indices,
            });
        }
    }
    #[inline]
    pub fn append_color_batch(&mut self, indices_before: u32, added_indices: u32) {
        if added_indices == 0 {
            return;
        }
        if let Some(DrawBatch::Color { count, .. }) = self.last_mergeable_batch_mut() {
            *count += added_indices;
        } else {
            self.batches.push(DrawBatch::Color {
                start_idx: indices_before,
                count: added_indices,
            });
        }
    }
    #[inline]
    pub fn append_invert_rect_batch(&mut self, indices_before: u32, added_indices: u32) {
        if added_indices == 0 {
            return;
        }
        if let Some(DrawBatch::ColorInvertRect { count, .. }) = self.last_mergeable_batch_mut() {
            *count += added_indices;
        } else {
            self.batches.push(DrawBatch::ColorInvertRect {
                start_idx: indices_before,
                count: added_indices,
            });
        }
    }
    #[inline]
    pub fn append_additive_rect_batch(&mut self, indices_before: u32, added_indices: u32) {
        if added_indices == 0 {
            return;
        }
        if let Some(DrawBatch::ColorAdditiveRect { count, .. }) = self.last_mergeable_batch_mut() {
            *count += added_indices;
        } else {
            self.batches.push(DrawBatch::ColorAdditiveRect {
                start_idx: indices_before,
                count: added_indices,
            });
        }
    }
    #[inline]
    pub fn append_gui_atlas_batch(&mut self, indices_before: u32, added_indices: u32) {
        if added_indices == 0 {
            return;
        }
        if let Some(DrawBatch::GuiAtlas { count, .. }) = self.last_mergeable_batch_mut() {
            *count += added_indices;
        } else {
            self.batches.push(DrawBatch::GuiAtlas {
                start_idx: indices_before,
                count: added_indices,
            });
        }
    }
    #[inline]
    pub fn append_gui_glint_atlas_batch(&mut self, indices_before: u32, added_indices: u32) {
        if added_indices == 0 {
            return;
        }
        if let Some(DrawBatch::GuiGlintAtlas { count, .. }) = self.last_mergeable_batch_mut() {
            *count += added_indices;
        } else {
            self.batches.push(DrawBatch::GuiGlintAtlas {
                start_idx: indices_before,
                count: added_indices,
            });
        }
    }
    #[inline]
    pub fn append_crosshair_batch(&mut self, indices_before: u32, added_indices: u32) {
        if added_indices == 0 {
            return;
        }
        if let Some(DrawBatch::Crosshair { count, .. }) = self.last_mergeable_batch_mut() {
            *count += added_indices;
        } else {
            self.batches.push(DrawBatch::Crosshair {
                start_idx: indices_before,
                count: added_indices,
            });
        }
    }
    #[inline]
    pub fn append_font_sheet_batch(
        &mut self,
        sheet_idx: usize,
        indices_before: u32,
        added_indices: u32,
    ) {
        if added_indices == 0 {
            return;
        }
        if let Some(DrawBatch::FontSheet {
            sheet_idx: last_sheet,
            count,
            ..
        }) = self.last_mergeable_batch_mut()
            && *last_sheet == sheet_idx
        {
            *count += added_indices;
        } else {
            self.batches.push(DrawBatch::FontSheet {
                sheet_idx,
                start_idx: indices_before,
                count: added_indices,
            });
        }
    }
    #[inline]
    pub fn append_custom_texture_batch(
        &mut self,
        bind_group_idx: usize,
        indices_before: u32,
        added_indices: u32,
    ) {
        if added_indices == 0 {
            return;
        }
        if let Some(DrawBatch::CustomTexture {
            bind_group_idx: current,
            count,
            ..
        }) = self.last_mergeable_batch_mut()
            && *current == bind_group_idx
        {
            *count += added_indices;
        } else {
            self.batches.push(DrawBatch::CustomTexture {
                bind_group_idx,
                start_idx: indices_before,
                count: added_indices,
            });
        }
    }
    #[inline]
    pub fn append_item_cutout_batch(
        &mut self,
        bind_group_idx: Option<usize>,
        indices_before: u32,
        added_indices: u32,
    ) {
        if added_indices == 0 {
            return;
        }
        if let Some(DrawBatch::ItemCutout {
            bind_group_idx: current,
            count,
            ..
        }) = self.last_mergeable_batch_mut()
            && *current == bind_group_idx
        {
            *count += added_indices;
        } else {
            self.batches.push(DrawBatch::ItemCutout {
                bind_group_idx,
                start_idx: indices_before,
                count: added_indices,
            });
        }
    }
    #[inline]
    pub fn append_gui_glint_custom_texture_batch(
        &mut self,
        bind_group_idx: usize,
        indices_before: u32,
        added_indices: u32,
    ) {
        if added_indices == 0 {
            return;
        }
        if let Some(DrawBatch::GuiGlintCustomTexture {
            bind_group_idx: current,
            count,
            ..
        }) = self.last_mergeable_batch_mut()
            && *current == bind_group_idx
        {
            *count += added_indices;
        } else {
            self.batches.push(DrawBatch::GuiGlintCustomTexture {
                bind_group_idx,
                start_idx: indices_before,
                count: added_indices,
            });
        }
    }
    fn shaded_color(color: [f32; 4], shade: f32) -> [f32; 4] {
        [
            color[0] * shade,
            color[1] * shade,
            color[2] * shade,
            color[3],
        ]
    }
    fn push_special_model_parts_custom(
        &mut self,
        x: f32,
        y: f32,
        _texture_idx: usize,
        parts: &[GuiModelPart],
        view_matrix: Mat4,
        color: [f32; 4],
        apply_shade: bool,
    ) -> u32 {
        let mut quads = Vec::new();
        let mut bounds = Vec::new();
        for part in parts {
            collect_gui_model_part_quads(
                part,
                Mat4::IDENTITY,
                view_matrix,
                color,
                &mut quads,
                &mut bounds,
            );
        }
        quads.sort_by(|left, right| {
            right
                .depth
                .partial_cmp(&left.depth)
                .unwrap_or(Ordering::Equal)
        });
        let center = [x + 8.0, y + 8.0];
        let indices_before = self.indices.len() as u32;
        for quad in quads {
            let positions = quad.positions.map(|position| {
                [
                    center[0] + position[0] * 16.0,
                    center[1] + position[1] * 16.0,
                ]
            });
            let shade = if apply_shade {
                gui_item_face_shade(quad.normal)
            } else {
                1.0
            };
            self.add_custom_quad(
                positions[0],
                positions[1],
                positions[2],
                positions[3],
                quad.uvs[0],
                quad.uvs[1],
                quad.uvs[2],
                quad.uvs[3],
                Self::shaded_color(quad.color, shade),
            );
        }
        self.indices.len() as u32 - indices_before
    }
    fn push_special_model_parts_raw(
        &mut self,
        x: f32,
        y: f32,
        texture: &ResourceLocation,
        parts: &[GuiModelPart],
        view_matrix: Mat4,
        color: [f32; 4],
        apply_shade: bool,
    ) -> u32 {
        let Some(rect) = self.gui_atlas.sprites.get(texture).copied() else {
            return 0;
        };
        let rect = self.first_frame_rect(texture, rect);
        let mut quads = Vec::new();
        let mut bounds = Vec::new();
        for part in parts {
            collect_gui_model_part_quads(
                part,
                Mat4::IDENTITY,
                view_matrix,
                color,
                &mut quads,
                &mut bounds,
            );
        }
        if quads.is_empty() {
            return 0;
        }
        let scale = 16.0_f32;
        let center = [x + 8.0, y + 8.0];
        quads.sort_by(|left, right| {
            right
                .depth
                .partial_cmp(&left.depth)
                .unwrap_or(Ordering::Equal)
        });
        let indices_before = self.indices.len() as u32;
        for quad in quads {
            let positions = quad.positions.map(|position| {
                [
                    center[0] + position[0] * scale,
                    center[1] + position[1] * scale,
                ]
            });
            let uvs = quad.uvs.map(|uv| {
                [
                    rect.u0 + (rect.u1 - rect.u0) * uv[0],
                    rect.v0 + (rect.v1 - rect.v0) * uv[1],
                ]
            });
            let shade = if apply_shade {
                gui_item_face_shade(quad.normal)
            } else {
                1.0
            };
            self.add_custom_quad(
                positions[0],
                positions[1],
                positions[2],
                positions[3],
                uvs[0],
                uvs[1],
                uvs[2],
                uvs[3],
                Self::shaded_color(quad.color, shade),
            );
        }
        self.indices.len() as u32 - indices_before
    }
    pub fn draw_missing_item(&mut self, x: f32, y: f32) {
        let purple = [0.96, 0.0, 0.96, 1.0];
        let black = [0.02, 0.02, 0.02, 1.0];
        self.draw_rect(x, y, 8.0, 8.0, purple);
        self.draw_rect(x + 8.0, y, 8.0, 8.0, black);
        self.draw_rect(x, y + 8.0, 8.0, 8.0, black);
        self.draw_rect(x + 8.0, y + 8.0, 8.0, 8.0, purple);
    }
    pub fn draw_flat_item_model(
        &mut self,
        x: f32,
        y: f32,
        item: &ItemStack,
        color: [f32; 4],
    ) -> bool {
        let item_id = &item.item.location;
        let Some(model) = self.item_flat_models.get(item_id).cloned() else {
            return false;
        };
        let transform = model.gui_transform.unwrap_or_default();
        let tx = transform.translation[0];
        let ty = transform.translation[1];
        let sx = transform.scale[0];
        let sy = transform.scale[1];
        let w = 16.0 * sx;
        let h = 16.0 * sy;
        let draw_x = x + 8.0 - w / 2.0 + tx;
        let draw_y = y + 8.0 - h / 2.0 - ty;
        let mut rendered = false;
        for layer in &model.layers {
            let layer_tint = layer.tint.resolve(item);
            let layer_color = [
                color[0] * layer_tint[0],
                color[1] * layer_tint[1],
                color[2] * layer_tint[2],
                color[3] * layer_tint[3],
            ];
            rendered |= self.draw_item_sprite(draw_x, draw_y, w, h, &layer.texture, layer_color);
        }
        rendered
    }
    pub fn draw_block_item_model(
        &mut self,
        x: f32,
        y: f32,
        item_id: &ResourceLocation,
        color: [f32; 4],
    ) -> bool {
        let Some(model) = self.item_block_models.get(item_id).cloned() else {
            return false;
        };
        self.draw_gui_block_item_model(x, y, &model, color)
    }
    #[cfg(feature = "extensions")]
    pub fn draw_mod_item(&mut self, x: f32, y: f32, item_id: &ResourceLocation) -> bool {
        if let Some(model) = self.item_block_models.get(item_id).cloned() {
            return self.draw_gui_block_item_model(x, y, &model, [1.0, 1.0, 1.0, 1.0]);
        }
        let Some(model) = self.item_flat_models.get(item_id).cloned() else {
            return false;
        };
        let transform = model.gui_transform.unwrap_or_default();
        let tx = transform.translation[0];
        let ty = transform.translation[1];
        let sx = transform.scale[0];
        let sy = transform.scale[1];
        let w = 16.0 * sx;
        let h = 16.0 * sy;
        let draw_x = x + 8.0 - w / 2.0 + tx;
        let draw_y = y + 8.0 - h / 2.0 - ty;
        let mut rendered = false;
        for layer in &model.layers {
            let layer_tint = layer.tint.default_color();
            rendered |= self.draw_item_sprite(draw_x, draw_y, w, h, &layer.texture, layer_tint);
        }
        rendered
    }
    pub fn draw_special_item_model(
        &mut self,
        x: f32,
        y: f32,
        item_id: &ResourceLocation,
        color: [f32; 4],
    ) -> bool {
        let Some(model) = self.item_special_models.get(item_id).cloned() else {
            return false;
        };
        if !self.gui_atlas.sprites.contains_key(&model.texture) {
            log::warn!(
                "[GUI] Special model {:?} for {} — texture {:?} NOT in atlas!",
                model.kind,
                item_id,
                model.texture
            );
        }
        let tint = [
            color[0] * model.tint[0],
            color[1] * model.tint[1],
            color[2] * model.tint[2],
            color[3] * model.tint[3],
        ];
        let indices_before = self.indices.len() as u32;
        let parts: Vec<GuiModelPart> = match model.kind {
            GuiSpecialItemKind::Shield => shield_gui_model_parts(),
            GuiSpecialItemKind::Banner => banner_gui_model_parts(tint),
            GuiSpecialItemKind::Chest => chest_gui_model_parts(model.chest),
            GuiSpecialItemKind::ShulkerBox => shulker_box_gui_model_parts(),
            GuiSpecialItemKind::DecoratedPot => decorated_pot_gui_model_parts(),
            GuiSpecialItemKind::Conduit => conduit_gui_model_parts(),
            GuiSpecialItemKind::Trident => trident_gui_model_parts(),
            GuiSpecialItemKind::Head => skull_gui_model_parts(&model.texture, model.crop_pixels),
            GuiSpecialItemKind::CopperGolemStatue => copper_golem_statue_gui_model_parts(),
        };
        let screen_flip = Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0));
        let view_matrix = screen_flip * model.gui_transform;
        let actual_tint = match model.kind {
            GuiSpecialItemKind::Banner => [1.0, 1.0, 1.0, tint[3]],
            _ => tint,
        };
        let added_indices = self.push_special_model_parts_raw(
            x,
            y,
            &model.texture,
            &parts,
            view_matrix,
            actual_tint,
            model.kind != GuiSpecialItemKind::Banner && model.kind != GuiSpecialItemKind::Shield,
        );
        if added_indices == 0 {
            log::warn!(
                "[GUI] Special model {:?} for {} — 0 indices after culling!",
                model.kind,
                item_id
            );
            return false;
        }
        if model.kind == GuiSpecialItemKind::Head {
            self.append_item_cutout_batch(None, indices_before, added_indices);
        } else {
            self.append_gui_atlas_batch(indices_before, added_indices);
        }
        true
    }
    pub fn draw_block_as_isometric(
        &mut self,
        x: f32,
        y: f32,
        item_path: &str,
        color: [f32; 4],
    ) -> bool {
        let has_tex = |atlas: &TextureAtlas, name: &str| -> bool {
            atlas
                .sprites
                .contains_key(&ResourceLocation::new("minecraft", name))
        };
        let get_tex_name = |atlas: &TextureAtlas, item_path: &str| -> Option<String> {
            let direct = format!("block/{}", item_path);
            if has_tex(atlas, &direct) {
                return Some(direct);
            }
            let suffixes = [
                "_stairs",
                "_slab",
                "_wall",
                "_fence",
                "_fence_gate",
                "_door",
                "_button",
                "_pressure_plate",
                "_gate",
                "_trapdoor",
                "_hanging_sign",
                "_sign",
                "_pane",
            ];
            let mut base = item_path;
            for suffix in suffixes {
                if let Some(stripped) = item_path.strip_suffix(suffix) {
                    base = stripped;
                    break;
                }
            }
            let candidates = [
                base.to_string(),
                format!("{}_planks", base),
                format!("{}_block", base),
                format!("{}s", base),
                format!("polished_{}", base),
            ];
            for candidate in candidates {
                let name = format!("block/{}", candidate);
                if has_tex(atlas, &name) {
                    return Some(name);
                }
            }
            None
        };
        let resolved_path = get_tex_name(&self.gui_atlas, item_path);
        let top_tex = if item_path == "grass_block" {
            ResourceLocation::new("minecraft", "block/grass_block_top")
        } else if item_path.contains("log") {
            let log_base = item_path.strip_suffix("_log").unwrap_or(item_path);
            ResourceLocation::new("minecraft", &format!("block/{}_log_top", log_base))
        } else if let Some(ref path) = resolved_path {
            ResourceLocation::new("minecraft", path)
        } else {
            ResourceLocation::new("minecraft", &format!("block/{}", item_path))
        };
        let side_tex = if item_path == "grass_block" {
            ResourceLocation::new("minecraft", "block/grass_block_side")
        } else if let Some(ref path) = resolved_path {
            ResourceLocation::new("minecraft", path)
        } else {
            ResourceLocation::new("minecraft", &format!("block/{}", item_path))
        };
        let top_rect = self
            .gui_atlas
            .sprites
            .get(&top_tex)
            .or_else(|| {
                self.gui_atlas.sprites.get(&ResourceLocation::new(
                    "minecraft",
                    &format!("block/{}", item_path),
                ))
            })
            .copied()
            .map(|rect| self.first_frame_rect(&top_tex, rect));
        let side_rect = self
            .gui_atlas
            .sprites
            .get(&side_tex)
            .or_else(|| {
                self.gui_atlas.sprites.get(&ResourceLocation::new(
                    "minecraft",
                    &format!("block/{}", item_path),
                ))
            })
            .copied()
            .map(|rect| self.first_frame_rect(&side_tex, rect));
        if let (Some(top), Some(side)) = (top_rect, side_rect) {
            let indices_before = self.indices.len() as u32;
            self.push_isometric_textured_cube_quads(x, y, top, side, color);
            if item_path == "grass_block" {
                let overlay_tex =
                    ResourceLocation::new("minecraft", "block/grass_block_side_overlay");
                if let Some(overlay) = self.gui_atlas.sprites.get(&overlay_tex).copied() {
                    let overlay = self.first_frame_rect(&overlay_tex, overlay);
                    self.add_custom_quad(
                        [x, y + 4.0],
                        [x + 8.0, y + 8.0],
                        [x + 8.0, y + 16.0],
                        [x, y + 12.0],
                        [overlay.u0, overlay.v0],
                        [overlay.u1, overlay.v0],
                        [overlay.u1, overlay.v1],
                        [overlay.u0, overlay.v1],
                        [color[0] * 0.6, color[1] * 0.6, color[2] * 0.6, color[3]],
                    );
                    self.add_custom_quad(
                        [x + 8.0, y + 8.0],
                        [x + 16.0, y + 4.0],
                        [x + 16.0, y + 12.0],
                        [x + 8.0, y + 16.0],
                        [overlay.u0, overlay.v0],
                        [overlay.u1, overlay.v0],
                        [overlay.u1, overlay.v1],
                        [overlay.u0, overlay.v1],
                        [color[0] * 0.8, color[1] * 0.8, color[2] * 0.8, color[3]],
                    );
                }
            }
            let added_indices = self.indices.len() as u32 - indices_before;
            self.append_gui_atlas_batch(indices_before, added_indices);
            true
        } else {
            false
        }
    }
    fn draw_gui_block_item_model(
        &mut self,
        x: f32,
        y: f32,
        model: &GuiBlockItemModel,
        color: [f32; 4],
    ) -> bool {
        let indices_before = self.indices.len() as u32;
        for quad in &model.quads {
            let Some(rect) = self.gui_atlas.sprites.get(&quad.texture).copied() else {
                continue;
            };
            let rect = self.first_frame_rect(&quad.texture, rect);
            let map_uv = |uv: [f32; 2]| -> [f32; 2] {
                [
                    rect.u0 + (rect.u1 - rect.u0) * uv[0],
                    rect.v0 + (rect.v1 - rect.v0) * uv[1],
                ]
            };
            let shade = quad.shade;
            let quad_color = [
                color[0] * quad.tint[0] * shade,
                color[1] * quad.tint[1] * shade,
                color[2] * quad.tint[2] * shade,
                color[3],
            ];
            self.add_custom_quad(
                [
                    x + 8.0 + quad.positions[0][0],
                    y + 8.0 + quad.positions[0][1],
                ],
                [
                    x + 8.0 + quad.positions[1][0],
                    y + 8.0 + quad.positions[1][1],
                ],
                [
                    x + 8.0 + quad.positions[2][0],
                    y + 8.0 + quad.positions[2][1],
                ],
                [
                    x + 8.0 + quad.positions[3][0],
                    y + 8.0 + quad.positions[3][1],
                ],
                map_uv(quad.uvs[0]),
                map_uv(quad.uvs[1]),
                map_uv(quad.uvs[2]),
                map_uv(quad.uvs[3]),
                quad_color,
            );
        }
        let added_indices = self.indices.len() as u32 - indices_before;
        if added_indices == 0 {
            return false;
        }
        self.append_gui_atlas_batch(indices_before, added_indices);
        true
    }
    fn get_baked_flat_model(&self, model_id: &ResourceLocation) -> Option<&GuiFlatItemModel> {
        let key = if let Some(path) = model_id.path.strip_prefix("item/") {
            ResourceLocation::new(&model_id.namespace, path)
        } else if let Some(path) = model_id.path.strip_prefix("block/") {
            ResourceLocation::new(&model_id.namespace, path)
        } else {
            model_id.clone()
        };
        self.item_flat_models.get(&key)
    }
    fn get_baked_block_model(&self, model_id: &ResourceLocation) -> Option<&GuiBlockItemModel> {
        let key = if let Some(path) = model_id.path.strip_prefix("item/") {
            ResourceLocation::new(&model_id.namespace, path)
        } else if let Some(path) = model_id.path.strip_prefix("block/") {
            ResourceLocation::new(&model_id.namespace, path)
        } else {
            model_id.clone()
        };
        self.item_block_models.get(&key)
    }
    fn get_baked_special_model(&self, model_id: &ResourceLocation) -> Option<&GuiSpecialItemModel> {
        let key = if let Some(path) = model_id.path.strip_prefix("item/") {
            ResourceLocation::new(&model_id.namespace, path)
        } else if let Some(path) = model_id.path.strip_prefix("block/") {
            ResourceLocation::new(&model_id.namespace, path)
        } else {
            model_id.clone()
        };
        self.item_special_models.get(&key)
    }
    fn convert_special_part(part: &el_inventory::special::SpecialModelPart) -> GuiModelPart {
        let cubes = part
            .cubes
            .iter()
            .map(|cube| GuiModelCube {
                tex_offs: cube.tex_offs,
                origin: Vec3::from(cube.origin),
                dimensions: Vec3::from(cube.dimensions),
                grow: Vec3::from(cube.grow),
                mirror: cube.mirror,
                tex_size: cube.tex_size,
                visible_faces: cube.visible_faces,
            })
            .collect();
        let children = part
            .children
            .iter()
            .map(Self::convert_special_part)
            .collect();
        GuiModelPart {
            pose: GuiPartPose {
                offset: Vec3::from(part.pose.offset),
                rotation: Vec3::from(part.pose.rotation),
                scale: Vec3::from(part.pose.scale),
            },
            cubes,
            children,
            color: part.color,
        }
    }
    pub fn draw_gui_special_item_model(
        &mut self,
        x: f32,
        y: f32,
        model: &GuiSpecialItemModel,
        stack: Option<&ItemStack>,
        color: [f32; 4],
    ) -> bool {
        let tint = [
            color[0] * model.tint[0],
            color[1] * model.tint[1],
            color[2] * model.tint[2],
            color[3] * model.tint[3],
        ];
        if model.kind == GuiSpecialItemKind::DecoratedPot
            || model.kind == GuiSpecialItemKind::Banner
            || model.kind == GuiSpecialItemKind::Shield
        {
            let unbaked = match model.kind {
                GuiSpecialItemKind::DecoratedPot => {
                    el_inventory::special::UnbakedSpecialModel::DecoratedPot
                }
                GuiSpecialItemKind::Banner => {
                    let color = model
                        .banner_color
                        .clone()
                        .unwrap_or_else(|| "white".to_string());
                    el_inventory::special::UnbakedSpecialModel::Banner {
                        color,
                        attachment: "ground".to_string(),
                    }
                }
                GuiSpecialItemKind::Shield => el_inventory::special::UnbakedSpecialModel::Shield,
                _ => return false,
            };
            let renderer = unbaked.bake();
            let layers_tint = if model.kind == GuiSpecialItemKind::Banner {
                color
            } else {
                tint
            };
            let layers = renderer.get_layers(stack, layers_tint);
            let mut rendered_any = false;
            for layer in layers {
                let texture_location = ResourceLocation::from_string(&layer.texture);
                let texture_key =
                    if let Some(path) = texture_location.path.strip_prefix("textures/") {
                        let clean_path = path.strip_suffix(".png").unwrap_or(path);
                        ResourceLocation::new(&texture_location.namespace, clean_path)
                    } else {
                        texture_location
                    };
                let gui_parts: Vec<GuiModelPart> =
                    layer.parts.iter().map(Self::convert_special_part).collect();
                let screen_flip = Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0));
                let view_matrix = screen_flip * model.gui_transform;
                let layer_indices_before = self.indices.len() as u32;
                let added_indices = self.push_special_model_parts_raw(
                    x,
                    y,
                    &texture_key,
                    &gui_parts,
                    view_matrix,
                    layer.tint,
                    model.kind != GuiSpecialItemKind::Banner
                        && model.kind != GuiSpecialItemKind::Shield,
                );
                if added_indices > 0 {
                    self.append_gui_atlas_batch(layer_indices_before, added_indices);
                    rendered_any = true;
                }
            }
            return rendered_any;
        }
        let indices_before = self.indices.len() as u32;
        let parts: Vec<GuiModelPart> = match model.kind {
            GuiSpecialItemKind::Shield => shield_gui_model_parts(),
            GuiSpecialItemKind::Banner => banner_gui_model_parts(tint),
            GuiSpecialItemKind::Chest => chest_gui_model_parts(model.chest),
            GuiSpecialItemKind::ShulkerBox => shulker_box_gui_model_parts(),
            GuiSpecialItemKind::DecoratedPot => decorated_pot_gui_model_parts(),
            GuiSpecialItemKind::Conduit => conduit_gui_model_parts(),
            GuiSpecialItemKind::Trident => trident_gui_model_parts(),
            GuiSpecialItemKind::Head => skull_gui_model_parts(&model.texture, model.crop_pixels),
            GuiSpecialItemKind::CopperGolemStatue => copper_golem_statue_gui_model_parts(),
        };
        let screen_flip = Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0));
        let view_matrix = screen_flip * model.gui_transform;
        let actual_tint = match model.kind {
            GuiSpecialItemKind::Banner => [1.0, 1.0, 1.0, tint[3]],
            _ => tint,
        };
        let added_indices = self.push_special_model_parts_raw(
            x,
            y,
            &model.texture,
            &parts,
            view_matrix,
            actual_tint,
            model.kind != GuiSpecialItemKind::Banner && model.kind != GuiSpecialItemKind::Shield,
        );
        if added_indices == 0 {
            return false;
        }
        if model.kind == GuiSpecialItemKind::Head {
            self.append_item_cutout_batch(None, indices_before, added_indices);
        } else {
            self.append_gui_atlas_batch(indices_before, added_indices);
        }
        true
    }
    pub fn set_item_model_cooldown(&mut self, percent: f32) {
        self.item_model_cooldown = percent;
    }
    pub fn draw_dynamic_item(&mut self, x: f32, y: f32, item: &ItemStack, color: [f32; 4]) -> bool {
        let item_id = &item.item.location;
        let Some(def) = self.item_definitions.get(item_id).cloned() else {
            return false;
        };
        let mut collected = Vec::new();
        self.resolve_dynamic_item_model_recursive(
            def.get("model").unwrap_or(&def),
            item,
            Mat4::IDENTITY,
            &[],
            &mut collected,
        );
        if collected.is_empty() {
            return false;
        }
        let mut rendered = false;
        for (model_id, _transform, tints, special) in &collected {
            if let Some(special) = special {
                let mut resolved = special.clone();
                if let Some(gui_transform) = self
                    .get_baked_special_model(model_id)
                    .or_else(|| self.get_baked_special_model(&item.item.location))
                    .map(|baked| baked.gui_transform)
                {
                    resolved.gui_transform = gui_transform;
                }
                rendered |= self.draw_gui_special_item_model(x, y, &resolved, Some(item), color);
            } else if let Some(baked_flat) = self.get_baked_flat_model(model_id).cloned() {
                rendered |= self.draw_dynamic_flat_model(x, y, &baked_flat, tints, color);
            } else if let Some(baked_block) = self.get_baked_block_model(model_id).cloned() {
                rendered |= self.draw_gui_block_item_model(x, y, &baked_block, color);
            } else {
                let lookup_id = if self.get_baked_special_model(model_id).is_some() {
                    model_id
                } else {
                    &item.item.location
                };
                if let Some(baked_special) = self.get_baked_special_model(lookup_id).cloned() {
                    rendered |=
                        self.draw_gui_special_item_model(x, y, &baked_special, Some(item), color);
                }
            }
        }
        rendered
    }
    pub fn draw_dynamic_flat_model(
        &mut self,
        x: f32,
        y: f32,
        model: &GuiFlatItemModel,
        resolved_tints: &[[f32; 4]],
        color: [f32; 4],
    ) -> bool {
        let transform = model.gui_transform.unwrap_or_default();
        let tx = transform.translation[0];
        let ty = transform.translation[1];
        let sx = transform.scale[0];
        let sy = transform.scale[1];
        let w = 16.0 * sx;
        let h = 16.0 * sy;
        let draw_x = x + 8.0 - w / 2.0 + tx;
        let draw_y = y + 8.0 - h / 2.0 - ty;
        let mut rendered = false;
        for (i, layer) in model.layers.iter().enumerate() {
            let layer_tint = resolved_tints
                .get(i)
                .copied()
                .unwrap_or_else(|| layer.tint.default_color());
            let layer_color = [
                color[0] * layer_tint[0],
                color[1] * layer_tint[1],
                color[2] * layer_tint[2],
                color[3] * layer_tint[3],
            ];
            rendered |= self.draw_item_sprite(draw_x, draw_y, w, h, &layer.texture, layer_color);
        }
        rendered
    }
    pub fn resolve_dynamic_item_model_recursive(
        &self,
        value: &serde_json::Value,
        item: &ItemStack,
        mut current_transform: Mat4,
        current_tints: &[[f32; 4]],
        out: &mut Vec<GuiResolvedItemModel>,
    ) {
        let Some(object) = value.as_object() else {
            return;
        };
        if let Some(trans_val) = object.get("transformation") {
            current_transform = current_transform * parse_json_transformation(trans_val);
        }
        let type_str = object.get("type").and_then(|v| v.as_str());
        match type_str {
            Some("minecraft:model") | Some("model") => {
                if let Some(model_str) = object.get("model").and_then(|value| value.as_str()) {
                    let model = ResourceLocation::from_string(model_str);
                    let mut tints = Vec::new();
                    if let Some(tints_arr) = object.get("tints").and_then(|v| v.as_array()) {
                        for tint_val in tints_arr {
                            if let Some(tint_color) = evaluate_tint(tint_val, item) {
                                tints.push(tint_color);
                            }
                        }
                    }
                    let merged_tints = if tints.is_empty() {
                        current_tints.to_vec()
                    } else {
                        tints
                    };
                    out.push((model, current_transform, merged_tints, None));
                }
            }
            Some("minecraft:special") | Some("special") => {
                if let Some(base_str) = object.get("base").and_then(|value| value.as_str()) {
                    let base = ResourceLocation::from_string(base_str);
                    let special = object
                        .get("model")
                        .and_then(gui_special_item_model_from_json);
                    out.push((base, current_transform, Vec::new(), special));
                }
            }
            Some("minecraft:composite") | Some("composite") => {
                if let Some(models) = object.get("models").and_then(|value| value.as_array()) {
                    for sub_model_val in models {
                        self.resolve_dynamic_item_model_recursive(
                            sub_model_val,
                            item,
                            current_transform,
                            current_tints,
                            out,
                        );
                    }
                }
            }
            Some("minecraft:condition") | Some("condition") => {
                let condition_met = if let Some(component_name) =
                    object.get("component").and_then(|v| v.as_str())
                {
                    item.components.get(component_name).is_some()
                } else if let Some(prop_name) = object.get("property").and_then(|v| v.as_str()) {
                    match prop_name {
                        "minecraft:using_item" => false,
                        _ => false,
                    }
                } else {
                    false
                };
                let branch_key = if condition_met { "on_true" } else { "on_false" };
                if let Some(branch_val) = object.get(branch_key) {
                    self.resolve_dynamic_item_model_recursive(
                        branch_val,
                        item,
                        current_transform,
                        current_tints,
                        out,
                    );
                }
            }
            Some("minecraft:select") | Some("select") => {
                let prop_name = object
                    .get("property")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let prop_val: Option<String> = match prop_name {
                    "minecraft:display_context" => Some("gui".to_string()),
                    "minecraft:bundle/has_selected_item" => {
                        let has_selected = if let Some(DataComponent::Generic {
                            value: Tag::List(contents),
                            ..
                        }) = item.components.get("minecraft:bundle_contents")
                        {
                            !contents.is_empty()
                        } else {
                            false
                        };
                        if has_selected {
                            Some("true".to_string())
                        } else {
                            Some("false".to_string())
                        }
                    }
                    "minecraft:local_time" => local_time_select_value(object),
                    _ => None,
                };
                let mut matched = false;
                if let (Some(prop_val), Some(cases)) = (
                    prop_val.as_deref(),
                    object.get("cases").and_then(|v| v.as_array()),
                ) {
                    for case in cases {
                        if let Some(when_val) = case.get("when") {
                            let matches_when = if let Some(when_str) = when_val.as_str() {
                                when_str == prop_val
                            } else if let Some(when_arr) = when_val.as_array() {
                                when_arr.iter().any(|v| v.as_str() == Some(prop_val))
                            } else {
                                false
                            };
                            if matches_when {
                                if let Some(case_model) = case.get("model") {
                                    self.resolve_dynamic_item_model_recursive(
                                        case_model,
                                        item,
                                        current_transform,
                                        current_tints,
                                        out,
                                    );
                                    matched = true;
                                    break;
                                }
                            }
                        }
                    }
                }
                if !matched {
                    if let Some(fallback_val) = object.get("fallback") {
                        self.resolve_dynamic_item_model_recursive(
                            fallback_val,
                            item,
                            current_transform,
                            current_tints,
                            out,
                        );
                    }
                }
            }
            Some("minecraft:range_dispatch") | Some("range_dispatch") => {
                let scale = object.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                let val = gui_range_dispatch_value(object, item, self.item_model_cooldown);
                let scaled_val = val * scale;
                let mut matched_model = None;
                if let Some(entries) = object.get("entries").and_then(|v| v.as_array()) {
                    let mut best_threshold = -f32::INFINITY;
                    for entry in entries {
                        if let Some(threshold) = entry.get("threshold").and_then(|v| v.as_f64()) {
                            let threshold = threshold as f32;
                            if scaled_val >= threshold && threshold > best_threshold {
                                best_threshold = threshold;
                                matched_model = entry.get("model");
                            }
                        }
                    }
                }
                let model_to_resolve = matched_model.or_else(|| object.get("fallback"));
                if let Some(m) = model_to_resolve {
                    self.resolve_dynamic_item_model_recursive(
                        m,
                        item,
                        current_transform,
                        current_tints,
                        out,
                    );
                }
            }
            _ => {
                let mut sub_collected = Vec::new();
                for key in ["on_false", "on_true", "fallback", "model"] {
                    if let Some(val) = object.get(key) {
                        self.resolve_dynamic_item_model_recursive(
                            val,
                            item,
                            current_transform,
                            current_tints,
                            &mut sub_collected,
                        );
                        if !sub_collected.is_empty() {
                            out.extend(sub_collected);
                            return;
                        }
                    }
                }
                if let Some(entries) = object.get("entries").and_then(|value| value.as_array()) {
                    for entry in entries {
                        if let Some(val) = entry.get("model") {
                            self.resolve_dynamic_item_model_recursive(
                                val,
                                item,
                                current_transform,
                                current_tints,
                                &mut sub_collected,
                            );
                            if !sub_collected.is_empty() {
                                out.extend(sub_collected);
                                return;
                            }
                        }
                    }
                }
                if let Some(models) = object.get("models").and_then(|value| value.as_array()) {
                    for model in models {
                        self.resolve_dynamic_item_model_recursive(
                            model,
                            item,
                            current_transform,
                            current_tints,
                            out,
                        );
                    }
                }
                if let Some(cases) = object.get("cases").and_then(|value| value.as_array()) {
                    for case in cases {
                        if let Some(val) = case.get("model") {
                            self.resolve_dynamic_item_model_recursive(
                                val,
                                item,
                                current_transform,
                                current_tints,
                                &mut sub_collected,
                            );
                            if !sub_collected.is_empty() {
                                out.extend(sub_collected);
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
    fn push_isometric_textured_cube_quads(
        &mut self,
        x: f32,
        y: f32,
        top: SpriteUvRect,
        side: SpriteUvRect,
        color: [f32; 4],
    ) -> u32 {
        let indices_before = self.indices.len() as u32;
        let left = x + 1.0;
        let right = x + 15.0;
        let center = x + 8.0;
        let top_y = y + 1.0;
        let mid_y = y + 7.5;
        let left_mid_y = y + 4.5;
        let bottom_y = y + 15.0;
        let side_bottom_y = y + 11.5;
        self.add_custom_quad(
            [center, top_y],
            [right, left_mid_y],
            [center, mid_y],
            [left, left_mid_y],
            [top.u0, top.v0],
            [top.u1, top.v0],
            [top.u1, top.v1],
            [top.u0, top.v1],
            color,
        );
        self.add_custom_quad(
            [left, left_mid_y],
            [center, mid_y],
            [center, bottom_y],
            [left, side_bottom_y],
            [side.u0, side.v0],
            [side.u1, side.v0],
            [side.u1, side.v1],
            [side.u0, side.v1],
            [color[0] * 0.6, color[1] * 0.6, color[2] * 0.6, color[3]],
        );
        self.add_custom_quad(
            [center, mid_y],
            [right, left_mid_y],
            [right, side_bottom_y],
            [center, bottom_y],
            [side.u0, side.v0],
            [side.u1, side.v0],
            [side.u1, side.v1],
            [side.u0, side.v1],
            [color[0] * 0.8, color[1] * 0.8, color[2] * 0.8, color[3]],
        );
        self.indices.len() as u32 - indices_before
    }
    pub fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        self.draw_color_rect(x, y, w, h, color);
    }
    pub fn draw_color_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let indices_before = self.indices.len() as u32;
        self.add_quad(x, y, w, h, 0.0, 0.0, 1.0, 1.0, color);
        self.append_color_batch(indices_before, 6);
    }
    pub fn draw_invert_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let indices_before = self.indices.len() as u32;
        self.add_quad(x, y, w, h, 0.0, 0.0, 1.0, 1.0, color);
        self.append_invert_rect_batch(indices_before, 6);
    }
    pub fn draw_additive_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let indices_before = self.indices.len() as u32;
        self.add_quad(x, y, w, h, 0.0, 0.0, 1.0, 1.0, color);
        self.append_additive_rect_batch(indices_before, 6);
    }
    pub fn draw_text_highlight_rect(&mut self, x: f32, y: f32, w: f32, h: f32, invert: bool) {
        if invert {
            self.draw_invert_rect(x, y, w, h, [1.0, 1.0, 1.0, 1.0]);
        }
        self.draw_additive_rect(x, y, w, h, [0.0, 0.0, 1.0, 1.0]);
    }
    pub fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: [f32; 4]) {
        let segments = 32usize;
        let indices_before = self.indices.len() as u32;
        let start_idx = self.vertices.len() as u32;
        self.vertices.push(GuiVertex {
            position: [cx, cy],
            tex_coords: [0.0, 0.0],
            color,
        });
        for index in 0..=segments {
            let angle = index as f32 * std::f32::consts::TAU / segments as f32;
            self.vertices.push(GuiVertex {
                position: [cx + angle.cos() * radius, cy + angle.sin() * radius],
                tex_coords: [0.0, 0.0],
                color,
            });
        }
        for index in 0..segments as u32 {
            self.indices.push(start_idx);
            self.indices.push(start_idx + 1 + index);
            self.indices.push(start_idx + 2 + index);
        }
        let count = (self.indices.len() as u32) - indices_before;
        self.append_color_batch(indices_before, count);
    }
    pub fn draw_circle_outline(
        &mut self,
        cx: f32,
        cy: f32,
        outer_radius: f32,
        inner_radius: f32,
        color: [f32; 4],
    ) {
        let segments = 40usize;
        let indices_before = self.indices.len() as u32;
        let start_idx = self.vertices.len() as u32;
        for index in 0..=segments {
            let angle = index as f32 * std::f32::consts::TAU / segments as f32;
            let dx = angle.cos();
            let dy = angle.sin();
            self.vertices.push(GuiVertex {
                position: [cx + dx * outer_radius, cy + dy * outer_radius],
                tex_coords: [0.0, 0.0],
                color,
            });
            self.vertices.push(GuiVertex {
                position: [cx + dx * inner_radius, cy + dy * inner_radius],
                tex_coords: [0.0, 0.0],
                color,
            });
        }
        for index in 0..segments as u32 {
            let base = start_idx + index * 2;
            self.indices.push(base);
            self.indices.push(base + 1);
            self.indices.push(base + 2);
            self.indices.push(base + 1);
            self.indices.push(base + 2);
            self.indices.push(base + 3);
        }
        let count = (self.indices.len() as u32) - indices_before;
        self.append_color_batch(indices_before, count);
    }
    pub fn draw_gradient_rect(
        &mut self,
        mut x: f32,
        mut y: f32,
        mut w: f32,
        mut h: f32,
        mut color_top: [f32; 4],
        mut color_bottom: [f32; 4],
    ) {
        if let Some([sx0, sy0, sx1, sy1]) = self.active_scissor {
            let x0 = x;
            let y0 = y;
            let x1 = x + w;
            let y1 = y + h;
            let nx0 = x0.max(sx0);
            let ny0 = y0.max(sy0);
            let nx1 = x1.min(sx1);
            let ny1 = y1.min(sy1);
            if nx0 >= nx1 || ny0 >= ny1 {
                x = 0.0;
                y = 0.0;
                w = 0.0;
                h = 0.0;
            } else {
                if h > 0.0 {
                    let top_ratio = (ny0 - y0) / h;
                    let bottom_ratio = (y1 - ny1) / h;
                    let interpolate = |c1: [f32; 4], c2: [f32; 4], t: f32| -> [f32; 4] {
                        [
                            c1[0] + (c2[0] - c1[0]) * t,
                            c1[1] + (c2[1] - c1[1]) * t,
                            c1[2] + (c2[2] - c1[2]) * t,
                            c1[3] + (c2[3] - c1[3]) * t,
                        ]
                    };
                    let new_top = interpolate(color_top, color_bottom, top_ratio);
                    let new_bottom = interpolate(color_top, color_bottom, 1.0 - bottom_ratio);
                    color_top = new_top;
                    color_bottom = new_bottom;
                }
                x = nx0;
                y = ny0;
                w = nx1 - nx0;
                h = ny1 - ny0;
            }
        }
        let indices_before = self.indices.len() as u32;
        let start_idx = self.vertices.len() as u32;
        self.vertices.push(GuiVertex {
            position: [x, y],
            tex_coords: [0.0, 0.0],
            color: color_top,
        });
        self.vertices.push(GuiVertex {
            position: [x + w, y],
            tex_coords: [1.0, 0.0],
            color: color_top,
        });
        self.vertices.push(GuiVertex {
            position: [x + w, y + h],
            tex_coords: [1.0, 1.0],
            color: color_bottom,
        });
        self.vertices.push(GuiVertex {
            position: [x, y + h],
            tex_coords: [0.0, 1.0],
            color: color_bottom,
        });
        self.indices.push(start_idx);
        self.indices.push(start_idx + 1);
        self.indices.push(start_idx + 2);
        self.indices.push(start_idx);
        self.indices.push(start_idx + 2);
        self.indices.push(start_idx + 3);
        self.append_color_batch(indices_before, 6);
    }
    pub fn draw_color_mesh_quad(&mut self, corners: [[f32; 2]; 4], colors: [[f32; 4]; 4]) {
        let clip = self.active_scissor;
        let polygon: Vec<[f32; 6]> = corners
            .iter()
            .zip(colors.iter())
            .map(|(corner, color)| [corner[0], corner[1], color[0], color[1], color[2], color[3]])
            .collect();
        let clipped = clip
            .map(|rect| clip_color_polygon_to_rect(&polygon, rect))
            .unwrap_or_else(|| polygon);
        let indices_before = self.indices.len() as u32;
        let start_idx = self.vertices.len() as u32;
        let vertex_count = clipped.len();
        if vertex_count >= 3 {
            for vertex in &clipped {
                self.vertices.push(GuiVertex {
                    position: [vertex[0], vertex[1]],
                    tex_coords: [0.0, 0.0],
                    color: [vertex[2], vertex[3], vertex[4], vertex[5]],
                });
            }
            for corner in 1..vertex_count - 1 {
                self.indices.push(start_idx);
                self.indices.push(start_idx + corner as u32);
                self.indices.push(start_idx + corner as u32 + 1);
            }
            let added_indices = (vertex_count as u32 - 2) * 3;
            self.append_color_batch(indices_before, added_indices);
        }
    }
    pub fn get_or_load_custom_texture(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        resource_manager: &ClientResourceManager,
        id: &ResourceLocation,
    ) -> usize {
        if let Some(&idx) = self.custom_texture_registry.get(id) {
            return idx;
        }
        match self.load_custom_texture(
            device,
            queue,
            texture_bind_group_layout,
            resource_manager,
            id,
        ) {
            Ok(idx) => {
                self.custom_texture_registry.insert(id.clone(), idx);
                idx
            }
            Err(e) => {
                log::error!("Failed to load custom GUI texture {}: {:?}", id, e);
                0
            }
        }
    }
    #[cfg(feature = "extensions")]
    pub(crate) fn sprite_quad_count(
        &self,
        sprite_id: &ResourceLocation,
        width: u32,
        height: u32,
    ) -> Option<u32> {
        self.gui_atlas.sprites.get(sprite_id)?;
        if width == 0 || height == 0 {
            return None;
        }
        match self.gui_atlas.nine_slices.get(sprite_id) {
            Some(info) => nine_slice_quad_count(*info, width, height),
            None => match self.gui_atlas.sprite_tiles.get(sprite_id) {
                Some(tile) => tile_quad_count(*tile, width, height),
                None => Some(1),
            },
        }
    }
    pub fn draw_sprite(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        sprite_id: &ResourceLocation,
        color: [f32; 4],
    ) {
        let Some(rect) = self.gui_atlas.sprites.get(sprite_id).copied() else {
            log::warn!("Missing GUI sprite: {}", sprite_id);
            self.draw_rect(x, y, w, h, [1.0, 0.0, 1.0, 1.0]);
            return;
        };
        let indices_before = self.indices.len() as u32;
        let added_indices;
        if let Some(nine_slice) = self.gui_atlas.nine_slices.get(sprite_id).copied() {
            let quads = nine_slice_quads(rect, nine_slice, x, y, w, h);
            added_indices = quads.len() as u32 * 6;
            for quad in quads {
                self.add_quad(
                    quad.x,
                    quad.y,
                    quad.width,
                    quad.height,
                    quad.u0,
                    quad.v0,
                    quad.u1,
                    quad.v1,
                    color,
                );
            }
        } else if let Some(tile) = self.gui_atlas.sprite_tiles.get(sprite_id).copied() {
            let quads = tile_quads(rect, tile, x, y, w, h);
            added_indices = quads.len() as u32 * 6;
            for quad in quads {
                self.add_quad(
                    quad.x,
                    quad.y,
                    quad.width,
                    quad.height,
                    quad.u0,
                    quad.v0,
                    quad.u1,
                    quad.v1,
                    color,
                );
            }
        } else {
            self.add_quad(x, y, w, h, rect.u0, rect.v0, rect.u1, rect.v1, color);
            added_indices = 6;
        }
        self.append_gui_atlas_batch(indices_before, added_indices);
    }
    pub fn draw_sprite_clipped(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        sprite_id: &ResourceLocation,
        color: [f32; 4],
        clip_ratio_x: f32,
    ) {
        let Some(rect) = self.gui_atlas.sprites.get(sprite_id) else {
            log::warn!("Missing GUI sprite: {}", sprite_id);
            self.draw_rect(x, y, w * clip_ratio_x, h, [1.0, 0.0, 1.0, 1.0]);
            return;
        };
        let indices_before = self.indices.len() as u32;
        let final_w = w * clip_ratio_x;
        let final_u1 = rect.u0 + (rect.u1 - rect.u0) * clip_ratio_x;
        self.add_quad(x, y, final_w, h, rect.u0, rect.v0, final_u1, rect.v1, color);
        self.append_gui_atlas_batch(indices_before, 6);
    }
    pub fn draw_sprite_region(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        sprite_id: &ResourceLocation,
        color: [f32; 4],
        sprite_width: f32,
        sprite_height: f32,
        texture_x: f32,
        texture_y: f32,
    ) {
        if w == 0.0 || h == 0.0 || sprite_width <= 0.0 || sprite_height <= 0.0 {
            return;
        }
        let Some(rect) = self.gui_atlas.sprites.get(sprite_id).copied() else {
            log::warn!("Missing GUI sprite: {}", sprite_id);
            self.draw_rect(x, y, w, h, [1.0, 0.0, 1.0, 1.0]);
            return;
        };
        let (u0, v0, u1, v1) = sprite_region_uvs(
            rect,
            sprite_width,
            sprite_height,
            texture_x,
            texture_y,
            w,
            h,
        );
        let indices_before = self.indices.len() as u32;
        self.add_quad(x, y, w, h, u0, v0, u1, v1, color);
        self.append_gui_atlas_batch(indices_before, 6);
    }
    pub fn draw_sprite_clipped_vertical(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        sprite_id: &ResourceLocation,
        color: [f32; 4],
        clip_ratio_y: f32,
    ) {
        let Some(rect) = self.gui_atlas.sprites.get(sprite_id) else {
            return;
        };
        let ratio = clip_ratio_y.clamp(0.0, 1.0);
        if ratio <= 0.0 {
            return;
        }
        let clipped_h = h * ratio;
        let v0 = rect.v1 - (rect.v1 - rect.v0) * ratio;
        self.add_quad(
            x,
            y + h - clipped_h,
            w,
            clipped_h,
            rect.u0,
            v0,
            rect.u1,
            rect.v1,
            color,
        );
        let indices_before = self.indices.len() as u32 - 6;
        self.append_gui_atlas_batch(indices_before, 6);
    }
    pub fn draw_crosshair_sprite(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        sprite_id: &ResourceLocation,
        color: [f32; 4],
    ) {
        let Some(rect) = self.gui_atlas.sprites.get(sprite_id) else {
            return;
        };
        let indices_before = self.indices.len() as u32;
        self.add_quad(x, y, w, h, rect.u0, rect.v0, rect.u1, rect.v1, color);
        self.append_crosshair_batch(indices_before, 6);
    }
    pub fn draw_crosshair_sprite_region(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        sprite_id: &ResourceLocation,
        color: [f32; 4],
        sprite_width: f32,
        sprite_height: f32,
        texture_x: f32,
        texture_y: f32,
    ) {
        if w == 0.0 || h == 0.0 || sprite_width <= 0.0 || sprite_height <= 0.0 {
            return;
        }
        let Some(rect) = self.gui_atlas.sprites.get(sprite_id).copied() else {
            return;
        };
        let (u0, v0, u1, v1) = sprite_region_uvs(
            rect,
            sprite_width,
            sprite_height,
            texture_x,
            texture_y,
            w,
            h,
        );
        let indices_before = self.indices.len() as u32;
        self.add_quad(x, y, w, h, u0, v0, u1, v1, color);
        self.append_crosshair_batch(indices_before, 6);
    }
    pub fn draw_custom_texture(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        custom_tex_idx: usize,
        color: [f32; 4],
    ) {
        self.draw_custom_texture_region(x, y, w, h, 0.0, 0.0, 1.0, 1.0, custom_tex_idx, color);
    }
    #[cfg(feature = "extensions")]
    pub(crate) fn draw_custom_texture_nine_slice(
        &mut self,
        rect: [f32; 4],
        texture: usize,
        slice: &el_mod_content::tooltip::NineSlice,
        color: [f32; 4],
    ) {
        let [left, top, right, bottom] = slice.border;
        let info = NineSliceInfo {
            width: slice.size[0] as u32,
            height: slice.size[1] as u32,
            border_left: left as u32,
            border_top: top as u32,
            border_right: right as u32,
            border_bottom: bottom as u32,
            stretch_inner: slice.stretch_inner,
        };
        let [x, y, w, h] = rect;
        let uv = SpriteUvRect {
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        };
        for quad in nine_slice_quads(uv, info, x, y, w, h) {
            self.draw_custom_texture_region(
                quad.x,
                quad.y,
                quad.width,
                quad.height,
                quad.u0,
                quad.v0,
                quad.u1,
                quad.v1,
                texture,
                color,
            );
        }
    }
    pub fn draw_custom_texture_region(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
        custom_tex_idx: usize,
        color: [f32; 4],
    ) {
        if self
            .custom_textures
            .get(custom_tex_idx)
            .is_none_or(Option::is_none)
        {
            return;
        }
        let indices_before = self.indices.len() as u32;
        self.add_quad(x, y, w, h, u0, v0, u1, v1, color);
        self.append_custom_texture_batch(custom_tex_idx, indices_before, 6);
    }
    #[inline]
    fn scale_rgb_shadow(color: [f32; 4]) -> [f32; 4] {
        let r = (((color[0].clamp(0.0, 1.0) * 255.0) as u32 * 255 / 1020) as f32) / 255.0;
        let g = (((color[1].clamp(0.0, 1.0) * 255.0) as u32 * 255 / 1020) as f32) / 255.0;
        let b = (((color[2].clamp(0.0, 1.0) * 255.0) as u32 * 255 / 1020) as f32) / 255.0;
        [r, g, b, color[3]]
    }
    pub fn draw_text_with_shadow(
        &mut self,
        font: &FontRenderer,
        text: &str,
        start_x: f32,
        y: f32,
        color: [f32; 4],
        scale: f32,
    ) {
        let shadow_color = Self::scale_rgb_shadow(color);
        self.draw_text(
            font,
            text,
            start_x + 1.0 * scale,
            y + 1.0 * scale,
            shadow_color,
            scale,
        );
        self.draw_text(font, text, start_x, y, color, scale);
    }
    pub fn draw_text_styled_with_shadow(
        &mut self,
        font: &FontRenderer,
        text: &str,
        start_x: f32,
        y: f32,
        color: [f32; 4],
        scale: f32,
        bold: bool,
        italic: bool,
    ) {
        let shadow_color = Self::scale_rgb_shadow(color);
        self.draw_text_styled(
            font,
            text,
            start_x + scale,
            y + scale,
            shadow_color,
            scale,
            bold,
            italic,
        );
        self.draw_text_styled(font, text, start_x, y, color, scale, bold, italic);
    }
    pub fn draw_text_styled(
        &mut self,
        font: &FontRenderer,
        text: &str,
        start_x: f32,
        y: f32,
        color: [f32; 4],
        scale: f32,
        bold: bool,
        italic: bool,
    ) {
        if text.is_empty() {
            return;
        }
        if !italic && !bold {
            self.draw_text(font, text, start_x, y, color, scale);
            return;
        }
        let char_count = text.len();
        self.vertices.reserve(char_count * if bold { 8 } else { 4 });
        self.indices.reserve(char_count * if bold { 12 } else { 6 });
        let mut x = start_x;
        for ch in text.chars() {
            if ch == '\n' {
                continue;
            }
            let glyph = font.glyph(ch);
            if glyph.width > 0.0 {
                let glyph_x = (x + glyph.left * scale).round();
                let glyph_y = (y - glyph.ascent * scale).round();
                let glyph_w = (glyph.width * scale).round();
                let glyph_h = (glyph.height * scale).round();
                let shear_top = if italic { 1.0 * scale } else { 0.0 };
                let shear_bottom = if italic {
                    (1.0 - 0.25 * glyph.height) * scale
                } else {
                    0.0
                };
                let bold_offset = 1.0 * scale;
                let push_quad = |gx: f32, vertices: &mut Vec<GuiVertex>, indices: &mut Vec<u32>| {
                    let s_idx = vertices.len() as u32;
                    vertices.push(GuiVertex {
                        position: [gx + shear_top, glyph_y],
                        tex_coords: [glyph.u0, glyph.v0],
                        color,
                    });
                    vertices.push(GuiVertex {
                        position: [gx + glyph_w + shear_top, glyph_y],
                        tex_coords: [glyph.u1, glyph.v0],
                        color,
                    });
                    vertices.push(GuiVertex {
                        position: [gx + glyph_w + shear_bottom, glyph_y + glyph_h],
                        tex_coords: [glyph.u1, glyph.v1],
                        color,
                    });
                    vertices.push(GuiVertex {
                        position: [gx + shear_bottom, glyph_y + glyph_h],
                        tex_coords: [glyph.u0, glyph.v1],
                        color,
                    });
                    indices.extend_from_slice(&[
                        s_idx,
                        s_idx + 1,
                        s_idx + 2,
                        s_idx,
                        s_idx + 2,
                        s_idx + 3,
                    ]);
                };
                let quad_count = if bold { 12 } else { 6 };
                push_quad(glyph_x, &mut self.vertices, &mut self.indices);
                if bold {
                    push_quad(glyph_x + bold_offset, &mut self.vertices, &mut self.indices);
                }
                let sheet_idx = glyph.texture_index;
                self.append_font_sheet_batch(
                    sheet_idx,
                    (self.indices.len() as u32) - quad_count,
                    quad_count,
                );
            }
            x += (glyph.advance + if bold { 1.0 } else { 0.0 }) * scale;
        }
    }
    pub fn draw_text_top_with_shadow(
        &mut self,
        font: &FontRenderer,
        text: &str,
        start_x: f32,
        top_y: f32,
        color: [f32; 4],
        scale: f32,
    ) {
        let baseline_y = top_y + font.vanilla_ascent() * scale;
        self.draw_text_with_shadow(font, text, start_x, baseline_y, color, scale);
    }
    pub fn draw_text_top(
        &mut self,
        font: &FontRenderer,
        text: &str,
        start_x: f32,
        top_y: f32,
        color: [f32; 4],
        scale: f32,
    ) {
        let baseline_y = top_y + font.vanilla_ascent() * scale;
        self.draw_text(font, text, start_x, baseline_y, color, scale);
    }
    pub fn draw_text(
        &mut self,
        font: &FontRenderer,
        text: &str,
        start_x: f32,
        y: f32,
        color: [f32; 4],
        scale: f32,
    ) {
        if text.is_empty() {
            return;
        }
        self.vertices.reserve(text.len() * 4);
        self.indices.reserve(text.len() * 6);
        let mut x = start_x;
        for ch in text.chars() {
            if ch == '\n' {
                continue;
            }
            let glyph = font.glyph(ch);
            if glyph.width > 0.0 {
                let glyph_x = (x + glyph.left * scale).round();
                let glyph_y = (y - glyph.ascent * scale).round();
                let glyph_w = (glyph.width * scale).round();
                let glyph_h = (glyph.height * scale).round();
                let indices_before = self.indices.len() as u32;
                self.add_quad(
                    glyph_x, glyph_y, glyph_w, glyph_h, glyph.u0, glyph.v0, glyph.u1, glyph.v1,
                    color,
                );
                let sheet_idx = glyph.texture_index;
                self.append_font_sheet_batch(sheet_idx, indices_before, 6);
            }
            x += glyph.advance * scale;
        }
    }
    pub fn end_frame(&mut self, device: &Device, queue: &Queue) {
        if self.vertices.is_empty() {
            self.active_buffer_slot = None;
            return;
        }
        let slot_index = self.next_buffer_slot;
        self.next_buffer_slot = (slot_index + 1) % GUI_BUFFER_SLOT_COUNT;
        self.buffer_slots[slot_index].upload(
            device,
            queue,
            bytemuck::cast_slice(&self.vertices),
            bytemuck::cast_slice(&self.indices),
        );
        self.active_buffer_slot = Some(slot_index);
    }
    pub fn render<'rp>(
        &'rp self,
        render_pass: &mut wgpu::RenderPass<'rp>,
        font: &'rp FontRenderer,
    ) {
        self.render_filtered(render_pass, font, false);
    }
    pub fn render_without_color<'rp>(
        &'rp self,
        render_pass: &mut wgpu::RenderPass<'rp>,
        font: &'rp FontRenderer,
    ) {
        self.render_filtered(render_pass, font, true);
    }
    fn render_filtered<'rp>(
        &'rp self,
        render_pass: &mut wgpu::RenderPass<'rp>,
        font: &'rp FontRenderer,
        skip_color: bool,
    ) {
        let Some(buffer_slot) = self
            .active_buffer_slot
            .and_then(|slot_index| self.buffer_slots.get(slot_index))
        else {
            return;
        };
        let (Some(vertex_buffer), Some(index_buffer)) =
            (&buffer_slot.vertex_buffer, &buffer_slot.index_buffer)
        else {
            return;
        };
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        let mut current_pipeline = u8::MAX;
        let mut current_bind_group_1: Option<*const wgpu::BindGroup> = None;
        let mut current_bind_group_2: Option<*const wgpu::BindGroup> = None;
        for batch in &self.batches {
            match *batch {
                DrawBatch::EndPortal {
                    sky_idx,
                    portal_idx,
                    start_idx,
                    count,
                } => {
                    let (Some(Some((_, sky))), Some(Some((_, portal)))) = (
                        self.custom_textures.get(sky_idx),
                        self.custom_textures.get(portal_idx),
                    ) else {
                        continue;
                    };
                    render_pass.set_pipeline(&self.win_effects.portal);
                    render_pass.set_bind_group(1, sky, &[]);
                    render_pass.set_bind_group(2, portal, &[]);
                    current_pipeline = 4;
                    current_bind_group_1 = Some(sky as *const _);
                    current_bind_group_2 = Some(portal as *const _);
                    render_pass.draw_indexed(start_idx..(start_idx + count), 0, 0..1);
                }
                DrawBatch::Vignette {
                    bind_group_idx,
                    start_idx,
                    count,
                } => {
                    let Some(Some((_, texture))) = self.custom_textures.get(bind_group_idx) else {
                        continue;
                    };
                    render_pass.set_pipeline(&self.win_effects.vignette);
                    render_pass.set_bind_group(1, texture, &[]);
                    current_pipeline = 5;
                    current_bind_group_1 = Some(texture as *const _);
                    render_pass.draw_indexed(start_idx..(start_idx + count), 0, 0..1);
                }
                DrawBatch::Solid { start_idx, count } => {
                    if current_pipeline != 0 {
                        render_pass.set_pipeline(&self.pipeline);
                        current_pipeline = 0;
                    }
                    let bg = &self.white_bind_group;
                    let bg_ptr = bg as *const wgpu::BindGroup;
                    if current_bind_group_1 != Some(bg_ptr) {
                        render_pass.set_bind_group(1, bg, &[]);
                        current_bind_group_1 = Some(bg_ptr);
                    }
                    render_pass.draw_indexed(start_idx..(start_idx + count), 0, 0..1);
                }
                DrawBatch::Color { start_idx, count } => {
                    if skip_color {
                        continue;
                    }
                    if current_pipeline != 3 {
                        render_pass.set_pipeline(&self.color_pipeline);
                        current_pipeline = 3;
                        current_bind_group_1 = None;
                    }
                    render_pass.draw_indexed(start_idx..(start_idx + count), 0, 0..1);
                }
                DrawBatch::ColorInvertRect { start_idx, count } => {
                    if skip_color {
                        continue;
                    }
                    if current_pipeline != 1 {
                        render_pass.set_pipeline(&self.invert_pipeline);
                        current_pipeline = 1;
                    }
                    let bg = &self.white_bind_group;
                    let bg_ptr = bg as *const wgpu::BindGroup;
                    if current_bind_group_1 != Some(bg_ptr) {
                        render_pass.set_bind_group(1, bg, &[]);
                        current_bind_group_1 = Some(bg_ptr);
                    }
                    render_pass.draw_indexed(start_idx..(start_idx + count), 0, 0..1);
                }
                DrawBatch::ColorAdditiveRect { start_idx, count } => {
                    if skip_color {
                        continue;
                    }
                    if current_pipeline != 7 {
                        render_pass.set_pipeline(&self.color_additive_pipeline);
                        current_pipeline = 7;
                        current_bind_group_1 = None;
                    }
                    render_pass.draw_indexed(start_idx..(start_idx + count), 0, 0..1);
                }
                DrawBatch::GuiAtlas { start_idx, count } => {
                    if current_pipeline != 0 {
                        render_pass.set_pipeline(&self.pipeline);
                        current_pipeline = 0;
                    }
                    let bg = &self.gui_atlas.bind_group;
                    let bg_ptr = bg as *const wgpu::BindGroup;
                    if current_bind_group_1 != Some(bg_ptr) {
                        render_pass.set_bind_group(1, bg, &[]);
                        current_bind_group_1 = Some(bg_ptr);
                    }
                    render_pass.draw_indexed(start_idx..(start_idx + count), 0, 0..1);
                }
                DrawBatch::GuiGlintAtlas { start_idx, count } => {
                    let Some(glint_bind_group) = self.glint_bind_group.as_ref() else {
                        continue;
                    };
                    if current_pipeline != 2 {
                        render_pass.set_pipeline(&self.glint_pipeline);
                        current_pipeline = 2;
                    }
                    let bg = &self.gui_atlas.bind_group;
                    let bg_ptr = bg as *const wgpu::BindGroup;
                    if current_bind_group_1 != Some(bg_ptr) {
                        render_pass.set_bind_group(1, bg, &[]);
                        current_bind_group_1 = Some(bg_ptr);
                    }
                    let glint_ptr = glint_bind_group as *const wgpu::BindGroup;
                    if current_bind_group_2 != Some(glint_ptr) {
                        render_pass.set_bind_group(2, glint_bind_group, &[]);
                        current_bind_group_2 = Some(glint_ptr);
                    }
                    render_pass.draw_indexed(start_idx..(start_idx + count), 0, 0..1);
                }
                DrawBatch::Crosshair { start_idx, count } => {
                    if current_pipeline != 1 {
                        render_pass.set_pipeline(&self.invert_pipeline);
                        current_pipeline = 1;
                    }
                    let bg = &self.gui_atlas.bind_group;
                    let bg_ptr = bg as *const wgpu::BindGroup;
                    if current_bind_group_1 != Some(bg_ptr) {
                        render_pass.set_bind_group(1, bg, &[]);
                        current_bind_group_1 = Some(bg_ptr);
                    }
                    render_pass.draw_indexed(start_idx..(start_idx + count), 0, 0..1);
                }
                DrawBatch::FontSheet {
                    sheet_idx,
                    start_idx,
                    count,
                } => {
                    if current_pipeline != 0 {
                        render_pass.set_pipeline(&self.pipeline);
                        current_pipeline = 0;
                    }
                    if sheet_idx < font.bind_groups.len() {
                        let bg = &font.bind_groups[sheet_idx];
                        let bg_ptr = bg as *const wgpu::BindGroup;
                        if current_bind_group_1 != Some(bg_ptr) {
                            render_pass.set_bind_group(1, bg, &[]);
                            current_bind_group_1 = Some(bg_ptr);
                        }
                        render_pass.draw_indexed(start_idx..(start_idx + count), 0, 0..1);
                    }
                }
                DrawBatch::CustomTexture {
                    bind_group_idx,
                    start_idx,
                    count,
                } => {
                    if current_pipeline != 0 {
                        render_pass.set_pipeline(&self.pipeline);
                        current_pipeline = 0;
                    }
                    if let Some(Some((_, bg))) = self.custom_textures.get(bind_group_idx) {
                        let bg_ptr = bg as *const wgpu::BindGroup;
                        if current_bind_group_1 != Some(bg_ptr) {
                            render_pass.set_bind_group(1, bg, &[]);
                            current_bind_group_1 = Some(bg_ptr);
                        }
                        render_pass.draw_indexed(start_idx..(start_idx + count), 0, 0..1);
                    }
                }
                DrawBatch::ItemCutout {
                    bind_group_idx,
                    start_idx,
                    count,
                } => {
                    let bg = if let Some(texture_idx) = bind_group_idx {
                        let Some(Some((_, bg))) = self.custom_textures.get(texture_idx) else {
                            continue;
                        };
                        bg
                    } else {
                        &self.gui_atlas.bind_group
                    };
                    if current_pipeline != 6 {
                        render_pass.set_pipeline(&self.item_cutout_pipeline);
                        current_pipeline = 6;
                    }
                    let bg_ptr = bg as *const wgpu::BindGroup;
                    if current_bind_group_1 != Some(bg_ptr) {
                        render_pass.set_bind_group(1, bg, &[]);
                        current_bind_group_1 = Some(bg_ptr);
                    }
                    render_pass.draw_indexed(start_idx..(start_idx + count), 0, 0..1);
                }
                DrawBatch::GuiGlintCustomTexture {
                    bind_group_idx,
                    start_idx,
                    count,
                } => {
                    let Some(glint_bind_group) = self.glint_bind_group.as_ref() else {
                        continue;
                    };
                    let Some(Some((_, bg))) = self.custom_textures.get(bind_group_idx) else {
                        continue;
                    };
                    if current_pipeline != 2 {
                        render_pass.set_pipeline(&self.glint_pipeline);
                        current_pipeline = 2;
                    }
                    let bg_ptr = bg as *const wgpu::BindGroup;
                    if current_bind_group_1 != Some(bg_ptr) {
                        render_pass.set_bind_group(1, bg, &[]);
                        current_bind_group_1 = Some(bg_ptr);
                    }
                    let glint_ptr = glint_bind_group as *const wgpu::BindGroup;
                    if current_bind_group_2 != Some(glint_ptr) {
                        render_pass.set_bind_group(2, glint_bind_group, &[]);
                        current_bind_group_2 = Some(glint_ptr);
                    }
                    render_pass.draw_indexed(start_idx..(start_idx + count), 0, 0..1);
                }
            }
        }
    }
    pub fn render_color_batches<'rp>(&'rp self, render_pass: &mut wgpu::RenderPass<'rp>) {
        let Some(color_unorm_pipeline) = self.color_unorm_pipeline.as_ref() else {
            return;
        };
        let Some(buffer_slot) = self
            .active_buffer_slot
            .and_then(|slot_index| self.buffer_slots.get(slot_index))
        else {
            return;
        };
        let (Some(vertex_buffer), Some(index_buffer)) =
            (&buffer_slot.vertex_buffer, &buffer_slot.index_buffer)
        else {
            return;
        };
        let mut current_pipeline = 0;
        render_pass.set_pipeline(color_unorm_pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        for batch in &self.batches {
            match *batch {
                DrawBatch::Color { start_idx, count } => {
                    if current_pipeline != 0 {
                        render_pass.set_pipeline(color_unorm_pipeline);
                        current_pipeline = 0;
                    }
                    render_pass.draw_indexed(start_idx..(start_idx + count), 0, 0..1);
                }
                DrawBatch::ColorAdditiveRect { start_idx, count } => {
                    let Some(additive_unorm_pipeline) = self.color_additive_unorm_pipeline.as_ref()
                    else {
                        continue;
                    };
                    if current_pipeline != 1 {
                        render_pass.set_pipeline(additive_unorm_pipeline);
                        current_pipeline = 1;
                    }
                    render_pass.draw_indexed(start_idx..(start_idx + count), 0, 0..1);
                }
                _ => {}
            }
        }
    }
}
fn gui_sprite_id_from_texture_id(id: &ResourceLocation) -> ResourceLocation {
    let path = id
        .path
        .strip_prefix("gui/sprites/")
        .unwrap_or(id.path.as_str());
    ResourceLocation::new(&id.namespace, path)
}
fn all_model_faces() -> [bool; 6] {
    [true; 6]
}
fn single_model_face(face: Direction) -> [bool; 6] {
    let mut faces = [false; 6];
    faces[direction_index(face)] = true;
    faces
}
fn all_model_faces_except(face: Direction) -> [bool; 6] {
    let mut faces = all_model_faces();
    faces[direction_index(face)] = false;
    faces
}
fn direction_index(direction: Direction) -> usize {
    direction.data_3d() as usize
}
fn direction_unit(direction: Direction) -> Vec3 {
    let (x, y, z) = direction.step();
    Vec3::new(x as f32, y as f32, z as f32)
}
fn mirror_model_facing(direction: Direction) -> Direction {
    match direction {
        Direction::East => Direction::West,
        Direction::West => Direction::East,
        _ => direction,
    }
}
fn multiply_color(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    [
        left[0] * right[0],
        left[1] * right[1],
        left[2] * right[2],
        left[3] * right[3],
    ]
}
fn gui_model_polygon(
    mut vertices: [GuiModelVertex3; 4],
    [u0, v0, u1, v1]: [f32; 4],
    [tex_width, tex_height]: [f32; 2],
    mirror: bool,
    facing: Direction,
) -> GuiModelPolygon {
    vertices[0].uv = [u1 / tex_width, v0 / tex_height];
    vertices[1].uv = [u0 / tex_width, v0 / tex_height];
    vertices[2].uv = [u0 / tex_width, v1 / tex_height];
    vertices[3].uv = [u1 / tex_width, v1 / tex_height];
    let normal_facing = if mirror {
        vertices.reverse();
        mirror_model_facing(facing)
    } else {
        facing
    };
    GuiModelPolygon {
        vertices,
        normal: direction_unit(normal_facing),
    }
}
fn collect_gui_model_part_quads(
    part: &GuiModelPart,
    parent_matrix: Mat4,
    view_matrix: Mat4,
    color: [f32; 4],
    quads: &mut Vec<GuiProjectedModelQuad>,
    bounds: &mut Vec<[f32; 2]>,
) {
    let part_matrix = parent_matrix * part.pose.matrix();
    let part_color = multiply_color(color, part.color);
    for cube in &part.cubes {
        for polygon in cube.polygons() {
            let transformed = polygon.vertices.map(|vertex| {
                view_matrix.transform_point3(part_matrix.transform_point3(vertex.pos / 16.0))
            });
            let normal_matrix = Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0)) * view_matrix;
            let normal = normal_matrix
                .transform_vector3(part_matrix.transform_vector3(polygon.normal))
                .normalize_or_zero();
            let positions = transformed.map(|position| [position.x, position.y]);
            bounds.extend_from_slice(&positions);
            let is_flipped = (view_matrix * part_matrix).determinant() < 0.0;
            let edge1_x = transformed[1].x - transformed[0].x;
            let edge1_y = transformed[1].y - transformed[0].y;
            let edge2_x = transformed[2].x - transformed[0].x;
            let edge2_y = transformed[2].y - transformed[0].y;
            let cross_z = edge1_x * edge2_y - edge1_y * edge2_x;
            let should_cull = if is_flipped {
                cross_z.abs() > 0.0001 && cross_z < 0.0
            } else {
                cross_z.abs() > 0.0001 && cross_z > 0.0
            };
            if should_cull {
                continue;
            }
            quads.push(GuiProjectedModelQuad {
                positions,
                uvs: polygon.vertices.map(|vertex| vertex.uv),
                normal,
                depth: transformed.iter().map(|position| position.z).sum::<f32>() / 4.0,
                color: part_color,
            });
        }
    }
    for child in &part.children {
        collect_gui_model_part_quads(child, part_matrix, view_matrix, part_color, quads, bounds);
    }
}
fn gui_model_cube(
    tex_offs: [f32; 2],
    origin: [f32; 3],
    dimensions: [f32; 3],
    tex_size: [f32; 2],
) -> GuiModelCube {
    GuiModelCube::new(
        tex_offs,
        Vec3::new(origin[0], origin[1], origin[2]),
        Vec3::new(dimensions[0], dimensions[1], dimensions[2]),
        tex_size,
    )
}
fn gui_model_part(
    pose: GuiPartPose,
    cubes: Vec<GuiModelCube>,
    children: Vec<GuiModelPart>,
) -> GuiModelPart {
    GuiModelPart::new(pose, cubes, children, [1.0, 1.0, 1.0, 1.0])
}
fn gui_model_part_colored(
    pose: GuiPartPose,
    cubes: Vec<GuiModelCube>,
    children: Vec<GuiModelPart>,
    color: [f32; 4],
) -> GuiModelPart {
    GuiModelPart::new(pose, cubes, children, color)
}
fn shield_gui_model_parts() -> Vec<GuiModelPart> {
    let tex = [64.0, 64.0];
    vec![
        gui_model_part(
            GuiPartPose::zero(),
            vec![
                gui_model_cube([0.0, 0.0], [-6.0, -11.0, -2.0], [12.0, 22.0, 1.0], tex)
                    .visible_faces([false, false, false, true, false, true]),
            ],
            Vec::new(),
        ),
        gui_model_part(
            GuiPartPose::zero(),
            vec![gui_model_cube(
                [26.0, 0.0],
                [-1.0, -3.0, -1.0],
                [2.0, 6.0, 6.0],
                tex,
            )],
            Vec::new(),
        ),
    ]
}
fn banner_gui_model_parts(flag_color: [f32; 4]) -> Vec<GuiModelPart> {
    let tex = [64.0, 64.0];
    let hardware_color = [0.82, 0.82, 0.82, flag_color[3]];
    vec![
        gui_model_part_colored(
            GuiPartPose::zero(),
            vec![gui_model_cube(
                [44.0, 0.0],
                [-1.0, -42.0, -1.0],
                [2.0, 42.0, 2.0],
                tex,
            )],
            Vec::new(),
            hardware_color,
        ),
        gui_model_part_colored(
            GuiPartPose::zero(),
            vec![gui_model_cube(
                [0.0, 42.0],
                [-10.0, -44.0, -1.0],
                [20.0, 2.0, 2.0],
                tex,
            )],
            Vec::new(),
            hardware_color,
        ),
        gui_model_part_colored(
            GuiPartPose::offset_and_rotation(
                0.0,
                -44.0,
                0.0,
                -0.0025 * std::f32::consts::PI,
                0.0,
                0.0,
            ),
            vec![gui_model_cube(
                [0.0, 0.0],
                [-10.0, 0.0, -2.0],
                [20.0, 40.0, 1.0],
                tex,
            )],
            Vec::new(),
            flag_color,
        ),
    ]
}
fn chest_gui_model_parts(params: GuiChestParams) -> Vec<GuiModelPart> {
    let tex = [64.0, 64.0];
    let lid_x_rot = -(params.openness * std::f32::consts::FRAC_PI_2);
    let (body_origin_x, body_width, lock_origin_x, lock_width, faces) = match params.chest_type {
        GuiChestType::Single => (1.0, 14.0, 7.0, 2.0, all_model_faces()),
        GuiChestType::Left => (0.0, 15.0, 0.0, 1.0, all_model_faces_except(Direction::West)),
        GuiChestType::Right => (
            1.0,
            15.0,
            15.0,
            1.0,
            all_model_faces_except(Direction::East),
        ),
    };
    vec![
        gui_model_part(
            GuiPartPose::zero(),
            vec![
                gui_model_cube(
                    [0.0, 19.0],
                    [body_origin_x, 0.0, 1.0],
                    [body_width, 10.0, 14.0],
                    tex,
                )
                .visible_faces(faces),
            ],
            Vec::new(),
        ),
        gui_model_part(
            GuiPartPose::offset_and_rotation(0.0, 9.0, 1.0, lid_x_rot, 0.0, 0.0),
            vec![
                gui_model_cube(
                    [0.0, 0.0],
                    [body_origin_x, 0.0, 0.0],
                    [body_width, 5.0, 14.0],
                    tex,
                )
                .visible_faces(faces),
            ],
            Vec::new(),
        ),
        gui_model_part(
            GuiPartPose::offset_and_rotation(0.0, 9.0, 1.0, lid_x_rot, 0.0, 0.0),
            vec![
                gui_model_cube(
                    [0.0, 0.0],
                    [lock_origin_x, -2.0, 14.0],
                    [lock_width, 4.0, 1.0],
                    tex,
                )
                .visible_faces(faces),
            ],
            Vec::new(),
        ),
    ]
}
fn shulker_box_gui_model_parts() -> Vec<GuiModelPart> {
    let tex = [64.0, 64.0];
    vec![
        gui_model_part(
            GuiPartPose::offset(0.0, 24.0, 0.0),
            vec![gui_model_cube(
                [0.0, 28.0],
                [-8.0, -8.0, -8.0],
                [16.0, 8.0, 16.0],
                tex,
            )],
            Vec::new(),
        ),
        gui_model_part(
            GuiPartPose::offset(0.0, 24.0, 0.0),
            vec![gui_model_cube(
                [0.0, 0.0],
                [-8.0, -16.0, -8.0],
                [16.0, 12.0, 16.0],
                tex,
            )],
            Vec::new(),
        ),
    ]
}
fn decorated_pot_gui_model_parts() -> Vec<GuiModelPart> {
    let base_tex = [32.0, 32.0];
    let side_tex = [16.0, 16.0];
    let side_plane = || {
        gui_model_cube([1.0, 0.0], [0.0, 0.0, 0.0], [14.0, 16.0, 0.0], side_tex)
            .visible_faces(single_model_face(Direction::North))
    };
    vec![
        gui_model_part(
            GuiPartPose::offset_and_rotation(0.0, 37.0, 16.0, std::f32::consts::PI, 0.0, 0.0),
            vec![
                gui_model_cube([0.0, 0.0], [4.0, 17.0, 4.0], [8.0, 3.0, 8.0], base_tex).grown(-0.1),
                gui_model_cube([0.0, 5.0], [5.0, 20.0, 5.0], [6.0, 1.0, 6.0], base_tex).grown(0.2),
            ],
            Vec::new(),
        ),
        gui_model_part(
            GuiPartPose::offset(1.0, 16.0, 1.0),
            vec![gui_model_cube(
                [-14.0, 13.0],
                [0.0, 0.0, 0.0],
                [14.0, 0.0, 14.0],
                base_tex,
            )],
            Vec::new(),
        ),
        gui_model_part(
            GuiPartPose::offset(1.0, 0.0, 1.0),
            vec![gui_model_cube(
                [-14.0, 13.0],
                [0.0, 0.0, 0.0],
                [14.0, 0.0, 14.0],
                base_tex,
            )],
            Vec::new(),
        ),
        gui_model_part(
            GuiPartPose::offset_and_rotation(15.0, 16.0, 1.0, 0.0, 0.0, std::f32::consts::PI),
            vec![side_plane()],
            Vec::new(),
        ),
        gui_model_part(
            GuiPartPose::offset_and_rotation(
                1.0,
                16.0,
                1.0,
                0.0,
                -std::f32::consts::FRAC_PI_2,
                std::f32::consts::PI,
            ),
            vec![side_plane()],
            Vec::new(),
        ),
        gui_model_part(
            GuiPartPose::offset_and_rotation(
                15.0,
                16.0,
                15.0,
                0.0,
                std::f32::consts::FRAC_PI_2,
                std::f32::consts::PI,
            ),
            vec![side_plane()],
            Vec::new(),
        ),
        gui_model_part(
            GuiPartPose::offset_and_rotation(1.0, 16.0, 15.0, std::f32::consts::PI, 0.0, 0.0),
            vec![side_plane()],
            Vec::new(),
        ),
    ]
}
fn conduit_gui_model_parts() -> Vec<GuiModelPart> {
    vec![gui_model_part(
        GuiPartPose::zero(),
        vec![gui_model_cube(
            [0.0, 0.0],
            [-3.0, -3.0, -3.0],
            [6.0, 6.0, 6.0],
            [32.0, 16.0],
        )],
        Vec::new(),
    )]
}
fn trident_gui_model_parts() -> Vec<GuiModelPart> {
    let tex = [32.0, 32.0];
    vec![gui_model_part(
        GuiPartPose::zero(),
        vec![gui_model_cube(
            [0.0, 6.0],
            [-0.5, 2.0, -0.5],
            [1.0, 25.0, 1.0],
            tex,
        )],
        vec![
            gui_model_part(
                GuiPartPose::zero(),
                vec![gui_model_cube(
                    [4.0, 0.0],
                    [-1.5, 0.0, -0.5],
                    [3.0, 2.0, 1.0],
                    tex,
                )],
                Vec::new(),
            ),
            gui_model_part(
                GuiPartPose::zero(),
                vec![gui_model_cube(
                    [4.0, 3.0],
                    [-2.5, -3.0, -0.5],
                    [1.0, 4.0, 1.0],
                    tex,
                )],
                Vec::new(),
            ),
            gui_model_part(
                GuiPartPose::zero(),
                vec![gui_model_cube(
                    [0.0, 0.0],
                    [-0.5, -4.0, -0.5],
                    [1.0, 4.0, 1.0],
                    tex,
                )],
                Vec::new(),
            ),
            gui_model_part(
                GuiPartPose::zero(),
                vec![
                    gui_model_cube([4.0, 3.0], [1.5, -3.0, -0.5], [1.0, 4.0, 1.0], tex).mirrored(),
                ],
                Vec::new(),
            ),
        ],
    )]
}
fn skull_gui_model_parts(
    texture: &ResourceLocation,
    crop_pixels: Option<[f32; 4]>,
) -> Vec<GuiModelPart> {
    if crop_pixels == Some([112.0, 30.0, 16.0, 16.0]) {
        return dragon_head_gui_model_parts();
    }
    if texture.path.contains("piglin") {
        return piglin_head_gui_model_parts();
    }
    let tex = if texture.path.contains("skeleton") || texture.path.contains("creeper") {
        [64.0, 32.0]
    } else {
        [64.0, 64.0]
    };
    let mut children = Vec::new();
    if texture.path.contains("player") || texture.path.contains("zombie") {
        children.push(gui_model_part(
            GuiPartPose::zero(),
            vec![gui_model_cube([32.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], tex).grown(0.25)],
            Vec::new(),
        ));
    }
    vec![gui_model_part(
        GuiPartPose::zero(),
        vec![gui_model_cube(
            [0.0, 0.0],
            [-4.0, -8.0, -4.0],
            [8.0, 8.0, 8.0],
            tex,
        )],
        children,
    )]
}
fn piglin_head_gui_model_parts() -> Vec<GuiModelPart> {
    let tex = [64.0, 64.0];
    vec![gui_model_part(
        GuiPartPose::zero(),
        vec![
            gui_model_cube([0.0, 0.0], [-5.0, -8.0, -4.0], [10.0, 8.0, 8.0], tex),
            gui_model_cube([31.0, 1.0], [-2.0, -4.0, -5.0], [4.0, 4.0, 1.0], tex),
            gui_model_cube([2.0, 4.0], [2.0, -2.0, -5.0], [1.0, 2.0, 1.0], tex),
            gui_model_cube([2.0, 0.0], [-3.0, -2.0, -5.0], [1.0, 2.0, 1.0], tex),
        ],
        vec![
            gui_model_part(
                GuiPartPose::offset_and_rotation(
                    4.5,
                    -6.0,
                    0.0,
                    0.0,
                    0.0,
                    -std::f32::consts::FRAC_PI_6,
                ),
                vec![gui_model_cube(
                    [51.0, 6.0],
                    [0.0, 0.0, -2.0],
                    [1.0, 5.0, 4.0],
                    tex,
                )],
                Vec::new(),
            ),
            gui_model_part(
                GuiPartPose::offset_and_rotation(
                    -4.5,
                    -6.0,
                    0.0,
                    0.0,
                    0.0,
                    std::f32::consts::FRAC_PI_6,
                ),
                vec![gui_model_cube(
                    [39.0, 6.0],
                    [-1.0, 0.0, -2.0],
                    [1.0, 5.0, 4.0],
                    tex,
                )],
                Vec::new(),
            ),
        ],
    )]
}
fn dragon_head_gui_model_parts() -> Vec<GuiModelPart> {
    let tex = [256.0, 256.0];
    vec![gui_model_part(
        GuiPartPose::offset(0.0, -7.986666, 0.0).scaled(0.75),
        vec![
            gui_model_cube([176.0, 44.0], [-6.0, -1.0, -24.0], [12.0, 5.0, 16.0], tex),
            gui_model_cube([112.0, 30.0], [-8.0, -8.0, -10.0], [16.0, 16.0, 16.0], tex),
            gui_model_cube([0.0, 0.0], [-5.0, -12.0, -4.0], [2.0, 4.0, 6.0], tex).mirrored(),
            gui_model_cube([112.0, 0.0], [-5.0, -3.0, -22.0], [2.0, 2.0, 4.0], tex).mirrored(),
            gui_model_cube([0.0, 0.0], [3.0, -12.0, -4.0], [2.0, 4.0, 6.0], tex),
            gui_model_cube([112.0, 0.0], [3.0, -3.0, -22.0], [2.0, 2.0, 4.0], tex),
        ],
        vec![gui_model_part(
            GuiPartPose::offset(0.0, 4.0, -8.0),
            vec![gui_model_cube(
                [176.0, 65.0],
                [-6.0, 0.0, -16.0],
                [12.0, 4.0, 16.0],
                tex,
            )],
            Vec::new(),
        )],
    )]
}
fn copper_golem_statue_gui_model_parts() -> Vec<GuiModelPart> {
    let tex = [64.0, 64.0];
    vec![gui_model_part(
        GuiPartPose::offset_and_rotation(0.0, 0.0, 0.0, 0.0, 0.0, 180.0_f32.to_radians()),
        Vec::new(),
        vec![
            gui_model_part(
                GuiPartPose::offset(0.0, -5.0, 0.0),
                vec![gui_model_cube(
                    [0.0, 15.0],
                    [-4.0, -6.0, -3.0],
                    [8.0, 6.0, 6.0],
                    tex,
                )],
                vec![
                    gui_model_part(
                        GuiPartPose::offset(0.0, -6.0, 0.0),
                        vec![
                            gui_model_cube([0.0, 0.0], [-4.0, -5.0, -5.0], [8.0, 5.0, 10.0], tex)
                                .grown(0.015),
                            gui_model_cube([56.0, 0.0], [-1.0, -2.0, -6.0], [2.0, 3.0, 2.0], tex),
                            gui_model_cube([37.0, 8.0], [-1.0, -9.0, -1.0], [2.0, 4.0, 2.0], tex)
                                .grown(-0.015),
                            gui_model_cube([37.0, 0.0], [-2.0, -13.0, -2.0], [4.0, 4.0, 4.0], tex)
                                .grown(-0.015),
                        ],
                        Vec::new(),
                    ),
                    gui_model_part(
                        GuiPartPose::offset(-4.0, -6.0, 0.0),
                        vec![gui_model_cube(
                            [36.0, 16.0],
                            [-3.0, -1.0, -2.0],
                            [3.0, 10.0, 4.0],
                            tex,
                        )],
                        Vec::new(),
                    ),
                    gui_model_part(
                        GuiPartPose::offset(4.0, -6.0, 0.0),
                        vec![gui_model_cube(
                            [50.0, 16.0],
                            [0.0, -1.0, -2.0],
                            [3.0, 10.0, 4.0],
                            tex,
                        )],
                        Vec::new(),
                    ),
                ],
            ),
            gui_model_part(
                GuiPartPose::offset(0.0, -5.0, 0.0),
                vec![gui_model_cube(
                    [0.0, 27.0],
                    [-4.0, 0.0, -2.0],
                    [4.0, 5.0, 4.0],
                    tex,
                )],
                Vec::new(),
            ),
            gui_model_part(
                GuiPartPose::offset(0.0, -5.0, 0.0),
                vec![gui_model_cube(
                    [16.0, 27.0],
                    [0.0, 0.0, -2.0],
                    [4.0, 5.0, 4.0],
                    tex,
                )],
                Vec::new(),
            ),
        ],
    )]
}
fn collect_gui_item_models(
    resource_manager: &ClientResourceManager,
    value: &serde_json::Value,
    mut current_transform: Mat4,
    current_tints: &[GuiItemTintSource],
    out: &mut Vec<(ResourceLocation, Mat4, Vec<GuiItemTintSource>)>,
) {
    let Some(object) = value.as_object() else {
        return;
    };
    if let Some(trans_val) = object.get("transformation") {
        current_transform = current_transform * parse_json_transformation(trans_val);
    }
    if is_display_context_select(object) {
        if let Some(model) = display_context_gui_model(object) {
            collect_gui_item_models(
                resource_manager,
                model,
                current_transform,
                current_tints,
                out,
            );
        }
        return;
    }
    match object.get("type").and_then(|value| value.as_str()) {
        Some("minecraft:model") | Some("model") => {
            if let Some(model_str) = object.get("model").and_then(|value| value.as_str()) {
                let model = ResourceLocation::from_string(model_str);
                let tints = item_layer_tint_sources(resource_manager, object);
                let merged_tints = if tints.is_empty() {
                    current_tints.to_vec()
                } else {
                    tints
                };
                out.push((model, current_transform, merged_tints));
            }
        }
        Some("minecraft:special") | Some("special") => {
            if let Some(base_str) = object.get("base").and_then(|value| value.as_str()) {
                let base = ResourceLocation::from_string(base_str);
                out.push((base, current_transform, Vec::new()));
            }
        }
        Some("minecraft:composite") | Some("composite") => {
            if let Some(models) = object.get("models").and_then(|value| value.as_array()) {
                for sub_model_val in models {
                    collect_gui_item_models(
                        resource_manager,
                        sub_model_val,
                        current_transform,
                        current_tints,
                        out,
                    );
                }
            }
        }
        _ => {
            let mut sub_collected = Vec::new();
            for key in ["on_false", "on_true", "fallback", "model"] {
                if let Some(val) = object.get(key) {
                    collect_gui_item_models(
                        resource_manager,
                        val,
                        current_transform,
                        current_tints,
                        &mut sub_collected,
                    );
                    if !sub_collected.is_empty() {
                        out.extend(sub_collected);
                        return;
                    }
                }
            }
            if let Some(entries) = object.get("entries").and_then(|value| value.as_array()) {
                for entry in entries {
                    if let Some(val) = entry.get("model") {
                        collect_gui_item_models(
                            resource_manager,
                            val,
                            current_transform,
                            current_tints,
                            &mut sub_collected,
                        );
                        if !sub_collected.is_empty() {
                            out.extend(sub_collected);
                            return;
                        }
                    }
                }
            }
            if let Some(models) = object.get("models").and_then(|value| value.as_array()) {
                for model in models {
                    collect_gui_item_models(
                        resource_manager,
                        model,
                        current_transform,
                        current_tints,
                        out,
                    );
                }
            }
            if let Some(cases) = object.get("cases").and_then(|value| value.as_array()) {
                for case in cases {
                    if let Some(val) = case.get("model") {
                        collect_gui_item_models(
                            resource_manager,
                            val,
                            current_transform,
                            current_tints,
                            &mut sub_collected,
                        );
                        if !sub_collected.is_empty() {
                            out.extend(sub_collected);
                            return;
                        }
                    }
                }
            }
        }
    }
}
fn build_item_model_texture_index(
    resource_manager: &ClientResourceManager,
) -> (
    HashMap<ResourceLocation, GuiFlatItemModel>,
    HashMap<ResourceLocation, GuiBlockItemModel>,
    HashMap<ResourceLocation, GuiSpecialItemModel>,
    HashMap<ResourceLocation, serde_json::Value>,
) {
    let mut flat_index = HashMap::new();
    let mut block_index = HashMap::new();
    let mut special_index = HashMap::new();
    let mut item_definitions = HashMap::new();
    if let Ok(item_definition_ids) = resource_manager.list_item_definition_ids("minecraft") {
        for item_id in item_definition_ids {
            let Ok(text) =
                resource_manager.read_string(ClientResourceKind::ItemDefinition, &item_id)
            else {
                continue;
            };
            let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
                continue;
            };
            item_definitions.insert(item_id.clone(), json.clone());
            if let Some(mut special_model) = json.get("model").and_then(first_special_item_model) {
                special_model.gui_transform =
                    special_item_gui_transform(resource_manager, json.get("model"));
                special_index.insert(item_id.clone(), special_model);
                continue;
            }
            let mut collected = Vec::new();
            if let Some(model_val) = json.get("model") {
                collect_gui_item_models(
                    resource_manager,
                    model_val,
                    Mat4::IDENTITY,
                    &[],
                    &mut collected,
                );
            }
            if collected.is_empty() {
                continue;
            }
            let mut baked = false;
            if collected.len() == 1 {
                let (model_id, _transform, tints) = &collected[0];
                if let Some(model) = flat_item_model_from_model(resource_manager, model_id, tints) {
                    flat_index.insert(item_id.clone(), model);
                    baked = true;
                }
            }
            if !baked {
                let static_collected: Vec<_> = collected
                    .iter()
                    .map(|(model_id, transform, tints)| {
                        (
                            model_id.clone(),
                            *transform,
                            tints
                                .iter()
                                .copied()
                                .map(GuiItemTintSource::default_color)
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect();
                if let Some(model) =
                    gui_block_item_model_from_composite(resource_manager, &static_collected)
                {
                    block_index.insert(item_id.clone(), model);
                }
            }
        }
    }
    let Ok(model_ids) = resource_manager.list_model_ids_in_directory("minecraft", "item/") else {
        return (flat_index, block_index, special_index, item_definitions);
    };
    for model_id in model_ids {
        let Some(item_path) = model_id.path.strip_prefix("item/") else {
            continue;
        };
        let item_id = ResourceLocation::new(&model_id.namespace, item_path);
        if flat_index.contains_key(&item_id)
            || block_index.contains_key(&item_id)
            || special_index.contains_key(&item_id)
        {
            continue;
        }
        if let Some(model) = flat_item_model_from_model(resource_manager, &model_id, &[]) {
            flat_index.insert(item_id, model);
        }
    }
    (flat_index, block_index, special_index, item_definitions)
}
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
struct GuiItemModelReference {
    model: ResourceLocation,
    tints: Vec<[f32; 4]>,
}
#[allow(dead_code)]
fn first_item_model_reference_with_tints(
    resource_manager: &ClientResourceManager,
    value: &serde_json::Value,
) -> Option<GuiItemModelReference> {
    let object = value.as_object()?;
    if is_display_context_select(object) {
        if let Some(model) = display_context_gui_model(object) {
            return first_item_model_reference_with_tints(resource_manager, model);
        }
    }
    match object.get("type").and_then(|value| value.as_str()) {
        Some("minecraft:model") => {
            return object
                .get("model")
                .and_then(|value| value.as_str())
                .map(|model| GuiItemModelReference {
                    model: ResourceLocation::from_string(model),
                    tints: item_layer_tints(resource_manager, object),
                });
        }
        Some("minecraft:special") => {
            if let Some(base) = object.get("base").and_then(|value| value.as_str()) {
                return Some(GuiItemModelReference {
                    model: ResourceLocation::from_string(base),
                    tints: Vec::new(),
                });
            }
        }
        _ => {}
    }
    for key in ["on_false", "on_true", "fallback"] {
        if let Some(model_ref) = object
            .get(key)
            .and_then(|value| first_item_model_reference_with_tints(resource_manager, value))
        {
            return Some(model_ref);
        }
    }
    if let Some(entries) = object.get("entries").and_then(|value| value.as_array()) {
        for entry in entries {
            if let Some(model_ref) = entry
                .get("model")
                .and_then(|value| first_item_model_reference_with_tints(resource_manager, value))
            {
                return Some(model_ref);
            }
        }
    }
    if let Some(model) = object.get("model") {
        if let Some(model_ref) = first_item_model_reference_with_tints(resource_manager, model) {
            return Some(model_ref);
        }
    }
    if let Some(models) = object.get("models").and_then(|value| value.as_array()) {
        for model in models {
            if let Some(model_ref) = first_item_model_reference_with_tints(resource_manager, model)
            {
                return Some(model_ref);
            }
        }
    }
    if let Some(cases) = object.get("cases").and_then(|value| value.as_array()) {
        for case in cases {
            if let Some(model_ref) = case
                .get("model")
                .and_then(|value| first_item_model_reference_with_tints(resource_manager, value))
            {
                return Some(model_ref);
            }
        }
    }
    None
}
#[allow(dead_code)]
fn first_item_model_reference(value: &serde_json::Value) -> Option<ResourceLocation> {
    let object = value.as_object()?;
    if is_display_context_select(object) {
        if let Some(model) = display_context_gui_model(object) {
            return first_item_model_reference(model);
        }
    }
    match object.get("type").and_then(|value| value.as_str()) {
        Some("minecraft:model") => {
            return object
                .get("model")
                .and_then(|value| value.as_str())
                .map(ResourceLocation::from_string);
        }
        Some("minecraft:special") => {
            if let Some(base) = object.get("base").and_then(|value| value.as_str()) {
                return Some(ResourceLocation::from_string(base));
            }
        }
        _ => {}
    }
    for key in ["on_false", "on_true", "fallback"] {
        if let Some(model_ref) = object.get(key).and_then(first_item_model_reference) {
            return Some(model_ref);
        }
    }
    if let Some(entries) = object.get("entries").and_then(|value| value.as_array()) {
        for entry in entries {
            if let Some(model_ref) = entry.get("model").and_then(first_item_model_reference) {
                return Some(model_ref);
            }
        }
    }
    if let Some(model) = object.get("model") {
        if let Some(model_ref) = first_item_model_reference(model) {
            return Some(model_ref);
        }
    }
    if let Some(models) = object.get("models").and_then(|value| value.as_array()) {
        for model in models {
            if let Some(model_ref) = first_item_model_reference(model) {
                return Some(model_ref);
            }
        }
    }
    if let Some(cases) = object.get("cases").and_then(|value| value.as_array()) {
        for case in cases {
            if let Some(model_ref) = case.get("model").and_then(first_item_model_reference) {
                return Some(model_ref);
            }
        }
    }
    None
}
fn first_special_item_model(value: &serde_json::Value) -> Option<GuiSpecialItemModel> {
    let object = value.as_object()?;
    if is_display_context_select(object) {
        if let Some(model) = display_context_gui_model(object) {
            return first_special_item_model(model);
        }
    }
    if object.get("type").and_then(|value| value.as_str()) == Some("minecraft:special") {
        if let Some(model) = object
            .get("model")
            .and_then(gui_special_item_model_from_json)
        {
            return Some(model);
        }
    }
    for key in ["on_false", "fallback", "on_true"] {
        if let Some(model) = object.get(key).and_then(first_special_item_model) {
            return Some(model);
        }
    }
    if let Some(model) = object.get("model").and_then(first_special_item_model) {
        return Some(model);
    }
    if let Some(entries) = object.get("entries").and_then(|value| value.as_array()) {
        for entry in entries {
            if let Some(model) = entry.get("model").and_then(first_special_item_model) {
                return Some(model);
            }
        }
    }
    if let Some(cases) = object.get("cases").and_then(|value| value.as_array()) {
        for case in cases {
            if let Some(model) = case.get("model").and_then(first_special_item_model) {
                return Some(model);
            }
        }
    }
    if let Some(models) = object.get("models").and_then(|value| value.as_array()) {
        for model in models {
            if let Some(model) = first_special_item_model(model) {
                return Some(model);
            }
        }
    }
    None
}
fn is_display_context_select(object: &serde_json::Map<String, serde_json::Value>) -> bool {
    object.get("type").and_then(|value| value.as_str()) == Some("minecraft:select")
        && object.get("property").and_then(|value| value.as_str())
            == Some("minecraft:display_context")
}
fn display_context_gui_model<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
) -> Option<&'a serde_json::Value> {
    let cases = object.get("cases")?.as_array()?;
    cases
        .iter()
        .find(|case| {
            case.get("when")
                .is_some_and(|when| display_context_when_contains_gui(when))
        })
        .and_then(|case| case.get("model"))
}
fn display_context_when_contains_gui(when: &serde_json::Value) -> bool {
    if when.as_str() == Some("gui") {
        return true;
    }
    when.as_array()
        .is_some_and(|values| values.iter().any(|value| value.as_str() == Some("gui")))
}
fn gui_special_item_model_from_json(value: &serde_json::Value) -> Option<GuiSpecialItemModel> {
    let object = value.as_object()?;
    let model_type = object.get("type")?.as_str()?;
    let model_type = model_type.strip_prefix("minecraft:").unwrap_or(model_type);
    match model_type {
        "shield" => Some(GuiSpecialItemModel {
            kind: GuiSpecialItemKind::Shield,
            texture: ResourceLocation::new("minecraft", "entity/shield/shield_base_nopattern"),
            crop_pixels: None,
            tint: [1.0, 1.0, 1.0, 1.0],
            gui_transform: Mat4::IDENTITY,
            banner_color: None,
            chest: GuiChestParams::default(),
        }),
        "banner" => {
            let color_str = object
                .get("color")
                .and_then(|value| value.as_str())
                .map(|s| s.to_string());
            let color = color_str
                .as_deref()
                .and_then(dye_color_rgb)
                .unwrap_or([1.0, 1.0, 1.0, 1.0]);
            Some(GuiSpecialItemModel {
                kind: GuiSpecialItemKind::Banner,
                texture: ResourceLocation::new("minecraft", "entity/banner/banner_base"),
                crop_pixels: None,
                tint: color,
                gui_transform: Mat4::IDENTITY,
                banner_color: color_str,
                chest: GuiChestParams::default(),
            })
        }
        "chest" => {
            let texture = object
                .get("texture")
                .and_then(|value| value.as_str())
                .map(chest_special_texture)
                .unwrap_or_else(|| ResourceLocation::new("minecraft", "entity/chest/normal"));
            let openness = match object.get("openness") {
                Some(value) => value.as_f64()? as f32,
                None => 0.0,
            };
            let chest_type = match object.get("chest_type") {
                Some(value) => GuiChestType::from_serialized_name(value.as_str()?)?,
                None => GuiChestType::Single,
            };
            Some(GuiSpecialItemModel {
                kind: GuiSpecialItemKind::Chest,
                texture,
                crop_pixels: None,
                tint: [1.0, 1.0, 1.0, 1.0],
                gui_transform: Mat4::IDENTITY,
                banner_color: None,
                chest: GuiChestParams {
                    openness,
                    chest_type,
                },
            })
        }
        "shulker_box" => {
            let texture = object
                .get("texture")
                .and_then(|value| value.as_str())
                .map(shulker_box_special_texture)
                .unwrap_or_else(|| ResourceLocation::new("minecraft", "entity/shulker/shulker"));
            Some(GuiSpecialItemModel {
                kind: GuiSpecialItemKind::ShulkerBox,
                texture,
                crop_pixels: None,
                tint: [1.0, 1.0, 1.0, 1.0],
                gui_transform: Mat4::IDENTITY,
                banner_color: None,
                chest: GuiChestParams::default(),
            })
        }
        "decorated_pot" => Some(GuiSpecialItemModel {
            kind: GuiSpecialItemKind::DecoratedPot,
            texture: ResourceLocation::new("minecraft", "entity/decorated_pot/decorated_pot_base"),
            crop_pixels: None,
            tint: [1.0, 1.0, 1.0, 1.0],
            gui_transform: Mat4::IDENTITY,
            banner_color: None,
            chest: GuiChestParams::default(),
        }),
        "conduit" => Some(GuiSpecialItemModel {
            kind: GuiSpecialItemKind::Conduit,
            texture: ResourceLocation::new("minecraft", "entity/conduit/base"),
            crop_pixels: None,
            tint: [1.0, 1.0, 1.0, 1.0],
            gui_transform: Mat4::IDENTITY,
            banner_color: None,
            chest: GuiChestParams::default(),
        }),
        "trident" => Some(GuiSpecialItemModel {
            kind: GuiSpecialItemKind::Trident,
            texture: ResourceLocation::new("minecraft", "entity/trident/trident"),
            crop_pixels: None,
            tint: [1.0, 1.0, 1.0, 1.0],
            gui_transform: Mat4::IDENTITY,
            banner_color: None,
            chest: GuiChestParams::default(),
        }),
        "head" | "player_head" => {
            let kind = object
                .get("kind")
                .and_then(|value| value.as_str())
                .unwrap_or("player");
            let (texture, crop_pixels) = head_special_texture(kind);
            Some(GuiSpecialItemModel {
                kind: GuiSpecialItemKind::Head,
                texture,
                crop_pixels,
                tint: [1.0, 1.0, 1.0, 1.0],
                gui_transform: Mat4::IDENTITY,
                banner_color: None,
                chest: GuiChestParams::default(),
            })
        }
        "copper_golem_statue" => {
            let texture = object
                .get("texture")
                .and_then(|value| value.as_str())
                .map(texture_id_from_special_texture)
                .unwrap_or_else(|| {
                    ResourceLocation::new("minecraft", "entity/copper_golem/copper_golem")
                });
            Some(GuiSpecialItemModel {
                kind: GuiSpecialItemKind::CopperGolemStatue,
                texture,
                crop_pixels: None,
                tint: [1.0, 1.0, 1.0, 1.0],
                gui_transform: Mat4::IDENTITY,
                banner_color: None,
                chest: GuiChestParams::default(),
            })
        }
        _ => None,
    }
}
fn parse_json_transformation(val: &serde_json::Value) -> Mat4 {
    let mut translation = Vec3::ZERO;
    let mut left_rot = glam::Quat::IDENTITY;
    let mut scale = Vec3::ONE;
    let mut right_rot = glam::Quat::IDENTITY;
    if let Some(obj) = val.as_object() {
        if let Some(t_val) = obj.get("translation").and_then(|v| v.as_array()) {
            if t_val.len() >= 3 {
                let tx = t_val[0].as_f64().unwrap_or(0.0) as f32;
                let ty = t_val[1].as_f64().unwrap_or(0.0) as f32;
                let tz = t_val[2].as_f64().unwrap_or(0.0) as f32;
                translation = Vec3::new(tx, ty, tz);
            }
        }
        if let Some(s_val) = obj.get("scale").and_then(|v| v.as_array()) {
            if s_val.len() >= 3 {
                let sx = s_val[0].as_f64().unwrap_or(1.0) as f32;
                let sy = s_val[1].as_f64().unwrap_or(1.0) as f32;
                let sz = s_val[2].as_f64().unwrap_or(1.0) as f32;
                scale = Vec3::new(sx, sy, sz);
            }
        }
        let parse_rot = |v_arr: &[serde_json::Value]| -> Option<glam::Quat> {
            if v_arr.len() == 4 {
                let x = v_arr[0].as_f64().unwrap_or(0.0) as f32;
                let y = v_arr[1].as_f64().unwrap_or(0.0) as f32;
                let z = v_arr[2].as_f64().unwrap_or(0.0) as f32;
                let w = v_arr[3].as_f64().unwrap_or(1.0) as f32;
                Some(glam::Quat::from_xyzw(x, y, z, w))
            } else if v_arr.len() == 3 {
                let rx = (v_arr[0].as_f64().unwrap_or(0.0) as f32).to_radians();
                let ry = (v_arr[1].as_f64().unwrap_or(0.0) as f32).to_radians();
                let rz = (v_arr[2].as_f64().unwrap_or(0.0) as f32).to_radians();
                Some(glam::Quat::from_euler(glam::EulerRot::XYZ, rx, ry, rz))
            } else {
                None
            }
        };
        if let Some(r_val) = obj.get("left_rotation").and_then(|v| v.as_array()) {
            if let Some(q) = parse_rot(r_val) {
                left_rot = q;
            }
        }
        if let Some(r_val) = obj.get("right_rotation").and_then(|v| v.as_array()) {
            if let Some(q) = parse_rot(r_val) {
                right_rot = q;
            }
        }
        if let Some(r_val) = obj.get("rotation").and_then(|v| v.as_array()) {
            if let Some(q) = parse_rot(r_val) {
                left_rot = q;
            }
        }
    }
    Mat4::from_translation(translation)
        * Mat4::from_quat(left_rot)
        * Mat4::from_scale(scale)
        * Mat4::from_quat(right_rot)
}
fn special_item_gui_transform(
    resource_manager: &ClientResourceManager,
    model_json: Option<&serde_json::Value>,
) -> Mat4 {
    fn find_base_id_and_transform(
        value: &serde_json::Value,
        current_transform: Mat4,
    ) -> Option<(String, Mat4)> {
        let obj = value.as_object()?;
        let parsed_transform = if let Some(t_node) = obj.get("transformation") {
            parse_json_transformation(t_node)
        } else {
            Mat4::IDENTITY
        };
        let node_transform = current_transform * parsed_transform;
        let ty = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if ty == "minecraft:special" || ty == "special" {
            let base_id = obj
                .get("base")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())?;
            return Some((base_id, node_transform));
        }
        for key in ["on_false", "on_true", "fallback", "model"] {
            if let Some(child) = obj.get(key) {
                if let Some(res) = find_base_id_and_transform(child, node_transform) {
                    return Some(res);
                }
            }
        }
        for key in ["entries", "cases", "models"] {
            if let Some(arr) = obj.get(key).and_then(|v| v.as_array()) {
                for item in arr {
                    let child = item.get("model").unwrap_or(item);
                    if let Some(res) = find_base_id_and_transform(child, node_transform) {
                        return Some(res);
                    }
                }
            }
        }
        None
    }
    let Some(val) = model_json else {
        return Mat4::IDENTITY;
    };
    let Some((base_id_str, extra_transform)) = find_base_id_and_transform(val, Mat4::IDENTITY)
    else {
        return Mat4::IDENTITY;
    };
    let base_id = ResourceLocation::from_string(&base_id_str);
    let model_id = if base_id.path.starts_with("item/") {
        base_id.clone()
    } else {
        ResourceLocation::new(&base_id.namespace, &format!("item/{}", base_id.path))
    };
    if let Ok(model) = BlockModel::load(resource_manager, &model_id) {
        let gui_transform = model.item_transform("gui").matrix(false);
        gui_transform * extra_transform
    } else {
        extra_transform
    }
}
fn local_time_select_value(object: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    let pattern = object.get("pattern")?.as_str()?;
    let locale = match object.get("locale") {
        Some(value) => value.as_str()?,
        None => "",
    };
    if !locale.is_empty() {
        return None;
    }
    match object.get("time_zone") {
        None => {}
        Some(value) if matches!(value.as_str(), Some("UTC") | Some("GMT")) => {}
        Some(_) => return None,
    }
    let unix_seconds = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    format_local_time_pattern(pattern, unix_seconds)
}
fn format_local_time_pattern(pattern: &str, unix_seconds: i64) -> Option<String> {
    let (year, month, day) = civil_from_unix_days(unix_seconds.div_euclid(86_400));
    let second_of_day = unix_seconds.rem_euclid(86_400);
    let (hour, minute, second) = (
        second_of_day / 3600,
        (second_of_day % 3600) / 60,
        second_of_day % 60,
    );
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::new();
    let mut index = 0;
    while index < chars.len() {
        let current = chars[index];
        if current == '\'' {
            index += 1;
            if chars.get(index) == Some(&'\'') {
                out.push('\'');
                index += 1;
                continue;
            }
            while index < chars.len() && chars[index] != '\'' {
                out.push(chars[index]);
                index += 1;
            }
            if index == chars.len() {
                return None;
            }
            index += 1;
            continue;
        }
        if !current.is_ascii_alphabetic() {
            out.push(current);
            index += 1;
            continue;
        }
        let mut count = 0usize;
        while chars.get(index) == Some(&current) {
            count += 1;
            index += 1;
        }
        let value = match current {
            'y' if count == 2 => year.rem_euclid(100),
            'y' => year,
            'M' if count <= 2 => i64::from(month),
            'd' => i64::from(day),
            'H' => hour,
            'm' => minute,
            's' => second,
            _ => return None,
        };
        out.push_str(&format!("{value:0count$}"));
    }
    Some(out)
}
fn civil_from_unix_days(unix_days: i64) -> (i64, u32, u32) {
    let shifted = unix_days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}
fn chest_special_texture(texture: &str) -> ResourceLocation {
    let loc = ResourceLocation::from_string(texture);
    let name = loc.path.strip_prefix("entity/chest/").unwrap_or(&loc.path);
    ResourceLocation::new(&loc.namespace, &format!("entity/chest/{name}"))
}
fn shulker_box_special_texture(texture: &str) -> ResourceLocation {
    let loc = ResourceLocation::from_string(texture);
    let name = loc
        .path
        .strip_prefix("entity/shulker/")
        .unwrap_or(&loc.path);
    ResourceLocation::new(&loc.namespace, &format!("entity/shulker/{name}"))
}
fn head_special_texture(kind: &str) -> (ResourceLocation, Option<[f32; 4]>) {
    match kind {
        "skeleton" => (
            ResourceLocation::new("minecraft", "entity/skeleton/skeleton"),
            Some([8.0, 8.0, 8.0, 8.0]),
        ),
        "wither_skeleton" => (
            ResourceLocation::new("minecraft", "entity/skeleton/wither_skeleton"),
            Some([8.0, 8.0, 8.0, 8.0]),
        ),
        "zombie" => (
            ResourceLocation::new("minecraft", "entity/zombie/zombie"),
            Some([8.0, 8.0, 8.0, 8.0]),
        ),
        "creeper" => (
            ResourceLocation::new("minecraft", "entity/creeper/creeper"),
            Some([8.0, 8.0, 8.0, 8.0]),
        ),
        "piglin" => (
            ResourceLocation::new("minecraft", "entity/piglin/piglin"),
            Some([8.0, 8.0, 8.0, 8.0]),
        ),
        "dragon" => (
            ResourceLocation::new("minecraft", "entity/enderdragon/dragon"),
            Some([112.0, 30.0, 16.0, 16.0]),
        ),
        _ => (
            ResourceLocation::new("minecraft", "entity/player/wide/steve"),
            Some([8.0, 8.0, 8.0, 8.0]),
        ),
    }
}
fn texture_id_from_special_texture(texture: &str) -> ResourceLocation {
    let loc = ResourceLocation::from_string(texture);
    let path = loc
        .path
        .strip_prefix("textures/")
        .unwrap_or(&loc.path)
        .strip_suffix(".png")
        .unwrap_or_else(|| loc.path.strip_prefix("textures/").unwrap_or(&loc.path));
    ResourceLocation::new(&loc.namespace, path)
}
fn dye_color_rgb(color: &str) -> Option<[f32; 4]> {
    let rgb = match color {
        "white" => 0xF9FFFE,
        "orange" => 0xF9801D,
        "magenta" => 0xC74EBD,
        "light_blue" => 0x3AB3DA,
        "yellow" => 0xFED83D,
        "lime" => 0x80C71F,
        "pink" => 0xF38BAA,
        "gray" => 0x474F52,
        "light_gray" => 0x9D9D97,
        "cyan" => 0x169C9C,
        "purple" => 0x8932B8,
        "blue" => 0x3C44AA,
        "brown" => 0x835432,
        "green" => 0x5E7C16,
        "red" => 0xB02E26,
        "black" => 0x1D1D21,
        _ => return None,
    };
    Some([
        ((rgb >> 16) & 0xFF) as f32 / 255.0,
        ((rgb >> 8) & 0xFF) as f32 / 255.0,
        (rgb & 0xFF) as f32 / 255.0,
        1.0,
    ])
}
#[allow(dead_code)]
fn item_layer_tints(
    resource_manager: &ClientResourceManager,
    object: &serde_json::Map<String, serde_json::Value>,
) -> Vec<[f32; 4]> {
    item_layer_tint_sources(resource_manager, object)
        .into_iter()
        .map(GuiItemTintSource::default_color)
        .collect()
}
fn item_layer_tint_sources(
    resource_manager: &ClientResourceManager,
    object: &serde_json::Map<String, serde_json::Value>,
) -> Vec<GuiItemTintSource> {
    object
        .get("tints")
        .and_then(|value| value.as_array())
        .map(|tints| {
            tints
                .iter()
                .filter_map(|tint| item_tint_source(resource_manager, tint))
                .collect()
        })
        .unwrap_or_default()
}
fn item_tint_source(
    resource_manager: &ClientResourceManager,
    value: &serde_json::Value,
) -> Option<GuiItemTintSource> {
    let object = value.as_object()?;
    let tint_type = object
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let tint_type = tint_type.strip_prefix("minecraft:").unwrap_or(tint_type);
    if tint_type == "potion" {
        let default_color = object
            .get("default")
            .and_then(|value| value.as_i64())
            .map(color_from_java_rgb)
            .unwrap_or_else(|| argb_to_color(-13_083_194));
        return Some(GuiItemTintSource::Potion { default_color });
    }
    item_tint_color(resource_manager, value).map(GuiItemTintSource::Constant)
}
fn item_tint_color(
    resource_manager: &ClientResourceManager,
    value: &serde_json::Value,
) -> Option<[f32; 4]> {
    let object = value.as_object()?;
    let tint_type = object
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let tint_type = tint_type.strip_prefix("minecraft:").unwrap_or(tint_type);
    if let Some(value) = object
        .get("value")
        .or_else(|| object.get("default"))
        .and_then(|value| value.as_i64())
    {
        return Some(color_from_java_rgb(value));
    }
    match tint_type {
        "grass" => {
            let temperature = object
                .get("temperature")
                .and_then(|value| value.as_f64())
                .unwrap_or(0.5);
            let downfall = object
                .get("downfall")
                .and_then(|value| value.as_f64())
                .unwrap_or(1.0);
            Some(colormap_color(
                resource_manager,
                "grass",
                temperature,
                downfall,
                -65281,
            ))
        }
        "foliage" => {
            let temperature = object
                .get("temperature")
                .and_then(|value| value.as_f64())
                .unwrap_or(0.5);
            let downfall = object
                .get("downfall")
                .and_then(|value| value.as_f64())
                .unwrap_or(1.0);
            Some(colormap_color(
                resource_manager,
                "foliage",
                temperature,
                downfall,
                -12012264,
            ))
        }
        "dry_foliage" => {
            let temperature = object
                .get("temperature")
                .and_then(|value| value.as_f64())
                .unwrap_or(0.5);
            let downfall = object
                .get("downfall")
                .and_then(|value| value.as_f64())
                .unwrap_or(1.0);
            Some(colormap_color(
                resource_manager,
                "dry_foliage",
                temperature,
                downfall,
                -10732494,
            ))
        }
        _ => None,
    }
}
fn colormap_color(
    resource_manager: &ClientResourceManager,
    colormap: &str,
    temperature: f64,
    downfall: f64,
    default_color: i64,
) -> [f32; 4] {
    let id = ResourceLocation::new("minecraft", &format!("colormap/{colormap}"));
    let Ok(image) = resource_manager.read_image(&id) else {
        return color_from_java_rgb(default_color);
    };
    let rgba = image.to_rgba8();
    let rain = downfall * temperature;
    let x = ((1.0 - temperature) * 255.0) as u32;
    let y = ((1.0 - rain) * 255.0) as u32;
    if x >= rgba.width() || y >= rgba.height() {
        return color_from_java_rgb(default_color);
    }
    let pixel = rgba.get_pixel(x, y).0;
    [
        pixel[0] as f32 / 255.0,
        pixel[1] as f32 / 255.0,
        pixel[2] as f32 / 255.0,
        1.0,
    ]
}
fn color_from_java_rgb(value: i64) -> [f32; 4] {
    let rgb = value as i32 as u32;
    [
        ((rgb >> 16) & 0xFF) as f32 / 255.0,
        ((rgb >> 8) & 0xFF) as f32 / 255.0,
        (rgb & 0xFF) as f32 / 255.0,
        1.0,
    ]
}
fn flat_item_model_from_model(
    resource_manager: &ClientResourceManager,
    model_id: &ResourceLocation,
    tints: &[GuiItemTintSource],
) -> Option<GuiFlatItemModel> {
    let model = BlockModel::load(resource_manager, model_id).ok()?;
    let textures = model.textures.as_ref()?;
    let mut layer_indices: Vec<usize> = textures
        .keys()
        .filter_map(|key| key.strip_prefix("layer")?.parse::<usize>().ok())
        .collect();
    layer_indices.sort_unstable();
    layer_indices.dedup();
    let mut layers = Vec::new();
    for layer_index in layer_indices {
        let key = format!("#layer{layer_index}");
        let resolved = model.resolve_texture(&key);
        if resolved.starts_with('#') {
            continue;
        }
        layers.push(GuiFlatItemLayer {
            texture: texture_id_from_model_texture(resolved, "item"),
            tint: tints
                .get(layer_index)
                .copied()
                .unwrap_or(GuiItemTintSource::Constant([1.0, 1.0, 1.0, 1.0])),
        });
    }
    let gui_transform = Some(model.item_transform("gui"));
    if layers.is_empty() {
        None
    } else {
        Some(GuiFlatItemModel {
            layers,
            gui_transform,
        })
    }
}
fn gui_block_item_model_from_composite(
    resource_manager: &ClientResourceManager,
    sub_models: &[(ResourceLocation, Mat4, Vec<[f32; 4]>)],
) -> Option<GuiBlockItemModel> {
    let mut quads = Vec::new();
    for (model_id, sub_transform, tints) in sub_models {
        let Ok(model) = BlockModel::load(resource_manager, model_id) else {
            continue;
        };
        let transform = model.item_transform("gui").matrix(false);
        let combined_transform = transform * *sub_transform;
        if let Some(elements) = model.elements.as_ref() {
            for element in elements {
                let from = Vec3::from(element.from);
                let to = Vec3::from(element.to);
                let element_rotation = element.rotation.as_ref().map(|rotation| {
                    let origin = Vec3::from(rotation.origin) / 16.0;
                    let angle = rotation.angle.to_radians();
                    let matrix = match rotation.axis.as_str() {
                        "x" => Mat4::from_rotation_x(angle),
                        "y" => Mat4::from_rotation_y(angle),
                        "z" => Mat4::from_rotation_z(angle),
                        _ => Mat4::IDENTITY,
                    };
                    (origin, matrix)
                });
                for (face_name, face) in &element.faces {
                    let Some(facing) = direction_from_face_name(face_name) else {
                        continue;
                    };
                    let raw_uvs = face
                        .uv
                        .unwrap_or_else(|| FaceBakery::default_face_uv(from, to, facing));
                    let (positions, uvs, baked_direction) = FaceBakery::bake_quad(
                        from,
                        to,
                        raw_uvs,
                        face.rotation.unwrap_or(0),
                        Mat4::IDENTITY,
                        facing,
                        Mat4::IDENTITY,
                        element_rotation,
                    );
                    let texture = model.resolve_texture(&face.texture);
                    if texture.starts_with('#') {
                        continue;
                    }
                    let apply_shade = element.shade.unwrap_or(true);
                    let normal = combined_transform
                        .transform_vector3(direction_unit(baked_direction))
                        .normalize_or_zero();
                    if let Some(quad) = gui_block_item_quad(
                        texture_id_from_model_texture(texture, "block"),
                        positions.map(|position| combined_transform.transform_point3(position)),
                        uvs,
                        face.tintindex
                            .and_then(|index| tints.get(index as usize).copied())
                            .unwrap_or([1.0, 1.0, 1.0, 1.0]),
                        apply_shade,
                        normal,
                        baked_direction,
                    ) {
                        quads.push(quad);
                    }
                }
            }
        }
    }
    if quads.is_empty() {
        if let Some((first_model_id, _, _)) = sub_models.first() {
            if let Ok(model) = BlockModel::load(resource_manager, first_model_id) {
                if let Some(texture) = model_particle_texture(&model) {
                    return Some(GuiBlockItemModel {
                        quads: vec![GuiBlockItemQuad {
                            texture,
                            positions: [[-6.0, -6.0], [6.0, -6.0], [6.0, 6.0], [-6.0, 6.0]],
                            uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                            shade: 1.0,
                            depth: 0.0,
                            tint: [1.0, 1.0, 1.0, 1.0],
                        }],
                    });
                }
            }
        }
        return None;
    }
    quads.sort_by(|left, right| {
        left.depth
            .partial_cmp(&right.depth)
            .unwrap_or(Ordering::Equal)
    });
    Some(GuiBlockItemModel { quads })
}
#[allow(dead_code)]
fn gui_block_item_model_from_model(
    resource_manager: &ClientResourceManager,
    model_id: &ResourceLocation,
    tints: &[[f32; 4]],
) -> Option<GuiBlockItemModel> {
    gui_block_item_model_from_composite(
        resource_manager,
        &[(model_id.clone(), Mat4::IDENTITY, tints.to_vec())],
    )
}
fn gui_block_item_quad(
    texture: ResourceLocation,
    positions: [Vec3; 4],
    uvs: [[f32; 2]; 4],
    tint: [f32; 4],
    apply_shade: bool,
    normal: Vec3,
    _baked_direction: Direction,
) -> Option<GuiBlockItemQuad> {
    if normal.length_squared() >= 0.0001 && normal.z <= 0.0 {
        return None;
    }
    let shade = if apply_shade {
        gui_item_face_shade(normal)
    } else {
        1.0
    };
    Some(GuiBlockItemQuad {
        texture,
        positions: positions.map(|position| [position.x * 16.0, -position.y * 16.0]),
        uvs,
        shade,
        depth: positions.iter().map(|position| position.z).sum::<f32>() / 4.0,
        tint,
    })
}
fn direction_from_face_name(face_name: &str) -> Option<Direction> {
    match face_name {
        "down" => Some(Direction::Down),
        "up" => Some(Direction::Up),
        "north" => Some(Direction::North),
        "south" => Some(Direction::South),
        "west" => Some(Direction::West),
        "east" => Some(Direction::East),
        _ => None,
    }
}
fn gui_item_face_shade(normal: Vec3) -> f32 {
    if normal.length_squared() < 0.0001 {
        return 1.0;
    }
    let (light_0, light_1) = vanilla_gui_items_3d_lights();
    let normal = Vec3::new(normal.x, -normal.y, normal.z).normalize_or_zero();
    let dot_0 = normal.dot(light_0).max(0.0);
    let dot_1 = normal.dot(light_1).max(0.0);
    ((dot_0 + dot_1) * 0.6 + 0.4).min(1.0)
}
fn vanilla_gui_items_3d_lights() -> (Vec3, Vec3) {
    let pose = Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0))
        * Mat4::from_euler(EulerRot::YXZ, 1.0821041, 3.2375858, 0.0)
        * Mat4::from_euler(
            EulerRot::YXZ,
            -std::f32::consts::PI / 8.0,
            std::f32::consts::PI * 3.0 / 4.0,
            0.0,
        );
    let diffuse_0 = Vec3::new(0.2, 1.0, -0.7).normalize();
    let diffuse_1 = Vec3::new(-0.2, 1.0, 0.7).normalize();
    (
        pose.transform_vector3(diffuse_0).normalize_or_zero(),
        pose.transform_vector3(diffuse_1).normalize_or_zero(),
    )
}
fn model_particle_texture(model: &BlockModel) -> Option<ResourceLocation> {
    let resolved = model.resolve_texture("#particle");
    if resolved.starts_with('#') {
        return None;
    }
    Some(texture_id_from_model_texture(resolved, "block"))
}
fn texture_id_from_model_texture(texture: &str, default_prefix: &str) -> ResourceLocation {
    let loc = ResourceLocation::from_string(texture);
    let path = if loc.path.contains('/') {
        loc.path
    } else {
        format!("{}/{}", default_prefix, loc.path)
    };
    ResourceLocation::new(&loc.namespace, &path)
}
pub(crate) fn clip_color_polygon_to_rect(polygon: &[[f32; 6]], rect: [f32; 4]) -> Vec<[f32; 6]> {
    if polygon.is_empty() {
        return Vec::new();
    }
    let (left, top, right, bottom) = (rect[0], rect[1], rect[2], rect[3]);
    let mut result: Vec<[f32; 6]> = polygon.to_vec();
    for axis in 0..4 {
        let (keep_if_positive, limit) = match axis {
            0 => (true, left),
            1 => (false, right),
            2 => (true, top),
            _ => (false, bottom),
        };
        let coordinate =
            |vertex: &[f32; 6]| -> f32 { if axis < 2 { vertex[0] } else { vertex[1] } };
        let mut clipped: Vec<[f32; 6]> = Vec::with_capacity(result.len() + 1);
        let count = result.len();
        for index in 0..count {
            let current = result[index];
            let next = result[(index + 1) % count];
            let (current_inside, next_inside) = if keep_if_positive {
                (coordinate(&current) >= limit, coordinate(&next) >= limit)
            } else {
                (coordinate(&current) <= limit, coordinate(&next) <= limit)
            };
            if next_inside {
                if !current_inside {
                    clipped.push(lerp_color_vertex(
                        &current,
                        &next,
                        crossing_t(coordinate(&current), coordinate(&next), limit),
                    ));
                }
                clipped.push(next);
            } else if current_inside {
                clipped.push(lerp_color_vertex(
                    &current,
                    &next,
                    crossing_t(coordinate(&current), coordinate(&next), limit),
                ));
            }
        }
        result = clipped;
        if result.is_empty() {
            return result;
        }
    }
    result
}
fn crossing_t(from: f32, to: f32, limit: f32) -> f32 {
    (limit - from) / (to - from)
}
fn lerp_color_vertex(current: &[f32; 6], next: &[f32; 6], t: f32) -> [f32; 6] {
    [
        current[0] + (next[0] - current[0]) * t,
        current[1] + (next[1] - current[1]) * t,
        current[2] + (next[2] - current[2]) * t,
        current[3] + (next[3] - current[3]) * t,
        current[4] + (next[4] - current[4]) * t,
        current[5] + (next[5] - current[5]) * t,
    ]
}
fn compound_get<'a>(fields: &'a [(String, Tag)], key: &str) -> Option<&'a Tag> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}
fn compound_string(fields: &[(String, Tag)], key: &str) -> Option<String> {
    match compound_get(fields, key)? {
        Tag::String(value) => Some(value.clone()),
        Tag::Compound(nested) => compound_string(nested, "id")
            .or_else(|| compound_string(nested, "name"))
            .or_else(|| compound_string(nested, "asset_id")),
        _ => None,
    }
}
fn player_head_skin_url(item: &ItemStack) -> Option<String> {
    let DataComponent::Generic {
        value: Tag::Compound(profile),
        ..
    } = item.components.get("minecraft:profile")?
    else {
        return None;
    };
    let properties = compound_get(profile, "properties")?;
    let mut texture_values = Vec::new();
    match properties {
        Tag::List(properties) => {
            for property in properties {
                let Tag::Compound(property) = property else {
                    continue;
                };
                if compound_get(property, "name").and_then(|value| match value {
                    Tag::String(value) => Some(value.as_str()),
                    _ => None,
                }) == Some("textures")
                    && let Some(Tag::String(encoded)) = compound_get(property, "value")
                {
                    texture_values.push(encoded.as_str());
                }
            }
        }
        Tag::Compound(properties) => {
            if let Some((_, value)) = properties.iter().find(|(name, _)| name == "textures") {
                match value {
                    Tag::List(values) => {
                        for value in values {
                            if let Tag::Compound(property) = value
                                && let Some(Tag::String(encoded)) = compound_get(property, "value")
                            {
                                texture_values.push(encoded.as_str());
                            }
                        }
                    }
                    Tag::Compound(property) => {
                        if let Some(Tag::String(encoded)) = compound_get(property, "value") {
                            texture_values.push(encoded.as_str());
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    for encoded in texture_values {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .ok()?;
        let payload = serde_json::from_slice::<serde_json::Value>(&bytes).ok()?;
        return payload
            .get("textures")
            .and_then(|textures| textures.get("SKIN"))
            .and_then(|skin| skin.get("url"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
    }
    None
}
fn player_head_profile_uuid(item: &ItemStack) -> Option<uuid::Uuid> {
    let DataComponent::Generic {
        value: Tag::Compound(profile),
        ..
    } = item.components.get("minecraft:profile")?
    else {
        return None;
    };
    let id = compound_get(profile, "id")?;
    match id {
        Tag::String(value) => uuid::Uuid::parse_str(value).ok(),
        Tag::IntArray(values) if values.len() == 4 => {
            let mut bytes = [0u8; 16];
            for (index, value) in values.iter().enumerate() {
                bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_be_bytes());
            }
            Some(uuid::Uuid::from_bytes(bytes))
        }
        _ => None,
    }
}
fn player_head_texture_id(url: &str) -> ResourceLocation {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    ResourceLocation::new("eltharn", &format!("player_head/{:016x}", hasher.finish()))
}
fn player_skin_texture_id(url: &str) -> ResourceLocation {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    ResourceLocation::new("eltharn", &format!("player_skin/{:016x}", hasher.finish()))
}
fn normalize_player_skin_image(mut image: RgbaImage) -> Option<RgbaImage> {
    let (width, height) = image.dimensions();
    if width != 64 || !matches!(height, 32 | 64) {
        return None;
    }
    if height == 32 {
        let source = image.clone();
        let mut expanded = RgbaImage::new(64, 64);
        for y in 0..32 {
            for x in 0..64 {
                expanded.put_pixel(x, y, *source.get_pixel(x, y));
            }
        }
        image = expanded;
        for (source_x, source_y, offset_x, offset_y, size_x, size_y) in [
            (4, 16, 16, 32, 4, 4),
            (8, 16, 16, 32, 4, 4),
            (0, 20, 24, 32, 4, 12),
            (4, 20, 16, 32, 4, 12),
            (8, 20, 8, 32, 4, 12),
            (12, 20, 16, 32, 4, 12),
            (44, 16, -8, 32, 4, 4),
            (48, 16, -8, 32, 4, 4),
            (40, 20, 0, 32, 4, 12),
            (44, 20, -8, 32, 4, 12),
            (48, 20, -16, 32, 4, 12),
            (52, 20, -8, 32, 4, 12),
        ] {
            copy_skin_rect(
                &mut image,
                source_x,
                source_y,
                source_x + offset_x,
                source_y + offset_y,
                size_x,
                size_y,
                true,
                false,
            );
        }
    }
    set_skin_region_opaque(&mut image, 0, 0, 32, 16);
    if height == 32 && skin_region_is_opaque(&image, 32, 0, 64, 32) {
        clear_skin_region_alpha(&mut image, 32, 0, 64, 32);
    }
    set_skin_region_opaque(&mut image, 0, 16, 64, 32);
    set_skin_region_opaque(&mut image, 16, 48, 48, 64);
    Some(image)
}
fn copy_skin_rect(
    image: &mut RgbaImage,
    source_x: i32,
    source_y: i32,
    target_x: i32,
    target_y: i32,
    width: i32,
    height: i32,
    swap_x: bool,
    swap_y: bool,
) {
    let source = image.clone();
    for y in 0..height {
        for x in 0..width {
            let sx = source_x + if swap_x { width - 1 - x } else { x };
            let sy = source_y + if swap_y { height - 1 - y } else { y };
            let tx = target_x + x;
            let ty = target_y + y;
            if (0..64).contains(&tx) && (0..64).contains(&ty) {
                image.put_pixel(
                    tx as u32,
                    ty as u32,
                    *source.get_pixel(sx as u32, sy as u32),
                );
            }
        }
    }
}
fn set_skin_region_opaque(image: &mut RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32) {
    for y in y0..y1 {
        for x in x0..x1 {
            image.get_pixel_mut(x, y).0[3] = 255;
        }
    }
}
fn skin_region_is_opaque(image: &RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32) -> bool {
    (y0..y1).all(|y| (x0..x1).all(|x| image.get_pixel(x, y).0[3] >= 128))
}
fn clear_skin_region_alpha(image: &mut RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32) {
    for y in y0..y1 {
        for x in x0..x1 {
            image.get_pixel_mut(x, y).0[3] = 0;
        }
    }
}
fn default_player_skin_index(profile_id: uuid::Uuid) -> usize {
    let bits = profile_id.as_u128();
    let most = (bits >> 64) as u64;
    let least = bits as u64;
    let folded = most ^ least;
    let java_hash = (folded ^ (folded >> 32)) as u32 as i32;
    java_hash.rem_euclid(18) as usize
}
fn default_player_skin_texture_path(profile_id: uuid::Uuid) -> String {
    const DEFAULT_NAMES: [&str; 9] = [
        "alex", "ari", "efe", "kai", "makena", "noor", "steve", "sunny", "zuri",
    ];
    let index = default_player_skin_index(profile_id);
    let model = if index < 9 { "slim" } else { "wide" };
    format!("entity/player/{model}/{}", DEFAULT_NAMES[index % 9])
}
fn gui_range_dispatch_value(
    object: &serde_json::Map<String, serde_json::Value>,
    item: &ItemStack,
    cooldown: f32,
) -> f32 {
    let normalize = object
        .get("normalize")
        .and_then(|value| value.as_bool())
        .unwrap_or(true);
    match object
        .get("property")
        .and_then(|value| value.as_str())
        .unwrap_or("")
    {
        "minecraft:count" | "count" => {
            let count = item.count as f32;
            let maximum = item.get_max_stack_size() as f32;
            if normalize {
                (count / maximum).clamp(0.0, 1.0)
            } else {
                count.clamp(0.0, maximum)
            }
        }
        "minecraft:damage" | "damage" => {
            let damage = item.get_damage_value() as f32;
            let maximum = item.get_max_damage() as f32;
            if normalize {
                (damage / maximum).clamp(0.0, 1.0)
            } else {
                damage.clamp(0.0, maximum)
            }
        }
        "minecraft:cooldown" | "cooldown" => cooldown,
        "minecraft:custom_model_data" | "custom_model_data" => {
            let index = object
                .get("index")
                .and_then(|value| value.as_u64())
                .unwrap_or(0) as usize;
            let Some(DataComponent::Generic {
                value: Tag::Compound(fields),
                ..
            }) = item.components.get("minecraft:custom_model_data")
            else {
                return 0.0;
            };
            let Some(Tag::List(floats)) = compound_get(fields, "floats") else {
                return 0.0;
            };
            match floats.get(index) {
                Some(Tag::Float(value)) => *value,
                Some(Tag::Double(value)) => *value as f32,
                Some(Tag::Int(value)) => *value as f32,
                _ => 0.0,
            }
        }
        _ => 0.0,
    }
}
fn get_potion_color_or(item: &ItemStack, default_color: [f32; 4]) -> Option<[f32; 4]> {
    if let Some(DataComponent::Generic {
        value: Tag::Compound(fields),
        ..
    }) = item.components.get("minecraft:potion_contents")
    {
        if let Some(custom_color) = compound_get(fields, "custom_color").and_then(tag_i32) {
            let c = custom_color as u32;
            return Some([
                ((c >> 16) & 0xFF) as f32 / 255.0,
                ((c >> 8) & 0xFF) as f32 / 255.0,
                (c & 0xFF) as f32 / 255.0,
                1.0,
            ]);
        }
        if let Some(Tag::List(custom_effects)) = compound_get(fields, "custom_effects") {
            let mut custom_colors = Vec::new();
            for effect in custom_effects {
                let Tag::Compound(effect_fields) = effect else {
                    continue;
                };
                let effect_id = compound_string(effect_fields, "id")
                    .or_else(|| compound_string(effect_fields, "effect"));
                let Some(effect_id) = effect_id else {
                    continue;
                };
                if !mob_effect_instance_is_visible(effect_fields) {
                    continue;
                }
                let Some(color) = potion_effect_rgb(&effect_id) else {
                    continue;
                };
                let amplifier = compound_get(effect_fields, "amplifier")
                    .and_then(tag_i32)
                    .unwrap_or(0)
                    .clamp(0, 255) as u32;
                custom_colors.push((color, amplifier + 1));
            }
            if !custom_colors.is_empty() {
                let mut effects = compound_get(fields, "potion")
                    .and_then(|value| match value {
                        Tag::String(potion_id) => Some(potion_base_effects(potion_id)),
                        _ => None,
                    })
                    .unwrap_or_default();
                effects.extend(custom_colors);
                return Some(weighted_avg_color(&effects));
            }
        }
        if let Some(Tag::String(potion_id)) = compound_get(fields, "potion") {
            let color = match potion_id.as_str() {
                "minecraft:water" | "minecraft:mundane" | "minecraft:thick"
                | "minecraft:awkward" => default_color,
                "minecraft:night_vision" | "minecraft:long_night_vision" => rgb_color(12779366),
                "minecraft:invisibility" | "minecraft:long_invisibility" => rgb_color(16185078),
                "minecraft:leaping" | "minecraft:long_leaping" | "minecraft:strong_leaping" => {
                    rgb_color(16646020)
                }
                "minecraft:fire_resistance" | "minecraft:long_fire_resistance" => {
                    rgb_color(16750848)
                }
                "minecraft:swiftness"
                | "minecraft:long_swiftness"
                | "minecraft:strong_swiftness" => rgb_color(3402751),
                "minecraft:slowness" | "minecraft:long_slowness" | "minecraft:strong_slowness" => {
                    rgb_color(9154528)
                }
                "minecraft:water_breathing" | "minecraft:long_water_breathing" => {
                    rgb_color(10017472)
                }
                "minecraft:healing" | "minecraft:strong_healing" => rgb_color(16262179),
                "minecraft:harming" | "minecraft:strong_harming" => rgb_color(11101546),
                "minecraft:poison" | "minecraft:long_poison" | "minecraft:strong_poison" => {
                    rgb_color(8889187)
                }
                "minecraft:regeneration"
                | "minecraft:long_regeneration"
                | "minecraft:strong_regeneration" => rgb_color(13458603),
                "minecraft:strength" | "minecraft:long_strength" | "minecraft:strong_strength" => {
                    rgb_color(16762624)
                }
                "minecraft:weakness" | "minecraft:long_weakness" => rgb_color(4738376),
                "minecraft:slow_falling" | "minecraft:long_slow_falling" => rgb_color(15978425),
                "minecraft:turtle_master" | "minecraft:long_turtle_master" => {
                    weighted_avg_color(&[(9154528, 4), (9520880, 3)])
                }
                "minecraft:strong_turtle_master" => {
                    weighted_avg_color(&[(9154528, 6), (9520880, 4)])
                }
                "minecraft:infestation" | "minecraft:infested" => rgb_color(9214860),
                "minecraft:oozing" => rgb_color(10092451),
                "minecraft:weaving" => rgb_color(7891290),
                "minecraft:wind_charged" => rgb_color(12438015),
                "minecraft:luck" => rgb_color(5882118),
                _ => default_color,
            };
            return Some(color);
        }
        return Some(default_color);
    }
    None
}
#[allow(dead_code)]
fn get_potion_color(item: &ItemStack) -> Option<[f32; 4]> {
    get_potion_color_or(item, argb_to_color(-13_083_194))
}
fn tag_i32(value: &Tag) -> Option<i32> {
    match value {
        Tag::Byte(value) => Some(*value as i32),
        Tag::Short(value) => Some(*value as i32),
        Tag::Int(value) => Some(*value),
        Tag::Long(value) => Some(*value as i32),
        _ => None,
    }
}
fn tag_bool(value: &Tag) -> Option<bool> {
    match value {
        Tag::Byte(value) => Some(*value != 0),
        Tag::Short(value) => Some(*value != 0),
        Tag::Int(value) => Some(*value != 0),
        Tag::Long(value) => Some(*value != 0),
        Tag::String(value) => match value.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        },
        _ => None,
    }
}
fn mob_effect_instance_is_visible(fields: &[(String, Tag)]) -> bool {
    compound_get(fields, "show_particles")
        .and_then(tag_bool)
        .unwrap_or(true)
}
fn potion_effect_rgb(effect_id: &str) -> Option<u32> {
    match effect_id.strip_prefix("minecraft:").unwrap_or(effect_id) {
        "speed" => Some(3_402_751),
        "slowness" => Some(9_154_528),
        "haste" => Some(14_270_531),
        "mining_fatigue" => Some(4_866_583),
        "strength" => Some(16_762_624),
        "instant_health" | "saturation" => Some(16_262_179),
        "instant_damage" => Some(11_101_546),
        "jump_boost" => Some(16_646_020),
        "nausea" => Some(5_578_058),
        "regeneration" => Some(13_458_603),
        "resistance" => Some(9_520_880),
        "fire_resistance" => Some(16_750_848),
        "water_breathing" => Some(10_017_472),
        "invisibility" => Some(16_185_078),
        "blindness" => Some(2_039_587),
        "night_vision" => Some(12_779_366),
        "hunger" => Some(5_797_459),
        "weakness" => Some(4_738_376),
        "poison" => Some(8_889_187),
        "wither" => Some(7_561_558),
        "health_boost" => Some(16_284_963),
        "absorption" => Some(2_445_989),
        "glowing" => Some(9_740_385),
        "levitation" => Some(13_565_951),
        "luck" => Some(5_882_118),
        "unluck" => Some(12_624_973),
        "slow_falling" => Some(15_978_425),
        "conduit_power" => Some(1_950_417),
        "dolphins_grace" => Some(8_954_814),
        "bad_omen" => Some(745_784),
        "hero_of_the_village" => Some(4_521_796),
        "darkness" => Some(2_696_993),
        "trial_omen" => Some(1_484_454),
        "raid_omen" => Some(14_565_464),
        "wind_charged" => Some(12_438_015),
        "weaving" => Some(7_891_290),
        "oozing" => Some(10_092_451),
        "infested" => Some(9_215_860),
        "breath_of_the_nautilus" => Some(65_518),
        _ => None,
    }
}
fn potion_base_effects(potion_id: &str) -> Vec<(u32, u32)> {
    match potion_id {
        "minecraft:night_vision" | "minecraft:long_night_vision" => vec![(12_779_366, 1)],
        "minecraft:invisibility" | "minecraft:long_invisibility" => vec![(16_185_078, 1)],
        "minecraft:leaping" | "minecraft:long_leaping" => vec![(16_646_020, 1)],
        "minecraft:strong_leaping" => vec![(16_646_020, 2)],
        "minecraft:fire_resistance" | "minecraft:long_fire_resistance" => {
            vec![(16_750_848, 1)]
        }
        "minecraft:swiftness" | "minecraft:long_swiftness" => vec![(3_402_751, 1)],
        "minecraft:strong_swiftness" => vec![(3_402_751, 2)],
        "minecraft:slowness" | "minecraft:long_slowness" => vec![(9_154_528, 1)],
        "minecraft:strong_slowness" => vec![(9_154_528, 2)],
        "minecraft:water_breathing" | "minecraft:long_water_breathing" => {
            vec![(10_017_472, 1)]
        }
        "minecraft:healing" | "minecraft:strong_healing" => vec![(16_262_179, 1)],
        "minecraft:harming" | "minecraft:strong_harming" => vec![(11_101_546, 1)],
        "minecraft:poison" | "minecraft:long_poison" => vec![(8_889_187, 1)],
        "minecraft:strong_poison" => vec![(8_889_187, 2)],
        "minecraft:regeneration" | "minecraft:long_regeneration" => vec![(13_458_603, 1)],
        "minecraft:strong_regeneration" => vec![(13_458_603, 2)],
        "minecraft:strength" | "minecraft:long_strength" => vec![(16_762_624, 1)],
        "minecraft:strong_strength" => vec![(16_762_624, 2)],
        "minecraft:weakness" | "minecraft:long_weakness" => vec![(4_738_376, 1)],
        "minecraft:slow_falling" | "minecraft:long_slow_falling" => vec![(15_978_425, 1)],
        "minecraft:turtle_master" => vec![(9_154_528, 4), (9_520_880, 3)],
        "minecraft:long_turtle_master" => vec![(9_154_528, 4), (9_520_880, 3)],
        "minecraft:strong_turtle_master" => vec![(9_154_528, 6), (9_520_880, 4)],
        "minecraft:infestation" | "minecraft:infested" => vec![(9_215_860, 1)],
        "minecraft:oozing" => vec![(10_092_451, 1)],
        "minecraft:weaving" => vec![(7_891_290, 1)],
        "minecraft:wind_charged" => vec![(12_438_015, 1)],
        "minecraft:luck" => vec![(5_882_118, 1)],
        _ => Vec::new(),
    }
}
fn rgb_color(rgb: u32) -> [f32; 4] {
    [
        ((rgb >> 16) & 0xFF) as f32 / 255.0,
        ((rgb >> 8) & 0xFF) as f32 / 255.0,
        (rgb & 0xFF) as f32 / 255.0,
        1.0,
    ]
}
fn argb_to_color(argb: i32) -> [f32; 4] {
    let c = argb as u32;
    [
        ((c >> 16) & 0xFF) as f32 / 255.0,
        ((c >> 8) & 0xFF) as f32 / 255.0,
        (c & 0xFF) as f32 / 255.0,
        ((c >> 24) & 0xFF) as f32 / 255.0,
    ]
}
fn weighted_avg_color(effects: &[(u32, u32)]) -> [f32; 4] {
    let mut r_total: u32 = 0;
    let mut g_total: u32 = 0;
    let mut b_total: u32 = 0;
    let mut w_total: u32 = 0;
    for &(color, weight) in effects {
        r_total += weight * ((color >> 16) & 0xFF);
        g_total += weight * ((color >> 8) & 0xFF);
        b_total += weight * (color & 0xFF);
        w_total += weight;
    }
    if w_total == 0 {
        return [0.22, 0.36, 0.78, 1.0];
    }
    [
        (r_total / w_total) as f32 / 255.0,
        (g_total / w_total) as f32 / 255.0,
        (b_total / w_total) as f32 / 255.0,
        1.0,
    ]
}
fn evaluate_tint(tint_val: &serde_json::Value, item: &ItemStack) -> Option<[f32; 4]> {
    let obj = tint_val.as_object()?;
    let ty = obj.get("type").and_then(|v| v.as_str())?;
    match ty {
        "minecraft:potion" | "potion" => {
            let default_color = obj
                .get("default")
                .and_then(|value| value.as_i64())
                .map(color_from_java_rgb)
                .unwrap_or_else(|| argb_to_color(-13_083_194));
            return Some(get_potion_color_or(item, default_color).unwrap_or(default_color));
        }
        "minecraft:dyed_color" | "dyed_color" | "minecraft:dye" | "dye" => {
            if let Some(DataComponent::Generic { value, .. }) =
                item.components.get("minecraft:dyed_color")
            {
                let color_val = match value {
                    Tag::Int(c) => Some(*c),
                    Tag::Compound(fields) => {
                        fields
                            .iter()
                            .find(|(k, _)| k == "rgb")
                            .and_then(|(_, v)| match v {
                                Tag::Int(c) => Some(*c),
                                _ => None,
                            })
                    }
                    _ => None,
                };
                if let Some(c) = color_val {
                    return Some([
                        ((c >> 16) & 0xFF) as f32 / 255.0,
                        ((c >> 8) & 0xFF) as f32 / 255.0,
                        (c & 0xFF) as f32 / 255.0,
                        1.0,
                    ]);
                }
            }
            if let Some(default_val) = obj.get("default").and_then(|v| v.as_i64()) {
                let default_color = if item.item.location.path == "leather_horse_armor" {
                    -10082445
                } else {
                    default_val as i32
                };
                return Some(argb_to_color(default_color));
            }
        }
        _ => {
            if let Some(default_val) = obj.get("default").and_then(|v| v.as_i64()) {
                let default_color = default_val as i32;
                return Some(argb_to_color(default_color));
            }
        }
    }
    None
}
