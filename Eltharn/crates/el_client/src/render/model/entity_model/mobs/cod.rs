use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_cod_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 22.0, 0.0)).cube(cube(
                [0.0, 0.0],
                [-1.0, -2.0, 0.0],
                [2.0, 4.0, 7.0],
            )),
        )
        .child(
            NamedPartSpec::new("head", PartPose::offset(0.0, 22.0, 0.0)).cube(cube(
                [11.0, 0.0],
                [-1.0, -2.0, -3.0],
                [2.0, 4.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("nose", PartPose::offset(0.0, 22.0, -3.0)).cube(cube(
                [0.0, 0.0],
                [-1.0, -2.0, -1.0],
                [2.0, 3.0, 1.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "right_fin",
                PartPose::offset_and_rotation(-1.0, 23.0, 0.0, 0.0, 0.0, -0.7853982),
            )
            .cube(cube([22.0, 1.0], [-2.0, 0.0, -1.0], [2.0, 0.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_fin",
                PartPose::offset_and_rotation(1.0, 23.0, 0.0, 0.0, 0.0, 0.7853982),
            )
            .cube(cube([22.0, 4.0], [0.0, 0.0, -1.0], [2.0, 0.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("tail_fin", PartPose::offset(0.0, 22.0, 7.0)).cube(cube(
                [22.0, 3.0],
                [0.0, -2.0, 0.0],
                [0.0, 4.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("top_fin", PartPose::offset(0.0, 20.0, 0.0)).cube(cube(
                [20.0, -6.0],
                [0.0, -1.0, -1.0],
                [0.0, 1.0, 6.0],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_cod_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_cod_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}

pub fn apply_cod_model_setup_anim(tree: &mut JavaModelPartTree, state: EntityModelAnimationState) {
    let amplitude_multiplier = if state.is_in_water { 1.0 } else { 1.5 };
    if let Some(t) = tree.part_mut("tail_fin") {
        t.pose.rotation.y = -amplitude_multiplier * 0.45 * (0.6 * state.age_in_ticks).sin();
    }
}

pub fn build_cod_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_cod_model_tree();
    tree.reset_pose();
    apply_cod_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
