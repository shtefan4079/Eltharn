use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_trident_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("pole", PartPose::ZERO)
            .cube(cube([0.0, 6.0], [-0.5, 2.0, -0.5], [1.0, 25.0, 1.0]))
            .child(NamedPartSpec::new("base", PartPose::ZERO).cube(cube(
                [4.0, 0.0],
                [-1.5, 0.0, -0.5],
                [3.0, 2.0, 1.0],
            )))
            .child(NamedPartSpec::new("left_spike", PartPose::ZERO).cube(cube(
                [4.0, 3.0],
                [-2.5, -3.0, -0.5],
                [1.0, 4.0, 1.0],
            )))
            .child(
                NamedPartSpec::new("middle_spike", PartPose::ZERO).cube(cube(
                    [0.0, 0.0],
                    [-0.5, -4.0, -0.5],
                    [1.0, 4.0, 1.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_spike", PartPose::ZERO).cube(cube_mirrored(
                    [4.0, 3.0],
                    [1.5, -3.0, -0.5],
                    [1.0, 4.0, 1.0],
                )),
            ),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_trident_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_trident_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}

pub fn apply_trident_model_setup_anim(
    _tree: &mut JavaModelPartTree,
    _state: EntityModelAnimationState,
) {
}

pub fn build_trident_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_trident_model_tree();
    tree.reset_pose();
    apply_trident_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
