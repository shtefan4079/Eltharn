#![allow(dead_code)]

use crate::world::client_level::ChunkPos;
use el_math::block_pos::BlockPos;
use std::collections::HashSet;
use uuid::Uuid;

pub struct ClientDebugSubscriber {
    pub active_subscriptions: HashSet<String>,
}

impl Default for ClientDebugSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientDebugSubscriber {
    pub fn new() -> Self {
        Self {
            active_subscriptions: HashSet::new(),
        }
    }

    pub fn clear(&mut self) {
        self.active_subscriptions.clear();
    }

    pub fn tick(&mut self, _game_time: i64) {}

    pub fn drop_level(&mut self) {}

    pub fn drop_chunk(&mut self, _chunk_pos: ChunkPos) {}

    pub fn drop_entity(&mut self, _uuid: Uuid) {}

    pub fn update_block(&mut self, _pos: BlockPos, _state: &str) {}
}
