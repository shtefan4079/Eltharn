#![allow(dead_code)]
use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;
pub fn build_humanoid_model_tree() -> JavaModelPartTree {
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
                    [-3.0, -2.0, -2.0],
                    [4.0, 12.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_arm", PartPose::offset(5.0, 2.0, 0.0)).cube(
                    cube_mirrored([40.0, 16.0], [-1.0, -2.0, -2.0], [4.0, 12.0, 4.0]),
                ),
            )
            .child(
                NamedPartSpec::new("right_leg", PartPose::offset(-1.9, 12.0, 0.0)).cube(cube(
                    [0.0, 16.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 12.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_leg", PartPose::offset(1.9, 12.0, 0.0)).cube(
                    cube_mirrored([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0]),
                ),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_zombie_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_humanoid_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_drowned_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
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
                [-3.0, -2.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_arm", PartPose::offset(5.0, 2.0, 0.0)).cube(cube(
                [32.0, 48.0],
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
            NamedPartSpec::new("left_leg", PartPose::offset(1.9, 12.0, 0.0)).cube(cube(
                [16.0, 48.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_drowned_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_drowned_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_zombie_villager_model_tree() -> JavaModelPartTree {
    let pi = std::f32::consts::PI;
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("head", PartPose::ZERO)
                .cube(cube([0.0, 0.0], [-4.0, -10.0, -4.0], [8.0, 10.0, 8.0]))
                .cube(cube([24.0, 0.0], [-1.0, -3.0, -6.0], [2.0, 4.0, 2.0]))
                .child(
                    NamedPartSpec::new("hat", PartPose::ZERO)
                        .cube(cube_grown(
                            [32.0, 0.0],
                            [-4.0, -10.0, -4.0],
                            [8.0, 10.0, 8.0],
                            0.5,
                        ))
                        .child(
                            NamedPartSpec::new(
                                "hat_rim",
                                PartPose::offset_and_rotation(0.0, 0.0, 0.0, -pi / 2.0, 0.0, 0.0),
                            )
                            .cube(cube(
                                [30.0, 47.0],
                                [-8.0, -8.0, -6.0],
                                [16.0, 16.0, 1.0],
                            )),
                        ),
                ),
        )
        .child(
            NamedPartSpec::new("body", PartPose::ZERO)
                .cube(cube([16.0, 20.0], [-4.0, 0.0, -3.0], [8.0, 12.0, 6.0]))
                .cube(cube_grown(
                    [0.0, 38.0],
                    [-4.0, 0.0, -3.0],
                    [8.0, 20.0, 6.0],
                    0.05,
                )),
        )
        .child(
            NamedPartSpec::new("right_arm", PartPose::offset(-5.0, 2.0, 0.0)).cube(cube(
                [44.0, 22.0],
                [-3.0, -2.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_arm", PartPose::offset(5.0, 2.0, 0.0)).cube(cube_mirrored(
                [44.0, 22.0],
                [-1.0, -2.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_leg", PartPose::offset(-2.0, 12.0, 0.0)).cube(cube(
                [0.0, 22.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_leg", PartPose::offset(2.0, 12.0, 0.0)).cube(cube_mirrored(
                [0.0, 22.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_zombie_villager_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_zombie_villager_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_baby_zombie_villager_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 18.75, 0.0))
                .cube(cube([0.0, 15.0], [-2.0, -2.75, -1.5], [4.0, 5.0, 3.0]))
                .cube(cube_grown(
                    [16.0, 22.0],
                    [-2.0, -2.75, -1.5],
                    [4.0, 6.0, 3.0],
                    0.1,
                )),
        )
        .child(
            NamedPartSpec::new("head", PartPose::offset(0.0, 16.0, 0.0))
                .cube(cube([0.0, 0.0], [-4.0, -8.0, -3.5], [8.0, 8.0, 7.0]))
                .child(
                    NamedPartSpec::new("hat", PartPose::offset(0.0, -4.0, 0.0))
                        .cube(cube_grown(
                            [0.0, 31.0],
                            [-4.0, -4.0, -3.5],
                            [8.0, 8.0, 7.0],
                            0.3,
                        ))
                        .child(
                            NamedPartSpec::new("hat_rim", PartPose::offset(0.0, -4.5, 0.0))
                                .cube(cube([0.0, 46.0], [-7.0, -0.5, -6.0], [14.0, 1.0, 12.0])),
                        ),
                )
                .child(
                    NamedPartSpec::new("nose", PartPose::offset(0.0, -1.0, -4.0)).cube(cube(
                        [23.0, 0.0],
                        [-1.0, -1.0, -0.5],
                        [2.0, 2.0, 1.0],
                    )),
                ),
        )
        .child(
            NamedPartSpec::new("right_arm", PartPose::offset(-3.0, 15.5, 0.0)).cube(cube(
                [24.0, 15.0],
                [-1.0, -0.5, -1.0],
                [2.0, 5.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_arm", PartPose::offset(3.0, 15.5, 0.0)).cube(cube(
                [16.0, 15.0],
                [-1.0, -0.5, -1.0],
                [2.0, 5.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_leg", PartPose::offset(-1.0, 21.5, 0.0)).cube(cube(
                [8.0, 23.0],
                [-1.0, -0.5, -1.0],
                [2.0, 3.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_leg", PartPose::offset(1.0, 21.5, 0.0)).cube(cube(
                [0.0, 23.0],
                [-1.0, -0.5, -1.0],
                [2.0, 3.0, 2.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_baby_zombie_villager_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_baby_zombie_villager_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_zombie_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    super::skeleton::apply_humanoid_base_anim(tree, &state);
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
pub fn build_baby_zombie_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 17.5, 0.0)).cube(cube(
                [16.0, 16.0],
                [-2.0, -2.5, -1.0],
                [4.0, 5.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("head", PartPose::offset(0.0, 15.25, 0.0))
                .cube(cube([3.0, 3.0], [-3.0, -6.25, -3.0], [6.0, 6.0, 6.0]))
                .cube(cube_grown(
                    [35.0, 3.0],
                    [-3.0, -6.15, -3.0],
                    [6.0, 6.0, 6.0],
                    0.25,
                ))
                .child(NamedPartSpec::new("hat", PartPose::ZERO)),
        )
        .child(
            NamedPartSpec::new("right_arm", PartPose::offset(-3.0, 15.5, 0.0)).cube(cube(
                [36.0, 16.0],
                [-1.0, -0.5, -1.0],
                [2.0, 5.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_arm", PartPose::offset(3.0, 15.5, 0.0)).cube(cube(
                [28.0, 16.0],
                [-1.0, -0.5, -1.0],
                [2.0, 5.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_leg", PartPose::offset(-1.0, 20.0, 0.0)).cube(cube(
                [8.0, 16.0],
                [-1.0, 0.0, -1.0],
                [2.0, 4.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_leg", PartPose::offset(1.0, 20.0, 0.0)).cube(cube(
                [0.0, 16.0],
                [-1.0, 0.0, -1.0],
                [2.0, 4.0, 2.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_baby_zombie_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_baby_zombie_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_zombie_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = if state.is_baby {
        build_baby_zombie_model_tree()
    } else {
        build_humanoid_model_tree()
    };
    tree.reset_pose();
    apply_zombie_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
fn rot_lerp_rad(alpha: f32, from: f32, to: f32) -> f32 {
    from + alpha * wrap_radians(to - from)
}
fn wrap_radians(value: f32) -> f32 {
    (value + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}
pub fn apply_drowned_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    apply_zombie_model_setup_anim(tree, state);
    if state.left_arm_pose == ArmPose::ThrowTrident {
        if let Some(la) = tree.part_mut("left_arm") {
            la.pose.rotation.x = la.pose.rotation.x * 0.5 - std::f32::consts::PI;
            la.pose.rotation.y = 0.0;
        }
    }
    if state.right_arm_pose == ArmPose::ThrowTrident {
        if let Some(ra) = tree.part_mut("right_arm") {
            ra.pose.rotation.x = ra.pose.rotation.x * 0.5 - std::f32::consts::PI;
            ra.pose.rotation.y = 0.0;
        }
    }
    let swim_amount = state.swim_amount;
    if swim_amount > 0.0 {
        let age = state.age_in_ticks;
        let sin_age = (0.1 * age).sin();
        let target_x_rot = -std::f32::consts::PI * 4.0 / 5.0;
        if let Some(ra) = tree.part_mut("right_arm") {
            ra.pose.rotation.x = rot_lerp_rad(swim_amount, ra.pose.rotation.x, target_x_rot)
                + swim_amount * 0.35 * sin_age;
            ra.pose.rotation.z = rot_lerp_rad(swim_amount, ra.pose.rotation.z, -0.15);
        }
        if let Some(la) = tree.part_mut("left_arm") {
            la.pose.rotation.x = rot_lerp_rad(swim_amount, la.pose.rotation.x, target_x_rot)
                - swim_amount * 0.35 * sin_age;
            la.pose.rotation.z = rot_lerp_rad(swim_amount, la.pose.rotation.z, 0.15);
        }
        if let Some(ll) = tree.part_mut("left_leg") {
            ll.pose.rotation.x -= swim_amount * 0.55 * sin_age;
        }
        if let Some(rl) = tree.part_mut("right_leg") {
            rl.pose.rotation.x += swim_amount * 0.55 * sin_age;
        }
        if let Some(head) = tree.part_mut("head") {
            head.pose.rotation.x = 0.0;
        }
    }
}
pub fn build_drowned_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_drowned_model_tree();
    tree.reset_pose();
    apply_drowned_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_zombie_villager_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = if state.is_baby {
        build_baby_zombie_villager_model_tree()
    } else {
        build_zombie_villager_model_tree()
    };
    tree.reset_pose();
    apply_zombie_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
