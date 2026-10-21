use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_salmon_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body_front", PartPose::offset(0.0, 20.0, -7.2))
                .cube(cube([0.0, 0.0], [-1.5, -2.5, 0.0], [3.0, 5.0, 8.0]))
                .child(
                    NamedPartSpec::new("top_front_fin", PartPose::offset(0.0, -4.5, 5.0))
                        .cube(cube([2.0, 1.0], [0.0, 0.0, 0.0], [0.0, 2.0, 3.0])),
                ),
        )
        .child(
            NamedPartSpec::new("body_back", PartPose::offset(0.0, 20.0, 0.8000002))
                .cube(cube([0.0, 13.0], [-1.5, -2.5, 0.0], [3.0, 5.0, 8.0]))
                .child(
                    NamedPartSpec::new("back_fin", PartPose::offset(0.0, 0.0, 8.0)).cube(cube(
                        [20.0, 10.0],
                        [0.0, -2.5, 0.0],
                        [0.0, 5.0, 6.0],
                    )),
                )
                .child(
                    NamedPartSpec::new("top_back_fin", PartPose::offset(0.0, -4.5, -1.0))
                        .cube(cube([0.0, 2.0], [0.0, 0.0, 0.0], [0.0, 2.0, 4.0])),
                ),
        )
        .child(
            NamedPartSpec::new("head", PartPose::offset(0.0, 20.0, -7.2)).cube(cube(
                [22.0, 0.0],
                [-1.0, -2.0, -3.0],
                [2.0, 4.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "right_fin",
                PartPose::offset_and_rotation(-1.5, 21.5, -7.2, 0.0, 0.0, -0.7853982),
            )
            .cube(cube([-4.0, 0.0], [-2.0, 0.0, 0.0], [2.0, 0.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_fin",
                PartPose::offset_and_rotation(1.5, 21.5, -7.2, 0.0, 0.0, 0.7853982),
            )
            .cube(cube([0.0, 0.0], [0.0, 0.0, 0.0], [2.0, 0.0, 2.0])),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_salmon_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_salmon_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}

pub fn apply_salmon_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let (amplitude_multiplier, angle_multiplier) = if state.is_in_water {
        (1.0, 1.0)
    } else {
        (1.3, 1.7)
    };
    if let Some(b) = tree.part_mut("body_back") {
        b.pose.rotation.y =
            -amplitude_multiplier * 0.25 * (angle_multiplier * 0.6 * state.age_in_ticks).sin();
    }
}

pub fn build_salmon_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_salmon_model_tree();
    tree.reset_pose();
    apply_salmon_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
