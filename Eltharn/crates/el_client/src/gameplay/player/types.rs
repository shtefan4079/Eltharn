#![allow(dead_code)]
use glam::DVec3;
use std::cmp::Ordering;
pub const DEFAULT_BASE_GRAVITY: f64 = 0.08;
pub const BASE_JUMP_POWER: f32 = 0.42;
pub const BASE_VERTICAL_AIR_DRAG: f32 = 0.98;
pub const INPUT_FRICTION: f32 = 0.98;
pub const AIR_HORIZONTAL_DRAG: f32 = 0.91;
pub const DEFAULT_BLOCK_FRICTION: f32 = 0.6;
pub const PLAYER_BASE_MOVEMENT_SPEED: f32 = 0.1;
pub const SPRINTING_SPEED_MODIFIER: f32 = 0.3;
pub const PLAYER_FLYING_SPEED: f32 = 0.02;
pub const PLAYER_SPRINT_FLYING_SPEED: f32 = 0.025_999_999;
pub const SNEAKING_SPEED: f32 = 0.3;
pub const GROUND_SPEED_FRICTION_NUMERATOR: f32 = 0.216_000_02;
pub const JUMP_COOLDOWN_TICKS: i32 = 10;
pub const PLAYER_BB_WIDTH: f64 = 0.6_f32 as f64;
pub const PLAYER_STANDING_BB_HEIGHT: f64 = 1.8_f32 as f64;
pub const PLAYER_CROUCHING_BB_HEIGHT: f64 = 1.5_f32 as f64;
pub const PLAYER_SWIMMING_BB_HEIGHT: f64 = 0.6_f32 as f64;
pub const PLAYER_MAX_UP_STEP: f32 = 0.6;
pub const SPRINT_WINDOW_MIN_TICKS: i32 = 0;
pub const SPRINT_WINDOW_MAX_TICKS: i32 = 10;
pub const DEFAULT_SPRINT_WINDOW_TICKS: i32 = 7;
pub const SLIDE_PROBE_ENV: &str = "ELTHARN_SLIDE_PROBE";
pub const MOVEMENT_DIAGNOSTICS_ENV: &str = "ELTHARN_MOVEMENT_DIAGNOSTICS";
pub const MOB_EFFECT_SPEED: u32 = 0;
pub const MOB_EFFECT_SLOWNESS: u32 = 1;
pub const MOB_EFFECT_HASTE: u32 = 2;
pub const MOB_EFFECT_MINING_FATIGUE: u32 = 3;
pub const MOB_EFFECT_REGENERATION: u32 = 9;
pub const MOB_EFFECT_JUMP_BOOST: u32 = 7;
pub const MOB_EFFECT_BLINDNESS: u32 = 14;
pub const MOB_EFFECT_HUNGER: u32 = 16;
pub const MOB_EFFECT_POISON: u32 = 18;
pub const MOB_EFFECT_WITHER: u32 = 19;
pub const MOB_EFFECT_LEVITATION: u32 = 24;
pub const MOB_EFFECT_SLOW_FALLING: u32 = 27;
pub const MOB_EFFECT_CONDUIT_POWER: u32 = 28;
pub const MOB_EFFECT_DOLPHINS_GRACE: u32 = 29;
pub const MOB_EFFECT_WEAVING: u32 = 36;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveMobEffect {
    pub amplifier: i32,
    pub duration: i32,
    pub ambient: bool,
    pub show_particles: bool,
    pub show_icon: bool,
    pub blend: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderwaterSoundCue {
    Enter,
    Exit,
}
impl UnderwaterSoundCue {
    pub const fn sound_event(self) -> &'static str {
        match self {
            Self::Enter => "ambient.underwater.enter",
            Self::Exit => "ambient.underwater.exit",
        }
    }
}
pub fn mob_effect_path(effect_id: u32) -> Option<&'static str> {
    match effect_id {
        0 => Some("speed"),
        1 => Some("slowness"),
        2 => Some("haste"),
        3 => Some("mining_fatigue"),
        4 => Some("strength"),
        5 => Some("instant_health"),
        6 => Some("instant_damage"),
        7 => Some("jump_boost"),
        8 => Some("nausea"),
        9 => Some("regeneration"),
        10 => Some("resistance"),
        11 => Some("fire_resistance"),
        12 => Some("water_breathing"),
        13 => Some("invisibility"),
        14 => Some("blindness"),
        15 => Some("night_vision"),
        16 => Some("hunger"),
        17 => Some("weakness"),
        18 => Some("poison"),
        19 => Some("wither"),
        20 => Some("health_boost"),
        21 => Some("absorption"),
        22 => Some("saturation"),
        23 => Some("glowing"),
        24 => Some("levitation"),
        25 => Some("luck"),
        26 => Some("unluck"),
        27 => Some("slow_falling"),
        28 => Some("conduit_power"),
        29 => Some("dolphins_grace"),
        30 => Some("bad_omen"),
        31 => Some("hero_of_the_village"),
        32 => Some("darkness"),
        33 => Some("trial_omen"),
        34 => Some("raid_omen"),
        35 => Some("wind_charged"),
        36 => Some("weaving"),
        37 => Some("oozing"),
        38 => Some("infested"),
        39 => Some("breath_of_the_nautilus"),
        _ => None,
    }
}
pub fn mob_effect_is_beneficial(effect_id: u32) -> bool {
    matches!(
        effect_id,
        0 | 2 | 4 | 7 | 9 | 10 | 11 | 12 | 15 | 20 | 22 | 25 | 28 | 29 | 31 | 39
    )
}
pub fn mob_effect_color(effect_id: u32) -> i32 {
    match effect_id {
        0 => 3_402_751,
        1 => 9_154_528,
        2 => 14_270_531,
        3 => 4_866_583,
        4 => 16_762_624,
        5 => 16_262_179,
        6 => 11_101_546,
        7 => 16_646_020,
        8 => 5_578_058,
        9 => 13_458_603,
        10 => 9_520_880,
        11 => 16_750_848,
        12 => 10_017_472,
        13 => 16_185_078,
        14 => 2_039_587,
        15 => 12_779_366,
        16 => 5_797_459,
        17 => 4_738_376,
        18 => 8_889_187,
        19 => 7_561_558,
        20 => 16_284_963,
        21 => 2_445_989,
        22 => 16_262_179,
        23 => 9_740_385,
        24 => 13_565_951,
        25 => 5_882_118,
        26 => 12_624_973,
        27 => 15_978_425,
        28 => 1_950_417,
        29 => 8_954_814,
        30 => 745_784,
        31 => 4_521_796,
        32 => 2_696_993,
        33 => 1_484_454,
        34 => 14_565_464,
        35 => 12_438_015,
        36 => 7_891_290,
        37 => 10_092_451,
        38 => 9_214_860,
        39 => 65_518,
        _ => i32::MAX,
    }
}
pub fn compare_mob_effects(
    left_id: u32,
    left: ActiveMobEffect,
    right_id: u32,
    right: ActiveMobEffect,
) -> Ordering {
    if (left.duration <= 32_147 || right.duration <= 32_147) && (!left.ambient || !right.ambient) {
        left.ambient
            .cmp(&right.ambient)
            .then_with(|| (left.duration == -1).cmp(&(right.duration == -1)))
            .then_with(|| left.duration.cmp(&right.duration))
            .then_with(|| mob_effect_color(left_id).cmp(&mob_effect_color(right_id)))
    } else {
        left.ambient
            .cmp(&right.ambient)
            .then_with(|| mob_effect_color(left_id).cmp(&mob_effect_color(right_id)))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerPose {
    Standing,
    Crouching,
    Swimming,
    FallFlying,
    SpinAttack,
    Sleeping,
    Dying,
}
impl PlayerPose {
    pub fn eye_height(self) -> f32 {
        match self {
            Self::Standing | Self::Dying => 1.62,
            Self::Crouching => 1.27,
            Self::Swimming | Self::FallFlying | Self::SpinAttack => 0.4,
            Self::Sleeping => 0.2,
        }
    }
    pub fn bb_height(self) -> f64 {
        match self {
            Self::Standing => PLAYER_STANDING_BB_HEIGHT,
            Self::Crouching => PLAYER_CROUCHING_BB_HEIGHT,
            Self::Swimming | Self::FallFlying | Self::SpinAttack => PLAYER_SWIMMING_BB_HEIGHT,
            Self::Sleeping | Self::Dying => 0.2_f32 as f64,
        }
    }
    pub fn bb_width(self) -> f64 {
        match self {
            Self::Sleeping | Self::Dying => 0.2_f32 as f64,
            _ => PLAYER_BB_WIDTH,
        }
    }
}
pub fn diagnostic_probe_value_enabled(value: Option<&str>) -> bool {
    value
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}
pub fn diagnostic_probe_env_enabled(name: &str) -> bool {
    diagnostic_probe_value_enabled(std::env::var(name).ok().as_deref())
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct MoveCollisionResult {
    pub(crate) position: DVec3,
    pub(crate) velocity: DVec3,
    pub(crate) attempted_movement: DVec3,
    pub(crate) movement: DVec3,
    pub(crate) x_collision: bool,
    pub(crate) z_collision: bool,
    pub(crate) horizontal_collision: bool,
    pub(crate) vertical_collision: bool,
    pub(crate) vertical_collision_below: bool,
}
pub fn entity_applies_collision_movement_like_java(delta: DVec3, movement: DVec3) -> bool {
    let movement_length_squared = movement.length_squared();
    movement_length_squared > 1.0E-7 || delta.length_squared() - movement_length_squared < 1.0E-7
}
