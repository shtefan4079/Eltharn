use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_shulker_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("lid", PartPose::offset(0.0, 24.0, 0.0)).cube(cube(
                [0.0, 0.0],
                [-8.0, -16.0, -8.0],
                [16.0, 12.0, 16.0],
            )),
        )
        .child(
            NamedPartSpec::new("base", PartPose::offset(0.0, 24.0, 0.0)).cube(cube(
                [0.0, 28.0],
                [-8.0, -8.0, -8.0],
                [16.0, 8.0, 16.0],
            )),
        )
        .child(
            NamedPartSpec::new("head", PartPose::offset(0.0, 12.0, 0.0)).cube(cube(
                [0.0, 52.0],
                [-3.0, 0.0, -3.0],
                [6.0, 6.0, 6.0],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_shulker_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_shulker_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

pub fn apply_shulker_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let deg2rad = std::f32::consts::PI / 180.0;
    let peek_amount = 0.0_f32;
    let bs = (0.5 + peek_amount) * std::f32::consts::PI;
    let q = -1.0 + bs.sin();
    let extra = if bs > std::f32::consts::PI {
        (state.age_in_ticks * 0.1).sin() * 0.7
    } else {
        0.0
    };

    if let Some(lid) = tree.part_mut("lid") {
        lid.pose.offset.y = 16.0 + bs.sin() * 8.0 + extra;
        if peek_amount > 0.3 {
            lid.pose.rotation.y = q * q * q * q * std::f32::consts::PI * 0.125;
        } else {
            lid.pose.rotation.y = 0.0;
        }
    }

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * deg2rad;
        h.pose.rotation.y = (state.y_head_rot - 180.0 - state.y_body_rot) * deg2rad;
    }
}

pub fn build_shulker_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_shulker_model_tree();
    tree.reset_pose();
    apply_shulker_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
