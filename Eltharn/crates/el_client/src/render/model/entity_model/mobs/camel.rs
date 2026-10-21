use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_camel_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 4.0, 9.5))
                .cube(cube([0.0, 25.0], [-7.5, -12.0, -23.5], [15.0, 12.0, 27.0]))
                .child(
                    NamedPartSpec::new("hump", PartPose::offset(0.0, -12.0, -10.0)).cube(cube(
                        [74.0, 0.0],
                        [-4.5, -5.0, -5.5],
                        [9.0, 5.0, 11.0],
                    )),
                )
                .child(
                    NamedPartSpec::new("tail", PartPose::offset(0.0, -9.0, 3.5)).cube(cube(
                        [122.0, 0.0],
                        [-1.5, 0.0, 0.0],
                        [3.0, 14.0, 0.0],
                    )),
                )
                .child(
                    NamedPartSpec::new("head", PartPose::offset(0.0, -3.0, -19.5))
                        .cube(cube([60.0, 24.0], [-3.5, -7.0, -15.0], [7.0, 8.0, 19.0]))
                        .cube(cube([21.0, 0.0], [-3.5, -21.0, -15.0], [7.0, 14.0, 7.0]))
                        .cube(cube([50.0, 0.0], [-2.5, -21.0, -21.0], [5.0, 5.0, 6.0]))
                        .child(
                            NamedPartSpec::new("left_ear", PartPose::offset(2.5, -21.0, -9.5))
                                .cube(cube([45.0, 0.0], [-0.5, 0.5, -1.0], [3.0, 1.0, 2.0])),
                        )
                        .child(
                            NamedPartSpec::new("right_ear", PartPose::offset(-2.5, -21.0, -9.5))
                                .cube(cube([67.0, 0.0], [-2.5, 0.5, -1.0], [3.0, 1.0, 2.0])),
                        ),
                ),
        )
        .child(
            NamedPartSpec::new("left_hind_leg", PartPose::offset(4.9, 1.0, 9.5)).cube(cube(
                [58.0, 16.0],
                [-2.5, 2.0, -2.5],
                [5.0, 21.0, 5.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_hind_leg", PartPose::offset(-4.9, 1.0, 9.5)).cube(cube(
                [94.0, 16.0],
                [-2.5, 2.0, -2.5],
                [5.0, 21.0, 5.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_front_leg", PartPose::offset(4.9, 1.0, -10.5)).cube(cube(
                [0.0, 0.0],
                [-2.5, 2.0, -2.5],
                [5.0, 21.0, 5.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_front_leg", PartPose::offset(-4.9, 1.0, -10.5)).cube(cube(
                [0.0, 26.0],
                [-2.5, 2.0, -2.5],
                [5.0, 21.0, 5.0],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_baby_camel_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 7.0, 0.0))
                .cube(cube([0.0, 14.0], [-4.5, -4.0, -8.0], [9.0, 8.0, 16.0]))
                .child(
                    NamedPartSpec::new("tail", PartPose::offset(0.0, -1.5, 8.05)).cube(cube(
                        [50.0, 38.0],
                        [-1.5, -0.5, 0.0],
                        [3.0, 9.0, 0.0],
                    )),
                )
                .child(
                    NamedPartSpec::new("head", PartPose::offset(0.0, 1.0, -7.5))
                        .cube(cube([20.0, 0.0], [-2.5, -3.0, -7.5], [5.0, 5.0, 7.0]))
                        .cube(cube([0.0, 0.0], [-2.5, -12.0, -7.5], [5.0, 9.0, 5.0]))
                        .cube(cube([0.0, 14.0], [-2.5, -12.0, -10.5], [5.0, 4.0, 3.0]))
                        .child(
                            NamedPartSpec::new("right_ear", PartPose::offset(-2.5, -11.0, -4.0))
                                .cube(cube([37.0, 0.0], [-3.0, -0.5, -1.0], [3.0, 1.0, 2.0])),
                        )
                        .child(
                            NamedPartSpec::new("left_ear", PartPose::offset(2.5, -11.0, -4.0))
                                .cube(cube([47.0, 0.0], [0.0, -0.5, -1.0], [3.0, 1.0, 2.0])),
                        ),
                ),
        )
        .child(
            NamedPartSpec::new("right_front_leg", PartPose::offset(-3.0, 11.5, -5.5)).cube(cube(
                [36.0, 14.0],
                [-1.5, -0.5, -1.5],
                [3.0, 13.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_front_leg", PartPose::offset(3.0, 11.5, -5.5)).cube(cube(
                [48.0, 14.0],
                [-1.5, -0.5, -1.5],
                [3.0, 13.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_hind_leg", PartPose::offset(3.0, 11.5, 5.5)).cube(cube(
                [12.0, 38.0],
                [-1.5, -0.5, -1.5],
                [3.0, 13.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_hind_leg", PartPose::offset(-3.0, 11.5, 5.5)).cube(cube(
                [0.0, 38.0],
                [-1.5, -0.5, -1.5],
                [3.0, 13.0, 3.0],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_camel_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_camel_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}

pub fn apply_camel_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: &EntityModelAnimationState,
) {
    let deg2rad = std::f32::consts::PI / 180.0;

    let y_rot = state.y_rot_degrees.clamp(-30.0, 30.0);
    let mut x_rot = state.x_rot_degrees.clamp(-25.0, 45.0);
    if state.jump_cooldown > 0.0 {
        let head_rotation = 45.0 * state.jump_cooldown / 55.0;
        x_rot = (x_rot + head_rotation).clamp(-25.0, 70.0);
    }

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = x_rot * deg2rad;
        h.pose.rotation.y = y_rot * deg2rad;
    }

    if state.is_baby {
        super::camel_animation::CAMEL_BABY_WALK.apply_walk(
            tree,
            state.walk_animation_pos,
            state.walk_animation_speed,
            2.0,
            2.5,
        );
        super::camel_animation::CAMEL_BABY_SIT.apply_state(
            tree,
            &state.camel_sit_animation_state,
            state.age_in_ticks,
        );
        super::camel_animation::CAMEL_BABY_SIT_POSE.apply_state(
            tree,
            &state.camel_sit_pose_animation_state,
            state.age_in_ticks,
        );
        super::camel_animation::CAMEL_BABY_STANDUP.apply_state(
            tree,
            &state.camel_sit_up_animation_state,
            state.age_in_ticks,
        );
        super::camel_animation::CAMEL_BABY_IDLE.apply_state(
            tree,
            &state.camel_idle_animation_state,
            state.age_in_ticks,
        );
        super::camel_animation::CAMEL_BABY_DASH.apply_state(
            tree,
            &state.camel_dash_animation_state,
            state.age_in_ticks,
        );
    } else {
        super::camel_animation::CAMEL_WALK.apply_walk(
            tree,
            state.walk_animation_pos,
            state.walk_animation_speed,
            2.0,
            2.5,
        );
        super::camel_animation::CAMEL_SIT.apply_state(
            tree,
            &state.camel_sit_animation_state,
            state.age_in_ticks,
        );
        super::camel_animation::CAMEL_SIT_POSE.apply_state(
            tree,
            &state.camel_sit_pose_animation_state,
            state.age_in_ticks,
        );
        super::camel_animation::CAMEL_STANDUP.apply_state(
            tree,
            &state.camel_sit_up_animation_state,
            state.age_in_ticks,
        );
        super::camel_animation::CAMEL_IDLE.apply_state(
            tree,
            &state.camel_idle_animation_state,
            state.age_in_ticks,
        );
        super::camel_animation::CAMEL_DASH.apply_state(
            tree,
            &state.camel_dash_animation_state,
            state.age_in_ticks,
        );
    }
}

pub fn build_camel_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    if state.is_baby {
        let mut tree = build_baby_camel_model_tree();
        tree.reset_pose();
        apply_camel_model_setup_anim(&mut tree, &state);
        tree.mesh(ModelTextureSize {
            width: 64.0,
            height: 64.0,
        })
    } else {
        let mut tree = build_camel_model_tree();
        tree.reset_pose();
        apply_camel_model_setup_anim(&mut tree, &state);
        tree.mesh(ModelTextureSize {
            width: 128.0,
            height: 128.0,
        })
    }
}
