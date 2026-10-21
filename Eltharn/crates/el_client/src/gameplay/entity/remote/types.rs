use super::state::RemoteEntityRenderState;
use crate::gameplay::player::local_player::PlayerPose;
use crate::world::client_level::ClientLevel;
use el_core::block_state::BlockState;
use el_core::collision_context::{CollisionContext, CollisionEntityKind};
use el_core::item::ItemStack;
use el_core::registry::BuiltInRegistries;
use el_core::resource_location::ResourceLocation;
use el_math::aabb::AABB;
use el_math::block_pos::BlockPos;
use el_protocol_backend::{
    BackendCustomNameUpdate, BackendEndCrystalBeamTargetUpdate,
    BackendRelativeMovementFlags as RelativeMovementFlags, BackendRotations,
};
use std::collections::{HashMap, HashSet, VecDeque};

pub const REMOTE_ENTITY_INTERPOLATION_STEPS: i32 = 3;

#[derive(Debug, Clone, Default)]
pub struct RustMinecartStep {
    pub position: glam::Vec3,
    pub movement: glam::Vec3,
    pub yaw_radians: f32,
    pub pitch_radians: f32,
    pub weight: f32,
}

#[derive(Debug, Clone, Default)]
pub struct RemotePlayerSkinInfo {
    pub texture_url: Option<String>,
    pub model: Option<String>,
    pub cape_url: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct RemoteBoatInputLatch {
    pub vehicle_id: i32,
    pub left: bool,
    pub right: bool,
    pub forward: bool,
    pub backward: bool,
}

impl RemoteBoatInputLatch {
    pub const fn empty(vehicle_id: i32) -> Self {
        Self {
            vehicle_id,
            left: false,
            right: false,
            forward: false,
            backward: false,
        }
    }

