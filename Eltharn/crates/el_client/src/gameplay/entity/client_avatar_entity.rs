#![allow(dead_code)]

use crate::gameplay::entity::client_avatar_state::ClientAvatarState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlayerModelType {
    #[default]
    Wide,
    Slim,
}

impl PlayerModelType {
    pub fn by_legacy_services_name(name: Option<&str>) -> Self {
        match name.map(str::trim) {
            Some(name) if name.eq_ignore_ascii_case("slim") => Self::Slim,
            _ => Self::Wide,
        }
    }

    pub fn legacy_services_id(self) -> &'static str {
        match self {
            Self::Wide => "default",
            Self::Slim => "slim",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlayerSkin {
    pub body: String,
    pub cape: Option<String>,
    pub elytra: Option<String>,
    pub model: PlayerModelType,
    pub secure: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParrotVariant {
    RedBlue,
    Blue,
    Green,
    Yellow,
    Grey,
}

pub trait ClientAvatarEntity {
    fn avatar_state(&self) -> &ClientAvatarState;
    fn avatar_state_mut(&mut self) -> &mut ClientAvatarState;
    fn get_skin(&self) -> PlayerSkin;
    fn get_parrot_variant_on_shoulder(&self, left: bool) -> Option<ParrotVariant>;
    fn show_extra_ears(&self) -> bool;
}
