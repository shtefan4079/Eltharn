#![allow(dead_code)]
use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;
pub fn build_skeleton_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 0.0, 0.0))
                    .cube(cube([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]))
                    .child(NamedPartSpec::new("hat", PartPose::ZERO).cube(cube_grown(
                        [32.0, 0.0],
                        [-4.0, -8.0, -4.0],
                        [8.0, 8.0, 8.0],
                        0.5,
                    ))),
            )
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, 0.0, 0.0)).cube(cube(
                    [16.0, 16.0],
                    [-4.0, 0.0, -2.0],
                    [8.0, 12.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_arm", PartPose::offset(-5.0, 2.0, 0.0)).cube(cube(
                    [40.0, 16.0],
                    [-1.0, -2.0, -1.0],
                    [2.0, 12.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_arm", PartPose::offset(5.0, 2.0, 0.0)).cube(
                    cube_mirrored([40.0, 16.0], [-1.0, -2.0, -1.0], [2.0, 12.0, 2.0]),
                ),
            )
            .child(
                NamedPartSpec::new("right_leg", PartPose::offset(-2.0, 12.0, 0.0)).cube(cube(
                    [0.0, 16.0],
                    [-1.0, 0.0, -1.0],
                    [2.0, 12.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_leg", PartPose::offset(2.0, 12.0, 0.0)).cube(
                    cube_mirrored([0.0, 16.0], [-1.0, 0.0, -1.0], [2.0, 12.0, 2.0]),
                ),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_skeleton_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_skeleton_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_wither_skeleton_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 0.0, 0.0))
                    .cube(cube([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]))
                    .child(NamedPartSpec::new("hat", PartPose::ZERO).cube(cube_grown(
                        [32.0, 0.0],
                        [-4.0, -8.0, -4.0],
                        [8.0, 8.0, 8.0],
                        0.5,
                    ))),
            )
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, 0.0, 0.0)).cube(cube(
                    [16.0, 16.0],
                    [-4.0, 0.0, -2.0],
                    [8.0, 12.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_arm", PartPose::offset(-5.0, 2.0, 0.0)).cube(cube(
                    [40.0, 16.0],
                    [-1.0, -2.0, -1.0],
                    [2.0, 12.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_arm", PartPose::offset(5.0, 2.0, 0.0)).cube(
                    cube_mirrored([40.0, 16.0], [-1.0, -2.0, -1.0], [2.0, 12.0, 2.0]),
                ),
            )
            .child(
                NamedPartSpec::new("right_leg", PartPose::offset(-2.0, 12.0, 0.0)).cube(cube(
                    [0.0, 16.0],
                    [-1.0, 0.0, -1.0],
                    [2.0, 12.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_leg", PartPose::offset(2.0, 12.0, 0.0)).cube(
                    cube_mirrored([0.0, 16.0], [-1.0, 0.0, -1.0], [2.0, 12.0, 2.0]),
                ),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_wither_skeleton_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_wither_skeleton_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_bogged_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 0.0, 0.0))
                    .cube(cube([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]))
                    .cube(cube([0.0, 32.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]))
                    .child(NamedPartSpec::new("hat", PartPose::ZERO).cube(cube_grown(
                        [32.0, 0.0],
                        [-4.0, -8.0, -4.0],
                        [8.0, 8.0, 8.0],
                        0.5,
                    ))),
            )
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, 0.0, 0.0)).cube(cube(
                    [16.0, 16.0],
                    [-4.0, 0.0, -2.0],
                    [8.0, 12.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_arm", PartPose::offset(-5.0, 2.0, 0.0)).cube(cube(
                    [40.0, 16.0],
                    [-1.0, -2.0, -1.0],
                    [2.0, 12.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_arm", PartPose::offset(5.0, 2.0, 0.0)).cube(
                    cube_mirrored([40.0, 16.0], [-1.0, -2.0, -1.0], [2.0, 12.0, 2.0]),
                ),
            )
            .child(
                NamedPartSpec::new("right_leg", PartPose::offset(-2.0, 12.0, 0.0)).cube(cube(
                    [0.0, 16.0],
                    [-1.0, 0.0, -1.0],
                    [2.0, 12.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_leg", PartPose::offset(2.0, 12.0, 0.0)).cube(
                    cube_mirrored([0.0, 16.0], [-1.0, 0.0, -1.0], [2.0, 12.0, 2.0]),
                ),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_bogged_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_bogged_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
fn bob_model_part(tree: &mut JavaModelPartTree, part_name: &str, age_in_ticks: f32, scale: f32) {
    if let Some(part) = tree.part_mut(part_name) {
        part.pose.rotation.z += scale * ((age_in_ticks * 0.09).cos() * 0.05 + 0.05);
        part.pose.rotation.x += scale * ((age_in_ticks * 0.067).sin() * 0.05);
    }
}
pub(super) fn bob_arms(tree: &mut JavaModelPartTree, age_in_ticks: f32) {
    bob_model_part(tree, "right_arm", age_in_ticks, 1.0);
    bob_model_part(tree, "left_arm", age_in_ticks, -1.0);
}
fn rot_lerp_rad(a: f32, from: f32, to: f32) -> f32 {
    let diff = (to - from).rem_euclid(std::f32::consts::TAU);
    let short = if diff > std::f32::consts::PI {
        diff - std::f32::consts::TAU
    } else {
        diff
    };
    from + short * a
}
fn pose_right_arm_item(tree: &mut JavaModelPartTree) {
    if let Some(ra) = tree.part_mut("right_arm") {
        ra.pose.rotation.x = ra.pose.rotation.x * 0.5 - std::f32::consts::PI / 10.0;
        ra.pose.rotation.y = 0.0;
    }
}
fn pose_left_arm_item(tree: &mut JavaModelPartTree) {
    if let Some(la) = tree.part_mut("left_arm") {
        la.pose.rotation.x = la.pose.rotation.x * 0.5 - std::f32::consts::PI / 10.0;
        la.pose.rotation.y = 0.0;
    }
}
fn pose_blocking_arm(
    tree: &mut JavaModelPartTree,
    arm_name: &str,
    head_x_rot: f32,
    head_y_rot: f32,
    right: bool,
) {
    if let Some(arm) = tree.part_mut(arm_name) {
        arm.pose.rotation.x = arm.pose.rotation.x * 0.5 - 0.9424779
            + head_x_rot.clamp(-std::f32::consts::PI * 4.0 / 9.0, 0.43633232);
        let y_sign = if right { -30.0 } else { 30.0 };
        arm.pose.rotation.y = y_sign * std::f32::consts::PI / 180.0
            + head_y_rot.clamp(-std::f32::consts::PI / 6.0, std::f32::consts::PI / 6.0);
    }
}
fn pose_bow_and_arrow(tree: &mut JavaModelPartTree, head_x_rot: f32, head_y_rot: f32) {
    if let Some(ra) = tree.part_mut("right_arm") {
        ra.pose.rotation.y = -0.1 + head_y_rot;
        ra.pose.rotation.x = -std::f32::consts::PI / 2.0 + head_x_rot;
    }
    if let Some(la) = tree.part_mut("left_arm") {
        la.pose.rotation.y = 0.1 + head_y_rot + 0.4;
        la.pose.rotation.x = -std::f32::consts::PI / 2.0 + head_x_rot;
    }
}
pub(super) fn pose_crossbow_hold(
    tree: &mut JavaModelPartTree,
    head_x_rot: f32,
    head_y_rot: f32,
    holding_in_right: bool,
) {
    let (holding_arm, shooting_arm) = if holding_in_right {
        ("right_arm", "left_arm")
    } else {
        ("left_arm", "right_arm")
    };
    let sign = if holding_in_right { -0.3 } else { 0.3 };
    if let Some(ha) = tree.part_mut(holding_arm) {
        ha.pose.rotation.y = sign + head_y_rot;
        ha.pose.rotation.x = -std::f32::consts::PI / 2.0 + head_x_rot + 0.1;
    }
    let sign2 = if holding_in_right { 0.6 } else { -0.6 };
    if let Some(sa) = tree.part_mut(shooting_arm) {
        sa.pose.rotation.y = sign2 + head_y_rot;
        sa.pose.rotation.x = -1.5 + head_x_rot;
    }
}
pub(super) fn pose_crossbow_charge(
    tree: &mut JavaModelPartTree,
    max_charge: f32,
    ticks_using: f32,
    holding_in_right: bool,
) {
    let (holding_arm, pulling_arm) = if holding_in_right {
        ("right_arm", "left_arm")
    } else {
        ("left_arm", "right_arm")
    };
    let y_sign = if holding_in_right { -0.8 } else { 0.8 };
    if let Some(ha) = tree.part_mut(holding_arm) {
        ha.pose.rotation.y = y_sign;
        ha.pose.rotation.x = -0.97079635;
    }
    let use_ticks = ticks_using.clamp(0.0, max_charge);
    let lerp_alpha = if max_charge > 0.0 {
        use_ticks / max_charge
    } else {
        0.0
    };
    let pull_y = (0.4 + (0.85 - 0.4) * lerp_alpha) * if holding_in_right { 1.0 } else { -1.0 };
    if let Some(pa) = tree.part_mut(pulling_arm) {
        pa.pose.rotation.x = -0.97079635;
        pa.pose.rotation.y = pull_y;
        pa.pose.rotation.x =
            pa.pose.rotation.x + (-std::f32::consts::PI / 2.0 - pa.pose.rotation.x) * lerp_alpha;
    }
}
fn pose_spyglass(
    tree: &mut JavaModelPartTree,
    arm_name: &str,
    head_x_rot: f32,
    head_y_rot: f32,
    is_crouching: bool,
    right: bool,
) {
    if let Some(arm) = tree.part_mut(arm_name) {
        let crouch_offset = if is_crouching {
            std::f32::consts::PI / 12.0
        } else {
            0.0
        };
        arm.pose.rotation.x = (head_x_rot - 1.9198622 - crouch_offset).clamp(-2.4, 3.3);
        let y_offset = if right {
            -std::f32::consts::PI / 12.0
        } else {
            std::f32::consts::PI / 12.0
        };
        arm.pose.rotation.y = head_y_rot + y_offset;
    }
}
fn pose_throw_trident(tree: &mut JavaModelPartTree, arm_name: &str) {
    if let Some(arm) = tree.part_mut(arm_name) {
        arm.pose.rotation.x = arm.pose.rotation.x * 0.5 - std::f32::consts::PI;
        arm.pose.rotation.y = 0.0;
    }
}
fn pose_toot_horn(
    tree: &mut JavaModelPartTree,
    arm_name: &str,
    head_x_rot: f32,
    head_y_rot: f32,
    right: bool,
) {
    if let Some(arm) = tree.part_mut(arm_name) {
        arm.pose.rotation.x = head_x_rot.clamp(-1.2, 1.2) - 1.4835298;
        let y_offset = if right {
            -std::f32::consts::PI / 6.0
        } else {
            std::f32::consts::PI / 6.0
        };
        arm.pose.rotation.y = head_y_rot + y_offset;
    }
}
fn pose_brush(tree: &mut JavaModelPartTree, arm_name: &str) {
    if let Some(arm) = tree.part_mut(arm_name) {
        arm.pose.rotation.x = arm.pose.rotation.x * 0.5 - std::f32::consts::PI / 5.0;
        arm.pose.rotation.y = 0.0;
    }
}
fn setup_attack_animation(tree: &mut JavaModelPartTree, state: &EntityModelAnimationState) {
    let attack_time = state.attack_time;
    if attack_time <= 0.0 {
        return;
    }
    let pi = std::f32::consts::PI;
    let body_y_rot = (attack_time.sqrt() * pi * 2.0).sin() * 0.2;
    let body_y_rot = if state.attack_arm == HumanoidArm::Left {
        -body_y_rot
    } else {
        body_y_rot
    };
    if let Some(b) = tree.part_mut("body") {
        b.pose.rotation.y = body_y_rot;
    }
    let age_scale = state.age_scale;
    if let Some(ra) = tree.part_mut("right_arm") {
        ra.pose.offset.z = body_y_rot.sin() * 5.0 * age_scale;
        ra.pose.offset.x = -body_y_rot.cos() * 5.0 * age_scale;
        ra.pose.rotation.y += body_y_rot;
    }
    if let Some(la) = tree.part_mut("left_arm") {
        la.pose.offset.z = -body_y_rot.sin() * 5.0 * age_scale;
        la.pose.offset.x = body_y_rot.cos() * 5.0 * age_scale;
        la.pose.rotation.y += body_y_rot;
        la.pose.rotation.x += body_y_rot;
    }
    match state.swing_animation_type {
        SwingAnimationType::Whack => {
            let swing = ease_out_quart(attack_time);
            let aa = (swing * pi).sin();
            let head_x_rot = state.x_rot_degrees * pi / 180.0;
            let bb = (attack_time * pi).sin() * -(head_x_rot - 0.7) * 0.75;
            let attack_arm_name = match state.attack_arm {
                HumanoidArm::Right => "right_arm",
                HumanoidArm::Left => "left_arm",
            };
            if let Some(arm) = tree.part_mut(attack_arm_name) {
                arm.pose.rotation.x -= aa * 1.2 + bb;
                arm.pose.rotation.y += body_y_rot * 2.0;
                arm.pose.rotation.z += (attack_time * pi).sin() * -0.4;
            }
        }
        SwingAnimationType::Stab => {
            if let Some(ra) = tree.part_mut("right_arm") {
                ra.pose.rotation.y -= body_y_rot;
            }
            if let Some(la) = tree.part_mut("left_arm") {
                la.pose.rotation.y -= body_y_rot;
                la.pose.rotation.x -= body_y_rot;
            }
            let prepare = ease_in_out_sine(progress(attack_time, 0.0, 0.05));
            let attack = ease_in_quad(progress(attack_time, 0.05, 0.2));
            let retract = ease_in_out_expo(progress(attack_time, 0.4, 1.0));
            let attack_arm_name = match state.attack_arm {
                HumanoidArm::Right => "right_arm",
                HumanoidArm::Left => "left_arm",
            };
            if let Some(arm) = tree.part_mut(attack_arm_name) {
                arm.pose.rotation.x +=
                    (90.0 * prepare - 120.0 * attack + 30.0 * retract) * pi / 180.0;
            }
        }
        SwingAnimationType::None => {}
    }
}
fn progress(time: f32, start: f32, end: f32) -> f32 {
    if end <= start {
        return 0.0;
    }
    ((time - start) / (end - start)).clamp(0.0, 1.0)
}
fn ease_out_quart(x: f32) -> f32 {
    1.0 - (1.0 - x).powi(4)
}
fn ease_in_out_sine(x: f32) -> f32 {
    -(std::f32::consts::PI * x).cos() / 2.0 + 0.5
}
fn ease_in_quad(x: f32) -> f32 {
    x * x
}
fn ease_in_out_expo(x: f32) -> f32 {
    if x < 0.5 {
        if x == 0.0 {
            0.0
        } else {
            (2.0_f32.powf(20.0 * x - 10.0)) / 2.0
        }
    } else if x == 1.0 {
        1.0
    } else {
        (2.0 - 2.0_f32.powf(-20.0 * x + 10.0)) / 2.0
    }
}
pub fn apply_humanoid_base_anim(tree: &mut JavaModelPartTree, state: &EntityModelAnimationState) {
    let pi = std::f32::consts::PI;
    let deg2rad = pi / 180.0;
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;
    let speed_val = if state.speed_value > 0.0 {
        state.speed_value
    } else {
        1.0
    };
    let swim_amount = state.swim_amount;
    let fall_flying = state.is_fall_flying;
    let head_x_rot = state.x_rot_degrees * deg2rad;
    let head_y_rot = state.y_rot_degrees * deg2rad;
    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = head_x_rot;
        h.pose.rotation.y = head_y_rot;
        if fall_flying {
            h.pose.rotation.x = -pi / 4.0;
        } else if swim_amount > 0.0 {
            h.pose.rotation.x = rot_lerp_rad(swim_amount, h.pose.rotation.x, -pi / 4.0);
        }
    }
    if let Some(ra) = tree.part_mut("right_arm") {
        ra.pose.rotation.x = (anim_pos * 0.6662 + pi).cos() * 2.0 * anim_speed * 0.5 / speed_val;
    }
    if let Some(la) = tree.part_mut("left_arm") {
        la.pose.rotation.x = (anim_pos * 0.6662).cos() * 2.0 * anim_speed * 0.5 / speed_val;
    }
    if let Some(rl) = tree.part_mut("right_leg") {
        rl.pose.rotation.x = (anim_pos * 0.6662).cos() * 1.4 * anim_speed / speed_val;
        rl.pose.rotation.y = 0.005;
        rl.pose.rotation.z = 0.005;
    }
    if let Some(ll) = tree.part_mut("left_leg") {
        ll.pose.rotation.x = (anim_pos * 0.6662 + pi).cos() * 1.4 * anim_speed / speed_val;
        ll.pose.rotation.y = -0.005;
        ll.pose.rotation.z = -0.005;
    }
    if state.is_passenger {
        if let Some(ra) = tree.part_mut("right_arm") {
            ra.pose.rotation.x += -pi / 5.0;
        }
        if let Some(la) = tree.part_mut("left_arm") {
            la.pose.rotation.x += -pi / 5.0;
        }
        if let Some(rl) = tree.part_mut("right_leg") {
            rl.pose.rotation.x = -1.4137167;
            rl.pose.rotation.y = pi / 10.0;
            rl.pose.rotation.z = 0.07853982;
        }
        if let Some(ll) = tree.part_mut("left_leg") {
            ll.pose.rotation.x = -1.4137167;
            ll.pose.rotation.y = -pi / 10.0;
            ll.pose.rotation.z = -0.07853982;
        }
    }
    let right_handed = state.main_arm == HumanoidArm::Right;
    let right_arm_pose = state.right_arm_pose;
    let left_arm_pose = state.left_arm_pose;
    if state.is_using_item {
        let main_hand_used = state.use_item_hand == InteractionHand::MainHand;
        if main_hand_used == right_handed {
            pose_right_arm_by_pose(tree, right_arm_pose, head_x_rot, head_y_rot, state);
            if !right_arm_pose.affects_offhand_pose() {
                pose_left_arm_by_pose(tree, left_arm_pose, head_x_rot, head_y_rot, state);
            }
        } else {
            pose_left_arm_by_pose(tree, left_arm_pose, head_x_rot, head_y_rot, state);
            if !left_arm_pose.affects_offhand_pose() {
                pose_right_arm_by_pose(tree, right_arm_pose, head_x_rot, head_y_rot, state);
            }
        }
    } else {
        let two_handed_offhand = if right_handed {
            left_arm_pose.is_two_handed()
        } else {
            right_arm_pose.is_two_handed()
        };
        if right_handed != two_handed_offhand {
            pose_left_arm_by_pose(tree, left_arm_pose, head_x_rot, head_y_rot, state);
            if !left_arm_pose.affects_offhand_pose() {
                pose_right_arm_by_pose(tree, right_arm_pose, head_x_rot, head_y_rot, state);
            }
        } else {
            pose_right_arm_by_pose(tree, right_arm_pose, head_x_rot, head_y_rot, state);
            if !right_arm_pose.affects_offhand_pose() {
                pose_left_arm_by_pose(tree, left_arm_pose, head_x_rot, head_y_rot, state);
            }
        }
    }
    setup_attack_animation(tree, state);
    if state.is_crouching {
        if let Some(b) = tree.part_mut("body") {
            b.pose.rotation.x = 0.5;
        }
        if let Some(ra) = tree.part_mut("right_arm") {
            ra.pose.rotation.x += 0.4;
        }
        if let Some(la) = tree.part_mut("left_arm") {
            la.pose.rotation.x += 0.4;
        }
        if let Some(rl) = tree.part_mut("right_leg") {
            rl.pose.offset.z += 4.0;
        }
        if let Some(ll) = tree.part_mut("left_leg") {
            ll.pose.offset.z += 4.0;
        }
        if let Some(h) = tree.part_mut("head") {
            h.pose.offset.y += 4.2;
        }
        if let Some(b) = tree.part_mut("body") {
            b.pose.offset.y += 3.2;
        }
        if let Some(la) = tree.part_mut("left_arm") {
            la.pose.offset.y += 3.2;
        }
        if let Some(ra) = tree.part_mut("right_arm") {
            ra.pose.offset.y += 3.2;
        }
    }
    if right_arm_pose != ArmPose::Spyglass {
        bob_model_part(tree, "right_arm", state.age_in_ticks, 1.0);
    }
    if left_arm_pose != ArmPose::Spyglass {
        bob_model_part(tree, "left_arm", state.age_in_ticks, -1.0);
    }
    if swim_amount > 0.0 {
        let swim_pos = anim_pos % 26.0;
        let attack_arm = state.attack_arm;
        let right_arm_swim = if right_arm_pose != ArmPose::Spear
            && (attack_arm != HumanoidArm::Right || !(state.attack_time > 0.0))
        {
            swim_amount
        } else {
            0.0
        };
        let left_arm_swim = if left_arm_pose != ArmPose::Spear
            && (attack_arm != HumanoidArm::Left || !(state.attack_time > 0.0))
        {
            swim_amount
        } else {
            0.0
        };
        if !state.is_using_item {
            if swim_pos < 14.0 {
                if let Some(la) = tree.part_mut("left_arm") {
                    la.pose.rotation.x = rot_lerp_rad(left_arm_swim, la.pose.rotation.x, 0.0);
                    la.pose.rotation.y = rot_lerp_rad(left_arm_swim, la.pose.rotation.y, pi);
                    la.pose.rotation.z = rot_lerp_rad(
                        left_arm_swim,
                        la.pose.rotation.z,
                        pi + 1.8707964 * quadratic_arm_update(swim_pos)
                            / quadratic_arm_update(14.0),
                    );
                }
                if let Some(ra) = tree.part_mut("right_arm") {
                    ra.pose.rotation.x =
                        ra.pose.rotation.x + (0.0 - ra.pose.rotation.x) * right_arm_swim;
                    ra.pose.rotation.y =
                        ra.pose.rotation.y + (pi - ra.pose.rotation.y) * right_arm_swim;
                    ra.pose.rotation.z = ra.pose.rotation.z
                        + (pi
                            - 1.8707964 * quadratic_arm_update(swim_pos)
                                / quadratic_arm_update(14.0)
                            - ra.pose.rotation.z)
                            * right_arm_swim;
                }
            } else if swim_pos >= 14.0 && swim_pos < 22.0 {
                let internal = (swim_pos - 14.0) / 8.0;
                if let Some(la) = tree.part_mut("left_arm") {
                    la.pose.rotation.x =
                        rot_lerp_rad(left_arm_swim, la.pose.rotation.x, pi / 2.0 * internal);
                    la.pose.rotation.y = rot_lerp_rad(left_arm_swim, la.pose.rotation.y, pi);
                    la.pose.rotation.z = rot_lerp_rad(
                        left_arm_swim,
                        la.pose.rotation.z,
                        5.012389 - 1.8707964 * internal,
                    );
                }
                if let Some(ra) = tree.part_mut("right_arm") {
                    ra.pose.rotation.x = ra.pose.rotation.x
                        + (pi / 2.0 * internal - ra.pose.rotation.x) * right_arm_swim;
                    ra.pose.rotation.y =
                        ra.pose.rotation.y + (pi - ra.pose.rotation.y) * right_arm_swim;
                    ra.pose.rotation.z = ra.pose.rotation.z
                        + (1.2707963 + 1.8707964 * internal - ra.pose.rotation.z) * right_arm_swim;
                }
            } else if swim_pos >= 22.0 && swim_pos < 26.0 {
                let internal = (swim_pos - 22.0) / 4.0;
                if let Some(la) = tree.part_mut("left_arm") {
                    la.pose.rotation.x = rot_lerp_rad(
                        left_arm_swim,
                        la.pose.rotation.x,
                        pi / 2.0 - pi / 2.0 * internal,
                    );
                    la.pose.rotation.y = rot_lerp_rad(left_arm_swim, la.pose.rotation.y, pi);
                    la.pose.rotation.z = rot_lerp_rad(left_arm_swim, la.pose.rotation.z, pi);
                }
                if let Some(ra) = tree.part_mut("right_arm") {
                    ra.pose.rotation.x = ra.pose.rotation.x
                        + (pi / 2.0 - pi / 2.0 * internal - ra.pose.rotation.x) * right_arm_swim;
                    ra.pose.rotation.y =
                        ra.pose.rotation.y + (pi - ra.pose.rotation.y) * right_arm_swim;
                    ra.pose.rotation.z =
                        ra.pose.rotation.z + (pi - ra.pose.rotation.z) * right_arm_swim;
                }
            }
        }
        if let Some(ll) = tree.part_mut("left_leg") {
            ll.pose.rotation.x = ll.pose.rotation.x
                + (0.3 * (anim_pos * 0.33333334 + pi).cos() - ll.pose.rotation.x) * swim_amount;
        }
        if let Some(rl) = tree.part_mut("right_leg") {
            rl.pose.rotation.x = rl.pose.rotation.x
                + (0.3 * (anim_pos * 0.33333334).cos() - rl.pose.rotation.x) * swim_amount;
        }
    }
}
fn quadratic_arm_update(x: f32) -> f32 {
    -65.0 * x + x * x
}
fn pose_right_arm_by_pose(
    tree: &mut JavaModelPartTree,
    pose: ArmPose,
    head_x_rot: f32,
    head_y_rot: f32,
    state: &EntityModelAnimationState,
) {
    match pose {
        ArmPose::Empty => {
            if let Some(ra) = tree.part_mut("right_arm") {
                ra.pose.rotation.y = 0.0;
            }
        }
        ArmPose::Item => pose_right_arm_item(tree),
        ArmPose::Block => pose_blocking_arm(tree, "right_arm", head_x_rot, head_y_rot, true),
        ArmPose::BowAndArrow => pose_bow_and_arrow(tree, head_x_rot, head_y_rot),
        ArmPose::ThrowTrident => pose_throw_trident(tree, "right_arm"),
        ArmPose::CrossbowCharge => pose_crossbow_charge(
            tree,
            state.max_crossbow_charge_duration,
            state.ticks_using_item,
            true,
        ),
        ArmPose::CrossbowHold => pose_crossbow_hold(tree, head_x_rot, head_y_rot, true),
        ArmPose::Spyglass => pose_spyglass(
            tree,
            "right_arm",
            head_x_rot,
            head_y_rot,
            state.is_crouching,
            true,
        ),
        ArmPose::TootHorn => pose_toot_horn(tree, "right_arm", head_x_rot, head_y_rot, true),
        ArmPose::Brush => pose_brush(tree, "right_arm"),
        ArmPose::Spear => {}
    }
}
fn pose_left_arm_by_pose(
    tree: &mut JavaModelPartTree,
    pose: ArmPose,
    head_x_rot: f32,
    head_y_rot: f32,
    state: &EntityModelAnimationState,
) {
    match pose {
        ArmPose::Empty => {
            if let Some(la) = tree.part_mut("left_arm") {
                la.pose.rotation.y = 0.0;
            }
        }
        ArmPose::Item => pose_left_arm_item(tree),
        ArmPose::Block => pose_blocking_arm(tree, "left_arm", head_x_rot, head_y_rot, false),
        ArmPose::BowAndArrow => {
            if let Some(ra) = tree.part_mut("right_arm") {
                ra.pose.rotation.y = -0.1 + head_y_rot - 0.4;
                ra.pose.rotation.x = -std::f32::consts::PI / 2.0 + head_x_rot;
            }
            if let Some(la) = tree.part_mut("left_arm") {
                la.pose.rotation.y = 0.1 + head_y_rot;
                la.pose.rotation.x = -std::f32::consts::PI / 2.0 + head_x_rot;
            }
        }
        ArmPose::ThrowTrident => pose_throw_trident(tree, "left_arm"),
        ArmPose::CrossbowCharge => pose_crossbow_charge(
            tree,
            state.max_crossbow_charge_duration,
            state.ticks_using_item,
            false,
        ),
        ArmPose::CrossbowHold => pose_crossbow_hold(tree, head_x_rot, head_y_rot, false),
        ArmPose::Spyglass => pose_spyglass(
            tree,
            "left_arm",
            head_x_rot,
            head_y_rot,
            state.is_crouching,
            false,
        ),
        ArmPose::TootHorn => pose_toot_horn(tree, "left_arm", head_x_rot, head_y_rot, false),
        ArmPose::Brush => pose_brush(tree, "left_arm"),
        ArmPose::Spear => {}
    }
}
pub fn apply_skeleton_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    apply_humanoid_base_anim(tree, &state);
    if state.is_aggressive && !state.is_holding_bow {
        let attack_time = state.attack_time;
        let attack2 = (attack_time * std::f32::consts::PI).sin();
        let attack =
            ((1.0 - (1.0 - attack_time) * (1.0 - attack_time)) * std::f32::consts::PI).sin();
        if let Some(ra) = tree.part_mut("right_arm") {
            ra.pose.rotation.z = 0.0;
            ra.pose.rotation.y = -(0.1 - attack2 * 0.6);
            ra.pose.rotation.x = -std::f32::consts::PI / 2.0 - (attack2 * 1.2 - attack * 0.4);
        }
        if let Some(la) = tree.part_mut("left_arm") {
            la.pose.rotation.z = 0.0;
            la.pose.rotation.y = 0.1 - attack2 * 0.6;
            la.pose.rotation.x = -std::f32::consts::PI / 2.0 - (attack2 * 1.2 - attack * 0.4);
        }
        bob_arms(tree, state.age_in_ticks);
    }
}
pub fn build_skeleton_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_skeleton_model_tree();
    tree.reset_pose();
    apply_skeleton_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_wither_skeleton_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_wither_skeleton_model_tree();
    tree.reset_pose();
    apply_skeleton_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_bogged_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    apply_skeleton_model_setup_anim(tree, state);
}
pub fn skeleton_translate_to_hand(tree: &mut JavaModelPartTree, arm: HumanoidArm) {
    let offset = if arm == HumanoidArm::Right { 1.0 } else { -1.0 };
    let arm_name = if arm == HumanoidArm::Right {
        "right_arm"
    } else {
        "left_arm"
    };
    if let Some(arm_part) = tree.part_mut(arm_name) {
        arm_part.pose.offset.x += offset;
    }
}
pub(super) fn model_arm_item_matrix(
    tree: &JavaModelPartTree,
    hand: crate::render::model::player::player_model::HandSide,
) -> glam::Mat4 {
    use crate::render::model::player::player_model::{HandSide, third_person_item_layer_matrix};
    let arm_name = if hand == HandSide::Right {
        "right_arm"
    } else {
        "left_arm"
    };
    let basis = glam::Mat4::from_scale(glam::Vec3::new(1.0, -1.0, -1.0));
    glam::Mat4::from_translation(glam::Vec3::new(0.0, 24.0 / 16.0, 0.0))
        * basis
        * glam::Mat4::from_scale(glam::Vec3::splat(1.0 / 16.0))
        * tree.part_or_panic("root").pose.matrix()
        * tree.part_or_panic(arm_name).pose.matrix()
        * glam::Mat4::from_scale(glam::Vec3::splat(16.0))
        * basis
        * third_person_item_layer_matrix(hand)
}
pub fn skeleton_item_in_hand_matrix(
    model_kind: VanillaEntityModelKind,
    state: EntityModelAnimationState,
    hand: crate::render::model::player::player_model::HandSide,
) -> glam::Mat4 {
    use crate::render::model::player::player_model::HandSide;
    let mut tree = match model_kind {
        VanillaEntityModelKind::WitherSkeleton => build_wither_skeleton_model_tree(),
        VanillaEntityModelKind::Bogged => build_bogged_model_tree(),
        _ => build_skeleton_model_tree(),
    };
    apply_skeleton_model_setup_anim(&mut tree, state);
    skeleton_translate_to_hand(
        &mut tree,
        if hand == HandSide::Right {
            HumanoidArm::Right
        } else {
            HumanoidArm::Left
        },
    );
    model_arm_item_matrix(&tree, hand)
}
pub fn build_bogged_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_bogged_model_tree();
    tree.reset_pose();
    apply_bogged_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
