use super::{ClientSession, phases::InputOptions};
use crate::gameplay::entity::remote::RemoteEntitiesTickOutput;
use crate::network::protocol_backend::RemotePacketSender;
use crate::render::model::player::player_model::HandSide;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ClientTickGate {
    pub runs_normally: bool,
    pub local_player_loaded: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PreprocessedClientTick {
    pub tick_index: u32,
    pub gate: ClientTickGate,
}

pub trait ScreenTickHost {
    fn begin_tick_observation(&mut self) {}
    fn finish_tick_observation(&mut self) {}
    fn session(&self) -> &ClientSession;
    fn session_mut(&mut self) -> &mut ClientSession;
    fn tick_connection_loading(&mut self);
    fn tick_gui(&mut self);
    fn screen_active(&self) -> bool;
    fn handle_keybinds(&mut self);
    fn input_options(&self) -> InputOptions;
    fn tick_camera(&mut self);
    fn tick_active_item_use(&mut self);
    fn before_remote_entity_tick(&mut self);
    fn apply_remote_entity_effects(&mut self, output: RemoteEntitiesTickOutput);
    fn tick_controlled_vehicle(&mut self, vehicle_id: i32) -> Option<(bool, bool)>;
    fn attach_player_to_vehicle(&mut self, vehicle_id: i32);
    fn latch_vehicle_input(&mut self, vehicle_id: i32);
    fn main_hand(&self) -> HandSide;
    fn controlled_camera(&self) -> bool;
    fn auto_jump_enabled(&self) -> bool;
    fn tick_level_effects(&mut self, runs_normally: bool);
}

pub fn begin_client_tick(host: &mut impl ScreenTickHost, runs_normally: bool) -> ClientTickGate {
    host.begin_tick_observation();
    host.session_mut().begin_client_tick();
    if host.session().level.is_some() {
        host.tick_connection_loading();
    }
    host.tick_gui();
    if host.screen_active() {
        host.session_mut().miss_time = 10_000;
    } else {
        host.handle_keybinds();
        let options = host.input_options();
        host.session_mut()
            .sync_player_input_from_key_mappings(options);
    }
    ClientTickGate {
        runs_normally,
        local_player_loaded: host.session().remote_connection.is_none()
            || host.session().player_loaded_sent,
    }
}

pub fn finish_client_tick(host: &mut impl ScreenTickHost, runs_normally: bool) {
    let session = host.session_mut();
    if session.remote_connection.is_none() || !session.remote_movement_was_held {
        session.player.tick_death();
    }
    if runs_normally {
        if let Some(level) = session.level.as_mut() {
            level.tick_time();
        }
    }
    host.tick_level_effects(runs_normally);
    host.session_mut().finish_client_tick();
    host.finish_tick_observation();
}

pub(crate) fn tick_remote_entities(
    host: &mut impl ScreenTickHost,
    runs_normally: bool,
    vehicle_id: Option<i32>,
) {
    if host.session().level.is_none() {
        return;
    }
    host.before_remote_entity_tick();
    let output = host
        .session_mut()
        .tick_remote_entities(runs_normally, vehicle_id);
    host.apply_remote_entity_effects(output);
}

fn attach_and_latch_vehicle(host: &mut impl ScreenTickHost, vehicle_id: i32) {
    host.attach_player_to_vehicle(vehicle_id);
    host.latch_vehicle_input(vehicle_id);
}

fn send_screen_player_packets(
    host: &mut impl ScreenTickHost,
    packet_tx: &RemotePacketSender,
    vehicle_id: Option<i32>,
    controlled_camera: bool,
) {
    let auto_jump_enabled = host.auto_jump_enabled();
    let session = host.session_mut();
    if vehicle_id.is_some() {
        session.send_local_player_input_if_changed();
        if let Some(connection) = session.connection.as_mut() {
            let packet = el_protocol::packets::game::ServerboundMovePlayerRotPacket {
                y_rot: session.player.y_rot_degrees(),
                x_rot: session.player.x_rot_degrees(),
                on_ground: session.player.on_ground,
                horizontal_collision: session.player.horizontal_collision,
            };
            let _ = connection.send_packet(&packet);
        }
    }
    session.send_remote_fall_flying_before_player_input_if_needed(packet_tx);
    let sent_input = session.player.input.key_presses.clone();
    ClientSession::send_remote_player_input_if_changed(
        packet_tx,
        &sent_input,
        &mut session.remote_last_sent_input,
    );
    session.send_remote_move_if_needed(packet_tx, "screen", controlled_camera);
    if controlled_camera && session.remote_vehicle_entity_id.is_none() {
        session.player.auto_jump_enabled = auto_jump_enabled;
    }
}

pub fn tick_remote_player_while_screen_active(
    host: &mut impl ScreenTickHost,
    client_ticks: u32,
    tick_runs_normally: &[bool],
) -> Option<PreprocessedClientTick> {
    let packet_tx = host.session().remote_connection.as_ref()?.packet_tx.clone();
    host.session().level.as_ref()?;

    for tick_index in 0..client_ticks {
        let gate = begin_client_tick(
            host,
            tick_runs_normally
                .get(tick_index as usize)
                .copied()
                .unwrap_or(true),
        );
        if !host.screen_active() {
            return Some(PreprocessedClientTick { tick_index, gate });
        }
        host.session_mut().remote_movement_was_held = !gate.local_player_loaded;
        host.tick_camera();
        if !gate.local_player_loaded {
            let vehicle_id = host.session().remote_vehicle_entity_id;
            tick_remote_entities(host, gate.runs_normally, vehicle_id);
            if let Some(vehicle_id) = vehicle_id {
                if let Some((left, right)) = host.tick_controlled_vehicle(vehicle_id) {
                    packet_tx.send_paddle_boat(left, right);
                }
                attach_and_latch_vehicle(host, vehicle_id);
            }
            finish_client_tick(host, gate.runs_normally);
            continue;
        }

        host.tick_active_item_use();
        let vehicle_id = host.session().remote_vehicle_entity_id;
        host.session_mut().player.is_passenger = vehicle_id.is_some();
        tick_remote_entities(host, gate.runs_normally, vehicle_id);
        let boat_paddle = vehicle_id.and_then(|id| host.tick_controlled_vehicle(id));
        if let Some((left, right)) = boat_paddle {
            packet_tx.send_paddle_boat(left, right);
        }

        let main_hand = host.main_hand();
        host.session_mut().tick_attack_animation_once(main_hand);
        let session = host.session_mut();
        session
            .player
            .set_attack_anim_for_living_tick(session.attack_anim);
        session.tick_creative_flight_toggle_like_java();
        session.enforce_spectator_flying_like_java();
        let controlled_camera = host.controlled_camera();
        let session = host.session_mut();
        session
            .player
            .set_controlled_camera_like_java(controlled_camera);
        session.tick_screen_player_travel(vehicle_id);
        session
            .player
            .tick_attack_strength(session.active_hotbar_slot);

        send_screen_player_packets(host, &packet_tx, vehicle_id, controlled_camera);
        if let Some(vehicle_id) = vehicle_id {
            attach_and_latch_vehicle(host, vehicle_id);
        }
        finish_client_tick(host, gate.runs_normally);
    }
    None
}
