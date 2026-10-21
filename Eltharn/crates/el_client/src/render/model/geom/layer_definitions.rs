#![allow(dead_code)]
use super::model_layers::{
    ModelLayerLocation, PLAYER, PLAYER_ARMOR_BOOTS, PLAYER_ARMOR_CHESTPLATE, PLAYER_ARMOR_HELMET,
    PLAYER_ARMOR_LEGGINGS, PLAYER_CAPE, PLAYER_EARS, PLAYER_HEAD, PLAYER_SLIM,
    PLAYER_SLIM_ARMOR_BOOTS, PLAYER_SLIM_ARMOR_CHESTPLATE, PLAYER_SLIM_ARMOR_HELMET,
    PLAYER_SLIM_ARMOR_LEGGINGS, PLAYER_SPIN_ATTACK,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerDefinition {
    pub location: ModelLayerLocation,
    pub texture_width: u16,
    pub texture_height: u16,
    pub slim: bool,
}
impl LayerDefinition {
    pub const fn player(location: ModelLayerLocation, slim: bool) -> Self {
        Self {
            location,
            texture_width: 64,
            texture_height: 64,
            slim,
        }
    }
    pub const fn player_armor(location: ModelLayerLocation, slim: bool) -> Self {
        Self {
            location,
            texture_width: 64,
            texture_height: 32,
            slim,
        }
    }
    pub const fn player_cape(location: ModelLayerLocation) -> Self {
        Self {
            location,
            texture_width: 64,
            texture_height: 32,
            slim: false,
        }
    }
}
pub const PLAYER_LAYERS: [LayerDefinition; 14] = [
    LayerDefinition::player(PLAYER, false),
    LayerDefinition::player(PLAYER_SLIM, true),
    LayerDefinition::player_armor(PLAYER_ARMOR_HELMET, false),
    LayerDefinition::player_armor(PLAYER_ARMOR_CHESTPLATE, false),
    LayerDefinition::player_armor(PLAYER_ARMOR_LEGGINGS, false),
    LayerDefinition::player_armor(PLAYER_ARMOR_BOOTS, false),
    LayerDefinition::player_armor(PLAYER_SLIM_ARMOR_HELMET, true),
    LayerDefinition::player_armor(PLAYER_SLIM_ARMOR_CHESTPLATE, true),
    LayerDefinition::player_armor(PLAYER_SLIM_ARMOR_LEGGINGS, true),
    LayerDefinition::player_armor(PLAYER_SLIM_ARMOR_BOOTS, true),
    LayerDefinition::player_cape(PLAYER_CAPE),
    LayerDefinition::player(PLAYER_EARS, false),
    LayerDefinition::player(PLAYER_HEAD, false),
    LayerDefinition::player(PLAYER_SPIN_ATTACK, false),
];
pub fn create_player_roots() -> impl Iterator<Item = LayerDefinition> {
    PLAYER_LAYERS.into_iter()
}
pub fn get_player_layer(location: ModelLayerLocation) -> Option<LayerDefinition> {
    PLAYER_LAYERS
        .iter()
        .copied()
        .find(|layer| layer.location == location)
}
