use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_spider_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("head", PartPose::offset(0.0, 15.0, -3.0)).cube(cube(
                [32.0, 4.0],
                [-4.0, -4.0, -8.0],
                [8.0, 8.0, 8.0],
            )),
        )
        .child(
            NamedPartSpec::new("body0", PartPose::offset(0.0, 15.0, 0.0)).cube(cube(
                [0.0, 0.0],
                [-3.0, -3.0, -3.0],
                [6.0, 6.0, 6.0],
            )),
        )
        .child(
            NamedPartSpec::new("body1", PartPose::offset(0.0, 15.0, 9.0)).cube(cube(
                [0.0, 12.0],
                [-5.0, -4.0, -6.0],
                [10.0, 8.0, 12.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "right_hind_leg",
                PartPose::offset_and_rotation(-4.0, 15.0, 2.0, 0.0, 0.7853982, -0.7853982),
            )
            .cube(cube([18.0, 0.0], [-15.0, -1.0, -1.0], [16.0, 2.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_hind_leg",
                PartPose::offset_and_rotation(4.0, 15.0, 2.0, 0.0, -0.7853982, 0.7853982),
            )
            .cube(cube_mirrored(
                [18.0, 0.0],
                [-1.0, -1.0, -1.0],
                [16.0, 2.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "right_middle_hind_leg",
                PartPose::offset_and_rotation(-4.0, 15.0, 1.0, 0.0, 0.3926991, -0.58119464),
            )
            .cube(cube([18.0, 0.0], [-15.0, -1.0, -1.0], [16.0, 2.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_middle_hind_leg",
                PartPose::offset_and_rotation(4.0, 15.0, 1.0, 0.0, -0.3926991, 0.58119464),
            )
            .cube(cube_mirrored(
                [18.0, 0.0],
                [-1.0, -1.0, -1.0],
                [16.0, 2.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "right_middle_front_leg",
                PartPose::offset_and_rotation(-4.0, 15.0, 0.0, 0.0, -0.3926991, -0.58119464),
            )
            .cube(cube([18.0, 0.0], [-15.0, -1.0, -1.0], [16.0, 2.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_middle_front_leg",
                PartPose::offset_and_rotation(4.0, 15.0, 0.0, 0.0, 0.3926991, 0.58119464),
            )
            .cube(cube_mirrored(
                [18.0, 0.0],
                [-1.0, -1.0, -1.0],
                [16.0, 2.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "right_front_leg",
                PartPose::offset_and_rotation(-4.0, 15.0, -1.0, 0.0, -0.7853982, -0.7853982),
            )
            .cube(cube([18.0, 0.0], [-15.0, -1.0, -1.0], [16.0, 2.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_front_leg",
                PartPose::offset_and_rotation(4.0, 15.0, -1.0, 0.0, 0.7853982, 0.7853982),
            )
            .cube(cube_mirrored(
                [18.0, 0.0],
                [-1.0, -1.0, -1.0],
                [16.0, 2.0, 2.0],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_spider_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_spider_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn apply_spider_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    }

    let anim_pos = state.walk_animation_pos * 0.6662;
    let anim_speed = state.walk_animation_speed;

    let swing_hind = -(anim_pos * 2.0).cos() * 0.4 * anim_speed;
    let swing_middle_hind = -(anim_pos * 2.0 + std::f32::consts::PI).cos() * 0.4 * anim_speed;
    let swing_middle_front =
        -(anim_pos * 2.0 + std::f32::consts::PI * 0.5).cos() * 0.4 * anim_speed;
    let swing_front = -(anim_pos * 2.0 + std::f32::consts::PI * 1.5).cos() * 0.4 * anim_speed;

    let step_hind = (anim_pos).sin().abs() * 0.4 * anim_speed;
    let step_middle_hind = (anim_pos + std::f32::consts::PI).sin().abs() * 0.4 * anim_speed;
    let step_middle_front = (anim_pos + std::f32::consts::PI * 0.5).sin().abs() * 0.4 * anim_speed;
    let step_front = (anim_pos + std::f32::consts::PI * 1.5).sin().abs() * 0.4 * anim_speed;

    if let Some(l) = tree.part_mut("right_hind_leg") {
        l.pose.rotation.y = 0.7853982 + swing_hind;
        l.pose.rotation.z = -0.7853982 + step_hind;
    }
    if let Some(l) = tree.part_mut("left_hind_leg") {
        l.pose.rotation.y = -0.7853982 - swing_hind;
        l.pose.rotation.z = 0.7853982 - step_hind;
    }
    if let Some(l) = tree.part_mut("right_middle_hind_leg") {
        l.pose.rotation.y = 0.3926991 + swing_middle_hind;
        l.pose.rotation.z = -0.58119464 + step_middle_hind;
    }
    if let Some(l) = tree.part_mut("left_middle_hind_leg") {
        l.pose.rotation.y = -0.3926991 - swing_middle_hind;
        l.pose.rotation.z = 0.58119464 - step_middle_hind;
    }
    if let Some(l) = tree.part_mut("right_middle_front_leg") {
        l.pose.rotation.y = -0.3926991 + swing_middle_front;
        l.pose.rotation.z = -0.58119464 + step_middle_front;
    }
    if let Some(l) = tree.part_mut("left_middle_front_leg") {
        l.pose.rotation.y = 0.3926991 - swing_middle_front;
        l.pose.rotation.z = 0.58119464 - step_middle_front;
    }
    if let Some(l) = tree.part_mut("right_front_leg") {
        l.pose.rotation.y = -0.7853982 + swing_front;
        l.pose.rotation.z = -0.7853982 + step_front;
    }
    if let Some(l) = tree.part_mut("left_front_leg") {
        l.pose.rotation.y = 0.7853982 - swing_front;
        l.pose.rotation.z = 0.7853982 - step_front;
    }
}

pub fn build_spider_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_spider_model_tree();
    tree.reset_pose();
    apply_spider_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
