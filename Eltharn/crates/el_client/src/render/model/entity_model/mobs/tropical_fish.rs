#![allow(dead_code)]
use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;
pub fn build_tropical_fish_small_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 22.0, 0.0)).cube(cube(
                [0.0, 0.0],
                [-1.0, -1.5, -3.0],
                [2.0, 3.0, 6.0],
            )),
        )
        .child(
            NamedPartSpec::new("tail", PartPose::offset(0.0, 22.0, 3.0)).cube(cube(
                [22.0, -6.0],
                [0.0, -1.5, 0.0],
                [0.0, 3.0, 6.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "right_fin",
                PartPose::offset_and_rotation(-1.0, 22.5, 0.0, 0.0, 0.7853982, 0.0),
            )
            .cube(cube([2.0, 16.0], [-2.0, -1.0, 0.0], [2.0, 2.0, 0.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_fin",
                PartPose::offset_and_rotation(1.0, 22.5, 0.0, 0.0, -0.7853982, 0.0),
            )
            .cube(cube([2.0, 12.0], [0.0, -1.0, 0.0], [2.0, 2.0, 0.0])),
        )
        .child(
            NamedPartSpec::new("top_fin", PartPose::offset(0.0, 20.5, -3.0)).cube(cube(
                [10.0, -5.0],
                [0.0, -3.0, 0.0],
                [0.0, 3.0, 6.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_tropical_fish_large_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 19.0, 0.0)).cube(cube(
                [0.0, 20.0],
                [-1.0, -3.0, -3.0],
                [2.0, 6.0, 6.0],
            )),
        )
        .child(
            NamedPartSpec::new("tail", PartPose::offset(0.0, 19.0, 3.0)).cube(cube(
                [21.0, 16.0],
                [0.0, -3.0, 0.0],
                [0.0, 6.0, 5.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "right_fin",
                PartPose::offset_and_rotation(-1.0, 20.0, 0.0, 0.0, 0.7853982, 0.0),
            )
            .cube(cube([2.0, 16.0], [-2.0, 0.0, 0.0], [2.0, 2.0, 0.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_fin",
                PartPose::offset_and_rotation(1.0, 20.0, 0.0, 0.0, -0.7853982, 0.0),
            )
            .cube(cube([2.0, 12.0], [0.0, 0.0, 0.0], [2.0, 2.0, 0.0])),
        )
        .child(
            NamedPartSpec::new("top_fin", PartPose::offset(0.0, 16.0, -3.0)).cube(cube(
                [20.0, 11.0],
                [0.0, -4.0, 0.0],
                [0.0, 4.0, 6.0],
            )),
        )
        .child(
            NamedPartSpec::new("bottom_fin", PartPose::offset(0.0, 22.0, -3.0)).cube(cube(
                [20.0, 21.0],
                [0.0, 0.0, 0.0],
                [0.0, 4.0, 6.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_tropical_fish_small_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_tropical_fish_small_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
pub fn build_tropical_fish_large_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_tropical_fish_large_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
pub fn apply_tropical_fish_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let amplitude_multiplier = if state.is_in_water { 1.0 } else { 1.5 };
    if let Some(t) = tree.part_mut("tail") {
        t.pose.rotation.y = -amplitude_multiplier * 0.45 * (0.6 * state.age_in_ticks).sin();
    }
}
pub fn build_tropical_fish_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = if state.tropical_fish_large {
        build_tropical_fish_large_model_tree()
    } else {
        build_tropical_fish_small_model_tree()
    };
    tree.reset_pose();
    apply_tropical_fish_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