    pub fn paddle_state(self) -> (bool, bool) {
        (
            (self.right && !self.left) || self.forward,
            (self.left && !self.right) || self.forward,
        )
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RemoteBoatStatus {
    InWater,
    UnderWater,
    UnderFlowingWater,
    OnLand,
    InAir,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteAttributeSnapshot {
    pub base: f64,
    pub modifiers: Vec<crate::gameplay::attributes::AttributeModifier>,
}

impl RemoteAttributeSnapshot {
    pub fn value(&self, definition: crate::gameplay::attributes::RangedAttribute) -> f64 {
        let mut instance = crate::gameplay::attributes::AttributeInstance::new(definition);
        instance.set_base(self.base);
        for modifier in &self.modifiers {
            instance.add_transient_modifier(modifier.clone());
        }
        instance.value()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteEntityInterpolationKind {
    DefaultThreeTicks,
    DisplaySnap,
    None,
    ExistingMinecartPipeline,
}

#[derive(Clone, Copy, Debug)]
pub struct RemoteVecDeltaCodec {
    pub base: glam::DVec3,
}

impl RemoteVecDeltaCodec {
    pub fn new(base: glam::DVec3) -> Self {
        Self { base }
    }

    pub fn encode_component(input: f64) -> i64 {
        (input * 4096.0 + 0.5).floor() as i64
    }

    pub fn decode_component(value: i64) -> f64 {
        value as f64 / 4096.0
    }

    pub fn decode(&self, xa: i16, ya: i16, za: i16) -> glam::DVec3 {
        if xa == 0 && ya == 0 && za == 0 {
            return self.base;
        }

        let decode_axis = |base: f64, delta: i16| {
            if delta == 0 {
                base
            } else {
                Self::decode_component(Self::encode_component(base) + i64::from(delta))
            }
        };
        glam::DVec3::new(
            decode_axis(self.base.x, xa),
            decode_axis(self.base.y, ya),
            decode_axis(self.base.z, za),
        )
    }

    pub fn set_base(&mut self, base: glam::DVec3) {
        self.base = base;
    }
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct RemotePositionMoveRotation {
    pub position: glam::DVec3,
    pub velocity: glam::DVec3,
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThrownItemRendererSpec {
    pub default_item_path: &'static str,
    pub scale: f32,
    pub full_bright: bool,
    pub eye_height: f32,
}

pub const JAVA_FALLING_BLOCK_WIDTH: f32 = 0.98;
pub const JAVA_FALLING_BLOCK_HEIGHT: f32 = 0.98;

pub const REMOTE_ABSTRACT_BOAT_WIDTH: f64 = 1.375;
pub const REMOTE_ABSTRACT_BOAT_HEIGHT: f64 = 0.5625;

pub fn wrap_radians(value: f32) -> f32 {
    (value + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

pub fn wrap_degrees(mut value: f32) -> f32 {
    value %= 360.0;
    if value >= 180.0 {
        value -= 360.0;
    }
    if value < -180.0 {
        value += 360.0;
    }
    value
}

pub fn wrap_degrees_f64(mut value: f64) -> f64 {
    value %= 360.0;
    if value >= 180.0 {
        value -= 360.0;
    }
    if value < -180.0 {
        value += 360.0;
    }
    value
}

#[allow(dead_code)]
pub fn java_entity_yaw_degrees(yaw: f32) -> f32 {
    yaw % 360.0
}

pub fn java_entity_pitch(pitch: f32) -> f32 {
    (pitch % std::f32::consts::TAU).clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2)
}

pub fn java_entity_pitch_degrees(pitch: f32) -> f32 {
    (pitch % 360.0).clamp(-90.0, 90.0)
}

#[allow(dead_code)]
pub fn java_entity_set_x_rot(pitch: f32) -> f32 {
    java_entity_pitch(pitch)
}

pub fn unwrap_old_rotation(current: f32, mut old: f32) -> f32 {
    while current - old < -std::f32::consts::PI {
        old -= std::f32::consts::TAU;
    }
    while current - old >= std::f32::consts::PI {
        old += std::f32::consts::TAU;
    }
    old
}

pub fn unwrap_old_rotation_degrees(current: f32, mut old: f32) -> f32 {
    while current - old < -180.0 {
        old -= 360.0;
    }
    while current - old >= 180.0 {
        old += 360.0;
    }
    old
}

pub fn rot_lerp_radians(alpha: f32, start: f32, end: f32) -> f32 {
    start + alpha * wrap_radians(end - start)
}

pub fn rot_lerp_degrees(alpha: f64, start: f32, end: f32) -> f32 {
    (f64::from(start) + alpha * wrap_degrees_f64(f64::from(end) - f64::from(start))) as f32
}

pub fn projectile_lerp_rotation_like_java(previous: f32, target: f32) -> f32 {
    let mut previous = previous;
    while target - previous < -180.0 {
        previous -= 360.0;
    }
    while target - previous >= 180.0 {
        previous += 360.0;
    }
    previous + (target - previous) * 0.2
}

pub fn remote_entity_type_key(kind: &str) -> &str {
    kind.strip_prefix("minecraft:").unwrap_or(kind)
}

pub fn remote_entity_type_key_eq(kind: &str, expected: &str) -> bool {
    kind.strip_prefix("minecraft:")
        .unwrap_or(kind)
        .eq_ignore_ascii_case(expected)
}

pub fn thrown_item_renderer_spec_like_java(kind: &str) -> Option<ThrownItemRendererSpec> {
    match remote_entity_type_key(kind) {
        "egg" => Some(ThrownItemRendererSpec {
            default_item_path: "egg",
            scale: 1.0,
            full_bright: false,
            eye_height: 0.2125,
        }),
        "ender_pearl" => Some(ThrownItemRendererSpec {
            default_item_path: "ender_pearl",
            scale: 1.0,
            full_bright: false,
            eye_height: 0.2125,
        }),
        "experience_bottle" => Some(ThrownItemRendererSpec {
            default_item_path: "experience_bottle",
            scale: 1.0,
            full_bright: false,
            eye_height: 0.2125,
        }),
        "eye_of_ender" => Some(ThrownItemRendererSpec {
            default_item_path: "ender_eye",
            scale: 1.0,
            full_bright: true,
            eye_height: 0.2125,
        }),
        "fireball" => Some(ThrownItemRendererSpec {
            default_item_path: "fire_charge",
            scale: 3.0,
            full_bright: true,
            eye_height: 0.85,
        }),
        "lingering_potion" => Some(ThrownItemRendererSpec {
            default_item_path: "lingering_potion",
            scale: 1.0,
            full_bright: false,
            eye_height: 0.2125,
        }),
        "small_fireball" => Some(ThrownItemRendererSpec {
            default_item_path: "fire_charge",
            scale: 0.75,
            full_bright: true,
            eye_height: 0.265625,
        }),
        "snowball" => Some(ThrownItemRendererSpec {
            default_item_path: "snowball",
            scale: 1.0,
            full_bright: false,
            eye_height: 0.2125,
        }),
        "splash_potion" => Some(ThrownItemRendererSpec {
            default_item_path: "splash_potion",
            scale: 1.0,
            full_bright: false,
            eye_height: 0.2125,
        }),
        _ => None,
    }
}

pub fn default_thrown_item_stack_like_java(kind: &str) -> Option<ItemStack> {
    let spec = thrown_item_renderer_spec_like_java(kind)?;
    let item = BuiltInRegistries::items()
        .get_by_location(&ResourceLocation::new("minecraft", spec.default_item_path))
        .clone();
    Some(ItemStack::new(item, 1))
}

pub fn default_falling_block_state_like_java() -> BlockState {
    BuiltInRegistries::blocks()
        .get_by_location(&ResourceLocation::new("minecraft", "sand"))
        .default_state()
        .clone()
}

pub fn falling_block_state_from_backend_like_java(state_id: Option<usize>) -> BlockState {
    let Some(state_id) = state_id else {
        return default_falling_block_state_like_java();
    };
    BuiltInRegistries::get_state_by_id(state_id)
        .cloned()
        .unwrap_or_else(|| {
            BuiltInRegistries::blocks()
                .get_by_location(&ResourceLocation::new("minecraft", "air"))
                .default_state()
                .clone()
        })
}

pub fn falling_block_start_pos_from_add_entity_like_java(x: f64, y: f64, z: f64) -> BlockPos {
    BlockPos::new(x.floor() as i32, y.floor() as i32, z.floor() as i32)
}

pub fn falling_block_entity_block_pos_like_java(position: glam::Vec3) -> BlockPos {
    BlockPos::new(
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    )
}

pub fn remote_entity_intersects_water(
    entity: &RemoteEntityRenderState,
    level: Option<&ClientLevel>,
) -> bool {
    let Some(level) = level else {
        return false;
    };
    let pos = BlockPos::new(
        entity.position.x.floor() as i32,
        entity.position.y.floor() as i32,
        entity.position.z.floor() as i32,
    );
    let fluid = level.get_block_state(pos).get_fluid_state();
    fluid.is_water() && entity.position.y as f64 <= pos.y as f64 + fluid.own_height() as f64
}

pub fn remote_entity_is_ticking_from_chunk_status(
    entity: &RemoteEntityRenderState,
    current_chunk_is_loaded: bool,
) -> bool {
    is_remote_player_entity(&entity.kind) || current_chunk_is_loaded
}

pub fn remote_entity_is_ticking(level: &ClientLevel, entity: &RemoteEntityRenderState) -> bool {
    let pos = BlockPos::new(
        entity.position.x.floor() as i32,
        entity.position.y.floor() as i32,
        entity.position.z.floor() as i32,
    );
    remote_entity_is_ticking_from_chunk_status(entity, level.has_chunk_at(pos))
}

pub fn remote_squid_initial_tentacle_speed(entity_id: i32) -> f32 {
    use el_math::random::RandomSource;

    let mut rng = el_math::random::LegacyRandomSource::new(entity_id as i64);
    1.0 / (rng.next_float() + 1.0) * 0.2
}

pub fn is_remote_camel(kind: &str) -> bool {
    matches!(kind, "camel" | "camel_husk")
}

pub fn remote_abstract_boat_aabb(position: glam::DVec3) -> AABB {
    let half_width = REMOTE_ABSTRACT_BOAT_WIDTH * 0.5;
    AABB::new(
        position.x - half_width,
        position.y,
        position.z - half_width,
        position.x + half_width,
        position.y + REMOTE_ABSTRACT_BOAT_HEIGHT,
        position.z + half_width,
    )
}

pub fn remote_abstract_boat_collision_context(
    boat: &RemoteEntityRenderState,
    position_y: f64,
) -> CollisionContext {
    CollisionContext::entity(
        CollisionEntityKind::Entity,
        boat.shared_flags & (1 << 1) != 0,
        position_y,
        None,
        None,
        false,
        0.0,
    )
}

pub fn is_remote_player_entity(kind: &str) -> bool {
    remote_entity_type_key_eq(kind, "player")
}

pub fn is_remote_item_entity(kind: &str) -> bool {
    remote_entity_type_key_eq(kind, "item")
}

pub fn is_remote_living_entity_type(kind: &str) -> bool {
    matches!(
        remote_entity_type_key(kind),
        "allay"
            | "armadillo"
            | "armor_stand"
            | "axolotl"
            | "bat"
            | "bee"
            | "blaze"
            | "bogged"
            | "breeze"
            | "camel"
            | "camel_husk"
            | "cat"
            | "cave_spider"
            | "chicken"
            | "cod"
            | "copper_golem"
            | "cow"
            | "creaking"
            | "creeper"
            | "dolphin"
            | "donkey"
            | "drowned"
            | "elder_guardian"
            | "enderman"
            | "endermite"
            | "ender_dragon"
            | "evoker"
            | "fox"
            | "frog"
            | "ghast"
            | "happy_ghast"
            | "giant"
            | "glow_squid"
            | "goat"
            | "guardian"
            | "hoglin"
            | "horse"
            | "husk"
            | "illusioner"
            | "iron_golem"
            | "llama"
            | "magma_cube"
            | "mannequin"
            | "mooshroom"
            | "mule"
            | "nautilus"
            | "ocelot"
            | "panda"
            | "parched"
            | "parrot"
            | "phantom"
            | "pig"
            | "piglin"
            | "piglin_brute"
            | "pillager"
            | "player"
            | "polar_bear"
            | "pufferfish"
            | "rabbit"
            | "ravager"
            | "salmon"
            | "sheep"
            | "shulker"
            | "silverfish"
            | "skeleton"
            | "skeleton_horse"
            | "slime"
            | "sniffer"
            | "snow_golem"
            | "spider"
            | "squid"
            | "stray"
            | "strider"
            | "sulfur_cube"
            | "tadpole"
            | "trader_llama"
            | "tropical_fish"
            | "turtle"
            | "vex"
            | "villager"
            | "vindicator"
            | "wandering_trader"
            | "warden"
            | "witch"
            | "wither"
            | "wither_skeleton"
            | "wolf"
            | "zoglin"
            | "zombie"
            | "zombie_horse"
            | "zombie_nautilus"
            | "zombie_villager"
            | "zombified_piglin"
    )
}

pub fn is_remote_mob_entity_type(kind: &str) -> bool {
    is_remote_living_entity_type(kind)
        && !matches!(
            remote_entity_type_key(kind),
            "armor_stand" | "mannequin" | "player"
        )
}

pub fn is_remote_abstract_boat(kind: &str) -> bool {
    matches!(
        remote_entity_type_key(kind),
        "acacia_boat"
            | "acacia_chest_boat"
            | "bamboo_raft"
            | "bamboo_chest_raft"
            | "birch_boat"
            | "birch_chest_boat"
            | "cherry_boat"
            | "cherry_chest_boat"
            | "dark_oak_boat"
            | "dark_oak_chest_boat"
            | "jungle_boat"
            | "jungle_chest_boat"
            | "mangrove_boat"
            | "mangrove_chest_boat"
            | "oak_boat"
            | "oak_chest_boat"
            | "pale_oak_boat"
            | "pale_oak_chest_boat"
            | "spruce_boat"
            | "spruce_chest_boat"
    )
}

pub fn is_remote_minecart(kind: &str) -> bool {
    matches!(
        remote_entity_type_key(kind),
        "minecart"
            | "chest_minecart"
            | "command_block_minecart"
            | "furnace_minecart"
            | "hopper_minecart"
            | "spawner_minecart"
            | "tnt_minecart"
    )
}

pub fn remote_avatar_pose(pose: el_protocol_backend::BackendEntityPose) -> PlayerPose {
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
        | el_protocol_backend::BackendEntityPose::Inhaling => PlayerPose::Standing,
        el_protocol_backend::BackendEntityPose::FallFlying => PlayerPose::FallFlying,
        el_protocol_backend::BackendEntityPose::Sleeping => PlayerPose::Sleeping,
        el_protocol_backend::BackendEntityPose::Swimming => PlayerPose::Swimming,
        el_protocol_backend::BackendEntityPose::SpinAttack => PlayerPose::SpinAttack,
        el_protocol_backend::BackendEntityPose::Crouching => PlayerPose::Crouching,
        el_protocol_backend::BackendEntityPose::Dying => PlayerPose::Dying,
    }
}

pub fn is_remote_projectile_render_entity(kind: &str) -> bool {
    matches!(
        remote_entity_type_key(kind),
        "arrow"
            | "spectral_arrow"
            | "trident"
            | "ender_pearl"
            | "egg"
            | "snowball"
            | "fireball"
            | "small_fireball"
            | "dragon_fireball"
            | "wither_skull"
            | "llama_spit"
            | "wind_charge"
            | "breeze_wind_charge"
            | "potion"
            | "experience_bottle"
            | "eye_of_ender"
            | "firework_rocket"
            | "fishing_bobber"
    )
}

pub fn is_remote_abstract_arrow(kind: &str) -> bool {
    matches!(
        remote_entity_type_key(kind),
        "arrow" | "spectral_arrow" | "trident"
    )
}

pub fn remote_entity_pick_size(entity: &RemoteEntityRenderState) -> (f32, f32) {
    if remote_entity_type_key(&entity.kind) == "interaction" {
        return (entity.interaction_width, entity.interaction_height);
    }

    match remote_entity_type_key(&entity.kind) {
        "cow" | "mooshroom" => (0.9, 1.4),
        "pig" => (0.9, 0.9),
        "sheep" => (0.9, 1.3),
        "creeper" => (0.6, 1.7),
        "zombie" | "drowned" | "piglin" | "piglin_brute" | "zombified_piglin" => (0.6, 1.95),
        "skeleton" | "bogged" => (0.6, 1.99),
        "wither_skeleton" => (0.7, 2.4),
        "player" => (0.6, 1.8),
        "armor_stand" => {
            if entity.armor_stand_client_flags & 0x10 != 0 {
                (0.0, 0.0)
            } else if entity.armor_stand_client_flags & 0x01 != 0 {
                (0.25, 0.9875)
            } else {
                (0.5, 1.975)
            }
        }
        "text_display" | "block_display" | "item_display" => (0.0, 0.0),
        "chicken" => (0.4, 0.7),
        "rabbit" => (0.49, 0.6),
        "spider" => (1.4, 0.9),
        "cave_spider" => (0.7, 0.5),
        "slime" | "magma_cube" => (0.52, 0.52),
        "cod" => (0.5, 0.3),
        "salmon" => (0.7, 0.4),
        "pufferfish" => (0.7, 0.7),
        "tadpole" => (0.4, 0.3),
        "squid" | "glow_squid" => (0.8, 0.8),
        "fishing_bobber" => (0.25, 0.25),
        "wind_charge" | "breeze_wind_charge" => (0.3125, 0.3125),
        _ => (0.6, 1.8),
    }
}

pub fn fallback_invisibility_hides_all_layers(entity: &RemoteEntityRenderState) -> bool {
    crate::render::model_frame::fallback_hides_all_layers(
        entity.shared_flag(5),
        entity.shared_flag(6),
        is_remote_player_entity(&entity.kind),
        remote_entity_type_key_eq(&entity.kind, "armor_stand"),
        !entity.head_equipment.is_empty()
            || !entity.chest_equipment.is_empty()
            || !entity.legs_equipment.is_empty()
            || !entity.feet_equipment.is_empty(),
    )
}

pub fn remote_player_fallback_body_visible(entity: &RemoteEntityRenderState) -> bool {
    crate::render::model_frame::player_fallback_body_visible(
        entity.shared_flag(5),
        entity.shared_flag(6),
    )
}

pub fn get_entity_eye_height(kind: &str) -> f32 {
    match remote_entity_type_key(kind) {
        "arrow" | "spectral_arrow" | "trident" => 0.13,
        "end_crystal" => 1.7,
        "falling_block" => 0.98 * 0.85,
        "tnt" => 0.15,
        "item" => 0.2125,
        "experience_orb" => 0.425,
        "player" => 1.62,
        "creeper" => 1.5,
        "zombie" | "skeleton" | "wither_skeleton" => 1.74,
        "spider" | "cave_spider" => 0.5,
        "cow" => 1.3,
        "pig" => 0.8,
        "sheep" => 0.95,
        "chicken" => 0.57,
        "squid" => 0.7,
        "enderman" => 2.55,
        "villager" => 1.62,
        "iron_golem" => 2.25,
        _ => 0.85,
    }
}

pub fn java_vec3_x_rot(vec: glam::DVec3, radians: f32) -> glam::DVec3 {
    let cos = el_math::mth::cos(radians) as f64;
    let sin = el_math::mth::sin(radians) as f64;
    glam::DVec3::new(vec.x, vec.y * cos + vec.z * sin, vec.z * cos - vec.y * sin)
}

pub fn java_vec3_y_rot(vec: glam::DVec3, radians: f32) -> glam::DVec3 {
    let cos = el_math::mth::cos(radians) as f64;
    let sin = el_math::mth::sin(radians) as f64;
    glam::DVec3::new(vec.x * cos + vec.z * sin, vec.y, vec.z * cos - vec.x * sin)
}

pub fn absolute_delta_from_backend_relatives(
    current_delta: glam::DVec3,
    change_delta: glam::DVec3,
    current_yaw_degrees: f32,
    current_pitch_degrees: f32,
    absolute_yaw_degrees: f32,
    absolute_pitch_degrees: f32,
    relative: RelativeMovementFlags,
) -> glam::DVec3 {
    let mut rotated_current_delta = current_delta;
    if relative.rotate_delta {
        let diff_yaw = (current_yaw_degrees - absolute_yaw_degrees).to_radians();
        let diff_pitch = (current_pitch_degrees - absolute_pitch_degrees).to_radians();
        rotated_current_delta = java_vec3_x_rot(rotated_current_delta, diff_pitch);
        rotated_current_delta = java_vec3_y_rot(rotated_current_delta, diff_yaw);
    }

    glam::DVec3::new(
        if relative.delta_x {
            rotated_current_delta.x + change_delta.x
        } else {
            change_delta.x
        },
        if relative.delta_y {
            rotated_current_delta.y + change_delta.y
        } else {
            change_delta.y
        },
        if relative.delta_z {
            rotated_current_delta.z + change_delta.z
        } else {
            change_delta.z
        },
    )
}

pub fn absolute_remote_position_move_rotation(
    source: RemotePositionMoveRotation,
    change: RemotePositionMoveRotation,
    relative: RelativeMovementFlags,
) -> RemotePositionMoveRotation {
    let yaw_degrees = if relative.y_rot {
        source.yaw_degrees + change.yaw_degrees
    } else {
        change.yaw_degrees
    };
    let pitch_degrees = (if relative.x_rot {
        source.pitch_degrees + change.pitch_degrees
    } else {
        change.pitch_degrees
    })
    .clamp(-90.0, 90.0);

    RemotePositionMoveRotation {
        position: glam::DVec3::new(
            if relative.x {
                source.position.x + change.position.x
            } else {
                change.position.x
            },
            if relative.y {
                source.position.y + change.position.y
            } else {
                change.position.y
            },
            if relative.z {
                source.position.z + change.position.z
            } else {
                change.position.z
            },
        ),
        velocity: absolute_delta_from_backend_relatives(
            source.velocity,
            change.velocity,
            source.yaw_degrees,
            source.pitch_degrees,
            yaw_degrees,
            pitch_degrees,
            relative,
        ),
        yaw_degrees,
        pitch_degrees,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RemoteSetPassengersGraphResult {
    pub was_local_indirect_passenger: bool,
    pub local_started_riding: bool,
}

pub fn remote_entity_could_accept_passenger_like_java(kind: &str) -> bool {
    !matches!(
        remote_entity_type_key(kind),
        "marker" | "ominous_item_spawner"
    )
}

pub fn remote_start_riding_would_make_cycle_like_java(
    passenger_id: i32,
    vehicle_id: i32,
    remote_entities: &HashMap<i32, RemoteEntityRenderState>,
) -> bool {
    let mut current_vehicle_id = vehicle_id;
    let mut visited = HashSet::new();
    while visited.insert(current_vehicle_id) {
        let parent_vehicle_id = remote_entities
            .values()
            .find(|candidate| candidate.passengers.contains(&current_vehicle_id))
            .map(|candidate| candidate.id);
        let Some(parent_vehicle_id) = parent_vehicle_id else {
            return false;
        };
        if parent_vehicle_id == passenger_id {
            return true;
        }
        current_vehicle_id = parent_vehicle_id;
    }

    true
}

pub fn remote_root_vehicle_id(
    entity_id: i32,
    remote_entities: &HashMap<i32, RemoteEntityRenderState>,
) -> i32 {
    let mut current = entity_id;
    let mut visited = HashSet::new();

    while visited.insert(current) {
        let parent = remote_entities
            .values()
            .find(|candidate| candidate.passengers.contains(&current))
            .map(|candidate| candidate.id);
        let Some(parent) = parent else {
            break;
        };
        current = parent;
    }

    current
}

pub fn remote_entity_has_player_passenger(
    entity_id: i32,
    remote_player_entity_id: Option<i32>,
    remote_entities: &HashMap<i32, RemoteEntityRenderState>,
) -> bool {
    let Some(entity) = remote_entities.get(&entity_id) else {
        return false;
    };

    let mut pending = VecDeque::from(entity.passengers.clone());
    let mut visited = HashSet::new();
    while let Some(passenger_id) = pending.pop_front() {
        if !visited.insert(passenger_id) {
            continue;
        }
        if Some(passenger_id) == remote_player_entity_id
            || remote_entities
                .get(&passenger_id)
                .is_some_and(|passenger| is_remote_player_entity(&passenger.kind))
        {
            return true;
        }
        if let Some(passenger) = remote_entities.get(&passenger_id) {
            pending.extend(passenger.passengers.iter().copied());
        }
    }

    false
}

pub fn remote_entity_has_local_player_indirect_passenger(
    entity_id: i32,
    remote_player_entity_id: Option<i32>,
    remote_entities: &HashMap<i32, RemoteEntityRenderState>,
) -> bool {
    let Some(local_player_id) = remote_player_entity_id else {
        return false;
    };
    let Some(entity) = remote_entities.get(&entity_id) else {
        return false;
    };

    let mut pending = VecDeque::from(entity.passengers.clone());
    let mut visited = HashSet::new();
    while let Some(passenger_id) = pending.pop_front() {
        if !visited.insert(passenger_id) {
            continue;
        }
        if passenger_id == local_player_id {
            return true;
        }
        if let Some(passenger) = remote_entities.get(&passenger_id) {
            pending.extend(passenger.passengers.iter().copied());
        }
    }

    false
}

pub fn rebuild_remote_passenger_graph_like_java(
    vehicle_id: i32,
    packet_passenger_ids: &[i32],
    local_player_entity_id: Option<i32>,
    local_vehicle_entity_id: &mut Option<i32>,
    remote_entities: &mut HashMap<i32, RemoteEntityRenderState>,
) -> RemoteSetPassengersGraphResult {
    debug_assert!(remote_entities.contains_key(&vehicle_id));

    let was_local_indirect_passenger = remote_entity_has_local_player_indirect_passenger(
        vehicle_id,
        local_player_entity_id,
        remote_entities,
    );
    let vehicle_could_accept_passengers = remote_entities
        .get(&vehicle_id)
        .is_some_and(|vehicle| remote_entity_could_accept_passenger_like_java(&vehicle.kind));

    if *local_vehicle_entity_id == Some(vehicle_id) {
        *local_vehicle_entity_id = None;
    }
    remote_entities
        .get_mut(&vehicle_id)
        .expect("checked vehicle before rebuilding passenger graph")
        .passengers
        .clear();

    if !vehicle_could_accept_passengers {
        return RemoteSetPassengersGraphResult {
            was_local_indirect_passenger,
            local_started_riding: false,
        };
    }

    let mut local_started_riding = false;
    for &passenger_id in packet_passenger_ids {
        let is_local_player = Some(passenger_id) == local_player_entity_id;
        let is_known_remote_entity = remote_entities.contains_key(&passenger_id);

        if !is_local_player && !is_known_remote_entity {
            continue;
        }

        if is_known_remote_entity
            && remote_start_riding_would_make_cycle_like_java(
                passenger_id,
                vehicle_id,
                remote_entities,
            )
        {
            continue;
        }

        let already_riding_this_vehicle = remote_entities
            .get(&vehicle_id)
            .is_some_and(|vehicle| vehicle.passengers.contains(&passenger_id));
        if already_riding_this_vehicle {
            continue;
        }

        for candidate in remote_entities.values_mut() {
            candidate.passengers.retain(|id| *id != passenger_id);
        }
        remote_entities
            .get_mut(&vehicle_id)
            .expect("vehicle cannot disappear while rebuilding passenger graph")
            .passengers
            .push(passenger_id);

        if is_local_player {
            *local_vehicle_entity_id = Some(vehicle_id);
            local_started_riding = true;
        }
    }

    RemoteSetPassengersGraphResult {
        was_local_indirect_passenger,
        local_started_riding,
    }
}

pub fn is_remote_raft(kind: &str) -> bool {
    matches!(
        remote_entity_type_key(kind),
        "bamboo_raft" | "bamboo_chest_raft"
    )
}

pub fn is_remote_abstract_chest_boat(kind: &str) -> bool {
    matches!(
        remote_entity_type_key(kind),
        "acacia_chest_boat"
            | "bamboo_chest_raft"
            | "birch_chest_boat"
            | "cherry_chest_boat"
            | "dark_oak_chest_boat"
            | "jungle_chest_boat"
            | "mangrove_chest_boat"
            | "oak_chest_boat"
            | "pale_oak_chest_boat"
            | "spruce_chest_boat"
    )
}

pub fn remote_abstract_boat_is_locally_controlled(
    vehicle: &RemoteEntityRenderState,
    remote_player_entity_id: Option<i32>,
) -> bool {
    is_remote_abstract_boat(&vehicle.kind)
        && remote_player_entity_id.is_some_and(|local_player_id| {
            vehicle.passengers.first().copied() == Some(local_player_id)
        })
}

pub fn known_locally_authoritative_root_vehicle_id(
    direct_vehicle_id: i32,
    remote_player_entity_id: Option<i32>,
    remote_entities: &HashMap<i32, RemoteEntityRenderState>,
) -> Option<i32> {
    let root_vehicle_id = remote_root_vehicle_id(direct_vehicle_id, remote_entities);
    remote_entities
        .get(&root_vehicle_id)
        .filter(|vehicle| {
            remote_abstract_boat_is_locally_controlled(vehicle, remote_player_entity_id)
        })
        .map(|_| root_vehicle_id)
}

pub fn is_hurting_projectile_kind(kind: &str) -> bool {
    matches!(
        remote_entity_type_key(kind),
        "fireball" | "small_fireball" | "wither_skull" | "dragon_fireball"
    )
}

pub fn apply_projectile_power_like_java(
    entities: &mut HashMap<i32, RemoteEntityRenderState>,
    entity_id: i32,
    acceleration_power: f64,
) {
    if let Some(entity) = entities.get_mut(&entity_id) {
        if is_hurting_projectile_kind(&entity.kind) {
            entity.acceleration_power = acceleration_power;
        }
    }
}

pub fn is_remote_leashable(kind: &str) -> bool {
    is_remote_abstract_boat(kind)
        || (is_remote_living_entity_type(kind)
            && !matches!(remote_entity_type_key(kind), "player" | "armor_stand"))
}

pub fn apply_remote_entity_link_like_java(
    entities: &mut HashMap<i32, RemoteEntityRenderState>,
    source_id: i32,
    dest_id: i32,
) {
    if let Some(entity) = entities.get_mut(&source_id) {
        if is_remote_leashable(&entity.kind) {
            entity.leash_holder_id = dest_id;
        }
    }
}

pub fn apply_remote_item_stack_delta_like_java(
    entity: &mut RemoteEntityRenderState,
    item_stack: Option<ItemStack>,
) {
    if let Some(item_stack) = item_stack {
        entity.item_stack = Some(item_stack);
    }
}

pub fn apply_remote_custom_name_metadata(
    entity: &mut RemoteEntityRenderState,
    custom_name: Option<BackendCustomNameUpdate>,
    custom_name_visible: Option<bool>,
) {
    if let Some(custom_name) = custom_name {
        entity.custom_name = match custom_name {
            BackendCustomNameUpdate::Clear => None,
            BackendCustomNameUpdate::Set(value) => Some(value),
        };
    }
    if let Some(custom_name_visible) = custom_name_visible {
        entity.custom_name_visible = custom_name_visible;
    }
}

pub fn apply_remote_special_entity_metadata(
    entity: &mut RemoteEntityRenderState,
    primed_tnt_fuse: Option<i32>,
    experience_orb_value: Option<i32>,
    primed_tnt_block_state: Option<BlockState>,
    end_crystal_beam_target: Option<BackendEndCrystalBeamTargetUpdate>,
    end_crystal_show_bottom: Option<bool>,
    abstract_arrow_flags: Option<u8>,
    abstract_arrow_pierce_level: Option<u8>,
    abstract_arrow_in_ground: Option<bool>,
    arrow_effect_color: Option<i32>,
) {
    let entity_type = remote_entity_type_key(&entity.kind);
    match entity_type {
        "tnt" => {
            if let Some(fuse) = primed_tnt_fuse {
                entity.primed_tnt_fuse = fuse;
            }
            if let Some(block_state) = primed_tnt_block_state {
                entity.primed_tnt_block_state = block_state;
            }
        }
        "experience_orb" => {
            if let Some(value) = experience_orb_value {
                entity.experience_orb_value = value;
            }
        }
        "end_crystal" => {
            if let Some(target) = end_crystal_beam_target {
                entity.end_crystal_beam_target = match target {
                    BackendEndCrystalBeamTargetUpdate::Clear => None,
                    BackendEndCrystalBeamTargetUpdate::Set(pos) => {
                        Some(BlockPos::new(pos.x, pos.y, pos.z))
                    }
                };
            }
            if let Some(show_bottom) = end_crystal_show_bottom {
                entity.end_crystal_show_bottom = show_bottom;
            }
        }
        "arrow" | "spectral_arrow" | "trident" => {
            if let Some(flags) = abstract_arrow_flags {
                entity.abstract_arrow_flags = flags;
            }
            if let Some(level) = abstract_arrow_pierce_level {
                entity.abstract_arrow_pierce_level = level;
            }
            if let Some(in_ground) = abstract_arrow_in_ground {
                if !entity.abstract_arrow_first_tick
                    && entity.abstract_arrow_shake_time <= 0
                    && in_ground
                {
                    entity.abstract_arrow_shake_time = 7;
                }
                entity.abstract_arrow_in_ground = in_ground;
            }
            if entity_type == "arrow" {
                if let Some(color) = arrow_effect_color {
                    entity.arrow_effect_color = color;
                }
            }
        }
        _ => {}
    }
}

pub fn apply_remote_armor_stand_metadata(
    entity: &mut RemoteEntityRenderState,
    client_flags: Option<u8>,
    head_pose: Option<BackendRotations>,
    body_pose: Option<BackendRotations>,
    left_arm_pose: Option<BackendRotations>,
    right_arm_pose: Option<BackendRotations>,
    left_leg_pose: Option<BackendRotations>,
    right_leg_pose: Option<BackendRotations>,
) {
    if remote_entity_type_key(&entity.kind) != "armor_stand" {
        return;
    }
    if let Some(flags) = client_flags {
        entity.armor_stand_client_flags = flags;
    }
    let to_vec3 = |rotation: BackendRotations| glam::Vec3::new(rotation.x, rotation.y, rotation.z);
    if let Some(rotation) = head_pose {
        entity.armor_stand_head_pose = to_vec3(rotation);
    }
    if let Some(rotation) = body_pose {
        entity.armor_stand_body_pose = to_vec3(rotation);
    }
    if let Some(rotation) = left_arm_pose {
        entity.armor_stand_left_arm_pose = to_vec3(rotation);
    }
    if let Some(rotation) = right_arm_pose {
        entity.armor_stand_right_arm_pose = to_vec3(rotation);
    }
    if let Some(rotation) = left_leg_pose {
        entity.armor_stand_left_leg_pose = to_vec3(rotation);
    }
    if let Some(rotation) = right_leg_pose {
        entity.armor_stand_right_leg_pose = to_vec3(rotation);
    }
}

pub fn apply_remote_mob_flags(entity: &mut RemoteEntityRenderState, flags: Option<u8>) {
    if is_remote_mob_entity_type(&entity.kind)
        && let Some(flags) = flags
    {
        entity.main_arm = if flags & 2 != 0 {
            crate::render::entity::state::HumanoidArm::Left
        } else {
            crate::render::entity::state::HumanoidArm::Right
        };
        entity.is_aggressive = flags & 4 != 0;
    }
}

pub fn apply_remote_mob_boolean_metadata(
    entity: &mut RemoteEntityRenderState,
    deltas: &[(u8, bool)],
) {
    if remote_entity_type_key(&entity.kind) == "pillager" {
        for &(index, value) in deltas {
            if index == 17 {
                entity.pillager_is_charging_crossbow = value;
            }
        }
    }
}

pub fn apply_remote_mob_animation_metadata(
    entity: &mut RemoteEntityRenderState,
    is_baby: Option<bool>,
    is_aggressive: Option<bool>,
    camel_dashing: Option<bool>,
    camel_last_pose_change_tick: Option<i64>,
    piglin_is_baby: Option<bool>,
    piglin_is_charging_crossbow: Option<bool>,
    piglin_is_dancing: Option<bool>,
    tamable_flags: Option<u8>,
    creaking_can_move: Option<bool>,
    creaking_is_active: Option<bool>,
    creaking_is_tearing_down: Option<bool>,
) {
    let key = remote_entity_type_key(&entity.kind);
    if let Some(is_baby) = is_baby.filter(|_| key != "pillager") {
        entity.is_baby = is_baby;
    }
    if let Some(is_aggressive) = is_aggressive {
        entity.is_aggressive = is_aggressive;
    }
    if let Some(camel_dashing) = camel_dashing {
        if camel_dashing && !entity.camel_dashing && entity.camel_dash_cooldown == 0 {
            entity.camel_dash_cooldown = 55;
        }
        entity.camel_dashing = camel_dashing;
    }
    if let Some(camel_last_pose_change_tick) = camel_last_pose_change_tick {
        entity.camel_last_pose_change_tick = camel_last_pose_change_tick;
    }
    if key == "piglin" || key == "piglin_brute" {
        if let Some(baby) = piglin_is_baby {
            entity.is_baby = baby;
        }
        if let Some(charging) = piglin_is_charging_crossbow {
            entity.piglin_is_charging_crossbow = charging;
        }
        if let Some(dancing) = piglin_is_dancing {
            entity.piglin_is_dancing = dancing;
        }
    }
    if let Some(tamable_flags) = tamable_flags {
        entity.is_sitting = tamable_flags & 0x01 != 0;
    }
    if key == "creaking" {
        if let Some(can_move) = creaking_can_move {
            entity.creaking_can_move = can_move;
        }
        if let Some(is_active) = creaking_is_active {
            entity.creaking_is_active = is_active;
        }
        if let Some(is_tearing_down) = creaking_is_tearing_down {
            entity.creaking_is_tearing_down = is_tearing_down;
        }
    }
}

pub fn apply_remote_display_metadata(
    entity: &mut RemoteEntityRenderState,
    display_interpolation_start_delta_ticks: Option<i32>,
    display_interpolation_duration: Option<i32>,
    display_teleport_duration: Option<i32>,
    display_translation: Option<[f32; 3]>,
    display_scale: Option<[f32; 3]>,
    display_left_rotation: Option<[f32; 4]>,
    display_right_rotation: Option<[f32; 4]>,
    display_billboard: Option<u8>,
    display_brightness_override: Option<i32>,
    display_view_range: Option<f32>,
    display_shadow_radius: Option<f32>,
    display_shadow_strength: Option<f32>,
    display_width: Option<f32>,
    display_height: Option<f32>,
    display_glow_color_override: Option<i32>,
    text_display_text: Option<String>,
    text_display_line_width: Option<i32>,
    text_display_background: Option<i32>,
    text_display_text_opacity: Option<i8>,
    text_display_flags: Option<u8>,
) {
    if !remote_entity_type_key_eq(&entity.kind, "text_display") {
        return;
    }
    let state = entity
        .text_display_state
        .get_or_insert_with(crate::render::entity::display::TextDisplayState::default);

    if let Some(start) = display_interpolation_start_delta_ticks {
        state.start_interpolation = start;
    }
    if let Some(duration) = display_interpolation_duration {
        state.interpolation_duration = duration;
    }
    if let Some(teleport) = display_teleport_duration {
        state.teleport_duration = teleport;
    }
    if let Some(translation) = display_translation {
        state.transformation.translation = glam::Vec3::from_array(translation);
    }
    if let Some(scale) = display_scale {
        state.transformation.scale = glam::Vec3::from_array(scale);
    }
    if let Some(left_rot) = display_left_rotation {
        state.transformation.left_rotation =
            glam::Quat::from_array([left_rot[0], left_rot[1], left_rot[2], left_rot[3]]);
    }
    if let Some(right_rot) = display_right_rotation {
        state.transformation.right_rotation =
            glam::Quat::from_array([right_rot[0], right_rot[1], right_rot[2], right_rot[3]]);
    }
    if let Some(billboard) = display_billboard {
        state.billboard = crate::render::entity::display::BillboardConstraints::from_id(billboard);
    }
    if let Some(brightness) = display_brightness_override {
        state.brightness_override = brightness;
    }
    if let Some(view_range) = display_view_range {
        state.view_range = view_range;
    }
    if let Some(radius) = display_shadow_radius {
        state.shadow_radius = radius;
    }
    if let Some(strength) = display_shadow_strength {
        state.shadow_strength = strength;
    }
    if let Some(width) = display_width {
        state.width = width;
    }
    if let Some(height) = display_height {
        state.height = height;
    }
    if let Some(glow) = display_glow_color_override {
        state.glow_color_override = glow;
    }
    if let Some(text) = text_display_text {
        state.text = text;
    }
    if let Some(line_width) = text_display_line_width {
        state.line_width = line_width;
    }
    if let Some(background) = text_display_background {
        state.background_color = background;
    }
    if let Some(opacity) = text_display_text_opacity {
        state.text_opacity = opacity;
    }
    if let Some(flags) = text_display_flags {
        state.flags = flags;
    }
}

pub fn remote_entity_hard_collision_size(kind: &str) -> Option<(f32, f32)> {
    match remote_entity_type_key(kind) {
        "boat"
        | "oak_boat"
        | "spruce_boat"
        | "birch_boat"
        | "jungle_boat"
        | "acacia_boat"
        | "dark_oak_boat"
        | "mangrove_boat"
        | "cherry_boat"
        | "bamboo_raft"
        | "pale_oak_boat"
        | "chest_boat"
        | "oak_chest_boat"
        | "spruce_chest_boat"
        | "birch_chest_boat"
        | "jungle_chest_boat"
        | "acacia_chest_boat"
        | "dark_oak_chest_boat"
        | "mangrove_chest_boat"
        | "cherry_chest_boat"
        | "bamboo_chest_raft"
        | "pale_oak_chest_boat" => Some((1.375, 0.5625)),
        "shulker" => Some((1.0, 1.0)),
        _ => None,
    }
}

pub fn remote_entity_can_be_collided_with(kind: &str) -> bool {
    remote_entity_hard_collision_size(kind).is_some()
}

pub fn collect_hard_entity_colliders(
    remote_entities: &HashMap<i32, RemoteEntityRenderState>,
    remote_player_entity_id: Option<i32>,
    local_direct_vehicle_id: Option<i32>,
) -> Vec<el_math::VoxelShape> {
    let local_root_vehicle = local_direct_vehicle_id
        .map(|vehicle_id| remote_root_vehicle_id(vehicle_id, remote_entities));
    let mut shapes = Vec::new();
    for entity in remote_entities.values() {
        if entity.dead || remote_player_entity_id == Some(entity.id) {
            continue;
        }
        if !remote_entity_can_be_collided_with(&entity.kind) {
            continue;
        }
        let root_vehicle_id = remote_root_vehicle_id(entity.id, remote_entities);
        if local_root_vehicle == Some(root_vehicle_id) {
            continue;
        }
        let (width, height) = remote_entity_hard_collision_size(&entity.kind).unwrap();
        let half_width = width as f64 * 0.5;
        let pos = entity.position_d;
        let min_x = pos.x - half_width;
        let min_y = pos.y;
        let min_z = pos.z - half_width;
        let max_x = pos.x + half_width;
        let max_y = pos.y + height as f64;
        let max_z = pos.z + half_width;
        shapes.push(el_math::VoxelShape::from_aabb(AABB::new(
            min_x, min_y, min_z, max_x, max_y, max_z,
        )));
    }
    shapes
}

pub fn player_hurt_sound_path_by_damage_type_id_like_java(source_type_id: u32) -> &'static str {
    const HURT: &str = "entity.player.hurt";
    const HURT_DROWN: &str = "entity.player.hurt_drown";
    const HURT_ON_FIRE: &str = "entity.player.hurt_on_fire";
    const HURT_SWEET_BERRY_BUSH: &str = "entity.player.hurt_sweet_berry_bush";
    const HURT_FREEZE: &str = "entity.player.hurt_freeze";
    match source_type_id {
        0 | 1 | 3 | 4 | 5 | 6 | 37 | 38 => HURT_ON_FIRE,
        9 => HURT_DROWN,
        21 => HURT_SWEET_BERRY_BUSH,
        22 => HURT_FREEZE,
        _ => HURT,
    }
}

pub(crate) fn remote_entity_is_player_rideable_jumping(kind: &str) -> bool {
    matches!(
        remote_entity_type_key(kind),
        "horse"
            | "donkey"
            | "mule"
            | "zombie_horse"
            | "skeleton_horse"
            | "camel"
            | "nautilus"
            | "zombie_nautilus"
    )
}
