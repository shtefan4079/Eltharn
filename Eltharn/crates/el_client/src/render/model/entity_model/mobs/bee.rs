use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_bee_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("bone", PartPose::offset(0.0, 19.0, 0.0))
            .child(
                NamedPartSpec::new("body", PartPose::ZERO)
                    .cube(cube([0.0, 0.0], [-3.5, -4.0, -5.0], [7.0, 7.0, 10.0]))
                    .child(NamedPartSpec::new("stinger", PartPose::ZERO).cube(cube(
                        [26.0, 7.0],
                        [0.0, -1.0, 5.0],
                        [0.0, 1.0, 2.0],
                    )))
                    .child(
                        NamedPartSpec::new("left_antenna", PartPose::offset(0.0, -2.0, -5.0))
                            .cube(cube([2.0, 0.0], [1.5, -2.0, -3.0], [1.0, 2.0, 3.0])),
                    )
                    .child(
                        NamedPartSpec::new("right_antenna", PartPose::offset(0.0, -2.0, -5.0))
                            .cube(cube([2.0, 3.0], [-2.5, -2.0, -3.0], [1.0, 2.0, 3.0])),
                    ),
            )
            .child(
                NamedPartSpec::new(
                    "right_wing",
                    PartPose::offset_and_rotation(-1.5, -4.0, -3.0, 0.0, -0.2618, 0.0),
                )
                .cube(cube_grown(
                    [0.0, 18.0],
                    [-9.0, 0.0, 0.0],
                    [9.0, 0.0, 6.0],
                    0.001,
                )),
            )
            .child(
                NamedPartSpec::new(
                    "left_wing",
                    PartPose::offset_and_rotation(1.5, -4.0, -3.0, 0.0, 0.2618, 0.0),
                )
                .cube(cube_mirrored([0.0, 18.0], [0.0, 0.0, 0.0], [9.0, 0.0, 6.0])),
            )
            .child(
                NamedPartSpec::new("front_legs", PartPose::offset(1.5, 3.0, -2.0)).cube(cube(
                    [26.0, 1.0],
                    [-5.0, 0.0, 0.0],
                    [7.0, 2.0, 0.0],
                )),
            )
            .child(
                NamedPartSpec::new("middle_legs", PartPose::offset(1.5, 3.0, 0.0)).cube(cube(
                    [26.0, 3.0],
                    [-5.0, 0.0, 0.0],
                    [7.0, 2.0, 0.0],
                )),
            )
            .child(
                NamedPartSpec::new("back_legs", PartPose::offset(1.5, 3.0, 2.0)).cube(cube(
                    [26.0, 5.0],
                    [-5.0, 0.0, 0.0],
                    [7.0, 2.0, 0.0],
                )),
            ),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_bee_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_bee_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

pub fn apply_bee_model_setup_anim(tree: &mut JavaModelPartTree, state: EntityModelAnimationState) {
    let age = state.age_in_ticks;
    let speed = age * 120.32113 * (std::f32::consts::PI / 180.0);

    if let Some(rw) = tree.part_mut("right_wing") {
        rw.pose.rotation.z = speed.cos() * std::f32::consts::PI * 0.15;
    }
    if let Some(lw) = tree.part_mut("left_wing") {
        lw.pose.rotation.z = -speed.cos() * std::f32::consts::PI * 0.15;
    }
}

pub fn build_bee_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_bee_model_tree();
    tree.reset_pose();
    apply_bee_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
