use super::item_physics::{tick_remote_item_entity_like_java, tick_remote_tnt_motion_like_java};
use super::state::*;
use super::types::*;
use crate::render::entity::state::HumanoidArm;
use crate::world::client_level::ClientLevel;
use el_core::item::ItemStack;
use el_core::registry::BuiltInRegistries;
use el_math::block_pos::BlockPos;
use el_protocol_backend::{BackendEntityPose, BackendNetworkEvent};
use glam::{DVec3, Vec3};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
fn item_frame_pose_from_anchor_like_java(
    position: DVec3,
    direction_data: i32,
) -> (DVec3, f32, f32) {
    let direction = (direction_data % 6).unsigned_abs();
    let (step, yaw, pitch) = match direction {
        0 => (-DVec3::Y, 0.0, 90.0),
        1 => (DVec3::Y, 0.0, -90.0),
        2 => (-DVec3::Z, 180.0, 0.0),
        3 => (DVec3::Z, 0.0, 0.0),
        4 => (-DVec3::X, 90.0, 0.0),
        _ => (DVec3::X, 270.0, 0.0),
    };
    (
        position.floor() + DVec3::splat(0.5) - step * 0.46875,
        yaw,
        pitch,
    )
}
pub fn apply_respawn_level_transition<T>(
    current_dimension: Option<&str>,
    incoming_dimension: &str,
    state: &mut T,
    replace: impl FnOnce(&mut T),
) -> bool {
    let changed = current_dimension != Some(incoming_dimension);
    if changed {
        replace(state);
    }
    changed
}
#[derive(Debug, Clone)]
pub struct RemoteEntitySpawnParams {
    pub id: i32,
    pub uuid: Option<uuid::Uuid>,
    pub profile_name: Option<String>,
    pub skin_texture_url: Option<String>,
    pub skin_model: Option<String>,
    pub kind: String,
    pub position: DVec3,
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub head_yaw_degrees: f32,
    pub velocity: DVec3,
    pub spawn_data: i32,
    pub falling_block_state_id: Option<usize>,
}
#[derive(Debug, Clone)]
pub struct RemoteEntityFrameSnapshot {
    pub id: i32,
    pub kind: String,
    pub position_d: DVec3,
    pub render_position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub body_yaw: f32,
    pub head_yaw: f32,
    pub pose: BackendEntityPose,
    pub is_body_visible: bool,
    pub is_invisible: bool,
    pub has_independent_layers: bool,
    pub health: f32,
    pub is_dead: bool,
    pub death_time: f32,
    pub hurt_time: f32,
    pub walk_anim_pos: f32,
    pub walk_anim_speed: f32,
    pub main_hand_item: ItemStack,
    pub off_hand_item: ItemStack,
    pub head_item: ItemStack,
    pub chest_item: ItemStack,
    pub legs_item: ItemStack,
    pub feet_item: ItemStack,
    pub body_item: ItemStack,
    pub saddle_item: ItemStack,
}
pub struct RemoteEntityEventContext<'a> {
    pub level: Option<&'a ClientLevel>,
    pub local_player_entity_id: Option<i32>,
    pub local_player_uuid: Option<uuid::Uuid>,
    pub remote_player_skins: Option<&'a HashMap<uuid::Uuid, RemotePlayerSkinInfo>>,
    pub remote_player_uuid_to_name: Option<&'a HashMap<uuid::Uuid, String>>,
    pub remote_vehicle_entity_id: Option<i32>,
    pub removed_player_vehicle_id: Option<i32>,
    pub local_player_pos_rot: Option<RemotePositionMoveRotation>,
    pub local_player_old_pos_rot: Option<RemotePositionMoveRotation>,
    pub current_dimension: Option<&'a str>,
    pub particle_seed: Option<&'a mut dyn FnMut() -> i64>,
    pub boat_input_latch: Option<RemoteBoatInputLatch>,
}
impl<'a> RemoteEntityEventContext<'a> {
    fn next_particle_seed(&mut self) -> i64 {
        self.particle_seed.as_mut().map_or_else(
            crate::render::particle::java_unique_particle_seed,
            |source| source(),
        )
    }
    pub fn empty() -> Self {
        Self {
            level: None,
            local_player_entity_id: None,
            local_player_uuid: None,
            remote_player_skins: None,
            remote_player_uuid_to_name: None,
            remote_vehicle_entity_id: None,
            removed_player_vehicle_id: None,
            local_player_pos_rot: None,
            local_player_old_pos_rot: None,
            current_dimension: None,
            particle_seed: None,
            boat_input_latch: None,
        }
    }
}
#[derive(Debug, Clone, Default)]
pub struct RemoteEntityEventOutcome {
    pub handled: bool,
    pub is_local_player: bool,
    pub spawned_entity_id: Option<i32>,
    pub spawned_entity_kind: Option<String>,
    pub spawned_entity_uuid: Option<uuid::Uuid>,
    pub local_health: Option<f32>,
    pub local_absorption: Option<f32>,
    pub local_ticks_frozen: Option<i32>,
    pub local_sleeping_pos: Option<Option<el_math::block_pos::BlockPos>>,
    pub local_shared_flags: Option<u8>,
    pub local_living_flags: Option<u8>,
    pub local_pose: Option<crate::gameplay::player::local_player::PlayerPose>,
    pub local_velocity: Option<glam::DVec3>,
    pub attribute_updates: Vec<(
        String,
        f64,
        Vec<crate::gameplay::attributes::AttributeModifier>,
    )>,
    pub set_camera_id: Option<i32>,
    pub hurt_entity_id: Option<i32>,
    pub hurt_dir: Option<f32>,
    pub local_damage_sound: Option<(&'static str, glam::Vec3)>,
    pub passenger_graph_result: Option<RemoteSetPassengersGraphResult>,
    pub new_local_vehicle_id: Option<Option<i32>>,
    pub align_local_boat_yaw: Option<f32>,
    pub reset_boat_input_latch: bool,
    pub init_boat_input_latch: Option<i32>,
    pub vehicle_move_echo: Option<(glam::DVec3, f32, f32, bool)>,
    pub should_position_local_rider_vehicle_id: Option<i32>,
    pub local_player_teleport_direct:
        Option<(RemotePositionMoveRotation, RemotePositionMoveRotation)>,
    pub local_player_teleport_echo: Option<glam::DVec3>,
    pub removed_ids: Vec<i32>,
    pub removed_player_vehicle_id: Option<i32>,
    pub local_player_unseated: bool,
    pub dimension_replaced_level: bool,
    pub swing_hand: Option<crate::render::model::player::player_model::HandSide>,
    pub particles: Vec<(el_protocol_backend::BackendParticle, glam::Vec3, glam::Vec3)>,
    pub tracking_emitters: Vec<(i32, el_protocol_backend::BackendParticle, usize)>,
    pub death_poof: Option<(glam::Vec3, f32, f32)>,
    pub sounds: Vec<(
        &'static str,
        crate::client::sound_engine::SoundSource,
        glam::Vec3,
        f32,
        f32,
    )>,
    pub local_kinetic_hit: bool,
    pub player_death_sound: Option<glam::Vec3>,
    pub eat_complete_event_9: bool,
    pub totem_used: bool,
}
pub fn remote_backend_pose_to_player_pose(
    pose: el_protocol_backend::BackendEntityPose,
) -> crate::gameplay::player::local_player::PlayerPose {
    match pose {
        el_protocol_backend::BackendEntityPose::Standing
        | el_protocol_backend::BackendEntityPose::LongJumping
        | el_protocol_backend::BackendEntityPose::Croaking
        | el_protocol_backend::BackendEntityPose::UsingTongue
        | el_protocol_backend::BackendEntityPose::Sitting
        | el_protocol_backend::BackendEntityPose::Roaring
        | el_protocol_backend::BackendEntityPose::Sniffing
        | el_protocol_backend::BackendEntityPose::Emerging
        | el_protocol_backend::BackendEntityPose::Digging
        | el_protocol_backend::BackendEntityPose::Sliding
        | el_protocol_backend::BackendEntityPose::Shooting
        | el_protocol_backend::BackendEntityPose::Inhaling => {
            crate::gameplay::player::local_player::PlayerPose::Standing
        }
        el_protocol_backend::BackendEntityPose::FallFlying => {
            crate::gameplay::player::local_player::PlayerPose::FallFlying
        }
        el_protocol_backend::BackendEntityPose::Sleeping => {
            crate::gameplay::player::local_player::PlayerPose::Sleeping
        }
        el_protocol_backend::BackendEntityPose::Swimming => {
            crate::gameplay::player::local_player::PlayerPose::Swimming
        }
        el_protocol_backend::BackendEntityPose::SpinAttack => {
            crate::gameplay::player::local_player::PlayerPose::SpinAttack
        }
        el_protocol_backend::BackendEntityPose::Crouching => {
            crate::gameplay::player::local_player::PlayerPose::Crouching
        }
        el_protocol_backend::BackendEntityPose::Dying => {
            crate::gameplay::player::local_player::PlayerPose::Dying
        }
    }
}
#[derive(Debug, Clone, Default)]
pub struct RemoteEntities {
    pub entities: HashMap<i32, RemoteEntityRenderState>,
}
impl Deref for RemoteEntities {
    type Target = HashMap<i32, RemoteEntityRenderState>;
    fn deref(&self) -> &Self::Target {
        &self.entities
    }
}
impl DerefMut for RemoteEntities {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entities
    }
}
impl<'a> IntoIterator for &'a RemoteEntities {
    type Item = (&'a i32, &'a RemoteEntityRenderState);
    type IntoIter = std::collections::hash_map::Iter<'a, i32, RemoteEntityRenderState>;
    fn into_iter(self) -> Self::IntoIter {
        self.entities.iter()
    }
}
impl<'a> IntoIterator for &'a mut RemoteEntities {
    type Item = (&'a i32, &'a mut RemoteEntityRenderState);
    type IntoIter = std::collections::hash_map::IterMut<'a, i32, RemoteEntityRenderState>;
    fn into_iter(self) -> Self::IntoIter {
        self.entities.iter_mut()
    }
}
impl IntoIterator for RemoteEntities {
    type Item = (i32, RemoteEntityRenderState);
    type IntoIter = std::collections::hash_map::IntoIter<i32, RemoteEntityRenderState>;
    fn into_iter(self) -> Self::IntoIter {
        self.entities.into_iter()
    }
}
impl From<HashMap<i32, RemoteEntityRenderState>> for RemoteEntities {
    fn from(entities: HashMap<i32, RemoteEntityRenderState>) -> Self {
        Self { entities }
    }
}
fn detach_removed_entity_from_passenger_graph(
    entities: &mut HashMap<i32, RemoteEntityRenderState>,
    removed_id: i32,
) {
    for entity in entities.values_mut() {
        entity
            .passengers
            .retain(|passenger_id| *passenger_id != removed_id);
        if entity.leash_holder_id == removed_id {
            entity.leash_holder_id = 0;
        }
    }
}
impl RemoteEntities {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }
    pub fn spawn(&mut self, params: RemoteEntitySpawnParams) -> &mut RemoteEntityRenderState {
        let frame_pose = matches!(
            remote_entity_type_key(&params.kind),
            "item_frame" | "glow_item_frame"
        )
        .then(|| item_frame_pose_from_anchor_like_java(params.position, params.spawn_data));
        let (position, yaw_degrees, pitch_degrees) =
            frame_pose.unwrap_or((params.position, params.yaw_degrees, params.pitch_degrees));
        let mut render_state = RemoteEntityRenderState::new_with_precise_position(
            params.id,
            params.uuid,
            params.kind.clone(),
            position,
            yaw_degrees.to_radians(),
            pitch_degrees.to_radians(),
            params.head_yaw_degrees.to_radians(),
        );
        render_state.spawn_data = params.spawn_data;
        if remote_entity_type_key(&render_state.kind) == "falling_block" {
            render_state.falling_block_state =
                falling_block_state_from_backend_like_java(params.falling_block_state_id);
            render_state.falling_block_state_is_canonical = params.falling_block_state_id.is_some();
            render_state.falling_block_start_pos =
                falling_block_start_pos_from_add_entity_like_java(
                    params.position.x,
                    params.position.y,
                    params.position.z,
                );
        }
        if is_remote_player_entity(&render_state.kind) {
            render_state.profile_name = params.profile_name;
            render_state.skin_texture_url = params.skin_texture_url;
            render_state.skin_model = params.skin_model;
        }
        render_state.set_rotation_degrees_like_java(yaw_degrees, pitch_degrees);
        render_state.copy_current_rotation_to_old_like_java();
        render_state.set_interpolation_target_rotation_degrees(
            render_state.y_rot_degrees(),
            render_state.x_rot_degrees(),
        );
        render_state.network_position_codec.set_base(position);
        render_state.set_packet_velocity(params.velocity);
        if is_remote_abstract_boat(&render_state.kind) {
            render_state.set_boat_physics_state(params.position, params.velocity);
            render_state.boat_old_physics_position = Some(params.position);
        }
        self.entities.insert(params.id, render_state);
        self.entities.get_mut(&params.id).unwrap()
    }
    pub fn clear(&mut self) {
        self.entities.clear();
    }
    pub fn handle_respawn(
        &mut self,
        current_dimension: Option<&str>,
        incoming_dimension: &str,
    ) -> bool {
        apply_respawn_level_transition(current_dimension, incoming_dimension, self, Self::clear)
    }
    pub fn apply_network_event(
        &mut self,
        event: &BackendNetworkEvent,
        context: &mut RemoteEntityEventContext,
    ) -> RemoteEntityEventOutcome {
        let mut outcome = RemoteEntityEventOutcome::default();
        match event {
            BackendNetworkEvent::EntityAdded { entity } => {
                outcome.handled = true;
                if Some(entity.id) == context.local_player_entity_id
                    || Some(entity.uuid) == context.local_player_uuid
                {
                    outcome.is_local_player = true;
                    return outcome;
                }
                if Some(entity.id) == context.removed_player_vehicle_id {
                    context.removed_player_vehicle_id = None;
                    outcome.removed_player_vehicle_id = None;
                }
                let skin_info = context
                    .remote_player_skins
                    .and_then(|m| m.get(&entity.uuid));
                let (skin_texture_url, skin_model) = if let Some(skin) = skin_info {
                    (skin.texture_url.clone(), skin.model.clone())
                } else {
                    (None, None)
                };
                let profile_name = if is_remote_player_entity(&entity.kind) {
                    context
                        .remote_player_uuid_to_name
                        .and_then(|m| m.get(&entity.uuid).cloned())
                } else {
                    None
                };
                let previous_entity = self.entities.get(&entity.id).cloned();
                let render_state = self.spawn(RemoteEntitySpawnParams {
                    id: entity.id,
                    uuid: Some(entity.uuid),
                    profile_name,
                    skin_texture_url,
                    skin_model,
                    kind: entity.kind.clone(),
                    position: glam::DVec3::new(entity.x, entity.y, entity.z),
                    yaw_degrees: entity.yaw_degrees,
                    pitch_degrees: entity.pitch_degrees,
                    head_yaw_degrees: entity.head_yaw_degrees,
                    velocity: glam::DVec3::new(
                        entity.velocity_x,
                        entity.velocity_y,
                        entity.velocity_z,
                    ),
                    spawn_data: entity.spawn_data,
                    falling_block_state_id: entity.falling_block_state_id,
                });
                if is_remote_player_entity(&render_state.kind)
                    || previous_entity
                        .as_ref()
                        .is_some_and(|previous| is_remote_player_entity(&previous.kind))
                {
                    el_protocol_backend::diagnostics::write_line(&format!(
                        "[RemotePlayerDebug] phase=add id={} kind={} pos=[{:.3},{:.3},{:.3}] flags={:#04x} replaced={} previous_kind={} previous_flags={:#04x}",
                        entity.id,
                        render_state.kind,
                        render_state.position_d.x,
                        render_state.position_d.y,
                        render_state.position_d.z,
                        render_state.shared_flags,
                        previous_entity.is_some(),
                        previous_entity
                            .as_ref()
                            .map_or("none", |previous| previous.kind.as_str()),
                        previous_entity
                            .as_ref()
                            .map_or(0, |previous| previous.shared_flags),
                    ));
                }
                outcome.spawned_entity_id = Some(entity.id);
                outcome.spawned_entity_kind = Some(entity.kind.clone());
                outcome.spawned_entity_uuid = Some(entity.uuid);
            }
            BackendNetworkEvent::EntityMetadata {
                id,
                no_gravity,
                item_stack,
                item_frame_rotation,
                item_frame_direction,
                shared_flags,
                living_flags,
                living_health,
                custom_name,
                custom_name_visible,
                pose,
                sleeping_pos,
                player_main_arm_right,
                player_model_customisation,
                player_absorption,
                ticks_frozen,
                interaction_width,
                interaction_height,
                interaction_response,
                mob_size,
                primed_tnt_fuse,
                primed_tnt_block_state_id,
                end_crystal_beam_target,
                end_crystal_show_bottom,
                abstract_arrow_flags,
                abstract_arrow_pierce_level,
                abstract_arrow_in_ground,
                arrow_effect_color,
                experience_orb_value,
                armor_stand_client_flags,
                armor_stand_head_pose,
                armor_stand_body_pose,
                armor_stand_left_arm_pose,
                armor_stand_right_arm_pose,
                armor_stand_left_leg_pose,
                armor_stand_right_leg_pose,
                is_baby,
                is_aggressive,
                mob_flags,
                camel_dashing,
                camel_last_pose_change_tick,
                mob_boolean_metadata,
                piglin_is_baby,
                piglin_is_charging_crossbow,
                piglin_is_dancing,
                tamable_flags,
                creaking_can_move,
                creaking_is_active,
                creaking_is_tearing_down,
                display_interpolation_start_delta_ticks,
                display_interpolation_duration,
                display_teleport_duration,
                display_translation,
                display_scale,
                display_left_rotation,
                display_right_rotation,
                display_billboard,
                display_brightness_override,
                display_view_range,
                display_shadow_radius,
                display_shadow_strength,
                display_width,
                display_height,
                display_glow_color_override,
                text_display_text,
                text_display_line_width,
                text_display_background,
                text_display_text_opacity,
                text_display_flags,
                ..
            } => {
                outcome.handled = true;
                if Some(*id) == context.local_player_entity_id {
                    outcome.is_local_player = true;
                    outcome.local_health = *living_health;
                    outcome.local_absorption = *player_absorption;
                    outcome.local_ticks_frozen = *ticks_frozen;
                    if let Some(sleeping) = sleeping_pos.as_ref() {
                        outcome.local_sleeping_pos = Some(match sleeping {
                            el_protocol_backend::BackendSleepingPosUpdate::Clear => None,
                            el_protocol_backend::BackendSleepingPosUpdate::Set(pos) => {
                                Some(BlockPos::new(pos.x, pos.y, pos.z))
                            }
                        });
                    }
                    outcome.local_shared_flags = *shared_flags;
                    outcome.local_living_flags = *living_flags;
                    outcome.local_pose = pose.map(remote_backend_pose_to_player_pose);
                }
                let item_stack = item_stack
                    .clone()
                    .map(crate::network::backend_mappers::map_backend_item_stack);
                if let Some(entity) = self.entities.get_mut(id) {
                    if let Some(value) = no_gravity {
                        entity.no_gravity = *value;
                    }
                    let player_metadata_before = is_remote_player_entity(&entity.kind).then_some((
                        entity.shared_flags,
                        entity.pose,
                        entity.living_health.to_bits(),
                    ));
                    apply_remote_item_stack_delta_like_java(entity, item_stack);
                    if matches!(
                        remote_entity_type_key(&entity.kind),
                        "item_frame" | "glow_item_frame"
                    ) && let Some(rotation) = item_frame_rotation
                    {
                        entity.item_frame_rotation = *rotation;
                    }
                    if matches!(
                        remote_entity_type_key(&entity.kind),
                        "item_frame" | "glow_item_frame"
                    ) && let Some(direction) =
                        item_frame_direction.filter(|direction| *direction < 6)
                    {
                        entity.spawn_data = i32::from(direction);
                        let (position, yaw, pitch) = item_frame_pose_from_anchor_like_java(
                            entity.position_d,
                            entity.spawn_data,
                        );
                        entity.set_render_position_d(position);
                        entity.set_render_target_position_d(position);
                        entity.network_position_codec.set_base(position);
                        entity.set_rotation_degrees_like_java(yaw, pitch);
                        entity.copy_current_rotation_to_old_like_java();
                        entity.set_interpolation_target_rotation_degrees(yaw, pitch);
                    }
                    apply_remote_custom_name_metadata(
                        entity,
                        custom_name.clone(),
                        *custom_name_visible,
                    );
                    if let Some(mob_size) = mob_size {
                        entity.mob_size = *mob_size;
                    }
                    if entity.uses_living_body_rotation() {
                        entity.apply_living_health_metadata(*living_health);
                        if let Some(pose) = pose {
                            entity.pose = *pose;
                        }
                        if let Some(sleeping_pos) = sleeping_pos {
                            entity.sleeping_pos = match sleeping_pos {
                                el_protocol_backend::BackendSleepingPosUpdate::Clear => None,
                                el_protocol_backend::BackendSleepingPosUpdate::Set(pos) => {
                                    Some(BlockPos::new(pos.x, pos.y, pos.z))
                                }
                            };
                        }
                    }
                    if let Some(shared_flags) = shared_flags {
                        entity.shared_flags = *shared_flags;
                    }
                    if let Some((old_flags, old_pose, old_health)) = player_metadata_before {
                        if (old_flags, old_pose, old_health)
                            != (
                                entity.shared_flags,
                                entity.pose,
                                entity.living_health.to_bits(),
                            )
                        {
                            el_protocol_backend::diagnostics::write_line(&format!(
                                "[RemotePlayerDebug] phase=metadata route=network id={} pos=[{:.3},{:.3},{:.3}] flags={:#04x}->{:#04x} pose={:?}->{:?} health={:.3}->{:.3} dead={}",
                                id,
                                entity.position_d.x,
                                entity.position_d.y,
                                entity.position_d.z,
                                old_flags,
                                entity.shared_flags,
                                old_pose,
                                entity.pose,
                                f32::from_bits(old_health),
                                entity.living_health,
                                entity.dead,
                            ));
                        }
                    }
                    if remote_entity_type_key(&entity.kind) == "interaction" {
                        if let Some(width) = interaction_width {
                            entity.interaction_width = *width;
                        }
                        if let Some(height) = interaction_height {
                            entity.interaction_height = *height;
                        }
                        if let Some(response) = interaction_response {
                            entity.interaction_response = *response;
                        }
                    }
                    if let Some(living_flags) = living_flags {
                        if entity.uses_living_body_rotation() {
                            entity.living_flags = *living_flags;
                            entity.refresh_use_item_from_living_flags();
                        }
                    }
                    if let Some(player_main_arm_right) = player_main_arm_right {
                        entity.main_arm = if *player_main_arm_right {
                            HumanoidArm::Right
                        } else {
                            HumanoidArm::Left
                        };
                    }
                    if let Some(player_model_customisation) = player_model_customisation {
                        entity.player_model_customisation = *player_model_customisation;
                        if remote_entity_type_key(&entity.kind) == "bat" {
                            entity.is_bat_resting = player_model_customisation & 0x01 != 0;
                        }
                    }
                    let primed_tnt_block_state = primed_tnt_block_state_id.and_then(|bs_id| {
                        BuiltInRegistries::block_states()
                            .get_state_by_id(bs_id)
                            .cloned()
                    });
                    apply_remote_special_entity_metadata(
                        entity,
                        *primed_tnt_fuse,
                        *experience_orb_value,
                        primed_tnt_block_state,
                        end_crystal_beam_target.clone(),
                        *end_crystal_show_bottom,
                        *abstract_arrow_flags,
                        *abstract_arrow_pierce_level,
                        *abstract_arrow_in_ground,
                        *arrow_effect_color,
                    );
                    apply_remote_armor_stand_metadata(
                        entity,
                        *armor_stand_client_flags,
                        armor_stand_head_pose.clone(),
                        armor_stand_body_pose.clone(),
                        armor_stand_left_arm_pose.clone(),
                        armor_stand_right_arm_pose.clone(),
                        armor_stand_left_leg_pose.clone(),
                        armor_stand_right_leg_pose.clone(),
                    );
                    apply_remote_mob_boolean_metadata(entity, mob_boolean_metadata);
                    apply_remote_mob_flags(entity, *mob_flags);
                    apply_remote_mob_animation_metadata(
                        entity,
                        *is_baby,
                        *is_aggressive,
                        *camel_dashing,
                        *camel_last_pose_change_tick,
                        *piglin_is_baby,
                        *piglin_is_charging_crossbow,
                        *piglin_is_dancing,
                        *tamable_flags,
                        *creaking_can_move,
                        *creaking_is_active,
                        *creaking_is_tearing_down,
                    );
                    apply_remote_display_metadata(
                        entity,
                        *display_interpolation_start_delta_ticks,
                        *display_interpolation_duration,
                        *display_teleport_duration,
                        *display_translation,
                        *display_scale,
                        *display_left_rotation,
                        *display_right_rotation,
                        *display_billboard,
                        *display_brightness_override,
                        *display_view_range,
                        *display_shadow_radius,
                        *display_shadow_strength,
                        *display_width,
                        *display_height,
                        *display_glow_color_override,
                        text_display_text.clone(),
                        *text_display_line_width,
                        *text_display_background,
                        *text_display_text_opacity,
                        *text_display_flags,
                    );
                }
            }
            BackendNetworkEvent::EntityVelocity { id, x, y, z } => {
                outcome.handled = true;
                let velocity = glam::DVec3::new(*x, *y, *z);
                el_protocol_backend::diagnostics::write_line(&format!(
                    "[CombatTrace] kb_inbound id={id} is_local={} vel=[{x:.6},{y:.6},{z:.6}]",
                    Some(*id) == context.local_player_entity_id,
                ));
                if Some(*id) == context.local_player_entity_id {
                    outcome.is_local_player = true;
                    outcome.local_velocity = Some(velocity);
                } else if let Some(entity) = self.entities.get_mut(id) {
                    entity.lerp_packet_velocity(velocity);
                }
            }
            BackendNetworkEvent::UpdateAttributes {
                entity_id,
                attributes,
            } => {
                outcome.handled = true;
                for snap in attributes {
                    let modifiers: Vec<crate::gameplay::attributes::AttributeModifier> = snap
                        .modifiers
                        .iter()
                        .filter_map(|(id, amount, op)| {
                            crate::gameplay::attributes::AttributeModifierOperation::from_id(*op)
                                .map(|operation| crate::gameplay::attributes::AttributeModifier {
                                    id: id.clone(),
                                    amount: *amount,
                                    operation,
                                })
                        })
                        .collect();
                    if Some(*entity_id) == context.local_player_entity_id {
                        outcome.is_local_player = true;
                        outcome.attribute_updates.push((
                            snap.attribute_id.clone(),
                            snap.base,
                            modifiers,
                        ));
                    } else if let Some(entity) = self.entities.get_mut(entity_id) {
                        let _ = entity.apply_remote_attribute_snapshot(
                            &snap.attribute_id,
                            snap.base,
                            modifiers,
                        );
                    }
                }
            }
            BackendNetworkEvent::SetCamera { camera_id } => {
                outcome.handled = true;
                let known = Some(*camera_id) == context.local_player_entity_id
                    || self.entities.contains_key(camera_id);
                if known {
                    outcome.set_camera_id = Some(*camera_id);
                }
            }
            BackendNetworkEvent::HurtAnimation { entity_id, yaw } => {
                outcome.handled = true;
                if Some(*entity_id) == context.local_player_entity_id {
                    outcome.is_local_player = true;
                    outcome.hurt_entity_id = Some(*entity_id);
                    outcome.hurt_dir = Some(*yaw);
                } else if let Some(entity) = self.entities.get_mut(entity_id) {
                    entity.hurt_time = 10.0;
                    entity.hurt_duration = 10.0;
                    if remote_entity_type_key(&entity.kind) == "player" {
                        entity.hurt_dir = *yaw;
                    }
                }
            }
            BackendNetworkEvent::DamageEvent {
                entity_id,
                source_type_id,
                ..
            } => {
                outcome.handled = true;
                if Some(*entity_id) == context.local_player_entity_id {
                    outcome.is_local_player = true;
                    outcome.hurt_entity_id = Some(*entity_id);
                    outcome.local_damage_sound = Some((
                        player_hurt_sound_path_by_damage_type_id_like_java(*source_type_id),
                        glam::Vec3::ZERO,
                    ));
                } else if let Some(entity) = self.entities.get_mut(entity_id) {
                    entity.hurt_time = 10.0;
                    entity.hurt_duration = 10.0;
                    entity.invulnerable_time = 20.0;
                    entity.walk_animation_speed = 1.5;
                }
            }
            BackendNetworkEvent::TakeItem { item_id, .. } => {
                outcome.handled = true;
                self.entities.remove(item_id);
                detach_removed_entity_from_passenger_graph(&mut self.entities, *item_id);
                outcome.removed_ids.push(*item_id);
            }
            BackendNetworkEvent::SetPassengers {
                vehicle_id,
                passenger_ids,
            } => {
                outcome.handled = true;
                if !self.entities.contains_key(vehicle_id) {
                    return outcome;
                }
                let local_id = context.local_player_entity_id;
                let previous_vehicle_id = context.remote_vehicle_entity_id;
                let mut local_vehicle = previous_vehicle_id;
                let graph_result = rebuild_remote_passenger_graph_like_java(
                    *vehicle_id,
                    passenger_ids,
                    local_id,
                    &mut local_vehicle,
                    &mut self.entities,
                );
                context.remote_vehicle_entity_id = local_vehicle;
                outcome.passenger_graph_result = Some(graph_result);
                outcome.new_local_vehicle_id = Some(local_vehicle);
                if graph_result.local_started_riding {
                    context.removed_player_vehicle_id = None;
                    outcome.removed_player_vehicle_id = None;
                    if !graph_result.was_local_indirect_passenger {
                        if let Some(vehicle) = self.entities.get(vehicle_id) {
                            if is_remote_abstract_boat(&vehicle.kind) {
                                outcome.align_local_boat_yaw = Some(vehicle.y_rot_degrees());
                            }
                        }
                    }
                    let local_controls_boat = self
                        .entities
                        .get(vehicle_id)
                        .is_some_and(|v| remote_abstract_boat_is_locally_controlled(v, local_id));
                    let latch_is_for_vehicle = context
                        .boat_input_latch
                        .is_some_and(|latch| latch.vehicle_id == *vehicle_id);
                    if previous_vehicle_id != Some(*vehicle_id) || !latch_is_for_vehicle {
                        if local_controls_boat {
                            outcome.init_boat_input_latch = Some(*vehicle_id);
                        } else {
                            outcome.reset_boat_input_latch = true;
                        }
                    } else if !local_controls_boat {
                        outcome.reset_boat_input_latch = true;
                    }
                } else if previous_vehicle_id == Some(*vehicle_id) {
                    outcome.reset_boat_input_latch = true;
                }
            }
            BackendNetworkEvent::MoveVehicle {
                x,
                y,
                z,
                yaw_degrees,
                pitch_degrees,
            } => {
                outcome.handled = true;
                if let Some(direct_vehicle_id) = context.remote_vehicle_entity_id {
                    let root_vehicle_id = remote_root_vehicle_id(direct_vehicle_id, &self.entities);
                    let root_is_boat = self
                        .entities
                        .get(&root_vehicle_id)
                        .is_some_and(|vehicle| is_remote_abstract_boat(&vehicle.kind));
                    if root_is_boat {
                        let locally_authoritative =
                            self.entities.get(&root_vehicle_id).is_some_and(|vehicle| {
                                remote_abstract_boat_is_locally_controlled(
                                    vehicle,
                                    context.local_player_entity_id,
                                )
                            });
                        if locally_authoritative {
                            let target = glam::DVec3::new(*x, *y, *z);
                            if let Some(vehicle) = self.entities.get_mut(&root_vehicle_id) {
                                let current_target = if vehicle.interpolation_steps > 0 {
                                    vehicle.boat_interpolation_target_position.unwrap_or_else(
                                        || vehicle.render_target_position_d_from_authority(),
                                    )
                                } else {
                                    vehicle.boat_physics_position()
                                };
                                if target.distance(current_target) > 1.0e-5 {
                                    vehicle.cancel_interpolation_like_java();
                                    vehicle.abs_snap_to_d_degrees(
                                        target,
                                        *yaw_degrees,
                                        *pitch_degrees,
                                    );
                                    vehicle.set_boat_physics_position(target);
                                    vehicle.boat_old_physics_position = Some(target);
                                }
                                outcome.vehicle_move_echo = Some((
                                    vehicle.boat_physics_position(),
                                    vehicle.y_rot_degrees(),
                                    vehicle.x_rot_degrees(),
                                    vehicle.on_ground,
                                ));
                            }
                        }
                    }
                }
            }
            BackendNetworkEvent::MoveMinecartAlongTrack { entity_id, steps } => {
                outcome.handled = true;
                if let Some(entity) = self.entities.get_mut(entity_id) {
                    let rust_steps = steps
                        .iter()
                        .map(|step| RustMinecartStep {
                            position: glam::Vec3::new(step.x as f32, step.y as f32, step.z as f32),
                            movement: glam::Vec3::new(
                                step.dx as f32,
                                step.dy as f32,
                                step.dz as f32,
                            ),
                            yaw_radians: step.yaw_degrees.to_radians(),
                            pitch_radians: step.pitch_degrees.to_radians(),
                            weight: step.weight,
                        })
                        .collect::<Vec<_>>();
                    entity.minecart_lerp_steps.extend(rust_steps);
                }
            }
            BackendNetworkEvent::EntityPositionSync {
                id,
                x,
                y,
                z,
                yaw_degrees,
                pitch_degrees,
                on_ground,
            } => {
                outcome.handled = true;
                if Some(*id) != context.local_player_entity_id {
                    let entity_is_ticking = context.level.is_some_and(|level| {
                        self.entities
                            .get(id)
                            .is_some_and(|entity| remote_entity_is_ticking(level, entity))
                    });
                    let locally_authoritative = self.entities.get(id).is_some_and(|entity| {
                        remote_abstract_boat_is_locally_controlled(
                            entity,
                            context.local_player_entity_id,
                        )
                    });
                    let should_position_local_rider =
                        if let Some(entity) = self.entities.get_mut(id) {
                            let target_position_d = glam::DVec3::new(*x, *y, *z);
                            entity.network_position_codec.set_base(target_position_d);
                            if locally_authoritative {
                                false
                            } else {
                                let current_position = if is_remote_abstract_boat(&entity.kind) {
                                    entity.boat_physics_position()
                                } else {
                                    entity.render_position_d_from_authority()
                                };
                                let too_big_to_interpolate =
                                    current_position.distance_squared(target_position_d) > 4096.0;
                                let did_hard_snap = !entity_is_ticking || too_big_to_interpolate;
                                if !did_hard_snap {
                                    entity.note_projectile_render_pose_before_network_update();
                                    if is_remote_abstract_boat(&entity.kind) {
                                        entity.move_or_interpolate_boat_to_degrees(
                                            Some(target_position_d),
                                            Some(*yaw_degrees),
                                            Some(*pitch_degrees),
                                        );
                                    } else {
                                        entity.move_or_interpolate_to_d_degrees(
                                            Some(target_position_d),
                                            Some(*yaw_degrees),
                                            Some(*pitch_degrees),
                                        );
                                    }
                                } else {
                                    entity.snap_to_d_degrees(
                                        target_position_d,
                                        *yaw_degrees,
                                        *pitch_degrees,
                                    );
                                    if is_remote_abstract_boat(&entity.kind) {
                                        entity.set_boat_physics_position(target_position_d);
                                        entity.boat_old_physics_position = Some(target_position_d);
                                    }
                                }
                                entity.on_ground = *on_ground;
                                entity.interpolation_steps <= 0
                            }
                        } else {
                            false
                        };
                    if should_position_local_rider {
                        outcome.should_position_local_rider_vehicle_id = Some(*id);
                    }
                }
            }
            BackendNetworkEvent::EntityTeleport {
                id,
                raw_x,
                raw_y,
                raw_z,
                raw_delta_x,
                raw_delta_y,
                raw_delta_z,
                raw_yaw_degrees,
                raw_pitch_degrees,
                relative,
                on_ground,
            } => {
                outcome.handled = true;
                let change = RemotePositionMoveRotation {
                    position: glam::DVec3::new(*raw_x, *raw_y, *raw_z),
                    velocity: glam::DVec3::new(*raw_delta_x, *raw_delta_y, *raw_delta_z),
                    yaw_degrees: *raw_yaw_degrees,
                    pitch_degrees: *raw_pitch_degrees,
                };
                if !self.entities.contains_key(id) && Some(*id) == context.removed_player_vehicle_id
                {
                    if let (Some(source), Some(old_source)) = (
                        context.local_player_pos_rot,
                        context.local_player_old_pos_rot,
                    ) {
                        let absolute =
                            absolute_remote_position_move_rotation(source, change, *relative);
                        let old =
                            absolute_remote_position_move_rotation(old_source, change, *relative);
                        outcome.local_player_teleport_direct = Some((absolute, old));
                        outcome.local_player_teleport_echo = Some(absolute.position);
                    }
                } else if Some(*id) != context.local_player_entity_id {
                    let entity_is_ticking = context.level.is_some_and(|level| {
                        self.entities
                            .get(id)
                            .is_some_and(|entity| remote_entity_is_ticking(level, entity))
                    });
                    let locally_authoritative = self.entities.get(id).is_some_and(|entity| {
                        remote_abstract_boat_is_locally_controlled(
                            entity,
                            context.local_player_entity_id,
                        )
                    });
                    let has_relative_position = relative.x || relative.y || relative.z;
                    let has_local_indirect_passenger =
                        remote_entity_has_local_player_indirect_passenger(
                            *id,
                            context.local_player_entity_id,
                            &self.entities,
                        );
                    let was_interpolated = if let Some(entity) = self.entities.get_mut(id) {
                        let source = entity.position_move_rotation();
                        let absolute =
                            absolute_remote_position_move_rotation(source, change, *relative);
                        let too_big_to_interpolate =
                            source.position.distance_squared(absolute.position) > 4096.0;
                        let interpolate =
                            entity_is_ticking || !locally_authoritative || has_relative_position;
                        let was_interpolated = interpolate && !too_big_to_interpolate;
                        if was_interpolated {
                            entity.note_projectile_render_pose_before_network_update();
                            if is_remote_abstract_boat(&entity.kind) {
                                entity.move_or_interpolate_boat_to_degrees(
                                    Some(absolute.position),
                                    Some(absolute.yaw_degrees),
                                    Some(absolute.pitch_degrees),
                                );
                                entity.set_boat_physics_velocity(absolute.velocity);
                            } else {
                                entity.move_or_interpolate_to_d_degrees(
                                    Some(absolute.position),
                                    Some(absolute.yaw_degrees),
                                    Some(absolute.pitch_degrees),
                                );
                                entity.set_packet_velocity(absolute.velocity);
                            }
                        } else {
                            let old_source = entity.old_position_move_rotation();
                            let old = absolute_remote_position_move_rotation(
                                old_source, change, *relative,
                            );
                            entity.set_teleport_pose_direct(absolute, old);
                        }
                        entity.on_ground = *on_ground;
                        was_interpolated
                    } else {
                        false
                    };
                    if !was_interpolated && has_local_indirect_passenger {
                        outcome.should_position_local_rider_vehicle_id = Some(*id);
                        if locally_authoritative {
                            if let Some(pose) = self
                                .entities
                                .get(id)
                                .map(RemoteEntityRenderState::position_move_rotation)
                            {
                                outcome.vehicle_move_echo = Some((
                                    pose.position,
                                    pose.yaw_degrees,
                                    pose.pitch_degrees,
                                    self.entities.get(id).is_some_and(|entity| entity.on_ground),
                                ));
                            }
                        }
                    }
                }
            }
            BackendNetworkEvent::EntityDeltaMoved {
                id,
                xa,
                ya,
                za,
                has_position,
                yaw_degrees,
                pitch_degrees,
                on_ground,
            } => {
                outcome.handled = true;
                if Some(*id) != context.local_player_entity_id {
                    let locally_authoritative = self.entities.get(id).is_some_and(|entity| {
                        remote_abstract_boat_is_locally_controlled(
                            entity,
                            context.local_player_entity_id,
                        )
                    });
                    if let Some(entity) = self.entities.get_mut(id) {
                        entity.apply_move_entity_packet(
                            *xa,
                            *ya,
                            *za,
                            *has_position,
                            *yaw_degrees,
                            *pitch_degrees,
                            *on_ground,
                            locally_authoritative,
                        );
                    }
                }
            }
            BackendNetworkEvent::EntityHeadLook {
                id,
                y_head_rot_degrees,
            } => {
                outcome.handled = true;
                let target_head_yaw = y_head_rot_degrees.to_radians();
                if Some(*id) == context.local_player_entity_id {
                    outcome.is_local_player = true;
                    outcome.hurt_dir = Some(target_head_yaw);
                } else if let Some(entity) = self.entities.get_mut(id) {
                    if entity.uses_living_body_rotation() {
                        entity.target_head_yaw = target_head_yaw;
                        entity.head_yaw_steps = 3;
                    } else {
                        entity.head_yaw = target_head_yaw;
                        entity.target_head_yaw = target_head_yaw;
                        entity.head_yaw_steps = 0;
                    }
                }
            }
            BackendNetworkEvent::EntityEquipment {
                id,
                main_hand,
                off_hand,
                head,
                chest,
                legs,
                feet,
                body,
                saddle,
            } => {
                outcome.handled = true;
                if Some(*id) != context.local_player_entity_id {
                    if let Some(entity) = self.entities.get_mut(id) {
                        entity.apply_equipment_slots(
                            main_hand
                                .clone()
                                .map(crate::network::backend_mappers::map_backend_item_stack),
                            off_hand
                                .clone()
                                .map(crate::network::backend_mappers::map_backend_item_stack),
                            head.clone()
                                .map(crate::network::backend_mappers::map_backend_item_stack),
                            chest
                                .clone()
                                .map(crate::network::backend_mappers::map_backend_item_stack),
                            legs.clone()
                                .map(crate::network::backend_mappers::map_backend_item_stack),
                            feet.clone()
                                .map(crate::network::backend_mappers::map_backend_item_stack),
                            body.clone()
                                .map(crate::network::backend_mappers::map_backend_item_stack),
                            saddle
                                .clone()
                                .map(crate::network::backend_mappers::map_backend_item_stack),
                        );
                    }
                }
            }
            BackendNetworkEvent::EntitiesRemoved { ids } => {
                outcome.handled = true;
                for id in ids {
                    if let Some(entity) = self
                        .entities
                        .get(id)
                        .filter(|entity| is_remote_player_entity(&entity.kind))
                    {
                        el_protocol_backend::diagnostics::write_line(&format!(
                            "[RemotePlayerDebug] phase=remove route=network id={} pos=[{:.3},{:.3},{:.3}] flags={:#04x} pose={:?} health={:.3} dead={}",
                            id,
                            entity.position_d.x,
                            entity.position_d.y,
                            entity.position_d.z,
                            entity.shared_flags,
                            entity.pose,
                            entity.living_health,
                            entity.dead,
                        ));
                    }
                    if remote_entity_has_local_player_indirect_passenger(
                        *id,
                        context.local_player_entity_id,
                        &self.entities,
                    ) {
                        context.removed_player_vehicle_id = Some(*id);
                        outcome.removed_player_vehicle_id = Some(*id);
                    }
                    if Some(*id) == context.remote_vehicle_entity_id {
                        context.removed_player_vehicle_id = Some(*id);
                        context.remote_vehicle_entity_id = None;
                        outcome.removed_player_vehicle_id = Some(*id);
                        outcome.local_player_unseated = true;
                    }
                    self.entities.remove(id);
                    detach_removed_entity_from_passenger_graph(&mut self.entities, *id);
                    outcome.removed_ids.push(*id);
                }
            }
            BackendNetworkEvent::ProjectilePower {
                entity_id,
                acceleration_power,
            } => {
                outcome.handled = true;
                apply_projectile_power_like_java(
                    &mut self.entities,
                    *entity_id,
                    *acceleration_power,
                );
            }
            BackendNetworkEvent::SetEntityLink { source_id, dest_id } => {
                outcome.handled = true;
                apply_remote_entity_link_like_java(&mut self.entities, *source_id, *dest_id);
            }
            BackendNetworkEvent::EntityEvent { id, event_id } => {
                outcome.handled = true;
                use el_math::random::{LegacyRandomSource, RandomSource};
                if Some(*id) == context.local_player_entity_id {
                    outcome.is_local_player = true;
                    match *event_id {
                        2 => {
                            outcome.local_kinetic_hit = true;
                        }
                        3 => {
                            outcome.sounds.push((
                                "entity.player.death",
                                crate::client::sound_engine::SoundSource::Players,
                                glam::Vec3::ZERO,
                                1.0,
                                1.0,
                            ));
                        }
                        60 => {
                            outcome.death_poof = Some((glam::Vec3::ZERO, 0.6, 1.8));
                        }
                        9 => {
                            outcome.eat_complete_event_9 = true;
                        }
                        35 => {
                            outcome.totem_used = true;
                            let totem = el_protocol_backend::BackendParticle::Simple {
                                name: "minecraft:totem_of_undying".to_string(),
                            };
                            outcome.tracking_emitters.push((*id, totem, 30));
                            outcome.sounds.push((
                                "item.totem.use",
                                crate::client::sound_engine::SoundSource::Players,
                                glam::Vec3::ZERO,
                                1.0,
                                1.0,
                            ));
                        }
                        _ => {}
                    }
                    return outcome;
                }
                let Some(entity) = self.entities.get_mut(id) else {
                    return outcome;
                };
                let pos = entity.position;
                let (w, h) = remote_entity_pick_size(entity);
                match *event_id {
                    2 => {}
                    3 => {
                        entity.apply_death_entity_event();
                        if is_remote_player_entity(&entity.kind) {
                            el_protocol_backend::diagnostics::write_line(&format!(
                                "[CombatTrace] player_death_sound id={id} retained=true"
                            ));
                            outcome.player_death_sound = Some(pos);
                        }
                    }
                    35 => {
                        let totem = el_protocol_backend::BackendParticle::Simple {
                            name: "minecraft:totem_of_undying".to_string(),
                        };
                        outcome.tracking_emitters.push((*id, totem, 30));
                        outcome.sounds.push((
                            "item.totem.use",
                            crate::client::sound_engine::SoundSource::Players,
                            pos,
                            1.0,
                            1.0,
                        ));
                    }
                    20 | 60 => {
                        outcome.death_poof = Some((pos, w, h));
                    }
                    6 => {
                        let mut rng = LegacyRandomSource::new(context.next_particle_seed());
                        let smoke = el_protocol_backend::BackendParticle::Simple {
                            name: "minecraft:smoke".to_string(),
                        };
                        for _ in 0..7 {
                            let p = glam::Vec3::new(
                                pos.x + (rng.next_float() * 2.0 - 1.0) * w,
                                pos.y + 0.5 + rng.next_float() * h,
                                pos.z + (rng.next_float() * 2.0 - 1.0) * w,
                            );
                            let vel = glam::Vec3::new(
                                (rng.next_float() - 0.5) * 0.02,
                                rng.next_float() * 0.05,
                                (rng.next_float() - 0.5) * 0.02,
                            );
                            outcome.particles.push((smoke.clone(), p, vel));
                        }
                    }
                    7 | 12 | 18 => {
                        let mut rng = LegacyRandomSource::new(context.next_particle_seed());
                        let heart = el_protocol_backend::BackendParticle::Simple {
                            name: "minecraft:heart".to_string(),
                        };
                        for _ in 0..7 {
                            let p = glam::Vec3::new(
                                pos.x + (rng.next_float() * 2.0 - 1.0) * w,
                                pos.y + 0.5 + rng.next_float() * h,
                                pos.z + (rng.next_float() * 2.0 - 1.0) * w,
                            );
                            let vel = glam::Vec3::new(
                                (rng.next_float() - 0.5) * 0.02,
                                rng.next_float() * 0.05,
                                (rng.next_float() - 0.5) * 0.02,
                            );
                            outcome.particles.push((heart.clone(), p, vel));
                        }
                    }
                    13 => {
                        let mut rng = LegacyRandomSource::new(context.next_particle_seed());
                        let angry = el_protocol_backend::BackendParticle::Simple {
                            name: "minecraft:angry_villager".to_string(),
                        };
                        for _ in 0..5 {
                            let p = glam::Vec3::new(
                                pos.x + (rng.next_float() * 2.0 - 1.0) * w * 0.5,
                                pos.y + h + 0.5 + rng.next_float() * 0.5,
                                pos.z + (rng.next_float() * 2.0 - 1.0) * w * 0.5,
                            );
                            outcome.particles.push((angry.clone(), p, glam::Vec3::ZERO));
                        }
                    }
                    14 | 38 => {
                        let mut rng = LegacyRandomSource::new(context.next_particle_seed());
                        let happy = el_protocol_backend::BackendParticle::Simple {
                            name: "minecraft:happy_villager".to_string(),
                        };
                        for _ in 0..7 {
                            let p = glam::Vec3::new(
                                pos.x + (rng.next_float() * 2.0 - 1.0) * w,
                                pos.y + 0.5 + rng.next_float() * h,
                                pos.z + (rng.next_float() * 2.0 - 1.0) * w,
                            );
                            let vel = glam::Vec3::new(
                                (rng.next_float() - 0.5) * 0.02,
                                rng.next_float() * 0.02,
                                (rng.next_float() - 0.5) * 0.02,
                            );
                            outcome.particles.push((happy.clone(), p, vel));
                        }
                    }
                    15 => {
                        let mut rng = LegacyRandomSource::new(context.next_particle_seed());
                        let witch = el_protocol_backend::BackendParticle::Simple {
                            name: "minecraft:witch".to_string(),
                        };
                        let count = 10 + (rng.next_float() * 25.0) as usize;
                        for _ in 0..count {
                            let p = glam::Vec3::new(
                                pos.x + (rng.next_float() * 2.0 - 1.0) * w * 0.5,
                                pos.y + h + rng.next_float() * 0.5,
                                pos.z + (rng.next_float() * 2.0 - 1.0) * w * 0.5,
                            );
                            let vel = glam::Vec3::new(
                                (rng.next_float() - 0.5) * 0.05,
                                rng.next_float() * 0.05,
                                (rng.next_float() - 0.5) * 0.05,
                            );
                            outcome.particles.push((witch.clone(), p, vel));
                        }
                    }
                    42 => {
                        let mut rng = LegacyRandomSource::new(context.next_particle_seed());
                        let splash = el_protocol_backend::BackendParticle::Simple {
                            name: "minecraft:splash".to_string(),
                        };
                        for _ in 0..8 {
                            let p = glam::Vec3::new(
                                pos.x + (rng.next_float() * 2.0 - 1.0) * w,
                                pos.y + 0.5 + rng.next_float() * h,
                                pos.z + (rng.next_float() * 2.0 - 1.0) * w,
                            );
                            let vel = glam::Vec3::new(
                                (rng.next_float() - 0.5) * 0.05,
                                rng.next_float() * 0.1,
                                (rng.next_float() - 0.5) * 0.05,
                            );
                            outcome.particles.push((splash.clone(), p, vel));
                        }
                    }
                    46 => {
                        let mut rng = LegacyRandomSource::new(context.next_particle_seed());
                        let portal = el_protocol_backend::BackendParticle::Simple {
                            name: "minecraft:portal".to_string(),
                        };
                        for _ in 0..64 {
                            let p = glam::Vec3::new(
                                pos.x + (rng.next_float() * 2.0 - 1.0) * w,
                                pos.y + rng.next_float() * h,
                                pos.z + (rng.next_float() * 2.0 - 1.0) * w,
                            );
                            let vel = glam::Vec3::new(
                                (rng.next_float() - 0.5) * 0.5,
                                (rng.next_float() - 0.5) * 0.5,
                                (rng.next_float() - 0.5) * 0.5,
                            );
                            outcome.particles.push((portal.clone(), p, vel));
                        }
                    }
                    66 => {
                        if remote_entity_type_key(&entity.kind) == "creaking" {
                            entity.creaking_invulnerability_remaining_ticks = 8;
                        }
                    }
                    4 => {
                        if remote_entity_type_key(&entity.kind) == "creaking" {
                            entity.creaking_attack_remaining_ticks = 15;
                        }
                    }
                    67 => {
                        let mut rng = LegacyRandomSource::new(context.next_particle_seed());
                        let bubble = el_protocol_backend::BackendParticle::Simple {
                            name: "minecraft:bubble".to_string(),
                        };
                        for _ in 0..8 {
                            let p = glam::Vec3::new(
                                pos.x + (rng.next_float() * 2.0 - 1.0) * w * 0.5,
                                pos.y + h * 0.8 + rng.next_float() * 0.2,
                                pos.z + (rng.next_float() * 2.0 - 1.0) * w * 0.5,
                            );
                            let vel = glam::Vec3::new(
                                (rng.next_float() - 0.5) * 0.05,
                                0.05 + rng.next_float() * 0.1,
                                (rng.next_float() - 0.5) * 0.05,
                            );
                            outcome.particles.push((bubble.clone(), p, vel));
                        }
                    }
                    69 => {
                        let poof = el_protocol_backend::BackendParticle::Simple {
                            name: "minecraft:poof".to_string(),
                        };
                        for i in 0..40 {
                            let angle = i as f32 * std::f32::consts::PI * 2.0 / 40.0;
                            let speed = 0.5;
                            let vel =
                                glam::Vec3::new(angle.cos() * speed, 0.1, angle.sin() * speed);
                            outcome.particles.push((
                                poof.clone(),
                                pos + glam::Vec3::new(0.0, h * 0.5, 0.0),
                                vel,
                            ));
                        }
                    }
                    _ => {}
                }
            }
            BackendNetworkEvent::EntityAnimation { id, action } => {
                outcome.handled = true;
                if Some(*id) == context.local_player_entity_id {
                    outcome.is_local_player = true;
                    match *action {
                        0 => {
                            outcome.swing_hand =
                                Some(crate::render::model::player::player_model::HandSide::Right);
                        }
                        3 => {
                            outcome.swing_hand =
                                Some(crate::render::model::player::player_model::HandSide::Left);
                        }
                        4 => {
                            let crit = el_protocol_backend::BackendParticle::Simple {
                                name: "minecraft:crit".to_string(),
                            };
                            outcome.tracking_emitters.push((*id, crit, 3));
                        }
                        5 => {
                            let magic_crit = el_protocol_backend::BackendParticle::Simple {
                                name: "minecraft:enchanted_hit".to_string(),
                            };
                            outcome.tracking_emitters.push((*id, magic_crit, 3));
                        }
                        _ => {}
                    }
                } else if let Some(entity) = self.entities.get_mut(id) {
                    match *action {
                        0 => {
                            if entity.uses_living_body_rotation() {
                                entity
                                    .swing(crate::render::entity::state::InteractionHand::MainHand);
                            }
                        }
                        3 => {
                            if entity.uses_living_body_rotation() {
                                entity
                                    .swing(crate::render::entity::state::InteractionHand::OffHand);
                            }
                        }
                        4 => {
                            let crit = el_protocol_backend::BackendParticle::Simple {
                                name: "minecraft:crit".to_string(),
                            };
                            outcome.tracking_emitters.push((*id, crit, 3));
                        }
                        5 => {
                            let magic_crit = el_protocol_backend::BackendParticle::Simple {
                                name: "minecraft:enchanted_hit".to_string(),
                            };
                            outcome.tracking_emitters.push((*id, magic_crit, 3));
                        }
                        _ => {}
                    }
                }
            }
            BackendNetworkEvent::Respawn { dimension, .. } => {
                outcome.handled = true;
                let replaces_level = self.handle_respawn(context.current_dimension, dimension);
                if replaces_level {
                    context.remote_vehicle_entity_id = None;
                    outcome.dimension_replaced_level = true;
                }
            }
            _ => {}
        }
        outcome
    }
}
#[derive(Debug, Clone, Default)]
pub struct RemoteEntitiesTickOutput {
    pub projectile_particles: Vec<(el_protocol_backend::BackendParticle, glam::Vec3, glam::Vec3)>,
}
impl RemoteEntities {
    pub fn tick_all(
        &mut self,
        level: Option<&ClientLevel>,
        locally_controlled_boat: Option<i32>,
        frozen_entity_ids: Option<&std::collections::HashSet<i32>>,
        local_player_pos_and_eye: Option<(glam::Vec3, f32)>,
    ) -> RemoteEntitiesTickOutput {
        let has_any_boats = self
            .entities
            .values()
            .any(|e| is_remote_abstract_boat(&e.kind));
        let boat_interpolation_colliders: HashMap<i32, Vec<el_math::VoxelShape>> = if has_any_boats
        {
            self.entities
                .iter()
                .filter_map(|(&id, entity)| {
                    if !is_remote_abstract_boat(&entity.kind)
                        || Some(id) == locally_controlled_boat
                        || !level.is_some_and(|lvl| remote_entity_is_ticking(lvl, entity))
                        || frozen_entity_ids
                            .as_ref()
                            .is_some_and(|frozen| frozen.contains(&id))
                    {
                        return None;
                    }
                    Some((
                        id,
                        collect_hard_entity_colliders(&self.entities, None, Some(id)),
                    ))
                })
                .collect()
        } else {
            HashMap::new()
        };
        let has_any_passengers = self.entities.values().any(|e| !e.passengers.is_empty());
        let remote_mob_entity_ids: std::collections::HashSet<i32> = if has_any_passengers {
            self.entities
                .iter()
                .filter_map(|(&id, entity)| is_remote_mob_entity_type(&entity.kind).then_some(id))
                .collect()
        } else {
            std::collections::HashSet::new()
        };
        let has_any_parrots = self
            .entities
            .values()
            .any(|e| e.kind == "parrot" || e.kind == "minecraft:parrot");
        let remote_passenger_ids: std::collections::HashSet<i32> =
            if has_any_parrots && has_any_passengers {
                self.entities
                    .values()
                    .flat_map(|vehicle| vehicle.passengers.iter().copied())
                    .collect()
            } else {
                std::collections::HashSet::new()
            };
        let has_any_exp_orbs = self
            .entities
            .values()
            .any(|e| remote_entity_type_key_eq(&e.kind, "experience_orb"));
        let experience_follow_players = if has_any_exp_orbs {
            let mut players = Vec::new();
            if let Some((pos, eye)) = local_player_pos_and_eye {
                players.push((pos, eye));
            }
            players.extend(
                self.entities
                    .values()
                    .filter(|entity| is_remote_player_entity(&entity.kind) && !entity.dead)
                    .map(|entity| (entity.position, entity.camera_eye_height())),
            );
            players
        } else {
            Vec::new()
        };
        let mut projectile_particles: Vec<(
            el_protocol_backend::BackendParticle,
            glam::Vec3,
            glam::Vec3,
        )> = Vec::new();
        for entity in self.entities.values_mut() {
            if Some(entity.id) == locally_controlled_boat
                || (level.is_some() && !remote_entity_is_ticking(level.unwrap(), entity))
                || frozen_entity_ids
                    .as_ref()
                    .is_some_and(|frozen| frozen.contains(&entity.id))
            {
                continue;
            }
            entity.tick_death_animation();
            if entity.hurt_time > 0.0 {
                entity.hurt_time = (entity.hurt_time - 1.0).max(0.0);
            }
            if entity.invulnerable_time > 0.0 {
                entity.invulnerable_time = (entity.invulnerable_time - 1.0).max(0.0);
            }
            entity.begin_client_tick();
            if is_remote_item_entity(&entity.kind) {
                entity.age_in_ticks = entity.age_in_ticks.saturating_add(1);
                if entity.interpolation_steps > 0 {
                    entity.tick_interpolation();
                    continue;
                }
                if let Some(lvl) = level {
                    tick_remote_item_entity_like_java(lvl, entity, &[]);
                } else {
                    entity.velocity.y -= entity.gravity_from_default_like_java(0.04) as f32;
                    entity.set_render_position_d(entity.position_d + entity.velocity.as_dvec3());
                    entity.velocity *= 0.98;
                }
            } else {
                let entity_type = remote_entity_type_key(&entity.kind);
                if entity_type == "falling_block" {
                    entity.age_in_ticks = entity.age_in_ticks.saturating_add(1);
                    if entity.interpolation_steps > 0 {
                        entity.tick_interpolation();
                    } else {
                        entity.velocity.y -= entity.gravity_from_default_like_java(0.04) as f32;
                        entity
                            .set_render_position_d(entity.position_d + entity.velocity.as_dvec3());
                        entity.velocity *= 0.98;
                    }
                } else if entity_type == "tnt" {
                    entity.age_in_ticks = entity.age_in_ticks.saturating_add(1);
                    entity.primed_tnt_fuse = entity.primed_tnt_fuse.saturating_sub(1);
                    if entity.interpolation_steps > 0 {
                        entity.tick_interpolation();
                    } else if let Some(level) = level {
                        tick_remote_tnt_motion_like_java(level, entity);
                    } else {
                        entity.velocity.y -= entity.gravity_from_default_like_java(0.04) as f32;
                        entity
                            .set_render_position_d(entity.position_d + entity.velocity.as_dvec3());
                        entity.velocity *= 0.98;
                        if entity.on_ground {
                            entity.velocity *= glam::Vec3::new(0.7, -0.5, 0.7);
                        }
                    }
                    if entity.primed_tnt_fuse > 0 {
                        projectile_particles.push((
                            el_protocol_backend::BackendParticle::Simple {
                                name: "minecraft:smoke".to_string(),
                            },
                            entity.position + glam::Vec3::new(0.0, 0.5, 0.0),
                            glam::Vec3::ZERO,
                        ));
                    }
                } else if entity_type == "experience_orb" {
                    if entity.interpolation_steps > 0 {
                        entity.tick_interpolation();
                    }
                    if let Some(lvl) = level {
                        entity
                            .tick_experience_orb_motion_like_java(lvl, &experience_follow_players);
                    }
                } else if is_remote_abstract_boat(&entity.kind) {
                    let colliders = boat_interpolation_colliders
                        .get(&entity.id)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]);
                    if let Some(lvl) = level {
                        entity.tick_boat_interpolation_like_java(lvl, colliders);
                    } else {
                        entity.tick_interpolation();
                    }
                    entity.zero_boat_delta_movement_like_java();
                } else if is_remote_projectile_render_entity(&entity.kind) {
                    if let Some(lvl) = level {
                        entity.tick_projectile_motion_like_java(lvl);
                    } else {
                        entity.tick_interpolation();
                    }
                    let entity_type = remote_entity_type_key(&entity.kind);
                    if matches!(entity_type, "arrow" | "spectral_arrow")
                        && (entity.abstract_arrow_flags & 1) != 0
                        && !entity.abstract_arrow_in_ground
                    {
                        let movement = entity.velocity;
                        for i in 0..4 {
                            let f = i as f32 / 4.0;
                            let pos = entity.position - movement * f;
                            let vel = glam::Vec3::new(
                                -movement.x * 0.2,
                                -movement.y * 0.2 + 0.1,
                                -movement.z * 0.2,
                            );
                            projectile_particles.push((
                                el_protocol_backend::BackendParticle::Simple {
                                    name: "minecraft:crit".to_string(),
                                },
                                pos,
                                vel,
                            ));
                        }
                    } else if entity_type == "firework_rocket" && entity.age_in_ticks % 2 == 0 {
                        projectile_particles.push((
                            el_protocol_backend::BackendParticle::Simple {
                                name: "minecraft:firework".to_string(),
                            },
                            entity.position,
                            glam::Vec3::new(0.0, -entity.velocity.y * 0.5, 0.0),
                        ));
                    } else if entity_type == "dragon_fireball" {
                        projectile_particles.push((
                            el_protocol_backend::BackendParticle::Simple {
                                name: "minecraft:dragon_breath".to_string(),
                            },
                            entity.position + glam::Vec3::new(0.0, 0.5, 0.0),
                            glam::Vec3::ZERO,
                        ));
                    } else if matches!(entity_type, "wither_skull" | "small_fireball" | "fireball")
                    {
                        projectile_particles.push((
                            el_protocol_backend::BackendParticle::Simple {
                                name: "minecraft:smoke".to_string(),
                            },
                            entity.position + glam::Vec3::new(0.0, 0.5, 0.0),
                            glam::Vec3::ZERO,
                        ));
                    }
                } else {
                    entity.tick_interpolation();
                }
                if entity.uses_living_body_rotation() {
                    entity.tick_using_item();
                    let first_passenger_is_mob = entity
                        .passengers
                        .first()
                        .is_some_and(|id| remote_mob_entity_ids.contains(id));
                    entity.tick_attack_animation();
                    if entity.head_yaw_steps > 0 {
                        entity.head_yaw = rot_lerp_radians(
                            1.0 / entity.head_yaw_steps as f32,
                            entity.head_yaw,
                            entity.target_head_yaw,
                        );
                        entity.head_yaw_steps -= 1;
                    }
                    entity.tick_player_velocity_interpolation();
                    entity.tick_living_body_rotation(first_passenger_is_mob);
                    entity.normalize_old_living_rotations();
                }
                entity.age_in_ticks = entity.age_in_ticks.saturating_add(1);
                entity.tick_swim_amount();
                entity.prev_walk_animation_speed = entity.walk_animation_speed;
                let movement = (entity.position_d - entity.last_tick_position_d).as_vec3();
                entity.last_tick_position_d = entity.position_d;
                entity.last_tick_position = entity.position;
                let distance = glam::Vec2::new(movement.x, movement.z).length();
                if is_remote_player_entity(&entity.kind) {
                    entity
                        .avatar_state
                        .tick(entity.position_d.as_vec3(), entity.velocity);
                    let horizontal_vel =
                        glam::Vec2::new(entity.velocity.x, entity.velocity.z).length();
                    let t_bob = if entity.on_ground && !entity.dead && !entity.shared_flag(4) {
                        horizontal_vel.min(0.1)
                    } else {
                        0.0
                    };
                    entity.avatar_state.update_bob(t_bob);
                }
                if is_remote_camel(&entity.kind) {
                    let standing = entity.pose == el_protocol_backend::BackendEntityPose::Standing;
                    let target_speed =
                        if standing && !entity.camel_dash_animation_state.is_started() {
                            (distance * 6.0).min(1.0)
                        } else {
                            0.0
                        };
                    entity.walk_animation_speed +=
                        (target_speed - entity.walk_animation_speed) * 0.2;
                    entity.walk_animation_pos += entity.walk_animation_speed;
                    entity.walk_position_scale = if entity.is_baby { 3.0 } else { 1.0 };
                } else {
                    let target_speed = (distance * 4.0).min(1.0);
                    entity.walk_animation_speed +=
                        (target_speed - entity.walk_animation_speed) * 0.4;
                    entity.walk_animation_pos += entity.walk_animation_speed;
                    entity.walk_position_scale = 1.0;
                }
                match entity.kind.as_str() {
                    "chicken" => tick_remote_chicken_animation(entity),
                    "parrot" => tick_remote_parrot_animation(
                        entity,
                        remote_passenger_ids.contains(&entity.id),
                    ),
                    "bat" => tick_remote_bat_animation(entity),
                    "slime" | "magma_cube" => tick_remote_cube_mob_animation(entity),
                    "squid" | "glow_squid" => {
                        let is_in_water = remote_entity_intersects_water(entity, level);
                        tick_remote_squid_animation(entity, is_in_water);
                    }
                    "guardian" | "elder_guardian" => {
                        let is_in_water = remote_entity_intersects_water(entity, level);
                        tick_remote_guardian_animation(entity, is_in_water);
                    }
                    "camel" | "camel_husk" => {
                        let game_time = level
                            .map(|l| l.game_time())
                            .unwrap_or(entity.age_in_ticks as i64);
                        tick_remote_camel_animation(entity, game_time);
                    }
                    "creaking" => tick_remote_creaking_animation(entity),
                    _ => {}
                }
                entity.tick_abstract_arrow_metadata_like_java();
            }
        }
        RemoteEntitiesTickOutput {
            projectile_particles,
        }
    }
    pub fn extract_frame(&self, id: i32, partial_tick: f32) -> Option<RemoteEntityFrameSnapshot> {
        let entity = self.entities.get(&id)?;
        let partial_tick = partial_tick.clamp(0.0, 1.0);
        let render_pos = entity.render_position(partial_tick);
        let yaw = entity.render_yaw(partial_tick);
        let pitch = entity.render_pitch(partial_tick);
        let body_yaw = entity.render_body_yaw(partial_tick);
        let head_yaw = rot_lerp_radians(partial_tick, entity.prev_head_yaw, entity.head_yaw);
        let is_invisible = entity.shared_flag(5);
        let is_body_visible = remote_player_fallback_body_visible(entity);
        let hides_all = fallback_invisibility_hides_all_layers(entity);
        let has_independent_layers = !hides_all
            && (is_remote_player_entity(&entity.kind)
                || remote_entity_type_key_eq(&entity.kind, "armor_stand"));
        Some(RemoteEntityFrameSnapshot {
            id,
            kind: entity.kind.clone(),
            position_d: entity.position_d,
            render_position: render_pos,
            yaw,
            pitch,
            body_yaw,
            head_yaw,
            pose: entity.pose,
            is_body_visible,
            is_invisible,
            has_independent_layers,
            health: entity.living_health,
            is_dead: entity.dead,
            death_time: entity.death_time,
            hurt_time: entity.hurt_time,
            walk_anim_pos: entity.walk_animation_position(partial_tick),
            walk_anim_speed: entity.walk_animation_speed(partial_tick),
            main_hand_item: entity.main_hand_item_stack.clone(),
            off_hand_item: entity.off_hand_item_stack.clone(),
            head_item: entity.head_equipment.clone(),
            chest_item: entity.chest_equipment.clone(),
            legs_item: entity.legs_equipment.clone(),
            feet_item: entity.feet_equipment.clone(),
            body_item: entity.body_equipment.clone(),
            saddle_item: entity.saddle_equipment.clone(),
        })
    }
}
