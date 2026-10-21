use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;
pub fn build_illager_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::ZERO)
                    .cube(cube([0.0, 0.0], [-4.0, -10.0, -4.0], [8.0, 10.0, 8.0]))
                    .child(NamedPartSpec::new("hat", PartPose::ZERO).cube(cube_grown(
                        [32.0, 0.0],
                        [-4.0, -10.0, -4.0],
                        [8.0, 12.0, 8.0],
                        0.45,
                    )))
                    .child(
                        NamedPartSpec::new("nose", PartPose::offset(0.0, -2.0, 0.0)).cube(cube(
                            [24.0, 0.0],
                            [-1.0, -1.0, -6.0],
                            [2.0, 4.0, 2.0],
                        )),
                    ),
            )
            .child(
                NamedPartSpec::new("body", PartPose::ZERO)
                    .cube(cube([16.0, 20.0], [-4.0, 0.0, -3.0], [8.0, 12.0, 6.0]))
                    .cube(cube_grown(
                        [0.0, 38.0],
                        [-4.0, 0.0, -3.0],
                        [8.0, 20.0, 6.0],
                        0.5,
                    )),
            )
            .child(
                NamedPartSpec::new(
                    "arms",
                    PartPose::offset_and_rotation(0.0, 3.0, -1.0, -0.75, 0.0, 0.0),
                )
                .cube(cube([44.0, 22.0], [-8.0, -2.0, -2.0], [4.0, 8.0, 4.0]))
                .cube(cube([40.0, 38.0], [-4.0, 2.0, -2.0], [8.0, 4.0, 4.0]))
                .child(
                    NamedPartSpec::new("left_shoulder", PartPose::ZERO).cube(cube_mirrored(
                        [44.0, 22.0],
                        [4.0, -2.0, -2.0],
                        [4.0, 8.0, 4.0],
                    )),
                ),
            )
            .child(
                NamedPartSpec::new("right_leg", PartPose::offset(-2.0, 12.0, 0.0)).cube(cube(
                    [0.0, 22.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 12.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_leg", PartPose::offset(2.0, 12.0, 0.0)).cube(
                    cube_mirrored([0.0, 22.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0]),
                ),
            )
            .child(
                NamedPartSpec::new("right_arm", PartPose::offset(-5.0, 2.0, 0.0)).cube(cube(
                    [40.0, 46.0],
                    [-3.0, -2.0, -2.0],
                    [4.0, 12.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_arm", PartPose::offset(5.0, 2.0, 0.0)).cube(
                    cube_mirrored([40.0, 46.0], [-1.0, -2.0, -2.0], [4.0, 12.0, 4.0]),
                ),
            );
    let mut tree = JavaModelPartTree::from_root(root);
    tree.part_mut_or_panic("hat").visible = false;
    tree
}
pub fn apply_illager_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    let pi = std::f32::consts::PI;
    let head_x = state.x_rot_degrees.to_radians();
    let head_y = state.y_rot_degrees.to_radians();
    tree.part_mut_or_panic("head")
        .set_rotation(head_x, head_y, 0.0);
    if state.is_riding {
        tree.part_mut_or_panic("right_arm")
            .set_rotation(-pi / 5.0, 0.0, 0.0);
        tree.part_mut_or_panic("left_arm")
            .set_rotation(-pi / 5.0, 0.0, 0.0);
        tree.part_mut_or_panic("right_leg")
            .set_rotation(-1.4137167, pi / 10.0, 0.07853982);
        tree.part_mut_or_panic("left_leg")
            .set_rotation(-1.4137167, -pi / 10.0, -0.07853982);
    } else {
        let cycle = state.walk_animation_pos * 0.6662;
        let speed = state.walk_animation_speed;
        tree.part_mut_or_panic("right_arm")
            .set_rotation((cycle + pi).cos() * speed, 0.0, 0.0);
        tree.part_mut_or_panic("left_arm")
            .set_rotation(cycle.cos() * speed, 0.0, 0.0);
        tree.part_mut_or_panic("right_leg")
            .set_rotation(cycle.cos() * 1.4 * speed * 0.5, 0.0, 0.0);
        tree.part_mut_or_panic("left_leg").set_rotation(
            (cycle + pi).cos() * 1.4 * speed * 0.5,
            0.0,
            0.0,
        );
    }
    match state.illager_arm_pose {
        IllagerArmPose::CrossbowHold => {
            super::skeleton::pose_crossbow_hold(tree, head_x, head_y, true)
        }
        IllagerArmPose::CrossbowCharge => super::skeleton::pose_crossbow_charge(
            tree,
            state.max_crossbow_charge_duration,
            state.ticks_using_item,
            true,
        ),
        IllagerArmPose::BowAndArrow => {
            tree.part_mut_or_panic("right_arm").set_rotation(
                -pi / 2.0 + head_x,
                -0.1 + head_y,
                0.0,
            );
            tree.part_mut_or_panic("left_arm").set_rotation(
                -0.9424779 + head_x,
                head_y - 0.4,
                pi / 2.0,
            );
        }
        IllagerArmPose::Attacking => {
            if state.main_hand_item_empty {
                if state.swing_animation_type != SwingAnimationType::Stab {
                    let raise = !state.is_baby || state.main_hand_item_empty;
                    let attack_y =
                        (if raise { 1.0 } else { -1.0 }) * (state.attack_time * pi).sin();
                    let attack_x = ((1.0 - (1.0 - state.attack_time).powi(2)) * pi).sin();
                    let drop = if raise { -pi / 1.5 } else { 0.0 };
                    let x = drop + attack_y * 1.2 - attack_x * 0.4;
                    let y = 0.1 - attack_y * 0.6;
                    tree.part_mut_or_panic("right_arm").set_rotation(
                        x,
                        if raise { -y } else { y },
                        0.0,
                    );
                    tree.part_mut_or_panic("left_arm").set_rotation(
                        x,
                        if raise { y } else { -y },
                        0.0,
                    );
                }
                super::skeleton::bob_arms(tree, state.age_in_ticks);
            } else {
                let attack2 = (state.attack_anim * pi).sin();
                let attack = ((1.0 - (1.0 - state.attack_anim).powi(2)) * pi).sin();
                for (name, main, y) in [
                    ("right_arm", state.main_arm == HumanoidArm::Right, pi / 20.0),
                    ("left_arm", state.main_arm == HumanoidArm::Left, -pi / 20.0),
                ] {
                    let x = if main {
                        -1.8849558 + (state.age_in_ticks * 0.09).cos() * 0.15 + attack2 * 2.2
                            - attack * 0.4
                    } else {
                        (state.age_in_ticks * 0.19).cos() * 0.5 + attack2 * 1.2 - attack * 0.4
                    };
                    tree.part_mut_or_panic(name).set_rotation(x, y, 0.0);
                }
                super::skeleton::bob_arms(tree, state.age_in_ticks);
            }
        }
        IllagerArmPose::Spellcasting | IllagerArmPose::Celebrating => {
            let spellcasting = state.illager_arm_pose == IllagerArmPose::Spellcasting;
            let x = (state.age_in_ticks * 0.6662).cos() * if spellcasting { 0.25 } else { 0.05 };
            for (name, pivot, z) in [
                (
                    "right_arm",
                    -5.0,
                    if spellcasting {
                        pi * 3.0 / 4.0
                    } else {
                        2.670354
                    },
                ),
                ("left_arm", 5.0, -pi * 3.0 / 4.0),
            ] {
                let arm = tree.part_mut_or_panic(name);
                arm.pose.offset.x = pivot;
                arm.pose.offset.z = 0.0;
                arm.set_rotation(x, 0.0, z);
            }
        }
        IllagerArmPose::Crossed | IllagerArmPose::Neutral => {}
    }
    let crossed = state.illager_arm_pose == IllagerArmPose::Crossed;
    tree.part_mut_or_panic("arms").visible = crossed;
    tree.part_mut_or_panic("left_arm").visible = !crossed;
    tree.part_mut_or_panic("right_arm").visible = !crossed;
}
pub fn illager_item_in_hand_matrix(
    state: EntityModelAnimationState,
    hand: crate::render::model::player::player_model::HandSide,
) -> glam::Mat4 {
    let mut tree = build_illager_model_tree();
    apply_illager_model_setup_anim(&mut tree, state);
    super::skeleton::model_arm_item_matrix(&tree, hand)
}
pub fn build_illager_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_illager_model_tree();
    apply_illager_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_illager_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let tree = build_illager_model_tree();
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_illusioner_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_illager_model_tree();
    apply_illager_model_setup_anim(&mut tree, state);
    tree.part_mut_or_panic("hat").visible = true;
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_illusioner_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_illusioner_mesh_animated(EntityModelAnimationState::default())
}
