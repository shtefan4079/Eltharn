#![allow(dead_code)]

use crate::network::multiplayer::level_load_tracker::LevelLoadTracker;
use crate::network::multiplayer::player_info::PlayerInfo;
use crate::network::multiplayer::server_data::ServerData;
use el_core::GameProfile;
use el_core::registry::RegistryAccess;
use el_core::resource_location::ResourceLocation;
use std::collections::HashMap;
use uuid::Uuid;

pub struct CommonListenerCookie {
    pub level_load_tracker: LevelLoadTracker,
    pub local_game_profile: GameProfile,
    pub received_registries: RegistryAccess,
    pub server_brand: Option<String>,
    pub server_data: Option<ServerData>,
    pub server_cookies: HashMap<ResourceLocation, Vec<u8>>,
    pub seen_players: HashMap<Uuid, PlayerInfo>,
    pub seen_insecure_chat_warning: bool,
}

impl CommonListenerCookie {
    pub fn new(
        level_load_tracker: LevelLoadTracker,
        local_game_profile: GameProfile,
        received_registries: RegistryAccess,
        server_data: Option<ServerData>,
    ) -> Self {
        Self {
            level_load_tracker,
            local_game_profile,
            received_registries,
            server_brand: None,
            server_data,
            server_cookies: HashMap::new(),
            seen_players: HashMap::new(),
            seen_insecure_chat_warning: false,
        }
    }
}
