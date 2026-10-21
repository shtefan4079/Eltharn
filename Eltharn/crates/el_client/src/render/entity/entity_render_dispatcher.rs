#![allow(dead_code)]
use crate::gameplay::entity::PlayerSkin;
use crate::render::entity::entity_renderers::{
    AvatarRenderers, PlayerModelType, create_avatar_renderers, has_registered_entity_renderer,
};
use crate::render::entity::player::AvatarRenderer;
use glam::Vec3;
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PreparedEntityRenderContext {
    pub camera_position: Option<Vec3>,
    pub crosshair_pick_entity_id: Option<i32>,
}
#[derive(Debug, Clone, Copy)]
pub struct EntityRenderDispatcher {
    player_renderers: AvatarRenderers,
    prepared: PreparedEntityRenderContext,
}
impl Default for EntityRenderDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
impl EntityRenderDispatcher {
    pub const fn new() -> Self {
        Self {
            player_renderers: create_avatar_renderers(),
            prepared: PreparedEntityRenderContext {
                camera_position: None,
                crosshair_pick_entity_id: None,
            },
        }
    }
    pub fn get_player_renderer(&self, skin: &PlayerSkin) -> &AvatarRenderer {
        self.player_renderers.get(skin.model)
    }
    pub fn get_renderer_for_entity_type(&self, entity_type_path: &str) -> EntityRendererLookup {
        match entity_type_path {
            "player" | "mannequin" => EntityRendererLookup::Avatar(PlayerModelType::Wide),
            path if has_registered_entity_renderer(path) => EntityRendererLookup::Registered {
                entity_type: path.to_string(),
            },
            path => EntityRendererLookup::Missing {
                entity_type: path.to_string(),
            },
        }
    }
    pub fn prepare(&mut self, camera_position: Vec3, crosshair_pick_entity_id: Option<i32>) {
        self.prepared = PreparedEntityRenderContext {
            camera_position: Some(camera_position),
            crosshair_pick_entity_id,
        };
    }
    pub fn reset_camera(&mut self) {
        self.prepared.camera_position = None;
    }
    pub const fn prepared(&self) -> PreparedEntityRenderContext {
        self.prepared
    }
    pub fn distance_to_sqr(&self, entity_position: Vec3) -> Option<f32> {
        self.prepared
            .camera_position
            .map(|camera_position| camera_position.distance_squared(entity_position))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityRendererLookup {
    Avatar(PlayerModelType),
    Registered { entity_type: String },
    Missing { entity_type: String },
}
