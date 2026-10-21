use super::ClientSession;
use crate::gameplay::entity::remote::*;
use crate::gameplay::player::LocalPlayer;
use crate::input::PlayerInput;
use crate::network::protocol_backend::RemotePacketSender as PacketSender;
use el_math::block_pos::BlockPos;
use web_time::Instant;
fn movement_diagnostics_enabled() -> bool {
    LocalPlayer::movement_diagnostics_enabled()
}
pub(crate) fn remote_send_position_is_allowed(is_controlled_camera: bool) -> bool {
    is_controlled_camera
}
const PARITY_PROBE_MOVEMENT_WINDOW_CAPACITY: usize = 24;
const VANILLA_MOVE_POSITION_DELTA_SQUARED: f64 = 4.0E-8;
const DEBUG_MOVE_AND_CHUNKS: bool = false;
#[derive(Clone, Copy)]
pub(crate) struct RemoteMoveDebugSample {
    pub(crate) pos: glam::Vec3,
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
    pub(crate) sent_at: Instant,
    pub(crate) sequence: u64,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct VanillaMoveSendState {
    pub(crate) last_pos: Option<glam::DVec3>,
    pub(crate) last_yaw: f32,
    pub(crate) last_pitch: f32,
    pub(crate) last_on_ground: bool,
    pub(crate) last_horizontal_collision: bool,
    pub(crate) position_reminder: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MoveSendAction {
    None,
    PosRot,
    Pos,
    Rot,
    StatusOnly,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct MoveSendDecision {
    pub(crate) baseline_pos: glam::DVec3,
    pub(crate) delta: glam::DVec3,
    pub(crate) delta_len_sq: f64,
    pub(crate) reminder_before: u32,
    pub(crate) reminder_after_increment: u32,
    pub(crate) moved: bool,
    pub(crate) rotated: bool,
    pub(crate) status_changed: bool,
    pub(crate) action: MoveSendAction,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum RemoteMovementTraceReason {
    Screen,
    Walking,
    Flying,
    Other,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum RemoteMovementTracePath {
    SendPosition,
    Passenger,
    CameraNotControlled,
}
#[derive(Clone, Debug)]
pub(crate) struct RemoteMovementParityFrame {
    pub(crate) client_tick: u64,
    pub(crate) reason: RemoteMovementTraceReason,
    pub(crate) path: RemoteMovementTracePath,
    pub(crate) action: MoveSendAction,
    pub(crate) packet_sequence: u64,
    pub(crate) pos: glam::DVec3,
    pub(crate) velocity: glam::DVec3,
    pub(crate) yaw_degrees: f32,
    pub(crate) pitch_degrees: f32,
    pub(crate) on_ground: bool,
    pub(crate) horizontal_collision: bool,
    pub(crate) input: PlayerInput,
    pub(crate) decision: Option<MoveSendDecision>,
    pub(crate) sprinting: bool,
    pub(crate) minor_horizontal_collision: bool,
    pub(crate) movement_speed: f64,
    pub(crate) aabb: [f64; 6],
    pub(crate) player_chunk_loaded: Option<bool>,
    pub(crate) loaded_chunks: usize,
}
impl Default for VanillaMoveSendState {
    fn default() -> Self {
        Self {
            last_pos: Some(glam::DVec3::ZERO),
            last_yaw: 0.0,
            last_pitch: 0.0,
            last_on_ground: false,
            last_horizontal_collision: false,
            position_reminder: 0,
        }
    }
}
impl VanillaMoveSendState {
    pub(crate) fn next_decision(
        &self,
        pos: glam::DVec3,
        yaw: f32,
        pitch: f32,
        on_ground: bool,
        horizontal_collision: bool,
    ) -> MoveSendDecision {
        let baseline_pos = self.last_pos.unwrap_or(glam::DVec3::ZERO);
        let delta = pos - baseline_pos;
        let delta_len_sq = delta.length_squared();
        let reminder_before = self.position_reminder;
        let reminder_after_increment = reminder_before.saturating_add(1);
        let moved =
            delta_len_sq > VANILLA_MOVE_POSITION_DELTA_SQUARED || reminder_after_increment >= 20;
        let rotated = yaw != self.last_yaw || pitch != self.last_pitch;
        let status_changed = on_ground != self.last_on_ground
            || horizontal_collision != self.last_horizontal_collision;
        let action = if moved && rotated {
            MoveSendAction::PosRot
        } else if moved {
            MoveSendAction::Pos
        } else if rotated {
            MoveSendAction::Rot
        } else if status_changed {
            MoveSendAction::StatusOnly
        } else {
            MoveSendAction::None
        };
        MoveSendDecision {
            baseline_pos,
            delta,
            delta_len_sq,
            reminder_before,
            reminder_after_increment,
            moved,
            rotated,
            status_changed,
            action,
        }
    }
    pub(crate) fn get_send_action(
        &mut self,
        pos: glam::DVec3,
        yaw: f32,
        pitch: f32,
        on_ground: bool,
        horizontal_collision: bool,
    ) -> MoveSendAction {
        let decision = self.next_decision(pos, yaw, pitch, on_ground, horizontal_collision);
        self.position_reminder = decision.reminder_after_increment;
        if decision.moved {
            self.last_pos = Some(pos);
            self.position_reminder = 0;
        }
        if decision.rotated {
            self.last_yaw = yaw;
            self.last_pitch = pitch;
        }
        self.last_on_ground = on_ground;
        self.last_horizontal_collision = horizontal_collision;
        decision.action
    }
}
impl RemoteMovementTraceReason {
    pub(crate) fn from_send_reason(reason: &str) -> Self {
        match reason {
            "screen" => Self::Screen,
            "walking" => Self::Walking,
            "flying" => Self::Flying,
            _ => Self::Other,
        }
    }
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Screen => "screen",
            Self::Walking => "walking",
            Self::Flying => "flying",
            Self::Other => "other",
        }
    }
}
impl RemoteMovementTracePath {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SendPosition => "send_position",
            Self::Passenger => "passenger",
            Self::CameraNotControlled => "camera_not_controlled",
        }
    }
}
impl ClientSession {
    pub(crate) fn send_remote_move_if_needed(
        &mut self,
        packet_tx: &PacketSender,
        reason: &str,
        is_controlled_camera: bool,
    ) -> bool {
        let abilities_flying = self.player_flying;
        let entity_id = self.remote_player_entity_id;
        let vehicle_entity_id = self.remote_vehicle_entity_id;
        let locally_authoritative_root_vehicle_id = vehicle_entity_id.and_then(|vehicle_id| {
            known_locally_authoritative_root_vehicle_id(
                vehicle_id,
                entity_id,
                &self.remote_entities,
            )
        });
        let (
            pos,
            exact_pos,
            yaw,
            pitch,
            yaw_degrees,
            pitch_degrees,
            on_ground,
            horizontal_collision,
            input,
            velocity,
            is_sprinting,
            pose,
            crouching,
        ) = {
            let player = &self.player;
            (
                player.position,
                player.exact_position(),
                player.yaw,
                player.pitch,
                player.y_rot_degrees(),
                player.x_rot_degrees(),
                player.on_ground,
                player.horizontal_collision,
                player.input.key_presses.clone(),
                player.exact_velocity(),
                player.is_sprinting,
                player.current_pose(abilities_flying),
                player.is_crouching_like_java(),
            )
        };
        let velocity_y = velocity.y;
        if vehicle_entity_id.is_some() {
            self.trace_remote_movement_decision(
                reason,
                RemoteMovementTracePath::Passenger,
                MoveSendAction::Rot,
                self.remote_move_packets_sent.saturating_add(1),
                exact_pos,
                velocity,
                yaw_degrees,
                pitch_degrees,
                on_ground,
                horizontal_collision,
                &input,
                None,
            );
            packet_tx.send_move_rot(yaw_degrees, pitch_degrees, on_ground, horizontal_collision);
            if let Some(root_vehicle_id) = locally_authoritative_root_vehicle_id {
                if let Some(vehicle) = self.remote_entities.get(&root_vehicle_id) {
                    let vehicle_pose = vehicle.position_move_rotation();
                    packet_tx.send_move_vehicle(
                        vehicle_pose.position.x,
                        vehicle_pose.position.y,
                        vehicle_pose.position.z,
                        vehicle_pose.yaw_degrees,
                        vehicle_pose.pitch_degrees,
                        vehicle.on_ground,
                    );
                    Self::send_remote_sprinting_if_needed(
                        packet_tx,
                        entity_id,
                        is_sprinting,
                        &mut self.remote_was_sprinting,
                    );
                }
            }
            self.remote_move_packets_sent = self.remote_move_packets_sent.saturating_add(1);
            return true;
        }
        Self::send_remote_sprinting_if_needed(
            packet_tx,
            entity_id,
            is_sprinting,
            &mut self.remote_was_sprinting,
        );
        if !remote_send_position_is_allowed(is_controlled_camera) {
            self.trace_remote_movement_decision(
                reason,
                RemoteMovementTracePath::CameraNotControlled,
                MoveSendAction::None,
                self.remote_move_packets_sent,
                exact_pos,
                velocity,
                yaw_degrees,
                pitch_degrees,
                on_ground,
                horizontal_collision,
                &input,
                None,
            );
            return false;
        }
        let decision = self.remote_move_send_state.next_decision(
            exact_pos,
            yaw_degrees,
            pitch_degrees,
            on_ground,
            horizontal_collision,
        );
        let action = self.remote_move_send_state.get_send_action(
            exact_pos,
            yaw_degrees,
            pitch_degrees,
            on_ground,
            horizontal_collision,
        );
        debug_assert_eq!(action, decision.action);
        let packet_sequence = if action == MoveSendAction::None {
            self.remote_move_packets_sent
        } else {
            self.remote_move_packets_sent.wrapping_add(1)
        };
        self.trace_remote_movement_decision(
            reason,
            RemoteMovementTracePath::SendPosition,
            action,
            packet_sequence,
            exact_pos,
            velocity,
            yaw_degrees,
            pitch_degrees,
            on_ground,
            horizontal_collision,
            &input,
            Some(decision),
        );
        if action != MoveSendAction::None {
            match action {
                MoveSendAction::PosRot => packet_tx.send_move_pos_rot(
                    exact_pos.x,
                    exact_pos.y,
                    exact_pos.z,
                    yaw_degrees,
                    pitch_degrees,
                    on_ground,
                    horizontal_collision,
                ),
                MoveSendAction::Pos => packet_tx.send_move_pos(
                    exact_pos.x,
                    exact_pos.y,
                    exact_pos.z,
                    on_ground,
                    horizontal_collision,
                ),
                MoveSendAction::Rot => packet_tx.send_move_rot(
                    yaw_degrees,
                    pitch_degrees,
                    on_ground,
                    horizontal_collision,
                ),
                MoveSendAction::StatusOnly => {
                    packet_tx.send_move_status_only(on_ground, horizontal_collision)
                }
                MoveSendAction::None => {}
            }
            self.remote_move_packets_sent = self.remote_move_packets_sent.wrapping_add(1);
            self.remote_last_move_sent = Some(RemoteMoveDebugSample {
                pos,
                yaw,
                pitch,
                sent_at: Instant::now(),
                sequence: self.remote_move_packets_sent,
            });
            if movement_diagnostics_enabled() || LocalPlayer::slide_probe_enabled() {
                crate::gameplay::player::local_player::LocalPlayer::slide_probe_log(format!(
                    "[MovementDebug] outbound seq={} action={:?} reason={} pos=[{:.12},{:.12},{:.12}] yaw={:.6} pitch={:.6} on_ground={} horiz_collision={} vel_y={:.12} input=[f:{} b:{} l:{} r:{} jump:{} sneak:{} sprint:{}] pose={:?} crouching={} reminder={}",
                    self.remote_move_packets_sent,
                    action,
                    reason,
                    exact_pos.x,
                    exact_pos.y,
                    exact_pos.z,
                    yaw_degrees,
                    pitch_degrees,
                    on_ground,
                    horizontal_collision,
                    velocity_y,
                    input.forward,
                    input.backward,
                    input.left,
                    input.right,
                    input.jump,
                    input.sneak,
                    input.sprint,
                    pose,
                    crouching,
                    self.remote_move_send_state.position_reminder,
                ));
            }
        }
        action != MoveSendAction::None
    }
    pub(crate) fn trace_remote_movement_decision(
        &mut self,
        reason: &str,
        path: RemoteMovementTracePath,
        action: MoveSendAction,
        packet_sequence: u64,
        pos: glam::DVec3,
        velocity: glam::DVec3,
        yaw_degrees: f32,
        pitch_degrees: f32,
        on_ground: bool,
        horizontal_collision: bool,
        input: &PlayerInput,
        decision: Option<MoveSendDecision>,
    ) {
        let reason = RemoteMovementTraceReason::from_send_reason(reason);
        if movement_diagnostics_enabled()
            && let Some(decision) = decision
        {
            println!(
                "[ParityProbe] movement_decision tick={} reason={} path={} seq={} action={:?} pos=[{:.12},{:.12},{:.12}] baseline=[{:.12},{:.12},{:.12}] delta=[{:.12},{:.12},{:.12}] delta_len_sq={:.12e} threshold={:.12e} reminder_before={} reminder_after_increment={} move={} rot={} status={} on_ground={} horiz_collision={} vel=[{:.12},{:.12},{:.12}] input=[f:{} b:{} l:{} r:{} jump:{} sneak:{} sprint={}] yaw={:.6} pitch={:.6}",
                self.logical_client_tick_sequence,
                reason.as_str(),
                path.as_str(),
                packet_sequence,
                action,
                pos.x,
                pos.y,
                pos.z,
                decision.baseline_pos.x,
                decision.baseline_pos.y,
                decision.baseline_pos.z,
                decision.delta.x,
                decision.delta.y,
                decision.delta.z,
                decision.delta_len_sq,
                VANILLA_MOVE_POSITION_DELTA_SQUARED,
                decision.reminder_before,
                decision.reminder_after_increment,
                decision.moved,
                decision.rotated,
                decision.status_changed,
                on_ground,
                horizontal_collision,
                velocity.x,
                velocity.y,
                velocity.z,
                input.forward,
                input.backward,
                input.left,
                input.right,
                input.jump,
                input.sneak,
                input.sprint,
                yaw_degrees,
                pitch_degrees,
            );
        } else if movement_diagnostics_enabled() {
            println!(
                "[ParityProbe] movement_decision tick={} reason={} path={} seq={} action={:?} decision=not_applicable pos=[{:.12},{:.12},{:.12}] on_ground={} horiz_collision={} vel=[{:.12},{:.12},{:.12}] input=[f:{} b:{} l:{} r:{} jump:{} sneak:{} sprint={}] yaw={:.6} pitch={:.6}",
                self.logical_client_tick_sequence,
                reason.as_str(),
                path.as_str(),
                packet_sequence,
                action,
                pos.x,
                pos.y,
                pos.z,
                on_ground,
                horizontal_collision,
                velocity.x,
                velocity.y,
                velocity.z,
                input.forward,
                input.backward,
                input.left,
                input.right,
                input.jump,
                input.sneak,
                input.sprint,
                yaw_degrees,
                pitch_degrees,
            );
        }
        if self.remote_movement_parity_frames.len() >= PARITY_PROBE_MOVEMENT_WINDOW_CAPACITY {
            self.remote_movement_parity_frames.pop_front();
        }
        self.remote_movement_parity_frames
            .push_back(RemoteMovementParityFrame {
                client_tick: self.logical_client_tick_sequence,
                reason,
                path,
                action,
                packet_sequence,
                pos,
                velocity,
                yaw_degrees,
                pitch_degrees,
                on_ground,
                horizontal_collision,
                input: input.clone(),
                decision,
                sprinting: self.player.is_sprinting,
                minor_horizontal_collision: self.player.minor_horizontal_collision,
                movement_speed: self
                    .player
                    .attr_f64(crate::gameplay::attributes::MOVEMENT_SPEED),
                aabb: {
                    let bb = self.player.get_aabb();
                    [bb.min_x, bb.min_y, bb.min_z, bb.max_x, bb.max_y, bb.max_z]
                },
                player_chunk_loaded: self.level.as_ref().map(|level| {
                    level.has_chunk_at(BlockPos::new(
                        pos.x.floor() as i32,
                        pos.y.floor() as i32,
                        pos.z.floor() as i32,
                    ))
                }),
                loaded_chunks: self
                    .level
                    .as_ref()
                    .map_or(0, |level| level.loaded_chunk_count()),
            });
    }
    pub(crate) fn send_local_move_if_needed(&mut self, is_controlled_camera: bool) -> bool {
        if !is_controlled_camera || self.connection.is_none() {
            return false;
        }
        self.send_local_player_input_if_changed();
        let exact_pos = self.player.exact_position();
        let yaw_degrees = self.player.y_rot_degrees();
        let pitch_degrees = self.player.x_rot_degrees();
        let on_ground = self.player.on_ground;
        let horizontal_collision = self.player.horizontal_collision;
        let action = self.local_move_send_state.get_send_action(
            exact_pos,
            yaw_degrees,
            pitch_degrees,
            on_ground,
            horizontal_collision,
        );
        let connection = self
            .connection
            .as_mut()
            .expect("connection checked before LocalPlayer.sendPosition");
        match action {
            MoveSendAction::None => {}
            MoveSendAction::PosRot => {
                let _ = connection.send_packet(
                    &el_protocol::packets::game::ServerboundMovePlayerPosRotPacket {
                        x: exact_pos.x,
                        y: exact_pos.y,
                        z: exact_pos.z,
                        y_rot: yaw_degrees,
                        x_rot: pitch_degrees,
                        on_ground,
                        horizontal_collision,
                    },
                );
            }
            MoveSendAction::Pos => {
                let _ = connection.send_packet(
                    &el_protocol::packets::game::ServerboundMovePlayerPosPacket {
                        x: exact_pos.x,
                        y: exact_pos.y,
                        z: exact_pos.z,
                        on_ground,
                        horizontal_collision,
                    },
                );
            }
            MoveSendAction::Rot => {
                let _ = connection.send_packet(
                    &el_protocol::packets::game::ServerboundMovePlayerRotPacket {
                        y_rot: yaw_degrees,
                        x_rot: pitch_degrees,
                        on_ground,
                        horizontal_collision,
                    },
                );
            }
            MoveSendAction::StatusOnly => {
                let _ = connection.send_packet(
                    &el_protocol::packets::game::ServerboundMovePlayerStatusOnlyPacket {
                        on_ground,
                        horizontal_collision,
                    },
                );
            }
        }
        action != MoveSendAction::None
    }
    pub(crate) fn send_local_player_input_if_changed(&mut self) {
        let input = self.player.input.key_presses.clone();
        if self.remote_last_sent_input.as_ref() == Some(&input) {
            return;
        }
        let Some(connection) = self.connection.as_mut() else {
            return;
        };
        let packet = el_protocol::packets::game::ServerboundPlayerInputPacket {
            forward: input.forward,
            backward: input.backward,
            left: input.left,
            right: input.right,
            jump: input.jump,
            shift: input.sneak,
            sprint: input.sprint,
        };
        if connection.send_packet(&packet).is_ok() {
            self.remote_last_sent_input = Some(input);
        }
    }
    pub(crate) fn send_remote_player_input_if_changed(
        packet_tx: &PacketSender,
        input: &PlayerInput,
        last_sent_input: &mut Option<PlayerInput>,
    ) {
        if last_sent_input.as_ref() == Some(input) {
            return;
        }
        Self::send_remote_player_input(packet_tx, input);
        *last_sent_input = Some(input.clone());
    }
    pub(crate) fn send_remote_sprinting_if_needed(
        packet_tx: &PacketSender,
        entity_id: Option<i32>,
        is_sprinting: bool,
        was_sprinting: &mut bool,
    ) {
        if is_sprinting == *was_sprinting {
            return;
        }
        if let Some(entity_id) = entity_id {
            let action = if is_sprinting {
                "start_sprinting"
            } else {
                "stop_sprinting"
            };
            packet_tx.send_player_command(entity_id, action, 0);
            *was_sprinting = is_sprinting;
            if DEBUG_MOVE_AND_CHUNKS || movement_diagnostics_enabled() {
                println!(
                    "[MovementDebug] outbound command action={} entity_id={}",
                    action, entity_id
                );
            }
        }
    }
    pub(crate) fn send_remote_player_input(packet_tx: &PacketSender, input: &PlayerInput) {
        packet_tx.send_player_input(
            input.forward,
            input.backward,
            input.left,
            input.right,
            input.jump,
            input.sneak,
            input.sprint,
        );
    }
}
