use super::types::*;
use crate::gameplay::physics::collision;
use crate::gameplay::player::local_player::PlayerPose;
use crate::render::item_in_hand_renderer::ItemUseAnimation;
use crate::world::client_level::ClientLevel;
use el_core::block_state::BlockState;
use el_core::collision_context::CollisionContext;
use el_core::registry::BuiltInRegistries;
use el_core::resource_location::ResourceLocation;
use el_math::aabb::AABB;
use el_math::block_pos::BlockPos;
use std::collections::HashMap;
#[derive(Debug, Clone)]
pub struct RemoteEntityRenderState {
    pub id: i32,
    pub uuid: Option<uuid::Uuid>,
    pub profile_name: Option<String>,
    pub skin_texture_url: Option<String>,
    pub skin_model: Option<String>,
    pub kind: String,
    pub spawn_data: i32,
    pub falling_block_state: BlockState,
    pub falling_block_state_is_canonical: bool,
    pub falling_block_start_pos: BlockPos,
    pub acceleration_power: f64,
    pub leash_holder_id: i32,
    pub position_d: glam::DVec3,
    pub position: glam::Vec3,
    pub prev_position_d: glam::DVec3,
    pub prev_position: glam::Vec3,
    pub projectile_render_previous_position_d: glam::DVec3,
    pub projectile_render_previous_position: glam::Vec3,
    pub projectile_render_previous_yaw: f32,
    pub projectile_render_previous_pitch: f32,
    pub projectile_render_pose_pending: bool,
    pub target_position_d: glam::DVec3,
    pub target_position: glam::Vec3,
    pub network_position_codec: RemoteVecDeltaCodec,
    pub last_tick_position_d: glam::DVec3,
    pub last_tick_position: glam::Vec3,
    pub java_y_rot_degrees: f32,
    pub java_y_rot_o_degrees: f32,
    pub target_y_rot_degrees: f32,
    pub yaw: f32,
    pub prev_yaw: f32,
    pub target_yaw: f32,
    pub prev_body_yaw: f32,
    pub body_yaw: f32,
    pub prev_head_yaw: f32,
    pub head_yaw: f32,
    pub target_head_yaw: f32,
    pub head_yaw_steps: i32,
    pub mob_head_stable_time: i32,
    pub mob_last_stable_head_yaw: f32,
    pub swinging: bool,
    pub swinging_hand: crate::render::entity::state::InteractionHand,
    pub swing_time: i32,
    pub o_attack_anim: f32,
    pub attack_anim: f32,
    pub java_x_rot_degrees: f32,
    pub java_x_rot_o_degrees: f32,
    pub target_x_rot_degrees: f32,
    pub pitch: f32,
    pub prev_pitch: f32,
    pub target_pitch: f32,
    pub interpolation_steps: i32,
    pub velocity: glam::Vec3,
    pub player_velocity_d: glam::DVec3,
    pub player_lerp_velocity_d: glam::DVec3,
    pub player_lerp_velocity_steps: u8,
    pub age_in_ticks: u32,
    pub prev_walk_animation_speed: f32,
    pub walk_animation_speed: f32,
    pub walk_animation_pos: f32,
    pub prev_swim_amount: f32,
    pub swim_amount: f32,
    pub prev_flap: f32,
    pub flap: f32,
    pub prev_flap_speed: f32,
    pub flap_speed: f32,
    pub flapping: f32,
    pub old_tentacle_angle: f32,
    pub tentacle_angle: f32,
    pub old_tentacle_movement: f32,
    pub tentacle_movement: f32,
    pub tentacle_speed: f32,
    pub old_tail_animation: f32,
    pub tail_animation: f32,
    pub tail_animation_speed: f32,
    pub old_spikes_animation: f32,
    pub spikes_animation: f32,
    pub prev_squish: f32,
    pub squish: f32,
    pub target_squish: f32,
    pub was_on_ground: bool,
    pub item_stack: Option<el_core::item::ItemStack>,
    pub item_frame_rotation: i32,
    pub main_hand_item_stack: el_core::item::ItemStack,
    pub off_hand_item_stack: el_core::item::ItemStack,
    pub head_equipment: el_core::item::ItemStack,
    pub chest_equipment: el_core::item::ItemStack,
    pub legs_equipment: el_core::item::ItemStack,
    pub feet_equipment: el_core::item::ItemStack,
    pub body_equipment: el_core::item::ItemStack,
    pub saddle_equipment: el_core::item::ItemStack,
    pub shared_flags: u8,
    pub no_gravity: bool,
    pub living_flags: u8,
    pub living_health: f32,
    pub custom_name: Option<String>,
    pub custom_name_visible: bool,
    pub mob_size: i32,
    pub pose: el_protocol_backend::BackendEntityPose,
    pub sleeping_pos: Option<BlockPos>,
    pub interaction_width: f32,
    pub interaction_height: f32,
    pub interaction_response: bool,
    pub primed_tnt_fuse: i32,
    pub experience_orb_value: i32,
    pub armor_stand_client_flags: u8,
    pub armor_stand_head_pose: glam::Vec3,
    pub armor_stand_body_pose: glam::Vec3,
    pub armor_stand_left_arm_pose: glam::Vec3,
    pub armor_stand_right_arm_pose: glam::Vec3,
    pub armor_stand_left_leg_pose: glam::Vec3,
    pub armor_stand_right_leg_pose: glam::Vec3,
    pub primed_tnt_block_state: BlockState,
    pub end_crystal_beam_target: Option<BlockPos>,
    pub end_crystal_show_bottom: bool,
    pub abstract_arrow_flags: u8,
    pub abstract_arrow_pierce_level: u8,
    pub abstract_arrow_in_ground: bool,
    pub abstract_arrow_first_tick: bool,
    pub abstract_arrow_shake_time: i32,
    pub arrow_effect_color: i32,
    pub camera_attributes: Option<crate::gameplay::attributes::AttributeMap>,
    pub received_attribute_snapshots: Option<HashMap<String, RemoteAttributeSnapshot>>,
    pub use_item_snapshot: el_core::item::ItemStack,
    pub use_item_remaining_ticks: f32,
    pub use_item_duration_ticks: f32,
    pub use_item_snapshot_hand: crate::render::entity::state::InteractionHand,
    pub main_arm: crate::render::entity::state::HumanoidArm,
    pub player_model_customisation: u8,
    pub text_display_state: Option<crate::render::entity::display::TextDisplayState>,
    pub avatar_state: crate::gameplay::entity::client_avatar_state::ClientAvatarState,
    pub on_ground: bool,
    pub dead: bool,
    pub death_time: f32,
    pub hurt_time: f32,
    pub hurt_duration: f32,
    pub hurt_dir: f32,
    pub invulnerable_time: f32,
    pub bob_offset: f32,
    pub passengers: Vec<i32>,
    pub boat_delta_rotation_degrees: f32,
    pub boat_status: Option<RemoteBoatStatus>,
    pub boat_old_status: Option<RemoteBoatStatus>,
    pub boat_water_level: f64,
    pub boat_land_friction: f32,
    pub boat_last_yd: f64,
    pub boat_main_supporting_block_pos: Option<BlockPos>,
    pub boat_on_ground_no_blocks: bool,
    pub boat_physics_position: Option<glam::DVec3>,
    pub boat_old_physics_position: Option<glam::DVec3>,
    pub boat_physics_velocity: Option<glam::DVec3>,
    pub boat_interpolation_target_position: Option<glam::DVec3>,
    pub boat_interpolation_previous_tick_position: Option<glam::DVec3>,
    pub boat_interpolation_previous_tick_rotation_degrees: Option<(f32, f32)>,
    pub minecart_lerp_steps: Vec<RustMinecartStep>,
    pub minecart_current_lerp_steps: Vec<RustMinecartStep>,
    pub minecart_lerp_delay: i32,
    pub minecart_old_lerp: RustMinecartStep,
    pub minecart_current_total_weight: f32,
    pub is_baby: bool,
    pub is_aggressive: bool,
    pub is_sitting: bool,
    pub is_bat_resting: bool,
    pub camel_dashing: bool,
    pub camel_last_pose_change_tick: i64,
    pub camel_dash_cooldown: i32,
    pub piglin_is_charging_crossbow: bool,
    pub pillager_is_charging_crossbow: bool,
    pub piglin_is_dancing: bool,
    pub camel_idle_animation_timeout: i32,
    pub camel_sit_animation_state: crate::render::model::animation::AnimationState,
    pub camel_sit_pose_animation_state: crate::render::model::animation::AnimationState,
    pub camel_sit_up_animation_state: crate::render::model::animation::AnimationState,
    pub camel_idle_animation_state: crate::render::model::animation::AnimationState,
    pub camel_dash_animation_state: crate::render::model::animation::AnimationState,
    pub bat_fly_animation_state: crate::render::model::animation::AnimationState,
    pub bat_rest_animation_state: crate::render::model::animation::AnimationState,
    pub creaking_can_move: bool,
    pub creaking_is_active: bool,
    pub creaking_is_tearing_down: bool,
    pub creaking_attack_remaining_ticks: i32,
    pub creaking_invulnerability_remaining_ticks: i32,
    pub creaking_attack_animation_state: crate::render::model::animation::AnimationState,
    pub creaking_invulnerability_animation_state: crate::render::model::animation::AnimationState,
    pub creaking_death_animation_state: crate::render::model::animation::AnimationState,
    pub walk_position_scale: f32,
}
impl RemoteEntityRenderState {
    pub fn new(
        id: i32,
        uuid: Option<uuid::Uuid>,
        kind: String,
        position: glam::Vec3,
        yaw: f32,
        pitch: f32,
        head_yaw: f32,
    ) -> Self {
        Self::new_with_precise_position(id, uuid, kind, position.as_dvec3(), yaw, pitch, head_yaw)
    }
    pub fn new_with_precise_position(
        id: i32,
        uuid: Option<uuid::Uuid>,
        kind: String,
        position: glam::DVec3,
        yaw: f32,
        pitch: f32,
        head_yaw: f32,
    ) -> Self {
        let pitch = java_entity_set_x_rot(pitch);
        let default_item_stack = default_thrown_item_stack_like_java(&kind);
        let yaw_degrees = yaw.to_degrees();
        let pitch_degrees = pitch.to_degrees();
        let is_living_entity = is_remote_living_entity_type(&kind);
        let is_text_display = remote_entity_type_key_eq(&kind, "text_display");
        let camera_attributes = is_living_entity
            .then(crate::gameplay::attributes::AttributeMap::living_camera_defaults);
        let received_attribute_snapshots = is_living_entity.then(HashMap::new);
        let position_f = position.as_vec3();
        Self {
            id,
            uuid,
            profile_name: None,
            skin_texture_url: None,
            skin_model: None,
            kind,
            spawn_data: 0,
            falling_block_state: default_falling_block_state_like_java(),
            falling_block_state_is_canonical: false,
            falling_block_start_pos: BlockPos::ZERO,
            acceleration_power: 0.1,
            leash_holder_id: 0,
            position_d: position,
            position: position_f,
            prev_position_d: position,
            prev_position: position_f,
            projectile_render_previous_position_d: position,
            projectile_render_previous_position: position_f,
            projectile_render_previous_yaw: yaw,
            projectile_render_previous_pitch: pitch,
            projectile_render_pose_pending: false,
            target_position_d: position,
            target_position: position_f,
            network_position_codec: RemoteVecDeltaCodec::new(position),
            last_tick_position_d: position,
            last_tick_position: position_f,
            java_y_rot_degrees: yaw_degrees,
            java_y_rot_o_degrees: yaw_degrees,
            target_y_rot_degrees: yaw_degrees,
            yaw,
            prev_yaw: yaw,
            target_yaw: yaw,
            prev_body_yaw: head_yaw,
            body_yaw: head_yaw,
            prev_head_yaw: head_yaw,
            head_yaw,
            target_head_yaw: head_yaw,
            head_yaw_steps: 0,
            mob_head_stable_time: 0,
            mob_last_stable_head_yaw: 0.0,
            swinging: false,
            swinging_hand: crate::render::entity::state::InteractionHand::MainHand,
            swing_time: 0,
            o_attack_anim: 0.0,
            attack_anim: 0.0,
            java_x_rot_degrees: pitch_degrees,
            java_x_rot_o_degrees: pitch_degrees,
            target_x_rot_degrees: pitch_degrees,
            pitch,
            prev_pitch: pitch,
            target_pitch: pitch,
            interpolation_steps: 0,
            velocity: glam::Vec3::ZERO,
            player_velocity_d: glam::DVec3::ZERO,
            player_lerp_velocity_d: glam::DVec3::ZERO,
            player_lerp_velocity_steps: 0,
            age_in_ticks: 0,
            prev_walk_animation_speed: 0.0,
            walk_animation_speed: 0.0,
            walk_animation_pos: 0.0,
            prev_swim_amount: 0.0,
            swim_amount: 0.0,
            prev_flap: 0.0,
            flap: 0.0,
            prev_flap_speed: 0.0,
            flap_speed: 0.0,
            flapping: 1.0,
            old_tentacle_angle: 0.0,
            tentacle_angle: 0.0,
            old_tentacle_movement: 0.0,
            tentacle_movement: 0.0,
            tentacle_speed: remote_squid_initial_tentacle_speed(id),
            old_tail_animation: 0.0,
            tail_animation: 0.0,
            tail_animation_speed: 0.0,
            old_spikes_animation: 1.0,
            spikes_animation: 1.0,
            prev_squish: 0.0,
            squish: 0.0,
            target_squish: 0.0,
            was_on_ground: false,
            item_stack: default_item_stack,
            item_frame_rotation: 0,
            main_hand_item_stack: el_core::item::ItemStack::empty(),
            off_hand_item_stack: el_core::item::ItemStack::empty(),
            head_equipment: el_core::item::ItemStack::empty(),
            chest_equipment: el_core::item::ItemStack::empty(),
            legs_equipment: el_core::item::ItemStack::empty(),
            feet_equipment: el_core::item::ItemStack::empty(),
            body_equipment: el_core::item::ItemStack::empty(),
            saddle_equipment: el_core::item::ItemStack::empty(),
            shared_flags: 0,
            no_gravity: false,
            living_flags: 0,
            living_health: 1.0,
            custom_name: None,
            custom_name_visible: false,
            mob_size: 1,
            pose: el_protocol_backend::BackendEntityPose::Standing,
            sleeping_pos: None,
            interaction_width: 1.0,
            interaction_height: 1.0,
            interaction_response: false,
            primed_tnt_fuse: 80,
            experience_orb_value: 0,
            armor_stand_client_flags: 0,
            armor_stand_head_pose: glam::Vec3::ZERO,
            armor_stand_body_pose: glam::Vec3::ZERO,
            armor_stand_left_arm_pose: glam::Vec3::new(-10.0, 0.0, -10.0),
            armor_stand_right_arm_pose: glam::Vec3::new(-15.0, 0.0, 10.0),
            armor_stand_left_leg_pose: glam::Vec3::new(-1.0, 0.0, -1.0),
            armor_stand_right_leg_pose: glam::Vec3::new(1.0, 0.0, 1.0),
            primed_tnt_block_state: BuiltInRegistries::blocks()
                .get_by_location(&ResourceLocation::new("minecraft", "tnt"))
                .default_state()
                .clone(),
            end_crystal_beam_target: None,
            end_crystal_show_bottom: true,
            abstract_arrow_flags: 0,
            abstract_arrow_pierce_level: 0,
            abstract_arrow_in_ground: false,
            abstract_arrow_first_tick: true,
            abstract_arrow_shake_time: 0,
            arrow_effect_color: -1,
            camera_attributes,
            received_attribute_snapshots,
            use_item_snapshot: el_core::item::ItemStack::empty(),
            use_item_remaining_ticks: 0.0,
            use_item_duration_ticks: 0.0,
            use_item_snapshot_hand: crate::render::entity::state::InteractionHand::MainHand,
            main_arm: crate::render::entity::state::HumanoidArm::Right,
            player_model_customisation: 0,
            text_display_state: is_text_display
                .then(crate::render::entity::display::TextDisplayState::default),
            avatar_state: crate::gameplay::entity::client_avatar_state::ClientAvatarState::new(),
            on_ground: false,
            dead: false,
            death_time: 0.0,
            hurt_time: 0.0,
            hurt_duration: 0.0,
            hurt_dir: 0.0,
            invulnerable_time: 0.0,
            bob_offset: (id as f32 * 0.15) % (std::f32::consts::PI * 2.0),
            passengers: Vec::new(),
            boat_delta_rotation_degrees: 0.0,
            boat_status: None,
            boat_old_status: None,
            boat_water_level: 0.0,
            boat_land_friction: 0.0,
            boat_last_yd: 0.0,
            boat_main_supporting_block_pos: None,
            boat_on_ground_no_blocks: false,
            boat_physics_position: None,
            boat_old_physics_position: None,
            boat_physics_velocity: None,
            boat_interpolation_target_position: None,
            boat_interpolation_previous_tick_position: None,
            boat_interpolation_previous_tick_rotation_degrees: None,
            minecart_lerp_steps: Vec::new(),
            minecart_current_lerp_steps: Vec::new(),
            minecart_lerp_delay: 0,
            minecart_old_lerp: RustMinecartStep::default(),
            minecart_current_total_weight: 0.0,
            is_baby: false,
            is_aggressive: false,
            is_sitting: false,
            is_bat_resting: false,
            camel_dashing: false,
            camel_last_pose_change_tick: 0,
            camel_dash_cooldown: 0,
            piglin_is_charging_crossbow: false,
            pillager_is_charging_crossbow: false,
            piglin_is_dancing: false,
            camel_idle_animation_timeout: 0,
            camel_sit_animation_state: crate::render::model::animation::AnimationState::new(),
            camel_sit_pose_animation_state: crate::render::model::animation::AnimationState::new(),
            camel_sit_up_animation_state: crate::render::model::animation::AnimationState::new(),
            camel_idle_animation_state: crate::render::model::animation::AnimationState::new(),
            camel_dash_animation_state: crate::render::model::animation::AnimationState::new(),
            bat_fly_animation_state: crate::render::model::animation::AnimationState::new(),
            bat_rest_animation_state: crate::render::model::animation::AnimationState::new(),
            creaking_can_move: true,
            creaking_is_active: false,
            creaking_is_tearing_down: false,
            creaking_attack_remaining_ticks: 0,
            creaking_invulnerability_remaining_ticks: 0,
            creaking_attack_animation_state: crate::render::model::animation::AnimationState::new(),
            creaking_invulnerability_animation_state:
                crate::render::model::animation::AnimationState::new(),
            creaking_death_animation_state: crate::render::model::animation::AnimationState::new(),
            walk_position_scale: 1.0,
        }
    }
    #[inline]
    pub fn set_render_position_d(&mut self, position: glam::DVec3) {
        self.position_d = position;
        self.position = position.as_vec3();
    }
    #[inline]
    pub fn set_render_prev_position_d(&mut self, position: glam::DVec3) {
        self.prev_position_d = position;
        self.prev_position = position.as_vec3();
    }
    #[inline]
    pub fn set_render_target_position_d(&mut self, position: glam::DVec3) {
        self.target_position_d = position;
        self.target_position = position.as_vec3();
    }
    #[inline]
    pub fn render_position_d_from_authority(&self) -> glam::DVec3 {
        if self.position == self.position_d.as_vec3() {
            self.position_d
        } else {
            self.position.as_dvec3()
        }
    }
    #[inline]
    pub fn render_prev_position_d_from_authority(&self) -> glam::DVec3 {
        if self.prev_position == self.prev_position_d.as_vec3() {
            self.prev_position_d
        } else {
            self.prev_position.as_dvec3()
        }
    }
    #[inline]
    pub fn render_target_position_d_from_authority(&self) -> glam::DVec3 {
        if self.target_position == self.target_position_d.as_vec3() {
            self.target_position_d
        } else {
            self.target_position.as_dvec3()
        }
    }
    pub fn sync_precise_render_pose_from_legacy_if_mutated(&mut self) {
        if self.position != self.position_d.as_vec3() {
            self.position_d = self.position.as_dvec3();
        }
        if self.prev_position != self.prev_position_d.as_vec3() {
            self.prev_position_d = self.prev_position.as_dvec3();
        }
        if self.target_position != self.target_position_d.as_vec3() {
            self.target_position_d = self.target_position.as_dvec3();
        }
        if self.last_tick_position != self.last_tick_position_d.as_vec3() {
            self.last_tick_position_d = self.last_tick_position.as_dvec3();
        }
    }
    pub fn gravity_from_default_like_java(&self, default_gravity: f64) -> f64 {
        if self.no_gravity {
            0.0
        } else {
            default_gravity
        }
    }
    pub fn begin_client_tick(&mut self) {
        self.sync_precise_render_pose_from_legacy_if_mutated();
        if is_remote_abstract_boat(&self.kind) {
            let position = self.boat_physics_position();
            self.boat_old_physics_position = Some(position);
            self.set_render_prev_position_d(position);
            self.copy_current_rotation_to_old_like_java();
        } else if is_remote_projectile_render_entity(&self.kind)
            && self.projectile_render_pose_pending
        {
            self.set_render_prev_position_d(self.projectile_render_previous_position_d);
            self.prev_yaw = self.projectile_render_previous_yaw;
            self.prev_pitch = self.projectile_render_previous_pitch;
            self.projectile_render_pose_pending = false;
        } else {
            self.set_render_prev_position_d(self.position_d);
            self.copy_current_rotation_to_old_like_java();
        }
        self.prev_body_yaw = self.body_yaw;
        self.prev_head_yaw = self.head_yaw;
    }
    pub fn note_projectile_render_pose_before_network_update(&mut self) {
        if is_remote_projectile_render_entity(&self.kind) && !self.projectile_render_pose_pending {
            self.projectile_render_previous_position_d = self.position_d;
            self.projectile_render_previous_position = self.position;
            self.projectile_render_previous_yaw = self.yaw;
            self.projectile_render_previous_pitch = self.pitch;
            self.projectile_render_pose_pending = true;
        }
    }
    pub fn tick_projectile_motion_like_java(&mut self, level: &ClientLevel) {
        debug_assert!(is_remote_projectile_render_entity(&self.kind));
        let key = remote_entity_type_key(&self.kind);
        let in_water = remote_entity_intersects_water(self, Some(level));
        let movement = self.velocity;
        match key {
            "arrow" | "spectral_arrow" | "trident" => {
                if self.abstract_arrow_in_ground && (self.abstract_arrow_flags & 2) == 0 {
                    self.velocity = glam::Vec3::ZERO;
                    return;
                }
                let no_physics = (self.abstract_arrow_flags & 2) != 0;
                if in_water {
                    self.velocity *= 0.6;
                }
                let movement = self.velocity;
                let horizontal = glam::Vec2::new(movement.x, movement.z).length();
                let pitch_target = movement.y.atan2(horizontal).to_degrees();
                let yaw_target = if no_physics {
                    (-movement.x).atan2(-movement.z).to_degrees()
                } else {
                    movement.x.atan2(movement.z).to_degrees()
                };
                self.set_x_rot_degrees_like_java(projectile_lerp_rotation_like_java(
                    self.x_rot_degrees(),
                    pitch_target,
                ));
                self.set_y_rot_degrees_like_java(projectile_lerp_rotation_like_java(
                    self.y_rot_degrees(),
                    yaw_target,
                ));
                self.set_render_position_d(self.position_d + movement.as_dvec3());
                if !in_water {
                    self.velocity *= 0.99;
                }
                if !no_physics && !self.abstract_arrow_in_ground {
                    self.velocity.y -= self.gravity_from_default_like_java(0.05) as f32;
                }
            }
            "ender_pearl" | "egg" | "snowball" | "potion" | "experience_bottle" => {
                self.velocity.y -= self.gravity_from_default_like_java(0.03) as f32;
                self.velocity *= if in_water { 0.8 } else { 0.99 };
                let movement = self.velocity;
                let horizontal = glam::Vec2::new(movement.x, movement.z).length();
                self.set_x_rot_degrees_like_java(projectile_lerp_rotation_like_java(
                    self.x_rot_degrees(),
                    movement.y.atan2(horizontal).to_degrees(),
                ));
                self.set_y_rot_degrees_like_java(projectile_lerp_rotation_like_java(
                    self.y_rot_degrees(),
                    movement.x.atan2(movement.z).to_degrees(),
                ));
                self.set_render_position_d(self.position_d + movement.as_dvec3());
            }
            "eye_of_ender" => {
                self.set_render_position_d(self.position_d + movement.as_dvec3());
            }
            "llama_spit" => {
                let horizontal = glam::Vec2::new(movement.x, movement.z).length();
                self.set_x_rot_degrees_like_java(projectile_lerp_rotation_like_java(
                    self.x_rot_degrees(),
                    movement.y.atan2(horizontal).to_degrees(),
                ));
                self.set_y_rot_degrees_like_java(projectile_lerp_rotation_like_java(
                    self.y_rot_degrees(),
                    movement.x.atan2(movement.z).to_degrees(),
                ));
                self.set_render_position_d(self.position_d + movement.as_dvec3());
                self.velocity *= 0.99;
                self.velocity.y -= self.gravity_from_default_like_java(0.06) as f32;
            }
            "fireball" | "small_fireball" | "dragon_fireball" | "wither_skull" | "wind_charge"
            | "breeze_wind_charge" => {
                let direction = movement.normalize_or_zero();
                self.velocity = (movement + direction * 0.1) * 0.95;
                self.set_render_position_d(self.position_d + self.velocity.as_dvec3());
                let horizontal = glam::Vec2::new(self.velocity.x, self.velocity.z).length();
                self.set_x_rot_degrees_like_java(projectile_lerp_rotation_like_java(
                    self.x_rot_degrees(),
                    self.velocity.y.atan2(horizontal).to_degrees(),
                ));
                self.set_y_rot_degrees_like_java(projectile_lerp_rotation_like_java(
                    self.y_rot_degrees(),
                    self.velocity.x.atan2(self.velocity.z).to_degrees(),
                ));
            }
            "firework_rocket" => {
                self.velocity = glam::Vec3::new(
                    self.velocity.x * 1.15,
                    self.velocity.y + 0.04,
                    self.velocity.z * 1.15,
                );
                self.set_render_position_d(self.position_d + self.velocity.as_dvec3());
                let horizontal = glam::Vec2::new(self.velocity.x, self.velocity.z).length();
                self.set_x_rot_degrees_like_java(projectile_lerp_rotation_like_java(
                    self.x_rot_degrees(),
                    self.velocity.y.atan2(horizontal).to_degrees(),
                ));
                self.set_y_rot_degrees_like_java(projectile_lerp_rotation_like_java(
                    self.y_rot_degrees(),
                    self.velocity.x.atan2(self.velocity.z).to_degrees(),
                ));
            }
            _ => {}
        }
    }
    pub fn tick_experience_orb_motion_like_java(
        &mut self,
        level: &ClientLevel,
        players: &[(glam::Vec3, f32)],
    ) {
        debug_assert_eq!(remote_entity_type_key(&self.kind), "experience_orb");
        let position = self.position;
        let bounding_box = AABB::new(
            f64::from(position.x) - 0.25,
            f64::from(position.y),
            f64::from(position.z) - 0.25,
            f64::from(position.x) + 0.25,
            f64::from(position.y) + 0.5,
            f64::from(position.z) + 0.25,
        );
        let colliding = !level.no_block_collision(&bounding_box);
        let in_water = remote_entity_intersects_water(self, Some(level));
        if in_water {
            self.velocity.x *= 0.99;
            self.velocity.y = (self.velocity.y + 0.0005).min(0.06);
            self.velocity.z *= 0.99;
        } else if !colliding {
            self.velocity.y -= self.gravity_from_default_like_java(0.03) as f32;
        }
        let nearest_player = players
            .iter()
            .filter_map(|(player_position, player_eye_height)| {
                let delta = glam::Vec3::new(
                    player_position.x - self.position.x,
                    player_position.y + player_eye_height / 2.0 - self.position.y,
                    player_position.z - self.position.z,
                );
                let length_squared = delta.length_squared();
                (length_squared > 0.0 && length_squared <= 64.0).then_some((length_squared, delta))
            })
            .min_by(|left, right| left.0.total_cmp(&right.0));
        if let Some((length_squared, delta)) = nearest_player {
            let power = 1.0 - length_squared.sqrt() / 8.0;
            self.velocity += delta.normalize() * (power * power * 0.1);
        }
        let attempted_movement = self.velocity.as_dvec3();
        let movement = collision::collide_movement_like_java(
            level,
            &bounding_box,
            &CollisionContext::empty(),
            self.on_ground,
            0.0,
            attempted_movement,
            &[],
        );
        self.set_render_position_d(self.position_d + movement);
        let vertical_collision_below =
            movement.y != attempted_movement.y && attempted_movement.y < 0.0;
        self.on_ground = vertical_collision_below;
        let fall_speed = attempted_movement.y;
        let mut friction = 0.98_f32;
        if self.on_ground {
            let below = BlockPos::new(
                self.position.x.floor() as i32,
                (self.position.y - 0.999999).floor() as i32,
                self.position.z.floor() as i32,
            );
            friction *= level.get_block_state(below).get_friction();
        }
        self.velocity *= friction;
        if vertical_collision_below && fall_speed < -0.03 {
            self.velocity.y = (-fall_speed * 0.4) as f32;
        }
    }
    pub fn tick_abstract_arrow_metadata_like_java(&mut self) {
        if !is_remote_abstract_arrow(&self.kind) {
            return;
        }
        if self.abstract_arrow_shake_time > 0 {
            self.abstract_arrow_shake_time -= 1;
        }
        self.abstract_arrow_first_tick = false;
    }
    pub fn y_rot_degrees(&self) -> f32 {
        self.java_y_rot_degrees
    }
    pub fn x_rot_degrees(&self) -> f32 {
        self.java_x_rot_degrees
    }
    pub fn set_y_rot_degrees_like_java(&mut self, y_rot: f32) {
        if y_rot.is_finite() {
            self.java_y_rot_degrees = y_rot;
            self.yaw = y_rot.to_radians();
        }
    }
    pub fn set_x_rot_degrees_like_java(&mut self, x_rot: f32) {
        if x_rot.is_finite() {
            let x_rot = java_entity_pitch_degrees(x_rot);
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
    pub fn set_interpolation_target_rotation_degrees(&mut self, y_rot: f32, x_rot: f32) {
        self.target_y_rot_degrees = y_rot;
        self.target_x_rot_degrees = x_rot;
        self.target_yaw = y_rot.to_radians();
        self.target_pitch = x_rot.to_radians();
    }
    pub fn sync_java_rotation_from_radian_mirrors_if_externally_mutated(&mut self) {
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
        if self.target_yaw.to_bits() != self.target_y_rot_degrees.to_radians().to_bits() {
            self.target_y_rot_degrees = self.target_yaw.to_degrees();
        }
        if self.target_pitch.to_bits() != self.target_x_rot_degrees.to_radians().to_bits() {
            self.target_x_rot_degrees = self.target_pitch.to_degrees();
        }
    }
    pub fn boat_physics_position(&self) -> glam::DVec3 {
        self.boat_physics_position
            .unwrap_or_else(|| self.render_position_d_from_authority())
    }
    pub fn boat_physics_velocity(&self) -> glam::DVec3 {
        self.boat_physics_velocity
            .unwrap_or_else(|| self.velocity.as_dvec3())
    }
    pub fn set_boat_physics_position(&mut self, position: glam::DVec3) {
        self.boat_physics_position = Some(position);
        self.set_render_position_d(position);
    }
    pub fn set_boat_physics_velocity(&mut self, velocity: glam::DVec3) {
        self.boat_physics_velocity = Some(velocity);
        self.velocity = velocity.as_vec3();
    }
    pub fn set_boat_physics_state(&mut self, position: glam::DVec3, velocity: glam::DVec3) {
        self.set_boat_physics_position(position);
        self.set_boat_physics_velocity(velocity);
    }
    pub fn zero_boat_delta_movement_like_java(&mut self) {
        debug_assert!(is_remote_abstract_boat(&self.kind));
        self.set_boat_physics_velocity(glam::DVec3::ZERO);
    }
    pub fn cancel_interpolation_like_java(&mut self) {
        self.interpolation_steps = 0;
        self.boat_interpolation_previous_tick_position = None;
        self.boat_interpolation_previous_tick_rotation_degrees = None;
    }
    pub fn interpolate_boat_to_degrees(
        &mut self,
        target_position: glam::DVec3,
        target_yaw_degrees: f32,
        target_pitch_degrees: f32,
    ) {
        debug_assert!(is_remote_abstract_boat(&self.kind));
        self.sync_java_rotation_from_radian_mirrors_if_externally_mutated();
        let target_changed = self.interpolation_steps <= 0
            || self.boat_interpolation_target_position != Some(target_position)
            || self.target_y_rot_degrees.to_bits() != target_yaw_degrees.to_bits()
            || self.target_x_rot_degrees.to_bits() != target_pitch_degrees.to_bits();
        if !target_changed {
            return;
        }
        self.boat_interpolation_target_position = Some(target_position);
        self.set_render_target_position_d(target_position);
        self.set_interpolation_target_rotation_degrees(target_yaw_degrees, target_pitch_degrees);
        self.interpolation_steps = REMOTE_ENTITY_INTERPOLATION_STEPS;
        self.boat_interpolation_previous_tick_position = Some(self.boat_physics_position());
        self.boat_interpolation_previous_tick_rotation_degrees =
            Some((self.java_x_rot_degrees, self.java_y_rot_degrees));
    }
    pub fn move_or_interpolate_boat_to_degrees(
        &mut self,
        position: Option<glam::DVec3>,
        yaw_degrees: Option<f32>,
        pitch_degrees: Option<f32>,
    ) {
        debug_assert!(is_remote_abstract_boat(&self.kind));
        self.sync_java_rotation_from_radian_mirrors_if_externally_mutated();
        let active = self.interpolation_steps > 0;
        let target_position = position.unwrap_or_else(|| {
            if active {
                self.boat_interpolation_target_position
                    .unwrap_or_else(|| self.render_target_position_d_from_authority())
            } else {
                self.boat_physics_position()
            }
        });
        let target_yaw_degrees = yaw_degrees.unwrap_or(if active {
            self.target_y_rot_degrees
        } else {
            self.java_y_rot_degrees
        });
        let target_pitch_degrees = pitch_degrees.unwrap_or(if active {
            self.target_x_rot_degrees
        } else {
            self.java_x_rot_degrees
        });
        self.interpolate_boat_to_degrees(target_position, target_yaw_degrees, target_pitch_degrees);
    }
    pub fn tick_boat_interpolation_like_java(
        &mut self,
        level: &ClientLevel,
        entity_colliders: &[el_math::VoxelShape],
    ) {
        debug_assert!(is_remote_abstract_boat(&self.kind));
        self.sync_java_rotation_from_radian_mirrors_if_externally_mutated();
        if self.interpolation_steps <= 0 {
            self.cancel_interpolation_like_java();
            return;
        }
        let current_position = self.boat_physics_position();
        let mut target_position = self
            .boat_interpolation_target_position
            .unwrap_or_else(|| self.render_target_position_d_from_authority());
        if let Some(previous_position) = self.boat_interpolation_previous_tick_position {
            let delta_since_last_interpolation = current_position - previous_position;
            let source_box = remote_abstract_boat_aabb(current_position);
            let target_box =
                remote_abstract_boat_aabb(target_position + delta_since_last_interpolation);
            let context = remote_abstract_boat_collision_context(self, current_position.y);
            if level.no_collision_with_context_and_entities(
                Some(&source_box),
                &target_box,
                &context,
                entity_colliders,
            ) {
                target_position += delta_since_last_interpolation;
                self.boat_interpolation_target_position = Some(target_position);
                self.set_render_target_position_d(target_position);
            }
        }
        if let Some((previous_x_rot, previous_y_rot)) =
            self.boat_interpolation_previous_tick_rotation_degrees
        {
            self.set_interpolation_target_rotation_degrees(
                self.target_y_rot_degrees + (self.java_y_rot_degrees - previous_y_rot),
                self.target_x_rot_degrees + (self.java_x_rot_degrees - previous_x_rot),
            );
        }
        let alpha = 1.0 / f64::from(self.interpolation_steps);
        let new_position = current_position.lerp(target_position, alpha);
        let new_y_rot = rot_lerp_degrees(alpha, self.java_y_rot_degrees, self.target_y_rot_degrees);
        let new_x_rot = (f64::from(self.java_x_rot_degrees)
            + (f64::from(self.target_x_rot_degrees) - f64::from(self.java_x_rot_degrees)) * alpha)
            as f32;
        self.set_boat_physics_position(new_position);
        self.set_y_rot_degrees_like_java(java_entity_yaw_degrees(new_y_rot));
        self.set_x_rot_degrees_like_java(new_x_rot);
        self.interpolation_steps -= 1;
        self.boat_interpolation_previous_tick_position = Some(new_position);
        self.boat_interpolation_previous_tick_rotation_degrees =
            Some((self.java_x_rot_degrees, self.java_y_rot_degrees));
    }
    pub fn position_move_rotation(&self) -> RemotePositionMoveRotation {
        let interpolating = self.interpolation_steps > 0;
        RemotePositionMoveRotation {
            position: if interpolating {
                if is_remote_abstract_boat(&self.kind) {
                    self.boat_interpolation_target_position
                        .unwrap_or_else(|| self.render_target_position_d_from_authority())
                } else {
                    self.render_target_position_d_from_authority()
                }
            } else if is_remote_abstract_boat(&self.kind) {
                self.boat_physics_position()
            } else {
                self.render_position_d_from_authority()
            },
            velocity: if is_remote_abstract_boat(&self.kind) {
                self.boat_physics_velocity()
            } else if is_remote_player_entity(&self.kind) {
                self.player_velocity_d
            } else {
                self.velocity.as_dvec3()
            },
            yaw_degrees: if interpolating {
                self.target_y_rot_degrees
            } else {
                self.java_y_rot_degrees
            },
            pitch_degrees: if interpolating {
                self.target_x_rot_degrees
            } else {
                self.java_x_rot_degrees
            },
        }
    }
    pub fn old_position_move_rotation(&self) -> RemotePositionMoveRotation {
        RemotePositionMoveRotation {
            position: if is_remote_abstract_boat(&self.kind) {
                self.boat_old_physics_position
                    .unwrap_or_else(|| self.render_prev_position_d_from_authority())
            } else {
                self.render_prev_position_d_from_authority()
            },
            velocity: glam::DVec3::ZERO,
            yaw_degrees: self.java_y_rot_o_degrees,
            pitch_degrees: self.java_x_rot_o_degrees,
        }
    }
    pub fn set_teleport_pose_direct(
        &mut self,
        current: RemotePositionMoveRotation,
        old: RemotePositionMoveRotation,
    ) {
        self.set_render_position_d(current.position);
        self.set_packet_velocity(current.velocity);
        self.set_rotation_degrees_like_java(current.yaw_degrees, current.pitch_degrees);
        self.set_render_prev_position_d(old.position);
        self.set_old_rotation_degrees_raw_like_java(old.yaw_degrees, old.pitch_degrees);
        if is_remote_abstract_boat(&self.kind) {
            self.boat_physics_position = Some(current.position);
            self.boat_old_physics_position = Some(old.position);
            self.boat_physics_velocity = Some(current.velocity);
        }
    }
    pub fn apply_move_entity_packet(
        &mut self,
        xa: i16,
        ya: i16,
        za: i16,
        has_position: bool,
        yaw_degrees: Option<f32>,
        pitch_degrees: Option<f32>,
        on_ground: bool,
        locally_authoritative: bool,
    ) {
        if !locally_authoritative
            && (has_position || yaw_degrees.is_some() || pitch_degrees.is_some())
        {
            self.note_projectile_render_pose_before_network_update();
        }
        if locally_authoritative {
            let decoded = self.network_position_codec.decode(xa, ya, za);
            self.network_position_codec.set_base(decoded);
            return;
        }
        let target_position_d = has_position.then(|| {
            let decoded = self.network_position_codec.decode(xa, ya, za);
            self.network_position_codec.set_base(decoded);
            decoded
        });
        if target_position_d.is_some() || yaw_degrees.is_some() || pitch_degrees.is_some() {
            if is_remote_abstract_boat(&self.kind) {
                self.move_or_interpolate_boat_to_degrees(
                    target_position_d,
                    yaw_degrees,
                    pitch_degrees,
                );
            } else {
                self.move_or_interpolate_to_d_degrees(
                    target_position_d,
                    yaw_degrees,
                    pitch_degrees,
                );
            }
        }
        self.on_ground = on_ground;
    }
    pub fn interpolate_to_degrees(
        &mut self,
        target_position: glam::Vec3,
        target_yaw_degrees: f32,
        target_pitch_degrees: f32,
    ) {
        self.interpolate_to_d_degrees(
            target_position.as_dvec3(),
            target_yaw_degrees,
            target_pitch_degrees,
        );
    }
    pub fn interpolate_to_d_degrees(
        &mut self,
        target_position: glam::DVec3,
        target_yaw_degrees: f32,
        target_pitch_degrees: f32,
    ) {
        self.sync_precise_render_pose_from_legacy_if_mutated();
        if is_remote_abstract_boat(&self.kind) {
            self.interpolate_boat_to_degrees(
                target_position,
                target_yaw_degrees,
                target_pitch_degrees,
            );
            return;
        }
        let target_changed = self.interpolation_steps <= 0
            || self.target_position_d != target_position
            || self.target_y_rot_degrees.to_bits() != target_yaw_degrees.to_bits()
            || self.target_x_rot_degrees.to_bits() != target_pitch_degrees.to_bits();
        if target_changed {
            self.set_render_target_position_d(target_position);
            self.set_interpolation_target_rotation_degrees(
                target_yaw_degrees,
                target_pitch_degrees,
            );
            self.interpolation_steps = REMOTE_ENTITY_INTERPOLATION_STEPS;
        }
    }
    pub fn interpolate_to(
        &mut self,
        target_position: glam::Vec3,
        target_yaw: f32,
        target_pitch: f32,
    ) {
        self.sync_java_rotation_from_radian_mirrors_if_externally_mutated();
        self.interpolate_to_degrees(
            target_position,
            target_yaw.to_degrees(),
            target_pitch.to_degrees(),
        );
    }
    pub fn move_or_interpolate_to(
        &mut self,
        position: Option<glam::Vec3>,
        yaw: Option<f32>,
        pitch: Option<f32>,
    ) {
        self.sync_java_rotation_from_radian_mirrors_if_externally_mutated();
        self.move_or_interpolate_to_degrees(
            position,
            yaw.map(f32::to_degrees),
            pitch.map(f32::to_degrees),
        );
    }
    pub fn move_or_interpolate_to_degrees(
        &mut self,
        position: Option<glam::Vec3>,
        yaw_degrees: Option<f32>,
        pitch_degrees: Option<f32>,
    ) {
        self.move_or_interpolate_to_d_degrees(
            position.map(|position| position.as_dvec3()),
            yaw_degrees,
            pitch_degrees,
        );
    }
    pub fn move_or_interpolate_to_d_degrees(
        &mut self,
        position: Option<glam::DVec3>,
        yaw_degrees: Option<f32>,
        pitch_degrees: Option<f32>,
    ) {
        self.sync_precise_render_pose_from_legacy_if_mutated();
        if is_remote_abstract_boat(&self.kind) {
            self.move_or_interpolate_boat_to_degrees(position, yaw_degrees, pitch_degrees);
            return;
        }
        match self.interpolation_kind() {
            RemoteEntityInterpolationKind::DefaultThreeTicks => {
                let active = self.interpolation_steps > 0;
                let target_position = position.unwrap_or(if active {
                    self.target_position_d
                } else {
                    self.position_d
                });
                if is_remote_abstract_boat(&self.kind) && !active && position.is_none() {
                    self.boat_interpolation_target_position = Some(self.boat_physics_position());
                }
                let target_yaw_degrees = yaw_degrees.unwrap_or(if active {
                    self.target_y_rot_degrees
                } else {
                    self.java_y_rot_degrees
                });
                let target_pitch_degrees = pitch_degrees.unwrap_or(if active {
                    self.target_x_rot_degrees
                } else {
                    self.java_x_rot_degrees
                });
                self.interpolate_to_d_degrees(
                    target_position,
                    target_yaw_degrees,
                    target_pitch_degrees,
                );
            }
            RemoteEntityInterpolationKind::DisplaySnap => {
                self.snap_to_d_degrees(
                    position.unwrap_or(self.position_d),
                    yaw_degrees.unwrap_or(self.java_y_rot_degrees),
                    pitch_degrees.unwrap_or(self.java_x_rot_degrees),
                );
                self.cancel_interpolation_like_java();
            }
            RemoteEntityInterpolationKind::None => {
                if let Some(position) = position {
                    self.set_render_position_d(position);
                }
                if let Some(yaw_degrees) = yaw_degrees {
                    self.set_y_rot_degrees_like_java(java_entity_yaw_degrees(yaw_degrees));
                }
                if let Some(pitch_degrees) = pitch_degrees {
                    self.set_x_rot_degrees_like_java(java_entity_pitch_degrees(pitch_degrees));
                }
            }
            RemoteEntityInterpolationKind::ExistingMinecartPipeline => {
                let target_position = position.unwrap_or(self.target_position_d);
                let target_yaw_degrees = yaw_degrees.unwrap_or(self.target_y_rot_degrees);
                let target_pitch_degrees = pitch_degrees.unwrap_or(self.target_x_rot_degrees);
                self.interpolate_to_d_degrees(
                    target_position,
                    target_yaw_degrees,
                    target_pitch_degrees,
                );
            }
        }
    }
    pub fn snap_to(&mut self, position: glam::Vec3, yaw: f32, pitch: f32) {
        self.sync_java_rotation_from_radian_mirrors_if_externally_mutated();
        self.snap_to_degrees(position, yaw.to_degrees(), pitch.to_degrees());
    }
    pub fn snap_to_degrees(&mut self, position: glam::Vec3, yaw_degrees: f32, pitch_degrees: f32) {
        self.snap_to_d_degrees(position.as_dvec3(), yaw_degrees, pitch_degrees);
    }
    pub fn snap_to_d_degrees(
        &mut self,
        position: glam::DVec3,
        yaw_degrees: f32,
        pitch_degrees: f32,
    ) {
        self.set_render_position_d(position);
        self.set_rotation_degrees_like_java(yaw_degrees, pitch_degrees);
        self.set_render_prev_position_d(position);
        self.copy_current_rotation_to_old_like_java();
    }
    pub fn abs_snap_to(&mut self, position: glam::Vec3, yaw: f32, pitch: f32) {
        self.sync_java_rotation_from_radian_mirrors_if_externally_mutated();
        self.abs_snap_to_degrees(position, yaw.to_degrees(), pitch.to_degrees());
    }
    pub fn abs_snap_to_degrees(
        &mut self,
        position: glam::Vec3,
        yaw_degrees: f32,
        pitch_degrees: f32,
    ) {
        self.abs_snap_to_d_degrees(position.as_dvec3(), yaw_degrees, pitch_degrees);
    }
    pub fn abs_snap_to_d_degrees(
        &mut self,
        position: glam::DVec3,
        yaw_degrees: f32,
        pitch_degrees: f32,
    ) {
        self.set_render_position_d(position);
        self.set_y_rot_degrees_like_java(java_entity_yaw_degrees(yaw_degrees));
        self.set_x_rot_degrees_like_java(pitch_degrees.clamp(-90.0, 90.0));
        self.set_render_prev_position_d(position);
        self.copy_current_rotation_to_old_like_java();
    }
    pub fn interpolation_kind(&self) -> RemoteEntityInterpolationKind {
        let key = remote_entity_type_key(&self.kind);
        if key.contains("minecart") {
            return RemoteEntityInterpolationKind::ExistingMinecartPipeline;
        }
        if matches!(key, "block_display" | "item_display" | "text_display") {
            return RemoteEntityInterpolationKind::DisplaySnap;
        }
        if (is_remote_living_entity_type(&self.kind) && key != "shulker")
            || is_remote_abstract_boat(&self.kind)
            || matches!(key, "experience_orb" | "fishing_bobber")
        {
            RemoteEntityInterpolationKind::DefaultThreeTicks
        } else {
            RemoteEntityInterpolationKind::None
        }
    }
    pub fn get_cart_lerp_step(&self, partial_tick: f32) -> RustMinecartStep {
        if self.minecart_current_lerp_steps.is_empty() {
            return RustMinecartStep {
                position: self.position,
                movement: self.velocity,
                yaw_radians: self.yaw,
                pitch_radians: self.pitch,
                weight: 0.0,
            };
        }
        let alpha = (3.0 - self.minecart_lerp_delay as f32 + partial_tick) / 3.0;
        let alpha = alpha.clamp(0.0, 1.0);
        let mut count_up = 0.0;
        let mut index = 0;
        let mut found_index = false;
        for (i, step) in self.minecart_current_lerp_steps.iter().enumerate() {
            if step.weight > 0.0 {
                count_up += step.weight;
                if count_up >= self.minecart_current_total_weight * alpha {
                    index = i;
                    found_index = true;
                    break;
                }
            }
        }
        if !found_index {
            index = self.minecart_current_lerp_steps.len().saturating_sub(1);
        }
        let current_step = &self.minecart_current_lerp_steps[index];
        let previous_step = if index > 0 {
            &self.minecart_current_lerp_steps[index - 1]
        } else {
            &self.minecart_old_lerp
        };
        let step_alpha = if current_step.weight > 0.0 {
            let current = count_up - current_step.weight;
            ((alpha * self.minecart_current_total_weight - current) / current_step.weight)
                .clamp(0.0, 1.0)
        } else {
            1.0
        };
        RustMinecartStep {
            position: previous_step
                .position
                .lerp(current_step.position, step_alpha),
            movement: previous_step
                .movement
                .lerp(current_step.movement, step_alpha),
            yaw_radians: rot_lerp_radians(
                step_alpha,
                previous_step.yaw_radians,
                current_step.yaw_radians,
            ),
            pitch_radians: rot_lerp_radians(
                step_alpha,
                previous_step.pitch_radians,
                current_step.pitch_radians,
            ),
            weight: 0.0,
        }
    }
    pub fn tick_interpolation(&mut self) {
        self.sync_precise_render_pose_from_legacy_if_mutated();
        self.sync_java_rotation_from_radian_mirrors_if_externally_mutated();
        let is_minecart = remote_entity_type_key(&self.kind).contains("minecart");
        if is_minecart {
            self.minecart_lerp_delay -= 1;
            if self.minecart_lerp_delay <= 0 {
                self.minecart_old_lerp = RustMinecartStep {
                    position: self.position,
                    movement: self.velocity,
                    yaw_radians: self.yaw,
                    pitch_radians: self.pitch,
                    weight: 0.0,
                };
                self.minecart_current_lerp_steps.clear();
                if !self.minecart_lerp_steps.is_empty() {
                    self.minecart_current_lerp_steps
                        .extend(self.minecart_lerp_steps.drain(..));
                    self.minecart_current_total_weight = 0.0;
                    for step in &self.minecart_current_lerp_steps {
                        self.minecart_current_total_weight += step.weight;
                    }
                    self.minecart_lerp_delay = if self.minecart_current_total_weight == 0.0 {
                        0
                    } else {
                        3
                    };
                }
            }
            if !self.minecart_current_lerp_steps.is_empty() {
                let interpolated = self.get_cart_lerp_step(1.0);
                self.set_render_position_d(interpolated.position.as_dvec3());
                self.velocity = interpolated.movement;
                self.set_rotation_degrees_like_java(
                    interpolated.yaw_radians.to_degrees(),
                    interpolated.pitch_radians.to_degrees(),
                );
            }
            return;
        }
        if self.interpolation_steps <= 0 {
            return;
        }
        let alpha = 1.0 / self.interpolation_steps as f64;
        self.set_render_position_d(self.position_d.lerp(self.target_position_d, alpha));
        let y_rot = rot_lerp_degrees(alpha, self.java_y_rot_degrees, self.target_y_rot_degrees);
        let x_rot = (f64::from(self.java_x_rot_degrees)
            + (f64::from(self.target_x_rot_degrees) - f64::from(self.java_x_rot_degrees)) * alpha)
            as f32;
        self.set_y_rot_degrees_like_java(java_entity_yaw_degrees(y_rot));
        self.set_x_rot_degrees_like_java(java_entity_pitch_degrees(x_rot));
        self.interpolation_steps -= 1;
    }
    pub fn render_position_d(&self, partial_tick: f32) -> glam::DVec3 {
        let is_minecart = remote_entity_type_key(&self.kind).contains("minecart");
        if is_minecart && !self.minecart_current_lerp_steps.is_empty() {
            return self.get_cart_lerp_step(partial_tick).position.as_dvec3();
        }
        let partial_tick = f64::from(partial_tick.clamp(0.0, 1.0));
        self.render_prev_position_d_from_authority()
            .lerp(self.render_position_d_from_authority(), partial_tick)
    }
    pub fn render_position(&self, partial_tick: f32) -> glam::Vec3 {
        self.render_position_d(partial_tick).as_vec3()
    }
    pub fn render_yaw(&self, partial_tick: f32) -> f32 {
        let is_minecart = remote_entity_type_key(&self.kind).contains("minecart");
        if is_minecart && !self.minecart_current_lerp_steps.is_empty() {
            return self.get_cart_lerp_step(partial_tick).yaw_radians;
        }
        rot_lerp_radians(partial_tick.clamp(0.0, 1.0), self.prev_yaw, self.yaw)
    }
    pub fn render_view_yaw(&self, partial_tick: f32) -> f32 {
        let partial_tick = partial_tick.clamp(0.0, 1.0);
        if self.uses_living_body_rotation() {
            rot_lerp_radians(partial_tick, self.prev_head_yaw, self.head_yaw)
        } else {
            self.render_yaw(partial_tick)
        }
    }
    pub fn render_body_yaw(&self, partial_tick: f32) -> f32 {
        if self.uses_living_body_rotation() {
            rot_lerp_radians(
                partial_tick.clamp(0.0, 1.0),
                self.prev_body_yaw,
                self.body_yaw,
            )
        } else {
            self.render_yaw(partial_tick)
        }
    }
    pub fn uses_living_body_rotation(&self) -> bool {
        is_remote_living_entity_type(&self.kind)
    }
    pub fn camera_entity_settings(&self) -> Option<crate::gameplay::camera::CameraEntitySettings> {
        self.camera_attributes.as_ref().map(|attributes| {
            crate::gameplay::camera::CameraEntitySettings {
                scale: attributes.value_f32(crate::gameplay::attributes::SCALE),
                distance: attributes.value_f32(crate::gameplay::attributes::CAMERA_DISTANCE),
            }
        })
    }
    pub fn apply_received_attribute_snapshot(
        &mut self,
        attribute_id: &str,
        base: f64,
        modifiers: Vec<crate::gameplay::attributes::AttributeModifier>,
    ) -> bool {
        let Some(received_attribute_snapshots) = self.received_attribute_snapshots.as_mut() else {
            return false;
        };
        received_attribute_snapshots.insert(
            attribute_id.to_owned(),
            RemoteAttributeSnapshot { base, modifiers },
        );
        true
    }
    pub fn apply_remote_attribute_snapshot(
        &mut self,
        attribute_id: &str,
        base: f64,
        modifiers: Vec<crate::gameplay::attributes::AttributeModifier>,
    ) -> bool {
        let is_living_entity =
            self.apply_received_attribute_snapshot(attribute_id, base, modifiers.clone());
        if is_living_entity && let Some(camera_attributes) = self.camera_attributes.as_mut() {
            let _ = camera_attributes.apply_snapshot(attribute_id, base, modifiers);
        }
        is_living_entity
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply_equipment_slots(
        &mut self,
        main_hand: Option<el_core::item::ItemStack>,
        off_hand: Option<el_core::item::ItemStack>,
        head: Option<el_core::item::ItemStack>,
        chest: Option<el_core::item::ItemStack>,
        legs: Option<el_core::item::ItemStack>,
        feet: Option<el_core::item::ItemStack>,
        body: Option<el_core::item::ItemStack>,
        saddle: Option<el_core::item::ItemStack>,
    ) {
        if let Some(main_hand) = main_hand {
            self.main_hand_item_stack = main_hand;
        }
        if let Some(off_hand) = off_hand {
            self.off_hand_item_stack = off_hand;
        }
        if let Some(head) = head {
            self.head_equipment = head;
        }
        if let Some(chest) = chest {
            self.chest_equipment = chest;
        }
        if let Some(legs) = legs {
            self.legs_equipment = legs;
        }
        if let Some(feet) = feet {
            self.feet_equipment = feet;
        }
        if let Some(body) = body {
            self.body_equipment = body;
        }
        if let Some(saddle) = saddle {
            self.saddle_equipment = saddle;
        }
    }
    #[inline]
    pub fn is_sleeping(&self) -> bool {
        self.sleeping_pos.is_some()
    }
    pub fn camera_eye_height(&self) -> f32 {
        if is_remote_player_entity(&self.kind) {
            let pose = remote_avatar_pose(self.pose);
            let scale = if matches!(pose, PlayerPose::Sleeping | PlayerPose::Dying) {
                1.0
            } else {
                self.camera_entity_settings()
                    .map(|settings| settings.scale)
                    .unwrap_or(1.0)
            };
            return pose.eye_height() * scale;
        }
        get_entity_eye_height(&self.kind)
    }
    pub fn tick_generic_living_body_rotation(&mut self) {
        let horizontal_delta = self.position - self.prev_position;
        let mut body_target = self.body_yaw;
        if horizontal_delta.x * horizontal_delta.x + horizontal_delta.z * horizontal_delta.z
            > 0.0025000002
        {
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
        if self.attack_anim > 0.0 {
            body_target = self.yaw;
        }
        self.body_yaw += wrap_radians(body_target - self.body_yaw) * 0.3;
        let head_diff = wrap_radians(self.yaw - self.body_yaw);
        let max_head_rotation = 50.0_f32.to_radians();
        if head_diff.abs() > max_head_rotation {
            self.body_yaw += head_diff - head_diff.signum() * max_head_rotation;
        }
    }
    pub fn rotate_yaw_if_necessary(base_yaw: f32, target_yaw: f32, max_delta: f32) -> f32 {
        target_yaw - wrap_radians(target_yaw - base_yaw).clamp(-max_delta, max_delta)
    }
    pub fn mob_max_head_yaw(&self) -> f32 {
        let degrees = match remote_entity_type_key(&self.kind) {
            "axolotl" | "dolphin" => 1.0_f32,
            "frog" => 5.0_f32,
            "goat" => 15.0_f32,
            "breeze" => 30.0_f32,
            "ravager" => 45.0_f32,
            "sniffer" => 50.0_f32,
            _ => 75.0_f32,
        };
        degrees.to_radians()
    }
    pub fn tick_mob_body_rotation(&mut self, first_passenger_is_mob: bool) {
        let horizontal_delta = self.position - self.prev_position;
        let max_head_yaw = self.mob_max_head_yaw();
        if horizontal_delta.x * horizontal_delta.x + horizontal_delta.z * horizontal_delta.z
            > 2.500_000_3e-7
        {
            self.body_yaw = self.yaw;
            self.head_yaw =
                Self::rotate_yaw_if_necessary(self.head_yaw, self.body_yaw, max_head_yaw);
            self.mob_last_stable_head_yaw = self.head_yaw;
            self.mob_head_stable_time = 0;
            return;
        }
        if first_passenger_is_mob {
            return;
        }
        if (self.head_yaw - self.mob_last_stable_head_yaw).abs() > 15.0_f32.to_radians() {
            self.mob_head_stable_time = 0;
            self.mob_last_stable_head_yaw = self.head_yaw;
            self.body_yaw =
                Self::rotate_yaw_if_necessary(self.body_yaw, self.head_yaw, max_head_yaw);
        } else {
            self.mob_head_stable_time += 1;
            if self.mob_head_stable_time > 10 {
                let face_forward_fraction =
                    ((self.mob_head_stable_time - 10) as f32 / 10.0).clamp(0.0, 1.0);
                let remaining = max_head_yaw * (1.0 - face_forward_fraction);
                self.body_yaw =
                    Self::rotate_yaw_if_necessary(self.body_yaw, self.head_yaw, remaining);
            }
        }
    }
    pub fn tick_living_body_rotation(&mut self, first_passenger_is_mob: bool) {
        match remote_entity_type_key(&self.kind) {
            "armor_stand" => {
                self.prev_body_yaw = self.prev_yaw;
                self.body_yaw = self.yaw;
            }
            "phantom" => {
                self.head_yaw = self.body_yaw;
                self.body_yaw = self.yaw;
            }
            "shulker" => {}
            "happy_ghast" => {
                if !self.passengers.is_empty() {
                    self.head_yaw = self.yaw;
                    self.body_yaw = self.head_yaw;
                }
                self.tick_mob_body_rotation(first_passenger_is_mob);
            }
            "armadillo" | "camel" | "camel_husk" | "creaking" => {}
            _ if is_remote_mob_entity_type(&self.kind) => {
                self.tick_mob_body_rotation(first_passenger_is_mob);
            }
            _ => self.tick_generic_living_body_rotation(),
        }
    }
    const DEFAULT_SWING_DURATION: i32 = 6;
    pub fn current_swing_duration(&self) -> i32 {
        Self::DEFAULT_SWING_DURATION
    }
    pub fn swing(&mut self, hand: crate::render::entity::state::InteractionHand) {
        let swing_duration = self.current_swing_duration();
        if !self.swinging || self.swing_time >= swing_duration / 2 || self.swing_time < 0 {
            self.swing_time = -1;
            self.swinging = true;
            self.swinging_hand = hand;
        }
    }
    pub fn tick_attack_animation(&mut self) {
        self.o_attack_anim = self.attack_anim;
        let swing_duration = self.current_swing_duration();
        if self.swinging {
            self.swing_time += 1;
            if self.swing_time >= swing_duration {
                self.swing_time = 0;
                self.swinging = false;
            }
        } else {
            self.swing_time = 0;
        }
        self.attack_anim = self.swing_time as f32 / swing_duration as f32;
    }
    pub fn render_attack_anim(&self, partial_tick: f32) -> f32 {
        let mut diff = self.attack_anim - self.o_attack_anim;
        if diff < 0.0 {
            diff += 1.0;
        }
        self.o_attack_anim + diff * partial_tick.clamp(0.0, 1.0)
    }
    pub fn normalize_old_living_rotations(&mut self) {
        self.java_y_rot_o_degrees =
            unwrap_old_rotation_degrees(self.java_y_rot_degrees, self.java_y_rot_o_degrees);
        self.java_x_rot_o_degrees =
            unwrap_old_rotation_degrees(self.java_x_rot_degrees, self.java_x_rot_o_degrees);
        self.prev_yaw = self.java_y_rot_o_degrees.to_radians();
        self.prev_pitch = self.java_x_rot_o_degrees.to_radians();
        self.prev_body_yaw = unwrap_old_rotation(self.body_yaw, self.prev_body_yaw);
        self.prev_head_yaw = unwrap_old_rotation(self.head_yaw, self.prev_head_yaw);
    }
    pub fn render_pitch(&self, partial_tick: f32) -> f32 {
        let is_minecart = remote_entity_type_key(&self.kind).contains("minecart");
        if is_minecart && !self.minecart_current_lerp_steps.is_empty() {
            return self.get_cart_lerp_step(partial_tick).pitch_radians;
        }
        let alpha = partial_tick.clamp(0.0, 1.0);
        self.prev_pitch + (self.pitch - self.prev_pitch) * alpha
    }
    pub fn is_visually_swimming(&self) -> bool {
        self.pose == el_protocol_backend::BackendEntityPose::Swimming
    }
    pub fn tick_swim_amount(&mut self) {
        self.prev_swim_amount = self.swim_amount;
        self.swim_amount = if self.is_visually_swimming() {
            (self.swim_amount + 0.09).min(1.0)
        } else {
            (self.swim_amount - 0.09).max(0.0)
        };
    }
    pub fn set_packet_velocity(&mut self, velocity: glam::DVec3) {
        self.velocity = velocity.as_vec3();
        if is_remote_player_entity(&self.kind) {
            self.player_velocity_d = velocity;
        }
    }
    pub fn lerp_packet_velocity(&mut self, velocity: glam::DVec3) {
        if is_remote_player_entity(&self.kind) {
            self.player_lerp_velocity_d = velocity;
            self.player_lerp_velocity_steps = 3;
        } else {
            self.set_packet_velocity(velocity);
            if is_remote_abstract_boat(&self.kind) {
                self.boat_physics_velocity = Some(velocity);
            }
        }
    }
    pub fn tick_player_velocity_interpolation(&mut self) {
        if self.player_lerp_velocity_steps > 0 {
            let velocity = self.player_velocity_d
                + (self.player_lerp_velocity_d - self.player_velocity_d)
                    / f64::from(self.player_lerp_velocity_steps);
            self.set_packet_velocity(velocity);
            self.player_lerp_velocity_steps -= 1;
        }
    }
    pub fn apply_death_entity_event(&mut self) {
        if is_remote_player_entity(&self.kind) {
            return;
        }
        self.living_health = 0.0;
        self.dead = true;
    }
    pub fn tick_death_animation(&mut self) {
        if self.uses_living_body_rotation() && self.living_health <= 0.0 {
            self.death_time += 1.0;
        }
    }
    pub fn apply_living_health_metadata(&mut self, health: Option<f32>) {
        if self.uses_living_body_rotation() {
            if let Some(health) = health {
                self.living_health = health;
                self.dead = health <= 0.0;
            }
        }
    }
    pub fn shared_flag(&self, flag: u8) -> bool {
        (self.shared_flags & (1 << flag)) != 0
    }
    pub fn living_flag(&self, flag: u8) -> bool {
        (self.living_flags & flag) != 0
    }
    pub fn is_using_item(&self) -> bool {
        self.living_flag(1)
    }
    pub fn used_item_hand(&self) -> crate::render::entity::state::InteractionHand {
        if self.living_flag(2) {
            crate::render::entity::state::InteractionHand::OffHand
        } else {
            crate::render::entity::state::InteractionHand::MainHand
        }
    }
    pub fn current_used_item_stack(&self) -> &el_core::item::ItemStack {
        match self.used_item_hand() {
            crate::render::entity::state::InteractionHand::MainHand => &self.main_hand_item_stack,
            crate::render::entity::state::InteractionHand::OffHand => &self.off_hand_item_stack,
        }
    }
    pub fn used_item_stack(&self) -> &el_core::item::ItemStack {
        &self.use_item_snapshot
    }
    pub fn is_model_part_shown(&self, mask: u8) -> bool {
        (self.player_model_customisation & mask) == mask
    }
    pub fn refresh_use_item_from_living_flags(&mut self) {
        if self.is_using_item() && self.use_item_snapshot.is_empty() {
            let stack = self.current_used_item_stack().clone();
            if !stack.is_empty() {
                let duration = ItemUseAnimation::from_stack(&stack)
                    .map(|(_, duration)| duration)
                    .unwrap_or(0.0);
                self.use_item_snapshot = stack;
                self.use_item_snapshot_hand = self.used_item_hand();
                self.use_item_duration_ticks = duration;
                self.use_item_remaining_ticks = duration;
            }
        } else if !self.is_using_item() && !self.use_item_snapshot.is_empty() {
            self.clear_use_item_snapshot();
        }
    }
    pub fn render_ticks_using_item(&self, partial_tick: f32) -> f32 {
        if self.is_using_item() {
            self.use_item_duration_ticks - self.use_item_remaining_ticks + partial_tick
        } else {
            0.0
        }
    }
    pub fn clear_use_item_snapshot(&mut self) {
        self.use_item_snapshot = el_core::item::ItemStack::empty();
        self.use_item_duration_ticks = 0.0;
        self.use_item_remaining_ticks = 0.0;
    }
    pub fn tick_using_item(&mut self) {
        if !self.is_using_item() || self.use_item_snapshot.is_empty() {
            return;
        }
        let current = self.current_used_item_stack().clone();
        if !current.is_empty() && current.item.location == self.use_item_snapshot.item.location {
            self.use_item_snapshot = current;
            self.use_item_remaining_ticks -= 1.0;
        } else {
            self.clear_use_item_snapshot();
        }
    }
    pub fn walk_animation_position(&self, partial_tick: f32) -> f32 {
        let partial_tick = partial_tick.clamp(0.0, 1.0);
        (self.walk_animation_pos - self.walk_animation_speed * (1.0 - partial_tick))
            * self.walk_position_scale
    }
    pub fn walk_animation_speed(&self, partial_tick: f32) -> f32 {
        let partial_tick = partial_tick.clamp(0.0, 1.0);
        (self.prev_walk_animation_speed
            + (self.walk_animation_speed - self.prev_walk_animation_speed) * partial_tick)
            .min(1.0)
    }
    pub fn render_swim_amount(&self, partial_tick: f32) -> f32 {
        self.prev_swim_amount
            + (self.swim_amount - self.prev_swim_amount) * partial_tick.clamp(0.0, 1.0)
    }
    pub fn is_moving(&self) -> bool {
        self.velocity.length_squared() > 1.0E-7
    }
}
pub fn tick_remote_chicken_animation(entity: &mut RemoteEntityRenderState) {
    entity.prev_flap = entity.flap;
    entity.prev_flap_speed = entity.flap_speed;
    entity.flap_speed += if entity.on_ground { -0.3 } else { 1.2 };
    entity.flap_speed = entity.flap_speed.clamp(0.0, 1.0);
    if !entity.on_ground && entity.flapping < 1.0 {
        entity.flapping = 1.0;
    }
    entity.flapping *= 0.9;
    entity.flap += entity.flapping * 2.0;
}
pub fn tick_remote_parrot_animation(entity: &mut RemoteEntityRenderState, is_passenger: bool) {
    entity.prev_flap = entity.flap;
    entity.prev_flap_speed = entity.flap_speed;
    entity.flap_speed += if !entity.on_ground && !is_passenger {
        1.2
    } else {
        -0.3
    };
    entity.flap_speed = entity.flap_speed.clamp(0.0, 1.0);
    if !entity.on_ground && entity.flapping < 1.0 {
        entity.flapping = 1.0;
    }
    entity.flapping *= 0.9;
    entity.flap += entity.flapping * 2.0;
}
pub fn tick_remote_bat_animation(entity: &mut RemoteEntityRenderState) {
    if entity.is_bat_resting {
        entity.bat_fly_animation_state.stop();
        entity
            .bat_rest_animation_state
            .start_if_stopped(entity.age_in_ticks as f32);
    } else {
        entity.bat_rest_animation_state.stop();
        entity
            .bat_fly_animation_state
            .start_if_stopped(entity.age_in_ticks as f32);
    }
}
pub fn tick_remote_cube_mob_animation(entity: &mut RemoteEntityRenderState) {
    entity.prev_squish = entity.squish;
    entity.squish += (entity.target_squish - entity.squish) * 0.5;
    if entity.on_ground && !entity.was_on_ground {
        entity.target_squish = -0.5;
    } else if !entity.on_ground && entity.was_on_ground {
        entity.target_squish = 1.0;
    }
    entity.was_on_ground = entity.on_ground;
    entity.target_squish *= if entity.kind == "magma_cube" {
        0.9
    } else {
        0.6
    };
}
pub fn tick_remote_squid_animation(entity: &mut RemoteEntityRenderState, is_in_water: bool) {
    entity.old_tentacle_movement = entity.tentacle_movement;
    entity.old_tentacle_angle = entity.tentacle_angle;
    entity.tentacle_movement += entity.tentacle_speed;
    if entity.tentacle_movement > std::f32::consts::TAU {
        entity.tentacle_movement -= std::f32::consts::TAU;
    }
    if is_in_water {
        if entity.tentacle_movement < std::f32::consts::PI {
            let tentacle_scale = entity.tentacle_movement / std::f32::consts::PI;
            entity.tentacle_angle = (tentacle_scale * tentacle_scale * std::f32::consts::PI).sin()
                * std::f32::consts::PI
                * 0.25;
        } else {
            entity.tentacle_angle = 0.0;
        }
    } else {
        entity.tentacle_angle = entity.tentacle_movement.sin().abs() * std::f32::consts::PI * 0.25;
    }
}
pub fn tick_remote_guardian_animation(entity: &mut RemoteEntityRenderState, is_in_water: bool) {
    entity.old_tail_animation = entity.tail_animation;
    if !is_in_water {
        entity.tail_animation_speed = 2.0;
    } else if entity.is_moving() {
        if entity.tail_animation_speed < 0.5 {
            entity.tail_animation_speed = 4.0;
        } else {
            entity.tail_animation_speed += (0.5 - entity.tail_animation_speed) * 0.1;
        }
    } else {
        entity.tail_animation_speed += (0.125 - entity.tail_animation_speed) * 0.2;
    }
    entity.tail_animation += entity.tail_animation_speed;
    entity.old_spikes_animation = entity.spikes_animation;
    if !is_in_water {
        entity.spikes_animation = ((entity.id as u32 as f64 * 0.618_034) % 1.0) as f32;
    } else if entity.is_moving() {
        entity.spikes_animation += (0.0 - entity.spikes_animation) * 0.25;
    } else {
        entity.spikes_animation += (1.0 - entity.spikes_animation) * 0.06;
    }
}
pub fn tick_remote_camel_animation(entity: &mut RemoteEntityRenderState, game_time: i64) {
    if entity.camel_dash_cooldown > 0 {
        entity.camel_dash_cooldown -= 1;
    }
    if entity.camel_idle_animation_timeout <= 0 {
        use el_math::random::RandomSource;
        let mut rng =
            el_math::random::LegacyRandomSource::new((entity.id as i64) ^ (game_time << 16));
        entity.camel_idle_animation_timeout = rng.next_int_bound(40) + 80;
        entity
            .camel_idle_animation_state
            .start(entity.age_in_ticks as f32);
    } else {
        entity.camel_idle_animation_timeout -= 1;
    }
    let camel_sitting = entity.camel_last_pose_change_tick < 0;
    let pose_time = game_time - entity.camel_last_pose_change_tick.abs();
    let camel_visually_sitting = (pose_time < 0) != camel_sitting;
    if camel_visually_sitting {
        entity.camel_sit_up_animation_state.stop();
        entity.camel_dash_animation_state.stop();
        if camel_sitting && pose_time < 40 && pose_time >= 0 {
            entity
                .camel_sit_animation_state
                .start_if_stopped(entity.age_in_ticks as f32);
            entity.camel_sit_pose_animation_state.stop();
        } else {
            entity.camel_sit_animation_state.stop();
            entity
                .camel_sit_pose_animation_state
                .start_if_stopped(entity.age_in_ticks as f32);
        }
    } else {
        entity.camel_sit_animation_state.stop();
        entity.camel_sit_pose_animation_state.stop();
        entity
            .camel_dash_animation_state
            .animate_when(entity.camel_dashing, entity.age_in_ticks as f32);
        let in_pose_transition = pose_time < if camel_sitting { 40 } else { 52 };
        entity.camel_sit_up_animation_state.animate_when(
            in_pose_transition && pose_time >= 0,
            entity.age_in_ticks as f32,
        );
    }
}
pub fn tick_remote_creaking_animation(entity: &mut RemoteEntityRenderState) {
    if entity.creaking_invulnerability_remaining_ticks > 0 {
        entity.creaking_invulnerability_remaining_ticks -= 1;
    }
    if entity.creaking_attack_remaining_ticks > 0 {
        entity.creaking_attack_remaining_ticks -= 1;
    }
    let age = entity.age_in_ticks as f32;
    entity
        .creaking_attack_animation_state
        .animate_when(entity.creaking_attack_remaining_ticks > 0, age);
    entity
        .creaking_invulnerability_animation_state
        .animate_when(entity.creaking_invulnerability_remaining_ticks > 0, age);
    entity
        .creaking_death_animation_state
        .animate_when(entity.creaking_is_tearing_down, age);
}
