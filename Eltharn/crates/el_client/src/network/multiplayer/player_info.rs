#![allow(dead_code)]

use crate::gameplay::game_mode::GameType;
use el_core::Component;
use el_core::GameProfile;

#[derive(Clone, Debug)]
pub struct PlayerInfo {
    pub profile: GameProfile,
    pub game_mode: GameType,
    pub latency: i32,
    pub tab_list_display_name: Option<Component>,
    pub show_hat: bool,
    pub tab_list_order: i32,
}

impl PlayerInfo {
    pub fn new(profile: GameProfile) -> Self {
        Self {
            profile,
            game_mode: GameType::DEFAULT_MODE,
            latency: 0,
            tab_list_display_name: None,
            show_hat: true,
            tab_list_order: 0,
        }
    }
}
