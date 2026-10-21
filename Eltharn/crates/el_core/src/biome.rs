use crate::resource_location::ResourceLocation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MobCategory {
    Monster,
    Creature,
    Ambient,
    Axolotls,
    UndergroundWaterCreature,
    WaterCreature,
    WaterAmbient,
    Misc,
}

impl MobCategory {
    pub fn get_serialized_name(&self) -> &'static str {
        match self {
            MobCategory::Monster => "monster",
            MobCategory::Creature => "creature",
            MobCategory::Ambient => "ambient",
            MobCategory::Axolotls => "axolotls",
            MobCategory::UndergroundWaterCreature => "underground_water_creature",
            MobCategory::WaterCreature => "water_creature",
            MobCategory::WaterAmbient => "water_ambient",
            MobCategory::Misc => "misc",
        }
    }

    pub fn get_max_instances_per_chunk(&self) -> i32 {
        match self {
            MobCategory::Monster => 70,
            MobCategory::Creature => 10,
            MobCategory::Ambient => 15,
            MobCategory::Axolotls => 5,
            MobCategory::UndergroundWaterCreature => 5,
            MobCategory::WaterCreature => 5,
            MobCategory::WaterAmbient => 20,
            MobCategory::Misc => -1,
        }
    }

    pub fn is_friendly(&self) -> bool {
        match self {
            MobCategory::Monster => false,
            _ => true,
        }
    }

    pub fn is_persistent(&self) -> bool {
        match self {
            MobCategory::Creature | MobCategory::Misc => true,
            _ => false,
        }
    }

    pub fn get_despawn_distance(&self) -> i32 {
        match self {
            MobCategory::WaterAmbient => 64,
            _ => 128,
        }
    }

    pub fn get_no_despawn_distance(&self) -> i32 {
        32
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpawnerData {
    #[serde(rename = "type")]
    pub entity_type: ResourceLocation,
    #[serde(rename = "minCount")]
    pub min_count: i32,
    #[serde(rename = "maxCount")]
    pub max_count: i32,
    pub weight: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MobSpawnSettings {
    pub spawners: HashMap<MobCategory, Vec<SpawnerData>>,
    pub creature_spawn_probability: f32,
}

impl MobSpawnSettings {
    pub fn empty() -> Self {
        Self {
            spawners: HashMap::new(),
            creature_spawn_probability: 0.1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Biome {
    pub location: ResourceLocation,
    pub spawn_settings: MobSpawnSettings,
}
