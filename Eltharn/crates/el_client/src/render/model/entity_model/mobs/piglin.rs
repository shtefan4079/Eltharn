use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_piglin_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(NamedPartSpec::new("body", PartPose::ZERO).cube(cube(
            [16.0, 16.0],
            [-4.0, 0.0, -2.0],
            [8.0, 12.0, 4.0],
        )))
        .child(
            NamedPartSpec::new("head", PartPose::ZERO)
                .cube(cube([0.0, 0.0], [-5.0, -8.0, -4.0], [10.0, 8.0, 8.0]))
                .cube(cube([31.0, 1.0], [-2.0, -4.0, -5.0], [4.0, 4.0, 1.0]))
                .cube(cube([2.0, 4.0], [2.0, -2.0, -5.0], [1.0, 2.0, 1.0]))
                .cube(cube([2.0, 0.0], [-3.0, -2.0, -5.0], [1.0, 2.0, 1.0]))
                .child(
                    NamedPartSpec::new(
                        "left_ear",
                        PartPose::offset_and_rotation(4.5, -6.0, 0.0, 0.0, 0.0, -0.5235988),
                    )
                    .cube(cube([51.0, 6.0], [0.0, 0.0, -2.0], [1.0, 5.0, 4.0])),
                )
                .child(
                    NamedPartSpec::new(
                        "right_ear",
                        PartPose::offset_and_rotation(-4.5, -6.0, 0.0, 0.0, 0.0, 0.5235988),
                    )
                    .cube(cube(
                        [39.0, 6.0],
                        [-1.0, 0.0, -2.0],
                        [1.0, 5.0, 4.0],
                    )),
                ),
        )
        .child(
            NamedPartSpec::new("right_arm", PartPose::offset(-5.0, 2.0, 0.0)).cube(cube(
                [40.0, 16.0],
                [-3.0, -2.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_arm", PartPose::offset(5.0, 2.0, 0.0)).cube(cube_mirrored(
                [40.0, 16.0],
                [-1.0, -2.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_leg", PartPose::offset(-1.9, 12.0, 0.0)).cube(cube(
                [0.0, 16.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_leg", PartPose::offset(1.9, 12.0, 0.0)).cube(cube_mirrored(
                [0.0, 16.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_baby_piglin_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 18.0, -0.5)).cube(cube(
                [0.0, 13.0],
                [-3.0, -3.0, -1.0],
                [6.0, 5.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("head", PartPose::offset(0.0, 15.0, 0.0))
                .cube(cube([21.0, 30.0], [-1.5, -3.0, -4.5], [3.0, 3.0, 1.0]))
                .cube(cube([0.0, 0.0], [-4.5, -6.0, -3.5], [9.0, 6.0, 7.0]))
                .child(
                    NamedPartSpec::new("left_ear", PartPose::offset(4.2, -4.0, 0.0)).child(
                        NamedPartSpec::new(
                            "left_ear_r1",
                            PartPose::offset_and_rotation(1.0, 1.75, 0.0, 0.0, 0.0, -0.6109),
                        )
                        .cube(cube(
                            [0.0, 21.0],
                            [-0.5, -3.0, -2.0],
                            [1.0, 6.0, 4.0],
                        )),
                    ),
                )
                .child(
                    NamedPartSpec::new("right_ear", PartPose::offset(-4.2, -4.0, 0.0)).child(
                        NamedPartSpec::new(
                            "right_ear_r1",
                            PartPose::offset_and_rotation(-1.0, 1.75, 0.0, 0.0, 0.0, 0.6109),
                        )
                        .cube(cube(
                            [18.0, 13.0],
                            [-0.5, -3.0, -2.0],
                            [1.0, 6.0, 4.0],
                        )),
                    ),
                ),
        )
        .child(
            NamedPartSpec::new("left_arm", PartPose::offset(4.0, 15.0, 0.0)).cube(cube(
                [28.0, 13.0],
                [-1.0, 0.0, -1.5],
                [2.0, 5.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_arm", PartPose::offset(-4.0, 15.0, 0.0)).cube(cube(
                [10.0, 30.0],
                [-1.0, 0.0, -1.5],
                [2.0, 5.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_leg", PartPose::offset(-1.5, 20.0, 0.0)).cube(cube(
                [22.0, 23.0],
                [-1.5, 0.0, -1.5],
                [3.0, 4.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_leg", PartPose::offset(1.5, 20.0, 0.0)).cube(cube(
                [10.0, 23.0],
                [-1.5, 0.0, -1.5],
                [3.0, 4.0, 3.0],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_piglin_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_piglin_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

fn apply_piglin_ears_animation(tree: &mut JavaModelPartTree, state: &EntityModelAnimationState) {
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;
    let default_ear_angle =
        (if state.is_baby { 5.0 } else { 30.0 }) * (std::f32::consts::PI / 180.0);
    let frequency = state.age_in_ticks * 0.1 + anim_pos * 0.5;
    let amplitude = 0.08 + anim_speed * 0.4;

    if let Some(le) = tree.part_mut("left_ear") {
        le.pose.rotation.z = -default_ear_angle - (frequency * 1.2).cos() * amplitude;
    }
    if let Some(re) = tree.part_mut("right_ear") {
        re.pose.rotation.z = default_ear_angle + frequency.cos() * amplitude;
    }
}

pub fn apply_piglin_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    super::skeleton::apply_humanoid_base_anim(tree, &state);
    apply_piglin_ears_animation(tree, &state);

    let pi = std::f32::consts::PI;
    let deg2rad = pi / 180.0;

    if state.is_dancing {
        let dance_pos = state.age_in_ticks / 60.0;
        if let Some(re) = tree.part_mut("right_ear") {
            re.pose.rotation.z = (pi / 6.0) + (dance_pos * 30.0).sin() * 10.0 * deg2rad;
        }
        if let Some(le) = tree.part_mut("left_ear") {
            le.pose.rotation.z = -(pi / 6.0) - (dance_pos * 30.0).cos() * 10.0 * deg2rad;
        }
        if let Some(h) = tree.part_mut("head") {
            h.pose.offset.x += (dance_pos * 10.0).sin();
            h.pose.offset.y += (dance_pos * 40.0).sin() + 0.4;
        }
        let r_arm_z = deg2rad * (70.0 + (dance_pos * 40.0).cos() * 10.0);
        if let Some(ra) = tree.part_mut("right_arm") {
            ra.pose.rotation.z = r_arm_z;
            ra.pose.offset.y += (dance_pos * 40.0).sin() * 0.5 - 0.5;
        }
        if let Some(la) = tree.part_mut("left_arm") {
            la.pose.rotation.z = -r_arm_z;
            la.pose.offset.y += (dance_pos * 40.0).sin() * 0.5 + 0.5;
        }
        if let Some(b) = tree.part_mut("body") {
            b.pose.offset.y += (dance_pos * 40.0).sin() * 0.35;
        }
    } else if state.is_aggressive && state.attack_time == 0.0 && state.is_holding_item {
        if state.attack_arm == HumanoidArm::Left {
            if let Some(la) = tree.part_mut("left_arm") {
                la.pose.rotation.x = -1.8;
            }
        } else if let Some(ra) = tree.part_mut("right_arm") {
            ra.pose.rotation.x = -1.8;
        }
    } else if state.right_arm_pose == ArmPose::CrossbowHold
        || state.left_arm_pose == ArmPose::CrossbowHold
    {
        let holding_in_right = state.right_arm_pose == ArmPose::CrossbowHold;
        let head_x_rot = state.x_rot_degrees * deg2rad;
        let head_y_rot = state.y_rot_degrees * deg2rad;
        let (holding_arm, shooting_arm) = if holding_in_right {
            ("right_arm", "left_arm")
        } else {
            ("left_arm", "right_arm")
        };
        let sign = if holding_in_right { -0.3 } else { 0.3 };
        if let Some(ha) = tree.part_mut(holding_arm) {
            ha.pose.rotation.y = sign + head_y_rot;
            ha.pose.rotation.x = -pi / 2.0 + head_x_rot + 0.1;
        }
        let sign2 = if holding_in_right { 0.6 } else { -0.6 };
        if let Some(sa) = tree.part_mut(shooting_arm) {
            sa.pose.rotation.y = sign2 + head_y_rot;
            sa.pose.rotation.x = -1.5 + head_x_rot;
        }
    } else if state.right_arm_pose == ArmPose::CrossbowCharge
        || state.left_arm_pose == ArmPose::CrossbowCharge
    {
        let holding_in_right = state.right_arm_pose == ArmPose::CrossbowCharge;
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
        let max_charge = 25.0;
        let use_ticks = (state.holding_animation_progress * max_charge).clamp(0.0, max_charge);
        let lerp_alpha = use_ticks / max_charge;
        let pull_y = (0.4 + (0.85 - 0.4) * lerp_alpha) * if holding_in_right { 1.0 } else { -1.0 };
        if let Some(pa) = tree.part_mut(pulling_arm) {
            pa.pose.rotation.x = -0.97079635 + (-pi / 2.0 - (-0.97079635)) * lerp_alpha;
            pa.pose.rotation.y = pull_y;
        }
    }
}

pub fn apply_zombified_piglin_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    super::skeleton::apply_humanoid_base_anim(tree, &state);
    apply_piglin_ears_animation(tree, &state);

    let animate_attack = state.swing_animation_type != SwingAnimationType::Stab;
    if animate_attack {
        let raise_arms = !state.is_baby || state.main_hand_item_empty;
        let aggressive = state.is_aggressive;
        let arm_drop = if raise_arms {
            -std::f32::consts::PI / if aggressive { 1.5 } else { 2.25 }
        } else {
            0.0
        };
        let attack_time = state.attack_time;
        let attack_y_rot_mod =
            (if raise_arms { 1.0 } else { -1.0 }) * (attack_time * std::f32::consts::PI).sin();
        let attack_x_rot_mod =
            ((1.0 - (1.0 - attack_time) * (1.0 - attack_time)) * std::f32::consts::PI).sin();
        let x_rot = arm_drop + attack_y_rot_mod * 1.2 - attack_x_rot_mod * 0.4;
        let y_rot = 0.1 - attack_y_rot_mod * 0.6;

        if let Some(ra) = tree.part_mut("right_arm") {
            ra.pose.rotation.x = x_rot;
            ra.pose.rotation.y = if raise_arms { -y_rot } else { y_rot };
            ra.pose.rotation.z = 0.0;
        }
        if let Some(la) = tree.part_mut("left_arm") {
            la.pose.rotation.x = x_rot;
            la.pose.rotation.y = if raise_arms { y_rot } else { -y_rot };
            la.pose.rotation.z = 0.0;
        }
    }

    let age = state.age_in_ticks;
    if let Some(ra) = tree.part_mut("right_arm") {
        ra.pose.rotation.z += (age * 0.09).cos() * 0.05 + 0.05;
        ra.pose.rotation.x += (age * 0.067).sin() * 0.05;
    }
    if let Some(la) = tree.part_mut("left_arm") {
        la.pose.rotation.z -= (age * 0.09).cos() * 0.05 + 0.05;
        la.pose.rotation.x -= (age * 0.067).sin() * 0.05;
    }
}

pub fn build_piglin_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = if state.is_baby {
        build_baby_piglin_model_tree()
    } else {
        build_piglin_model_tree()
    };
    tree.reset_pose();
    apply_piglin_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

pub fn build_zombified_piglin_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = if state.is_baby {
        build_baby_piglin_model_tree()
    } else {
        build_piglin_model_tree()
    };
    tree.reset_pose();
    apply_zombified_piglin_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
