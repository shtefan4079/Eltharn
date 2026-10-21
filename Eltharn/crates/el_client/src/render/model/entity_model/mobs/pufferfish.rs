use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_pufferfish_model_tree() -> JavaModelPartTree {
    let pi = std::f32::consts::PI;
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, 22.0, 0.0)).cube(cube(
                    [0.0, 0.0],
                    [-4.0, -8.0, -4.0],
                    [8.0, 8.0, 8.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_blue_fin", PartPose::offset(-4.0, 15.0, -2.0))
                    .cube(cube([24.0, 0.0], [-2.0, 0.0, -1.0], [2.0, 1.0, 2.0])),
            )
            .child(
                NamedPartSpec::new("left_blue_fin", PartPose::offset(4.0, 15.0, -2.0)).cube(cube(
                    [24.0, 3.0],
                    [0.0, 0.0, -1.0],
                    [2.0, 1.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new(
                    "top_front_fin",
                    PartPose::offset_and_rotation(0.0, 14.0, -4.0, pi / 4.0, 0.0, 0.0),
                )
                .cube(cube([15.0, 17.0], [-4.0, -1.0, 0.0], [8.0, 1.0, 0.0])),
            )
            .child(
                NamedPartSpec::new("top_middle_fin", PartPose::offset(0.0, 14.0, 0.0)).cube(cube(
                    [14.0, 16.0],
                    [-4.0, -1.0, 0.0],
                    [8.0, 1.0, 1.0],
                )),
            )
            .child(
                NamedPartSpec::new(
                    "top_back_fin",
                    PartPose::offset_and_rotation(0.0, 14.0, 4.0, -pi / 4.0, 0.0, 0.0),
                )
                .cube(cube([23.0, 18.0], [-4.0, -1.0, 0.0], [8.0, 1.0, 0.0])),
            )
            .child(
                NamedPartSpec::new(
                    "right_front_fin",
                    PartPose::offset_and_rotation(-4.0, 22.0, -4.0, 0.0, -pi / 4.0, 0.0),
                )
                .cube(cube([5.0, 17.0], [-1.0, -8.0, 0.0], [1.0, 8.0, 0.0])),
            )
            .child(
                NamedPartSpec::new(
                    "left_front_fin",
                    PartPose::offset_and_rotation(4.0, 22.0, -4.0, 0.0, pi / 4.0, 0.0),
                )
                .cube(cube([1.0, 17.0], [0.0, -8.0, 0.0], [1.0, 8.0, 0.0])),
            )
            .child(
                NamedPartSpec::new(
                    "bottom_front_fin",
                    PartPose::offset_and_rotation(0.0, 22.0, -4.0, -pi / 4.0, 0.0, 0.0),
                )
                .cube(cube([15.0, 20.0], [-4.0, 0.0, 0.0], [8.0, 1.0, 0.0])),
            )
            .child(
                NamedPartSpec::new("bottom_middle_fin", PartPose::offset(0.0, 22.0, 0.0))
                    .cube(cube([15.0, 20.0], [-4.0, 0.0, 0.0], [8.0, 1.0, 0.0])),
            )
            .child(
                NamedPartSpec::new(
                    "bottom_back_fin",
                    PartPose::offset_and_rotation(0.0, 22.0, 4.0, pi / 4.0, 0.0, 0.0),
                )
                .cube(cube([15.0, 20.0], [-4.0, 0.0, 0.0], [8.0, 1.0, 0.0])),
            )
            .child(
                NamedPartSpec::new(
                    "right_back_fin",
                    PartPose::offset_and_rotation(-4.0, 22.0, 4.0, 0.0, pi / 4.0, 0.0),
                )
                .cube(cube([9.0, 17.0], [-1.0, -8.0, 0.0], [1.0, 8.0, 0.0])),
            )
            .child(
                NamedPartSpec::new(
                    "left_back_fin",
                    PartPose::offset_and_rotation(4.0, 22.0, 4.0, 0.0, -pi / 4.0, 0.0),
                )
                .cube(cube([9.0, 17.0], [0.0, -8.0, 0.0], [1.0, 8.0, 0.0])),
            );

    JavaModelPartTree::from_root(root)
}

pub fn build_pufferfish_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_pufferfish_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}

pub fn apply_pufferfish_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    if let Some(r) = tree.part_mut("right_blue_fin") {
        r.pose.rotation.z = -0.2 + 0.4 * (state.age_in_ticks * 0.2).sin();
    }
    if let Some(l) = tree.part_mut("left_blue_fin") {
        l.pose.rotation.z = 0.2 - 0.4 * (state.age_in_ticks * 0.2).sin();
    }
}

pub fn build_pufferfish_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_pufferfish_model_tree();
    tree.reset_pose();
    apply_pufferfish_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
