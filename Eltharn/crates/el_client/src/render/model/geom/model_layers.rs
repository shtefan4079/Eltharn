#![allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModelLayerLocation {
    pub model: &'static str,
    pub layer: &'static str,
}
impl ModelLayerLocation {
    pub const fn new(model: &'static str, layer: &'static str) -> Self {
        Self { model, layer }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArmorModelSet<T> {
    pub helmet: T,
    pub chestplate: T,
    pub leggings: T,
    pub boots: T,
}
const fn register(model: &'static str) -> ModelLayerLocation {
    ModelLayerLocation::new(model, "main")
}
const fn register_layer(model: &'static str, layer: &'static str) -> ModelLayerLocation {
    ModelLayerLocation::new(model, layer)
}
const fn register_armor_set(model: &'static str) -> ArmorModelSet<ModelLayerLocation> {
    ArmorModelSet {
        helmet: register_layer(model, "helmet"),
        chestplate: register_layer(model, "chestplate"),
        leggings: register_layer(model, "leggings"),
        boots: register_layer(model, "boots"),
    }
}
pub const PLAYER: ModelLayerLocation = register("player");
pub const PLAYER_CAPE: ModelLayerLocation = register_layer("player", "cape");
pub const PLAYER_EARS: ModelLayerLocation = register_layer("player", "ears");
pub const PLAYER_HEAD: ModelLayerLocation = register("player_head");
pub const PLAYER_ARMOR: ArmorModelSet<ModelLayerLocation> = register_armor_set("player");
pub const PLAYER_ARMOR_HELMET: ModelLayerLocation = PLAYER_ARMOR.helmet;
pub const PLAYER_ARMOR_CHESTPLATE: ModelLayerLocation = PLAYER_ARMOR.chestplate;
pub const PLAYER_ARMOR_LEGGINGS: ModelLayerLocation = PLAYER_ARMOR.leggings;
pub const PLAYER_ARMOR_BOOTS: ModelLayerLocation = PLAYER_ARMOR.boots;
pub const PLAYER_SLIM: ModelLayerLocation = register("player_slim");
pub const PLAYER_SLIM_ARMOR: ArmorModelSet<ModelLayerLocation> = register_armor_set("player_slim");
pub const PLAYER_SLIM_ARMOR_HELMET: ModelLayerLocation = PLAYER_SLIM_ARMOR.helmet;
pub const PLAYER_SLIM_ARMOR_CHESTPLATE: ModelLayerLocation = PLAYER_SLIM_ARMOR.chestplate;
pub const PLAYER_SLIM_ARMOR_LEGGINGS: ModelLayerLocation = PLAYER_SLIM_ARMOR.leggings;
pub const PLAYER_SLIM_ARMOR_BOOTS: ModelLayerLocation = PLAYER_SLIM_ARMOR.boots;
pub const PLAYER_SPIN_ATTACK: ModelLayerLocation = register("spin_attack");
pub const PLAYER_ARMOR_INNER: ModelLayerLocation = PLAYER_ARMOR_LEGGINGS;
pub const PLAYER_ARMOR_OUTER: ModelLayerLocation = PLAYER_ARMOR_CHESTPLATE;
pub const PLAYER_SLIM_ARMOR_INNER: ModelLayerLocation = PLAYER_SLIM_ARMOR_LEGGINGS;
pub const PLAYER_SLIM_ARMOR_OUTER: ModelLayerLocation = PLAYER_SLIM_ARMOR_CHESTPLATE;
pub const PLAYER_KNOWN_LOCATIONS: &[ModelLayerLocation] = &[
    PLAYER,
    PLAYER_CAPE,
    PLAYER_EARS,
    PLAYER_HEAD,
    PLAYER_ARMOR_HELMET,
    PLAYER_ARMOR_CHESTPLATE,
    PLAYER_ARMOR_LEGGINGS,
    PLAYER_ARMOR_BOOTS,
    PLAYER_SLIM,
    PLAYER_SLIM_ARMOR_HELMET,
    PLAYER_SLIM_ARMOR_CHESTPLATE,
    PLAYER_SLIM_ARMOR_LEGGINGS,
    PLAYER_SLIM_ARMOR_BOOTS,
    PLAYER_SPIN_ATTACK,
];
pub fn get_known_player_locations() -> impl Iterator<Item = ModelLayerLocation> {
    PLAYER_KNOWN_LOCATIONS.iter().copied()
}
pub fn is_known_player_location(location: ModelLayerLocation) -> bool {
    PLAYER_KNOWN_LOCATIONS.contains(&location)
}
