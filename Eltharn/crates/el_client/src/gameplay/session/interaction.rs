use super::ClientSession;
use crate::network::protocol_backend::RemotePacketSender as PacketSender;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackClickDecision {
    SuppressedByMissTime,
    DrainQueueWhileUsing,
    HandsBusy,
    Spectator,
    RejectedCharge,
    Piercing,
    Block,
    Entity,
    Miss,
}
pub struct AttackClickGuards {
    pub miss_time: i32,
    pub is_using_item: bool,
    pub hands_busy: bool,
    pub is_spectator: bool,
    pub charge_rejected: bool,
    pub has_piercing_weapon: bool,
    pub has_block_target: bool,
    pub target_entity: Option<i32>,
}
pub fn classify_attack_click(guards: &AttackClickGuards) -> AttackClickDecision {
    if guards.miss_time > 0 {
        return AttackClickDecision::SuppressedByMissTime;
    }
    if guards.is_using_item {
        return AttackClickDecision::DrainQueueWhileUsing;
    }
    if guards.hands_busy {
        return AttackClickDecision::HandsBusy;
    }
    if guards.is_spectator {
        return AttackClickDecision::Spectator;
    }
    if guards.charge_rejected {
        return AttackClickDecision::RejectedCharge;
    }
    if guards.has_piercing_weapon {
        return AttackClickDecision::Piercing;
    }
    if guards.has_block_target {
        return AttackClickDecision::Block;
    }
    if guards.target_entity.is_some() {
        return AttackClickDecision::Entity;
    }
    AttackClickDecision::Miss
}
impl ClientSession {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn start_block_attack_clicks_for_tick(
        game_mode: &mut crate::network::multiplayer::multiplayer_game_mode::MultiPlayerGameMode,
        level: &mut crate::world::client_level::ClientLevel,
        break_target_block: Option<el_math::block_pos::BlockPos>,
        block_attack_clicks: u32,
        crosshair_dir: el_math::Direction,
        player: &mut crate::gameplay::player::LocalPlayer,
        destroy_progress_entity_id: i32,
        active_hotbar_slot: usize,
        remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut crate::network::local_connection::LocalConnection>,
        seq: &mut u32,
        miss_time: &mut i32,
    ) -> (bool, bool) {
        let Some(pos) = break_target_block else {
            return (false, false);
        };
        let mut swung = false;
        let mut instant_attack = false;
        for _ in 0..block_attack_clicks {
            if *miss_time > 0 {
                continue;
            }
            if level.get_block_state(pos).is_known_vanilla_air() {
                player.reset_attack_strength_ticker();
                if game_mode.has_miss_time() {
                    *miss_time = 10;
                }
            } else {
                let _ = game_mode.start_destroy_block(
                    pos,
                    crosshair_dir,
                    player,
                    level,
                    destroy_progress_entity_id,
                    active_hotbar_slot,
                    remote_conn,
                    local_conn,
                    seq,
                );
                instant_attack |= level.get_block_state(pos).is_known_vanilla_air();
            }
            if let Some(remote) = remote_conn {
                remote.send_swing_main_hand();
            }
            swung = true;
        }
        (swung, instant_attack)
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn continue_block_action_for_tick(
        game_mode: &mut crate::network::multiplayer::multiplayer_game_mode::MultiPlayerGameMode,
        level: &mut crate::world::client_level::ClientLevel,
        break_target_block: Option<el_math::block_pos::BlockPos>,
        attack_down: bool,
        instant_attack: bool,
        crosshair_dir: el_math::Direction,
        player: &mut crate::gameplay::player::LocalPlayer,
        destroy_progress_entity_id: i32,
        active_hotbar_slot: usize,
        remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut crate::network::local_connection::LocalConnection>,
        seq: &mut u32,
        miss_time: &mut i32,
    ) -> bool {
        if !attack_down {
            *miss_time = 0;
        }
        if *miss_time > 0 || player.is_using_item {
            return false;
        }
        if player.inventory[active_hotbar_slot]
            .components
            .get("minecraft:piercing_weapon")
            .is_some()
        {
            return false;
        }
        if attack_down && !instant_attack {
            if let Some(pos) = break_target_block {
                if level.get_block_state(pos).is_known_vanilla_air() {
                    return false;
                }
                if game_mode.continue_destroy_block(
                    pos,
                    crosshair_dir,
                    player,
                    level,
                    destroy_progress_entity_id,
                    active_hotbar_slot,
                    remote_conn,
                    local_conn,
                    seq,
                ) {
                    if let Some(remote) = remote_conn {
                        remote.send_swing_main_hand();
                    }
                    return true;
                }
                return false;
            }
        }
        game_mode.stop_destroy_block(
            level,
            player,
            destroy_progress_entity_id,
            remote_conn,
            local_conn,
        );
        false
    }
}
