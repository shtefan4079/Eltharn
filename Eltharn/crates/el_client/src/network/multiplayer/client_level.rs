use crate::network::multiplayer::{
    client_chunk_cache::{ChunkPos, ClientChunkCache},
    client_explosion_tracker::ClientExplosionTracker,
    prediction::BlockStatePredictionHandler,
};
use std::collections::{BTreeSet, HashMap};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Difficulty {
    #[default]
    Normal,
    Peaceful,
    Easy,
    Hard,
}
impl Difficulty {
    pub fn from_id(id: u8) -> Self {
        match id {
            0 => Self::Peaceful,
            1 => Self::Easy,
            2 => Self::Normal,
            3 => Self::Hard,
            _ => Self::Normal,
        }
    }
}
#[derive(Debug, Clone)]
pub struct BlockDestructionProgress {
    pub id: i32,
    pub pos: [i32; 3],
    pub progress: i32,
    pub updated_render_tick: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockDestructionProgressOrder {
    pub progress: i32,
    pub id: i32,
}
impl BlockDestructionProgress {
    pub fn new(id: i32, pos: [i32; 3]) -> Self {
        BlockDestructionProgress {
            id,
            pos,
            progress: 0,
            updated_render_tick: 0,
        }
    }
    pub fn set_progress(&mut self, progress: i32) {
        self.progress = progress;
    }
    pub fn update_tick(&mut self, game_time: i64) {
        self.updated_render_tick = game_time;
    }
    pub fn order_key(&self) -> BlockDestructionProgressOrder {
        BlockDestructionProgressOrder {
            progress: self.progress,
            id: self.id,
        }
    }
    pub fn pos_key(&self) -> i64 {
        pack_block_pos(self.pos)
    }
}
fn pack_block_pos(pos: [i32; 3]) -> i64 {
    let x = pos[0] as i64 & 0x3FF_FFFF;
    let y = pos[1] as i64 & 0xFFF;
    let z = pos[2] as i64 & 0x3FF_FFFF;
    (x << 38) | (y << 26) | z
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ClientLevelData {
    pub difficulty: Difficulty,
    pub hardcore: bool,
    pub is_flat: bool,
    pub game_time: i64,
    pub difficulty_locked: bool,
    pub spawn: Option<[i32; 3]>,
}
#[allow(dead_code)]
impl ClientLevelData {
    pub fn new(difficulty: Difficulty, hardcore: bool, is_flat: bool) -> Self {
        ClientLevelData {
            difficulty,
            hardcore,
            is_flat,
            game_time: 0,
            difficulty_locked: false,
            spawn: None,
        }
    }
    pub fn set_game_time(&mut self, time: i64) {
        self.game_time = time;
    }
    pub fn set_difficulty(&mut self, difficulty: Difficulty) {
        self.difficulty = difficulty;
    }
    pub fn set_difficulty_locked(&mut self, locked: bool) {
        self.difficulty_locked = locked;
    }
    pub fn set_spawn(&mut self, spawn: [i32; 3]) {
        self.spawn = Some(spawn);
    }
    pub fn horizon_height(&self, min_y: i32) -> f64 {
        if self.is_flat { min_y as f64 } else { 63.0 }
    }
    pub fn void_darkness_onset_range(&self) -> f32 {
        if self.is_flat { 1.0 } else { 32.0 }
    }
}
#[allow(dead_code)]
pub struct ClientLevel {
    pub level_data: ClientLevelData,
    pub chunk_source: ClientChunkCache,
    pub block_state_prediction_handler: BlockStatePredictionHandler,
    pub explosion_tracker: ClientExplosionTracker,
    sky_flash_time: i32,
    pub server_simulation_distance: i32,
    pub sea_level: i32,
    pub destroying_blocks: HashMap<i32, BlockDestructionProgress>,
    pub destruction_progress: HashMap<i64, BTreeSet<BlockDestructionProgressOrder>>,
    pub map_data: HashMap<i32, Vec<u8>>,
    pub light_update_count: usize,
}
#[allow(dead_code)]
impl ClientLevel {
    pub fn new(
        level_data: ClientLevelData,
        server_chunk_radius: i32,
        server_simulation_distance: i32,
        sea_level: i32,
    ) -> Self {
        ClientLevel {
            chunk_source: ClientChunkCache::new(server_chunk_radius),
            block_state_prediction_handler: BlockStatePredictionHandler::new(),
            explosion_tracker: ClientExplosionTracker::new(),
            sky_flash_time: 0,
            server_simulation_distance,
            sea_level,
            destroying_blocks: HashMap::new(),
            destruction_progress: HashMap::new(),
            map_data: HashMap::new(),
            light_update_count: 0,
            level_data,
        }
    }
    pub fn tick(&mut self, running_normally: bool) {
        if running_normally {
            self.tick_time();
            self.remove_block_breaking_progress();
        }
        if self.sky_flash_time > 0 {
            self.sky_flash_time -= 1;
        }
        self.explosion_tracker.tick();
    }
    fn tick_time(&mut self) {
        self.level_data.game_time += 1;
    }
    pub fn set_time_from_server(&mut self, game_time: i64) {
        self.level_data.game_time = game_time;
    }
    pub fn destroy_block_progress(&mut self, id: i32, pos: [i32; 3], progress: i32) {
        if (0..10).contains(&progress) {
            let previous = self.destroying_blocks.remove(&id);
            if let Some(previous) = previous.as_ref() {
                self.remove_progress_entry(previous);
            }
            let mut entry = match previous {
                Some(entry) if entry.pos == pos => entry,
                _ => BlockDestructionProgress::new(id, pos),
            };
            entry.set_progress(progress);
            entry.update_tick(self.level_data.game_time);
            let key = entry.pos_key();
            self.destruction_progress
                .entry(key)
                .or_default()
                .insert(entry.order_key());
            self.destroying_blocks.insert(id, entry);
        } else if let Some(removed) = self.destroying_blocks.remove(&id) {
            self.remove_progress_entry(&removed);
        }
    }
    fn remove_progress_entry(&mut self, entry: &BlockDestructionProgress) {
        let key = entry.pos_key();
        let remove_position = self
            .destruction_progress
            .get_mut(&key)
            .map(|set| {
                set.remove(&entry.order_key());
                set.is_empty()
            })
            .unwrap_or(false);
        if remove_position {
            self.destruction_progress.remove(&key);
        }
    }
    fn remove_block_breaking_progress(&mut self) {
        if self.level_data.game_time % 20 != 0 {
            return;
        }
        let game_time = self.level_data.game_time;
        let stale: Vec<i32> = self
            .destroying_blocks
            .iter()
            .filter(|(_, p)| game_time - p.updated_render_tick > 400)
            .map(|(id, _)| *id)
            .collect();
        for id in stale {
            if let Some(removed) = self.destroying_blocks.remove(&id) {
                self.remove_progress_entry(&removed);
            }
        }
    }
    pub fn handle_block_changed_ack(&mut self, sequence: i32) {
        let sequence = sequence.max(0) as u32;
        self.block_state_prediction_handler
            .server_verified_states
            .retain(|_pos, state| state.sequence > sequence);
    }
    pub fn chunk_source(&self) -> &ClientChunkCache {
        &self.chunk_source
    }
    pub fn chunk_source_mut(&mut self) -> &mut ClientChunkCache {
        &mut self.chunk_source
    }
    pub fn on_chunk_loaded(&mut self, pos: ChunkPos) {
        let _ = pos;
    }
    pub fn unload_chunk(&mut self, _pos: ChunkPos) {}
    pub fn set_sky_flash_time(&mut self, t: i32) {
        self.sky_flash_time = t;
    }
    pub fn sky_flash_time(&self) -> i32 {
        self.sky_flash_time
    }
    pub fn set_server_simulation_distance(&mut self, d: i32) {
        self.server_simulation_distance = d;
    }
    pub fn get_server_simulation_distance(&self) -> i32 {
        self.server_simulation_distance
    }
    pub fn get_map_data(&self, id: i32) -> Option<&Vec<u8>> {
        self.map_data.get(&id)
    }
    pub fn override_map_data(&mut self, id: i32, data: Vec<u8>) {
        self.map_data.insert(id, data);
    }
    pub fn gather_chunk_source_stats(&self) -> String {
        format!("Chunks[C] W: {}", self.chunk_source.gather_stats())
    }
    pub fn to_string_repr(&self) -> &'static str {
        "ClientLevel"
    }
}
