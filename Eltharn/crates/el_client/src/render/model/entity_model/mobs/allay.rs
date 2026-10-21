use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_allay_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::offset(0.0, 23.5, 0.0))
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, -3.99, 0.0)).cube(cube(
                    [0.0, 0.0],
                    [-2.5, -5.0, -2.5],
                    [5.0, 5.0, 5.0],
                )),
            )
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, -4.0, 0.0))
                    .cube(cube([0.0, 10.0], [-1.5, 0.0, -1.0], [3.0, 4.0, 2.0]))
                    .cube(cube_grown(
                        [0.0, 16.0],
                        [-1.5, 0.0, -1.0],
                        [3.0, 5.0, 2.0],
                        -0.2,
                    ))
                    .child(
                        NamedPartSpec::new("right_arm", PartPose::offset(-1.75, 0.5, 0.0)).cube(
                            cube_grown([23.0, 0.0], [-0.75, -0.5, -1.0], [1.0, 4.0, 2.0], -0.01),
                        ),
                    )
                    .child(
                        NamedPartSpec::new("left_arm", PartPose::offset(1.75, 0.5, 0.0)).cube(
                            cube_grown([23.0, 6.0], [-0.25, -0.5, -1.0], [1.0, 4.0, 2.0], -0.01),
                        ),
                    )
                    .child(
                        NamedPartSpec::new("right_wing", PartPose::offset(-0.5, 0.0, 0.6))
                            .cube(cube([16.0, 14.0], [0.0, 1.0, 0.0], [0.0, 5.0, 8.0])),
                    )
                    .child(
                        NamedPartSpec::new("left_wing", PartPose::offset(0.5, 0.0, 0.6))
                            .cube(cube([16.0, 14.0], [0.0, 1.0, 0.0], [0.0, 5.0, 8.0])),
                    ),
            );

    JavaModelPartTree::from_root(root)
}

pub fn build_allay_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_allay_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}

pub fn apply_allay_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let animation_speed = state.walk_animation_speed;
    let animation_pos = state.walk_animation_pos;
    let flap_speed = state.age_in_ticks * 20.0 * (std::f32::consts::PI / 180.0) + animation_pos;
    let flap_amount = (flap_speed).cos() * std::f32::consts::PI * 0.15 + animation_speed;
    let idle_bob_speed = state.age_in_ticks * 9.0 * (std::f32::consts::PI / 180.0);
    let flying_factor = (animation_speed / 0.3).min(1.0);
    let idle_bob_factor = 1.0 - flying_factor;
    let holding_item_factor = state.holding_animation_progress;
    let arm_flying_rot_x = holding_item_factor
        * (-std::f32::consts::PI / 3.0_f32
            + (flying_factor) * (-1.134464 - -std::f32::consts::PI / 3.0_f32));
    let arm_idle_bob_factor = idle_bob_factor * (1.0 - holding_item_factor);
    let arm_idle_bob_amount = 0.43633232
        - (idle_bob_speed + (std::f32::consts::PI * 3.0 / 2.0)).cos()
            * std::f32::consts::PI
            * 0.075
            * arm_idle_bob_factor;
    let anim_value_0 = if state.is_dancing {
        if state.is_spinning {
            (std::f32::consts::PI * 4.0_f32) * (state.spinning_progress)
        } else {
            tree.part_or_panic("root").pose.rotation.y
        }
    } else {
        tree.part_or_panic("root").pose.rotation.y
    };
    tree.part_mut_or_panic("root").pose.rotation.y = anim_value_0;
    let anim_value_1 = if state.is_dancing {
        ((state.age_in_ticks * 8.0 * (std::f32::consts::PI / 180.0) + animation_speed).cos()
            * 16.0
            * (std::f32::consts::PI / 180.0))
            * (1.0 - (state.spinning_progress))
    } else {
        tree.part_or_panic("root").pose.rotation.z
    };
    tree.part_mut_or_panic("root").pose.rotation.z = anim_value_1;
    let anim_value_2 = if state.is_dancing {
        ((state.age_in_ticks * 8.0 * (std::f32::consts::PI / 180.0) + animation_speed).cos()
            * 30.0
            * (std::f32::consts::PI / 180.0))
            * (1.0 - (state.spinning_progress))
    } else {
        tree.part_or_panic("head").pose.rotation.y
    };
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_2;
    let anim_value_3 = if state.is_dancing {
        ((state.age_in_ticks * 8.0 * (std::f32::consts::PI / 180.0) + animation_speed).cos()
            * 14.0
            * (std::f32::consts::PI / 180.0))
            * (1.0 - (state.spinning_progress))
    } else {
        tree.part_or_panic("head").pose.rotation.z
    };
    tree.part_mut_or_panic("head").pose.rotation.z = anim_value_3;
    let anim_value_4 = if !(state.is_dancing) {
        state.x_rot_degrees * (std::f32::consts::PI / 180.0)
    } else {
        tree.part_or_panic("head").pose.rotation.x
    };
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_4;
    let anim_value_5 = if !(state.is_dancing) {
        state.y_rot_degrees * (std::f32::consts::PI / 180.0)
    } else {
        tree.part_or_panic("head").pose.rotation.y
    };
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_5;
    let anim_value_6 = 0.43633232 * (1.0 - flying_factor);
    tree.part_mut_or_panic("right_wing").pose.rotation.x = anim_value_6;
    let anim_value_7 = (-std::f32::consts::PI / 4.0_f32) + flap_amount;
    tree.part_mut_or_panic("right_wing").pose.rotation.y = anim_value_7;
    let anim_value_8 = 0.43633232 * (1.0 - flying_factor);
    tree.part_mut_or_panic("left_wing").pose.rotation.x = anim_value_8;
    let anim_value_9 = (std::f32::consts::PI / 4.0_f32) - flap_amount;
    tree.part_mut_or_panic("left_wing").pose.rotation.y = anim_value_9;
    let anim_value_10 = flying_factor * (std::f32::consts::PI / 4.0_f32);
    tree.part_mut_or_panic("body").pose.rotation.x = anim_value_10;
    let anim_value_11 =
        tree.part_or_panic("root").pose.offset.y + (idle_bob_speed).cos() * 0.25 * idle_bob_factor;
    tree.part_mut_or_panic("root").pose.offset.y = anim_value_11;
    let anim_value_12 = arm_flying_rot_x;
    tree.part_mut_or_panic("right_arm").pose.rotation.x = anim_value_12;
    let anim_value_13 = arm_flying_rot_x;
    tree.part_mut_or_panic("left_arm").pose.rotation.x = anim_value_13;
    let anim_value_14 = -arm_idle_bob_amount;
    tree.part_mut_or_panic("left_arm").pose.rotation.z = anim_value_14;
    let anim_value_15 = arm_idle_bob_amount;
    tree.part_mut_or_panic("right_arm").pose.rotation.z = anim_value_15;
    let anim_value_16 = 0.27925268 * holding_item_factor;
    tree.part_mut_or_panic("right_arm").pose.rotation.y = anim_value_16;
    let anim_value_17 = -0.27925268 * holding_item_factor;
    tree.part_mut_or_panic("left_arm").pose.rotation.y = anim_value_17;
}

pub fn build_allay_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_allay_model_tree();
    tree.reset_pose();
    apply_allay_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
