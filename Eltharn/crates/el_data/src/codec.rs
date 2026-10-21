use serde::Deserialize;
use std::collections::HashMap;
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TagJson {
    #[serde(default)]
    pub replace: bool,
    pub values: Vec<String>,
}
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct RecipeJson {
    #[serde(rename = "type")]
    pub recipe_type: String,
    pub category: Option<String>,
    #[serde(flatten)]
    pub details: HashMap<String, serde_json::Value>,
}
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct LootTableJson {
    #[serde(rename = "type")]
    pub table_type: Option<String>,
    #[serde(default)]
    pub pools: Vec<serde_json::Value>,
}
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct AdvancementJson {
    pub parent: Option<String>,
    pub display: Option<serde_json::Value>,
    pub criteria: HashMap<String, serde_json::Value>,
    pub requirements: Option<serde_json::Value>,
    pub rewards: Option<serde_json::Value>,
}
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct DamageTypeJson {
    pub message_id: String,
    pub scaling: String,
    pub exhaustion: f32,
}
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct DimensionTypeJson {
    pub ambient_light: f32,
    pub logical_height: i32,
    pub coordinate_scale: f64,
    pub infiniburn: String,
    pub min_y: i32,
    pub height: i32,
    #[serde(flatten)]
    pub attributes: HashMap<String, serde_json::Value>,
}
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct BiomeJson {
    pub temperature: f32,
    pub downfall: f32,
    pub effects: serde_json::Value,
    pub spawners: Option<serde_json::Value>,
}
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct StructureSetJson {
    pub structures: Vec<StructureSetEntryJson>,
    pub placement: StructurePlacementJson,
}
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct StructureSetEntryJson {
    pub structure: String,
    pub weight: i32,
}
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct StructurePlacementJson {
    #[serde(rename = "type")]
    pub placement_type: String,
    pub salt: Option<i64>,
    pub spacing: Option<i32>,
    pub separation: Option<i32>,
    pub count: Option<i32>,
    pub distance: Option<i32>,
    pub spread: Option<i32>,
}
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct StructureJson {
    #[serde(rename = "type")]
    pub structure_type: String,
    pub biomes: serde_json::Value,
    pub step: Option<String>,
}
