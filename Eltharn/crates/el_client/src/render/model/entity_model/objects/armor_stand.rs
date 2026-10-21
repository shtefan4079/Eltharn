use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;
pub fn build_armor_stand_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 1.0, 0.0)).cube(cube(
                    [0.0, 0.0],
                    [-1.0, -7.0, -1.0],
                    [2.0, 7.0, 2.0],
                )),
            )
            .child(NamedPartSpec::new("body", PartPose::ZERO).cube(cube(
                [0.0, 26.0],
                [-6.0, 0.0, -1.5],
                [12.0, 3.0, 3.0],
            )))
            .child(
                NamedPartSpec::new("right_arm", PartPose::offset(-5.0, 2.0, 0.0)).cube(cube(
                    [24.0, 0.0],
                    [-2.0, -2.0, -1.0],
                    [2.0, 12.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_arm", PartPose::offset(5.0, 2.0, 0.0)).cube(
                    cube_mirrored([32.0, 16.0], [0.0, -2.0, -1.0], [2.0, 12.0, 2.0]),
                ),
            )
            .child(
                NamedPartSpec::new("right_leg", PartPose::offset(-1.9, 12.0, 0.0)).cube(cube(
                    [8.0, 0.0],
                    [-1.0, 0.0, -1.0],
                    [2.0, 11.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_leg", PartPose::offset(1.9, 12.0, 0.0)).cube(
                    cube_mirrored([40.0, 16.0], [-1.0, 0.0, -1.0], [2.0, 11.0, 2.0]),
                ),
            )
            .child(
                NamedPartSpec::new("right_body_stick", PartPose::ZERO).cube(cube(
                    [16.0, 0.0],
                    [-3.0, 3.0, -1.0],
                    [2.0, 7.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_body_stick", PartPose::ZERO).cube(cube(
                    [48.0, 16.0],
                    [1.0, 3.0, -1.0],
                    [2.0, 7.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("shoulder_stick", PartPose::ZERO).cube(cube(
                    [0.0, 48.0],
                    [-4.0, 10.0, -1.0],
                    [8.0, 2.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("base_plate", PartPose::offset(0.0, 12.0, 0.0)).cube(cube(
                    [0.0, 32.0],
                    [-6.0, 11.0, -6.0],
                    [12.0, 1.0, 12.0],
                )),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_armor_stand_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_armor_stand_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_armor_stand_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let deg2rad = std::f32::consts::PI / 180.0;
    let rotation = |pose: glam::Vec3| pose * deg2rad;
    if let Some(head) = tree.part_mut("head") {
        head.set_rotation(
            rotation(state.armor_stand_head_pose).x,
            rotation(state.armor_stand_head_pose).y,
            rotation(state.armor_stand_head_pose).z,
        );
    }
    if let Some(body) = tree.part_mut("body") {
        body.set_rotation(
            rotation(state.armor_stand_body_pose).x,
            rotation(state.armor_stand_body_pose).y,
            rotation(state.armor_stand_body_pose).z,
        );
    }
    if let Some(left_arm) = tree.part_mut("left_arm") {
        let pose = rotation(state.armor_stand_left_arm_pose);
        left_arm.set_rotation(pose.x, pose.y, pose.z);
        left_arm.visible = state.show_arms;
    }
    if let Some(right_arm) = tree.part_mut("right_arm") {
        let pose = rotation(state.armor_stand_right_arm_pose);
        right_arm.set_rotation(pose.x, pose.y, pose.z);
        right_arm.visible = state.show_arms;
    }
    if let Some(left_leg) = tree.part_mut("left_leg") {
        let pose = rotation(state.armor_stand_left_leg_pose);
        left_leg.set_rotation(pose.x, pose.y, pose.z);
    }
    if let Some(right_leg) = tree.part_mut("right_leg") {
        let pose = rotation(state.armor_stand_right_leg_pose);
        right_leg.set_rotation(pose.x, pose.y, pose.z);
    }
    let body_pose = rotation(state.armor_stand_body_pose);
    for name in ["right_body_stick", "left_body_stick", "shoulder_stick"] {
        if let Some(part) = tree.part_mut(name) {
            part.set_rotation(body_pose.x, body_pose.y, body_pose.z);
        }
    }
    if let Some(bp) = tree.part_mut("base_plate") {
        bp.pose.rotation.y = -state.y_rot_degrees * deg2rad;
        bp.visible = state.show_base_plate;
    }
}
pub fn build_armor_stand_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_armor_stand_model_tree();
    tree.reset_pose();
    if state.armor_stand_is_small {
        let top_level = tree
            .nodes
            .first()
            .map(|root| root.children.clone())
            .unwrap_or_default();
        for index in top_level {
            if let Some(part) = tree.nodes.get_mut(index) {
                let (y, scale) = if part.name == "head" {
                    (16.0, 0.75)
                } else {
                    (24.0, 0.5)
                };
                part.pose = part.pose.translated(0.0, y, 0.0).scaled(scale);
            }
        }
    }
    apply_armor_stand_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
