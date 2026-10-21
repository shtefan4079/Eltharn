#![allow(dead_code)]
#[allow(dead_code)]
pub mod gameplay_tick;
pub(crate) mod interaction;
pub mod inventory;
pub(crate) mod maps;
pub(crate) mod movement;
pub mod phases;
pub mod pick;
pub mod player_state;
pub(crate) mod prediction;
pub mod screen_tick;
pub(crate) mod tick;
pub(crate) mod use_item_decision;
pub(crate) mod vehicle;
use crate::client::tick_timer::DeltaTrackerTimer;
use crate::gameplay::entity::remote::RemoteBoatInputLatch;
use crate::gameplay::entity::remote::{RemoteEntities, RemoteEntityFrameSnapshot};
use crate::gameplay::game_mode::GameType;
use crate::gameplay::player::{LocalPlayer, PlayerPose};
use crate::input::{PlayerInput, VanillaKeyMappings};
use crate::network::local_connection::LocalConnection;
use crate::network::multiplayer::multiplayer_game_mode::MultiPlayerGameMode;
use crate::network::protocol_backend::RemoteConnectionHandle;
use crate::world::client_level::{ChunkPos, ClientLevel};
use el_core::item::ItemSwingAnimationType;
use el_math::block_pos::BlockPos;
use el_protocol_backend::BackendNetworkEvent;
use glam::{DVec3, Vec3};
use movement::*;
use std::collections::HashMap;
use std::collections::VecDeque;
#[derive(Clone, Debug)]
pub struct SimulationFrameState {
    pub player_position: Vec3,
    pub player_eye_position: Vec3,
    pub player_yaw: f32,
    pub player_pitch: f32,
    pub player_pose: PlayerPose,
    pub player_health: f32,
    pub player_food: i32,
    pub tick_count: u64,
    pub remote_entity_frames: HashMap<i32, RemoteEntityFrameSnapshot>,
}
pub struct ClientSession {
    pub(crate) remote_last_move_sent: Option<RemoteMoveDebugSample>,
    pub(crate) remote_move_packets_sent: u64,
    pub(crate) remote_move_send_state: VanillaMoveSendState,
    pub(crate) remote_movement_parity_frames: VecDeque<RemoteMovementParityFrame>,
    pub(crate) remote_last_sent_input: Option<PlayerInput>,
    pub(crate) remote_was_sprinting: bool,
    pub(crate) remote_boat_input_latch: Option<RemoteBoatInputLatch>,
    pub(crate) local_move_send_state: VanillaMoveSendState,
    pub(crate) swing_time: i32,
    pub(crate) swing_duration: i32,
    pub(crate) swing_animation_type: ItemSwingAnimationType,
    pub(crate) swinging: bool,
    pub(crate) swinging_hand: crate::render::model::player::player_model::HandSide,
    pub(crate) o_attack_anim: f32,
    pub(crate) attack_anim: f32,
    pub player: LocalPlayer,
    pub level: Option<ClientLevel>,
    pub remote_entities: RemoteEntities,
    pub remote_push_policy: phases::RemoteEntityPushPolicy,
    pub maps: HashMap<u32, crate::render::map::MapData>,
    pub game_mode: MultiPlayerGameMode,
    pub remote_connection: Option<RemoteConnectionHandle>,
    pub connection: Option<LocalConnection>,
    pub controls: VanillaKeyMappings,
    pub player_input: PlayerInput,
    pub player_flying: bool,
    pub remote_player_entity_id: Option<i32>,
    pub remote_action_sequence: u32,
    pub logical_client_tick_sequence: u64,
    pub player_inventory_state_id: i32,
    pub player_inventory_carried_item: el_core::item::ItemStack,
    pub active_hotbar_slot: usize,
    pub is_left_mouse_pressed: bool,
    pub is_use_pressed: bool,
    pub miss_time: i32,
    pub right_click_delay_ticks: i32,
    pub delta_tracker: DeltaTrackerTimer,
    pub toggled_crouch_down: bool,
    pub toggled_sprint_down: bool,
    pub toggled_attack_down: bool,
    pub toggled_use_down: bool,
    pub jump_trigger_time: i32,
    pub(crate) item_use: crate::client::item_use::LocalItemUse,
    pub last_kinetic_hit_feedback_game_time: Option<i64>,
    pub remote_vehicle_entity_id: Option<i32>,
    pub removed_player_vehicle_id: Option<i32>,
    pub remote_movement_was_held: bool,
    pub remote_position_confirmed: bool,
    pub player_loaded_sent: bool,
    pub remote_teleport_wait_chunk: Option<ChunkPos>,
    pub remote_level_y_bounds: Option<(i32, i32)>,
    pub remote_sea_level: Option<i32>,
    pub remote_dimension: Option<String>,
    pub remote_level_hardcore: bool,
    pub remote_show_death_screen: bool,
    pub remote_respawn_request_sent: bool,
    pub remote_spawn_pos: Option<BlockPos>,
    pub remote_spawn_angle: f32,
    pub remote_player_uuid: Option<uuid::Uuid>,
    pub experience_display_start_tick: i64,
    pub chunk_cache_center: Option<(i32, i32)>,
    pub chunk_cache_radius: Option<u32>,
    pub simulation_distance: Option<u32>,
}
pub const RESPAWN_KEEP_ATTRIBUTE_MODIFIERS: u8 = 1;
pub const RESPAWN_KEEP_ENTITY_DATA: u8 = 2;
#[derive(Clone, Debug)]
pub struct LoginApply {
    pub entity_id: i32,
    pub uuid: uuid::Uuid,
    pub game_mode: u8,
    pub dimension: String,
    pub sea_level: i32,
    pub min_y: i32,
    pub height: u32,
}
#[derive(Clone, Debug)]
pub struct RespawnApply {
    pub game_mode: u8,
    pub dimension: String,
    pub sea_level: i32,
    pub min_y: i32,
    pub height: u32,
    pub data_to_keep: u8,
}
impl ClientSession {
    pub fn new(spawn_pos: Vec3, game_mode: MultiPlayerGameMode) -> Self {
        Self {
            remote_last_move_sent: None,
            remote_move_packets_sent: 0,
            remote_move_send_state: VanillaMoveSendState::default(),
            remote_movement_parity_frames: VecDeque::new(),
            remote_last_sent_input: Some(PlayerInput::new()),
            remote_was_sprinting: false,
            remote_boat_input_latch: None,
            local_move_send_state: VanillaMoveSendState::default(),
            swing_time: 0,
            swing_duration: 6,
            swing_animation_type: ItemSwingAnimationType::Whack,
            swinging: false,
            swinging_hand: crate::render::model::player::player_model::HandSide::Right,
            o_attack_anim: 0.0,
            attack_anim: 0.0,
            player: LocalPlayer::new(spawn_pos),
            game_mode,
            level: None,
            remote_entities: RemoteEntities::new(),
            remote_push_policy: phases::RemoteEntityPushPolicy::default(),
            maps: HashMap::new(),
            remote_connection: None,
            connection: None,
            controls: VanillaKeyMappings::new(),
            player_input: PlayerInput::new(),
            player_flying: false,
            remote_player_entity_id: None,
            remote_action_sequence: 0,
            logical_client_tick_sequence: 0,
            player_inventory_state_id: 0,
            player_inventory_carried_item: el_core::item::ItemStack::empty(),
            active_hotbar_slot: 0,
            is_left_mouse_pressed: false,
            is_use_pressed: false,
            miss_time: 0,
            right_click_delay_ticks: 0,
            delta_tracker: DeltaTrackerTimer::new(20.0),
            toggled_crouch_down: false,
            toggled_sprint_down: false,
            toggled_attack_down: false,
            toggled_use_down: false,
            jump_trigger_time: 0,
            item_use: Default::default(),
            last_kinetic_hit_feedback_game_time: None,
            remote_vehicle_entity_id: None,
            removed_player_vehicle_id: None,
            remote_movement_was_held: true,
            remote_position_confirmed: false,
            player_loaded_sent: false,
            remote_teleport_wait_chunk: None,
            remote_level_y_bounds: None,
            remote_sea_level: None,
            remote_dimension: None,
            remote_level_hardcore: false,
            remote_show_death_screen: true,
            remote_respawn_request_sent: false,
            remote_spawn_pos: None,
            remote_spawn_angle: 0.0,
            remote_player_uuid: None,
            experience_display_start_tick: i64::MIN,
            chunk_cache_center: None,
            chunk_cache_radius: None,
            simulation_distance: None,
        }
    }
    pub fn set_level(&mut self, level: ClientLevel) {
        self.level = Some(level);
    }
    pub fn level(&self) -> Option<&ClientLevel> {
        self.level.as_ref()
    }
    pub fn level_mut(&mut self) -> Option<&mut ClientLevel> {
        self.level.as_mut()
    }
    pub fn player(&self) -> &LocalPlayer {
        &self.player
    }
    pub fn player_mut(&mut self) -> &mut LocalPlayer {
        &mut self.player
    }
    pub fn remote_entities(&self) -> &RemoteEntities {
        &self.remote_entities
    }
    pub fn remote_entities_mut(&mut self) -> &mut RemoteEntities {
        &mut self.remote_entities
    }
    pub fn attach_remote_connection(&mut self, handle: RemoteConnectionHandle) {
        self.remote_connection = Some(handle);
    }
    pub fn drain_network_events(&mut self) {
        let Some(rx) = self.remote_connection.as_ref().map(|c| c.event_rx.clone()) else {
            return;
        };
        while let Ok(event) = rx.try_recv() {
            self.apply_network_event(&event);
        }
    }
    pub fn apply_network_event(&mut self, event: &BackendNetworkEvent) {
        let mut remote_context = crate::gameplay::entity::remote::RemoteEntityEventContext {
            level: self.level.as_ref(),
            local_player_entity_id: self.remote_player_entity_id,
            ..crate::gameplay::entity::remote::RemoteEntityEventContext::empty()
        };
        let remote_id = match event {
            BackendNetworkEvent::EntityAdded { entity } => Some(entity.id),
            BackendNetworkEvent::EntityMetadata { id, .. }
            | BackendNetworkEvent::EntityPositionSync { id, .. }
            | BackendNetworkEvent::EntityTeleport { id, .. }
            | BackendNetworkEvent::EntityDeltaMoved { id, .. }
            | BackendNetworkEvent::EntityVelocity { id, .. }
            | BackendNetworkEvent::EntityEquipment { id, .. }
            | BackendNetworkEvent::EntityHeadLook { id, .. }
            | BackendNetworkEvent::EntityEvent { id, .. }
            | BackendNetworkEvent::EntityAnimation { id, .. } => Some(*id),
            _ => None,
        };
        if matches!(event, BackendNetworkEvent::EntitiesRemoved { .. })
            || remote_id.is_some_and(|id| Some(id) != self.remote_player_entity_id)
        {
            self.remote_entities
                .apply_network_event(event, &mut remote_context);
            return;
        }
        match event {
            BackendNetworkEvent::PlayerPosition {
                teleport_id,
                raw_x,
                raw_y,
                raw_z,
                raw_yaw_degrees,
                raw_pitch_degrees,
                raw_delta_x,
                raw_delta_y,
                raw_delta_z,
                relative,
            } => {
                let correction = crate::gameplay::player::correction::PlayerCorrection::resolve(
                    &self.player,
                    crate::gameplay::entity::remote::RemotePositionMoveRotation {
                        position: DVec3::new(*raw_x, *raw_y, *raw_z),
                        velocity: DVec3::new(*raw_delta_x, *raw_delta_y, *raw_delta_z),
                        yaw_degrees: *raw_yaw_degrees,
                        pitch_degrees: *raw_pitch_degrees,
                    },
                    *relative,
                );
                if !self.player.is_passenger {
                    correction.apply(&mut self.player);
                }
                if let Some(handle) = &self.remote_connection {
                    let position = self.player.exact_position();
                    handle.packet_tx.send_teleport_confirm(
                        *teleport_id,
                        position.x,
                        position.y,
                        position.z,
                        self.player.y_rot_degrees(),
                        self.player.x_rot_degrees(),
                    );
                }
            }
            BackendNetworkEvent::BlockChangedAck { sequence } => {
                let _ = prediction::resolve_block_ack(self, *sequence);
            }
            BackendNetworkEvent::BlockUpdate { update } => {
                self.apply_block_update(update);
            }
            BackendNetworkEvent::SectionBlocksUpdate { updates } => {
                for update in updates {
                    self.apply_block_update(update);
                }
            }
            BackendNetworkEvent::EntityMetadata {
                id,
                shared_flags,
                no_gravity,
                ..
            } if self.remote_player_entity_id == Some(*id) => {
                self.apply_local_no_gravity_metadata(*id, *no_gravity);
                if let Some(flags) = shared_flags {
                    self.player.apply_synced_shared_flags(*flags);
                }
            }
            BackendNetworkEvent::MapItemData {
                map_id,
                scale,
                locked,
                tracking_position,
                color_patch,
                decorations,
            } => {
                self.apply_map_update(
                    *map_id,
                    *scale,
                    *locked,
                    *tracking_position,
                    color_patch.as_ref(),
                    decorations.as_deref(),
                );
            }
            _ => {}
        }
    }
    fn apply_block_update(&mut self, update: &el_protocol_backend::BackendBlockUpdateEvent) {
        let Some(level) = self.level.as_mut() else {
            return;
        };
        let Some(state) = el_core::registry::BuiltInRegistries::block_states()
            .get_state_by_id(update.state_id)
            .cloned()
        else {
            return;
        };
        let pos = BlockPos::new(update.x, update.y, update.z);
        if crate::gameplay::player::correction::retain_server_block_update(level, pos, &state)
            .is_none()
        {
            crate::gameplay::player::correction::apply_authoritative_block_state(
                level, pos, &state,
            );
        }
    }
    pub fn apply_time_event(
        &mut self,
        game_time: i64,
        day_time: Option<i64>,
        day_time_rate: Option<f32>,
        day_time_partial_tick: Option<f32>,
    ) {
        if let Some(level) = self.level.as_mut() {
            level.set_server_time(game_time, day_time, day_time_rate, day_time_partial_tick);
        }
    }
    pub fn apply_ticking_state_event(&mut self, tick_rate: f32, is_frozen: bool) {
        self.delta_tracker.set_tick_rate(tick_rate);
        self.delta_tracker.update_frozen_state(is_frozen);
    }
    pub fn apply_ticking_step_event(&mut self, tick_steps: u32) {
        self.delta_tracker.set_frozen_ticks_to_run(tick_steps);
    }
    pub fn apply_chunk_cache_event(
        &mut self,
        center: Option<(i32, i32)>,
        radius: Option<u32>,
        simulation_distance: Option<u32>,
    ) {
        if let Some((x, z)) = center {
            self.chunk_cache_center = Some((x, z));
            if let Some(level) = self.level.as_mut() {
                level.set_chunk_cache_center(x, z);
            }
        }
        if let Some(radius) = radius {
            self.chunk_cache_radius = Some(radius);
            if let Some(level) = self.level.as_mut() {
                level.update_view_radius(radius as i32);
            }
        }
        if let Some(distance) = simulation_distance {
            self.simulation_distance = Some(distance);
        }
    }
    pub fn apply_change_difficulty_event(&mut self, difficulty: u8, locked: bool) {
        if let Some(level) = self.level.as_mut() {
            level.set_difficulty(crate::world::client_level::Difficulty::from_id(difficulty));
            level.set_difficulty_locked(locked);
        }
    }
    pub fn apply_ping_event(&self, id: u32) {
        if let Some(conn) = self.remote_connection.as_ref() {
            conn.packet_tx.send_pong(id);
        }
    }
    pub(crate) fn send_perform_respawn_if_needed(&mut self, reason: &str, force: bool) {
        if self.remote_respawn_request_sent {
            return;
        }
        if !force && self.player.health > 0.0 {
            return;
        }
        let mut sent = false;
        if let Some(conn) = self.remote_connection.as_ref() {
            conn.packet_tx.send_perform_respawn();
            sent = true;
        }
        if let Some(conn) = self.connection.as_mut() {
            let _ = conn.send_packet(
                &el_protocol::packets::game::ServerboundClientCommandPacket { action: 0 },
            );
            sent = true;
        }
        if sent {
            self.remote_respawn_request_sent = true;
            log::info!(
                "[Network] Sent PERFORM_RESPAWN reason={} health={:.2} show_death_screen={} force={}",
                reason,
                self.player.health,
                self.remote_show_death_screen,
                force
            );
        }
    }
    pub fn apply_game_event(&mut self, event_id: u8, value: f32) {
        match event_id {
            1 => {
                if let Some(level) = self.level.as_mut() {
                    level.set_rain_level(0.0);
                }
            }
            2 => {
                if let Some(level) = self.level.as_mut() {
                    level.set_rain_level(1.0);
                }
            }
            3 => {
                self.game_mode.set_local_mode(GameType::by_id(value as i32));
                self.reconcile_player_flying_from_game_mode(false);
            }
            7 => {
                if let Some(level) = self.level.as_mut() {
                    level.set_rain_level(value);
                }
            }
            8 => {
                if let Some(level) = self.level.as_mut() {
                    level.set_thunder_level(value);
                }
            }
            11 => {
                self.remote_show_death_screen = value == 0.0;
            }
            _ => {}
        }
    }
    pub(crate) fn reset_connection_vehicle_state_like_java(&mut self) {
        self.player.is_passenger = false;
        self.remote_vehicle_entity_id = None;
        self.removed_player_vehicle_id = None;
        self.remote_boat_input_latch = None;
    }
    pub fn apply_login_event(&mut self, login: &LoginApply) {
        self.reset_connection_vehicle_state_like_java();
        self.maps.clear();
        self.player.apply_synced_shared_flags(0);
        self.player.no_gravity = false;
        self.game_mode
            .set_local_mode(GameType::by_id(login.game_mode as i32));
        self.reconcile_player_flying_from_game_mode(false);
        self.remote_player_entity_id = Some(login.entity_id);
        self.remote_player_uuid = Some(login.uuid);
        self.miss_time = 0;
        self.experience_display_start_tick = i64::MIN;
        self.remote_respawn_request_sent = false;
        self.remote_level_y_bounds = Some((login.min_y, login.min_y + login.height as i32 - 1));
        self.remote_sea_level = Some(login.sea_level);
        self.remote_dimension = Some(login.dimension.clone());
        self.remote_last_sent_input = Some(PlayerInput::new());
        self.remote_was_sprinting = false;
        self.player.reset_input_snapshot_like_java();
        self.remote_position_confirmed = false;
        self.remote_teleport_wait_chunk = None;
        self.player_loaded_sent = false;
    }
    pub fn apply_respawn_event(
        &mut self,
        respawn: &RespawnApply,
        replaces_level: bool,
        preserved_input: Option<PlayerInput>,
        preserved_sprinting: bool,
    ) {
        self.remote_dimension = Some(respawn.dimension.clone());
        if replaces_level {
            self.remote_level_y_bounds =
                Some((respawn.min_y, respawn.min_y + respawn.height as i32 - 1));
        }
        self.remote_sea_level = Some(respawn.sea_level);
        self.active_hotbar_slot = 0;
        let keeps_entity_data = (respawn.data_to_keep & RESPAWN_KEEP_ENTITY_DATA) != 0;
        self.remote_last_sent_input = Some(PlayerInput::new());
        self.remote_was_sprinting = false;
        if keeps_entity_data {
            self.remote_last_sent_input = preserved_input;
            self.remote_was_sprinting = preserved_sprinting;
        }
        self.miss_time = 0;
        self.remote_respawn_request_sent = false;
        self.player.reset_input_snapshot_like_java();
        self.remote_position_confirmed = false;
        self.remote_teleport_wait_chunk = None;
        self.player_loaded_sent = false;
    }
    pub fn extract_frame(&self, partial_tick: f32) -> SimulationFrameState {
        let player_pos = self
            .player
            .prev_position
            .lerp(self.player.position, partial_tick);
        let player_eye = player_pos + Vec3::new(0.0, self.player.pose_eye_height(false), 0.0);
        let player_yaw =
            self.player.prev_yaw + (self.player.yaw - self.player.prev_yaw) * partial_tick;
        let player_pitch =
            self.player.prev_pitch + (self.player.pitch - self.player.prev_pitch) * partial_tick;
        let mut remote_entity_frames = HashMap::new();
        for &id in self.remote_entities.keys() {
            if let Some(snapshot) = self.remote_entities.extract_frame(id, partial_tick) {
                remote_entity_frames.insert(id, snapshot);
            }
        }
        SimulationFrameState {
            player_position: player_pos,
            player_eye_position: player_eye,
            player_yaw,
            player_pitch,
            player_pose: self.player.current_pose(false),
            player_health: self.player.health,
            player_food: self.player.food,
            tick_count: self.logical_client_tick_sequence,
            remote_entity_frames,
        }
    }
}
