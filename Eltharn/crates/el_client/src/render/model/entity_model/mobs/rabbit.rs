use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_rabbit_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "body",
                PartPose::offset_and_rotation(0.0, 23.0, 4.0, -0.3927, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-4.0, -6.0, -9.0], [8.0, 6.0, 10.0]))
            .child(
                NamedPartSpec::new("tail", PartPose::offset(0.0, -4.9916, 0.0125)).cube(cube(
                    [20.0, 16.0],
                    [-2.0, -3.0084, -1.0125],
                    [4.0, 4.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new(
                    "head",
                    PartPose::offset_and_rotation(0.0, -5.2929, -8.1213, 0.3927, 0.0, 0.0),
                )
                .cube(cube([0.0, 16.0], [-2.5, -3.0, -4.0], [5.0, 5.0, 5.0]))
                .child(
                    NamedPartSpec::new("left_ear", PartPose::offset(1.5, -3.7071, -0.8787))
                        .cube(cube([32.0, 0.0], [-1.0, -4.2929, -0.1213], [2.0, 5.0, 1.0])),
                )
                .child(
                    NamedPartSpec::new("right_ear", PartPose::offset(-1.5, -3.7071, -0.8787))
                        .cube(cube([26.0, 0.0], [-1.0, -4.2929, -0.1213], [2.0, 5.0, 1.0])),
                ),
            )
            .child(
                NamedPartSpec::new("frontlegs", PartPose::offset(0.0, -1.5349, -6.3108))
                    .child(
                        NamedPartSpec::new(
                            "right_front_leg",
                            PartPose::offset_and_rotation(-2.0, 1.9239, 0.3827, 0.3927, 0.0, 0.0),
                        )
                        .cube(cube(
                            [36.0, 18.0],
                            [-0.9, -1.0, -0.9],
                            [2.0, 4.0, 2.0],
                        )),
                    )
                    .child(
                        NamedPartSpec::new(
                            "left_front_leg",
                            PartPose::offset_and_rotation(2.0, 1.9239, 0.4827, 0.3927, 0.0, 0.0),
                        )
                        .cube(cube(
                            [44.0, 18.0],
                            [-1.0, -1.0, -1.0],
                            [2.0, 4.0, 2.0],
                        )),
                    ),
            ),
        )
        .child(
            NamedPartSpec::new("backlegs", PartPose::offset(0.0, 23.0, 4.0))
                .child(
                    NamedPartSpec::new("right_hind_leg", PartPose::offset(-3.0, 0.5, 0.0)).child(
                        NamedPartSpec::new(
                            "right_haunch",
                            PartPose::offset_and_rotation(0.0, -0.5, 0.0, 0.0, 0.3927, 0.0),
                        )
                        .cube(cube(
                            [20.0, 24.0],
                            [-1.0, 0.0, -5.0],
                            [2.0, 1.0, 6.0],
                        )),
                    ),
                )
                .child(
                    NamedPartSpec::new("left_hind_leg", PartPose::offset(3.0, 0.5, 0.0)).child(
                        NamedPartSpec::new(
                            "left_haunch",
                            PartPose::offset_and_rotation(0.0, -0.5, 0.0, 0.0, -0.3927, 0.0),
                        )
                        .cube(cube(
                            [36.0, 24.0],
                            [-1.0, 0.0, -5.0],
                            [2.0, 1.0, 6.0],
                        )),
                    ),
                ),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_rabbit_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_rabbit_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

pub fn apply_rabbit_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    }
}

pub fn build_rabbit_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_rabbit_model_tree();
    tree.reset_pose();
    apply_rabbit_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
