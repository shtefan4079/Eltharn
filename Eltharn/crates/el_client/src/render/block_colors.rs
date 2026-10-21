#![allow(dead_code)]
use crate::render::baked_model::BakedQuad;
use crate::resources::manager::{ClientResourceManager, ResourceError};
use el_core::block_state::BlockState;
use el_core::resource_location::ResourceLocation;
use el_math::block_pos::BlockPos;
use el_math::noise::biome_info_noise;
use image::DynamicImage;
use std::collections::HashMap;
pub const WHITE_TINT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const DEFAULT_GRASS_MAP_COLOR: u32 = java_argb(-65_281);
const DEFAULT_FOLIAGE_MAP_COLOR: u32 = java_argb(-12_012_264);
const DEFAULT_DRY_FOLIAGE_MAP_COLOR: u32 = java_argb(-10_732_494);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorMap {
    pixels: Vec<u32>,
}
impl ColorMap {
    pub fn from_image(image: &DynamicImage) -> Self {
        let image = image.to_rgba8();
        let pixels = image
            .pixels()
            .map(|pixel| argb_from_rgba(pixel[0], pixel[1], pixel[2], pixel[3]))
            .collect();
        Self { pixels }
    }
    pub fn len(&self) -> usize {
        self.pixels.len()
    }
    pub fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }
    pub fn from_resource(
        resource_manager: &ClientResourceManager,
        id: &ResourceLocation,
    ) -> Result<Self, ResourceError> {
        let image = resource_manager.read_image(id)?;
        Ok(Self::from_image(&image))
    }
    pub fn sample(&self, temp: f64, mut rain: f64, default_map_color: u32) -> u32 {
        rain *= temp;
        let x = ((1.0 - temp) * 255.0) as usize;
        let y = ((1.0 - rain) * 255.0) as usize;
        let index = y << 8 | x;
        self.pixels.get(index).copied().unwrap_or(default_map_color)
    }
}
#[derive(Clone, Debug)]
pub struct Biome {
    pub id: String,
    pub has_precipitation: bool,
    pub temperature: f32,
    pub temperature_modifier: String,
    pub downfall: f32,
    pub water_color: u32,
    pub foliage_color_override: Option<u32>,
    pub dry_foliage_color_override: Option<u32>,
    pub grass_color_override: Option<u32>,
    pub grass_color_modifier: GrassColorModifier,
}
impl Biome {
    pub fn plains() -> Self {
        Self {
            id: "minecraft:plains".to_string(),
            has_precipitation: true,
            temperature: 0.8,
            temperature_modifier: "none".to_string(),
            downfall: 0.4,
            water_color: 4159204,
            foliage_color_override: None,
            dry_foliage_color_override: None,
            grass_color_override: None,
            grass_color_modifier: GrassColorModifier::None,
        }
    }
    pub fn desert() -> Self {
        Self {
            id: "minecraft:desert".to_string(),
            has_precipitation: false,
            temperature: 2.0,
            temperature_modifier: "none".to_string(),
            downfall: 0.0,
            water_color: 4159204,
            foliage_color_override: None,
            dry_foliage_color_override: None,
            grass_color_override: None,
            grass_color_modifier: GrassColorModifier::None,
        }
    }
    pub fn dark_forest() -> Self {
        Self {
            id: "minecraft:dark_forest".to_string(),
            has_precipitation: true,
            temperature: 0.7,
            temperature_modifier: "none".to_string(),
            downfall: 0.8,
            water_color: 4159204,
            foliage_color_override: None,
            dry_foliage_color_override: None,
            grass_color_override: None,
            grass_color_modifier: GrassColorModifier::DarkForest,
        }
    }
    pub fn swamp() -> Self {
        Self {
            id: "minecraft:swamp".to_string(),
            has_precipitation: true,
            temperature: 0.8,
            temperature_modifier: "none".to_string(),
            downfall: 0.9,
            water_color: 6388580,
            foliage_color_override: Some(6975543),
            dry_foliage_color_override: None,
            grass_color_override: None,
            grass_color_modifier: GrassColorModifier::Swamp,
        }
    }
    pub fn snowy_plains() -> Self {
        Self {
            id: "minecraft:snowy_plains".to_string(),
            has_precipitation: true,
            temperature: 0.0,
            temperature_modifier: "none".to_string(),
            downfall: 0.5,
            water_color: 4159204,
            foliage_color_override: None,
            dry_foliage_color_override: None,
            grass_color_override: None,
            grass_color_modifier: GrassColorModifier::None,
        }
    }
    pub fn default_overworld() -> Self {
        Self {
            id: "minecraft:default_overworld".to_string(),
            has_precipitation: true,
            temperature: 0.5,
            temperature_modifier: "none".to_string(),
            downfall: 1.0,
            water_color: 4159204,
            foliage_color_override: None,
            dry_foliage_color_override: None,
            grass_color_override: None,
            grass_color_modifier: GrassColorModifier::None,
        }
    }
    pub fn get_grass_color(&self, x: i32, z: i32, colormaps: &ClientBlockColors) -> u32 {
        let base_color = self.grass_color_override.unwrap_or_else(|| {
            let temp = self.temperature.clamp(0.0, 1.0) as f64;
            let rain = self.downfall.clamp(0.0, 1.0) as f64;
            colormaps.grass.sample(temp, rain, DEFAULT_GRASS_MAP_COLOR)
        });
        self.grass_color_modifier
            .modify_color(x as f64, z as f64, base_color)
    }
    pub fn get_foliage_color(&self, colormaps: &ClientBlockColors) -> u32 {
        self.foliage_color_override.unwrap_or_else(|| {
            let temp = self.temperature.clamp(0.0, 1.0) as f64;
            let rain = self.downfall.clamp(0.0, 1.0) as f64;
            colormaps
                .foliage
                .sample(temp, rain, DEFAULT_FOLIAGE_MAP_COLOR)
        })
    }
    pub fn get_dry_foliage_color(&self, colormaps: &ClientBlockColors) -> u32 {
        self.dry_foliage_color_override.unwrap_or_else(|| {
            let temp = self.temperature.clamp(0.0, 1.0) as f64;
            let rain = self.downfall.clamp(0.0, 1.0) as f64;
            colormaps
                .dry_foliage
                .sample(temp, rain, DEFAULT_DRY_FOLIAGE_MAP_COLOR)
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrassColorModifier {
    None,
    DarkForest,
    Swamp,
}
impl GrassColorModifier {
    pub fn modify_color(&self, x: f64, z: f64, base_color: u32) -> u32 {
        match self {
            GrassColorModifier::None => base_color,
            GrassColorModifier::DarkForest => {
                (((base_color & 0x00FE_FEFE) + 0x0028_340A) >> 1) | 0xFF00_0000
            }
            GrassColorModifier::Swamp => {
                let value = biome_info_noise(x * 0.0225, z * 0.0225);
                if value < -0.1 {
                    0xFF4C_763C
                } else {
                    0xFF6A_7039
                }
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum BlockTintSource {
    Constant {
        color_in_hand: u32,
        color_in_world: u32,
    },
    DoubleTallGrass,
    Grass,
    GrassBlock,
    SugarCane,
    Foliage,
    DryFoliage,
    Water,
    WaterParticles,
    Redstone,
    Stem,
}
impl BlockTintSource {
    pub fn color(&self, state: &BlockState, colormaps: &ClientBlockColors) -> u32 {
        match self {
            BlockTintSource::Constant { color_in_hand, .. } => *color_in_hand,
            BlockTintSource::DoubleTallGrass => colormaps.default_grass_color(),
            BlockTintSource::Grass => colormaps.default_grass_color(),
            BlockTintSource::GrassBlock => colormaps.default_grass_color(),
            BlockTintSource::SugarCane => java_argb(-1),
            BlockTintSource::Foliage => DEFAULT_FOLIAGE_MAP_COLOR,
            BlockTintSource::DryFoliage => DEFAULT_DRY_FOLIAGE_MAP_COLOR,
            BlockTintSource::Water => java_argb(-1),
            BlockTintSource::WaterParticles => java_argb(-1),
            BlockTintSource::Redstone => {
                let power = state
                    .get_property("power")
                    .and_then(|value| value.parse::<i32>().ok())
                    .unwrap_or(0)
                    .clamp(0, 15);
                redstone_wire_color_raw(power)
            }
            BlockTintSource::Stem => {
                let age = state
                    .get_property("age")
                    .and_then(|value| value.parse::<i32>().ok())
                    .unwrap_or(0)
                    .clamp(0, 7);
                argb_from_rgb((age * 32) as u8, (255 - age * 8) as u8, (age * 4) as u8)
            }
        }
    }
    pub fn color_in_world(
        &self,
        state: &BlockState,
        level: &dyn BlockAndTintGetter,
        pos: BlockPos,
    ) -> u32 {
        match self {
            BlockTintSource::Constant { color_in_world, .. } => *color_in_world,
            BlockTintSource::DoubleTallGrass => {
                let half = state
                    .get_property("half")
                    .map(|s| s.as_str())
                    .unwrap_or("lower");
                let sample_pos = if half == "upper" { pos.below() } else { pos };
                level.get_average_grass_color(sample_pos)
            }
            BlockTintSource::Grass => level.get_average_grass_color(pos),
            BlockTintSource::GrassBlock => level.get_average_grass_color(pos),
            BlockTintSource::SugarCane => level.get_average_grass_color(pos),
            BlockTintSource::Foliage => level.get_average_foliage_color(pos),
            BlockTintSource::DryFoliage => level.get_average_dry_foliage_color(pos),
            BlockTintSource::Water => level.get_average_water_color(pos),
            BlockTintSource::WaterParticles => self.color(state, level.colormaps()),
            _ => self.color(state, level.colormaps()),
        }
    }
    pub fn color_as_terrain_particle(
        &self,
        state: &BlockState,
        level: &dyn BlockAndTintGetter,
        pos: BlockPos,
    ) -> u32 {
        match self {
            BlockTintSource::GrassBlock => java_argb(-1),
            BlockTintSource::WaterParticles => level.get_average_water_color(pos),
            _ => self.color_in_world(state, level, pos),
        }
    }
}
pub trait BlockAndTintGetter {
    fn colormaps(&self) -> &ClientBlockColors;
    fn get_biome(&self, pos: BlockPos) -> &Biome;
    fn biome_zoom_seed(&self) -> i64 {
        0
    }
    fn blend_radius(&self) -> i32 {
        2
    }
}
pub(crate) fn java_biome_manager_quart(seed: i64, pos: BlockPos) -> (i32, i32, i32) {
    let abs_x = pos.x.wrapping_sub(2);
    let abs_y = pos.y.wrapping_sub(2);
    let abs_z = pos.z.wrapping_sub(2);
    let parent_x = abs_x >> 2;
    let parent_y = abs_y >> 2;
    let parent_z = abs_z >> 2;
    let fract_x = f64::from(abs_x & 3) / 4.0;
    let fract_y = f64::from(abs_y & 3) / 4.0;
    let fract_z = f64::from(abs_z & 3) / 4.0;
    let mut min_i = 0;
    let mut min_distance = f64::INFINITY;
    for i in 0..8 {
        let x_even = (i & 4) == 0;
        let y_even = (i & 2) == 0;
        let z_even = (i & 1) == 0;
        let corner_x = if x_even { parent_x } else { parent_x + 1 };
        let corner_y = if y_even { parent_y } else { parent_y + 1 };
        let corner_z = if z_even { parent_z } else { parent_z + 1 };
        let distance_x = if x_even { fract_x } else { fract_x - 1.0 };
        let distance_y = if y_even { fract_y } else { fract_y - 1.0 };
        let distance_z = if z_even { fract_z } else { fract_z - 1.0 };
        let next = java_fiddled_distance(
            seed, corner_x, corner_y, corner_z, distance_x, distance_y, distance_z,
        );
        if min_distance > next {
            min_i = i;
            min_distance = next;
        }
    }
    (
        if (min_i & 4) == 0 {
            parent_x
        } else {
            parent_x + 1
        },
        if (min_i & 2) == 0 {
            parent_y
        } else {
            parent_y + 1
        },
        if (min_i & 1) == 0 {
            parent_z
        } else {
            parent_z + 1
        },
    )
}
fn java_fiddled_distance(
    seed: i64,
    x_random: i32,
    y_random: i32,
    z_random: i32,
    distance_x: f64,
    distance_y: f64,
    distance_z: f64,
) -> f64 {
    let mut rval = java_lcg_next(seed, i64::from(x_random));
    rval = java_lcg_next(rval, i64::from(y_random));
    rval = java_lcg_next(rval, i64::from(z_random));
    rval = java_lcg_next(rval, i64::from(x_random));
    rval = java_lcg_next(rval, i64::from(y_random));
    rval = java_lcg_next(rval, i64::from(z_random));
    let fiddle_x = java_fiddle(rval);
    rval = java_lcg_next(rval, seed);
    let fiddle_y = java_fiddle(rval);
    rval = java_lcg_next(rval, seed);
    let fiddle_z = java_fiddle(rval);
    (distance_z + fiddle_z).powi(2)
        + (distance_y + fiddle_y).powi(2)
        + (distance_x + fiddle_x).powi(2)
}
#[inline]
fn java_lcg_next(value: i64, addend: i64) -> i64 {
    value
        .wrapping_mul(
            value
                .wrapping_mul(6_364_136_223_846_793_005_i64)
                .wrapping_add(1_442_695_040_888_963_407_i64),
        )
        .wrapping_add(addend)
}
#[inline]
fn java_fiddle(value: i64) -> f64 {
    let uniform = ((value >> 24).rem_euclid(1024)) as f64 / 1024.0;
    (uniform - 0.5) * 0.9
}
impl dyn BlockAndTintGetter + '_ {
    pub fn get_average_grass_color(&self, pos: BlockPos) -> u32 {
        get_average_color(self, pos, |biome, x, z| {
            biome.get_grass_color(x, z, self.colormaps())
        })
    }
    pub fn get_average_foliage_color(&self, pos: BlockPos) -> u32 {
        get_average_color(self, pos, |biome, _, _| {
            biome.get_foliage_color(self.colormaps())
        })
    }
    pub fn get_average_dry_foliage_color(&self, pos: BlockPos) -> u32 {
        get_average_color(self, pos, |biome, _, _| {
            biome.get_dry_foliage_color(self.colormaps())
        })
    }
    pub fn get_average_water_color(&self, pos: BlockPos) -> u32 {
        get_average_color(self, pos, |biome, _, _| biome.water_color)
    }
}
fn get_average_color(
    level: &dyn BlockAndTintGetter,
    pos: BlockPos,
    sample_fn: impl Fn(&Biome, i32, i32) -> u32,
) -> u32 {
    let dist = level.blend_radius();
    if dist <= 0 {
        let biome = level.get_biome(pos);
        return sample_fn(biome, pos.x, pos.z);
    }
    let mut total_red = 0;
    let mut total_green = 0;
    let mut total_blue = 0;
    let count = (dist * 2 + 1) * (dist * 2 + 1);
    for dx in -dist..=dist {
        for dz in -dist..=dist {
            let sample_pos = BlockPos::new(pos.x + dx, pos.y, pos.z + dz);
            let biome = level.get_biome(sample_pos);
            let color = sample_fn(biome, sample_pos.x, sample_pos.z);
            total_red += ((color >> 16) & 0xFF) as i32;
            total_green += ((color >> 8) & 0xFF) as i32;
            total_blue += (color & 0xFF) as i32;
        }
    }
    0xFF00_0000
        | (((total_red / count) as u32) << 16)
        | (((total_green / count) as u32) << 8)
        | ((total_blue / count) as u32)
}
#[derive(Clone, Debug)]
pub struct ClientBlockColors {
    grass: ColorMap,
    foliage: ColorMap,
    dry_foliage: ColorMap,
    sources: HashMap<String, Vec<BlockTintSource>>,
    pub water_still_sprite: std::sync::OnceLock<crate::render::texture::SpriteUvRect>,
    pub water_flow_sprite: std::sync::OnceLock<crate::render::texture::SpriteUvRect>,
    pub water_overlay_sprite: std::sync::OnceLock<crate::render::texture::SpriteUvRect>,
    pub lava_still_sprite: std::sync::OnceLock<crate::render::texture::SpriteUvRect>,
    pub lava_flow_sprite: std::sync::OnceLock<crate::render::texture::SpriteUvRect>,
    pub water_layer: std::sync::OnceLock<crate::render::chunk::ChunkSectionLayer>,
    pub lava_layer: std::sync::OnceLock<crate::render::chunk::ChunkSectionLayer>,
}
fn fluid_layer_from_sprites(
    atlas: &crate::render::texture::TextureAtlas,
    sprite_ids: &[&str],
) -> crate::render::chunk::ChunkSectionLayer {
    use crate::render::chunk::ChunkSectionLayer;
    use crate::render::texture::SpriteTransparency;
    let mut transparency = SpriteTransparency::Opaque;
    for id in sprite_ids {
        let location = ResourceLocation::from_string(id);
        if !atlas.contains_sprite(&location) {
            continue;
        }
        let sprite = atlas.sprite_transparency_or_missing(&location);
        transparency = match (transparency, sprite) {
            (SpriteTransparency::Translucent, _) | (_, SpriteTransparency::Translucent) => {
                SpriteTransparency::Translucent
            }
            (SpriteTransparency::Cutout, _) | (_, SpriteTransparency::Cutout) => {
                SpriteTransparency::Cutout
            }
            _ => SpriteTransparency::Opaque,
        };
    }
    match transparency {
        SpriteTransparency::Opaque => ChunkSectionLayer::Solid,
        SpriteTransparency::Cutout => ChunkSectionLayer::Cutout,
        SpriteTransparency::Translucent => ChunkSectionLayer::Translucent,
    }
}
impl ClientBlockColors {
    pub fn bind_fluid_sprites(&self, atlas: &crate::render::texture::TextureAtlas) {
        let bind = |slot: &std::sync::OnceLock<crate::render::texture::SpriteUvRect>,
                    path: &str| {
            if let Some(sprite) = atlas
                .sprites
                .get(&ResourceLocation::from_string(path))
                .copied()
            {
                let _ = slot.set(sprite);
            }
        };
        bind(&self.water_still_sprite, "minecraft:block/water_still");
        bind(&self.water_flow_sprite, "minecraft:block/water_flow");
        bind(&self.water_overlay_sprite, "minecraft:block/water_overlay");
        bind(&self.lava_still_sprite, "minecraft:block/lava_still");
        bind(&self.lava_flow_sprite, "minecraft:block/lava_flow");
        let _ = self.water_layer.set(fluid_layer_from_sprites(
            atlas,
            &[
                "minecraft:block/water_still",
                "minecraft:block/water_flow",
                "minecraft:block/water_overlay",
            ],
        ));
        let _ = self.lava_layer.set(fluid_layer_from_sprites(
            atlas,
            &["minecraft:block/lava_still", "minecraft:block/lava_flow"],
        ));
    }
    pub fn from_resource_manager(
        resource_manager: &ClientResourceManager,
    ) -> Result<Self, ResourceError> {
        let grass = ColorMap::from_resource(
            resource_manager,
            &ResourceLocation::from_string("minecraft:colormap/grass"),
        )?;
        let foliage = ColorMap::from_resource(
            resource_manager,
            &ResourceLocation::from_string("minecraft:colormap/foliage"),
        )?;
        let dry_foliage = ColorMap::from_resource(
            resource_manager,
            &ResourceLocation::from_string("minecraft:colormap/dry_foliage"),
        )?;
        let mut sources = HashMap::new();
        let register = |sources: &mut HashMap<String, Vec<BlockTintSource>>,
                        blocks: &[&str],
                        layers: Vec<BlockTintSource>| {
            for block in blocks {
                sources.insert((*block).to_string(), layers.clone());
            }
        };
        register(
            &mut sources,
            &["large_fern", "tall_grass"],
            vec![BlockTintSource::DoubleTallGrass],
        );
        register(
            &mut sources,
            &["fern", "short_grass", "potted_fern", "bush"],
            vec![BlockTintSource::Grass],
        );
        register(
            &mut sources,
            &["grass_block"],
            vec![BlockTintSource::GrassBlock],
        );
        register(
            &mut sources,
            &["pink_petals", "wildflowers"],
            vec![
                BlockTintSource::Constant {
                    color_in_hand: 0xFFFFFFFF,
                    color_in_world: 0xFFFFFFFF,
                },
                BlockTintSource::Grass,
            ],
        );
        register(
            &mut sources,
            &["spruce_leaves"],
            vec![BlockTintSource::Constant {
                color_in_hand: 0xFF61_9961,
                color_in_world: 0xFF61_9961,
            }],
        );
        register(
            &mut sources,
            &["birch_leaves"],
            vec![BlockTintSource::Constant {
                color_in_hand: 0xFF80_A755,
                color_in_world: 0xFF80_A755,
            }],
        );
        register(
            &mut sources,
            &[
                "oak_leaves",
                "jungle_leaves",
                "acacia_leaves",
                "dark_oak_leaves",
                "vine",
                "mangrove_leaves",
            ],
            vec![BlockTintSource::Foliage],
        );
        register(
            &mut sources,
            &["leaf_litter"],
            vec![BlockTintSource::DryFoliage],
        );
        register(
            &mut sources,
            &["water_cauldron"],
            vec![BlockTintSource::Water],
        );
        register(
            &mut sources,
            &["water", "bubble_column"],
            vec![BlockTintSource::WaterParticles],
        );
        register(
            &mut sources,
            &["redstone_wire"],
            vec![BlockTintSource::Redstone],
        );
        register(
            &mut sources,
            &["sugar_cane"],
            vec![BlockTintSource::SugarCane],
        );
        register(
            &mut sources,
            &["attached_melon_stem", "attached_pumpkin_stem"],
            vec![BlockTintSource::Constant {
                color_in_hand: 0xFFE0_C71C,
                color_in_world: 0xFFE0_C71C,
            }],
        );
        register(
            &mut sources,
            &["melon_stem", "pumpkin_stem"],
            vec![BlockTintSource::Stem],
        );
        register(
            &mut sources,
            &["lily_pad"],
            vec![BlockTintSource::Constant {
                color_in_hand: 0xFF71_A1C4,
                color_in_world: 0xFF20_8050,
            }],
        );
        Ok(Self {
            grass,
            foliage,
            dry_foliage,
            sources,
            water_still_sprite: std::sync::OnceLock::new(),
            water_flow_sprite: std::sync::OnceLock::new(),
            water_overlay_sprite: std::sync::OnceLock::new(),
            water_layer: std::sync::OnceLock::new(),
            lava_layer: std::sync::OnceLock::new(),
            lava_still_sprite: std::sync::OnceLock::new(),
            lava_flow_sprite: std::sync::OnceLock::new(),
        })
    }
    pub fn tint_for_quad(
        &self,
        state: &BlockState,
        quad: &BakedQuad,
        level: Option<&dyn BlockAndTintGetter>,
        pos: BlockPos,
    ) -> [f32; 4] {
        let Some(tint_index) = quad.tint_index else {
            return WHITE_TINT;
        };
        let block = state.block.path.as_str();
        let color = if let Some(sources) = self.sources.get(block) {
            if let Some(source) = sources.get(tint_index as usize) {
                if let Some(level) = level {
                    source.color_in_world(state, level, pos)
                } else {
                    source.color(state, self)
                }
            } else {
                java_argb(-1)
            }
        } else {
            java_argb(-1)
        };
        tint_from_argb(color)
    }
    pub fn tint_for_particle(
        &self,
        state: &BlockState,
        level: Option<&dyn BlockAndTintGetter>,
        pos: BlockPos,
    ) -> [f32; 4] {
        let color = self
            .sources
            .get(state.block.path.as_str())
            .and_then(|sources| sources.first())
            .map(|source| {
                level
                    .map(|level| source.color_as_terrain_particle(state, level, pos))
                    .unwrap_or_else(|| source.color(state, self))
            })
            .unwrap_or(0xFFFF_FFFF);
        tint_from_argb(color)
    }
    pub fn default_grass_color(&self) -> u32 {
        let default_biome = Biome::default_overworld();
        default_biome.get_grass_color(0, 0, self)
    }
    pub fn grass_color(&self, temperature: f32, downfall: f32) -> u32 {
        self.grass.sample(
            temperature.clamp(0.0, 1.0) as f64,
            downfall.clamp(0.0, 1.0) as f64,
            DEFAULT_GRASS_MAP_COLOR,
        )
    }
}
fn redstone_wire_color_raw(power: i32) -> u32 {
    let power = power as f32 / 15.0;
    let red = power * 0.6 + if power > 0.0 { 0.4 } else { 0.3 };
    let green = (power * power * 0.7 - 0.5).clamp(0.0, 1.0);
    let blue = (power * power * 0.6 - 0.7).clamp(0.0, 1.0);
    argb_from_float(1.0, red, green, blue)
}
pub fn tint_from_argb(color: u32) -> [f32; 4] {
    [
        ((color >> 16) & 0xFF) as f32 / 255.0,
        ((color >> 8) & 0xFF) as f32 / 255.0,
        (color & 0xFF) as f32 / 255.0,
        ((color >> 24) & 0xFF) as f32 / 255.0,
    ]
}
const fn java_argb(value: i32) -> u32 {
    value as u32
}
const fn argb_from_rgb(red: u8, green: u8, blue: u8) -> u32 {
    argb_from_rgba(red, green, blue, 0xFF)
}
const fn argb_from_rgba(red: u8, green: u8, blue: u8, alpha: u8) -> u32 {
    ((alpha as u32) << 24) | ((red as u32) << 16) | ((green as u32) << 8) | blue as u32
}
fn argb_from_float(alpha: f32, red: f32, green: f32, blue: f32) -> u32 {
    ((as_8_bit_channel(alpha) as u32) << 24)
        | ((as_8_bit_channel(red) as u32) << 16)
        | ((as_8_bit_channel(green) as u32) << 8)
        | as_8_bit_channel(blue) as u32
}
fn as_8_bit_channel(value: f32) -> u8 {
    (value * 255.0).floor() as u8
}
