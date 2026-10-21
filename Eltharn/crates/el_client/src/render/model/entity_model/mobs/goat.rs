use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;
pub fn build_goat_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(1.0, 14.0, 0.0))
                    .cube(cube([2.0, 61.0], [-6.0, -11.0, -10.0], [3.0, 2.0, 1.0]))
                    .cube(cube_mirrored(
                        [2.0, 61.0],
                        [2.0, -11.0, -10.0],
                        [3.0, 2.0, 1.0],
                    ))
                    .cube(cube_mirrored(
                        [23.0, 52.0],
                        [-0.5, -3.0, -14.0],
                        [0.0, 7.0, 5.0],
                    ))
                    .child(NamedPartSpec::new("left_horn", PartPose::ZERO).cube(cube(
                        [12.0, 55.0],
                        [-0.01, -16.0, -10.0],
                        [2.0, 7.0, 2.0],
                    )))
                    .child(NamedPartSpec::new("right_horn", PartPose::ZERO).cube(cube(
                        [12.0, 55.0],
                        [-2.99, -16.0, -10.0],
                        [2.0, 7.0, 2.0],
                    )))
                    .child(
                        NamedPartSpec::new(
                            "nose",
                            PartPose::offset_and_rotation(0.0, -8.0, -8.0, 0.9599, 0.0, 0.0),
                        )
                        .cube(cube(
                            [34.0, 46.0],
                            [-3.0, -4.0, -8.0],
                            [5.0, 7.0, 10.0],
                        )),
                    ),
            )
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, 24.0, 0.0))
                    .cube(cube([1.0, 1.0], [-4.0, -17.0, -7.0], [9.0, 11.0, 16.0]))
                    .cube(cube([0.0, 28.0], [-5.0, -18.0, -8.0], [11.0, 14.0, 11.0])),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(1.0, 14.0, 4.0)).cube(cube(
                    [36.0, 29.0],
                    [0.0, 4.0, 0.0],
                    [3.0, 6.0, 3.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-3.0, 14.0, 4.0)).cube(cube(
                    [49.0, 29.0],
                    [0.0, 4.0, 0.0],
                    [3.0, 6.0, 3.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(1.0, 14.0, -6.0)).cube(cube(
                    [49.0, 2.0],
                    [0.0, 0.0, 0.0],
                    [3.0, 10.0, 3.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-3.0, 14.0, -6.0))
                    .cube(cube([35.0, 2.0], [0.0, 0.0, 0.0], [3.0, 10.0, 3.0])),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_goat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_goat_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_goat_model_setup_anim(tree: &mut JavaModelPartTree, state: EntityModelAnimationState) {
    let deg2rad = std::f32::consts::PI / 180.0;
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;
    if let Some(lh) = tree.part_mut("left_horn") {
        lh.visible = state.has_left_horn;
    }
    if let Some(rh) = tree.part_mut("right_horn") {
        rh.visible = state.has_right_horn;
    }
    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * deg2rad;
        h.pose.rotation.y = state.y_rot_degrees * deg2rad;
        if state.ramming_x_head_rot != 0.0 {
            h.pose.rotation.x = state.ramming_x_head_rot;
        }
    }
    if let Some(l) = tree.part_mut("left_hind_leg") {
        l.pose.rotation.x = (anim_pos * 0.6662).cos() * 1.4 * anim_speed;
    }
    if let Some(l) = tree.part_mut("right_hind_leg") {
        l.pose.rotation.x = (anim_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * anim_speed;
    }
    if let Some(l) = tree.part_mut("left_front_leg") {
        l.pose.rotation.x = (anim_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * anim_speed;
    }
    if let Some(l) = tree.part_mut("right_front_leg") {
        l.pose.rotation.x = (anim_pos * 0.6662).cos() * 1.4 * anim_speed;
    }
}
pub fn build_goat_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_goat_model_tree();
    tree.reset_pose();
    apply_goat_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
