use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;
use glam::Vec3;

pub fn build_polar_bear_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new(
        "root",
        PartPose {
            offset: Vec3::new(0.0, -4.8032, 0.0),
            rotation: Vec3::splat(0.0),
            scale: Vec3::splat(1.2),
        },
    )
    .child(
        NamedPartSpec::new("head", PartPose::offset(0.0, 10.0, -16.0))
            .cube(cube([0.0, 0.0], [-3.5, -3.0, -3.0], [7.0, 7.0, 7.0]))
            .cube(cube([0.0, 44.0], [-2.5, 1.0, -6.0], [5.0, 3.0, 3.0]))
            .cube(cube([26.0, 0.0], [-4.5, -4.0, -1.0], [2.0, 2.0, 1.0]))
            .cube(cube_mirrored(
                [26.0, 0.0],
                [2.5, -4.0, -1.0],
                [2.0, 2.0, 1.0],
            )),
    )
    .child(
        NamedPartSpec::new(
            "body",
            PartPose::offset_and_rotation(-2.0, 9.0, 12.0, 1.570796, 0.0, 0.0),
        )
        .cube(cube([0.0, 19.0], [-5.0, -13.0, -7.0], [14.0, 14.0, 11.0]))
        .cube(cube([39.0, 0.0], [-4.0, -25.0, -7.0], [12.0, 12.0, 10.0])),
    )
    .child(
        NamedPartSpec::new("right_hind_leg", PartPose::offset(-4.5, 14.0, 6.0)).cube(cube(
            [50.0, 22.0],
            [-2.0, 0.0, -2.0],
            [4.0, 10.0, 8.0],
        )),
    )
    .child(
        NamedPartSpec::new("left_hind_leg", PartPose::offset(4.5, 14.0, 6.0)).cube(cube(
            [50.0, 22.0],
            [-2.0, 0.0, -2.0],
            [4.0, 10.0, 8.0],
        )),
    )
    .child(
        NamedPartSpec::new("right_front_leg", PartPose::offset(-3.5, 14.0, -8.0)).cube(cube(
            [50.0, 40.0],
            [-2.0, 0.0, -2.0],
            [4.0, 10.0, 6.0],
        )),
    )
    .child(
        NamedPartSpec::new("left_front_leg", PartPose::offset(3.5, 14.0, -8.0)).cube(cube(
            [50.0, 40.0],
            [-2.0, 0.0, -2.0],
            [4.0, 10.0, 6.0],
        )),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_polar_bear_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_polar_bear_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}

pub fn apply_polar_bear_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    }
    let leg_rot = (anim_pos * 0.6662).cos() * 1.4 * anim_speed;
    if let Some(l) = tree.part_mut("right_hind_leg") {
        l.pose.rotation.x = leg_rot;
    }
    if let Some(l) = tree.part_mut("left_hind_leg") {
        l.pose.rotation.x = -leg_rot;
    }
    if let Some(l) = tree.part_mut("right_front_leg") {
        l.pose.rotation.x = -leg_rot;
    }
    if let Some(l) = tree.part_mut("left_front_leg") {
        l.pose.rotation.x = leg_rot;
    }

    let stand_scale = state.stand_scale * state.stand_scale;
    let body_age_scale = state.age_scale;
    let anim_value_0 = stand_scale * std::f32::consts::PI * 0.35;
    tree.part_mut_or_panic("body").pose.rotation.x -= anim_value_0;
    let anim_value_1 = stand_scale * body_age_scale * 2.0;
    tree.part_mut_or_panic("body").pose.offset.y += anim_value_1;
    let anim_value_2 = stand_scale * body_age_scale * 20.0;
    tree.part_mut_or_panic("right_front_leg").pose.offset.y -= anim_value_2;
    let anim_value_3 = stand_scale * body_age_scale * 4.0;
    tree.part_mut_or_panic("right_front_leg").pose.offset.z += anim_value_3;
    let anim_value_4 = stand_scale * std::f32::consts::PI * 0.45;
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x -= anim_value_4;
    let anim_value_5 = tree.part_or_panic("right_front_leg").pose.offset.y;
    tree.part_mut_or_panic("left_front_leg").pose.offset.y = anim_value_5;
    let anim_value_6 = tree.part_or_panic("right_front_leg").pose.offset.z;
    tree.part_mut_or_panic("left_front_leg").pose.offset.z = anim_value_6;
    let anim_value_7 = stand_scale * std::f32::consts::PI * 0.45;
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x -= anim_value_7;
    let anim_value_8 = stand_scale * 24.0;
    tree.part_mut_or_panic("head").pose.offset.y -= anim_value_8;
    let anim_value_9 = stand_scale * 13.0;
    tree.part_mut_or_panic("head").pose.offset.z += anim_value_9;
    let anim_value_10 = stand_scale * std::f32::consts::PI * 0.15;
    tree.part_mut_or_panic("head").pose.rotation.x += anim_value_10;
}

pub fn build_polar_bear_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_polar_bear_model_tree();
    tree.reset_pose();
    apply_polar_bear_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
