use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_minecart_model_tree() -> JavaModelPartTree {
    let pi = std::f32::consts::PI;
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 4.0, 0.0, pi / 2.0, 0.0, 0.0),
            )
            .cube(cube([0.0, 10.0], [-10.0, -8.0, -1.0], [20.0, 16.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(-9.0, 4.0, 0.0, 0.0, pi * 1.5, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(9.0, 4.0, 0.0, 0.0, pi / 2.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left",
                PartPose::offset_and_rotation(0.0, 4.0, -7.0, 0.0, pi, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("right", PartPose::offset(0.0, 4.0, 7.0)).cube(cube(
                [0.0, 0.0],
                [-8.0, -9.0, -1.0],
                [16.0, 8.0, 2.0],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_minecart_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_minecart_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn apply_minecart_model_setup_anim(
    _tree: &mut JavaModelPartTree,
    _state: EntityModelAnimationState,
) {
}

pub fn build_minecart_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_minecart_model_tree();
    tree.reset_pose();
    apply_minecart_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
