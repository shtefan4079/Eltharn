use crate::network::multiplayer::{
    chunk_batch_size_calculator::ChunkBatchSizeCalculator,
    client_advancements::ClientAdvancements,
    client_debug_subscriber::ClientDebugSubscriber,
    client_level::{ClientLevel, ClientLevelData, Difficulty},
    client_recipe_container::ClientRecipeContainer,
    client_suggestion_provider::ClientSuggestionProvider,
    common_listener_cookie::CommonListenerCookie,
    known_packs_manager::KnownPacksManager,
    level_load_tracker::LevelLoadTracker,
    multiplayer_game_mode::MultiPlayerGameMode,
    ping_debug_monitor::PingDebugMonitor,
    player_info::PlayerInfo,
    registry_data_collector::RegistryDataCollector,
    session_search_trees::SessionSearchTrees,
    transfer_state::TransferState,
};
use el_core::UUID;
use std::collections::{HashMap, HashSet};
#[allow(dead_code)]
pub struct ClientPacketListener {
    pub local_game_profile_name: String,
    pub local_game_profile_id: UUID,
    pub level: Option<ClientLevel>,
    pub level_data: Option<ClientLevelData>,
    pub player_info_map: HashMap<UUID, PlayerInfo>,
    pub listed_players: HashSet<UUID>,
    pub game_mode: Option<MultiPlayerGameMode>,
    pub advancements: ClientAdvancements,
    pub suggestions_provider: ClientSuggestionProvider,
    pub recipes: ClientRecipeContainer,
    pub online_mode: bool,
    pub server_enforces_secure_chat: bool,
    pub next_chat_index: i32,
    pub seen_insecure_chat_warning: bool,
    pub server_chunk_radius: i32,
    pub server_simulation_distance: i32,
    pub chunk_batch_size_calculator: ChunkBatchSizeCalculator,
    pub ping_debug_monitor: PingDebugMonitor,
    pub debug_subscriber: ClientDebugSubscriber,
    pub level_load_tracker: Option<LevelLoadTracker>,
    pub closed: bool,
    pub client_loaded: bool,
    pub search_trees: SessionSearchTrees,
    pub transfer_state: Option<TransferState>,
    pub known_packs: KnownPacksManager,
    pub registry_data_collector: RegistryDataCollector,
}
#[allow(dead_code)]
impl ClientPacketListener {
    pub fn new(cookie: CommonListenerCookie) -> Self {
        let profile_name = cookie.local_game_profile.name().to_owned();
        let profile_id = cookie.local_game_profile.id();
        ClientPacketListener {
            local_game_profile_name: profile_name,
            local_game_profile_id: profile_id,
            level: None,
            level_data: None,
            player_info_map: HashMap::new(),
            listed_players: HashSet::new(),
            game_mode: None,
            advancements: ClientAdvancements::new(),
            suggestions_provider: ClientSuggestionProvider::new(),
            recipes: ClientRecipeContainer::new(),
            online_mode: false,
            server_enforces_secure_chat: false,
            next_chat_index: 0,
            seen_insecure_chat_warning: cookie.seen_insecure_chat_warning,
            server_chunk_radius: 3,
            server_simulation_distance: 3,
            chunk_batch_size_calculator: ChunkBatchSizeCalculator::new(),
            ping_debug_monitor: PingDebugMonitor::new(),
            debug_subscriber: ClientDebugSubscriber::new(),
            level_load_tracker: Some(cookie.level_load_tracker),
            closed: false,
            client_loaded: false,
            search_trees: SessionSearchTrees::new(),
            transfer_state: None,
            known_packs: KnownPacksManager::new(),
            registry_data_collector: RegistryDataCollector::new(),
        }
    }
    pub fn close(&mut self) {
        self.closed = true;
        self.clear_level();
    }
    pub fn clear_level(&mut self) {
        self.level = None;
        self.level_data = None;
        self.level_load_tracker = None;
    }
    pub fn set_client_loaded(&mut self, loaded: bool) {
        self.client_loaded = loaded;
    }
    pub fn is_client_loaded(&self) -> bool {
        self.client_loaded
    }
    pub fn get_player_info(&self, uuid: UUID) -> Option<&PlayerInfo> {
        self.player_info_map.get(&uuid)
    }
    pub fn listed_players(&self) -> impl Iterator<Item = &PlayerInfo> {
        self.listed_players
            .iter()
            .filter_map(|id| self.player_info_map.get(id))
    }
    pub fn tick(&mut self) {
        if let Some(ref mut level) = self.level {
            level.tick(true);
        }
    }
    pub fn handle_login(
        &mut self,
        _player_id: i32,
        hardcore: bool,
        is_flat: bool,
        chunk_radius: i32,
        simulation_distance: i32,
        sea_level: i32,
        online_mode: bool,
        enforces_secure_chat: bool,
    ) {
        self.server_chunk_radius = chunk_radius;
        self.server_simulation_distance = simulation_distance;
        let level_data = ClientLevelData::new(Difficulty::Normal, hardcore, is_flat);
        let level = ClientLevel::new(
            level_data.clone(),
            chunk_radius,
            simulation_distance,
            sea_level,
        );
        self.level_data = Some(level_data);
        self.level = Some(level);
        self.game_mode = Some(MultiPlayerGameMode::new());
        self.set_client_loaded(false);
        self.debug_subscriber.clear();
        self.next_chat_index = 0;
        self.online_mode = online_mode;
        self.server_enforces_secure_chat = enforces_secure_chat;
    }
    pub fn handle_respawn(
        &mut self,
        is_flat: bool,
        chunk_radius: i32,
        simulation_distance: i32,
        sea_level: i32,
    ) {
        if let Some(ref mut data) = self.level_data {
            let hardcore = data.hardcore;
            *data = ClientLevelData::new(data.difficulty, hardcore, is_flat);
        }
        if let Some(ref data) = self.level_data {
            self.level = Some(ClientLevel::new(
                data.clone(),
                chunk_radius,
                simulation_distance,
                sea_level,
            ));
        }
    }
    pub fn handle_chunk_batch_start(&mut self) {
        self.chunk_batch_size_calculator.on_batch_start();
    }
    pub fn handle_chunk_batch_finished(&mut self, batch_size: i32) {
        self.chunk_batch_size_calculator
            .on_batch_finished(batch_size);
    }
    pub fn handle_block_changed_ack(&mut self, sequence: i32) {
        if let Some(ref mut level) = self.level {
            level.handle_block_changed_ack(sequence);
        }
    }
    pub fn handle_set_simulation_distance(&mut self, distance: i32) {
        self.server_simulation_distance = distance;
        if let Some(ref mut level) = self.level {
            level.set_server_simulation_distance(distance);
        }
    }
    pub fn handle_set_chunk_cache_radius(&mut self, radius: i32) {
        self.server_chunk_radius = radius;
        if let Some(ref mut level) = self.level {
            level.chunk_source_mut().update_view_radius(radius);
        }
    }
    pub fn handle_set_chunk_cache_center(&mut self, x: i32, z: i32) {
        if let Some(ref mut level) = self.level {
            level.chunk_source_mut().update_view_center(x, z);
        }
    }
    pub fn upsert_player_info(&mut self, uuid: UUID, info: PlayerInfo) {
        self.player_info_map.insert(uuid, info);
    }
    pub fn remove_player_info(&mut self, uuid: UUID) {
        self.player_info_map.remove(&uuid);
        self.listed_players.remove(&uuid);
    }
    pub fn mark_message_as_processed(&mut self, _signature: &[u8], _acknowledged: bool) {}
    pub fn enforces_secure_chat(&self) -> bool {
        self.server_enforces_secure_chat
    }
    pub fn server_brand(&self) -> &str {
        "unknown"
    }
}
