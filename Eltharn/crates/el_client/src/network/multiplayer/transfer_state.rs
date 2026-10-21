#![allow(dead_code)]

use crate::network::multiplayer::player_info::PlayerInfo;
use el_core::resource_location::ResourceLocation;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone)]
pub struct TransferState {
    pub cookies: HashMap<ResourceLocation, Vec<u8>>,
    pub seen_players: HashMap<Uuid, PlayerInfo>,
    pub seen_insecure_chat_warning: bool,
}

impl TransferState {
    pub fn new(
        cookies: HashMap<ResourceLocation, Vec<u8>>,
        seen_players: HashMap<Uuid, PlayerInfo>,
        seen_insecure_chat_warning: bool,
    ) -> Self {
        Self {
            cookies,
            seen_players,
            seen_insecure_chat_warning,
        }
    }
}
