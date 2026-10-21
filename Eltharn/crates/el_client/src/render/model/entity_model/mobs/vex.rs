use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_vex_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::offset(0.0, -2.5, 0.0))
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 20.0, 0.0)).cube(cube(
                    [0.0, 0.0],
                    [-2.5, -5.0, -2.5],
                    [5.0, 5.0, 5.0],
                )),
            )
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, 20.0, 0.0))
                    .cube(cube([0.0, 10.0], [-1.5, 0.0, -1.0], [3.0, 4.0, 2.0]))
                    .cube(cube_grown(
                        [0.0, 16.0],
                        [-1.5, 1.0, -1.0],
                        [3.0, 5.0, 2.0],
                        -0.2,
                    ))
                    .child(
                        NamedPartSpec::new("right_arm", PartPose::offset(-1.75, 0.25, 0.0)).cube(
                            cube_grown([23.0, 0.0], [-1.25, -0.5, -1.0], [2.0, 4.0, 2.0], -0.1),
                        ),
                    )
                    .child(
                        NamedPartSpec::new("left_arm", PartPose::offset(1.75, 0.25, 0.0)).cube(
                            cube_grown([23.0, 6.0], [-0.75, -0.5, -1.0], [2.0, 4.0, 2.0], -0.1),
                        ),
                    )
                    .child(
                        NamedPartSpec::new("left_wing", PartPose::offset(0.5, 1.0, 1.0)).cube(
                            cube_mirrored([16.0, 14.0], [0.0, 0.0, 0.0], [0.0, 5.0, 8.0]),
                        ),
                    )
                    .child(
                        NamedPartSpec::new("right_wing", PartPose::offset(-0.5, 1.0, 1.0))
                            .cube(cube([16.0, 14.0], [0.0, 0.0, 0.0], [0.0, 5.0, 8.0])),
                    ),
            );

    let container = NamedPartSpec::new("container", PartPose::ZERO).child(root);
    JavaModelPartTree::from_root(container)
}

pub fn build_vex_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_vex_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}

pub fn apply_vex_model_setup_anim(tree: &mut JavaModelPartTree, state: EntityModelAnimationState) {
    let deg2rad = std::f32::consts::PI / 180.0;
    let age = state.age_in_ticks;

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.y = state.y_rot_degrees * deg2rad;
        h.pose.rotation.x = state.x_rot_degrees * deg2rad;
    }

    let moving_arm_z_bob = (age * 5.5 * deg2rad).cos() * 0.1;
    if let Some(ra) = tree.part_mut("right_arm") {
        ra.pose.rotation.z = std::f32::consts::PI / 5.0 + moving_arm_z_bob;
    }
    if let Some(la) = tree.part_mut("left_arm") {
        la.pose.rotation.z = -(std::f32::consts::PI / 5.0 + moving_arm_z_bob);
    }

    if let Some(b) = tree.part_mut("body") {
        b.pose.rotation.x = std::f32::consts::PI / 20.0;
    }

    let left_wing_y = 1.0995574 + (age * 45.836624 * deg2rad).cos() * deg2rad * 16.2;
    if let Some(lw) = tree.part_mut("left_wing") {
        lw.pose.rotation.y = left_wing_y;
        lw.pose.rotation.x = 0.47123888;
        lw.pose.rotation.z = -0.47123888;
    }
    if let Some(rw) = tree.part_mut("right_wing") {
        rw.pose.rotation.y = -left_wing_y;
        rw.pose.rotation.x = 0.47123888;
        rw.pose.rotation.z = 0.47123888;
    }
}

pub fn build_vex_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_vex_model_tree();
    tree.reset_pose();
    apply_vex_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
