pub use crate::network::multiplayer::client_level::Difficulty;
use crate::network::multiplayer::prediction::BlockStatePredictionHandler;
use crate::network::multiplayer::{BlockDestructionProgress, BlockDestructionProgressOrder};
use crate::render::block_colors::{
    Biome, BlockAndTintGetter, ClientBlockColors, java_biome_manager_quart,
};
use el_core::block_state::BlockState;
use el_core::chest::{UPDATE_SHAPE_ORDER, chest_state_after_neighbour_update};
use el_core::collision_context::CollisionContext;
use el_core::resource_location::ResourceLocation;
use el_math::block_pos::BlockPos;
use el_math::{AABB, Vec3 as McVec3, VoxelShape};
use el_world::chunk::Chunk;
use el_world::chunk::DataLayer;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::Arc;
pub const MIN_Y_SECTION: i32 = -4;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeatherMode {
    Clear,
    Rain,
    Thunder,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}
impl ChunkPos {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }
    pub fn from_block(pos: BlockPos) -> Self {
        Self::new(pos.x >> 4, pos.z >> 4)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SectionPos {
    pub cx: i32,
    pub cy: i32,
    pub cz: i32,
}
impl SectionPos {
    pub fn new(cx: i32, cy: i32, cz: i32) -> Self {
        Self { cx, cy, cz }
    }
    pub fn from_block(pos: BlockPos) -> Self {
        Self::new(pos.x >> 4, pos.y >> 4, pos.z >> 4)
    }
}
#[derive(Debug, Default)]
pub struct QueuedLightUpdate {
    pub chunk: ChunkPos,
    pub sky: Vec<(usize, Option<DataLayer>)>,
    pub block: Vec<(usize, Option<DataLayer>)>,
    pub mark_correct: bool,
}
impl QueuedLightUpdate {
    pub fn new(chunk: ChunkPos) -> Self {
        Self {
            chunk,
            sky: Vec::new(),
            block: Vec::new(),
            mark_correct: true,
        }
    }
}
pub const OVERWORLD_CLOUD_HEIGHT: f32 = 192.33;
pub const OVERWORLD_CLOUD_FOG_END_DISTANCE: f32 = 2048.0;
pub const OVERWORLD_HORIZON_HEIGHT: f64 = 63.0;
const OVERWORLD_SKY_TEMPERATURE: f32 = 0.8;
const OVERWORLD_BASE_FOG_COLOR: u32 = 0xFFC0_D8FF;
const OVERWORLD_BASE_CLOUD_COLOR: u32 = 0xCCFF_FFFF;
const NIGHT_FOG_COLOR_MULTIPLIER_START: u32 = argb_from_float(1.0, 0.05, 0.05, 0.09);
const NIGHT_FOG_COLOR_MULTIPLIER_END: u32 = argb_from_float(1.0, 0.09, 0.09, 0.09);
const NIGHT_CLOUD_COLOR_MULTIPLIER: u32 = argb_from_float(1.0, 0.1, 0.1, 0.15);
pub(crate) const RAIN_FOG_MULTIPLIER: u32 = argb_from_float(1.0, 0.5, 0.5, 0.6);
pub(crate) const THUNDER_FOG_MULTIPLIER: u32 = argb_from_float(1.0, 0.25, 0.25, 0.3);
pub(crate) const RAIN_SUNRISE_MULTIPLIER: u32 = argb_from_float(1.0, 0.5, 0.5, 0.6);
pub(crate) const THUNDER_SUNRISE_MULTIPLIER: u32 = argb_from_float(1.0, 0.25, 0.25, 0.3);
const SKY_COLOR_MULTIPLIERS: &[(i32, u32)] = &[
    (133, 0xFFFF_FFFF),
    (11867, 0xFFFF_FFFF),
    (13670, 0xFF00_0000),
    (22330, 0xFF00_0000),
];
const FOG_COLOR_MULTIPLIERS: &[(i32, u32)] = &[
    (133, 0xFFFF_FFFF),
    (11867, 0xFFFF_FFFF),
    (13670, NIGHT_FOG_COLOR_MULTIPLIER_START),
    (22330, NIGHT_FOG_COLOR_MULTIPLIER_END),
];
const CLOUD_COLOR_MULTIPLIERS: &[(i32, u32)] = &[
    (133, 0xFFFF_FFFF),
    (11867, 0xFFFF_FFFF),
    (13670, NIGHT_CLOUD_COLOR_MULTIPLIER),
    (22330, NIGHT_CLOUD_COLOR_MULTIPLIER),
];
const SKY_LIGHT_FACTOR_MULTIPLIERS: &[(i32, f32)] =
    &[(730, 1.0), (11270, 1.0), (13140, 0.24), (22860, 0.24)];
const NIGHT_SKY_LIGHT_COLOR: u32 = argb_from_float(1.0, 0.48, 0.48, 1.0);
const SKY_LIGHT_COLOR_MULTIPLIERS: &[(i32, u32)] = &[
    (730, 0xFFFF_FFFF),
    (11270, 0xFFFF_FFFF),
    (13140, NIGHT_SKY_LIGHT_COLOR),
    (22860, NIGHT_SKY_LIGHT_COLOR),
];
const RAIN_SKY_LIGHT_COLOR: u32 = argb_from_float(0.3125, 0.48, 0.48, 1.0);
const THUNDER_SKY_LIGHT_COLOR: u32 = argb_from_float(0.527_343_75, 0.48, 0.48, 1.0);
pub(crate) const fn argb(alpha: u32, red: u32, green: u32, blue: u32) -> u32 {
    ((alpha & 0xff) << 24) | ((red & 0xff) << 16) | ((green & 0xff) << 8) | (blue & 0xff)
}
pub(crate) const fn as_8bit_channel(value: f32) -> u32 {
    (value * 255.0) as u32
}
pub(crate) const fn argb_from_float(alpha: f32, red: f32, green: f32, blue: f32) -> u32 {
    argb(
        as_8bit_channel(alpha),
        as_8bit_channel(red),
        as_8bit_channel(green),
        as_8bit_channel(blue),
    )
}
pub(crate) fn argb_to_rgb(color: u32) -> [f32; 3] {
    [
        ((color >> 16) & 0xff) as f32 / 255.0,
        ((color >> 8) & 0xff) as f32 / 255.0,
        (color & 0xff) as f32 / 255.0,
    ]
}
pub(crate) fn argb_to_rgba(color: u32) -> [f32; 4] {
    [
        ((color >> 16) & 0xff) as f32 / 255.0,
        ((color >> 8) & 0xff) as f32 / 255.0,
        (color & 0xff) as f32 / 255.0,
        ((color >> 24) & 0xff) as f32 / 255.0,
    ]
}
fn channel(color: u32, shift: u32) -> u32 {
    (color >> shift) & 0xff
}
fn lerp_int(alpha: f32, from: u32, to: u32) -> u32 {
    (from as i32 + (alpha * (to as i32 - from as i32) as f32).floor() as i32).clamp(0, 255) as u32
}
pub(crate) fn srgb_lerp(alpha: f32, from: u32, to: u32) -> u32 {
    let alpha = alpha.clamp(0.0, 1.0);
    argb(
        lerp_int(alpha, channel(from, 24), channel(to, 24)),
        lerp_int(alpha, channel(from, 16), channel(to, 16)),
        lerp_int(alpha, channel(from, 8), channel(to, 8)),
        lerp_int(alpha, channel(from, 0), channel(to, 0)),
    )
}
pub(crate) fn multiply_argb(lhs: u32, rhs: u32) -> u32 {
    if lhs == 0xFFFF_FFFF {
        rhs
    } else if rhs == 0xFFFF_FFFF {
        lhs
    } else {
        argb(
            channel(lhs, 24) * channel(rhs, 24) / 255,
            channel(lhs, 16) * channel(rhs, 16) / 255,
            channel(lhs, 8) * channel(rhs, 8) / 255,
            channel(lhs, 0) * channel(rhs, 0) / 255,
        )
    }
}
fn alpha_blend_argb(destination: u32, source: u32) -> u32 {
    let destination_alpha = channel(destination, 24);
    let source_alpha = channel(source, 24);
    if source_alpha == 0xff {
        return source;
    }
    if source_alpha == 0 {
        return destination;
    }
    let result_alpha = source_alpha + destination_alpha * (0xff - source_alpha) / 0xff;
    let blend_channel = |shift| {
        (channel(source, shift) * source_alpha
            + channel(destination, shift) * (result_alpha - source_alpha))
            / result_alpha
    };
    argb(
        result_alpha,
        blend_channel(16),
        blend_channel(8),
        blend_channel(0),
    )
}
fn scale_rgb(color: u32, scale: f32) -> u32 {
    argb(
        channel(color, 24),
        ((channel(color, 16) as f32 * scale) as i32).clamp(0, 255) as u32,
        ((channel(color, 8) as f32 * scale) as i32).clamp(0, 255) as u32,
        ((channel(color, 0) as f32 * scale) as i32).clamp(0, 255) as u32,
    )
}
fn greyscale(color: u32) -> u32 {
    let grey = (channel(color, 16) as f32 * 0.3
        + channel(color, 8) as f32 * 0.59
        + channel(color, 0) as f32 * 0.11) as u32;
    argb(channel(color, 24), grey, grey, grey)
}
fn blend_to_gray(color: u32, brightness: f32, factor: f32) -> u32 {
    srgb_lerp(factor, color, scale_rgb(greyscale(color), brightness))
}
fn sample_argb_keyframes(ticks: f32, keyframes: &[(i32, u32)]) -> u32 {
    sample_keyframes(ticks, keyframes, |alpha, from, to| {
        srgb_lerp(alpha, from, to)
    })
}
fn sample_float_keyframes(ticks: f32, keyframes: &[(i32, f32)]) -> f32 {
    sample_keyframes(ticks, keyframes, |alpha, from, to| {
        from + alpha.clamp(0.0, 1.0) * (to - from)
    })
}
fn sample_keyframes<T: Copy>(
    ticks: f32,
    keyframes: &[(i32, T)],
    lerp: impl Fn(f32, T, T) -> T,
) -> T {
    debug_assert!(!keyframes.is_empty());
    if keyframes.len() == 1 {
        return keyframes[0].1;
    }
    let looped_ticks = ticks.rem_euclid(24000.0);
    let first = keyframes[0];
    let last = keyframes[keyframes.len() - 1];
    if looped_ticks < first.0 as f32 {
        let from_ticks = last.0 as f32 - 24000.0;
        let to_ticks = first.0 as f32;
        let alpha = (looped_ticks - from_ticks) / (to_ticks - from_ticks);
        return lerp(alpha, last.1, first.1);
    }
    if looped_ticks >= last.0 as f32 {
        let from_ticks = last.0 as f32;
        let to_ticks = first.0 as f32 + 24000.0;
        let alpha = (looped_ticks - from_ticks) / (to_ticks - from_ticks);
        return lerp(alpha, last.1, first.1);
    }
    for window in keyframes.windows(2) {
        let from = window[0];
        let to = window[1];
        if looped_ticks >= from.0 as f32 && looped_ticks < to.0 as f32 {
            let alpha = (looped_ticks - from.0 as f32) / (to.0 as f32 - from.0 as f32);
            return lerp(alpha, from.1, to.1);
        }
    }
    last.1
}
fn overworld_base_sky_color() -> u32 {
    let temperature = (OVERWORLD_SKY_TEMPERATURE / 3.0).clamp(-1.0, 1.0);
    0xFF00_0000
        | hsv_to_rgb(
            0.622_222_24 - temperature * 0.05,
            0.5 + temperature * 0.1,
            1.0,
        )
}
fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> u32 {
    let h = (hue * 6.0) as i32 % 6;
    let f = hue * 6.0 - h as f32;
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - f * saturation);
    let t = value * (1.0 - (1.0 - f) * saturation);
    let (red, green, blue) = match h {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        5 => (value, p, q),
        _ => (0.0, 0.0, 0.0),
    };
    ((red * 255.0) as u32).clamp(0, 255) << 16
        | ((green * 255.0) as u32).clamp(0, 255) << 8
        | ((blue * 255.0) as u32).clamp(0, 255)
}
fn cubic_bezier(x: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    fn curve(t: f32, p1: f32, p2: f32) -> f32 {
        let a = 3.0 * p1 - 3.0 * p2 + 1.0;
        let b = -6.0 * p1 + 3.0 * p2;
        let c = 3.0 * p1;
        ((a * t + b) * t + c) * t
    }
    fn gradient(t: f32, p1: f32, p2: f32) -> f32 {
        let a = 3.0 * p1 - 3.0 * p2 + 1.0;
        let b = -6.0 * p1 + 3.0 * p2;
        let c = 3.0 * p1;
        (3.0 * a * t + 2.0 * b) * t + c
    }
    let x = x.clamp(0.0, 1.0);
    let mut t = x;
    for _ in 0..4 {
        let error = curve(t, x1, x2) - x;
        if error.abs() < 1e-5 {
            return curve(t, y1, y2);
        }
        let slope = gradient(t, x1, x2);
        if slope.abs() < 1e-5 {
            break;
        }
        t = (t - (error / slope).clamp(-0.25, 0.25)).clamp(0.0, 1.0);
    }
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..20 {
        t = (low + high) * 0.5;
        if curve(t, x1, x2) < x {
            low = t;
        } else {
            high = t;
        }
    }
    curve(t, y1, y2)
}
#[cfg(any(feature = "extensions", feature = "shaderpacks"))]
fn next_world_identity() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .expect("world identity exhausted")
}
pub struct ClientLevel {
    #[cfg(any(feature = "extensions", feature = "shaderpacks"))]
    pub(crate) extension_identity: u64,
    pub chunks: HashMap<ChunkPos, Arc<Chunk>>,
    chunk_revision: u64,
    #[cfg(feature = "extensions")]
    block_revision: u64,
    #[cfg(feature = "extensions")]
    block_chunk_revisions: HashMap<ChunkPos, u64>,
    pub dirty_sections: HashSet<SectionPos>,
    light_update_queue: VecDeque<QueuedLightUpdate>,
    chunk_cache_center: Option<(i32, i32)>,
    chunk_storage_radius: Option<i32>,
    chest_neighbor_shape_updates: Vec<(BlockPos, BlockState)>,
    neighbor_prediction_old_states: Vec<(BlockPos, BlockState)>,
    pub block_colors: ClientBlockColors,
    fallback_biome: Biome,
    pub biome_definitions: Arc<HashMap<String, Biome>>,
    biome_blend_radius: i32,
    biome_zoom_seed: i64,
    pub air: BlockState,
    pub block_state_prediction_handler: BlockStatePredictionHandler,
    pub destroying_blocks: HashMap<i32, BlockDestructionProgress>,
    pub destruction_progress: HashMap<i64, BTreeSet<BlockDestructionProgressOrder>>,
    game_time: i64,
    day_time: i64,
    day_time_rate: f32,
    day_time_tick_remainder: f32,
    old_rain_level: f32,
    rain_level: f32,
    old_thunder_level: f32,
    thunder_level: f32,
    pub difficulty: Difficulty,
    pub difficulty_locked: bool,
    world_border_center_x: f64,
    world_border_center_z: f64,
    world_border_size: f64,
    world_border_absolute_max_size: u32,
    world_border_lerp_from: f64,
    world_border_lerp_to: f64,
    world_border_lerp_duration: u64,
    world_border_lerp_remaining: u64,
    world_border_warning_blocks: u32,
    world_border_warning_time: u32,
    world_border_initialized: bool,
}
impl ClientLevel {
    pub fn new(block_colors: ClientBlockColors) -> Self {
        let fallback_biome = Biome::plains();
        let biome_definitions =
            HashMap::from([(fallback_biome.id.clone(), fallback_biome.clone())]);
        Self {
            #[cfg(any(feature = "extensions", feature = "shaderpacks"))]
            extension_identity: next_world_identity(),
            chunks: HashMap::new(),
            chunk_revision: 0,
            #[cfg(feature = "extensions")]
            block_revision: 0,
            #[cfg(feature = "extensions")]
            block_chunk_revisions: HashMap::new(),
            dirty_sections: HashSet::new(),
            light_update_queue: VecDeque::new(),
            chunk_cache_center: None,
            chunk_storage_radius: None,
            chest_neighbor_shape_updates: Vec::new(),
            neighbor_prediction_old_states: Vec::new(),
            block_colors,
            fallback_biome,
            biome_definitions: Arc::new(biome_definitions),
            biome_blend_radius: 2,
            biome_zoom_seed: 0,
            air: BlockState::new(Arc::new(ResourceLocation::new("minecraft", "air"))),
            block_state_prediction_handler: BlockStatePredictionHandler::new(),
            destroying_blocks: HashMap::new(),
            destruction_progress: HashMap::new(),
            game_time: 0,
            day_time: 1000,
            day_time_rate: 1.0,
            day_time_tick_remainder: 0.0,
            old_rain_level: 0.0,
            rain_level: 0.0,
            old_thunder_level: 0.0,
            thunder_level: 0.0,
            difficulty: Difficulty::Normal,
            difficulty_locked: false,
            world_border_center_x: 0.0,
            world_border_center_z: 0.0,
            world_border_size: 5.999_996_8e7,
            world_border_absolute_max_size: 29999984,
            world_border_lerp_from: 5.999_996_8e7,
            world_border_lerp_to: 5.999_996_8e7,
            world_border_lerp_duration: 0,
            world_border_lerp_remaining: 0,
            world_border_warning_blocks: 5,
            world_border_warning_time: 15,
            world_border_initialized: false,
        }
    }
    pub fn set_biome_blend_radius(&mut self, radius: i32) {
        self.biome_blend_radius = radius.clamp(0, 7);
    }
    pub fn biome_blend_radius(&self) -> i32 {
        self.biome_blend_radius
    }
    pub fn set_biome_zoom_seed(&mut self, seed: i64) {
        self.biome_zoom_seed = seed;
    }
    pub fn biome_zoom_seed(&self) -> i64 {
        self.biome_zoom_seed
    }
    pub fn is_within_bounds(&self, pos: BlockPos) -> bool {
        if !self.world_border_initialized {
            return true;
        }
        let absolute_max = self.world_border_absolute_max_size as f64;
        let half = self.world_border_size * 0.5;
        let min_x = (self.world_border_center_x - half).clamp(-absolute_max, absolute_max);
        let max_x = (self.world_border_center_x + half).clamp(-absolute_max, absolute_max);
        let min_z = (self.world_border_center_z - half).clamp(-absolute_max, absolute_max);
        let max_z = (self.world_border_center_z + half).clamp(-absolute_max, absolute_max);
        (((pos.x + 1) as f64) > min_x)
            && (((pos.x) as f64) < max_x)
            && (((pos.z + 1) as f64) > min_z)
            && (((pos.z) as f64) < max_z)
    }
    pub fn set_world_border(
        &mut self,
        center_x: f64,
        center_z: f64,
        size: f64,
        absolute_max_size: u32,
    ) {
        if center_x.is_finite() {
            self.world_border_center_x = center_x;
        }
        if center_z.is_finite() {
            self.world_border_center_z = center_z;
        }
        if size.is_finite() && size > 0.0 {
            self.world_border_size = size;
        }
        if absolute_max_size > 0 {
            self.world_border_absolute_max_size = absolute_max_size;
        }
        self.world_border_initialized = true;
    }
    pub fn initialize_world_border(
        &mut self,
        center_x: f64,
        center_z: f64,
        old_size: f64,
        new_size: f64,
        lerp_time: u64,
        absolute_max_size: u32,
        warning_blocks: u32,
        warning_time: u32,
    ) {
        self.set_world_border_center(center_x, center_z);
        if lerp_time > 0 {
            self.lerp_world_border_size(old_size, new_size, lerp_time);
        } else {
            self.set_world_border_size(new_size);
        }
        if absolute_max_size > 0 {
            self.world_border_absolute_max_size = absolute_max_size;
        }
        self.set_world_border_warning_blocks(warning_blocks);
        self.set_world_border_warning_time(warning_time);
        self.world_border_initialized = true;
    }
    pub fn set_world_border_center(&mut self, center_x: f64, center_z: f64) {
        if center_x.is_finite() {
            self.world_border_center_x = center_x;
        }
        if center_z.is_finite() {
            self.world_border_center_z = center_z;
        }
        self.world_border_initialized = true;
    }
    pub fn set_world_border_size(&mut self, size: f64) {
        if size.is_finite() && size > 0.0 {
            self.world_border_size = size;
            self.world_border_lerp_from = size;
            self.world_border_lerp_to = size;
            self.world_border_lerp_duration = 0;
            self.world_border_lerp_remaining = 0;
        }
        self.world_border_initialized = true;
    }
    pub fn lerp_world_border_size(&mut self, old_size: f64, new_size: f64, lerp_time: u64) {
        if !(old_size.is_finite() && old_size > 0.0 && new_size.is_finite() && new_size > 0.0) {
            return;
        }
        if old_size == new_size || lerp_time == 0 {
            self.set_world_border_size(new_size);
            return;
        }
        self.world_border_size = old_size;
        self.world_border_lerp_from = old_size;
        self.world_border_lerp_to = new_size;
        self.world_border_lerp_duration = lerp_time;
        self.world_border_lerp_remaining = lerp_time;
        self.world_border_initialized = true;
    }
    pub fn set_world_border_warning_blocks(&mut self, warning_blocks: u32) {
        self.world_border_warning_blocks = warning_blocks;
        self.world_border_initialized = true;
    }
    pub fn set_world_border_warning_time(&mut self, warning_time: u32) {
        self.world_border_warning_time = warning_time;
        self.world_border_initialized = true;
    }
    fn tick_world_border(&mut self) {
        if self.world_border_lerp_remaining == 0 || self.world_border_lerp_duration == 0 {
            return;
        }
        self.world_border_lerp_remaining = self.world_border_lerp_remaining.saturating_sub(1);
        let progress = (self.world_border_lerp_duration - self.world_border_lerp_remaining) as f64
            / self.world_border_lerp_duration as f64;
        if progress < 1.0 {
            self.world_border_size = self.world_border_lerp_from
                + (self.world_border_lerp_to - self.world_border_lerp_from) * progress;
        } else {
            self.world_border_size = self.world_border_lerp_to;
        }
        if self.world_border_lerp_remaining == 0 {
            self.world_border_size = self.world_border_lerp_to;
            self.world_border_lerp_from = self.world_border_lerp_to;
            self.world_border_lerp_duration = 0;
        }
    }
    pub fn world_border_collision_shape(
        &self,
        source_bb: &AABB,
        test_bb: &AABB,
    ) -> Option<VoxelShape> {
        if !self.world_border_initialized {
            return None;
        }
        let absolute_max = self.world_border_absolute_max_size as f64;
        let half = self.world_border_size * 0.5;
        let min_x = (self.world_border_center_x - half).clamp(-absolute_max, absolute_max);
        let max_x = (self.world_border_center_x + half).clamp(-absolute_max, absolute_max);
        let min_z = (self.world_border_center_z - half).clamp(-absolute_max, absolute_max);
        let max_z = (self.world_border_center_z + half).clamp(-absolute_max, absolute_max);
        let bb_size = (test_bb.max_x - test_bb.min_x)
            .abs()
            .max((test_bb.max_z - test_bb.min_z).abs())
            .max(1.0);
        let source_x = (source_bb.min_x + source_bb.max_x) * 0.5;
        let source_z = (source_bb.min_z + source_bb.max_z) * 0.5;
        let dist_to_border = (source_x - min_x)
            .min(max_x - source_x)
            .min(source_z - min_z)
            .min(max_z - source_z);
        if dist_to_border >= bb_size * 2.0 {
            return None;
        }
        if source_x < min_x - bb_size
            || source_x >= max_x + bb_size
            || source_z < min_z - bb_size
            || source_z >= max_z + bb_size
        {
            return None;
        }
        let min_x = min_x.floor();
        let min_z = min_z.floor();
        let max_x = max_x.ceil();
        let max_z = max_z.ceil();
        let neg_inf = -absolute_max;
        let pos_inf = absolute_max;
        let aabbs = vec![
            AABB::new(neg_inf, neg_inf, neg_inf, min_x, pos_inf, pos_inf),
            AABB::new(max_x, neg_inf, neg_inf, pos_inf, pos_inf, pos_inf),
            AABB::new(min_x, neg_inf, neg_inf, max_x, pos_inf, min_z),
            AABB::new(min_x, neg_inf, max_z, max_x, pos_inf, pos_inf),
        ];
        Some(VoxelShape::from_aabbs(aabbs))
    }
    pub fn world_border_center_for_debug(&self) -> (f64, f64) {
        (self.world_border_center_x, self.world_border_center_z)
    }
    pub fn world_border_size_for_debug(&self) -> f64 {
        self.world_border_size
    }
    pub fn world_border_initialized_for_debug(&self) -> bool {
        self.world_border_initialized
    }
    pub fn world_border_lerp_from_for_debug(&self) -> f64 {
        self.world_border_lerp_from
    }
    pub fn world_border_lerp_to_for_debug(&self) -> f64 {
        self.world_border_lerp_to
    }
    pub fn world_border_lerp_remaining_for_debug(&self) -> u64 {
        self.world_border_lerp_remaining
    }
    pub fn tick_time(&mut self) {
        self.game_time = self.game_time.saturating_add(1);
        self.tick_world_border();
        self.remove_block_breaking_progress();
        self.tick_day_time_clock(1);
        self.old_rain_level = self.rain_level;
        self.old_thunder_level = self.thunder_level;
    }
    pub fn destroy_block_progress(&mut self, id: i32, pos: [i32; 3], progress: i32) {
        if (0..10).contains(&progress) {
            let previous = self.destroying_blocks.remove(&id);
            if let Some(previous) = previous.as_ref() {
                self.remove_progress_entry(previous);
            }
            let mut entry = match previous {
                Some(entry) if entry.pos == pos => entry,
                _ => BlockDestructionProgress::new(id, pos),
            };
            entry.set_progress(progress);
            entry.update_tick(self.game_time);
            let key = entry.pos_key();
            self.destruction_progress
                .entry(key)
                .or_default()
                .insert(entry.order_key());
            self.destroying_blocks.insert(id, entry);
        } else if let Some(removed) = self.destroying_blocks.remove(&id) {
            self.remove_progress_entry(&removed);
        }
    }
    fn remove_progress_entry(&mut self, entry: &BlockDestructionProgress) {
        let key = entry.pos_key();
        let remove_position = self
            .destruction_progress
            .get_mut(&key)
            .map(|set| {
                set.remove(&entry.order_key());
                set.is_empty()
            })
            .unwrap_or(false);
        if remove_position {
            self.destruction_progress.remove(&key);
        }
    }
    pub fn strongest_block_destruction_progress_at(
        &self,
        pos: BlockPos,
    ) -> Option<&BlockDestructionProgress> {
        let key = BlockDestructionProgress::new(0, [pos.x, pos.y, pos.z]).pos_key();
        let order = self.destruction_progress.get(&key)?.last()?;
        self.destroying_blocks
            .get(&order.id)
            .filter(|entry| entry.pos_key() == key && entry.order_key() == *order)
    }
    fn remove_block_breaking_progress(&mut self) {
        if self.game_time % 20 != 0 {
            return;
        }
        let game_time = self.game_time;
        let stale: Vec<i32> = self
            .destroying_blocks
            .iter()
            .filter(|(_, progress)| game_time - progress.updated_render_tick > 400)
            .map(|(id, _)| *id)
            .collect();
        for id in stale {
            if let Some(removed) = self.destroying_blocks.remove(&id) {
                self.remove_progress_entry(&removed);
            }
        }
    }
    pub fn game_time(&self) -> i64 {
        self.game_time
    }
    pub fn day_time(&self) -> i64 {
        self.day_time
    }
    pub fn set_day_time(&mut self, day_time: i64) {
        self.day_time = day_time;
        self.day_time_tick_remainder = 0.0;
    }
    pub fn set_server_time(
        &mut self,
        game_time: i64,
        day_time: Option<i64>,
        day_time_rate: Option<f32>,
        day_time_partial_tick: Option<f32>,
    ) {
        let game_time_delta = game_time.saturating_sub(self.game_time);
        self.tick_day_time_clock(game_time_delta);
        self.game_time = game_time;
        if let Some(day_time) = day_time {
            self.day_time = day_time;
        }
        if let Some(day_time_rate) = day_time_rate {
            self.day_time_rate = if day_time_rate.is_finite() {
                day_time_rate
            } else {
                1.0
            };
        }
        if let Some(day_time_partial_tick) = day_time_partial_tick {
            self.day_time_tick_remainder = day_time_partial_tick.fract();
        }
    }
    pub fn set_difficulty(&mut self, difficulty: Difficulty) {
        self.difficulty = difficulty;
    }
    pub fn set_difficulty_locked(&mut self, locked: bool) {
        self.difficulty_locked = locked;
    }
    fn tick_day_time_clock(&mut self, game_time_delta: i64) {
        if game_time_delta == 0
            || self.day_time < 0
            || !self.day_time_rate.is_finite()
            || self.day_time_rate == 0.0
        {
            return;
        }
        let new_partial_ticks = self.day_time_tick_remainder as f64
            + game_time_delta as f64 * self.day_time_rate as f64;
        let full_ticks = new_partial_ticks.floor() as i64;
        self.day_time_tick_remainder = (new_partial_ticks - full_ticks as f64) as f32;
        if full_ticks != 0 {
            self.day_time = self.day_time.saturating_add(full_ticks);
        }
    }
    pub fn add_day_time(&mut self, amount: i64) {
        self.day_time += amount;
    }
    pub fn set_rain_level(&mut self, rain_level: f32) {
        let rain_level = rain_level.clamp(0.0, 1.0);
        self.old_rain_level = rain_level;
        self.rain_level = rain_level;
    }
    pub fn rain_level(&self, partial_tick: f32) -> f32 {
        self.old_rain_level + (self.rain_level - self.old_rain_level) * partial_tick.clamp(0.0, 1.0)
    }
    pub fn set_thunder_level(&mut self, thunder_level: f32) {
        let thunder_level = thunder_level.clamp(0.0, 1.0);
        self.old_thunder_level = thunder_level;
        self.thunder_level = thunder_level;
    }
    pub fn thunder_level(&self, partial_tick: f32) -> f32 {
        (self.old_thunder_level
            + (self.thunder_level - self.old_thunder_level) * partial_tick.clamp(0.0, 1.0))
            * self.rain_level(partial_tick)
    }
    pub fn set_weather(&mut self, mode: WeatherMode) {
        match mode {
            WeatherMode::Clear => {
                self.set_rain_level(0.0);
                self.set_thunder_level(0.0);
            }
            WeatherMode::Rain => {
                self.set_rain_level(1.0);
                self.set_thunder_level(0.0);
            }
            WeatherMode::Thunder => {
                self.set_rain_level(1.0);
                self.set_thunder_level(1.0);
            }
        }
    }
    pub fn is_raining(&self) -> bool {
        self.rain_level(1.0) > 0.2
    }
    pub fn celestial_angle(&self, partial_tick: f32) -> f32 {
        let ticks = self.day_time as f32 + partial_tick.clamp(0.0, 1.0);
        let alpha = (ticks - 6000.0).rem_euclid(24000.0) / 24000.0;
        cubic_bezier(alpha, 0.362, 0.241, 1.0 - 0.362, 1.0 - 0.241)
    }
    pub fn cloud_color(&self, partial_tick: f32) -> [f32; 3] {
        argb_to_rgb(self.cloud_color_argb(partial_tick))
    }
    pub fn cloud_color_rgba(&self, partial_tick: f32) -> [f32; 4] {
        argb_to_rgba(self.cloud_color_argb(partial_tick))
    }
    pub fn cloud_color_argb(&self, partial_tick: f32) -> u32 {
        let ticks = self.day_time as f32 + partial_tick.clamp(0.0, 1.0);
        let mut color = multiply_argb(
            OVERWORLD_BASE_CLOUD_COLOR,
            sample_argb_keyframes(ticks, CLOUD_COLOR_MULTIPLIERS),
        );
        let (rain, thunder) = self.weather_layers(partial_tick);
        if rain > 0.0 {
            color = srgb_lerp(rain, color, blend_to_gray(color, 0.24, 0.5));
        }
        if thunder > 0.0 {
            color = srgb_lerp(thunder, color, blend_to_gray(color, 0.095, 0.94));
        }
        color
    }
    pub fn sky_color(&self, partial_tick: f32) -> [f32; 3] {
        argb_to_rgb(self.sky_color_argb(partial_tick))
    }
    pub fn sky_color_argb(&self, partial_tick: f32) -> u32 {
        let ticks = self.day_time as f32 + partial_tick.clamp(0.0, 1.0);
        let mut color = multiply_argb(
            overworld_base_sky_color(),
            sample_argb_keyframes(ticks, SKY_COLOR_MULTIPLIERS),
        );
        let (rain, thunder) = self.weather_layers(partial_tick);
        if rain > 0.0 {
            color = srgb_lerp(rain, color, blend_to_gray(color, 0.6, 0.75));
        }
        if thunder > 0.0 {
            color = srgb_lerp(thunder, color, blend_to_gray(color, 0.24, 0.94));
        }
        color
    }
    pub fn sky_light_color(&self, partial_tick: f32) -> [f32; 3] {
        argb_to_rgb(self.sky_light_color_argb(partial_tick))
    }
    pub fn sky_light_color_argb(&self, partial_tick: f32) -> u32 {
        let ticks = self.day_time as f32 + partial_tick.clamp(0.0, 1.0);
        let mut color = sample_argb_keyframes(ticks, SKY_LIGHT_COLOR_MULTIPLIERS);
        let (rain, thunder) = self.weather_layers(partial_tick);
        if rain > 0.0 {
            let rain_value = alpha_blend_argb(color, RAIN_SKY_LIGHT_COLOR);
            color = srgb_lerp(rain, color, rain_value);
        }
        if thunder > 0.0 {
            let thunder_value = alpha_blend_argb(color, THUNDER_SKY_LIGHT_COLOR);
            color = srgb_lerp(thunder, color, thunder_value);
        }
        color
    }
    pub fn fog_color(&self, partial_tick: f32) -> [f32; 3] {
        argb_to_rgb(self.fog_color_argb(partial_tick))
    }
    pub fn fog_color_argb(&self, partial_tick: f32) -> u32 {
        let ticks = self.day_time as f32 + partial_tick.clamp(0.0, 1.0);
        let mut color = multiply_argb(
            OVERWORLD_BASE_FOG_COLOR,
            sample_argb_keyframes(ticks, FOG_COLOR_MULTIPLIERS),
        );
        let (rain, thunder) = self.weather_layers(partial_tick);
        if rain > 0.0 {
            color = srgb_lerp(rain, color, multiply_argb(color, RAIN_FOG_MULTIPLIER));
        }
        if thunder > 0.0 {
            color = srgb_lerp(thunder, color, multiply_argb(color, THUNDER_FOG_MULTIPLIER));
        }
        color
    }
    pub fn sky_light_factor(&self, partial_tick: f32) -> f32 {
        let ticks = self.day_time as f32 + partial_tick.clamp(0.0, 1.0);
        let mut factor = sample_float_keyframes(ticks, SKY_LIGHT_FACTOR_MULTIPLIERS);
        let (rain, thunder) = self.weather_layers(partial_tick);
        if rain > 0.0 {
            let rain_factor = factor + 0.3125 * (0.24 - factor);
            factor += rain * (rain_factor - factor);
        }
        if thunder > 0.0 {
            let thunder_factor = factor + 0.527_343_75 * (0.24 - factor);
            factor += thunder * (thunder_factor - factor);
        }
        factor.clamp(0.0, 1.0)
    }
    pub fn moon_phase_index(&self) -> u32 {
        ((self.day_time.div_euclid(24000)).rem_euclid(8)) as u32
    }
    pub fn cloud_height(&self) -> f32 {
        OVERWORLD_CLOUD_HEIGHT
    }
    pub fn cloud_fog_end_distance(&self) -> f32 {
        OVERWORLD_CLOUD_FOG_END_DISTANCE
    }
    pub fn horizon_height(&self) -> f64 {
        OVERWORLD_HORIZON_HEIGHT
    }
    pub fn should_render_dark_disc(&self, eye_y: f32, camera_underwater: bool) -> bool {
        eye_y as f64 - self.horizon_height() < 0.0 && !camera_underwater
    }
    pub(crate) fn weather_layers(&self, partial_tick: f32) -> (f32, f32) {
        let thunder = self.thunder_level(partial_tick).clamp(0.0, 1.0);
        let rain = (self.rain_level(partial_tick) - thunder).clamp(0.0, 1.0);
        (rain, thunder)
    }
    pub fn load_chunk(&mut self, chunk: Chunk) -> bool {
        let pos = ChunkPos::new(chunk.x, chunk.z);
        if !self.is_in_storage_range(pos.x, pos.z) {
            log::warn!(
                "Ignoring chunk since it's not in the view range: {}, {}",
                pos.x,
                pos.z
            );
            return false;
        }
        for section in &chunk.sections {
            self.dirty_sections
                .insert(SectionPos::new(pos.x, section.y_index, pos.z));
        }
        self.chunks.insert(pos, Arc::new(chunk));
        #[cfg(feature = "extensions")]
        self.mark_block_revision(pos);
        self.chunk_revision = self.chunk_revision.wrapping_add(1);
        true
    }
    pub fn unload_chunk(&mut self, cx: i32, cz: i32) {
        let removed = self.chunks.remove(&ChunkPos::new(cx, cz)).is_some();
        #[cfg(feature = "extensions")]
        self.block_chunk_revisions.remove(&ChunkPos::new(cx, cz));
        if removed {
            self.chunk_revision = self.chunk_revision.wrapping_add(1);
        }
        self.dirty_sections
            .retain(|section| section.cx != cx || section.cz != cz);
    }
    pub fn chunk_revision(&self) -> u64 {
        self.chunk_revision
    }
    #[cfg(feature = "extensions")]
    pub(crate) fn block_chunk_revision(&self, cx: i32, cz: i32) -> Option<u64> {
        self.block_chunk_revisions
            .get(&ChunkPos::new(cx, cz))
            .copied()
    }
    #[cfg(feature = "extensions")]
    fn mark_block_revision(&mut self, pos: ChunkPos) {
        self.block_revision = self
            .block_revision
            .checked_add(1)
            .expect("block revision exhausted");
        self.block_chunk_revisions.insert(pos, self.block_revision);
    }
    pub fn set_chunk_cache_center(&mut self, x: i32, z: i32) {
        self.chunk_cache_center = Some((x, z));
    }
    pub fn update_view_radius(&mut self, radius: i32) -> Vec<ChunkPos> {
        self.chunk_storage_radius = Some(radius.max(2) + 3);
        self.evict_outside_view()
    }
    pub fn is_in_storage_range(&self, x: i32, z: i32) -> bool {
        match (self.chunk_cache_center, self.chunk_storage_radius) {
            (Some((cx, cz)), Some(radius)) => (x - cx).abs() <= radius && (z - cz).abs() <= radius,
            _ => true,
        }
    }
    pub fn evict_outside_view(&mut self) -> Vec<ChunkPos> {
        let outside: Vec<ChunkPos> = self
            .chunks
            .keys()
            .copied()
            .filter(|pos| !self.is_in_storage_range(pos.x, pos.z))
            .collect();
        for pos in &outside {
            self.unload_chunk(pos.x, pos.z);
        }
        outside
    }
    pub fn queue_light_update(&mut self, update: QueuedLightUpdate) {
        self.light_update_queue.push_back(update);
    }
    pub fn pending_light_update_count(&self) -> usize {
        self.light_update_queue.len()
    }
    pub fn poll_light_updates(&mut self) {
        let size = self.light_update_queue.len();
        let budget = if size < 1000 {
            (size / 10).max(10)
        } else {
            size
        };
        for _ in 0..budget {
            let Some(update) = self.light_update_queue.pop_front() else {
                break;
            };
            self.apply_queued_light_update(update);
        }
    }
    fn apply_queued_light_update(&mut self, update: QueuedLightUpdate) {
        let Some(chunk) = self.chunks.get_mut(&update.chunk) else {
            return;
        };
        let chunk = Arc::make_mut(chunk);
        let min_y_section = chunk.min_y_section;
        let section_count = chunk.sections.len() as i32;
        let mut dirty = Vec::new();
        for (index, layer) in update.sky {
            chunk.set_light_layer(true, index, layer);
            dirty.push(index);
        }
        for (index, layer) in update.block {
            chunk.set_light_layer(false, index, layer);
            dirty.push(index);
        }
        if update.mark_correct {
            chunk.is_light_correct = true;
        }
        for index in dirty {
            let section_y = min_y_section + index as i32 - 1;
            if (min_y_section..min_y_section + section_count).contains(&section_y) {
                self.dirty_sections.insert(SectionPos::new(
                    update.chunk.x,
                    section_y,
                    update.chunk.z,
                ));
            }
        }
    }
    pub fn chunk(&self, cx: i32, cz: i32) -> Option<&Chunk> {
        self.chunks
            .get(&ChunkPos::new(cx, cz))
            .map(|chunk| &**chunk)
    }
    pub fn has_chunk_at(&self, pos: BlockPos) -> bool {
        self.chunks
            .contains_key(&ChunkPos::new(pos.x >> 4, pos.z >> 4))
    }
    pub fn loaded_chunk_count(&self) -> usize {
        self.chunks.len()
    }
    pub fn get_block_state(&self, pos: BlockPos) -> BlockState {
        let cx = pos.x >> 4;
        let cz = pos.z >> 4;
        match self.chunks.get(&ChunkPos::new(cx, cz)) {
            Some(chunk) => chunk
                .get_block_state(&pos)
                .unwrap_or_else(|| self.air.clone()),
            None => self.air.clone(),
        }
    }
    #[inline]
    pub fn get_block_state_ref(&self, pos: BlockPos) -> Option<&BlockState> {
        let cx = pos.x >> 4;
        let cz = pos.z >> 4;
        self.chunks.get(&ChunkPos::new(cx, cz)).and_then(|chunk| {
            let section = (pos.y >> 4) - chunk.min_y_section;
            chunk
                .sections
                .get(section as usize)
                .map(|section| section.get_block_state_ref(pos.x, pos.y, pos.z))
        })
    }
    pub fn fluid_height(&self, pos: BlockPos) -> f32 {
        let fluid = self.get_block_state(pos).get_fluid_state();
        let above = self.get_block_state(pos.above()).get_fluid_state();
        fluid.height_with_same_fluid_above(above)
    }
    pub fn has_fluid_and_loaded(
        &self,
        x0: i32,
        y0: i32,
        z0: i32,
        x1: i32,
        y1: i32,
        z1: i32,
    ) -> bool {
        let section_x0 = x0 >> 4;
        let section_y0 = y0 >> 4;
        let section_z0 = z0 >> 4;
        let section_x1 = x1 >> 4;
        let section_y1 = y1 >> 4;
        let section_z1 = z1 >> 4;
        let mut has_fluid = false;
        for chunk_z in section_z0..=section_z1 {
            for chunk_x in section_x0..=section_x1 {
                let Some(chunk) = self.chunk(chunk_x, chunk_z) else {
                    return false;
                };
                for section_y in section_y0..=section_y1 {
                    let section_index = section_y - chunk.min_y_section;
                    if section_index >= 0
                        && chunk
                            .sections
                            .get(section_index as usize)
                            .is_some_and(el_world::chunk::ChunkSection::has_fluid)
                    {
                        has_fluid = true;
                    }
                }
            }
        }
        has_fluid
    }
    pub fn get_block_collisions(&self, aabb: &AABB) -> Vec<VoxelShape> {
        self.get_block_collisions_with_context(aabb, &CollisionContext::empty())
    }
    pub fn get_block_collisions_with_context(
        &self,
        aabb: &AABB,
        context: &CollisionContext,
    ) -> Vec<VoxelShape> {
        self.get_block_collisions_with_positions_context(aabb, context)
            .into_iter()
            .map(|(_, shape)| shape)
            .collect()
    }
    pub fn get_block_and_liquid_collisions(&self, aabb: &AABB) -> Vec<VoxelShape> {
        self.get_block_collisions_with_context(
            aabb,
            &CollisionContext::empty_with_fluid_collisions(),
        )
    }
    pub fn get_block_collisions_with_context_and_positions(
        &self,
        aabb: &AABB,
        context: &CollisionContext,
    ) -> Vec<(BlockPos, VoxelShape)> {
        self.get_block_collisions_with_positions_context(aabb, context)
    }
    pub fn no_block_collision_with_context(&self, aabb: &AABB, context: &CollisionContext) -> bool {
        self.get_block_collisions_with_positions_context(aabb, context)
            .is_empty()
    }
    pub fn no_block_collision(&self, aabb: &AABB) -> bool {
        self.no_block_collision_with_context(aabb, &CollisionContext::empty())
    }
    pub fn get_entity_collisions_from_shapes(
        &self,
        test_area: &AABB,
        entity_colliders: &[VoxelShape],
    ) -> Vec<VoxelShape> {
        if test_area.get_size() < 1.0e-7 {
            return Vec::new();
        }
        let entity_query_aabb = test_area.inflate_all(el_math::aabb::EPSILON);
        entity_colliders
            .iter()
            .filter(|shape| shape.bounds().intersects(&entity_query_aabb))
            .cloned()
            .collect()
    }
    pub fn get_collisions_with_context_and_entities(
        &self,
        source_aabb: Option<&AABB>,
        aabb: &AABB,
        context: &CollisionContext,
        entity_colliders: &[VoxelShape],
    ) -> Vec<VoxelShape> {
        let selected_entity_colliders =
            self.get_entity_collisions_from_shapes(aabb, entity_colliders);
        self.get_collisions_with_context_and_preselected_entities(
            source_aabb,
            aabb,
            context,
            &selected_entity_colliders,
        )
    }
    pub fn get_collisions_with_context_and_preselected_entities(
        &self,
        source_aabb: Option<&AABB>,
        aabb: &AABB,
        context: &CollisionContext,
        selected_entity_colliders: &[VoxelShape],
    ) -> Vec<VoxelShape> {
        let mut collisions = selected_entity_colliders.to_vec();
        if let Some(source_aabb) = source_aabb {
            if let Some(border_shape) = self.world_border_collision_shape(source_aabb, aabb) {
                collisions.push(border_shape);
            }
        }
        collisions.extend(self.get_block_collisions_with_context(aabb, context));
        collisions
    }
    pub fn no_collision_with_context_and_entities(
        &self,
        source_aabb: Option<&AABB>,
        aabb: &AABB,
        context: &CollisionContext,
        entity_colliders: &[VoxelShape],
    ) -> bool {
        if !self
            .get_entity_collisions_from_shapes(aabb, entity_colliders)
            .is_empty()
        {
            return false;
        }
        if let Some(source_aabb) = source_aabb {
            if let Some(border_shape) = self.world_border_collision_shape(source_aabb, aabb) {
                if voxel_shape_intersects_aabb(&border_shape, aabb) {
                    return false;
                }
            }
        }
        self.no_block_collision_with_context(aabb, context)
    }
    pub fn contains_any_liquid(&self, aabb: &AABB) -> bool {
        let min_x = aabb.min_x.floor() as i32;
        let max_x = aabb.max_x.ceil() as i32;
        let min_y = aabb.min_y.floor() as i32;
        let max_y = aabb.max_y.ceil() as i32;
        let min_z = aabb.min_z.floor() as i32;
        let max_z = aabb.max_z.ceil() as i32;
        for x in min_x..max_x {
            for y in min_y..max_y {
                for z in min_z..max_z {
                    let state = self.get_block_state(BlockPos::new(x, y, z));
                    if !state.get_fluid_state().is_empty() {
                        return true;
                    }
                }
            }
        }
        false
    }
    pub fn find_supporting_block_with_context(
        &self,
        source_pos: McVec3,
        aabb: &AABB,
        context: &CollisionContext,
    ) -> Option<BlockPos> {
        let mut main_support = None;
        let mut main_support_distance = f64::MAX;
        for (pos, _) in self.get_block_collisions_with_positions_context(aabb, context) {
            let center = McVec3::new(pos.x as f64 + 0.5, pos.y as f64 + 0.5, pos.z as f64 + 0.5);
            let distance = center.distance_to_sq(&source_pos);
            let replace = distance < main_support_distance
                || (distance == main_support_distance
                    && main_support
                        .map(|current| block_pos_compare(current, pos).is_lt())
                        .unwrap_or(true));
            if replace {
                main_support = Some(pos);
                main_support_distance = distance;
            }
        }
        main_support
    }
    pub fn find_supporting_block(&self, source_pos: McVec3, aabb: &AABB) -> Option<BlockPos> {
        self.find_supporting_block_with_context(source_pos, aabb, &CollisionContext::empty())
    }
    fn get_block_collisions_with_positions_context(
        &self,
        aabb: &AABB,
        context: &CollisionContext,
    ) -> Vec<(BlockPos, VoxelShape)> {
        let (min_x, max_x, min_y, max_y, min_z, max_z) = block_collisions_cursor_bounds(aabb);
        let mut block_shapes = Vec::new();
        for z in min_z..=max_z {
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let block_pos = BlockPos::new(x, y, z);
                    let state = self.get_block_state(block_pos);
                    if !block_collision_candidate_passes_cursor_filter(
                        &state, min_x, max_x, min_y, max_y, min_z, max_z, x, y, z,
                    ) {
                        continue;
                    }
                    let block_shape = state.get_collision_shape_with_context(block_pos, context);
                    if block_shape.is_empty() {
                        continue;
                    }
                    let moved_shape = block_shape.move_by(x as f64, y as f64, z as f64);
                    if moved_shape
                        .to_aabbs()
                        .iter()
                        .any(|shape_aabb| shape_aabb.intersects(aabb))
                    {
                        block_shapes.push((block_pos, moved_shape));
                    }
                }
            }
        }
        block_shapes
    }
    pub fn collides_with_suffocating_block(&self, aabb: &AABB, context: &CollisionContext) -> bool {
        let (min_x, max_x, min_y, max_y, min_z, max_z) = block_collisions_cursor_bounds(aabb);
        for z in min_z..=max_z {
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let block_pos = BlockPos::new(x, y, z);
                    let state = self.get_block_state(block_pos);
                    if !block_collision_candidate_passes_cursor_filter(
                        &state, min_x, max_x, min_y, max_y, min_z, max_z, x, y, z,
                    ) || !java_state_can_suffocate(&state)
                    {
                        continue;
                    }
                    let block_shape = state.get_collision_shape_with_context(block_pos, context);
                    if !is_full_block_shape(&block_shape) {
                        continue;
                    }
                    if block_shape
                        .move_by(x as f64, y as f64, z as f64)
                        .to_aabbs()
                        .iter()
                        .any(|shape_aabb| shape_aabb.intersects(aabb))
                    {
                        return true;
                    }
                }
            }
        }
        false
    }
    pub fn brightness(&self, sky: bool, pos: BlockPos) -> u8 {
        self.chunks
            .get(&ChunkPos::from_block(pos))
            .map(|chunk| chunk.brightness(sky, &pos))
            .unwrap_or(if sky { 15 } else { 0 })
    }
    pub(crate) fn motion_blocking_height(&self, x: i32, z: i32) -> Option<i32> {
        self.chunk(x >> 4, z >> 4).and_then(|chunk| {
            chunk.height(
                el_world::chunk::HeightmapType::MotionBlocking,
                x & 15,
                z & 15,
            )
        })
    }
    pub(crate) fn precipitation_at(
        &self,
        x: i32,
        y: i32,
        z: i32,
        sea_level: i32,
    ) -> crate::render::weather_renderer::WeatherPrecipitation {
        use crate::render::weather_renderer::WeatherPrecipitation;
        let pos = BlockPos::new(x, y, z);
        if !self.has_chunk_at(pos) {
            return WeatherPrecipitation::None;
        }
        let biome = self.get_biome(pos);
        if !biome.has_precipitation {
            return WeatherPrecipitation::None;
        }
        let mut temperature = biome.temperature;
        if biome.temperature_modifier == "frozen" {
            let ice_patches =
                el_math::noise::biome_frozen_temperature_noise(f64::from(x), f64::from(z)) * 7.0
                    + el_math::noise::biome_info_noise(f64::from(x) * 0.2, f64::from(z) * 0.2);
            if ice_patches < 0.3
                && el_math::noise::biome_info_noise(f64::from(x) * 0.09, f64::from(z) * 0.09) < 0.8
            {
                temperature = 0.2;
            }
        }
        let snow_level = sea_level + 17;
        if y > snow_level {
            temperature -= (el_math::noise::biome_temperature_noise(f64::from(x), f64::from(z))
                * 8.0
                + f64::from(y - snow_level)) as f32
                * 0.05
                / 40.0;
        }
        if temperature < 0.15 {
            WeatherPrecipitation::Snow
        } else {
            WeatherPrecipitation::Rain
        }
    }
    pub(crate) fn weather_light_levels_at(&self, x: i32, y: i32, z: i32) -> [u8; 2] {
        let pos = BlockPos::new(x, y, z);
        [self.brightness(false, pos), self.brightness(true, pos)]
    }
    pub fn install_biomes(&mut self, biomes: impl IntoIterator<Item = Biome>) {
        let definitions = Arc::make_mut(&mut self.biome_definitions);
        for biome in biomes {
            definitions.insert(biome.id.clone(), biome);
        }
    }
    pub fn set_block(&mut self, pos: BlockPos, state: BlockState) -> bool {
        self.set_block_with_known_shape(pos, state, false)
    }
    pub fn set_block_server_verified(&mut self, pos: BlockPos, state: BlockState) -> bool {
        self.set_block_with_known_shape(pos, state, true)
    }
    fn set_block_with_known_shape(
        &mut self,
        pos: BlockPos,
        state: BlockState,
        known_shape: bool,
    ) -> bool {
        self.chest_neighbor_shape_updates.clear();
        self.neighbor_prediction_old_states.clear();
        let key = ChunkPos::from_block(pos);
        let changed = self.set_block_state_without_neighbour_shapes(pos, state.clone());
        if state.is_known_vanilla_air() {
            if let Some(chunk) = self.chunks.get_mut(&key) {
                let chunk = Arc::make_mut(chunk);
                chunk.block_entities.remove(&pos);
            }
        }
        if changed {
            self.mark_block_dirty(pos);
            if !known_shape {
                self.update_chest_neighbour_shapes_after_set_block(pos, &state);
            }
        }
        changed
    }
    pub fn take_chest_neighbor_shape_updates(&mut self) -> Vec<(BlockPos, BlockState)> {
        std::mem::take(&mut self.chest_neighbor_shape_updates)
    }
    pub(crate) fn take_neighbor_prediction_old_states(&mut self) -> Vec<(BlockPos, BlockState)> {
        std::mem::take(&mut self.neighbor_prediction_old_states)
    }
    fn set_block_state_without_neighbour_shapes(
        &mut self,
        pos: BlockPos,
        state: BlockState,
    ) -> bool {
        let key = ChunkPos::from_block(pos);
        let Some(chunk) = self.chunks.get_mut(&key) else {
            return false;
        };
        let chunk = Arc::make_mut(chunk);
        let changed = if chunk.get_block_state(&pos).as_ref() == Some(&state) {
            false
        } else {
            chunk.set_block_state(&pos, state)
        };
        #[cfg(feature = "extensions")]
        if changed {
            self.mark_block_revision(key);
        }
        changed
    }
    fn update_chest_neighbour_shapes_after_set_block(
        &mut self,
        changed_pos: BlockPos,
        changed_state: &BlockState,
    ) {
        let mut pending = VecDeque::from([(changed_pos, changed_state.clone(), 512_u16)]);
        while let Some((changed_pos, changed_state, remaining)) = pending.pop_front() {
            if remaining == 0 {
                continue;
            }
            for direction_from_changed in UPDATE_SHAPE_ORDER {
                let neighbour_pos = changed_pos.relative(direction_from_changed);
                let neighbour_state = self.get_block_state(neighbour_pos);
                let direction_to_changed = direction_from_changed.opposite();
                let Some(updated_state) = chest_state_after_neighbour_update(
                    &neighbour_state,
                    direction_to_changed,
                    &changed_state,
                )
                .or_else(|| {
                    crate::network::multiplayer::multiplayer_game_mode::placement::neighbor_state(
                        self,
                        neighbour_pos,
                        &neighbour_state,
                        direction_to_changed,
                        &changed_state,
                    )
                }) else {
                    continue;
                };
                if updated_state == neighbour_state {
                    continue;
                }
                if self
                    .set_block_state_without_neighbour_shapes(neighbour_pos, updated_state.clone())
                {
                    self.neighbor_prediction_old_states
                        .push((neighbour_pos, neighbour_state));
                    if updated_state.is_known_vanilla_air()
                        && let Some(chunk) =
                            self.chunks.get_mut(&ChunkPos::from_block(neighbour_pos))
                    {
                        Arc::make_mut(chunk).block_entities.remove(&neighbour_pos);
                    }
                    self.mark_block_dirty(neighbour_pos);
                    self.chest_neighbor_shape_updates
                        .push((neighbour_pos, updated_state.clone()));
                    pending.push_back((neighbour_pos, updated_state, remaining - 1));
                }
            }
        }
    }
    pub fn set_air(&mut self, pos: BlockPos) -> bool {
        self.set_block(pos, self.air.clone())
    }
    fn mark_block_dirty(&mut self, pos: BlockPos) {
        for z in (pos.z - 1)..=(pos.z + 1) {
            for x in (pos.x - 1)..=(pos.x + 1) {
                for y in (pos.y - 1)..=(pos.y + 1) {
                    self.dirty_sections
                        .insert(SectionPos::new(x >> 4, y >> 4, z >> 4));
                }
            }
        }
    }
    pub fn take_dirty_sections(&mut self) -> Vec<SectionPos> {
        self.dirty_sections.drain().collect()
    }
}
fn block_pos_compare(left: BlockPos, right: BlockPos) -> std::cmp::Ordering {
    left.y
        .cmp(&right.y)
        .then_with(|| left.z.cmp(&right.z))
        .then_with(|| left.x.cmp(&right.x))
}
fn voxel_shape_intersects_aabb(shape: &VoxelShape, aabb: &AABB) -> bool {
    shape
        .to_aabbs()
        .iter()
        .any(|shape_aabb| shape_aabb.intersects(aabb))
}
fn block_collisions_cursor_bounds(aabb: &AABB) -> (i32, i32, i32, i32, i32, i32) {
    let min_x = ((aabb.min_x - 1.0e-7).floor() as i32) - 1;
    let max_x = ((aabb.max_x + 1.0e-7).floor() as i32) + 1;
    let min_y = ((aabb.min_y - 1.0e-7).floor() as i32) - 1;
    let max_y = ((aabb.max_y + 1.0e-7).floor() as i32) + 1;
    let min_z = ((aabb.min_z - 1.0e-7).floor() as i32) - 1;
    let max_z = ((aabb.max_z + 1.0e-7).floor() as i32) + 1;
    (min_x, max_x, min_y, max_y, min_z, max_z)
}
fn block_collisions_cursor_type(
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    min_z: i32,
    max_z: i32,
    x: i32,
    y: i32,
    z: i32,
) -> i32 {
    let mut cursor_type = 0;
    if x == min_x || x == max_x {
        cursor_type += 1;
    }
    if y == min_y || y == max_y {
        cursor_type += 1;
    }
    if z == min_z || z == max_z {
        cursor_type += 1;
    }
    cursor_type
}
#[allow(clippy::too_many_arguments)]
fn block_collision_candidate_passes_cursor_filter(
    state: &BlockState,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    min_z: i32,
    max_z: i32,
    x: i32,
    y: i32,
    z: i32,
) -> bool {
    match block_collisions_cursor_type(min_x, max_x, min_y, max_y, min_z, max_z, x, y, z) {
        0 => true,
        1 => state.has_large_collision_shape(),
        2 => state.block.path == "moving_piston",
        _ => false,
    }
}
fn is_full_block_shape(shape: &VoxelShape) -> bool {
    let boxes = shape.to_aabbs();
    boxes.len() == 1
        && boxes[0].min_x <= 1.0e-7
        && boxes[0].min_y <= 1.0e-7
        && boxes[0].min_z <= 1.0e-7
        && boxes[0].max_x >= 1.0 - 1.0e-7
        && boxes[0].max_y >= 1.0 - 1.0e-7
        && boxes[0].max_z >= 1.0 - 1.0e-7
}
fn java_state_can_suffocate(state: &BlockState) -> bool {
    if state.block.namespace != "minecraft" {
        return true;
    }
    let name = state.block.path.as_str();
    if java_never_suffocates_vanilla_block(name) {
        return false;
    }
    if matches!(name, "piston" | "sticky_piston") {
        return state
            .get_property("extended")
            .is_none_or(|value| value != "true");
    }
    true
}
fn java_never_suffocates_vanilla_block(name: &str) -> bool {
    name.ends_with("_leaves")
        || name.ends_with("_stained_glass")
        || name.ends_with("_copper_grate")
        || matches!(
            name,
            "glass" | "tinted_glass" | "mangrove_roots" | "moving_piston" | "copper_grate"
        )
}
impl BlockAndTintGetter for ClientLevel {
    fn colormaps(&self) -> &ClientBlockColors {
        &self.block_colors
    }
    fn get_biome(&self, pos: BlockPos) -> &Biome {
        let (quart_x, quart_y, quart_z) = java_biome_manager_quart(self.biome_zoom_seed, pos);
        let selected_pos = BlockPos::new(quart_x << 2, quart_y << 2, quart_z << 2);
        self.chunks
            .get(&ChunkPos::from_block(selected_pos))
            .and_then(|chunk| chunk.get_biome(&selected_pos))
            .and_then(|id| self.biome_definitions.get(&id.to_string()))
            .unwrap_or(&self.fallback_biome)
    }
    fn biome_zoom_seed(&self) -> i64 {
        self.biome_zoom_seed
    }
    fn blend_radius(&self) -> i32 {
        self.biome_blend_radius
    }
}
