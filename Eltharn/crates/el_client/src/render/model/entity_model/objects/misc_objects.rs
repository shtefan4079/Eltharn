use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_leash_knot_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("knot", PartPose::ZERO).cube(cube(
            [0.0, 0.0],
            [-3.0, -8.0, -3.0],
            [6.0, 8.0, 6.0],
        )),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_leash_knot_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_leash_knot_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}

pub fn apply_leash_knot_model_setup_anim(
    _tree: &mut JavaModelPartTree,
    _state: EntityModelAnimationState,
) {
}

pub fn build_leash_knot_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_leash_knot_model_tree();
    tree.reset_pose();
    apply_leash_knot_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}

pub fn build_llama_spit_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("main", PartPose::ZERO)
            .cube(cube([0.0, 0.0], [-4.0, 0.0, 0.0], [2.0, 2.0, 2.0]))
            .cube(cube([0.0, 0.0], [0.0, -4.0, 0.0], [2.0, 2.0, 2.0]))
            .cube(cube([0.0, 0.0], [0.0, 0.0, -4.0], [2.0, 2.0, 2.0]))
            .cube(cube([0.0, 0.0], [0.0, 0.0, 0.0], [2.0, 2.0, 2.0]))
            .cube(cube([0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 2.0, 2.0]))
            .cube(cube([0.0, 0.0], [0.0, 2.0, 0.0], [2.0, 2.0, 2.0]))
            .cube(cube([0.0, 0.0], [0.0, 0.0, 2.0], [2.0, 2.0, 2.0])),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_llama_spit_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_llama_spit_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn apply_llama_spit_model_setup_anim(
    _tree: &mut JavaModelPartTree,
    _state: EntityModelAnimationState,
) {
}

pub fn build_llama_spit_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_llama_spit_model_tree();
    tree.reset_pose();
    apply_llama_spit_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
