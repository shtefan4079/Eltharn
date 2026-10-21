#![allow(dead_code)]
pub use super::types::*;
use crate::client::state::{fluid_flow_like_java, java_normalize_or_zero};
use crate::gameplay::entity::{
    ClientAvatarEntity, ClientAvatarState, ParrotVariant, PlayerModelType, PlayerSkin,
};
use crate::gameplay::physics::collision;
use crate::gameplay::player::client_input::ClientInput;
use crate::input::PlayerInput;
use crate::world::client_level::ClientLevel;
use el_core::block_state::BlockState;
use el_core::collision_context::CollisionContext;
use el_math::block_pos::BlockPos;
use el_math::{AABB, Axis, AxisDirection, Direction, Vec3 as MathVec3};
use el_nbt::{Tag, compound_get};
use glam::{DVec3, Vec2, Vec3};
use std::collections::HashMap;
use std::sync::OnceLock;
pub struct LocalPlayer {
    pub position: Vec3,
    pub prev_position: Vec3,
    pub velocity: Vec3,
    exact_position: DVec3,
    exact_prev_position: DVec3,
    exact_velocity: DVec3,
    mirrored_position: Vec3,
    mirrored_prev_position: Vec3,
    mirrored_velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub prev_yaw: f32,
    pub prev_pitch: f32,
    java_y_rot_degrees: f32,
    java_x_rot_degrees: f32,
    java_y_rot_o_degrees: f32,
    java_x_rot_o_degrees: f32,
    pub prev_body_yaw: f32,
    pub body_yaw: f32,
    pub prev_head_yaw: f32,
    pub head_yaw: f32,
    target_head_yaw: f32,
    head_yaw_steps: i32,
    pub on_ground: bool,
    pub horizontal_collision: bool,
    pub minor_horizontal_collision: bool,
    pub is_sprinting: bool,
    pub synced_shared_flags: u8,
    pub no_gravity: bool,
    sprint_trigger_time: i32,
    sprint_window_ticks: i32,
    pub is_using_item: bool,
    pub attack_strength_ticker: i32,
    pub item_swap_ticker: i32,
    last_attack_item_id: Option<String>,
    attack_anim_for_living_tick: f32,
    movement_crouching: bool,
    entity_pose: PlayerPose,
    pub is_sneaking: bool,
    applied_input: Vec2,
    controlled_camera: bool,
    sleeping_pos: Option<BlockPos>,
    pub target_block: Option<BlockPos>,
    pub break_progress: f32,
    pub health: f32,
    pub absorption: f32,
    pub food: i32,
    pub food_saturation: f32,
    pub armor: i32,
    pub air: i32,
    pub xp_level: i32,
    pub xp_progress: f32,
    pub hurt_time: i32,
    pub hurt_duration: i32,
    pub invulnerable_time: i32,
    pub flash_on_set_health: bool,
    pub hurt_dir: f32,
    pub death_time: i32,
    pub inventory: Vec<el_core::item::ItemStack>,
    pub cooldowns: crate::gameplay::player::cooldowns::ItemCooldowns,
    pub prev_walk_animation_pos: f32,
    pub prev_walk_animation_speed: f32,
    pub walk_animation_pos: f32,
    pub walk_animation_speed: f32,
    pub prev_walk_dist: f32,
    pub walk_dist: f32,
    pub prev_x_bob: f32,
    pub x_bob: f32,
    pub prev_y_bob: f32,
    pub y_bob: f32,
    pub age_in_ticks: f32,
    pub o_bob: f32,
    pub bob: f32,
    pub is_in_water: bool,
    pub(crate) water_entry_debug_ticks: u8,
    fluid_on_eyes_has_water: bool,
    pub is_under_water: bool,
    pub water_vision_time: i32,
    pub pending_underwater_sound: Option<UnderwaterSoundCue>,
    pub is_in_lava: bool,
    pub prev_swim_amount: f32,
    pub swim_amount: f32,
    pub is_fall_flying: bool,
    pub pending_start_fall_flying: bool,
    pub auto_jump_enabled: bool,
    auto_jump_time: i32,
    just_toggled_creative_flight: bool,
    pub is_auto_spin_attack: bool,
    pub auto_spin_attack_ticks: i32,
    pub is_visually_swimming: bool,
    pub firework_boost_ticks: i32,
    pub no_physics: bool,
    pub avatar_state: ClientAvatarState,
    pub input: ClientInput,
    pub attributes: crate::gameplay::attributes::AttributeMap,
    no_jump_delay: i32,
    main_supporting_block_pos: Option<BlockPos>,
    on_ground_no_blocks: bool,
    pub fall_distance: f32,
    stuck_speed_multiplier: DVec3,
    piston_deltas: [f64; 3],
    piston_deltas_game_time: i64,
    pub active_effects: HashMap<u32, ActiveMobEffect>,
    pub ticks_frozen: i32,
    pub is_passenger: bool,
    pub controlling_passenger_is_player: bool,
    pub discard_friction: bool,
    pub is_vehicle: bool,
    pub omnidirectional_air_mover: bool,
    pub model_customisation: u8,
}
impl LocalPlayer {
    pub fn is_model_part_shown(&self, mask: u8) -> bool {
        (self.model_customisation & mask) == mask
    }
    pub fn movement_diagnostics_enabled() -> bool {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        *ENABLED.get_or_init(|| {
            diagnostic_probe_env_enabled(MOVEMENT_DIAGNOSTICS_ENV)
                || diagnostic_probe_env_enabled("ELTHARN_DIAGNOSTICS")
                || diagnostic_probe_env_enabled("ELTHARN_NOISY_DEBUG")
        })
    }
    pub fn slide_probe_enabled() -> bool {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        *ENABLED.get_or_init(|| {
            Self::movement_diagnostics_enabled() || diagnostic_probe_env_enabled(SLIDE_PROBE_ENV)
        })
    }
    fn slide_probe_file_enabled() -> bool {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        *ENABLED.get_or_init(|| {
            Self::slide_probe_enabled() && el_protocol_backend::diagnostics::file_enabled()
        })
    }
    pub fn slide_probe_log(line: impl AsRef<str>) {
        let file_enabled = Self::slide_probe_file_enabled();
        let console_enabled = el_protocol_backend::diagnostics::console_enabled();
        if !file_enabled && !console_enabled {
            return;
        }
        let line = line.as_ref();
        if console_enabled {
            std::println!("{line}");
        }
        if !file_enabled {
            return;
        }
        el_protocol_backend::diagnostics::write_line(line);
    }
    pub fn new(pos: Vec3) -> Self {
        let exact_pos = pos.as_dvec3();
        Self {
            position: pos,
            prev_position: pos,
            velocity: Vec3::ZERO,
            exact_position: exact_pos,
            exact_prev_position: exact_pos,
            exact_velocity: DVec3::ZERO,
            mirrored_position: pos,
            mirrored_prev_position: pos,
            mirrored_velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            prev_yaw: 0.0,
            prev_pitch: 0.0,
            java_y_rot_degrees: 0.0,
            java_x_rot_degrees: 0.0,
            java_y_rot_o_degrees: 0.0,
            java_x_rot_o_degrees: 0.0,
            prev_body_yaw: 0.0,
            body_yaw: 0.0,
            prev_head_yaw: 0.0,
            head_yaw: 0.0,
            target_head_yaw: 0.0,
            head_yaw_steps: 0,
            on_ground: false,
            horizontal_collision: false,
            minor_horizontal_collision: false,
            is_sprinting: false,
            synced_shared_flags: 0,
            no_gravity: false,
            sprint_trigger_time: 0,
            sprint_window_ticks: DEFAULT_SPRINT_WINDOW_TICKS,
            is_using_item: false,
            attack_strength_ticker: 0,
            item_swap_ticker: 0,
            last_attack_item_id: None,
            attack_anim_for_living_tick: 0.0,
            movement_crouching: false,
            entity_pose: PlayerPose::Standing,
            is_sneaking: false,
            applied_input: Vec2::ZERO,
            controlled_camera: true,
            sleeping_pos: None,
            target_block: None,
            break_progress: 0.0,
            health: 20.0,
            absorption: 0.0,
            food: 20,
            food_saturation: 5.0,
            armor: 0,
            air: 300,
            xp_level: 0,
            xp_progress: 0.0,
            hurt_time: 0,
            hurt_duration: 0,
            invulnerable_time: 0,
            flash_on_set_health: false,
            hurt_dir: 0.0,
            death_time: 0,
            inventory: (0..46).map(|_| el_core::item::ItemStack::empty()).collect(),
            cooldowns: crate::gameplay::player::cooldowns::ItemCooldowns::default(),
            prev_walk_animation_pos: 0.0,
            prev_walk_animation_speed: 0.0,
            walk_animation_pos: 0.0,
            walk_animation_speed: 0.0,
            prev_walk_dist: 0.0,
            walk_dist: 0.0,
            prev_x_bob: 0.0,
            x_bob: 0.0,
            prev_y_bob: 0.0,
            y_bob: 0.0,
            age_in_ticks: 0.0,
            o_bob: 0.0,
            bob: 0.0,
            is_in_water: false,
            water_entry_debug_ticks: 0,
            fluid_on_eyes_has_water: false,
            is_under_water: false,
            water_vision_time: 0,
            pending_underwater_sound: None,
            is_in_lava: false,
            prev_swim_amount: 0.0,
            swim_amount: 0.0,
            is_fall_flying: false,
            pending_start_fall_flying: false,
            auto_jump_enabled: true,
            auto_jump_time: 0,
            just_toggled_creative_flight: false,
            is_auto_spin_attack: false,
            auto_spin_attack_ticks: 0,
            is_visually_swimming: false,
            firework_boost_ticks: 0,
            no_physics: false,
            avatar_state: ClientAvatarState::new(),
            input: ClientInput::new(),
            attributes: crate::gameplay::attributes::AttributeMap::player_defaults(),
            no_jump_delay: 0,
            main_supporting_block_pos: None,
            on_ground_no_blocks: false,
            fall_distance: 0.0,
            stuck_speed_multiplier: DVec3::ZERO,
            piston_deltas: [0.0; 3],
            piston_deltas_game_time: i64::MIN,
            active_effects: HashMap::new(),
            ticks_frozen: 0,
            is_passenger: false,
            controlling_passenger_is_player: false,
            discard_friction: false,
            is_vehicle: false,
            omnidirectional_air_mover: false,
            model_customisation: 0x7F,
        }
    }
    pub fn current_pose(&self, _abilities_flying: bool) -> PlayerPose {
        self.entity_pose
    }
    #[inline]
    pub fn is_crouching_like_java(&self) -> bool {
        self.movement_crouching
    }
    pub fn item_cooldown_percent(&self, item: &el_core::item::ItemStack, partial_tick: f32) -> f32 {
        if item.is_empty() {
            return 0.0;
        }
        self.cooldowns.cooldown_percent(
            &crate::gameplay::player::cooldowns::ItemCooldowns::cooldown_group(item),
            partial_tick,
        )
    }
    pub fn is_visually_swimming(&self, _abilities_flying: bool) -> bool {
        self.entity_pose == PlayerPose::Swimming
            || (!self.is_fall_flying && self.entity_pose == PlayerPose::FallFlying)
    }
    #[inline]
    pub fn set_pose_from_server(&mut self, pose: PlayerPose) {
        self.entity_pose = pose;
    }
    pub fn pose_eye_height(&self, abilities_flying: bool) -> f32 {
        let pose = self.current_pose(abilities_flying);
        pose.eye_height() * self.dimensions_scale_for_pose(pose)
    }
    pub fn inventory_preview_bounding_box_height_like_java(&self, abilities_flying: bool) -> f32 {
        let pose = self.current_pose(abilities_flying);
        let scaled_height = self.dimensions_for_pose_like_java(pose).1;
        scaled_height / self.attr_f32(crate::gameplay::attributes::SCALE)
    }
    pub fn vehicle_attachment_y(&self, abilities_flying: bool) -> f32 {
        let pose = self.current_pose(abilities_flying);
        match pose {
            PlayerPose::Standing | PlayerPose::Crouching => {
                0.6 * self.dimensions_scale_for_pose(pose)
            }
            PlayerPose::Swimming
            | PlayerPose::FallFlying
            | PlayerPose::SpinAttack
            | PlayerPose::Sleeping
            | PlayerPose::Dying => 0.0,
        }
    }
    #[inline]
    pub fn has_effect(&self, effect_id: u32) -> bool {
        self.active_effects.contains_key(&effect_id)
    }
    #[inline]
    pub fn get_effect_amplifier(&self, effect_id: u32) -> Option<i32> {
        self.active_effects
            .get(&effect_id)
            .map(|effect| effect.amplifier)
    }
    pub fn current_swing_duration(&self, hand_stack: &el_core::item::ItemStack) -> Option<i32> {
        let swing_duration = hand_stack.swing_animation()?.duration;
        if self.has_effect(MOB_EFFECT_HASTE) || self.has_effect(MOB_EFFECT_CONDUIT_POWER) {
            let haste_amplifier = self.get_effect_amplifier(MOB_EFFECT_HASTE).unwrap_or(0);
            let conduit_amplifier = self
                .get_effect_amplifier(MOB_EFFECT_CONDUIT_POWER)
                .unwrap_or(0);
            Some(swing_duration - (1 + haste_amplifier.max(conduit_amplifier)))
        } else if let Some(amplifier) = self.get_effect_amplifier(MOB_EFFECT_MINING_FATIGUE) {
            Some(swing_duration + (1 + amplifier) * 2)
        } else {
            Some(swing_duration)
        }
    }
    pub fn format_effect_duration(duration: i32) -> String {
        if duration == -1 {
            return crate::ui::i18n::translate("effect.duration.infinite").to_string();
        }
        let total_seconds = duration.max(0) / 20;
        format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
    }
    pub fn can_glide(&self) -> bool {
        if self.on_ground || self.no_physics || self.is_passenger {
            return false;
        }
        if self.has_effect(MOB_EFFECT_LEVITATION) {
            return false;
        }
        for slot in [36usize, 37, 38, 39, 40] {
            if self.inventory.get(slot).is_some_and(|stack| {
                !stack.is_empty()
                    && (stack.item.location.path.contains("elytra")
                        || stack.item.location.path.contains("glider"))
            }) {
                return true;
            }
        }
        false
    }
    pub fn is_in_wall(&self, level: &ClientLevel) -> bool {
        if self.no_physics {
            return false;
        }
        let check_width = self.bb_width_like_java() * 0.8_f32;
        let eye_pos = DVec3::new(
            self.exact_position.x,
            self.exact_position.y + self.pose_eye_height(false) as f64,
            self.exact_position.z,
        );
        let eye_bb = AABB::new(
            eye_pos.x - check_width as f64 * 0.5,
            eye_pos.y - 1.0E-6 * 0.5,
            eye_pos.z - check_width as f64 * 0.5,
            eye_pos.x + check_width as f64 * 0.5,
            eye_pos.y + 1.0E-6 * 0.5,
            eye_pos.z + check_width as f64 * 0.5,
        );
        level.collides_with_suffocating_block(&eye_bb, &self.collision_context())
    }
    pub fn try_to_start_fall_flying(&mut self) -> bool {
        if !self.is_fall_flying && self.can_glide() && !self.is_in_water {
            self.is_fall_flying = true;
            self.pending_start_fall_flying = true;
            true
        } else {
            false
        }
    }
    pub fn stop_fall_flying(&mut self) {
        self.is_fall_flying = false;
    }
    pub fn apply_attribute_snapshot(
        &mut self,
        attribute_id: &str,
        base: f64,
        modifiers: Vec<crate::gameplay::attributes::AttributeModifier>,
    ) -> bool {
        self.attributes
            .apply_snapshot(attribute_id, base, modifiers)
    }
    pub fn attr_f32(&self, def: crate::gameplay::attributes::RangedAttribute) -> f32 {
        self.attributes.value_f32(def)
    }
    pub fn attr_f64(&self, def: crate::gameplay::attributes::RangedAttribute) -> f64 {
        self.attributes.value(def)
    }
    pub fn max_up_step(&self) -> f32 {
        let step = self.attr_f32(crate::gameplay::attributes::STEP_HEIGHT);
        if self.controlling_passenger_is_player {
            step.max(1.0)
        } else {
            step
        }
    }
    pub fn block_interaction_range(&self) -> f32 {
        self.attr_f32(crate::gameplay::attributes::BLOCK_INTERACTION_RANGE)
    }
    pub fn entity_interaction_range(&self) -> f32 {
        self.attr_f32(crate::gameplay::attributes::ENTITY_INTERACTION_RANGE)
    }
    pub fn reset_attack_strength_ticker(&mut self) {
        self.attack_strength_ticker = 0;
        self.item_swap_ticker = 0;
    }
    pub fn reset_only_attack_strength_ticker(&mut self) {
        self.attack_strength_ticker = 0;
    }
    pub fn tick_attack_strength(&mut self, selected_slot: usize) {
        self.attack_strength_ticker = self.attack_strength_ticker.wrapping_add(1);
        self.item_swap_ticker = self.item_swap_ticker.wrapping_add(1);
        let item_id = self
            .inventory
            .get(selected_slot)
            .filter(|stack| !stack.is_empty())
            .map(|stack| stack.item.location.to_string());
        if self.last_attack_item_id.as_ref() != item_id.as_ref() {
            self.reset_attack_strength_ticker();
            self.last_attack_item_id = item_id;
        }
    }
    pub fn set_attack_anim_for_living_tick(&mut self, attack_anim: f32) {
        self.attack_anim_for_living_tick = attack_anim;
    }
    pub fn tick_death(&mut self) {
        if self.health <= 0.0 {
            self.death_time += 1;
        } else {
            self.death_time = 0;
        }
    }
    pub const fn is_dead_or_dying(&self) -> bool {
        self.health <= 0.0
    }
    pub fn hurt_to(&mut self, new_health: f32) {
        let max_health = self.attr_f32(crate::gameplay::attributes::MAX_HEALTH);
        if self.flash_on_set_health {
            let dmg = self.health - new_health;
            if dmg <= 0.0 {
                self.health = new_health.clamp(0.0, max_health);
                if dmg < 0.0 {
                    self.invulnerable_time = 10;
                }
            } else {
                self.invulnerable_time = 20;
                self.health = new_health.clamp(0.0, max_health);
                self.hurt_duration = 10;
                self.hurt_time = self.hurt_duration;
            }
        } else {
            self.health = new_health.clamp(0.0, max_health);
            self.flash_on_set_health = true;
        }
    }
    pub fn reset_death_time(&mut self) {
        self.death_time = 0;
    }
    pub fn attack_strength_scale(&self, partial_tick: f32) -> f32 {
        ((self.attack_strength_ticker as f32 + partial_tick)
            / self.current_item_attack_strength_delay())
        .clamp(0.0, 1.0)
    }
    pub fn item_swap_scale(&self, partial_tick: f32) -> f32 {
        ((self.item_swap_ticker as f32 + partial_tick) / self.current_item_attack_strength_delay())
            .clamp(0.0, 1.0)
    }
    pub fn current_item_attack_strength_delay(&self) -> f32 {
        let attack_speed = self.attr_f64(crate::gameplay::attributes::ATTACK_SPEED);
        ((1.0 / attack_speed) * 20.0) as f32
    }
    pub fn cannot_attack_with_item(&self, item: &el_core::item::ItemStack, tolerance: i32) -> bool {
        let required_strength = item
            .components
            .get("minecraft:minimum_attack_charge")
            .and_then(|component| match component {
                el_core::item::DataComponent::Generic { value, .. } => match value {
                    Tag::Float(value) => Some(*value),
                    Tag::Double(value) => Some(*value as f32),
                    _ => None,
                },
                _ => None,
            })
            .unwrap_or(0.0);
        if required_strength <= 0.0 {
            return false;
        }
        (self.attack_strength_ticker.wrapping_add(tolerance) as f32)
            / self.current_item_attack_strength_delay()
            < required_strength
    }
    pub fn set_position(&mut self, pos: Vec3) {
        self.set_position_d(pos.as_dvec3());
    }
    pub fn set_position_d(&mut self, pos: DVec3) {
        self.exact_position = pos;
        self.exact_prev_position = pos;
        self.sync_public_from_exact();
    }
    pub fn set_position_from_vehicle_attachment_d(&mut self, pos: DVec3) {
        self.exact_position = pos;
        self.sync_public_from_exact();
    }
    pub fn finish_passenger_ride_tick(&mut self) {
        self.fall_distance = 0.0;
        self.avatar_state.reset_bob();
    }
    pub fn set_old_pos_and_rot(&mut self) {
        self.exact_prev_position = self.exact_position;
        self.copy_current_rotation_to_old_like_java();
        self.sync_public_from_exact();
    }
    #[inline]
    pub fn y_rot_degrees(&self) -> f32 {
        self.java_y_rot_degrees
    }
    #[inline]
    pub fn x_rot_degrees(&self) -> f32 {
        self.java_x_rot_degrees
    }
    #[inline]
    pub fn old_y_rot_degrees(&self) -> f32 {
        self.java_y_rot_o_degrees
    }
    #[inline]
    pub fn old_x_rot_degrees(&self) -> f32 {
        self.java_x_rot_o_degrees
    }
    pub fn set_y_rot_degrees_like_java(&mut self, y_rot: f32) {
        if y_rot.is_finite() {
            self.java_y_rot_degrees = y_rot;
            self.yaw = y_rot.to_radians();
        }
    }
    pub fn set_x_rot_degrees_like_java(&mut self, x_rot: f32) {
        if x_rot.is_finite() {
            let x_rot = (x_rot % 360.0).clamp(-90.0, 90.0);
            self.java_x_rot_degrees = x_rot;
            self.pitch = x_rot.to_radians();
        }
    }
    pub fn set_rotation_degrees_like_java(&mut self, y_rot: f32, x_rot: f32) {
        self.set_y_rot_degrees_like_java(y_rot);
        self.set_x_rot_degrees_like_java(x_rot);
    }
    pub fn set_old_rotation_degrees_raw_like_java(&mut self, y_rot: f32, x_rot: f32) {
        self.java_y_rot_o_degrees = y_rot;
        self.java_x_rot_o_degrees = x_rot;
        self.prev_yaw = y_rot.to_radians();
        self.prev_pitch = x_rot.to_radians();
    }
    pub fn copy_current_rotation_to_old_like_java(&mut self) {
        self.set_old_rotation_degrees_raw_like_java(
            self.java_y_rot_degrees,
            self.java_x_rot_degrees,
        );
    }
    pub fn look_at_like_java(&mut self, from_anchor: u8, target: DVec3) {
        let from = match from_anchor {
            1 => self.exact_position() + DVec3::Y * f64::from(self.pose_eye_height(false)),
            _ => self.exact_position(),
        };
        let xd = target.x - from.x;
        let yd = target.y - from.y;
        let zd = target.z - from.z;
        let sd = (xd * xd + zd * zd).sqrt();
        let x_rot = wrap_degrees_like_java((-(yd.atan2(sd) * 180.0 / std::f64::consts::PI)) as f32);
        let y_rot =
            wrap_degrees_like_java((zd.atan2(xd) * 180.0 / std::f64::consts::PI - 90.0) as f32);
        self.set_x_rot_degrees_like_java(x_rot);
        self.set_y_rot_degrees_like_java(y_rot);
        let x_rot = self.java_x_rot_degrees;
        let y_rot = self.java_y_rot_degrees;
        self.head_yaw = y_rot.to_radians();
        self.set_old_rotation_degrees_raw_like_java(y_rot, x_rot);
    }
    fn sync_java_rotation_from_public_if_externally_mutated(&mut self) {
        if self.yaw.to_bits() != self.java_y_rot_degrees.to_radians().to_bits() {
            self.java_y_rot_degrees = self.yaw.to_degrees();
        }
        if self.pitch.to_bits() != self.java_x_rot_degrees.to_radians().to_bits() {
            self.java_x_rot_degrees = self.pitch.to_degrees();
        }
        if self.prev_yaw.to_bits() != self.java_y_rot_o_degrees.to_radians().to_bits() {
            self.java_y_rot_o_degrees = self.prev_yaw.to_degrees();
        }
        if self.prev_pitch.to_bits() != self.java_x_rot_o_degrees.to_radians().to_bits() {
            self.java_x_rot_o_degrees = self.prev_pitch.to_degrees();
        }
    }
    pub fn set_sleeping_position_from_server(&mut self, sleeping_pos: BlockPos) {
        self.sleeping_pos = Some(sleeping_pos);
        self.exact_position = DVec3::new(
            sleeping_pos.x as f64 + 0.5,
            sleeping_pos.y as f64 + 0.6875,
            sleeping_pos.z as f64 + 0.5,
        );
        self.sync_public_from_exact();
    }
    pub fn clear_sleeping_position_from_server(&mut self) {
        self.sleeping_pos = None;
    }
    #[inline]
    pub fn sleeping_position(&self) -> Option<BlockPos> {
        self.sleeping_pos
    }
    pub fn set_position_with_old_position_d(&mut self, pos: DVec3, old_pos: DVec3) {
        self.exact_position = pos;
        self.exact_prev_position = old_pos;
        self.sync_public_from_exact();
    }
    pub fn exact_position(&self) -> DVec3 {
        self.exact_position
    }
    pub fn exact_old_position(&self) -> DVec3 {
        self.exact_prev_position
    }
    pub fn exact_velocity(&self) -> DVec3 {
        self.exact_velocity
    }
    pub fn set_velocity(&mut self, velocity: Vec3) {
        self.set_velocity_d(velocity.as_dvec3());
    }
    pub fn set_velocity_d(&mut self, velocity: DVec3) {
        self.exact_velocity = velocity;
        self.sync_public_from_exact();
    }
    pub fn reset_input_snapshot_like_java(&mut self) {
        self.input = ClientInput::new();
        self.movement_crouching = false;
        self.is_sneaking = false;
        self.applied_input = Vec2::ZERO;
    }
    pub fn set_controlled_camera_like_java(&mut self, controlled: bool) {
        self.controlled_camera = controlled;
    }
    pub fn reset_fluid_interaction_like_new_local_player(&mut self) {
        self.is_in_water = false;
        self.fluid_on_eyes_has_water = false;
        self.is_under_water = false;
        self.water_vision_time = 0;
        self.pending_underwater_sound = None;
        self.is_in_lava = false;
        self.prev_swim_amount = 0.0;
        self.swim_amount = 0.0;
    }
    pub fn reset_transient_respawn_state_like_new_local_player(&mut self) {
        self.is_sneaking = false;
        self.is_using_item = false;
        self.pending_start_fall_flying = false;
        self.auto_spin_attack_ticks = 0;
        self.firework_boost_ticks = 0;
        self.auto_jump_time = 0;
        self.sprint_trigger_time = 0;
        self.just_toggled_creative_flight = false;
        self.no_jump_delay = 0;
        self.main_supporting_block_pos = None;
        self.on_ground_no_blocks = false;
        self.stuck_speed_multiplier = DVec3::ZERO;
        self.piston_deltas = [0.0; 3];
        self.piston_deltas_game_time = i64::MIN;
        self.on_ground = false;
        self.horizontal_collision = false;
        self.minor_horizontal_collision = false;
        self.is_passenger = false;
        self.controlling_passenger_is_player = false;
        self.discard_friction = false;
        self.is_vehicle = false;
        self.omnidirectional_air_mover = false;
    }
    pub fn reset_synced_entity_data_like_new_local_player(&mut self) {
        self.synced_shared_flags = 0;
        self.no_gravity = false;
        self.entity_pose = PlayerPose::Standing;
        self.air = 300;
        self.is_sprinting = false;
        self.is_fall_flying = false;
        self.is_auto_spin_attack = false;
        self.is_visually_swimming = false;
        self.ticks_frozen = 0;
    }
    pub fn apply_synced_shared_flags(&mut self, flags: u8) {
        self.synced_shared_flags = flags;
        self.is_sprinting = flags & (1 << 3) != 0;
        self.is_visually_swimming = flags & (1 << 4) != 0;
        self.is_fall_flying = flags & (1 << 7) != 0;
    }
    pub fn is_invisible(&self) -> bool {
        self.synced_shared_flags & (1 << 5) != 0
    }
    pub fn is_fully_frozen(&self) -> bool {
        self.ticks_frozen >= 140
    }
    pub fn reset_pos_like_java(&mut self, level: &ClientLevel, min_y: i32, max_y: i32) -> bool {
        let before = self.exact_position;
        self.entity_pose = PlayerPose::Standing;
        let min_y = min_y as f64;
        let max_y = max_y as f64;
        let mut test_y = self.exact_position.y;
        while test_y > min_y && test_y <= max_y {
            self.set_position_d(DVec3::new(
                self.exact_position.x,
                test_y,
                self.exact_position.z,
            ));
            let context = self.collision_context();
            if level.no_block_collision_with_context(&self.get_aabb(), &context) {
                break;
            }
            test_y += 1.0;
        }
        self.set_velocity_d(DVec3::ZERO);
        self.set_x_rot_degrees_like_java(0.0);
        self.health = 20.0;
        self.reset_death_time();
        self.sync_public_from_exact();
        self.exact_position != before
    }
    fn sync_exact_from_public_if_externally_mutated(&mut self) {
        if self.position != self.mirrored_position {
            self.exact_position = self.position.as_dvec3();
            self.mirrored_position = self.position;
        }
        if self.prev_position != self.mirrored_prev_position {
            self.exact_prev_position = self.prev_position.as_dvec3();
            self.mirrored_prev_position = self.prev_position;
        }
        if self.velocity != self.mirrored_velocity {
            self.exact_velocity = self.velocity.as_dvec3();
            self.mirrored_velocity = self.velocity;
        }
    }
    pub(crate) fn sync_public_from_exact(&mut self) {
        self.position = self.exact_position.as_vec3();
        self.prev_position = self.exact_prev_position.as_vec3();
        self.velocity = self.exact_velocity.as_vec3();
        self.mirrored_position = self.position;
        self.mirrored_prev_position = self.prev_position;
        self.mirrored_velocity = self.velocity;
    }
    pub fn turn_view(&mut self, yaw_delta: f32, pitch_delta: f32) {
        self.sync_java_rotation_from_public_if_externally_mutated();
        self.turn_view_degrees(yaw_delta.to_degrees(), pitch_delta.to_degrees());
    }
    pub fn turn_view_degrees(&mut self, yaw_delta: f32, pitch_delta: f32) {
        self.set_y_rot_degrees_like_java(self.java_y_rot_degrees + yaw_delta);
        self.java_y_rot_o_degrees += yaw_delta;
        self.prev_yaw = self.java_y_rot_o_degrees.to_radians();
        self.set_x_rot_degrees_like_java(self.java_x_rot_degrees + pitch_delta);
        self.java_x_rot_o_degrees = (self.java_x_rot_o_degrees + pitch_delta).clamp(-90.0, 90.0);
        self.prev_pitch = self.java_x_rot_o_degrees.to_radians();
    }
    pub(crate) fn lerp_head_to(&mut self, target_head_yaw: f32, steps: i32) {
        self.target_head_yaw = target_head_yaw;
        self.head_yaw_steps = steps;
    }
    pub(crate) fn reset_lerped_head_rotation(&mut self) {
        self.target_head_yaw = self.head_yaw;
        self.head_yaw_steps = 0;
    }
    fn tick_lerped_head_rotation(&mut self) {
        if self.head_yaw_steps > 0 {
            self.head_yaw = rot_lerp(
                self.head_yaw,
                self.target_head_yaw,
                1.0 / self.head_yaw_steps as f32,
            );
            self.head_yaw_steps -= 1;
        }
    }
    pub fn clamp_rotation_to_abstract_boat(&mut self, boat_yaw: f32) {
        self.clamp_rotation_to_abstract_boat_degrees(boat_yaw.to_degrees());
    }
    pub fn clamp_rotation_to_abstract_boat_degrees(&mut self, boat_yaw_degrees: f32) {
        self.sync_java_rotation_from_public_if_externally_mutated();
        self.body_yaw = boat_yaw_degrees.to_radians();
        let delta = wrap_degrees_like_java(self.java_y_rot_degrees - boat_yaw_degrees);
        let target_delta = delta.clamp(-105.0, 105.0);
        let correction = target_delta - delta;
        self.java_y_rot_o_degrees += correction;
        self.prev_yaw = self.java_y_rot_o_degrees.to_radians();
        self.set_y_rot_degrees_like_java(self.java_y_rot_degrees + correction);
        self.head_yaw = self.yaw;
    }
    pub fn apply_abstract_boat_rider_rotation(&mut self, boat_yaw: f32, boat_delta_rotation: f32) {
        self.apply_abstract_boat_rider_rotation_degrees(
            boat_yaw.to_degrees(),
            boat_delta_rotation.to_degrees(),
        );
    }
    pub fn apply_abstract_boat_rider_rotation_degrees(
        &mut self,
        boat_yaw_degrees: f32,
        boat_delta_rotation_degrees: f32,
    ) {
        self.sync_java_rotation_from_public_if_externally_mutated();
        self.set_y_rot_degrees_like_java(self.java_y_rot_degrees + boat_delta_rotation_degrees);
        self.head_yaw += boat_delta_rotation_degrees.to_radians();
        self.clamp_rotation_to_abstract_boat_degrees(boat_yaw_degrees);
    }
    pub fn render_position_d(&self, partial_tick: f32) -> DVec3 {
        self.exact_prev_position
            .lerp(self.exact_position, partial_tick.clamp(0.0, 1.0) as f64)
    }
    pub fn render_position(&self, partial_tick: f32) -> Vec3 {
        self.render_position_d(partial_tick).as_vec3()
    }
    pub fn render_pitch(&self, partial_tick: f32) -> f32 {
        self.prev_pitch + (self.pitch - self.prev_pitch) * partial_tick.clamp(0.0, 1.0)
    }
    #[allow(dead_code)]
    pub fn render_yaw(&self, partial_tick: f32) -> f32 {
        rot_lerp(self.prev_yaw, self.yaw, partial_tick)
    }
    pub fn view_pitch_for_camera(&self) -> f32 {
        self.pitch
    }
    pub fn view_yaw_for_camera(&self, partial_tick: f32) -> f32 {
        if self.is_passenger {
            self.render_yaw(partial_tick)
        } else {
            self.yaw
        }
    }
    pub fn render_walk_animation_pos(&self, partial_tick: f32) -> f32 {
        self.walk_animation_pos - self.walk_animation_speed * (1.0 - partial_tick.clamp(0.0, 1.0))
    }
    pub fn render_walk_animation_speed(&self, partial_tick: f32) -> f32 {
        self.prev_walk_animation_speed
            + (self.walk_animation_speed - self.prev_walk_animation_speed)
                * partial_tick.clamp(0.0, 1.0)
    }
    pub fn render_body_yaw(&self, partial_tick: f32) -> f32 {
        rot_lerp(self.prev_body_yaw, self.body_yaw, partial_tick)
    }
    pub fn render_head_yaw(&self, partial_tick: f32) -> f32 {
        rot_lerp(self.prev_head_yaw, self.head_yaw, partial_tick)
    }
    pub fn render_x_bob(&self, partial_tick: f32) -> f32 {
        self.prev_x_bob + (self.x_bob - self.prev_x_bob) * partial_tick.clamp(0.0, 1.0)
    }
    pub fn render_y_bob(&self, partial_tick: f32) -> f32 {
        rot_lerp(self.prev_y_bob, self.y_bob, partial_tick)
    }
    pub fn render_bob(&self, partial_tick: f32) -> f32 {
        self.o_bob + (self.bob - self.o_bob) * partial_tick.clamp(0.0, 1.0)
    }
    pub fn render_backwards_walk_dist(&self, partial_tick: f32) -> f32 {
        let delta = self.walk_dist - self.prev_walk_dist;
        -(self.walk_dist + delta * partial_tick.clamp(0.0, 1.0))
    }
    pub fn get_aabb(&self) -> AABB {
        self.aabb_at_position(self.exact_position)
    }
    fn aabb_at_position_for_pose(&self, position: DVec3, pose: PlayerPose) -> AABB {
        let (width, height) = self.dimensions_for_pose_like_java(pose);
        let half_width = width / 2.0_f32;
        AABB::new(
            position.x - half_width as f64,
            position.y,
            position.z - half_width as f64,
            position.x + half_width as f64,
            position.y + height as f64,
            position.z + half_width as f64,
        )
    }
    fn aabb_at_position(&self, position: DVec3) -> AABB {
        self.aabb_at_position_for_pose(position, self.current_pose(false))
    }
    fn bb_width(&self) -> f64 {
        self.bb_width_like_java() as f64
    }
    #[inline]
    fn bb_width_like_java(&self) -> f32 {
        self.dimensions_for_pose_like_java(self.current_pose(false))
            .0
    }
    #[inline]
    fn dimensions_for_pose_like_java(&self, pose: PlayerPose) -> (f32, f32) {
        let scale = self.dimensions_scale_for_pose(pose);
        (
            pose.bb_width() as f32 * scale,
            pose.bb_height() as f32 * scale,
        )
    }
    fn dimensions_scale_for_pose(&self, pose: PlayerPose) -> f32 {
        if matches!(pose, PlayerPose::Sleeping | PlayerPose::Dying) {
            1.0
        } else {
            self.attr_f32(crate::gameplay::attributes::SCALE)
        }
    }
    fn calculate_keyboard_impulse(positive: bool, negative: bool) -> f32 {
        if positive == negative {
            0.0
        } else if positive {
            1.0
        } else {
            -1.0
        }
    }
    fn keyboard_move_vector(input: &PlayerInput) -> Vec2 {
        let forward_impulse = Self::calculate_keyboard_impulse(input.forward, input.backward);
        let left_impulse = Self::calculate_keyboard_impulse(input.left, input.right);
        let move_vector = Vec2::new(left_impulse, forward_impulse);
        let length = Self::mth_sqrt_f32_like_java(move_vector.length_squared());
        if length < 1.0e-4 {
            Vec2::ZERO
        } else {
            Vec2::new(move_vector.x / length, move_vector.y / length)
        }
    }
    #[inline]
    fn mth_sqrt_f32_like_java(value: f32) -> f32 {
        (value as f64).sqrt() as f32
    }
    fn is_slow_due_to_using_item(&self) -> bool {
        self.is_using_item
    }
    fn item_use_speed_multiplier(&self) -> f32 {
        0.2
    }
    fn modify_input_for_local_player(&self, raw_input: Vec2, moving_slowly: bool) -> Vec2 {
        if raw_input.length_squared() == 0.0 {
            return raw_input;
        }
        let mut input = raw_input * INPUT_FRICTION;
        if self.is_using_item && !self.is_passenger {
            input *= self.item_use_speed_multiplier();
        }
        if moving_slowly {
            input *= self.attr_f32(crate::gameplay::attributes::SNEAKING_SPEED);
        }
        Self::modify_input_speed_for_square_movement(input)
    }
    fn apply_input_like_java(&mut self, raw_input: Vec2, moving_slowly: bool) -> Vec2 {
        if self.controlled_camera {
            self.applied_input = self.modify_input_for_local_player(raw_input, moving_slowly);
        } else {
            self.applied_input *= INPUT_FRICTION;
        }
        self.applied_input
    }
    fn modify_input_speed_for_square_movement(input: Vec2) -> Vec2 {
        let length = Self::mth_sqrt_f32_like_java(input.length_squared());
        if length <= 0.0 {
            return input;
        }
        let direction = input * (1.0 / length);
        let distance_to_unit_square = Self::distance_to_unit_square(direction);
        let modified_length = (length * distance_to_unit_square).min(1.0);
        direction * modified_length
    }
    fn distance_to_unit_square(direction: Vec2) -> f32 {
        let direction_x = direction.x.abs();
        let direction_y = direction.y.abs();
        let tan = if direction_y > direction_x {
            direction_x / direction_y
        } else {
            direction_y / direction_x
        };
        Self::mth_sqrt_f32_like_java(1.0 + tan * tan)
    }
    fn relative_input_vector_like_java(&self, local_input: Vec2, speed: f32) -> DVec3 {
        let input = DVec3::new(local_input.x as f64, 0.0, local_input.y as f64);
        let length = input.length_squared();
        if length < 1.0e-7 {
            return DVec3::ZERO;
        }
        let movement = if length > 1.0 {
            let normalized = input / length.sqrt();
            normalized * speed as f64
        } else {
            input * speed as f64
        };
        let y_rot_in_radians = self.java_y_rot_degrees * (std::f32::consts::PI / 180.0);
        let sin_y = el_math::mth::sin(y_rot_in_radians) as f64;
        let cos_y = el_math::mth::cos(y_rot_in_radians) as f64;
        DVec3::new(
            movement.x * cos_y - movement.z * sin_y,
            movement.y,
            movement.z * cos_y + movement.x * sin_y,
        )
    }
    fn world_input_vector(&self, local_input: Vec2) -> DVec3 {
        let y_rot_in_radians = self.java_y_rot_degrees * (std::f32::consts::PI / 180.0);
        let sin_y = el_math::mth::sin(y_rot_in_radians) as f64;
        let cos_y = el_math::mth::cos(y_rot_in_radians) as f64;
        let local_x = local_input.x as f64;
        let local_z = local_input.y as f64;
        DVec3::new(
            local_x * cos_y - local_z * sin_y,
            0.0,
            local_z * cos_y + local_x * sin_y,
        )
    }
    fn is_horizontal_collision_minor(desired_world_input: DVec3, movement: DVec3) -> bool {
        const MINOR_COLLISION_INPUT_EPSILON: f32 = 1.0e-5;
        const MINOR_COLLISION_ANGLE_RADIANS: f32 = 0.139_626_34;
        let desired_len_sq = desired_world_input.x * desired_world_input.x
            + desired_world_input.z * desired_world_input.z;
        let movement_len_sq = movement.x * movement.x + movement.z * movement.z;
        if desired_len_sq < MINOR_COLLISION_INPUT_EPSILON as f64
            || movement_len_sq < MINOR_COLLISION_INPUT_EPSILON as f64
        {
            return false;
        }
        let dot = desired_world_input.x * movement.x + desired_world_input.z * movement.z;
        let denom = (desired_len_sq * movement_len_sq).sqrt();
        let angle = (dot / denom).acos();
        angle < MINOR_COLLISION_ANGLE_RADIANS as f64
    }
    fn update_horizontal_collision_state(
        &mut self,
        desired_world_input: DVec3,
        actual: DVec3,
        horizontal_collision: bool,
    ) {
        self.horizontal_collision = horizontal_collision;
        self.minor_horizontal_collision = if self.horizontal_collision {
            Self::is_horizontal_collision_minor(desired_world_input, actual)
        } else {
            false
        };
    }
    fn set_on_ground_with_movement(
        &mut self,
        level: &ClientLevel,
        on_ground: bool,
        horizontal_collision: bool,
        movement: DVec3,
    ) {
        if Self::slide_probe_enabled() && on_ground != self.on_ground {
            Self::slide_probe_log(format!(
                "[SlideProbe][Rust][set_on_ground] tick={} old_on_ground={} new_on_ground={} horizontal_collision={} movement=[{:.12},{:.12},{:.12}]",
                self.age_in_ticks,
                self.on_ground,
                on_ground,
                horizontal_collision,
                movement.x,
                movement.y,
                movement.z,
            ));
        }
        self.on_ground = on_ground;
        self.horizontal_collision = horizontal_collision;
        self.check_supporting_block(level, on_ground, Some(movement));
    }
    fn apply_move_position_and_ground(
        &mut self,
        level: &ClientLevel,
        collision: &MoveCollisionResult,
    ) {
        self.exact_position = collision.position;
        if !self.no_physics {
            if Self::slide_probe_enabled() {
                let attempted = collision.attempted_movement;
                let movement = collision.movement;
                Self::slide_probe_log(format!(
                    "[SlideProbe][Rust][before_set_on_ground] tick={} attempted_movement=[{:.12},{:.12},{:.12}] movement=[{:.12},{:.12},{:.12}] vertical_collision={} vertical_collision_below={} horizontal_collision={} next_on_ground={}",
                    self.age_in_ticks,
                    attempted.x,
                    attempted.y,
                    attempted.z,
                    movement.x,
                    movement.y,
                    movement.z,
                    collision.vertical_collision,
                    collision.vertical_collision_below,
                    collision.horizontal_collision,
                    collision.vertical_collision_below,
                ));
            }
            self.set_on_ground_with_movement(
                level,
                collision.vertical_collision_below,
                collision.horizontal_collision,
                collision.movement,
            );
        }
    }
    fn check_supporting_block(
        &mut self,
        level: &ClientLevel,
        on_ground: bool,
        movement: Option<DVec3>,
    ) {
        if on_ground {
            let bounding_box = self.get_aabb();
            let test_area = AABB::new(
                bounding_box.min_x,
                bounding_box.min_y - 1.0e-6,
                bounding_box.min_z,
                bounding_box.max_x,
                bounding_box.min_y,
                bounding_box.max_z,
            );
            let source_pos = el_math::Vec3::new(
                self.exact_position.x,
                self.exact_position.y,
                self.exact_position.z,
            );
            let context = self.collision_context();
            let mut supporting_block =
                level.find_supporting_block_with_context(source_pos, &test_area, &context);
            if supporting_block.is_some() || self.on_ground_no_blocks {
                self.main_supporting_block_pos = supporting_block;
            } else if let Some(movement) = movement {
                let on_ground_collision_test_area =
                    test_area.move_by(-movement.x, 0.0, -movement.z);
                supporting_block = level.find_supporting_block_with_context(
                    source_pos,
                    &on_ground_collision_test_area,
                    &context,
                );
                self.main_supporting_block_pos = supporting_block;
            }
            self.on_ground_no_blocks = supporting_block.is_none();
        } else {
            self.on_ground_no_blocks = false;
            if self.main_supporting_block_pos.is_some() {
                self.main_supporting_block_pos = None;
            }
        }
    }
    fn current_supporting_block(&self, level: &ClientLevel) -> Option<BlockPos> {
        let bounding_box = self.get_aabb();
        let test_area = AABB::new(
            bounding_box.min_x,
            bounding_box.min_y - 1.0e-6,
            bounding_box.min_z,
            bounding_box.max_x,
            bounding_box.min_y,
            bounding_box.max_z,
        );
        let source_pos = el_math::Vec3::new(
            self.exact_position.x,
            self.exact_position.y,
            self.exact_position.z,
        );
        let context = self.collision_context();
        level.find_supporting_block_with_context(source_pos, &test_area, &context)
    }
    fn check_fall_distance(&mut self, ya: f64, on_ground: bool) {
        if !self.is_in_water && ya < 0.0 {
            self.fall_distance -= ya as f32;
        }
        if on_ground {
            self.fall_distance = 0.0;
        }
    }
    fn update_fall_flying_like_java(&mut self) {
        if self.exact_velocity.y > -0.5 && self.fall_distance > 1.0 {
            self.fall_distance = 1.0;
        }
    }
    fn push_entities_like_java(&mut self, _before: &AABB, _now: &AABB) {}
    fn check_auto_spin_attack_like_java(&mut self, horizontal_collision: bool) {
        if horizontal_collision {
            self.auto_spin_attack_ticks = 0;
            self.is_auto_spin_attack = false;
        }
    }
    pub(crate) fn gravity_like_java(&self) -> f64 {
        if self.no_gravity {
            0.0
        } else {
            self.attr_f64(crate::gameplay::attributes::GRAVITY)
        }
    }
    pub(crate) fn effective_gravity(&self) -> f64 {
        let gravity = self.gravity_like_java();
        if self.exact_velocity.y <= 0.0 && self.has_effect(MOB_EFFECT_SLOW_FALLING) {
            gravity.min(0.01)
        } else {
            gravity
        }
    }
    fn debug_vertical_prediction_if_suspicious(
        &self,
        level: &ClientLevel,
        input: &PlayerInput,
        attempted_movement: DVec3,
        resolved_movement: DVec3,
        started_on_ground: bool,
        collision: &MoveCollisionResult,
    ) {
        let y_fraction = self.exact_position.y - self.exact_position.y.floor();
        let near_block_top = y_fraction.abs() < 1.0e-4 || (1.0 - y_fraction).abs() < 1.0e-4;
        let grounded_y_mismatch = near_block_top
            && !self.on_ground
            && attempted_movement.y <= 0.0
            && !self.is_in_water
            && !self.no_physics;
        let blocked_jump = input.jump && !self.on_ground && near_block_top;
        let wall_ground_mismatch = collision.horizontal_collision && grounded_y_mismatch;
        if !grounded_y_mismatch && !blocked_jump && !wall_ground_mismatch {
            return;
        }
        let support = self.current_supporting_block(level);
        let below_pos = self.block_pos_below_that_affects_movement(level);
        let feet_pos = self.block_position();
        let below_state = level.get_block_state(below_pos);
        let feet_state = level.get_block_state(feet_pos);
        let support_state = support.map(|pos| (pos, level.get_block_state(pos)));
        Self::slide_probe_log(format!(
            "[SlideProbe][Rust][suspicious_y] pos=[{:.5},{:.5},{:.5}] y_frac={:.6} started_on_ground={} on_ground={} horiz_collision={} minor_horiz={} attempted=[{:.5},{:.5},{:.5}] resolved=[{:.5},{:.5},{:.5}] vertical_collision={} vertical_below={} x_collision={} z_collision={} vel_after=[{:.5},{:.5},{:.5}] input=[f={} b={} l={} r={} jump={} sneak={} sprint={}] feet={:?}:{} below={:?}:{} support={}",
            self.exact_position.x,
            self.exact_position.y,
            self.exact_position.z,
            y_fraction,
            started_on_ground,
            self.on_ground,
            self.horizontal_collision,
            self.minor_horizontal_collision,
            attempted_movement.x,
            attempted_movement.y,
            attempted_movement.z,
            resolved_movement.x,
            resolved_movement.y,
            resolved_movement.z,
            collision.vertical_collision,
            collision.vertical_collision_below,
            collision.x_collision,
            collision.z_collision,
            self.exact_velocity.x,
            self.exact_velocity.y,
            self.exact_velocity.z,
            input.forward,
            input.backward,
            input.left,
            input.right,
            input.jump,
            input.sneak,
            input.sprint,
            feet_pos,
            feet_state.block.path,
            below_pos,
            below_state.block.path,
            support_state
                .map(|(pos, state)| format!("{pos:?}:{}", state.block.path))
                .unwrap_or_else(|| "none".to_string()),
        ));
    }
    fn log_slide_probe_move(
        &self,
        level: &ClientLevel,
        input: &PlayerInput,
        attempted_movement: DVec3,
        resolved_movement: DVec3,
        velocity_before: DVec3,
        started_on_ground: bool,
        started_horizontal_collision: bool,
        started_minor_horizontal_collision: bool,
        collision: &MoveCollisionResult,
    ) {
        if !Self::slide_probe_enabled() && !Self::movement_diagnostics_enabled() {
            return;
        }
        let before = self.exact_prev_position;
        let after = self.exact_position;
        let support = self.current_supporting_block(level);
        let below_pos = self.block_pos_below_that_affects_movement(level);
        let feet_pos = self.block_position();
        let below_state = level.get_block_state(below_pos);
        let feet_state = level.get_block_state(feet_pos);
        let support_state = support.map(|pos| (pos, level.get_block_state(pos)));
        let clipped_x = attempted_movement.x != resolved_movement.x;
        let clipped_y = attempted_movement.y != resolved_movement.y;
        let clipped_z = attempted_movement.z != resolved_movement.z;
        let collider_summary = if collision.horizontal_collision {
            let before_aabb = self.aabb_at_position(before);
            let query_aabb = before_aabb.expand_towards(el_math::Vec3::new(
                attempted_movement.x,
                attempted_movement.y,
                attempted_movement.z,
            ));
            let context = self.collision_context();
            let colliders =
                level.get_block_collisions_with_context_and_positions(&query_aabb, &context);
            let mut entries = colliders
                .iter()
                .take(8)
                .map(|(pos, shape)| {
                    let state = level.get_block_state(*pos);
                    let bounds = shape.bounds();
                    format!(
                        "{:?}:{}=[{:.6},{:.6},{:.6}->{:.6},{:.6},{:.6}]",
                        pos,
                        state.block.path,
                        bounds.min_x,
                        bounds.min_y,
                        bounds.min_z,
                        bounds.max_x,
                        bounds.max_y,
                        bounds.max_z
                    )
                })
                .collect::<Vec<_>>();
            if colliders.len() > entries.len() {
                entries.push(format!("+{} more", colliders.len() - entries.len()));
            }
            format!(" colliders=[{}]", entries.join(";"))
        } else {
            String::new()
        };
        let outcome = if collision.horizontal_collision
            && attempted_movement.x.abs() + attempted_movement.z.abs() > 1.0e-5
            && resolved_movement.x.abs() + resolved_movement.z.abs() <= 1.0e-5
        {
            "blocked_no_horizontal_progress"
        } else if collision.horizontal_collision {
            "slid_or_step_resolved"
        } else if attempted_movement.length_squared() > 1.0e-8 {
            "free_motion"
        } else {
            "no_input_or_velocity"
        };
        let source = if collision.horizontal_collision {
            "local_collision_shape"
        } else {
            "local_physics"
        };
        Self::slide_probe_log(format!(
            "[SlideDebug] function=LocalPlayer::move_and_resolve_collisions outcome={} source={} tick={:.0} before=[{:.12},{:.12},{:.12}] after=[{:.12},{:.12},{:.12}] attempted=[{:.12},{:.12},{:.12}] resolved=[{:.12},{:.12},{:.12}] vel_before=[{:.12},{:.12},{:.12}] vel_after=[{:.12},{:.12},{:.12}] clipped=[x:{} y:{} z:{}] collision=[x:{} y:{} z:{} horiz:{} vert:{} below:{}] ground=[before:{} after:{}] horiz=[before:{} after:{}] minor=[before:{} after:{}] input=[f:{} b:{} l:{} r:{} jump:{} sneak:{} sprint:{}] pose={:?} crouching={} raw_shift={} feet={:?}:{} below={:?}:{} support={}{}",
            outcome,
            source,
            self.age_in_ticks,
            before.x,
            before.y,
            before.z,
            after.x,
            after.y,
            after.z,
            attempted_movement.x,
            attempted_movement.y,
            attempted_movement.z,
            resolved_movement.x,
            resolved_movement.y,
            resolved_movement.z,
            velocity_before.x,
            velocity_before.y,
            velocity_before.z,
            self.exact_velocity.x,
            self.exact_velocity.y,
            self.exact_velocity.z,
            clipped_x,
            clipped_y,
            clipped_z,
            collision.x_collision,
            collision.vertical_collision,
            collision.z_collision,
            collision.horizontal_collision,
            collision.vertical_collision,
            collision.vertical_collision_below,
            started_on_ground,
            self.on_ground,
            started_horizontal_collision,
            self.horizontal_collision,
            started_minor_horizontal_collision,
            self.minor_horizontal_collision,
            input.forward,
            input.backward,
            input.left,
            input.right,
            input.jump,
            input.sneak,
            input.sprint,
            self.entity_pose,
            self.movement_crouching,
            self.is_sneaking,
            feet_pos,
            feet_state.block.path,
            below_pos,
            below_state.block.path,
            support_state
                .map(|(pos, state)| format!("{pos:?}:{}", state.block.path))
                .unwrap_or_else(|| "none".to_string()),
            collider_summary,
        ));
    }
    fn make_stuck_in_block(&mut self, speed_multiplier: DVec3, abilities_flying: bool) {
        if abilities_flying {
            return;
        }
        self.fall_distance = 0.0;
        self.stuck_speed_multiplier = speed_multiplier;
    }
    fn apply_stuck_speed_multiplier(&mut self, movement: DVec3) -> DVec3 {
        if self.stuck_speed_multiplier.length_squared() <= 1.0e-7 {
            return movement;
        }
        let adjusted = DVec3::new(
            movement.x * self.stuck_speed_multiplier.x,
            movement.y * self.stuck_speed_multiplier.y,
            movement.z * self.stuck_speed_multiplier.z,
        );
        self.stuck_speed_multiplier = DVec3::ZERO;
        self.exact_velocity = DVec3::ZERO;
        adjusted
    }
    #[allow(dead_code)]
    fn limit_piston_movement(&mut self, level: &ClientLevel, movement: DVec3) -> DVec3 {
        if movement.length_squared() <= 1.0e-7 {
            return movement;
        }
        let current_game_time = level.game_time();
        if current_game_time != self.piston_deltas_game_time {
            self.piston_deltas = [0.0; 3];
            self.piston_deltas_game_time = current_game_time;
        }
        if movement.x != 0.0 {
            let x = self.apply_piston_movement_restriction(Axis::X, movement.x);
            if x.abs() <= 1.0e-5 {
                DVec3::ZERO
            } else {
                DVec3::new(x, 0.0, 0.0)
            }
        } else if movement.y != 0.0 {
            let y = self.apply_piston_movement_restriction(Axis::Y, movement.y);
            if y.abs() <= 1.0e-5 {
                DVec3::ZERO
            } else {
                DVec3::new(0.0, y, 0.0)
            }
        } else if movement.z != 0.0 {
            let z = self.apply_piston_movement_restriction(Axis::Z, movement.z);
            if z.abs() <= 1.0e-5 {
                DVec3::ZERO
            } else {
                DVec3::new(0.0, 0.0, z)
            }
        } else {
            DVec3::ZERO
        }
    }
    fn apply_piston_movement_restriction(&mut self, axis: Axis, amount: f64) -> f64 {
        let ordinal = Self::piston_axis_ordinal(axis);
        let clamped = el_math::mth::clamp_f64(amount + self.piston_deltas[ordinal], -0.51, 0.51);
        let restricted = clamped - self.piston_deltas[ordinal];
        self.piston_deltas[ordinal] = clamped;
        restricted
    }
    fn piston_axis_ordinal(axis: Axis) -> usize {
        match axis {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        }
    }
    fn update_inside_block_stuck_effects(&mut self, level: &ClientLevel, abilities_flying: bool) {
        if self.no_physics {
            return;
        }
        let aabb = self.get_aabb().deflate_all(1.0e-5);
        let min_x = aabb.min_x.floor() as i32;
        let max_x = aabb.max_x.floor() as i32;
        let min_y = aabb.min_y.floor() as i32;
        let max_y = aabb.max_y.floor() as i32;
        let min_z = aabb.min_z.floor() as i32;
        let max_z = aabb.max_z.floor() as i32;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                for z in min_z..=max_z {
                    let pos = BlockPos::new(x, y, z);
                    let block_box = AABB::new(
                        x as f64,
                        y as f64,
                        z as f64,
                        x as f64 + 1.0,
                        y as f64 + 1.0,
                        z as f64 + 1.0,
                    );
                    if !block_box.intersects(&aabb) {
                        continue;
                    }
                    let state = level.get_block_state(pos);
                    match state.block.path.as_str() {
                        "cobweb" => {
                            let multiplier = if self.has_effect(MOB_EFFECT_WEAVING) {
                                DVec3::new(0.5, 0.25, 0.5)
                            } else {
                                DVec3::new(0.25, 0.05, 0.25)
                            };
                            Self::slide_probe_log(format!(
                                "[SlideDebug] function=LocalPlayer::update_inside_block_stuck_effects stuck_effect source=block_entity_inside block={} pos={pos:?} multiplier=[{:.2},{:.2},{:.2}] weaving={}",
                                state.block.path,
                                multiplier.x,
                                multiplier.y,
                                multiplier.z,
                                self.has_effect(MOB_EFFECT_WEAVING),
                            ));
                            self.make_stuck_in_block(multiplier, abilities_flying);
                        }
                        "slime_block" => {
                            let abs_dy = self.exact_velocity.y.abs();
                            if abs_dy < 0.1 && !self.is_sneaking {
                                let scale = 0.4 + abs_dy * 0.2;
                                self.exact_velocity.x *= scale;
                                self.exact_velocity.z *= scale;
                                self.velocity = self.exact_velocity.as_vec3();
                                self.mirrored_velocity = self.velocity;
                            }
                        }
                        "sweet_berry_bush" => {
                            Self::slide_probe_log(format!(
                                "[SlideDebug] function=LocalPlayer::update_inside_block_stuck_effects stuck_effect source=block_entity_inside block={} pos={pos:?} multiplier=[0.8,0.75,0.8]",
                                state.block.path
                            ));
                            self.make_stuck_in_block(DVec3::new(0.8, 0.75, 0.8), abilities_flying);
                        }
                        "honey_block" if self.is_sliding_down_honey_block(pos) => {
                            Self::slide_probe_log(format!(
                                "[SlideDebug] function=LocalPlayer::update_inside_block_stuck_effects honey_slide source=block_entity_inside block={} pos={pos:?} velocity=[{:.5},{:.5},{:.5}]",
                                state.block.path,
                                self.exact_velocity.x,
                                self.exact_velocity.y,
                                self.exact_velocity.z,
                            ));
                            self.do_honey_block_slide_movement();
                        }
                        "powder_snow"
                            if level.get_block_state(self.block_position()).block.path
                                == "powder_snow" =>
                        {
                            Self::slide_probe_log(format!(
                                "[SlideDebug] function=LocalPlayer::update_inside_block_stuck_effects stuck_effect source=block_entity_inside block={} pos={pos:?} multiplier=[0.9,1.5,0.9]",
                                state.block.path
                            ));
                            self.make_stuck_in_block(DVec3::new(0.9, 1.5, 0.9), abilities_flying);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    fn honey_old_delta_y(delta_y: f64) -> f64 {
        delta_y / BASE_VERTICAL_AIR_DRAG as f64 + DEFAULT_BASE_GRAVITY
    }
    fn honey_new_delta_y(delta_y: f64) -> f64 {
        (delta_y - DEFAULT_BASE_GRAVITY) * BASE_VERTICAL_AIR_DRAG as f64
    }
    fn is_sliding_down_honey_block(&self, pos: BlockPos) -> bool {
        if self.on_ground {
            return false;
        }
        if self.exact_position.y > pos.y as f64 + 0.9375 - 1.0e-7 {
            return false;
        }
        if Self::honey_old_delta_y(self.exact_velocity.y) >= -0.08 {
            return false;
        }
        let dx = (pos.x as f64 + 0.5 - self.exact_position.x).abs();
        let dz = (pos.z as f64 + 0.5 - self.exact_position.z).abs();
        let overlap_distance = 0.4375 + (self.bb_width_like_java() / 2.0_f32) as f64;
        dx + 1.0e-7 > overlap_distance || dz + 1.0e-7 > overlap_distance
    }
    fn do_honey_block_slide_movement(&mut self) {
        let old_delta_y = Self::honey_old_delta_y(self.exact_velocity.y);
        if old_delta_y < -0.13 {
            let horizontal_reduction_factor = -0.05 / old_delta_y;
            self.exact_velocity.x *= horizontal_reduction_factor;
            self.exact_velocity.z *= horizontal_reduction_factor;
        }
        self.exact_velocity.y = Self::honey_new_delta_y(-0.05);
        self.fall_distance = 0.0;
    }
    fn apply_slime_step_on_like_java(&mut self, level: &ClientLevel) {
        if level
            .get_block_state(self.block_pos_below_that_affects_movement(level))
            .block
            .path
            != "slime_block"
        {
            return;
        }
        let abs_dy = self.exact_velocity.y.abs();
        if abs_dy < 0.1 && !self.is_sneaking {
            let scale = 0.4 + abs_dy * 0.2;
            self.exact_velocity.x *= scale;
            self.exact_velocity.z *= scale;
            self.velocity = self.exact_velocity.as_vec3();
            self.mirrored_velocity = self.velocity;
        }
    }
    fn float_in_water_while_ridden_like_java(&mut self, level: &ClientLevel) {
        let is_floatable_type = false;
        if is_floatable_type
            && self.is_vehicle
            && self.fluid_height(level, true) > self.fluid_jump_threshold()
        {
            self.exact_velocity.y += 0.04;
        }
    }
    fn movement_speed(&self) -> f32 {
        self.attr_f32(crate::gameplay::attributes::MOVEMENT_SPEED)
    }
    pub(crate) fn set_sprinting_like_java(&mut self, sprinting: bool) {
        if self.is_sprinting == sprinting {
            return;
        }
        self.is_sprinting = sprinting;
        const SPRINTING_MODIFIER_ID: &str = "minecraft:sprinting";
        if let Some(inst) = self
            .attributes
            .get_mut(crate::gameplay::attributes::MOVEMENT_SPEED.id)
        {
            inst.remove_modifier(SPRINTING_MODIFIER_ID);
            if sprinting {
                inst.add_transient_modifier(crate::gameplay::attributes::AttributeModifier {
                    id: SPRINTING_MODIFIER_ID.to_string(),
                    amount: SPRINTING_SPEED_MODIFIER as f64,
                    operation:
                        crate::gameplay::attributes::AttributeModifierOperation::AddMultipliedTotal,
                });
            }
        }
    }
    pub fn cause_extra_knockback_like_java(&mut self) {
        self.exact_velocity.x *= 0.6;
        self.exact_velocity.z *= 0.6;
        self.velocity = self.exact_velocity.as_vec3();
        self.mirrored_velocity = self.velocity;
        self.set_sprinting_like_java(false);
    }
    pub fn knockback_like_java(&mut self, power: f64, mut xd: f64, mut zd: f64) {
        let resistance = self.attr_f64(crate::gameplay::attributes::KNOCKBACK_RESISTANCE);
        let power = power * (1.0 - resistance);
        if power <= 0.0 {
            return;
        }
        if xd * xd + zd * zd < 1.0e-5 {
            use el_math::random::{LegacyRandomSource, RandomSource};
            let seed = (self.age_in_ticks as i64)
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407)
                ^ (self.exact_position.x.to_bits() as i64)
                ^ (self.exact_position.z.to_bits() as i64);
            let mut rng = LegacyRandomSource::new(seed);
            while xd * xd + zd * zd < 1.0e-5 {
                xd = (rng.next_double() - rng.next_double()) * 0.01;
                zd = (rng.next_double() - rng.next_double()) * 0.01;
            }
        }
        let len = (xd * xd + zd * zd).sqrt();
        let nx = xd / len * power;
        let nz = zd / len * power;
        let dx = self.exact_velocity.x;
        let dy = self.exact_velocity.y;
        let dz = self.exact_velocity.z;
        let new_y = if self.on_ground {
            (dy / 2.0 + power).min(0.4)
        } else {
            dy
        };
        self.exact_velocity = DVec3::new(dx / 2.0 - nx, new_y, dz / 2.0 - nz);
        self.velocity = self.exact_velocity.as_vec3();
        self.mirrored_velocity = self.velocity;
    }
    fn flying_speed_for_ground_travel(&self) -> f32 {
        if self.is_sprinting {
            PLAYER_SPRINT_FLYING_SPEED
        } else {
            PLAYER_FLYING_SPEED
        }
    }
    fn friction_influenced_speed(&self, block_friction: f32, on_ground: bool) -> f32 {
        if on_ground {
            let speed = self.movement_speed();
            if block_friction > 0.6 {
                speed
                    * (GROUND_SPEED_FRICTION_NUMERATOR
                        / (block_friction * block_friction * block_friction))
            } else {
                speed
            }
        } else {
            self.flying_speed_for_ground_travel()
        }
    }
    fn modified_block_friction(&self, raw_block_friction: f32) -> f32 {
        crate::gameplay::attributes::compute_modified_friction(
            raw_block_friction,
            self.attr_f32(crate::gameplay::attributes::FRICTION_MODIFIER),
        )
    }
    fn modified_air_drag(&self, base: f32) -> f32 {
        crate::gameplay::attributes::compute_modified_friction(
            base,
            self.attr_f32(crate::gameplay::attributes::AIR_DRAG_MODIFIER),
        )
    }
    fn air_drag_like_java(&self) -> f32 {
        self.modified_air_drag(if self.omnidirectional_air_mover {
            AIR_HORIZONTAL_DRAG
        } else {
            BASE_VERTICAL_AIR_DRAG
        })
    }
    fn block_friction_below(&self, level: &ClientLevel) -> f32 {
        let block_pos = self.block_pos_below_that_affects_movement(level);
        let state = level.get_block_state(block_pos);
        state.get_friction()
    }
    pub fn block_position(&self) -> BlockPos {
        BlockPos::new(
            self.exact_position.x.floor() as i32,
            self.exact_position.y.floor() as i32,
            self.exact_position.z.floor() as i32,
        )
    }
    pub fn on_climbable(&self, level: &ClientLevel) -> bool {
        let pos = self.block_position();
        let state = level.get_block_state(pos);
        let path = state.block.path.as_str();
        if self.is_fall_flying
            && (path == "vine"
                || path == "twisting_vines"
                || path == "twisting_vines_plant"
                || path == "weeping_vines"
                || path == "weeping_vines_plant"
                || path == "cave_vines"
                || path == "cave_vines_plant")
        {
            return false;
        }
        if path == "ladder"
            || path == "vine"
            || path.ends_with("_vine")
            || path == "scaffolding"
            || path == "weeping_vines"
            || path == "weeping_vines_plant"
            || path == "twisting_vines"
            || path == "twisting_vines_plant"
            || path == "cave_vines"
            || path == "cave_vines_plant"
        {
            return true;
        }
        if path.ends_with("_trapdoor")
            && state
                .get_property("open")
                .is_some_and(|open| open == "true")
        {
            let below = level.get_block_state(BlockPos::new(pos.x, pos.y - 1, pos.z));
            return below.block.path == "ladder"
                && below.get_property("facing") == state.get_property("facing");
        }
        false
    }
    fn is_free(
        &self,
        level: &ClientLevel,
        xa: f64,
        ya: f64,
        za: f64,
        extra_colliders: &[el_math::VoxelShape],
    ) -> bool {
        let box_ = self.get_aabb().move_by(xa, ya, za);
        let context = self.collision_context();
        let source_aabb = self.get_aabb();
        level.no_collision_with_context_and_entities(
            Some(&source_aabb),
            &box_,
            &context,
            extra_colliders,
        ) && !level.contains_any_liquid(&box_)
    }
    fn jump_out_of_fluid(
        &mut self,
        level: &ClientLevel,
        old_y: f64,
        extra_colliders: &[el_math::VoxelShape],
    ) {
        let movement = self.exact_velocity;
        if self.horizontal_collision
            && self.is_free(
                level,
                movement.x,
                movement.y + (0.6_f32 as f64) - self.exact_position.y + old_y,
                movement.z,
                extra_colliders,
            )
        {
            self.exact_velocity.y = 0.3_f32 as f64;
        }
    }
    fn was_in_powder_snow(&self, level: &ClientLevel) -> bool {
        let pos = self.block_position();
        level.get_block_state(pos).block.path == "powder_snow"
            || level
                .get_block_state(BlockPos::new(pos.x, pos.y - 1, pos.z))
                .block
                .path
                == "powder_snow"
    }
    fn can_walk_on_powder_snow(&self) -> bool {
        self.feet_item_path_for_collision().as_deref() == Some("leather_boots")
    }
    fn handle_on_climbable(&mut self, mut movement: DVec3, level: &ClientLevel) -> DVec3 {
        if !self.on_climbable(level) {
            return movement;
        }
        self.fall_distance = 0.0;
        const CLIMB_CLAMP: f64 = 0.15;
        movement.x = movement.x.clamp(-CLIMB_CLAMP, CLIMB_CLAMP);
        movement.z = movement.z.clamp(-CLIMB_CLAMP, CLIMB_CLAMP);
        let mut yd = movement.y.max(-CLIMB_CLAMP);
        let feet = level.get_block_state(self.block_position());
        if yd < 0.0 && feet.block.path != "scaffolding" && self.is_sneaking {
            yd = 0.0;
        }
        movement.y = yd;
        movement
    }
    fn travel_in_air_step(
        &mut self,
        level: &ClientLevel,
        input: &PlayerInput,
        local_input: Vec2,
        desired_world_input: DVec3,
        travel_started_on_ground: bool,
        jumping: bool,
        extra_colliders: &[el_math::VoxelShape],
    ) -> MoveCollisionResult {
        let block_friction = if travel_started_on_ground {
            self.modified_block_friction(self.block_friction_below(level))
        } else {
            1.0
        };
        let accel_multiplier =
            self.friction_influenced_speed(block_friction, travel_started_on_ground);
        self.exact_velocity += self.relative_input_vector_like_java(local_input, accel_multiplier);
        self.exact_velocity = self.handle_on_climbable(self.exact_velocity, level);
        let d_pos = self.apply_stuck_speed_multiplier(self.exact_velocity);
        let current_movement_for_restitution = self.exact_velocity;
        let collision = self.move_and_resolve_collisions(d_pos, level, false, extra_colliders);
        let new_vel = collision.velocity;
        self.apply_move_position_and_ground(level, &collision);
        if !self.is_in_water {
            self.update_fluid_interaction_like_java(level, false);
        }
        self.exact_velocity = self.restitute_movement_after_collisions_like_java(
            level,
            current_movement_for_restitution,
            collision.movement,
            new_vel,
            collision.x_collision,
            collision.z_collision,
            collision.vertical_collision,
            collision.vertical_collision_below,
        );
        self.update_horizontal_collision_state(
            desired_world_input,
            collision.movement,
            collision.horizontal_collision,
        );
        let block_speed_factor = self.block_speed_factor(level, false);
        self.exact_velocity.x *= block_speed_factor as f64;
        self.exact_velocity.z *= block_speed_factor as f64;
        if (collision.horizontal_collision || jumping || input.jump)
            && (self.on_climbable(level)
                || (self.was_in_powder_snow(level) && self.can_walk_on_powder_snow()))
        {
            self.exact_velocity.y = 0.2;
        }
        let pos_below = self.block_pos_below_that_affects_movement(level);
        let gravity = self.gravity_like_java();
        if let Some(lev_amp) = self.get_effect_amplifier(MOB_EFFECT_LEVITATION) {
            let target = 0.05 * (lev_amp + 1) as f64;
            self.exact_velocity.y += (target - self.exact_velocity.y) * 0.2;
        } else if level.has_chunk_at(pos_below) {
            let effective_gravity = self.effective_gravity();
            self.exact_velocity.y -= effective_gravity;
        } else if self.exact_position.y > -64.0 {
            self.exact_velocity.y = -0.1;
        } else {
            self.exact_velocity.y = 0.0;
        }
        let air_drag = self.modified_air_drag(AIR_HORIZONTAL_DRAG);
        let vertical_drag = if self.omnidirectional_air_mover {
            air_drag
        } else {
            self.modified_air_drag(BASE_VERTICAL_AIR_DRAG)
        };
        let friction = if self.discard_friction {
            1.0_f32
        } else {
            block_friction * air_drag
        } as f64;
        let velocity_before_drag = self.exact_velocity;
        self.exact_velocity.x *= friction;
        self.exact_velocity.z *= friction;
        if !self.discard_friction {
            self.exact_velocity.y *= vertical_drag as f64;
        }
        if Self::slide_probe_enabled() && collision.vertical_collision {
            Self::slide_probe_log(format!(
                "[SlideProbe][Rust][post_collision_velocity] tick={} collision_vel=[{:.12},{:.12},{:.12}] before_drag=[{:.12},{:.12},{:.12}] after_drag=[{:.12},{:.12},{:.12}] on_ground={} vertical_below={} gravity={:.12} vertical_drag={:.12} friction={:.12}",
                self.age_in_ticks,
                collision.velocity.x,
                collision.velocity.y,
                collision.velocity.z,
                velocity_before_drag.x,
                velocity_before_drag.y,
                velocity_before_drag.z,
                self.exact_velocity.x,
                self.exact_velocity.y,
                self.exact_velocity.z,
                collision.vertical_collision_below,
                collision.vertical_collision_below,
                gravity,
                vertical_drag,
                friction,
            ));
        }
        collision
    }
    fn feet_item_path_for_collision(&self) -> Option<String> {
        self.inventory.get(36).and_then(|stack| {
            if stack.is_empty() {
                None
            } else {
                Some(stack.item.location.path.clone())
            }
        })
    }
    pub fn collision_context(&self) -> CollisionContext {
        CollisionContext::player(
            self.is_sneaking,
            self.exact_position.y,
            None,
            self.feet_item_path_for_collision(),
            self.fall_distance,
        )
    }
    fn can_fit_with_pose(
        &self,
        level: &ClientLevel,
        pose: PlayerPose,
        extra_colliders: &[el_math::VoxelShape],
    ) -> bool {
        let source_aabb = self.get_aabb();
        let pose_aabb = self
            .aabb_at_position_for_pose(self.exact_position, pose)
            .deflate_all(el_math::aabb::EPSILON);
        let context = self.collision_context();
        level.no_collision_with_context_and_entities(
            Some(&source_aabb),
            &pose_aabb,
            &context,
            extra_colliders,
        )
    }
    fn is_swimming_like_java(&self, abilities_flying: bool) -> bool {
        !abilities_flying && !self.no_physics && self.is_visually_swimming
    }
    fn update_movement_crouching_from_previous_input(
        &mut self,
        level: &ClientLevel,
        extra_colliders: &[el_math::VoxelShape],
        abilities_flying: bool,
    ) {
        let previous_shift_down = self.input.key_presses.sneak;
        self.movement_crouching = !abilities_flying
            && !self.is_swimming_like_java(abilities_flying)
            && !self.is_passenger
            && self.can_fit_with_pose(level, PlayerPose::Crouching, extra_colliders)
            && (previous_shift_down
                || (self.sleeping_pos.is_none()
                    && !self.can_fit_with_pose(level, PlayerPose::Standing, extra_colliders)));
    }
    fn update_swimming_like_java(&mut self, level: &ClientLevel, abilities_flying: bool) {
        if abilities_flying {
            self.is_visually_swimming = false;
            return;
        }
        let should_swim = if self.is_swimming_like_java(abilities_flying) {
            self.is_sprinting && self.is_in_water && !self.is_passenger
        } else {
            let block_pos = BlockPos::new(
                self.exact_position.x.floor() as i32,
                self.exact_position.y.floor() as i32,
                self.exact_position.z.floor() as i32,
            );
            self.is_sprinting
                && self.is_under_water
                && !self.is_passenger
                && level
                    .get_block_state(block_pos)
                    .get_fluid_state()
                    .is_water()
        };
        self.is_visually_swimming = should_swim;
    }
    fn refresh_fluid_and_swim_state_before_ai_step(
        &mut self,
        level: &ClientLevel,
        abilities_flying: bool,
    ) {
        let old_is_underwater = self.is_under_water;
        self.is_under_water = self.fluid_on_eyes_has_water;
        if !self.no_physics {
            if !old_is_underwater && self.is_under_water {
                self.pending_underwater_sound = Some(UnderwaterSoundCue::Enter);
            } else if old_is_underwater && !self.is_under_water {
                self.pending_underwater_sound = Some(UnderwaterSoundCue::Exit);
            }
        }
        self.update_fluid_interaction_like_java(level, abilities_flying);
        self.update_swimming_like_java(level, abilities_flying);
        self.prev_swim_amount = self.swim_amount;
        self.swim_amount = if self.is_visually_swimming(false) {
            f32::min(1.0, self.swim_amount + 0.09)
        } else {
            f32::max(0.0, self.swim_amount - 0.09)
        };
    }
    fn update_fluid_interaction_like_java(&mut self, level: &ClientLevel, abilities_flying: bool) {
        self.is_in_water = self.water_intersects_body(level);
        if self.is_in_water && self.water_entry_debug_ticks == 0 {
            self.water_entry_debug_ticks = 5;
            if Self::slide_probe_enabled() {
                println!(
                    "[WaterEntry] tick={} pos=[{:.4},{:.4},{:.4}] vel=[{:.6},{:.6},{:.6}] on_ground={} is_sprinting={} is_swimming={} fluid_on_eyes={}",
                    self.age_in_ticks,
                    self.exact_position.x,
                    self.exact_position.y,
                    self.exact_position.z,
                    self.exact_velocity.x,
                    self.exact_velocity.y,
                    self.exact_velocity.z,
                    self.on_ground,
                    self.is_sprinting,
                    self.is_visually_swimming(false),
                    self.fluid_on_eyes_has_water,
                );
            }
        }
        if !self.is_in_water {
            self.water_entry_debug_ticks = 0;
        }
        self.is_in_lava = self.lava_intersects_body(level);
        if self.is_in_water {
            self.fall_distance = 0.0;
        }
        if !abilities_flying {
            if self.is_in_water {
                self.apply_fluid_current(level, true, 0.014);
            }
            if self.is_in_lava {
                self.apply_fluid_current(level, false, 0.0023333333333333335);
            }
        }
        self.fluid_on_eyes_has_water = self.eye_is_under_water(level);
    }
    pub fn update_is_underwater(&mut self) -> bool {
        let old_is_underwater = self.is_under_water;
        self.is_under_water = self.fluid_on_eyes_has_water;
        if !self.no_physics {
            if !old_is_underwater && self.is_under_water {
                self.pending_underwater_sound = Some(UnderwaterSoundCue::Enter);
            } else if old_is_underwater && !self.is_under_water {
                self.pending_underwater_sound = Some(UnderwaterSoundCue::Exit);
            }
        }
        self.is_under_water
    }
    #[inline]
    pub fn water_vision(&self) -> f32 {
        if !self.is_under_water {
            0.0
        } else if self.water_vision_time >= 600 {
            1.0
        } else {
            let a = (self.water_vision_time as f32 / 100.0).clamp(0.0, 1.0);
            let b = if self.water_vision_time < 100 {
                0.0
            } else {
                ((self.water_vision_time - 100) as f32 / 500.0).clamp(0.0, 1.0)
            };
            a * 0.6 + b * 0.39999998
        }
    }
    pub fn tick_water_vision(&mut self) {
        if self.is_under_water {
            let speed = if self.no_physics { 10 } else { 1 };
            self.water_vision_time = (self.water_vision_time + speed).clamp(0, 600);
        } else if self.water_vision_time > 0 {
            self.water_vision_time = (self.water_vision_time - 10).clamp(0, 600);
        }
    }
    fn desired_pose_like_java(&self, abilities_flying: bool) -> PlayerPose {
        if self.sleeping_pos.is_some() {
            PlayerPose::Sleeping
        } else if self.is_swimming_like_java(abilities_flying) {
            PlayerPose::Swimming
        } else if self.is_fall_flying {
            PlayerPose::FallFlying
        } else if self.is_auto_spin_attack {
            PlayerPose::SpinAttack
        } else if self.is_sneaking && !abilities_flying {
            PlayerPose::Crouching
        } else {
            PlayerPose::Standing
        }
    }
    fn update_player_pose_like_java(
        &mut self,
        level: &ClientLevel,
        extra_colliders: &[el_math::VoxelShape],
        abilities_flying: bool,
    ) {
        if !self.can_fit_with_pose(level, PlayerPose::Swimming, extra_colliders) {
            return;
        }
        let old_pose = self.entity_pose;
        let desired = self.desired_pose_like_java(abilities_flying);
        self.entity_pose = if self.no_physics
            || self.is_passenger
            || self.can_fit_with_pose(level, desired, extra_colliders)
        {
            desired
        } else if self.can_fit_with_pose(level, PlayerPose::Crouching, extra_colliders) {
            PlayerPose::Crouching
        } else {
            PlayerPose::Swimming
        };
        if self.entity_pose != old_pose
            && (Self::slide_probe_enabled() || Self::movement_diagnostics_enabled())
        {
            Self::slide_probe_log(format!(
                "[MovePoseDebug] function=Player::updatePlayerPose old={old_pose:?} desired={desired:?} new={:?} raw_shift={} crouching={} shared_swimming={} abilities_flying={} passenger={} spectator={}",
                self.entity_pose,
                self.is_sneaking,
                self.movement_crouching,
                self.is_visually_swimming,
                abilities_flying,
                self.is_passenger,
                self.no_physics,
            ));
        }
    }
    pub(crate) fn camera_collision_context(&self) -> CollisionContext {
        self.collision_context()
    }
    fn block_pos_below_that_affects_movement(&self, level: &ClientLevel) -> BlockPos {
        self.get_on_pos_like_java(level, 0.500_001)
    }
    fn on_pos_legacy(&self, level: &ClientLevel) -> BlockPos {
        self.get_on_pos_like_java(level, 0.2)
    }
    fn get_on_pos_like_java(&self, level: &ClientLevel, offset: f64) -> BlockPos {
        if let Some(support_pos) = self.main_supporting_block_pos {
            if offset <= 1.0e-5 {
                return support_pos;
            }
            let below_state = level.get_block_state(support_pos);
            let below_block = below_state.block.path.as_str();
            let is_fence = below_block.ends_with("_fence");
            let is_wall = below_block.ends_with("_wall");
            let is_fence_gate = below_block.ends_with("_fence_gate");
            if (offset > 0.5 || !is_fence) && !is_wall && !is_fence_gate {
                BlockPos::new(
                    support_pos.x,
                    (self.exact_position.y - offset).floor() as i32,
                    support_pos.z,
                )
            } else {
                support_pos
            }
        } else {
            BlockPos::new(
                self.exact_position.x.floor() as i32,
                (self.exact_position.y - offset).floor() as i32,
                self.exact_position.z.floor() as i32,
            )
        }
    }
    fn vanilla_block_speed_factor(block_path: &str) -> f32 {
        match block_path {
            "soul_sand" | "honey_block" => 0.4,
            _ => 1.0,
        }
    }
    fn vanilla_block_jump_factor(block_path: &str) -> f32 {
        match block_path {
            "honey_block" => 0.5,
            _ => 1.0,
        }
    }
    fn vanilla_block_bounce_restitution(block_path: &str) -> f64 {
        if block_path == "slime_block" {
            1.0
        } else if block_path.ends_with("_bed") {
            0.75
        } else {
            0.0
        }
    }
    fn vanilla_block_suppresses_bounce(block_path: &str) -> bool {
        block_path == "honey_block"
    }
    fn restitute_movement_after_collisions_like_java(
        &self,
        level: &ClientLevel,
        current_movement: DVec3,
        resolved_movement: DVec3,
        mut velocity_after_collision: DVec3,
        x_collision: bool,
        z_collision: bool,
        vertical_collision: bool,
        vertical_collision_below: bool,
    ) -> DVec3 {
        let entity_restitution = self.attr_f64(crate::gameplay::attributes::BOUNCINESS);
        if x_collision {
            velocity_after_collision.x = -current_movement.x * entity_restitution;
        }
        if z_collision {
            velocity_after_collision.z = -current_movement.z * entity_restitution;
        }
        if !vertical_collision {
            return velocity_after_collision;
        }
        if !vertical_collision_below {
            velocity_after_collision.y = 0.0;
            return velocity_after_collision;
        }
        let effect_pos = self.on_pos_legacy(level);
        let effect_state = level.get_block_state(effect_pos);
        let effect_block = effect_state.block.path.as_str();
        let current_y = current_movement.y;
        let effective_gravity = self.effective_gravity();
        let restitution = if -current_y >= effective_gravity
            && !self.is_sneaking
            && !Self::vanilla_block_suppresses_bounce(effect_block)
        {
            Self::vanilla_block_bounce_restitution(effect_block)
        } else {
            0.0
        };
        if restitution > 0.0 {
            let portion_with_movement = resolved_movement.y / current_y;
            let gravity_compensation = portion_with_movement * effective_gravity;
            let vertical_air_drag = self.air_drag_like_java() as f64;
            let effective_drag = 1.0 + portion_with_movement * (vertical_air_drag - 1.0);
            velocity_after_collision.y =
                (gravity_compensation - current_y) * effective_drag * restitution;
        } else {
            velocity_after_collision.y = 0.0;
        }
        velocity_after_collision
    }
    fn block_speed_factor(&self, level: &ClientLevel, abilities_flying: bool) -> f32 {
        if abilities_flying || self.is_fall_flying {
            return 1.0;
        }
        let state_here = level.get_block_state(self.block_position());
        let speed_here = Self::vanilla_block_speed_factor(&state_here.block.path);
        let base_factor = if state_here.block.path == "water"
            || state_here.block.path == "bubble_column"
            || speed_here != 1.0
        {
            speed_here
        } else {
            let below = level.get_block_state(self.block_pos_below_that_affects_movement(level));
            Self::vanilla_block_speed_factor(&below.block.path)
        };
        let movement_efficiency = self.attr_f32(crate::gameplay::attributes::MOVEMENT_EFFICIENCY);
        base_factor + movement_efficiency * (1.0 - base_factor)
    }
    fn block_jump_factor(&self, level: &ClientLevel) -> f32 {
        let state_here = level.get_block_state(self.block_position());
        let jump_here = Self::vanilla_block_jump_factor(&state_here.block.path);
        if jump_here != 1.0 {
            jump_here
        } else {
            let below = level.get_block_state(self.block_pos_below_that_affects_movement(level));
            Self::vanilla_block_jump_factor(&below.block.path)
        }
    }
    fn trim_small_movement_like_java(&mut self) {
        let horizontal_distance_sqr = self.exact_velocity.x * self.exact_velocity.x
            + self.exact_velocity.z * self.exact_velocity.z;
        if horizontal_distance_sqr < 9.0e-6 {
            if self.exact_velocity.x != 0.0 || self.exact_velocity.z != 0.0 {
                Self::slide_probe_log(format!(
                    "[SlideProbe][Rust][trim_movement] tick={} zeroed_horizontal before=[{:.12},{:.12},{:.12}] horizontal_distance_sqr={:.12e}",
                    self.age_in_ticks,
                    self.exact_velocity.x,
                    self.exact_velocity.y,
                    self.exact_velocity.z,
                    horizontal_distance_sqr,
                ));
            }
            self.exact_velocity.x = 0.0;
            self.exact_velocity.z = 0.0;
        }
        if self.exact_velocity.y.abs() < 0.003 {
            if self.exact_velocity.y != 0.0 {
                Self::slide_probe_log(format!(
                    "[SlideProbe][Rust][trim_movement] tick={} zeroed_vertical before=[{:.12},{:.12},{:.12}] on_ground={} vertical_below={}",
                    self.age_in_ticks,
                    self.exact_velocity.x,
                    self.exact_velocity.y,
                    self.exact_velocity.z,
                    self.on_ground,
                    self.exact_velocity.y < 0.0,
                ));
            }
            self.exact_velocity.y = 0.0;
        }
    }
    pub fn previous_jump_pressed_like_java(&self) -> bool {
        self.input.key_presses.jump
    }
    pub fn auto_jump_active_like_java(&self) -> bool {
        self.auto_jump_time > 0
    }
    pub fn set_sprint_window_ticks_like_java(&mut self, ticks: i32) {
        self.sprint_window_ticks = ticks.clamp(SPRINT_WINDOW_MIN_TICKS, SPRINT_WINDOW_MAX_TICKS);
    }
    pub fn is_swimming_for_creative_flight_toggle_like_java(&self, abilities_flying: bool) -> bool {
        self.is_swimming_like_java(abilities_flying)
    }
    pub fn suppress_fall_flying_start_for_creative_toggle_like_java(&mut self) {
        self.just_toggled_creative_flight = true;
    }
    pub fn jump_from_ground_like_java(&mut self, level: &ClientLevel) {
        let jump_boost = self
            .get_effect_amplifier(MOB_EFFECT_JUMP_BOOST)
            .map_or(0.0_f32, |amp| 0.1 * (amp + 1) as f32);
        let jump_power = self.attr_f32(crate::gameplay::attributes::JUMP_STRENGTH)
            * self.block_jump_factor(level)
            + jump_boost;
        if jump_power <= 1.0e-5 {
            return;
        }
        self.exact_velocity.y = self.exact_velocity.y.max(jump_power as f64);
        if self.is_sprinting {
            let f2 = self.yaw;
            self.exact_velocity.x += -(el_math::mth::sin(f2) as f64) * 0.2_f64;
            self.exact_velocity.z += (el_math::mth::cos(f2) as f64) * 0.2_f64;
        }
    }
    fn tick_sprint_trigger_time_like_java(&mut self) {
        if self.sprint_trigger_time > 0 {
            self.sprint_trigger_time -= 1;
        }
    }
    fn has_forward_impulse_like_java(move_vector: Vec2) -> bool {
        move_vector.y > 1.0e-5
    }
    fn update_sprinting_like_java(
        &mut self,
        was_shift_key_down: bool,
        had_forward_impulse: bool,
        mayfly: bool,
        abilities_flying: bool,
    ) {
        let key_presses = self.input.key_presses.clone();
        let has_forward_impulse = Self::has_forward_impulse_like_java(self.input.move_vector);
        if was_shift_key_down
            || (self.is_slow_due_to_using_item() && !self.is_passenger)
            || key_presses.backward
        {
            self.sprint_trigger_time = 0;
        }
        let can_start_sprinting = !self.is_sprinting
            && has_forward_impulse
            && self.is_sprinting_possible(mayfly, abilities_flying)
            && !self.is_slow_due_to_using_item()
            && (!self.is_fall_flying || self.is_under_water)
            && (!self.is_moving_slowly() || self.is_under_water);
        if can_start_sprinting {
            if !had_forward_impulse {
                if self.sprint_trigger_time > 0 {
                    self.set_sprinting_like_java(true);
                } else {
                    self.sprint_trigger_time = self.sprint_window_ticks;
                }
            }
            if key_presses.sprint {
                self.set_sprinting_like_java(true);
            }
        }
        if self.is_sprinting {
            let should_stop = if self.is_swimming_like_java(abilities_flying) {
                !self.is_sprinting_possible(mayfly, true)
                    || !self.is_in_water
                    || (!has_forward_impulse && !self.on_ground && !key_presses.sneak)
            } else {
                !self.is_sprinting_possible(mayfly, abilities_flying)
                    || !has_forward_impulse
                    || (self.horizontal_collision && !self.minor_horizontal_collision)
            };
            if should_stop {
                self.set_sprinting_like_java(false);
            }
        }
    }
    fn has_enough_food(&self) -> bool {
        self.food > 6
    }
    fn has_enough_food_to_do_exhaustive_manoeuvres(&self, mayfly: bool) -> bool {
        self.has_enough_food() || mayfly
    }
    fn is_mobility_restricted(&self) -> bool {
        self.has_effect(MOB_EFFECT_BLINDNESS)
    }
    fn is_in_shallow_water(&self) -> bool {
        self.is_in_water && !self.is_under_water
    }
    fn is_visually_crawling_like_java(&self) -> bool {
        self.is_visually_swimming(false) && !self.is_in_water
    }
    fn is_moving_slowly(&self) -> bool {
        self.movement_crouching || self.is_visually_crawling_like_java()
    }
    fn is_sprinting_possible(&self, mayfly: bool, allowed_in_shallow_water: bool) -> bool {
        let food_or_vehicle = if self.is_passenger {
            true
        } else {
            self.has_enough_food_to_do_exhaustive_manoeuvres(mayfly)
        };
        !self.is_mobility_restricted()
            && food_or_vehicle
            && (allowed_in_shallow_water || !self.is_in_shallow_water())
    }
    pub fn tick(
        &mut self,
        level: &ClientLevel,
        input: &PlayerInput,
        extra_colliders: &[el_math::VoxelShape],
        mayfly: bool,
    ) {
        self.sync_exact_from_public_if_externally_mutated();
        self.exact_prev_position = self.exact_position;
        self.prev_position = self.exact_prev_position.as_vec3();
        self.copy_current_rotation_to_old_like_java();
        self.prev_walk_animation_pos = self.walk_animation_pos;
        self.prev_walk_animation_speed = self.walk_animation_speed;
        self.prev_walk_dist = self.walk_dist;
        self.prev_x_bob = self.x_bob;
        self.prev_y_bob = self.y_bob;
        self.prev_body_yaw = self.body_yaw;
        self.prev_head_yaw = self.head_yaw;
        self.prev_swim_amount = self.swim_amount;
        self.avatar_state.tick(self.position, self.velocity);
        if self.is_passenger {
            self.exact_velocity = DVec3::ZERO;
            self.on_ground = false;
        }
        self.refresh_fluid_and_swim_state_before_ai_step(level, false);
        if self.is_in_lava {
            self.fall_distance *= 0.5;
        }
        self.tick_sprint_trigger_time_like_java();
        self.tick_lerped_head_rotation();
        if self.hurt_time > 0 {
            self.hurt_time -= 1;
        }
        if self.invulnerable_time > 0 {
            self.invulnerable_time -= 1;
        }
        if self.no_jump_delay > 0 {
            self.no_jump_delay -= 1;
        }
        let was_jumping = self.input.key_presses.jump;
        let was_shift_key_down = self.input.key_presses.sneak;
        let had_forward_impulse = Self::has_forward_impulse_like_java(self.input.move_vector);
        let was_auto_jump = self.auto_jump_time > 0;
        if was_auto_jump {
            self.auto_jump_time -= 1;
        }
        let mut tick_input = input.clone();
        if was_auto_jump {
            tick_input.jump = true;
        }
        let input = &tick_input;
        self.trim_small_movement_like_java();
        self.update_movement_crouching_from_previous_input(level, extra_colliders, false);
        let raw_move_input = Self::keyboard_move_vector(input);
        self.input.key_presses = input.clone();
        self.input.move_vector = raw_move_input;
        self.is_sneaking = input.sneak;
        self.move_towards_closest_space_at_corners(level);
        self.update_sprinting_like_java(was_shift_key_down, had_forward_impulse, mayfly, false);
        let jumping = input.jump;
        let just_toggled_creative_flight = self.just_toggled_creative_flight;
        self.just_toggled_creative_flight = false;
        if jumping && !just_toggled_creative_flight && !was_jumping && !self.on_climbable(level) {
            let _ = self.try_to_start_fall_flying();
        }
        if self.is_fall_flying && (self.on_ground || self.is_in_water || !self.can_glide()) {
            self.stop_fall_flying();
        }
        let modified_input = self.apply_input_like_java(raw_move_input, self.is_moving_slowly());
        let move_vec = self.world_input_vector(modified_input);
        self.handle_bubble_column(level);
        if self.auto_spin_attack_ticks > 0 {
            self.auto_spin_attack_ticks -= 1;
            if self.auto_spin_attack_ticks == 0 {
                self.is_auto_spin_attack = false;
            }
        }
        if self.horizontal_collision {
            self.auto_spin_attack_ticks = 0;
            self.is_auto_spin_attack = false;
        }
        for effect in self.active_effects.values_mut() {
            if effect.duration > 0 {
                effect.duration -= 1;
            }
        }
        if self.is_in_water && input.sneak && !self.is_passenger {
            self.exact_velocity.y -= 0.04;
        }
        self.tick_water_vision();
        if jumping && !self.is_passenger {
            let fluid_height = if self.is_in_lava {
                self.fluid_height(level, false)
            } else {
                self.fluid_height(level, true)
            };
            let in_water_and_has_fluid_height = self.is_in_water && fluid_height > 0.0;
            let fluid_jump_threshold = self.fluid_jump_threshold();
            if !in_water_and_has_fluid_height
                || (self.on_ground && !(fluid_height > fluid_jump_threshold))
            {
                if !self.is_in_lava || (self.on_ground && self.is_in_shallow_lava(level)) {
                    if (self.on_ground
                        || (in_water_and_has_fluid_height && fluid_height <= fluid_jump_threshold))
                        && self.no_jump_delay == 0
                    {
                        self.jump_from_ground_like_java(level);
                        self.no_jump_delay = JUMP_COOLDOWN_TICKS;
                    }
                } else {
                    self.exact_velocity.y += 0.04;
                }
            } else {
                self.exact_velocity.y += 0.04;
            }
        } else {
            self.no_jump_delay = 0;
        }
        if !self.is_passenger && self.is_swimming_like_java(false) {
            let head_pos = BlockPos::new(
                self.exact_position.x.floor() as i32,
                (self.exact_position.y + 1.0 - 0.1).floor() as i32,
                self.exact_position.z.floor() as i32,
            );
            let head_fluid_is_empty = level.get_block_state(head_pos).get_fluid_state().is_empty();
            self.apply_swimming_look_vertical_movement_like_java(jumping, head_fluid_is_empty);
        }
        let travel_started_on_ground = self.on_ground;
        let started_horizontal_collision = self.horizontal_collision;
        let started_minor_horizontal_collision = self.minor_horizontal_collision;
        let velocity_before_move = self.exact_velocity;
        if self.is_fall_flying {
            self.update_fall_flying_like_java();
        }
        let before_travel_aabb = self.get_aabb();
        if self.has_effect(MOB_EFFECT_SLOW_FALLING) || self.has_effect(MOB_EFFECT_LEVITATION) {
            self.fall_distance = 0.0;
        }
        let fluid_travel_is_falling = self.exact_velocity.y <= 0.0;
        let fluid_travel_old_y = self.exact_position.y;
        let prev_x = self.exact_position.x;
        let prev_z = self.exact_position.z;
        let collision = if self.is_in_water {
            let mut speed = 0.02_f32;
            let mut water_walker =
                self.attr_f32(crate::gameplay::attributes::WATER_MOVEMENT_EFFICIENCY);
            if !travel_started_on_ground {
                water_walker *= 0.5;
            }
            if water_walker > 0.0 {
                speed += (self.movement_speed() - speed) * water_walker;
            }
            self.exact_velocity += self.relative_input_vector_like_java(modified_input, speed);
            let d_pos = self.apply_stuck_speed_multiplier(self.exact_velocity);
            let current_movement_for_restitution = self.exact_velocity;
            let collision = self.move_and_resolve_collisions(d_pos, level, false, extra_colliders);
            self.apply_move_position_and_ground(level, &collision);
            if !self.is_in_water {
                self.update_fluid_interaction_like_java(level, false);
            }
            self.exact_velocity = self.restitute_movement_after_collisions_like_java(
                level,
                current_movement_for_restitution,
                collision.movement,
                collision.velocity,
                collision.x_collision,
                collision.z_collision,
                collision.vertical_collision,
                collision.vertical_collision_below,
            );
            if collision.horizontal_collision && self.on_climbable(level) {
                self.exact_velocity.y = 0.2;
            }
            self.update_horizontal_collision_state(
                move_vec,
                collision.movement,
                collision.horizontal_collision,
            );
            let block_speed_factor = self.block_speed_factor(level, false);
            self.exact_velocity.x *= block_speed_factor as f64;
            self.exact_velocity.z *= block_speed_factor as f64;
            let mut slow_down = if self.is_sprinting { 0.9_f32 } else { 0.8_f32 };
            let mut ww = self.attr_f32(crate::gameplay::attributes::WATER_MOVEMENT_EFFICIENCY);
            if !travel_started_on_ground {
                ww *= 0.5;
            }
            if ww > 0.0 {
                slow_down += (0.54600006_f32 - slow_down) * ww;
            }
            if self.has_effect(MOB_EFFECT_DOLPHINS_GRACE) {
                slow_down = 0.96_f32;
            }
            self.exact_velocity.x *= slow_down as f64;
            self.exact_velocity.z *= slow_down as f64;
            self.exact_velocity.y *= 0.8_f32 as f64;
            let gravity = self.gravity_like_java();
            let gravity = if fluid_travel_is_falling && self.has_effect(MOB_EFFECT_SLOW_FALLING) {
                gravity.min(0.01)
            } else {
                gravity
            };
            if gravity != 0.0 && !self.is_sprinting {
                let y = self.exact_velocity.y;
                let g16 = gravity / 16.0;
                if fluid_travel_is_falling && (y - 0.005).abs() >= 0.003 && (y - g16).abs() < 0.003
                {
                    self.exact_velocity.y = -0.003;
                } else {
                    self.exact_velocity.y = y - g16;
                }
            }
            self.jump_out_of_fluid(level, fluid_travel_old_y, extra_colliders);
            self.float_in_water_while_ridden_like_java(level);
            collision
        } else if self.is_in_lava {
            self.exact_velocity += self.relative_input_vector_like_java(modified_input, 0.02);
            let d_pos = self.apply_stuck_speed_multiplier(self.exact_velocity);
            let current_movement_for_restitution = self.exact_velocity;
            let collision = self.move_and_resolve_collisions(d_pos, level, false, extra_colliders);
            self.apply_move_position_and_ground(level, &collision);
            if !self.is_in_water {
                self.update_fluid_interaction_like_java(level, false);
            }
            self.exact_velocity = self.restitute_movement_after_collisions_like_java(
                level,
                current_movement_for_restitution,
                collision.movement,
                collision.velocity,
                collision.x_collision,
                collision.z_collision,
                collision.vertical_collision,
                collision.vertical_collision_below,
            );
            if collision.horizontal_collision && self.on_climbable(level) {
                self.exact_velocity.y = 0.2;
            }
            self.update_horizontal_collision_state(
                move_vec,
                collision.movement,
                collision.horizontal_collision,
            );
            let block_speed_factor = self.block_speed_factor(level, false);
            self.exact_velocity.x *= block_speed_factor as f64;
            self.exact_velocity.z *= block_speed_factor as f64;
            let gravity = self.gravity_like_java();
            let gravity = if fluid_travel_is_falling && self.has_effect(MOB_EFFECT_SLOW_FALLING) {
                gravity.min(0.01)
            } else {
                gravity
            };
            if self.is_in_shallow_lava(level) {
                self.exact_velocity.x *= 0.5_f32 as f64;
                self.exact_velocity.z *= 0.5_f32 as f64;
                self.exact_velocity.y *= 0.8_f32 as f64;
                if gravity != 0.0 && !self.is_sprinting {
                    let y = self.exact_velocity.y;
                    let g16 = gravity / 16.0;
                    if fluid_travel_is_falling
                        && (y - 0.005).abs() >= 0.003
                        && (y - g16).abs() < 0.003
                    {
                        self.exact_velocity.y = -0.003;
                    } else {
                        self.exact_velocity.y = y - g16;
                    }
                }
            } else {
                self.exact_velocity *= 0.5_f32 as f64;
            }
            if gravity != 0.0 {
                self.exact_velocity.y -= gravity / 4.0;
            }
            self.jump_out_of_fluid(level, fluid_travel_old_y, extra_colliders);
            collision
        } else if self.is_fall_flying {
            if self.on_climbable(level) {
                self.stop_fall_flying();
                self.travel_in_air_step(
                    level,
                    input,
                    modified_input,
                    move_vec,
                    travel_started_on_ground,
                    jumping,
                    extra_colliders,
                )
            } else {
                self.exact_velocity = self.update_fall_flying_movement(self.exact_velocity);
                let d_pos = self.apply_stuck_speed_multiplier(self.exact_velocity);
                let collision =
                    self.move_and_resolve_collisions(d_pos, level, false, extra_colliders);
                self.apply_move_position_and_ground(level, &collision);
                if !self.is_in_water {
                    self.update_fluid_interaction_like_java(level, false);
                }
                self.exact_velocity = collision.velocity;
                self.update_horizontal_collision_state(
                    move_vec,
                    collision.movement,
                    collision.horizontal_collision,
                );
                if self.firework_boost_ticks > 0 {
                    let look = self.look_angle_d();
                    let movement = self.exact_velocity;
                    self.exact_velocity += glam::DVec3::new(
                        look.x * 0.1 + (look.x * 1.5 - movement.x) * 0.5,
                        look.y * 0.1 + (look.y * 1.5 - movement.y) * 0.5,
                        look.z * 0.1 + (look.z * 1.5 - movement.z) * 0.5,
                    );
                    self.firework_boost_ticks -= 1;
                }
                collision
            }
        } else {
            self.travel_in_air_step(
                level,
                input,
                modified_input,
                move_vec,
                travel_started_on_ground,
                jumping,
                extra_colliders,
            )
        };
        let d_pos = collision.attempted_movement;
        let actual_movement = collision.movement;
        let next_on_ground = collision.vertical_collision_below;
        if self.water_entry_debug_ticks > 0 {
            self.water_entry_debug_ticks -= 1;
            if Self::slide_probe_enabled() {
                println!(
                    "[WaterTick] tick={} pos=[{:.4},{:.4},{:.4}] vel=[{:.6},{:.6},{:.6}] on_ground={} is_sprinting={} is_in_water={} is_swimming={} horizontal_coll={}",
                    self.age_in_ticks,
                    self.exact_position.x,
                    self.exact_position.y,
                    self.exact_position.z,
                    self.exact_velocity.x,
                    self.exact_velocity.y,
                    self.exact_velocity.z,
                    self.on_ground,
                    self.is_sprinting,
                    self.is_in_water,
                    self.is_visually_swimming(false),
                    self.horizontal_collision,
                );
            }
        }
        self.check_fall_distance(actual_movement.y, next_on_ground);
        self.debug_vertical_prediction_if_suspicious(
            level,
            input,
            d_pos,
            actual_movement,
            travel_started_on_ground,
            &collision,
        );
        self.log_slide_probe_move(
            level,
            input,
            d_pos,
            actual_movement,
            velocity_before_move,
            travel_started_on_ground,
            started_horizontal_collision,
            started_minor_horizontal_collision,
            &collision,
        );
        self.update_inside_block_stuck_effects(level, false);
        self.apply_slime_step_on_like_java(level);
        self.check_auto_spin_attack_like_java(collision.horizontal_collision);
        self.push_entities_like_java(&before_travel_aabb, &self.get_aabb());
        let horizontal_delta_d = DVec3::new(
            self.exact_position.x - self.exact_prev_position.x,
            0.0,
            self.exact_position.z - self.exact_prev_position.z,
        );
        let horizontal_delta = horizontal_delta_d.as_vec3();
        self.update_auto_jump(
            level,
            (self.exact_position.x - prev_x) as f32,
            (self.exact_position.z - prev_z) as f32,
            extra_colliders,
        );
        let eye_pos = BlockPos::new(
            self.exact_position.x.floor() as i32,
            (self.exact_position.y + self.pose_eye_height(false) as f64).floor() as i32,
            self.exact_position.z.floor() as i32,
        );
        let in_bubble_column = level.get_block_state(eye_pos).block.path == "bubble_column";
        if self.is_under_water && !in_bubble_column {
            self.air = self.decrease_air_supply(self.air);
            if self.should_take_drowning_damage() {
                self.air = 0;
            }
        } else if self.air < Self::MAX_AIR_SUPPLY {
            self.air = self.increase_air_supply(self.air);
        }
        self.age_in_ticks += 1.0;
        let horizontal_speed = horizontal_delta.length();
        if self.is_passenger {
            self.prev_walk_animation_pos = 0.0;
            self.prev_walk_animation_speed = 0.0;
            self.walk_animation_pos = 0.0;
            self.walk_animation_speed = 0.0;
        } else {
            let target_speed = (horizontal_speed * 4.0).min(1.0);
            self.walk_animation_speed += (target_speed - self.walk_animation_speed) * 0.4;
            self.walk_animation_pos += self.walk_animation_speed;
        }
        self.walk_dist += horizontal_speed * 0.6;
        if self.controlled_camera {
            self.x_bob += (self.pitch - self.x_bob) * 0.5;
            self.y_bob += wrap_radians(self.yaw - self.y_bob) * 0.5;
        }
        self.update_body_and_head_yaw(horizontal_delta);
        self.normalize_old_living_rotations();
        self.o_bob = self.bob;
        let is_dead = self.death_time > 0 || self.health <= 0.0;
        let bob_target = if self.on_ground && !is_dead && !self.is_visually_swimming(false) {
            horizontal_speed.min(0.1)
        } else {
            0.0
        };
        self.bob += (bob_target - self.bob) * 0.4;
        self.avatar_state.add_walk_distance(horizontal_speed * 0.6);
        self.avatar_state.update_bob(bob_target);
        self.update_player_pose_like_java(level, extra_colliders, false);
        self.sync_public_from_exact();
    }
    pub fn tick_flying(
        &mut self,
        level: &ClientLevel,
        input: &PlayerInput,
        flying_speed: f32,
        extra_colliders: &[el_math::VoxelShape],
        mayfly: bool,
    ) {
        self.just_toggled_creative_flight = false;
        self.sync_exact_from_public_if_externally_mutated();
        self.exact_prev_position = self.exact_position;
        self.prev_position = self.exact_prev_position.as_vec3();
        self.copy_current_rotation_to_old_like_java();
        self.prev_walk_animation_pos = self.walk_animation_pos;
        self.prev_walk_animation_speed = self.walk_animation_speed;
        self.prev_walk_dist = self.walk_dist;
        self.prev_x_bob = self.x_bob;
        self.prev_y_bob = self.y_bob;
        self.prev_body_yaw = self.body_yaw;
        self.prev_head_yaw = self.head_yaw;
        self.prev_swim_amount = self.swim_amount;
        self.avatar_state.tick(self.position, self.velocity);
        self.refresh_fluid_and_swim_state_before_ai_step(level, true);
        self.tick_sprint_trigger_time_like_java();
        self.tick_lerped_head_rotation();
        let was_shift_key_down = self.input.key_presses.sneak;
        let had_forward_impulse = Self::has_forward_impulse_like_java(self.input.move_vector);
        let was_auto_jump = self.auto_jump_time > 0;
        if was_auto_jump {
            self.auto_jump_time -= 1;
        }
        let mut tick_input = input.clone();
        if was_auto_jump {
            tick_input.jump = true;
        }
        let input = &tick_input;
        self.movement_crouching = false;
        let raw_move_input = Self::keyboard_move_vector(input);
        self.input.key_presses = input.clone();
        self.input.move_vector = raw_move_input;
        self.is_sneaking = input.sneak;
        self.move_towards_closest_space_at_corners(level);
        self.update_sprinting_like_java(was_shift_key_down, had_forward_impulse, mayfly, true);
        let mut expired = Vec::new();
        for (effect_id, effect) in self.active_effects.iter_mut() {
            if effect.duration > 0 {
                effect.duration -= 1;
                if effect.duration == 0 {
                    expired.push(*effect_id);
                }
            }
        }
        for effect_id in expired {
            self.active_effects.remove(&effect_id);
        }
        self.tick_water_vision();
        let fluid_affects_movement = false;
        let mut input_ya = 0.0;
        if input.jump {
            input_ya += 1.0;
        }
        if input.sneak {
            input_ya -= 1.0;
        }
        if input_ya != 0.0 {
            self.exact_velocity.y += (input_ya * flying_speed * 3.0) as f64;
        }
        let modified_input = self.apply_input_like_java(raw_move_input, false);
        let move_vec = self.world_input_vector(modified_input);
        let travel_started_on_ground = self.on_ground;
        let started_horizontal_collision = self.horizontal_collision;
        let started_minor_horizontal_collision = self.minor_horizontal_collision;
        let velocity_before_move = self.exact_velocity;
        let block_friction = if travel_started_on_ground {
            self.modified_block_friction(self.block_friction_below(level))
        } else {
            1.0
        };
        let accel_multiplier = if fluid_affects_movement && self.is_in_water {
            0.02
        } else if travel_started_on_ground {
            self.friction_influenced_speed(block_friction, true)
        } else {
            flying_speed * if self.is_sprinting { 2.0 } else { 1.0 }
        };
        self.exact_velocity +=
            self.relative_input_vector_like_java(modified_input, accel_multiplier);
        let movement_before_collision = self.apply_stuck_speed_multiplier(self.exact_velocity);
        let collision = self.move_and_resolve_collisions(
            movement_before_collision,
            level,
            true,
            extra_colliders,
        );
        self.apply_move_position_and_ground(level, &collision);
        let current_movement_for_restitution = self.exact_velocity;
        let mut new_vel = self.restitute_movement_after_collisions_like_java(
            level,
            current_movement_for_restitution,
            collision.movement,
            collision.velocity,
            collision.x_collision,
            collision.z_collision,
            collision.vertical_collision,
            collision.vertical_collision_below,
        );
        let actual_movement = collision.movement;
        self.update_horizontal_collision_state(
            move_vec,
            actual_movement,
            collision.horizontal_collision,
        );
        let block_speed_factor = self.block_speed_factor(level, true) as f64;
        new_vel.x *= block_speed_factor;
        new_vel.z *= block_speed_factor;
        if self.is_in_water {
            new_vel *= 0.8;
        } else if self.is_in_lava {
            new_vel *= 0.5;
        } else {
            let friction = (block_friction * AIR_HORIZONTAL_DRAG) as f64;
            new_vel.x *= friction;
            new_vel.z *= friction;
        }
        new_vel.y = velocity_before_move.y * 0.6;
        self.exact_velocity = new_vel;
        let next_on_ground = collision.vertical_collision_below;
        if !self.is_in_water {
            self.update_fluid_interaction_like_java(level, true);
        }
        self.check_fall_distance(actual_movement.y, next_on_ground);
        self.log_slide_probe_move(
            level,
            input,
            movement_before_collision,
            actual_movement,
            velocity_before_move,
            travel_started_on_ground,
            started_horizontal_collision,
            started_minor_horizontal_collision,
            &collision,
        );
        self.update_inside_block_stuck_effects(level, true);
        self.age_in_ticks += 1.0;
        let horizontal_delta_d = DVec3::new(
            self.exact_position.x - self.exact_prev_position.x,
            0.0,
            self.exact_position.z - self.exact_prev_position.z,
        );
        let horizontal_delta = horizontal_delta_d.as_vec3();
        let target_speed = (horizontal_delta.length() * 4.0).min(1.0);
        self.walk_animation_speed += (target_speed - self.walk_animation_speed) * 0.4;
        self.walk_animation_pos += self.walk_animation_speed;
        self.walk_dist += horizontal_delta.length() * 0.6;
        if self.controlled_camera {
            self.x_bob += (self.pitch - self.x_bob) * 0.5;
            self.y_bob += wrap_radians(self.yaw - self.y_bob) * 0.5;
        }
        self.update_body_and_head_yaw(horizontal_delta);
        self.normalize_old_living_rotations();
        self.o_bob = self.bob;
        self.bob += (0.0 - self.bob) * 0.4;
        self.avatar_state
            .add_walk_distance(horizontal_delta.length() * 0.6);
        self.avatar_state.update_bob(0.0);
        self.update_player_pose_like_java(level, extra_colliders, true);
        self.sync_public_from_exact();
    }
    fn update_body_and_head_yaw(&mut self, horizontal_delta: Vec3) {
        self.head_yaw = self.yaw;
        let mut body_target = self.body_yaw;
        if horizontal_delta.length_squared() > 0.0025000002 {
            let walk_direction =
                horizontal_delta.z.atan2(horizontal_delta.x) - std::f32::consts::FRAC_PI_2;
            let diff_between_direction_and_facing =
                (wrap_radians(self.yaw).to_degrees() - walk_direction.to_degrees()).abs();
            body_target = if (95.0..265.0).contains(&diff_between_direction_and_facing) {
                walk_direction - std::f32::consts::PI
            } else {
                walk_direction
            };
        }
        if self.attack_anim_for_living_tick > 0.0 {
            body_target = self.yaw;
        }
        self.body_yaw += wrap_radians(body_target - self.body_yaw) * 0.3;
        let head_diff = wrap_radians(self.yaw - self.body_yaw);
        let max_head_rotation = 50.0_f32.to_radians();
        if head_diff.abs() > max_head_rotation {
            self.body_yaw += head_diff - head_diff.signum() * max_head_rotation;
        }
    }
    fn normalize_old_living_rotations(&mut self) {
        normalize_old_rotation_degrees(&mut self.java_y_rot_o_degrees, self.java_y_rot_degrees);
        self.prev_yaw = self.java_y_rot_o_degrees.to_radians();
        normalize_old_rotation(&mut self.prev_body_yaw, self.body_yaw);
        normalize_old_rotation_degrees(&mut self.java_x_rot_o_degrees, self.java_x_rot_degrees);
        self.prev_pitch = self.java_x_rot_o_degrees.to_radians();
        normalize_old_rotation(&mut self.prev_head_yaw, self.head_yaw);
    }
    pub fn render_swim_amount(&self, partial_tick: f32) -> f32 {
        self.prev_swim_amount
            + (self.swim_amount - self.prev_swim_amount) * partial_tick.clamp(0.0, 1.0)
    }
    pub fn water_intersects_body(&self, level: &ClientLevel) -> bool {
        self.fluid_intersects_body(level, true)
    }
    pub fn lava_intersects_body(&self, level: &ClientLevel) -> bool {
        self.fluid_intersects_body(level, false)
    }
    pub fn handle_bubble_column(&mut self, level: &ClientLevel) {
        let interaction_box = self.get_aabb().deflate_all(0.001);
        let x0 = interaction_box.min_x.floor() as i32;
        let y0 = interaction_box.min_y.floor() as i32;
        let z0 = interaction_box.min_z.floor() as i32;
        let x1 = interaction_box.max_x.ceil() as i32 - 1;
        let y1 = interaction_box.max_y.ceil() as i32 - 1;
        let z1 = interaction_box.max_z.ceil() as i32 - 1;
        let mut found_bubble = false;
        let mut drag_down = true;
        let mut bubble_pos = BlockPos::ZERO;
        for x in x0..=x1 {
            for y in y0..=y1 {
                for z in z0..=z1 {
                    let pos = BlockPos::new(x, y, z);
                    let state = level.get_block_state(pos);
                    if state.block.path == "bubble_column" {
                        found_bubble = true;
                        drag_down = state
                            .get_property("drag")
                            .map(|s| s == "true")
                            .unwrap_or(true);
                        bubble_pos = pos;
                        break;
                    }
                }
                if found_bubble {
                    break;
                }
            }
            if found_bubble {
                break;
            }
        }
        if found_bubble {
            let pos_above = bubble_pos.above();
            let state_above = level.get_block_state(pos_above);
            let nothing_above = state_above.get_collision_shape().is_empty()
                && state_above.get_fluid_state().is_empty();
            if nothing_above {
                self.on_above_bubble_column(drag_down);
            } else {
                self.on_inside_bubble_column(drag_down);
            }
        }
    }
    fn on_above_bubble_column(&mut self, drag_down: bool) {
        let yd = if drag_down {
            f64::max(-0.9, self.exact_velocity.y - 0.03)
        } else {
            f64::min(1.8, self.exact_velocity.y + 0.1)
        };
        self.exact_velocity.y = yd;
    }
    fn on_inside_bubble_column(&mut self, drag_down: bool) {
        let yd = if drag_down {
            f64::max(-0.3, self.exact_velocity.y - 0.03)
        } else {
            f64::min(0.7, self.exact_velocity.y + 0.06)
        };
        self.exact_velocity.y = yd;
        self.fall_distance = 0.0;
    }
    pub const MAX_AIR_SUPPLY: i32 = 300;
    #[inline]
    pub fn max_air_supply(&self) -> i32 {
        Self::MAX_AIR_SUPPLY
    }
    #[inline]
    pub fn should_take_drowning_damage(&self) -> bool {
        self.air <= -20
    }
    pub fn decrease_air_supply(&self, current_supply: i32) -> i32 {
        let oxygen_bonus = self.attr_f64(crate::gameplay::attributes::OXYGEN_BONUS);
        if oxygen_bonus > 0.0 {
            use el_math::random::{LegacyRandomSource, RandomSource};
            let mut rng = LegacyRandomSource::new(self.age_in_ticks as i64);
            if rng.next_double() >= 1.0 / (oxygen_bonus + 1.0) {
                return current_supply;
            }
        }
        current_supply - 1
    }
    #[inline]
    pub fn increase_air_supply(&self, current_supply: i32) -> i32 {
        (current_supply + 4).min(Self::MAX_AIR_SUPPLY)
    }
    fn fluid_intersects_body(&self, level: &ClientLevel, water: bool) -> bool {
        self.fluid_height(level, water) > 0.0
    }
    fn fluid_interaction_is_loaded(level: &ClientLevel, interaction_box: AABB) -> bool {
        let x0 = interaction_box.min_x.floor() as i32;
        let y0 = interaction_box.min_y.floor() as i32;
        let z0 = interaction_box.min_z.floor() as i32;
        let x1 = interaction_box.max_x.ceil() as i32 - 1;
        let y1 = interaction_box.max_y.ceil() as i32 - 1;
        let z1 = interaction_box.max_z.ceil() as i32 - 1;
        level.has_fluid_and_loaded(x0 - 1, y0, z0 - 1, x1 + 1, y1, z1 + 1)
    }
    fn fluid_height(&self, level: &ClientLevel, water: bool) -> f64 {
        let interaction_box = self.get_aabb().deflate_all(0.001);
        let entity_box_min_y = self.get_aabb().min_y;
        if !Self::fluid_interaction_is_loaded(level, interaction_box) {
            return 0.0;
        }
        let x0 = interaction_box.min_x.floor() as i32;
        let y0 = interaction_box.min_y.floor() as i32;
        let z0 = interaction_box.min_z.floor() as i32;
        let x1 = interaction_box.max_x.ceil() as i32 - 1;
        let y1 = interaction_box.max_y.ceil() as i32 - 1;
        let z1 = interaction_box.max_z.ceil() as i32 - 1;
        let mut height = 0.0;
        for x in x0..=x1 {
            for y in y0..=y1 {
                for z in z0..=z1 {
                    let pos = BlockPos::new(x, y, z);
                    let fluid = level.get_block_state(pos).get_fluid_state();
                    let matches = if water {
                        fluid.is_water()
                    } else {
                        fluid.is_lava()
                    };
                    if matches {
                        let fluid_top = y as f64 + level.fluid_height(pos) as f64;
                        if fluid_top >= interaction_box.min_y {
                            height = f64::max(height, fluid_top - entity_box_min_y);
                        }
                    }
                }
            }
        }
        height
    }
    fn fluid_current(&self, level: &ClientLevel, water: bool) -> (glam::DVec3, u32) {
        let interaction_box = self.get_aabb().deflate_all(0.001);
        let entity_box_min_y = self.get_aabb().min_y;
        if !Self::fluid_interaction_is_loaded(level, interaction_box) {
            return (glam::DVec3::ZERO, 0);
        }
        let x0 = interaction_box.min_x.floor() as i32;
        let y0 = interaction_box.min_y.floor() as i32;
        let z0 = interaction_box.min_z.floor() as i32;
        let x1 = interaction_box.max_x.ceil() as i32 - 1;
        let y1 = interaction_box.max_y.ceil() as i32 - 1;
        let z1 = interaction_box.max_z.ceil() as i32 - 1;
        let mut current = glam::DVec3::ZERO;
        let mut height = 0.0;
        let mut cells = 0_u32;
        for x in x0..=x1 {
            for y in y0..=y1 {
                for z in z0..=z1 {
                    let pos = BlockPos::new(x, y, z);
                    let fluid = level.get_block_state(pos).get_fluid_state();
                    let matches = if water {
                        fluid.is_water()
                    } else {
                        fluid.is_lava()
                    };
                    if matches {
                        let fluid_top = y as f64 + level.fluid_height(pos) as f64;
                        if fluid_top >= interaction_box.min_y {
                            height = f64::max(height, fluid_top - entity_box_min_y);
                            let mut flow = fluid_flow_like_java(level, pos);
                            if height < 0.4 {
                                flow *= height;
                            }
                            current += flow;
                            cells += 1;
                        }
                    }
                }
            }
        }
        (current, cells)
    }
    fn apply_fluid_current(&mut self, level: &ClientLevel, water: bool, scale: f64) {
        let (current, cells) = self.fluid_current(level, water);
        if cells == 0 || current.length_squared() < 1.0e-5 {
            return;
        }
        let mut impulse = current * (1.0 / f64::from(cells));
        impulse *= scale;
        let old = self.exact_velocity;
        if old.x.abs() < 0.003 && old.z.abs() < 0.003 && impulse.length() < 0.0045000000000000005 {
            impulse = java_normalize_or_zero(impulse) * 0.0045000000000000005;
        }
        self.exact_velocity += impulse;
    }
    fn fluid_jump_threshold(&self) -> f64 {
        if self.entity_pose == PlayerPose::Swimming || self.is_auto_spin_attack {
            0.0
        } else {
            0.4
        }
    }
    fn is_in_shallow_lava(&self, level: &ClientLevel) -> bool {
        self.fluid_height(level, false) <= self.fluid_jump_threshold()
    }
    pub fn eye_is_under_water(&self, level: &ClientLevel) -> bool {
        let interaction_box = self.get_aabb().deflate_all(0.001);
        if !Self::fluid_interaction_is_loaded(level, interaction_box) {
            return false;
        }
        let x0 = interaction_box.min_x.floor() as i32;
        let y0 = interaction_box.min_y.floor() as i32;
        let z0 = interaction_box.min_z.floor() as i32;
        let x1 = interaction_box.max_x.ceil() as i32 - 1;
        let y1 = interaction_box.max_y.ceil() as i32 - 1;
        let z1 = interaction_box.max_z.ceil() as i32 - 1;
        let eye_x = self.exact_position.x.floor() as i32;
        let eye_y = self.exact_position.y + self.pose_eye_height(false) as f64;
        let eye_z = self.exact_position.z.floor() as i32;
        for x in x0..=x1 {
            for y in y0..=y1 {
                for z in z0..=z1 {
                    let pos = BlockPos::new(x, y, z);
                    let fluid = level.get_block_state(pos).get_fluid_state();
                    if !fluid.is_water() {
                        continue;
                    }
                    let fluid_bottom = y as f64;
                    let fluid_top = fluid_bottom + level.fluid_height(pos) as f64;
                    if fluid_top < interaction_box.min_y {
                        continue;
                    }
                    if x == eye_x && z == eye_z && eye_y >= fluid_bottom && eye_y <= fluid_top {
                        return true;
                    }
                }
            }
        }
        false
    }
    fn update_fall_flying_movement(&self, mut movement: DVec3) -> DVec3 {
        let look = self.look_angle_d();
        let lean = self.pitch;
        let look_hor = (look.x * look.x + look.z * look.z).sqrt();
        let move_hor = (movement.x * movement.x + movement.z * movement.z).sqrt();
        let gravity = if movement.y <= 0.0 && self.has_effect(MOB_EFFECT_SLOW_FALLING) {
            self.gravity_like_java().min(0.01)
        } else {
            self.gravity_like_java()
        };
        let lift_force = (lean as f64).cos();
        let lift_force = lift_force * lift_force;
        movement.y += gravity * (-1.0 + lift_force * 0.75);
        if movement.y < 0.0 && look_hor > 0.0 {
            let convert = movement.y * -0.1 * lift_force;
            movement.x += look.x * convert / look_hor;
            movement.y += convert;
            movement.z += look.z * convert / look_hor;
        }
        if lean < 0.0 && look_hor > 0.0 {
            let convert = move_hor * (-el_math::mth::sin(lean) as f64) * 0.04;
            movement.x += -look.x * convert / look_hor;
            movement.y += convert * 3.2;
            movement.z += -look.z * convert / look_hor;
        }
        if look_hor > 0.0 {
            movement.x += (look.x / look_hor * move_hor - movement.x) * 0.1;
            movement.z += (look.z / look_hor * move_hor - movement.z) * 0.1;
        }
        movement.x *= 0.99;
        movement.y *= 0.98;
        movement.z *= 0.99;
        movement
    }
    fn look_angle_d(&self) -> DVec3 {
        let sin_pitch = el_math::mth::sin(self.pitch);
        let cos_pitch = el_math::mth::cos(self.pitch);
        let sin_yaw = el_math::mth::sin(self.yaw);
        let cos_yaw = el_math::mth::cos(self.yaw);
        DVec3::new(
            (-cos_pitch * sin_yaw) as f64,
            (-sin_pitch) as f64,
            (cos_pitch * cos_yaw) as f64,
        )
    }
    fn apply_swimming_look_vertical_movement_like_java(
        &mut self,
        jumping: bool,
        head_fluid_is_empty: bool,
    ) {
        let look_angle_y = self.look_angle_d().y;
        let multiplier = if look_angle_y < -0.2 { 0.085 } else { 0.06 };
        if look_angle_y <= 0.0 || jumping || !head_fluid_is_empty {
            let movement_y = self.exact_velocity.y;
            self.exact_velocity.y += (look_angle_y - movement_y) * multiplier;
        }
    }
    fn forward_like_java_1_21_11(&self) -> DVec3 {
        let degrees_to_radians = std::f32::consts::PI / 180.0;
        let yaw = -self.java_y_rot_degrees * degrees_to_radians - std::f32::consts::PI;
        let pitch = -self.java_x_rot_degrees * degrees_to_radians;
        let cos_yaw = el_math::mth::cos(yaw);
        let sin_yaw = el_math::mth::sin(yaw);
        let horizontal = -el_math::mth::cos(pitch);
        let vertical = el_math::mth::sin(pitch);
        DVec3::new(
            (sin_yaw * horizontal) as f64,
            vertical as f64,
            (cos_yaw * horizontal) as f64,
        )
    }
    fn aabb_intersects_segment_bounds_like_java(aabb: &AABB, from: DVec3, to: DVec3) -> bool {
        aabb.intersects_xyz(
            from.x.min(to.x),
            from.y.min(to.y),
            from.z.min(to.z),
            from.x.max(to.x),
            from.y.max(to.y),
            from.z.max(to.z),
        )
    }
    fn update_auto_jump(
        &mut self,
        level: &ClientLevel,
        delta_x: f32,
        delta_z: f32,
        entity_colliders: &[el_math::VoxelShape],
    ) {
        if !self.can_auto_jump(level) {
            return;
        }
        let position = self.exact_position;
        let projected_position = position + DVec3::new(delta_x as f64, 0.0, delta_z as f64);
        let mut movement = DVec3::new(delta_x as f64, 0.0, delta_z as f64);
        let speed = self.movement_speed();
        let mut movement_length_squared = movement.length_squared() as f32;
        let mut used_input_fallback = false;
        if movement_length_squared <= 0.001 {
            used_input_fallback = true;
            let input = self.input.move_vector;
            let strafe = speed * input.x;
            let forward = speed * input.y;
            let yaw = self.java_y_rot_degrees * (std::f32::consts::PI / 180.0);
            let sin_yaw = el_math::mth::sin(yaw);
            let cos_yaw = el_math::mth::cos(yaw);
            movement = DVec3::new(
                (strafe * cos_yaw - forward * sin_yaw) as f64,
                movement.y,
                (forward * cos_yaw + strafe * sin_yaw) as f64,
            );
            movement_length_squared = movement.length_squared() as f32;
            if movement_length_squared <= 0.001 {
                return;
            }
        }
        let inverse_movement_length =
            1.0_f32 / Self::mth_sqrt_f32_like_java(movement_length_squared);
        let movement_direction = movement * inverse_movement_length as f64;
        let forward = self.forward_like_java_1_21_11();
        let forward_dot_movement =
            (forward.x * movement_direction.x + forward.z * movement_direction.z) as f32;
        if forward_dot_movement < -0.15 {
            return;
        }
        let context = self.collision_context();
        let player_aabb = self.get_aabb();
        let mut clearance_pos = BlockPos::new(
            position.x.floor() as i32,
            player_aabb.max_y.floor() as i32,
            position.z.floor() as i32,
        );
        if !level
            .get_block_state(clearance_pos)
            .get_collision_shape_with_context(clearance_pos, &context)
            .is_empty()
        {
            return;
        }
        clearance_pos = clearance_pos.above();
        if !level
            .get_block_state(clearance_pos)
            .get_collision_shape_with_context(clearance_pos, &context)
            .is_empty()
        {
            return;
        }
        let mut jump_height = 1.2_f32;
        if let Some(amplifier) = self.get_effect_amplifier(MOB_EFFECT_JUMP_BOOST) {
            jump_height += (amplifier + 1) as f32 * 0.75;
        }
        let look_ahead = (speed * 7.0).max(1.0 / inverse_movement_length);
        let sweep_end = projected_position + movement_direction * look_ahead as f64;
        let (bb_width, bb_height) = self.dimensions_for_pose_like_java(self.current_pose(false));
        let collision_query = AABB::from_vecs(
            MathVec3::new(position.x, position.y, position.z),
            MathVec3::new(sweep_end.x, sweep_end.y + bb_height as f64, sweep_end.z),
        )
        .inflate(bb_width as f64, 0.0, bb_width as f64);
        let side_ray_height = 0.51_f32 as f64;
        let side_ray_start = position + DVec3::new(0.0, side_ray_height, 0.0);
        let side_ray_end = sweep_end + DVec3::new(0.0, side_ray_height, 0.0);
        let perpendicular = movement_direction.cross(DVec3::Y);
        let side_offset = perpendicular * (bb_width * 0.5) as f64;
        let left_start = side_ray_start - side_offset;
        let left_end = side_ray_end - side_offset;
        let right_start = side_ray_start + side_offset;
        let right_end = side_ray_end + side_offset;
        let collisions = level.get_collisions_with_context_and_entities(
            None,
            &collision_query,
            &context,
            entity_colliders,
        );
        let no_obstacle_sentinel = f32::from_bits(1);
        let mut obstacle_top = no_obstacle_sentinel;
        'first_obstacle: for shape in collisions {
            for obstacle in shape.to_aabbs() {
                if !Self::aabb_intersects_segment_bounds_like_java(obstacle, left_start, left_end)
                    && !Self::aabb_intersects_segment_bounds_like_java(
                        obstacle,
                        right_start,
                        right_end,
                    )
                {
                    continue;
                }
                obstacle_top = obstacle.max_y as f32;
                let obstacle_center = obstacle.get_center();
                let obstacle_pos = BlockPos::new(
                    obstacle_center.x.floor() as i32,
                    obstacle_center.y.floor() as i32,
                    obstacle_center.z.floor() as i32,
                );
                let mut height_offset = 1_i32;
                while (height_offset as f32) < jump_height {
                    let above_obstacle = obstacle_pos.add(0, height_offset, 0);
                    let above_shape = level
                        .get_block_state(above_obstacle)
                        .get_collision_shape_with_context(above_obstacle, &context);
                    if !above_shape.is_empty() {
                        obstacle_top = above_shape.bounds().max_y as f32 + above_obstacle.y as f32;
                        if obstacle_top as f64 - position.y > jump_height as f64 {
                            return;
                        }
                    }
                    if height_offset > 1 {
                        clearance_pos = clearance_pos.above();
                        if !level
                            .get_block_state(clearance_pos)
                            .get_collision_shape_with_context(clearance_pos, &context)
                            .is_empty()
                        {
                            return;
                        }
                    }
                    height_offset += 1;
                }
                break 'first_obstacle;
            }
        }
        if obstacle_top == no_obstacle_sentinel {
            return;
        }
        let vertical_offset = (obstacle_top as f64 - position.y) as f32;
        if vertical_offset <= 0.5 || vertical_offset > jump_height {
            return;
        }
        if Self::slide_probe_enabled() || Self::movement_diagnostics_enabled() {
            Self::slide_probe_log(format!(
                "[AutoJumpDebug] function=LocalPlayer::updateAutoJump outcome=armed pos=[{:.12},{:.12},{:.12}] resolved=[{:.9},{:.9}] fallback={} speed={:.9} move=[{:.9},{:.9}] forward_dot={:.9} look_ahead={:.9} obstacle_top={:.9} vertical_offset={:.9} jump_height={:.9}",
                position.x,
                position.y,
                position.z,
                delta_x,
                delta_z,
                used_input_fallback,
                speed,
                movement.x,
                movement.z,
                forward_dot_movement,
                look_ahead,
                obstacle_top,
                vertical_offset,
                jump_height,
            ));
        }
        self.auto_jump_time = 1;
    }
    fn can_auto_jump(&self, level: &ClientLevel) -> bool {
        self.auto_jump_enabled
            && self.auto_jump_time <= 0
            && self.on_ground
            && !self.is_sneaking
            && !self.is_passenger
            && self.input.move_vector.length_squared() > 0.0
            && self.block_jump_factor(level) >= 1.0
    }
    pub fn tick_block_breaking(
        &mut self,
        level: &mut ClientLevel,
        target: Option<BlockPos>,
        active: bool,
        creative: bool,
        active_hotbar_slot: usize,
    ) -> bool {
        if let Some(pos) = target {
            if active {
                if self.target_block != Some(pos) {
                    self.target_block = Some(pos);
                    self.break_progress = 0.0;
                }
                if creative {
                    level.set_air(pos);
                    self.target_block = None;
                    self.break_progress = 0.0;
                    return true;
                }
                let state = level.get_block_state(pos);
                let speed = self.get_destroy_progress(&state, active_hotbar_slot);
                if speed <= 0.0 {
                    return true;
                }
                self.break_progress += speed;
                if self.break_progress >= 1.0 {
                    level.set_air(pos);
                    self.target_block = None;
                    self.break_progress = 0.0;
                    return true;
                }
            } else {
                self.target_block = None;
                self.break_progress = 0.0;
            }
        } else {
            self.target_block = None;
            self.break_progress = 0.0;
        }
        false
    }
    pub fn get_destroy_progress(&self, state: &BlockState, active_hotbar_slot: usize) -> f32 {
        let hardness = state.destroy_speed();
        if hardness < 0.0 {
            return 0.0;
        }
        if hardness == 0.0 {
            return 1.0;
        }
        let held_item = &self.inventory[active_hotbar_slot];
        let mut speed = self.get_tool_destroy_speed(held_item, state);
        if speed > 1.0 {
            speed += self.attr_f32(crate::gameplay::attributes::MINING_EFFICIENCY);
        }
        if let Some(amp) = self.get_effect_amplifier(MOB_EFFECT_HASTE) {
            speed *= 1.0 + (amp + 1) as f32 * 0.2;
        }
        if let Some(amp) = self.get_effect_amplifier(MOB_EFFECT_MINING_FATIGUE) {
            let scale = match amp {
                0 => 0.3,
                1 => 0.09,
                2 => 0.0027,
                _ => 8.1e-4,
            };
            speed *= scale;
        }
        speed *= self.attr_f32(crate::gameplay::attributes::BLOCK_BREAK_SPEED);
        if self.fluid_on_eyes_has_water {
            speed *= self.attr_f32(crate::gameplay::attributes::SUBMERGED_MINING_SPEED);
        }
        if !self.on_ground {
            speed /= 5.0;
        }
        let is_correct_tool = self.is_correct_tool_for_drops(held_item, state);
        let modifier = if is_correct_tool { 30.0 } else { 100.0 };
        speed / hardness / modifier
    }
    pub(crate) fn get_enchantment_level(
        &self,
        stack: &el_core::item::ItemStack,
        ench_id: &str,
    ) -> i32 {
        use el_core::item::DataComponent;
        if let Some(DataComponent::Generic {
            value: Tag::Compound(ench_map),
            ..
        }) = stack.components.get("minecraft:enchantments")
        {
            let ench_map = match compound_get(ench_map, "levels") {
                Some(Tag::Compound(levels)) => levels,
                _ => ench_map,
            };
            if let Some(level_tag) = compound_get(ench_map, ench_id) {
                return match level_tag {
                    Tag::Int(l) => *l,
                    Tag::Byte(b) => *b as i32,
                    Tag::Short(s) => *s as i32,
                    _ => 0,
                };
            }
        }
        if let Some(DataComponent::Generic {
            value: Tag::List(ench_list),
            ..
        }) = stack.components.get("minecraft:enchantments")
        {
            for tag in ench_list {
                if let Tag::Compound(m) = tag {
                    let id_match = match compound_get(m, "id") {
                        Some(Tag::String(s)) => s == ench_id || s.ends_with(ench_id),
                        _ => false,
                    };
                    if id_match {
                        if let Some(lvl_tag) = compound_get(m, "lvl") {
                            return match lvl_tag {
                                Tag::Int(l) => *l,
                                Tag::Byte(b) => *b as i32,
                                Tag::Short(s) => *s as i32,
                                _ => 0,
                            };
                        }
                    }
                }
            }
        }
        0
    }
    fn get_tool_destroy_speed(
        &self,
        stack: &el_core::item::ItemStack,
        block_state: &BlockState,
    ) -> f32 {
        if let Some(speed) = self.explicit_tool_component_mining_speed(stack, block_state) {
            return speed;
        }
        el_core::block_behaviour::java_26_2_vanilla_tool_mining_speed(
            &stack.item.location.to_string(),
            &block_state.block.to_string(),
        )
        .unwrap_or(1.0)
    }
    fn explicit_tool_component_mining_speed(
        &self,
        stack: &el_core::item::ItemStack,
        block_state: &BlockState,
    ) -> Option<f32> {
        use el_core::item::DataComponent;
        let Some(DataComponent::Generic { type_name, value }) =
            stack.components.get("minecraft:tool")
        else {
            return None;
        };
        if type_name != "minecraft:tool" {
            return None;
        }
        let Tag::Compound(fields) = value else {
            return Some(1.0);
        };
        let default_mining_speed = compound_get(fields, "default_mining_speed")
            .and_then(|tag| match tag {
                Tag::Float(speed) => Some(*speed),
                Tag::Double(speed) => Some(*speed as f32),
                _ => None,
            })
            .unwrap_or(1.0);
        if let Some(Tag::List(rules)) = compound_get(fields, "rules") {
            for rule_tag in rules {
                let Tag::Compound(rule_fields) = rule_tag else {
                    continue;
                };
                if !self.tool_component_rule_matches(block_state, rule_fields) {
                    continue;
                }
                if let Some(speed) = compound_get(rule_fields, "speed").and_then(|tag| match tag {
                    Tag::Float(speed) => Some(*speed),
                    Tag::Double(speed) => Some(*speed as f32),
                    _ => None,
                }) {
                    return Some(speed);
                }
            }
        }
        Some(default_mining_speed)
    }
    fn is_correct_tool_for_drops(
        &self,
        stack: &el_core::item::ItemStack,
        block_state: &BlockState,
    ) -> bool {
        if !self.block_requires_correct_tool(block_state) {
            return true;
        }
        if let Some(correct_for_drops) =
            self.explicit_tool_component_correct_for_drops(stack, block_state)
        {
            return correct_for_drops;
        }
        el_core::block_behaviour::java_26_2_vanilla_tool_correct_for_drops(
            &stack.item.location.to_string(),
            &block_state.block.to_string(),
        )
        .unwrap_or(false)
    }
    fn explicit_tool_component_correct_for_drops(
        &self,
        stack: &el_core::item::ItemStack,
        block_state: &BlockState,
    ) -> Option<bool> {
        use el_core::item::DataComponent;
        let Some(DataComponent::Generic { type_name, value }) =
            stack.components.get("minecraft:tool")
        else {
            return None;
        };
        if type_name != "minecraft:tool" {
            return None;
        }
        let Tag::Compound(fields) = value else {
            return Some(false);
        };
        if let Some(Tag::List(rules)) = compound_get(fields, "rules") {
            for rule_tag in rules {
                let Tag::Compound(rule_fields) = rule_tag else {
                    continue;
                };
                if !self.tool_component_rule_matches(block_state, rule_fields) {
                    continue;
                }
                if let Some(Tag::Byte(value)) = compound_get(rule_fields, "correct_for_drops") {
                    return Some(*value != 0);
                }
            }
        }
        Some(false)
    }
    fn tool_component_rule_matches(
        &self,
        block_state: &BlockState,
        rule_fields: &Vec<(String, Tag)>,
    ) -> bool {
        match compound_get(rule_fields, "blocks") {
            Some(Tag::String(tag_or_block)) => {
                self.block_matches_holder_set(block_state, tag_or_block)
            }
            Some(Tag::List(blocks)) => blocks.iter().any(|block| {
                matches!(
                    block, Tag::String(block_or_tag) if self
                    .block_matches_holder_set(block_state, block_or_tag)
                )
            }),
            _ => false,
        }
    }
    fn block_matches_holder_set(&self, block_state: &BlockState, tag_or_block: &str) -> bool {
        if tag_or_block.starts_with('#') {
            let tag_name = &tag_or_block[1..];
            self.block_belongs_to_tag(&block_state.block.to_string(), tag_name)
        } else {
            tag_or_block == block_state.block.to_string() || tag_or_block == block_state.block.path
        }
    }
    fn block_belongs_to_tag(&self, block_id: &str, tag_name: &str) -> bool {
        el_core::block_behaviour::is_java_26_2_vanilla_block_in_tag(block_id, tag_name)
    }
    fn block_requires_correct_tool(&self, block_state: &BlockState) -> bool {
        el_core::block_behaviour::java_26_2_vanilla_block_mining_properties(
            &block_state.block.to_string(),
        )
        .map(|(_, requires_correct_tool)| requires_correct_tool)
        .unwrap_or(false)
    }
    fn suffocates_at(&self, level: &ClientLevel, pos: BlockPos) -> bool {
        let bounding_box = self.get_aabb();
        let test_area = AABB::new(
            pos.x as f64,
            bounding_box.min_y,
            pos.z as f64,
            pos.x as f64 + 1.0,
            bounding_box.max_y,
            pos.z as f64 + 1.0,
        )
        .deflate_all(1.0e-7);
        let context = self.collision_context();
        level.collides_with_suffocating_block(&test_area, &context)
    }
    fn move_towards_closest_space(&mut self, level: &ClientLevel, x: f64, z: f64) {
        let pos = BlockPos::new(
            x.floor() as i32,
            self.exact_position.y.floor() as i32,
            z.floor() as i32,
        );
        if !self.suffocates_at(level, pos) {
            return;
        }
        let xd = x - pos.x as f64;
        let zd = z - pos.z as f64;
        let directions = [
            Direction::West,
            Direction::East,
            Direction::North,
            Direction::South,
        ];
        let mut closest = f64::MAX;
        let mut closest_direction = None;
        for direction in directions {
            let axis_distance = match direction.axis() {
                Axis::X => xd,
                Axis::Z => zd,
                Axis::Y => 0.0,
            };
            let distance_to_edge = if direction.axis_direction() == AxisDirection::Positive {
                1.0 - axis_distance
            } else {
                axis_distance
            };
            let neighbor = BlockPos::new(
                pos.x + direction.step_x(),
                pos.y,
                pos.z + direction.step_z(),
            );
            if distance_to_edge < closest && !self.suffocates_at(level, neighbor) {
                closest = distance_to_edge;
                closest_direction = Some(direction);
            }
        }
        if let Some(direction) = closest_direction {
            match direction.axis() {
                Axis::X => self.exact_velocity.x = 0.1 * direction.step_x() as f64,
                Axis::Z => self.exact_velocity.z = 0.1 * direction.step_z() as f64,
                Axis::Y => {}
            }
        }
    }
    fn move_towards_closest_space_at_corners(&mut self, level: &ClientLevel) {
        if self.no_physics {
            return;
        }
        let corner_offset = self.bb_width() * 0.35;
        self.move_towards_closest_space(
            level,
            self.exact_position.x - corner_offset,
            self.exact_position.z + corner_offset,
        );
        self.move_towards_closest_space(
            level,
            self.exact_position.x - corner_offset,
            self.exact_position.z - corner_offset,
        );
        self.move_towards_closest_space(
            level,
            self.exact_position.x + corner_offset,
            self.exact_position.z - corner_offset,
        );
        self.move_towards_closest_space(
            level,
            self.exact_position.x + corner_offset,
            self.exact_position.z + corner_offset,
        );
    }
    fn collide_movement_like_java(
        &self,
        movement: DVec3,
        level: &ClientLevel,
        extra_colliders: &[el_math::VoxelShape],
    ) -> DVec3 {
        let aabb = self.get_aabb();
        let context = self.collision_context();
        collision::collide_movement_like_java(
            level,
            &aabb,
            &context,
            self.on_ground,
            self.max_up_step(),
            movement,
            extra_colliders,
        )
    }
    fn can_fall_at_least(
        &self,
        level: &ClientLevel,
        delta_x: f64,
        delta_z: f64,
        min_height: f64,
        extra_colliders: &[el_math::VoxelShape],
    ) -> bool {
        let bounding_box = self.get_aabb();
        let test_box = AABB::new(
            bounding_box.min_x + 1.0e-7 + delta_x,
            bounding_box.min_y - min_height - 1.0e-7,
            bounding_box.min_z + 1.0e-7 + delta_z,
            bounding_box.max_x - 1.0e-7 + delta_x,
            bounding_box.min_y,
            bounding_box.max_z - 1.0e-7 + delta_z,
        );
        let context = self.collision_context();
        level.no_collision_with_context_and_entities(
            Some(&bounding_box),
            &test_box,
            &context,
            extra_colliders,
        )
    }
    fn is_above_ground(
        &self,
        level: &ClientLevel,
        max_down_step: f64,
        extra_colliders: &[el_math::VoxelShape],
    ) -> bool {
        self.on_ground
            || (self.fall_distance as f64) < max_down_step
                && !self.can_fall_at_least(
                    level,
                    0.0,
                    0.0,
                    max_down_step - self.fall_distance as f64,
                    extra_colliders,
                )
    }
    fn maybe_back_off_from_edge(
        &self,
        mut delta: DVec3,
        level: &ClientLevel,
        abilities_flying: bool,
        extra_colliders: &[el_math::VoxelShape],
    ) -> DVec3 {
        let max_down_step = self.max_up_step() as f64;
        if abilities_flying
            || delta.y > 0.0
            || !self.is_sneaking
            || !self.is_above_ground(level, max_down_step, extra_colliders)
        {
            return delta;
        }
        let mut delta_x = delta.x;
        let mut delta_z = delta.z;
        let step = 0.05_f64;
        let step_x = delta_x.signum() * step;
        let step_z = delta_z.signum() * step;
        while delta_x != 0.0
            && self.can_fall_at_least(level, delta_x, 0.0, max_down_step, extra_colliders)
        {
            if delta_x.abs() <= step {
                delta_x = 0.0;
                break;
            }
            delta_x -= step_x;
        }
        while delta_z != 0.0
            && self.can_fall_at_least(level, 0.0, delta_z, max_down_step, extra_colliders)
        {
            if delta_z.abs() <= step {
                delta_z = 0.0;
                break;
            }
            delta_z -= step_z;
        }
        while delta_x != 0.0
            && delta_z != 0.0
            && self.can_fall_at_least(level, delta_x, delta_z, max_down_step, extra_colliders)
        {
            if delta_x.abs() <= step {
                delta_x = 0.0;
            } else {
                delta_x -= step_x;
            }
            if delta_z.abs() <= step {
                delta_z = 0.0;
            } else {
                delta_z -= step_z;
            }
        }
        delta.x = delta_x;
        delta.z = delta_z;
        delta
    }
    pub(crate) fn move_and_resolve_collisions(
        &self,
        mut delta: DVec3,
        level: &ClientLevel,
        abilities_flying: bool,
        extra_colliders: &[el_math::VoxelShape],
    ) -> MoveCollisionResult {
        if self.no_physics {
            return MoveCollisionResult {
                position: self.exact_position + delta,
                velocity: self.exact_velocity,
                attempted_movement: delta,
                movement: delta,
                x_collision: false,
                z_collision: false,
                horizontal_collision: false,
                vertical_collision: false,
                vertical_collision_below: false,
            };
        }
        delta = self.maybe_back_off_from_edge(delta, level, abilities_flying, extra_colliders);
        let attempted_movement = delta;
        let movement = self.collide_movement_like_java(delta, level, extra_colliders);
        let position_movement = if entity_applies_collision_movement_like_java(delta, movement) {
            movement
        } else {
            DVec3::ZERO
        };
        let mut final_vel = self.exact_velocity;
        let new_pos = self.exact_position + position_movement;
        if !el_math::mth::equal_f64(movement.x, delta.x) {
            final_vel.x = 0.0;
        }
        if !el_math::mth::equal_f64(movement.z, delta.z) {
            final_vel.z = 0.0;
        }
        if movement.y != delta.y {
            final_vel.y = 0.0;
        }
        let x_collision = !el_math::mth::equal_f64(delta.x, movement.x);
        let z_collision = !el_math::mth::equal_f64(delta.z, movement.z);
        let vertical_collision = movement.y != delta.y;
        MoveCollisionResult {
            position: new_pos,
            velocity: final_vel,
            attempted_movement,
            movement,
            x_collision,
            z_collision,
            horizontal_collision: x_collision || z_collision,
            vertical_collision,
            vertical_collision_below: vertical_collision && delta.y < 0.0,
        }
    }
    pub fn set_experience_values(&mut self, experience_progress: f32, xp_level: i32) {
        self.xp_progress = experience_progress;
        self.xp_level = xp_level;
    }
    pub fn is_in_water_or_rain(&self, level: &ClientLevel) -> bool {
        self.is_in_water || (level.is_raining() && !self.has_roof_above(level))
    }
    pub fn has_roof_above(&self, level: &ClientLevel) -> bool {
        let pos = BlockPos::new(
            self.exact_position.x.floor() as i32,
            self.exact_position.y.floor() as i32,
            self.exact_position.z.floor() as i32,
        );
        for y in (pos.y + 1)..=319 {
            let p = BlockPos::new(pos.x, y, pos.z);
            let state = level.get_block_state(p);
            let path = &state.block.path;
            if path != "air" && path != "water" && path != "lava" {
                let shape =
                    state.get_collision_shape_with_context(p, &el_core::CollisionContext::empty());
                if !shape.is_empty() {
                    return true;
                }
            }
        }
        false
    }
}
impl ClientAvatarEntity for LocalPlayer {
    fn avatar_state(&self) -> &ClientAvatarState {
        &self.avatar_state
    }
    fn avatar_state_mut(&mut self) -> &mut ClientAvatarState {
        &mut self.avatar_state
    }
    fn get_skin(&self) -> PlayerSkin {
        PlayerSkin {
            body: "minecraft:textures/entity/player/wide/steve.png".to_string(),
            cape: None,
            elytra: None,
            model: PlayerModelType::Wide,
            secure: true,
        }
    }
    fn get_parrot_variant_on_shoulder(&self, _left: bool) -> Option<ParrotVariant> {
        None
    }
    fn show_extra_ears(&self) -> bool {
        false
    }
}
fn wrap_radians(value: f32) -> f32 {
    let two_pi = std::f32::consts::TAU;
    (value + std::f32::consts::PI).rem_euclid(two_pi) - std::f32::consts::PI
}
fn wrap_degrees_like_java(mut value: f32) -> f32 {
    value %= 360.0;
    if value >= 180.0 {
        value -= 360.0;
    }
    if value < -180.0 {
        value += 360.0;
    }
    value
}
fn normalize_old_rotation(old: &mut f32, current: f32) {
    while current - *old < -std::f32::consts::PI {
        *old -= std::f32::consts::TAU;
    }
    while current - *old >= std::f32::consts::PI {
        *old += std::f32::consts::TAU;
    }
}
fn normalize_old_rotation_degrees(old: &mut f32, current: f32) {
    while current - *old < -180.0 {
        *old -= 360.0;
    }
    while current - *old >= 180.0 {
        *old += 360.0;
    }
}
fn rot_lerp(from: f32, to: f32, partial_tick: f32) -> f32 {
    from + wrap_radians(to - from) * partial_tick.clamp(0.0, 1.0)
}
