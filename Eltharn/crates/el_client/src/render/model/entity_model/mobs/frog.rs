use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_frog_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("model_root", PartPose::offset(0.0, 24.0, 0.0))
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, -2.0, 4.0))
                    .cube(cube([3.0, 1.0], [-3.5, -2.0, -8.0], [7.0, 3.0, 9.0]))
                    .cube(cube([23.0, 22.0], [-3.5, -1.0, -8.0], [7.0, 0.0, 9.0]))
                    .child(
                        NamedPartSpec::new("head", PartPose::offset(0.0, -2.0, -1.0))
                            .cube(cube([23.0, 13.0], [-3.5, -1.0, -7.0], [7.0, 0.0, 9.0]))
                            .cube(cube([0.0, 13.0], [-3.5, -2.0, -7.0], [7.0, 3.0, 9.0]))
                            .child(
                                NamedPartSpec::new("eyes", PartPose::offset(-0.5, 0.0, 2.0))
                                    .child(
                                        NamedPartSpec::new(
                                            "right_eye",
                                            PartPose::offset(-1.5, -3.0, -6.5),
                                        )
                                        .cube(cube(
                                            [0.0, 0.0],
                                            [-1.5, -1.0, -1.5],
                                            [3.0, 2.0, 3.0],
                                        )),
                                    )
                                    .child(
                                        NamedPartSpec::new(
                                            "left_eye",
                                            PartPose::offset(2.5, -3.0, -6.5),
                                        )
                                        .cube(cube(
                                            [0.0, 5.0],
                                            [-1.5, -1.0, -1.5],
                                            [3.0, 2.0, 3.0],
                                        )),
                                    ),
                            ),
                    )
                    .child(
                        NamedPartSpec::new("croaking_body", PartPose::offset(0.0, -1.0, -5.0))
                            .cube(cube_grown(
                                [26.0, 5.0],
                                [-3.5, -0.1, -2.9],
                                [7.0, 2.0, 3.0],
                                -0.1,
                            )),
                    )
                    .child(
                        NamedPartSpec::new("tongue", PartPose::offset(0.0, -1.01, 1.0)).cube(cube(
                            [17.0, 13.0],
                            [-2.0, 0.0, -7.1],
                            [4.0, 0.0, 7.0],
                        )),
                    )
                    .child(
                        NamedPartSpec::new("left_arm", PartPose::offset(4.0, -1.0, -6.5))
                            .cube(cube([0.0, 32.0], [-1.0, 0.0, -1.0], [2.0, 3.0, 3.0]))
                            .child(
                                NamedPartSpec::new("left_hand", PartPose::offset(0.0, 3.0, -1.0))
                                    .cube(cube([18.0, 40.0], [-4.0, 0.01, -4.0], [8.0, 0.0, 8.0])),
                            ),
                    )
                    .child(
                        NamedPartSpec::new("right_arm", PartPose::offset(-4.0, -1.0, -6.5))
                            .cube(cube([0.0, 38.0], [-1.0, 0.0, -1.0], [2.0, 3.0, 3.0]))
                            .child(
                                NamedPartSpec::new("right_hand", PartPose::offset(0.0, 3.0, 0.0))
                                    .cube(cube([2.0, 40.0], [-4.0, 0.01, -5.0], [8.0, 0.0, 8.0])),
                            ),
                    ),
            )
            .child(
                NamedPartSpec::new("left_leg", PartPose::offset(3.5, -3.0, 4.0))
                    .cube(cube([14.0, 25.0], [-1.0, 0.0, -2.0], [3.0, 3.0, 4.0]))
                    .child(
                        NamedPartSpec::new("left_foot", PartPose::offset(2.0, 3.0, 0.0))
                            .cube(cube([2.0, 32.0], [-4.0, 0.01, -4.0], [8.0, 0.0, 8.0])),
                    ),
            )
            .child(
                NamedPartSpec::new("right_leg", PartPose::offset(-3.5, -3.0, 4.0))
                    .cube(cube([0.0, 25.0], [-2.0, 0.0, -2.0], [3.0, 3.0, 4.0]))
                    .child(
                        NamedPartSpec::new("right_foot", PartPose::offset(-2.0, 3.0, 0.0))
                            .cube(cube([18.0, 32.0], [-4.0, 0.01, -4.0], [8.0, 0.0, 8.0])),
                    ),
            ),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_frog_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_frog_model_tree().mesh(ModelTextureSize {
        width: 48.0,
        height: 48.0,
    })
}

pub fn apply_frog_model_setup_anim(tree: &mut JavaModelPartTree, state: EntityModelAnimationState) {
    let deg2rad = std::f32::consts::PI / 180.0;
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * deg2rad;
        h.pose.rotation.y = state.y_rot_degrees * deg2rad;
    }
    if let Some(l) = tree.part_mut("left_leg") {
        l.pose.rotation.x = (anim_pos * 0.6662).cos() * anim_speed;
    }
    if let Some(l) = tree.part_mut("right_leg") {
        l.pose.rotation.x = (anim_pos * 0.6662 + std::f32::consts::PI).cos() * anim_speed;
    }
}

pub fn build_frog_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_frog_model_tree();
    tree.reset_pose();
    apply_frog_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 48.0,
        height: 48.0,
    })
}

pub fn build_tadpole_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 22.0, -3.0)).cube(cube(
                [0.0, 0.0],
                [-1.5, -1.0, 0.0],
                [3.0, 2.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("tail", PartPose::offset(0.0, 22.0, 0.0)).cube(cube(
                [0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 2.0, 7.0],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_tadpole_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_tadpole_model_tree().mesh(ModelTextureSize {
        width: 16.0,
        height: 16.0,
    })
}

pub fn apply_tadpole_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let mult = if state.is_in_water { 1.0 } else { 1.5 };
    if let Some(t) = tree.part_mut("tail") {
        t.pose.rotation.y = -mult * 0.25 * (0.3 * state.age_in_ticks).sin();
    }
}

pub fn build_tadpole_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_tadpole_model_tree();
    tree.reset_pose();
    apply_tadpole_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 16.0,
        height: 16.0,
    })
}
