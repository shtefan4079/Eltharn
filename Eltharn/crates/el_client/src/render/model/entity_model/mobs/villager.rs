use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_villager_model_tree() -> JavaModelPartTree {
    let pi = std::f32::consts::PI;
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("head", PartPose::ZERO)
                .cube(cube([0.0, 0.0], [-4.0, -10.0, -4.0], [8.0, 10.0, 8.0]))
                .child(
                    NamedPartSpec::new("hat", PartPose::ZERO)
                        .cube(cube_grown(
                            [32.0, 0.0],
                            [-4.0, -10.0, -4.0],
                            [8.0, 10.0, 8.0],
                            0.51,
                        ))
                        .child(
                            NamedPartSpec::new(
                                "hat_rim",
                                PartPose::offset_and_rotation(0.0, 0.0, 0.0, -pi / 2.0, 0.0, 0.0),
                            )
                            .cube(cube(
                                [30.0, 47.0],
                                [-8.0, -8.0, -6.0],
                                [16.0, 16.0, 1.0],
                            )),
                        ),
                )
                .child(
                    NamedPartSpec::new("nose", PartPose::offset(0.0, -2.0, 0.0)).cube(cube(
                        [24.0, 0.0],
                        [-1.0, -1.0, -6.0],
                        [2.0, 4.0, 2.0],
                    )),
                ),
        )
        .child(
            NamedPartSpec::new("body", PartPose::ZERO)
                .cube(cube([16.0, 20.0], [-4.0, 0.0, -3.0], [8.0, 12.0, 6.0]))
                .child(
                    NamedPartSpec::new("jacket", PartPose::ZERO).cube(cube_grown(
                        [0.0, 38.0],
                        [-4.0, 0.0, -3.0],
                        [8.0, 20.0, 6.0],
                        0.5,
                    )),
                ),
        )
        .child(
            NamedPartSpec::new(
                "arms",
                PartPose::offset_and_rotation(0.0, 3.0, -1.0, -0.75, 0.0, 0.0),
            )
            .cube(cube([44.0, 22.0], [-8.0, -2.0, -2.0], [4.0, 8.0, 4.0]))
            .cube(cube_mirrored(
                [44.0, 22.0],
                [4.0, -2.0, -2.0],
                [4.0, 8.0, 4.0],
            ))
            .cube(cube([40.0, 38.0], [-4.0, 2.0, -2.0], [8.0, 4.0, 4.0])),
        )
        .child(
            NamedPartSpec::new("right_leg", PartPose::offset(-2.0, 12.0, 0.0)).cube(cube(
                [0.0, 22.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_leg", PartPose::offset(2.0, 12.0, 0.0)).cube(cube_mirrored(
                [0.0, 22.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_villager_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_villager_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

pub fn apply_villager_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let deg2rad = std::f32::consts::PI / 180.0;
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * deg2rad;
        h.pose.rotation.y = state.y_rot_degrees * deg2rad;
    }
    if let Some(l) = tree.part_mut("right_leg") {
        l.pose.rotation.x = (anim_pos * 0.6662).cos() * 1.4 * anim_speed * 0.5;
    }
    if let Some(l) = tree.part_mut("left_leg") {
        l.pose.rotation.x =
            (anim_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * anim_speed * 0.5;
    }
}

pub fn build_villager_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_villager_model_tree();
    tree.reset_pose();
    apply_villager_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
