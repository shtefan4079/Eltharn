use super::ClientSession;
use crate::gameplay::game_mode::Abilities;
use el_protocol_backend::BackendNetworkEvent;
pub(crate) const DEBUG_VERBOSE_NETWORK: bool = false;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerStateEffect {
    Applied,
}
impl ClientSession {
    pub(crate) fn apply_local_no_gravity_metadata(&mut self, id: i32, value: Option<bool>) {
        if Some(id) == self.remote_player_entity_id {
            if let Some(value) = value {
                self.player.no_gravity = value;
            }
        }
    }
    pub fn apply_player_state_event(
        &mut self,
        event: BackendNetworkEvent,
    ) -> Result<PlayerStateEffect, BackendNetworkEvent> {
        match event {
            BackendNetworkEvent::Health {
                health,
                food,
                saturation,
            } => {
                self.player.hurt_to(health);
                self.player.food = food as i32;
                self.player.food_saturation = saturation;
                if health > 0.0 {
                    self.remote_respawn_request_sent = false;
                }
            }
            BackendNetworkEvent::Experience {
                progress,
                level,
                total: _,
            } => {
                if progress != self.player.xp_progress {
                    self.experience_display_start_tick =
                        if self.experience_display_start_tick == i64::MIN {
                            -2_147_483_647
                        } else {
                            self.logical_client_tick_sequence as i64
                        };
                }
                self.player.xp_progress = progress;
                self.player.xp_level = level as i32;
            }
            BackendNetworkEvent::Abilities {
                invulnerable,
                flying,
                mayfly,
                instabuild,
                flying_speed,
                walking_speed,
            } => {
                if DEBUG_VERBOSE_NETWORK {
                    println!(
                        "[Network] Remote abilities: invulnerable={invulnerable}, flying={flying}, mayfly={mayfly}, instabuild={instabuild}, flying_speed={flying_speed:.3}, walking_speed={walking_speed:.3}, mode={}",
                        self.game_mode.local_player_mode().name()
                    );
                }
                self.game_mode.apply_server_abilities(Abilities {
                    invulnerable,
                    flying,
                    mayfly,
                    instabuild,
                    may_build: !self
                        .game_mode
                        .local_player_mode()
                        .is_block_placing_restricted(),
                    flying_speed,
                    walking_speed,
                });
                self.player_flying = self.game_mode.abilities().flying;
            }
            BackendNetworkEvent::GameModeChanged { game_mode } => {
                let mode = crate::gameplay::game_mode::GameType::by_id(game_mode as i32);
                if DEBUG_VERBOSE_NETWORK {
                    println!(
                        "[Network] Remote game mode changed: id={game_mode}, mode={}",
                        mode.name()
                    );
                }
                self.game_mode.set_local_mode(mode);
                self.reconcile_player_flying_from_game_mode(false);
            }
            BackendNetworkEvent::UpdateAttributes {
                entity_id,
                attributes,
            } => {
                for snap in attributes {
                    let modifiers = snap
                        .modifiers
                        .into_iter()
                        .filter_map(|(id, amount, op)| {
                            crate::gameplay::attributes::AttributeModifierOperation::from_id(op)
                                .map(|operation| crate::gameplay::attributes::AttributeModifier {
                                    id,
                                    amount,
                                    operation,
                                })
                        })
                        .collect();
                    if Some(entity_id) == self.remote_player_entity_id {
                        let _ = self.player.apply_attribute_snapshot(
                            &snap.attribute_id,
                            snap.base,
                            modifiers,
                        );
                    }
                }
            }
            BackendNetworkEvent::UpdateMobEffect {
                entity_id,
                effect_id,
                amplifier,
                duration,
                ambient,
                show_particles,
                show_icon,
                blend,
            } => {
                if Some(entity_id) == self.remote_player_entity_id {
                    self.player.active_effects.insert(
                        effect_id,
                        crate::gameplay::player::local_player::ActiveMobEffect {
                            amplifier: amplifier.clamp(0, 255),
                            duration,
                            ambient,
                            show_particles,
                            show_icon,
                            blend,
                        },
                    );
                }
            }
            BackendNetworkEvent::RemoveMobEffect {
                entity_id,
                effect_id,
            } => {
                if Some(entity_id) == self.remote_player_entity_id {
                    self.player.active_effects.remove(&effect_id);
                }
            }
            event => return Err(event),
        }
        Ok(PlayerStateEffect::Applied)
    }
}
