use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_wither_model_tree() -> JavaModelPartTree {
    let ribcage_x_rot = 0.20420352_f32;
    let tail_y = 6.9 + ribcage_x_rot.cos() * 10.0;
    let tail_z = -0.5 + ribcage_x_rot.sin() * 10.0;

    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(NamedPartSpec::new("shoulders", PartPose::ZERO).cube(cube(
            [0.0, 16.0],
            [-10.0, 3.9, -0.5],
            [20.0, 3.0, 3.0],
        )))
        .child(
            NamedPartSpec::new(
                "ribcage",
                PartPose::offset_and_rotation(-2.0, 6.9, -0.5, ribcage_x_rot, 0.0, 0.0),
            )
            .cube(cube([0.0, 22.0], [0.0, 0.0, 0.0], [3.0, 10.0, 3.0]))
            .cube(cube([24.0, 22.0], [-4.0, 1.5, 0.5], [11.0, 2.0, 2.0]))
            .cube(cube([24.0, 22.0], [-4.0, 4.0, 0.5], [11.0, 2.0, 2.0]))
            .cube(cube([24.0, 22.0], [-4.0, 6.5, 0.5], [11.0, 2.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "tail",
                PartPose::offset_and_rotation(-2.0, tail_y, tail_z, 0.83252203, 0.0, 0.0),
            )
            .cube(cube([12.0, 22.0], [0.0, 0.0, 0.0], [3.0, 6.0, 3.0])),
        )
        .child(NamedPartSpec::new("center_head", PartPose::ZERO).cube(cube(
            [0.0, 0.0],
            [-4.0, -4.0, -4.0],
            [8.0, 8.0, 8.0],
        )))
        .child(
            NamedPartSpec::new("right_head", PartPose::offset(-8.0, 4.0, 0.0)).cube(cube(
                [32.0, 0.0],
                [-4.0, -4.0, -4.0],
                [6.0, 6.0, 6.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_head", PartPose::offset(10.0, 4.0, 0.0)).cube(cube(
                [32.0, 0.0],
                [-4.0, -4.0, -4.0],
                [6.0, 6.0, 6.0],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_wither_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_wither_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

pub fn apply_wither_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let deg2rad = std::f32::consts::PI / 180.0;
    let anim = (state.age_in_ticks * 0.1).cos();

    let ribcage_x_rot = (0.065 + 0.05 * anim) * std::f32::consts::PI;
    if let Some(r) = tree.part_mut("ribcage") {
        r.pose.rotation.x = ribcage_x_rot;
    }
    if let Some(t) = tree.part_mut("tail") {
        t.pose.offset.y = 6.9 + ribcage_x_rot.cos() * 10.0;
        t.pose.offset.z = -0.5 + ribcage_x_rot.sin() * 10.0;
        t.pose.rotation.x = (0.265 + 0.1 * anim) * std::f32::consts::PI;
    }

    if let Some(ch) = tree.part_mut("center_head") {
        ch.pose.rotation.y = state.y_rot_degrees * deg2rad;
        ch.pose.rotation.x = state.x_rot_degrees * deg2rad;
    }
}

pub fn build_wither_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_wither_model_tree();
    tree.reset_pose();
    apply_wither_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
