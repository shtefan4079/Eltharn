use super::ClientSession;
use crate::gameplay::entity::remote::*;
use crate::gameplay::player::LocalPlayer;
use crate::input::KeyAction;
use el_math::AABB;
use std::collections::{HashMap, HashSet};
#[derive(Clone, Copy, Debug, Default)]
pub struct InputOptions {
    pub toggle_crouch: bool,
    pub toggle_sprint: bool,
    pub toggle_attack: bool,
    pub toggle_use: bool,
}
#[derive(Clone, Debug, Default)]
pub struct RemoteEntityPushPolicy {
    pub allowed_entity_ids: Option<HashSet<i32>>,
    pub tick_eligible_entity_ids: Option<HashSet<i32>>,
}
fn team_rules_allow_entity_push(
    own_team: Option<(&str, &str)>,
    local_team: Option<(&str, &str)>,
) -> bool {
    let own_rule = own_team.map_or("always", |(_, rule)| rule);
    let local_rule = local_team.map_or("always", |(_, rule)| rule);
    if own_rule == "never" || local_rule == "never" {
        return false;
    }
    let same_team =
        own_team.is_some_and(|(own, _)| local_team.is_some_and(|(local, _)| own == local));
    if (own_rule == "push_own_team" || local_rule == "push_own_team") && same_team {
        return false;
    }
    (own_rule != "push_other_teams" && local_rule != "push_other_teams") || same_team
}
impl RemoteEntityPushPolicy {
    pub fn from_scoreboard(
        local_name: &str,
        local_entity_id: Option<i32>,
        entities: &RemoteEntities,
        scoreboard: &crate::ui::scoreboard::Scoreboard,
        player_infos: &HashMap<uuid::Uuid, crate::ui::scoreboard::PlayerInfoState>,
    ) -> Self {
        let team = |name: &str| {
            scoreboard
                .get_players_team(name)
                .map(|team| (team.name.as_str(), team.collision_rule.as_str()))
        };
        let local_team = team(local_name);
        let allowed = entities
            .values()
            .filter(|entity| Some(entity.id) != local_entity_id)
            .filter(|entity| {
                if is_remote_player_entity(&entity.kind)
                    && entity
                        .uuid
                        .and_then(|id| player_infos.get(&id))
                        .is_some_and(|info| {
                            info.game_mode == crate::gameplay::game_mode::GameType::Spectator
                        })
                {
                    return false;
                }
                let score_name = if is_remote_player_entity(&entity.kind) {
                    entity.profile_name.clone().or_else(|| {
                        entity
                            .uuid
                            .and_then(|id| player_infos.get(&id))
                            .map(|info| info.name.clone())
                    })
                } else {
                    entity.uuid.map(|id| id.to_string())
                };
                team_rules_allow_entity_push(score_name.as_deref().and_then(team), local_team)
            })
            .map(|entity| entity.id)
            .collect();
        Self {
            allowed_entity_ids: Some(allowed),
            tick_eligible_entity_ids: None,
        }
    }
}
impl ClientSession {
    pub(crate) fn refresh_swing_animation_like_java(
        &mut self,
        hand: crate::render::model::player::player_model::HandSide,
        main_hand: crate::render::model::player::player_model::HandSide,
    ) {
        let item_slot = if hand == main_hand {
            self.active_hotbar_slot
        } else {
            40
        };
        let Some(stack) = self.player.inventory.get(item_slot) else {
            return;
        };
        let Some(swing_animation) = stack.swing_animation() else {
            return;
        };
        let Some(swing_duration) = self.player.current_swing_duration(stack) else {
            return;
        };
        self.swing_duration = swing_duration;
        self.swing_animation_type = swing_animation.animation_type;
    }
    pub(crate) fn start_swing_for_hand(
        &mut self,
        hand: crate::render::model::player::player_model::HandSide,
        main_hand: crate::render::model::player::player_model::HandSide,
    ) {
        self.refresh_swing_animation_like_java(hand, main_hand);
        if !self.swinging || self.swing_time >= self.swing_duration / 2 || self.swing_time < 0 {
            self.swing_time = -1;
            self.swinging = true;
            self.swinging_hand = hand;
        }
    }
    pub(crate) fn tick_attack_animation_once(
        &mut self,
        main_hand: crate::render::model::player::player_model::HandSide,
    ) {
        self.o_attack_anim = self.attack_anim;
        let duration_hand = if self.swinging {
            self.swinging_hand
        } else {
            main_hand
        };
        self.refresh_swing_animation_like_java(duration_hand, main_hand);
        if self.swinging {
            self.swing_time += 1;
            if self.swing_time >= self.swing_duration {
                self.swing_time = 0;
                self.swinging = false;
            }
        } else {
            self.swing_time = 0;
        }
        self.attack_anim = self.swing_time as f32 / self.swing_duration as f32;
    }
    pub(crate) fn local_player_hands_busy_like_java(&self) -> bool {
        let Some(vehicle_id) = self.remote_vehicle_entity_id else {
            return false;
        };
        self.remote_boat_input_latch
            .filter(|latch| latch.vehicle_id == vehicle_id)
            .is_some_and(|latch| latch.left || latch.right || latch.forward || latch.backward)
    }
    pub(crate) fn finish_handle_keybinds_miss_time(&mut self, screen_active: bool) {
        if screen_active {
            return;
        }
        if !self.is_left_mouse_pressed {
            self.miss_time = 0;
        }
        if self.miss_time > 0 {
            self.miss_time -= 1;
        }
    }
    pub fn tick_screen_player_travel(&mut self, vehicle_id: Option<i32>) -> bool {
        let Some(level) = self.level.as_ref() else {
            return false;
        };
        let player_on_climbable = self.player.on_climbable(level);
        let hard_colliders = collect_hard_entity_colliders(
            &self.remote_entities,
            self.remote_player_entity_id,
            vehicle_id,
        );
        if self.player_flying && vehicle_id.is_none() {
            self.player.is_passenger = false;
            crate::gameplay::player::correction::tick_player_travel(
                &mut self.player,
                level,
                &self.player_input,
                &hard_colliders,
                self.game_mode.abilities().mayfly,
                Some(self.game_mode.abilities().flying_speed),
            );
            if self.player.on_ground && self.player_flying && !self.game_mode.is_spectator() {
                self.game_mode.set_flying(false);
                self.reconcile_player_flying_from_game_mode(true);
            }
        } else {
            crate::gameplay::player::correction::tick_player_travel(
                &mut self.player,
                level,
                &self.player_input,
                &hard_colliders,
                self.game_mode.abilities().mayfly,
                None,
            );
        }
        push_remote_entities_into_local_player_with_policy_like_java(
            &mut self.player,
            player_on_climbable,
            &self.remote_entities,
            self.remote_player_entity_id,
            vehicle_id,
            &self.remote_push_policy,
        );
        self.tick_jump_trigger_time_after_player_ai_step_like_java();
        true
    }
    pub fn tick_flying_player_travel(&mut self, flying_speed: f32) -> bool {
        let Some(level) = self.level.as_ref() else {
            return false;
        };
        let player_on_climbable = self.player.on_climbable(level);
        let hard_colliders = collect_hard_entity_colliders(
            &self.remote_entities,
            self.remote_player_entity_id,
            None,
        );
        self.player.is_passenger = false;
        self.player
            .set_attack_anim_for_living_tick(self.attack_anim);
        if self.player_flying {
            crate::gameplay::player::correction::tick_player_travel(
                &mut self.player,
                level,
                &self.player_input,
                &hard_colliders,
                self.game_mode.abilities().mayfly,
                Some(flying_speed),
            );
        } else {
            crate::gameplay::player::correction::tick_player_travel(
                &mut self.player,
                level,
                &self.player_input,
                &hard_colliders,
                self.game_mode.abilities().mayfly,
                None,
            );
        }
        push_remote_entities_into_local_player_with_policy_like_java(
            &mut self.player,
            player_on_climbable,
            &self.remote_entities,
            self.remote_player_entity_id,
            None,
            &self.remote_push_policy,
        );
        self.tick_jump_trigger_time_after_player_ai_step_like_java();
        self.player.tick_attack_strength(self.active_hotbar_slot);
        true
    }
    pub fn tick_walking_player_travel(&mut self, passenger_vehicle_id: Option<i32>) -> bool {
        let Some(level) = self.level.as_ref() else {
            return false;
        };
        let player_on_climbable = self.player.on_climbable(level);
        self.player.is_passenger = passenger_vehicle_id.is_some();
        self.player
            .set_attack_anim_for_living_tick(self.attack_anim);
        let hard_colliders = collect_hard_entity_colliders(
            &self.remote_entities,
            self.remote_player_entity_id,
            passenger_vehicle_id,
        );
        if self.player_flying && passenger_vehicle_id.is_none() {
            self.player.is_passenger = false;
            crate::gameplay::player::correction::tick_player_travel(
                &mut self.player,
                level,
                &self.player_input,
                &hard_colliders,
                self.game_mode.abilities().mayfly,
                Some(self.game_mode.abilities().flying_speed),
            );
            if self.player.on_ground && self.player_flying && !self.game_mode.is_spectator() {
                self.game_mode.set_flying(false);
                self.reconcile_player_flying_from_game_mode(true);
            }
        } else {
            crate::gameplay::player::correction::tick_player_travel(
                &mut self.player,
                level,
                &self.player_input,
                &hard_colliders,
                self.game_mode.abilities().mayfly,
                None,
            );
        }
        push_remote_entities_into_local_player_with_policy_like_java(
            &mut self.player,
            player_on_climbable,
            &self.remote_entities,
            self.remote_player_entity_id,
            passenger_vehicle_id,
            &self.remote_push_policy,
        );
        self.tick_jump_trigger_time_after_player_ai_step_like_java();
        self.player.tick_attack_strength(self.active_hotbar_slot);
        true
    }
    pub fn sync_player_input_from_key_mappings(&mut self, options: InputOptions) {
        self.player_input.forward = self.controls.is_down(KeyAction::Forward);
        self.player_input.backward = self.controls.is_down(KeyAction::Back);
        self.player_input.left = self.controls.is_down(KeyAction::Left);
        self.player_input.right = self.controls.is_down(KeyAction::Right);
        self.player_input.jump = self.controls.is_down(KeyAction::Jump);
        self.player_input.sneak = if options.toggle_crouch {
            self.toggled_crouch_down
        } else {
            self.controls.is_down(KeyAction::Sneak)
        };
        self.player_input.sprint = if options.toggle_sprint {
            self.toggled_sprint_down
        } else {
            self.controls.is_down(KeyAction::Sprint)
        };
        self.is_left_mouse_pressed = if options.toggle_attack {
            self.toggled_attack_down
        } else {
            self.controls.is_down(KeyAction::Attack)
        };
        self.is_use_pressed = if options.toggle_use {
            self.toggled_use_down
        } else {
            self.controls.is_down(KeyAction::Use)
        };
    }
    pub fn tick_remote_entities(
        &mut self,
        runs_normally: bool,
        controlled_vehicle_id: Option<i32>,
    ) -> RemoteEntitiesTickOutput {
        let Some(level) = self.level.as_ref() else {
            return RemoteEntitiesTickOutput::default();
        };
        let locally_controlled_boat = controlled_vehicle_id.filter(|vehicle_id| {
            self.remote_entities.get(vehicle_id).is_some_and(|vehicle| {
                is_remote_abstract_boat(&vehicle.kind)
                    && remote_abstract_boat_is_locally_controlled(
                        vehicle,
                        self.remote_player_entity_id,
                    )
            })
        });
        let frozen_entity_ids: Option<HashSet<i32>> = if !runs_normally {
            Some(
                self.remote_entities
                    .iter()
                    .filter_map(|(&id, entity)| {
                        let frozen = tick_rate_manager_is_entity_frozen(
                            runs_normally,
                            remote_entity_type_key_eq(&entity.kind, "player"),
                            remote_entity_has_player_passenger(
                                id,
                                self.remote_player_entity_id,
                                &self.remote_entities,
                            ),
                        );
                        frozen.then_some(id)
                    })
                    .collect(),
            )
        } else {
            None
        };
        self.remote_push_policy.tick_eligible_entity_ids = Some(
            self.remote_entities
                .iter()
                .filter_map(|(id, entity)| {
                    (Some(*id) != locally_controlled_boat
                        && remote_entity_is_ticking(level, entity)
                        && !frozen_entity_ids
                            .as_ref()
                            .is_some_and(|frozen| frozen.contains(id)))
                    .then_some(*id)
                })
                .collect(),
        );
        let local_player_pos_and_eye = Some((
            self.player.exact_position().as_vec3(),
            self.player.pose_eye_height(false),
        ));
        self.remote_entities.tick_all(
            Some(level),
            locally_controlled_boat,
            frozen_entity_ids.as_ref(),
            local_player_pos_and_eye,
        )
    }
}
pub(crate) fn remote_entity_has_no_entity_push_like_java(kind: &str) -> bool {
    matches!(
        remote_entity_type_key(kind),
        "bat" | "armor_stand" | "mannequin"
    )
}
pub(crate) fn push_remote_entities_into_local_player_like_java(
    player: &mut LocalPlayer,
    player_on_climbable: bool,
    remote_entities: &HashMap<i32, RemoteEntityRenderState>,
    local_player_entity_id: Option<i32>,
    local_direct_vehicle_id: Option<i32>,
) {
    push_remote_entities_into_local_player_with_policy_like_java(
        player,
        player_on_climbable,
        remote_entities,
        local_player_entity_id,
        local_direct_vehicle_id,
        &RemoteEntityPushPolicy::default(),
    );
}
pub(crate) fn push_remote_entities_into_local_player_with_policy_like_java(
    player: &mut LocalPlayer,
    player_on_climbable: bool,
    remote_entities: &HashMap<i32, RemoteEntityRenderState>,
    local_player_entity_id: Option<i32>,
    local_direct_vehicle_id: Option<i32>,
    policy: &RemoteEntityPushPolicy,
) {
    if player.no_physics
        || player_on_climbable
        || player.is_dead_or_dying()
        || player.sleeping_position().is_some()
    {
        return;
    }
    let player_aabb = player.get_aabb();
    let player_position = player.exact_position();
    let local_root_vehicle = local_direct_vehicle_id
        .map(|vehicle_id| remote_root_vehicle_id(vehicle_id, remote_entities));
    for entity in remote_entities.values() {
        if local_player_entity_id == Some(entity.id)
            || !is_remote_living_entity_type(&entity.kind)
            || remote_entity_has_no_entity_push_like_java(&entity.kind)
            || policy
                .allowed_entity_ids
                .as_ref()
                .is_some_and(|allowed| !allowed.contains(&entity.id))
            || policy
                .tick_eligible_entity_ids
                .as_ref()
                .is_some_and(|eligible| !eligible.contains(&entity.id))
        {
            continue;
        }
        if local_root_vehicle
            .is_some_and(|root| remote_root_vehicle_id(entity.id, remote_entities) == root)
        {
            continue;
        }
        let (width, height) = remote_entity_pick_size(entity);
        let half = f64::from(width) * 0.5;
        let entity_aabb = AABB::new(
            entity.position_d.x - half,
            entity.position_d.y,
            entity.position_d.z - half,
            entity.position_d.x + half,
            entity.position_d.y + f64::from(height),
            entity.position_d.z + half,
        );
        if !entity_aabb.intersects(&player_aabb) {
            continue;
        }
        let mut xa = entity.position_d.x - player_position.x;
        let mut za = entity.position_d.z - player_position.z;
        let mut dd = f64::max(xa.abs(), za.abs());
        if dd < f64::from(0.01_f32) {
            continue;
        }
        dd = dd.sqrt();
        xa /= dd;
        za /= dd;
        let mut power = 1.0 / dd;
        if power > 1.0 {
            power = 1.0;
        }
        xa *= power;
        za *= power;
        xa *= f64::from(0.05_f32);
        za *= f64::from(0.05_f32);
        let velocity = player.exact_velocity();
        player.set_velocity_d(glam::DVec3::new(
            velocity.x - xa,
            velocity.y,
            velocity.z - za,
        ));
    }
}
pub(crate) fn tick_rate_manager_is_entity_frozen(
    runs_normally: bool,
    entity_is_player: bool,
    has_player_passenger: bool,
) -> bool {
    !runs_normally && !entity_is_player && !has_player_passenger
}
