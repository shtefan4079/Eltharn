#![allow(dead_code)]
use crate::gameplay::player::LocalPlayer;
use crate::render::entity::state::{
    ArmPose, HumanoidArm, HumanoidRenderState, InteractionHand, SwingAnimationType,
};
use crate::render::model::player::player_model::{self, HandSide, PlayerModelParts};
use std::f32::consts::{FRAC_PI_2, PI, TAU};
pub const OVERLAY_SCALE: f32 = 0.25;
pub const HAT_OVERLAY_SCALE: f32 = 0.5;
pub const LEGGINGS_OVERLAY_SCALE: f32 = -0.1;
const DUCK_WALK_ROTATION: f32 = 0.005;
const SPYGLASS_ARM_ROT_Y: f32 = PI / 12.0;
const SPYGLASS_ARM_ROT_X: f32 = 1.919_862_2;
const SPYGLASS_ARM_CROUCH_ROT_X: f32 = PI / 12.0;
const HIGHEST_SHIELD_BLOCKING_ANGLE: f32 = -PI * 4.0 / 9.0;
const LOWEST_SHIELD_BLOCKING_ANGLE: f32 = 0.436_332_32;
const HORIZONTAL_SHIELD_MOVEMENT_LIMIT: f32 = PI / 6.0;
pub const TOOT_HORN_XROT_BASE: f32 = 1.483_529_8;
pub const TOOT_HORN_YROT_BASE: f32 = PI / 6.0;
pub struct HumanoidModel;
impl HumanoidModel {
    pub fn setup_anim(player: &LocalPlayer) -> PlayerModelParts {
        Self::setup_anim_state(&HumanoidRenderState::from_player(player))
    }
    pub fn setup_anim_with_walk(
        player: &LocalPlayer,
        animation_pos: f32,
        animation_speed: f32,
    ) -> PlayerModelParts {
        let mut state = HumanoidRenderState::from_player(player);
        state.walk_animation_pos = animation_pos;
        state.walk_animation_speed = animation_speed;
        Self::setup_anim_state(&state)
    }
    pub fn setup_anim_state(state: &HumanoidRenderState) -> PlayerModelParts {
        let mut parts = PlayerModelParts::default();
        let left_arm_pose = state.left_arm_pose;
        let right_arm_pose = state.right_arm_pose;
        let swim_amount = state.swim_amount;
        parts.head_pitch = state.x_rot.to_radians();
        parts.head_yaw = state.y_rot.to_radians();
        if state.is_fall_flying {
            parts.head_pitch = -PI / 4.0;
        } else if swim_amount > 0.0 {
            parts.head_pitch = rot_lerp_rad(swim_amount, parts.head_pitch, -PI / 4.0);
        }
        let animation_pos = state.walk_animation_pos;
        let animation_speed = state.walk_animation_speed;
        parts.right_arm_pitch =
            (animation_pos * 0.6662 + PI).cos() * 2.0 * animation_speed * 0.5 / state.speed_value;
        parts.left_arm_pitch =
            (animation_pos * 0.6662).cos() * 2.0 * animation_speed * 0.5 / state.speed_value;
        parts.right_leg_pitch =
            (animation_pos * 0.6662).cos() * 1.4 * animation_speed / state.speed_value;
        parts.left_leg_pitch =
            (animation_pos * 0.6662 + PI).cos() * 1.4 * animation_speed / state.speed_value;
        parts.right_leg_yaw = DUCK_WALK_ROTATION;
        parts.left_leg_yaw = -DUCK_WALK_ROTATION;
        parts.right_leg_roll = DUCK_WALK_ROTATION;
        parts.left_leg_roll = -DUCK_WALK_ROTATION;
        if state.is_passenger {
            parts.right_arm_pitch += -PI / 5.0;
            parts.left_arm_pitch += -PI / 5.0;
            parts.right_leg_pitch = -1.413_716_7;
            parts.right_leg_yaw = PI / 10.0;
            parts.right_leg_roll = 0.078_539_82;
            parts.left_leg_pitch = -1.413_716_7;
            parts.left_leg_yaw = -PI / 10.0;
            parts.left_leg_roll = -0.078_539_82;
        }
        let right_handed = state.main_arm == HumanoidArm::Right;
        if state.is_using_item {
            let main_hand_used = state.use_item_hand == InteractionHand::MainHand;
            if main_hand_used == right_handed {
                pose_right_arm(&mut parts, state);
                if !state.right_arm_pose.affects_offhand_pose() {
                    pose_left_arm(&mut parts, state);
                }
            } else {
                pose_left_arm(&mut parts, state);
                if !state.left_arm_pose.affects_offhand_pose() {
                    pose_right_arm(&mut parts, state);
                }
            }
        } else {
            let two_handed_offhand = if right_handed {
                left_arm_pose.is_two_handed()
            } else {
                right_arm_pose.is_two_handed()
            };
            if right_handed != two_handed_offhand {
                pose_left_arm(&mut parts, state);
                if !state.left_arm_pose.affects_offhand_pose() {
                    pose_right_arm(&mut parts, state);
                }
            } else {
                pose_right_arm(&mut parts, state);
                if !state.right_arm_pose.affects_offhand_pose() {
                    pose_left_arm(&mut parts, state);
                }
            }
        }
        setup_attack_animation(&mut parts, state);
        if state.is_crouching {
            parts.body_pitch = 0.5;
            parts.right_arm_pitch += 0.4;
            parts.left_arm_pitch += 0.4;
            parts.right_leg_z_offset += 4.0;
            parts.left_leg_z_offset += 4.0;
            parts.head_y_offset += 4.2;
            parts.body_y_offset += 3.2;
            parts.left_arm_y_offset += 3.2;
            parts.right_arm_y_offset += 3.2;
        }
        if right_arm_pose != ArmPose::Spyglass {
            bob_model_part(
                &mut parts.right_arm_pitch,
                &mut parts.right_arm_roll,
                state.age_in_ticks,
                1.0,
            );
        }
        if left_arm_pose != ArmPose::Spyglass {
            bob_model_part(
                &mut parts.left_arm_pitch,
                &mut parts.left_arm_roll,
                state.age_in_ticks,
                -1.0,
            );
        }
        if swim_amount > 0.0 {
            setup_swim_animation(&mut parts, state, animation_pos, swim_amount);
        }
        parts
    }
    pub fn pose_item_arm(parts: &mut PlayerModelParts, hand: HandSide) {
        player_model::apply_item_arm_pose(parts, hand);
    }
}
fn pose_right_arm(parts: &mut PlayerModelParts, state: &HumanoidRenderState) {
    match state.right_arm_pose {
        ArmPose::Empty => parts.right_arm_yaw = 0.0,
        ArmPose::Item => {
            parts.right_arm_pitch = parts.right_arm_pitch * 0.5 - PI / 10.0;
            parts.right_arm_yaw = 0.0;
        }
        ArmPose::Block => pose_blocking_arm(parts, HumanoidArm::Right),
        ArmPose::BowAndArrow => {
            parts.right_arm_yaw = -0.1 + parts.head_yaw;
            parts.left_arm_yaw = 0.1 + parts.head_yaw + 0.4;
            parts.right_arm_pitch = -FRAC_PI_2 + parts.head_pitch;
            parts.left_arm_pitch = -FRAC_PI_2 + parts.head_pitch;
        }
        ArmPose::ThrowTrident => {
            parts.right_arm_pitch = parts.right_arm_pitch * 0.5 - PI;
            parts.right_arm_yaw = 0.0;
        }
        ArmPose::CrossbowCharge => animate_crossbow_charge(parts, state, true),
        ArmPose::CrossbowHold => animate_crossbow_hold(parts, true),
        ArmPose::Spyglass => {
            parts.right_arm_pitch = (parts.head_pitch
                - SPYGLASS_ARM_ROT_X
                - if state.is_crouching {
                    SPYGLASS_ARM_CROUCH_ROT_X
                } else {
                    0.0
                })
            .clamp(-2.4, 3.3);
            parts.right_arm_yaw = parts.head_yaw - SPYGLASS_ARM_ROT_Y;
        }
        ArmPose::TootHorn => {
            parts.right_arm_pitch = parts.head_pitch.clamp(-1.2, 1.2) - TOOT_HORN_XROT_BASE;
            parts.right_arm_yaw = parts.head_yaw - TOOT_HORN_YROT_BASE;
        }
        ArmPose::Brush => {
            parts.right_arm_pitch = parts.right_arm_pitch * 0.5 - PI / 5.0;
            parts.right_arm_yaw = 0.0;
        }
        ArmPose::Spear => pose_spear_arm(parts, state, HumanoidArm::Right),
    }
}
fn pose_left_arm(parts: &mut PlayerModelParts, state: &HumanoidRenderState) {
    match state.left_arm_pose {
        ArmPose::Empty => parts.left_arm_yaw = 0.0,
        ArmPose::Item => {
            parts.left_arm_pitch = parts.left_arm_pitch * 0.5 - PI / 10.0;
            parts.left_arm_yaw = 0.0;
        }
        ArmPose::Block => pose_blocking_arm(parts, HumanoidArm::Left),
        ArmPose::BowAndArrow => {
            parts.right_arm_yaw = -0.1 + parts.head_yaw - 0.4;
            parts.left_arm_yaw = 0.1 + parts.head_yaw;
            parts.right_arm_pitch = -FRAC_PI_2 + parts.head_pitch;
            parts.left_arm_pitch = -FRAC_PI_2 + parts.head_pitch;
        }
        ArmPose::ThrowTrident => {
            parts.left_arm_pitch = parts.left_arm_pitch * 0.5 - PI;
            parts.left_arm_yaw = 0.0;
        }
        ArmPose::CrossbowCharge => animate_crossbow_charge(parts, state, false),
        ArmPose::CrossbowHold => animate_crossbow_hold(parts, false),
        ArmPose::Spyglass => {
            parts.left_arm_pitch = (parts.head_pitch
                - SPYGLASS_ARM_ROT_X
                - if state.is_crouching {
                    SPYGLASS_ARM_CROUCH_ROT_X
                } else {
                    0.0
                })
            .clamp(-2.4, 3.3);
            parts.left_arm_yaw = parts.head_yaw + SPYGLASS_ARM_ROT_Y;
        }
        ArmPose::TootHorn => {
            parts.left_arm_pitch = parts.head_pitch.clamp(-1.2, 1.2) - TOOT_HORN_XROT_BASE;
            parts.left_arm_yaw = parts.head_yaw + TOOT_HORN_YROT_BASE;
        }
        ArmPose::Brush => {
            parts.left_arm_pitch = parts.left_arm_pitch * 0.5 - PI / 5.0;
            parts.left_arm_yaw = 0.0;
        }
        ArmPose::Spear => pose_spear_arm(parts, state, HumanoidArm::Left),
    }
}
fn pose_blocking_arm(parts: &mut PlayerModelParts, arm: HumanoidArm) {
    let pitch = match arm {
        HumanoidArm::Right => &mut parts.right_arm_pitch,
        HumanoidArm::Left => &mut parts.left_arm_pitch,
    };
    *pitch = *pitch * 0.5 - 0.942_477_9
        + parts
            .head_pitch
            .clamp(HIGHEST_SHIELD_BLOCKING_ANGLE, LOWEST_SHIELD_BLOCKING_ANGLE);
    let side = if arm == HumanoidArm::Right { -1.0 } else { 1.0 };
    let yaw = side * 30.0_f32.to_radians()
        + parts.head_yaw.clamp(
            -HORIZONTAL_SHIELD_MOVEMENT_LIMIT,
            HORIZONTAL_SHIELD_MOVEMENT_LIMIT,
        );
    match arm {
        HumanoidArm::Right => parts.right_arm_yaw = yaw,
        HumanoidArm::Left => parts.left_arm_yaw = yaw,
    }
}
fn animate_crossbow_hold(parts: &mut PlayerModelParts, right: bool) {
    if right {
        parts.right_arm_yaw = -0.3 + parts.head_yaw;
        parts.left_arm_yaw = 0.6 + parts.head_yaw;
        parts.right_arm_pitch = -FRAC_PI_2 + parts.head_pitch + 0.1;
        parts.left_arm_pitch = -1.5 + parts.head_pitch;
    } else {
        parts.left_arm_yaw = 0.3 + parts.head_yaw;
        parts.right_arm_yaw = -0.6 + parts.head_yaw;
        parts.left_arm_pitch = -FRAC_PI_2 + parts.head_pitch + 0.1;
        parts.right_arm_pitch = -1.5 + parts.head_pitch;
    }
}
fn animate_crossbow_charge(parts: &mut PlayerModelParts, state: &HumanoidRenderState, right: bool) {
    let duration = state.max_crossbow_charge_duration;
    let use_ticks = state.ticks_using_item.clamp(0.0, duration);
    let alpha = if duration == 0.0 {
        0.0
    } else {
        use_ticks / duration
    };
    if right {
        parts.right_arm_yaw = -0.8;
        parts.right_arm_pitch = -0.970_796_35;
        parts.left_arm_pitch = parts.right_arm_pitch;
        parts.left_arm_yaw = lerp(alpha, 0.4, 0.85);
        parts.left_arm_pitch = lerp(alpha, parts.left_arm_pitch, -FRAC_PI_2);
    } else {
        parts.left_arm_yaw = 0.8;
        parts.left_arm_pitch = -0.970_796_35;
        parts.right_arm_pitch = parts.left_arm_pitch;
        parts.right_arm_yaw = -lerp(alpha, 0.4, 0.85);
        parts.right_arm_pitch = lerp(alpha, parts.right_arm_pitch, -FRAC_PI_2);
    }
}
fn pose_spear_arm(parts: &mut PlayerModelParts, state: &HumanoidRenderState, arm: HumanoidArm) {
    let invert = if arm == HumanoidArm::Right { 1.0 } else { -1.0 };
    let mut yaw = -0.1 * invert + parts.head_yaw;
    let mut pitch = -FRAC_PI_2 + parts.head_pitch + 0.8;
    if state.is_fall_flying || state.swim_amount > 0.0 {
        pitch -= 0.959_931_1;
    }
    yaw = yaw.to_degrees().clamp(-60.0, 60.0).to_radians();
    pitch = pitch.to_degrees().clamp(-120.0, 30.0).to_radians();
    match arm {
        HumanoidArm::Right => {
            parts.right_arm_yaw = yaw;
            parts.right_arm_pitch = pitch;
        }
        HumanoidArm::Left => {
            parts.left_arm_yaw = yaw;
            parts.left_arm_pitch = pitch;
        }
    }
}
fn setup_attack_animation(parts: &mut PlayerModelParts, state: &HumanoidRenderState) {
    let attack_time = state.attack_time;
    if attack_time <= 0.0 {
        return;
    }
    parts.body_yaw = (attack_time.sqrt() * TAU).sin() * 0.2;
    if state.attack_arm == HumanoidArm::Left {
        parts.body_yaw *= -1.0;
    }
    let age_scale = state.age_scale;
    parts.right_arm_z_offset = parts.body_yaw.sin() * 5.0 * age_scale;
    parts.right_arm_x_offset = -parts.body_yaw.cos() * 5.0 * age_scale + 5.0;
    parts.left_arm_z_offset = -parts.body_yaw.sin() * 5.0 * age_scale;
    parts.left_arm_x_offset = parts.body_yaw.cos() * 5.0 * age_scale - 5.0;
    parts.right_arm_yaw += parts.body_yaw;
    parts.left_arm_yaw += parts.body_yaw;
    parts.left_arm_pitch += parts.body_yaw;
    match state.swing_animation_type {
        SwingAnimationType::Whack => {
            let swing = out_quart(attack_time);
            let aa = (swing * PI).sin();
            let bb = (attack_time * PI).sin() * -(parts.head_pitch - 0.7) * 0.75;
            let body_yaw = parts.body_yaw;
            if state.attack_arm == HumanoidArm::Right {
                parts.right_arm_pitch -= aa * 1.2 + bb;
                parts.right_arm_yaw += body_yaw * 2.0;
                parts.right_arm_roll += (attack_time * PI).sin() * -0.4;
            } else {
                parts.left_arm_pitch -= aa * 1.2 + bb;
                parts.left_arm_yaw += body_yaw * 2.0;
                parts.left_arm_roll += (attack_time * PI).sin() * -0.4;
            }
        }
        SwingAnimationType::Stab => setup_spear_attack_animation(parts, state),
        SwingAnimationType::None => {}
    }
}
fn setup_spear_attack_animation(parts: &mut PlayerModelParts, state: &HumanoidRenderState) {
    parts.right_arm_yaw -= parts.body_yaw;
    parts.left_arm_yaw -= parts.body_yaw;
    parts.left_arm_pitch -= parts.body_yaw;
    let prepare = in_out_sine(progress(state.attack_time, 0.0, 0.05));
    let attack = in_quad(progress(state.attack_time, 0.05, 0.2));
    let retract = in_out_expo(progress(state.attack_time, 0.4, 1.0));
    let offset = (90.0 * prepare - 120.0 * attack + 30.0 * retract).to_radians();
    match state.attack_arm {
        HumanoidArm::Right => parts.right_arm_pitch += offset,
        HumanoidArm::Left => parts.left_arm_pitch += offset,
    }
}
fn setup_swim_animation(
    parts: &mut PlayerModelParts,
    state: &HumanoidRenderState,
    animation_pos: f32,
    swim_amount: f32,
) {
    let swim_pos = animation_pos % 26.0;
    let right_amount = if state.right_arm_pose != ArmPose::Spear
        && (state.attack_arm != HumanoidArm::Right || state.attack_time <= 0.0)
    {
        swim_amount
    } else {
        0.0
    };
    let left_amount = if state.left_arm_pose != ArmPose::Spear
        && (state.attack_arm != HumanoidArm::Left || state.attack_time <= 0.0)
    {
        swim_amount
    } else {
        0.0
    };
    if !state.is_using_item {
        if swim_pos < 14.0 {
            let curve = quadratic_arm_update(swim_pos) / quadratic_arm_update(14.0);
            parts.left_arm_pitch = rot_lerp_rad(left_amount, parts.left_arm_pitch, 0.0);
            parts.right_arm_pitch = lerp(right_amount, parts.right_arm_pitch, 0.0);
            parts.left_arm_yaw = rot_lerp_rad(left_amount, parts.left_arm_yaw, PI);
            parts.right_arm_yaw = lerp(right_amount, parts.right_arm_yaw, PI);
            parts.left_arm_roll =
                rot_lerp_rad(left_amount, parts.left_arm_roll, PI + 1.870_796_4 * curve);
            parts.right_arm_roll =
                lerp(right_amount, parts.right_arm_roll, PI - 1.870_796_4 * curve);
        } else if swim_pos < 22.0 {
            let internal = (swim_pos - 14.0) / 8.0;
            parts.left_arm_pitch =
                rot_lerp_rad(left_amount, parts.left_arm_pitch, FRAC_PI_2 * internal);
            parts.right_arm_pitch = lerp(right_amount, parts.right_arm_pitch, FRAC_PI_2 * internal);
            parts.left_arm_yaw = rot_lerp_rad(left_amount, parts.left_arm_yaw, PI);
            parts.right_arm_yaw = lerp(right_amount, parts.right_arm_yaw, PI);
            parts.left_arm_roll = rot_lerp_rad(
                left_amount,
                parts.left_arm_roll,
                5.012_389 - 1.870_796_4 * internal,
            );
            parts.right_arm_roll = lerp(
                right_amount,
                parts.right_arm_roll,
                1.270_796_3 + 1.870_796_4 * internal,
            );
        } else if swim_pos < 26.0 {
            let internal = (swim_pos - 22.0) / 4.0;
            let pitch = FRAC_PI_2 - FRAC_PI_2 * internal;
            parts.left_arm_pitch = rot_lerp_rad(left_amount, parts.left_arm_pitch, pitch);
            parts.right_arm_pitch = lerp(right_amount, parts.right_arm_pitch, pitch);
            parts.left_arm_yaw = rot_lerp_rad(left_amount, parts.left_arm_yaw, PI);
            parts.right_arm_yaw = lerp(right_amount, parts.right_arm_yaw, PI);
            parts.left_arm_roll = rot_lerp_rad(left_amount, parts.left_arm_roll, PI);
            parts.right_arm_roll = lerp(right_amount, parts.right_arm_roll, PI);
        }
    }
    parts.left_leg_pitch = lerp(
        swim_amount,
        parts.left_leg_pitch,
        0.3 * (animation_pos * 0.333_333_34 + PI).cos(),
    );
    parts.right_leg_pitch = lerp(
        swim_amount,
        parts.right_leg_pitch,
        0.3 * (animation_pos * 0.333_333_34).cos(),
    );
}
fn bob_model_part(pitch: &mut f32, roll: &mut f32, age: f32, scale: f32) {
    *roll += scale * ((age * 0.09).cos() * 0.05 + 0.05);
    *pitch += scale * (age * 0.067).sin() * 0.05;
}
fn quadratic_arm_update(x: f32) -> f32 {
    -65.0 * x + x * x
}
fn lerp(alpha: f32, from: f32, to: f32) -> f32 {
    from + alpha * (to - from)
}
fn rot_lerp_rad(alpha: f32, from: f32, to: f32) -> f32 {
    from + alpha * wrap_radians(to - from)
}
fn wrap_radians(value: f32) -> f32 {
    (value + PI).rem_euclid(TAU) - PI
}
fn progress(value: f32, start: f32, end: f32) -> f32 {
    ((value - start) / (end - start)).clamp(0.0, 1.0)
}
fn out_quart(value: f32) -> f32 {
    1.0 - (1.0 - value).powi(4)
}
fn in_quad(value: f32) -> f32 {
    value * value
}
fn in_out_sine(value: f32) -> f32 {
    -((PI * value).cos() - 1.0) / 2.0
}
fn in_out_expo(value: f32) -> f32 {
    if value == 0.0 || value == 1.0 {
        value
    } else if value < 0.5 {
        2.0_f32.powf(20.0 * value - 10.0) / 2.0
    } else {
        (2.0 - 2.0_f32.powf(-20.0 * value + 10.0)) / 2.0
    }
}
