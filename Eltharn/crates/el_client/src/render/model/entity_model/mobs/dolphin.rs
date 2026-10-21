use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_dolphin_model_tree() -> JavaModelPartTree {
    let pi = std::f32::consts::PI;
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("body", PartPose::offset(0.0, 22.0, -5.0))
            .cube(cube([22.0, 0.0], [-4.0, -7.0, 0.0], [8.0, 7.0, 13.0]))
            .child(
                NamedPartSpec::new(
                    "back_fin",
                    PartPose::offset_and_rotation(0.0, 0.0, 0.0, pi / 3.0, 0.0, 0.0),
                )
                .cube(cube([51.0, 0.0], [-0.5, 0.0, 8.0], [1.0, 4.0, 5.0])),
            )
            .child(
                NamedPartSpec::new(
                    "left_fin",
                    PartPose::offset_and_rotation(2.0, -2.0, 4.0, pi / 3.0, 0.0, pi * 2.0 / 3.0),
                )
                .cube(cube_mirrored(
                    [48.0, 20.0],
                    [-0.5, -4.0, 0.0],
                    [1.0, 4.0, 7.0],
                )),
            )
            .child(
                NamedPartSpec::new(
                    "right_fin",
                    PartPose::offset_and_rotation(-2.0, -2.0, 4.0, pi / 3.0, 0.0, -pi * 2.0 / 3.0),
                )
                .cube(cube([48.0, 20.0], [-0.5, -4.0, 0.0], [1.0, 4.0, 7.0])),
            )
            .child(
                NamedPartSpec::new(
                    "tail",
                    PartPose::offset_and_rotation(0.0, -2.5, 11.0, -0.10471976, 0.0, 0.0),
                )
                .cube(cube([0.0, 19.0], [-2.0, -2.5, 0.0], [4.0, 5.0, 11.0]))
                .child(
                    NamedPartSpec::new("tail_fin", PartPose::offset(0.0, 0.0, 9.0)).cube(cube(
                        [19.0, 20.0],
                        [-5.0, -0.5, 0.0],
                        [10.0, 1.0, 6.0],
                    )),
                ),
            )
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, -4.0, -3.0))
                    .cube(cube([0.0, 0.0], [-4.0, -3.0, -3.0], [8.0, 7.0, 6.0]))
                    .child(NamedPartSpec::new("nose", PartPose::ZERO).cube(cube(
                        [0.0, 13.0],
                        [-1.0, 2.0, -7.0],
                        [2.0, 2.0, 4.0],
                    ))),
            ),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_dolphin_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_dolphin_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

pub fn apply_dolphin_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let deg2rad = std::f32::consts::PI / 180.0;
    let age = state.age_in_ticks;

    if let Some(b) = tree.part_mut("body") {
        b.pose.rotation.x = state.x_rot_degrees * deg2rad + (-0.05 - 0.05 * (age * 0.3).cos());
        b.pose.rotation.y = state.y_rot_degrees * deg2rad;
    }
    if let Some(t) = tree.part_mut("tail") {
        t.pose.rotation.x = -0.1 * (age * 0.3).cos();
    }
    if let Some(tf) = tree.part_mut("tail_fin") {
        tf.pose.rotation.x = -0.2 * (age * 0.3).cos();
    }
}

pub fn build_dolphin_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_dolphin_model_tree();
    tree.reset_pose();
    apply_dolphin_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
