use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_warden_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("bone", PartPose::offset(0.0, 24.0, 0.0))
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, -21.0, 0.0))
                    .cube(cube([0.0, 0.0], [-9.0, -13.0, -4.0], [18.0, 21.0, 11.0]))
                    .child(
                        NamedPartSpec::new("right_ribcage", PartPose::offset(-7.0, -2.0, -4.0))
                            .cube(cube([90.0, 11.0], [-2.0, -11.0, -0.1], [9.0, 21.0, 0.0])),
                    )
                    .child(
                        NamedPartSpec::new("left_ribcage", PartPose::offset(7.0, -2.0, -4.0)).cube(
                            cube_mirrored([90.0, 11.0], [-7.0, -11.0, -0.1], [9.0, 21.0, 0.0]),
                        ),
                    )
                    .child(
                        NamedPartSpec::new("head", PartPose::offset(0.0, -13.0, 0.0))
                            .cube(cube([0.0, 32.0], [-8.0, -16.0, -5.0], [16.0, 16.0, 10.0]))
                            .child(
                                NamedPartSpec::new(
                                    "right_tendril",
                                    PartPose::offset(-8.0, -12.0, 0.0),
                                )
                                .cube(cube(
                                    [52.0, 32.0],
                                    [-16.0, -13.0, 0.0],
                                    [16.0, 16.0, 0.0],
                                )),
                            )
                            .child(
                                NamedPartSpec::new(
                                    "left_tendril",
                                    PartPose::offset(8.0, -12.0, 0.0),
                                )
                                .cube(cube(
                                    [58.0, 0.0],
                                    [0.0, -13.0, 0.0],
                                    [16.0, 16.0, 0.0],
                                )),
                            ),
                    )
                    .child(
                        NamedPartSpec::new("right_arm", PartPose::offset(-13.0, -13.0, 1.0))
                            .cube(cube([44.0, 50.0], [-4.0, 0.0, -4.0], [8.0, 28.0, 8.0])),
                    )
                    .child(
                        NamedPartSpec::new("left_arm", PartPose::offset(13.0, -13.0, 1.0))
                            .cube(cube([0.0, 58.0], [-4.0, 0.0, -4.0], [8.0, 28.0, 8.0])),
                    ),
            )
            .child(
                NamedPartSpec::new("right_leg", PartPose::offset(-5.9, -13.0, 0.0)).cube(cube(
                    [76.0, 48.0],
                    [-3.1, 0.0, -3.0],
                    [6.0, 13.0, 6.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_leg", PartPose::offset(5.9, -13.0, 0.0)).cube(cube(
                    [76.0, 76.0],
                    [-2.9, 0.0, -3.0],
                    [6.0, 13.0, 6.0],
                )),
            ),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_warden_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_warden_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}

pub fn apply_warden_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;
    let age = state.age_in_ticks;

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    }

    let speed_modifier = (3.0 * anim_speed).min(0.5);
    let adjusted_pos = anim_pos * 0.8662;
    let adjusted_pos_cos = adjusted_pos.cos();
    let adjusted_pos_sin = adjusted_pos.sin();
    let speed_modifier_min = speed_modifier.min(0.35);

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.z += 0.3 * adjusted_pos_sin * speed_modifier;
        h.pose.rotation.x +=
            1.2 * (adjusted_pos + std::f32::consts::PI * 0.5).cos() * speed_modifier_min;
    }
    if let Some(b) = tree.part_mut("body") {
        b.pose.rotation.z = 0.1 * adjusted_pos_sin * speed_modifier;
        b.pose.rotation.x = 1.0 * adjusted_pos_cos * speed_modifier_min;
    }
    if let Some(l) = tree.part_mut("left_leg") {
        l.pose.rotation.x = 1.0 * adjusted_pos_cos * speed_modifier;
    }
    if let Some(l) = tree.part_mut("right_leg") {
        l.pose.rotation.x = 1.0 * (adjusted_pos + std::f32::consts::PI).cos() * speed_modifier;
    }
    if let Some(a) = tree.part_mut("left_arm") {
        a.pose.rotation.x = -(0.8 * adjusted_pos_cos * speed_modifier);
        a.pose.rotation.z = 0.0;
        a.pose.rotation.y = 0.0;
        a.pose.offset.x = 13.0;
        a.pose.offset.y = -13.0;
        a.pose.offset.z = 1.0;
    }
    if let Some(a) = tree.part_mut("right_arm") {
        a.pose.rotation.x = -(0.8 * adjusted_pos_sin * speed_modifier);
        a.pose.rotation.z = 0.0;
        a.pose.rotation.y = 0.0;
        a.pose.offset.x = -13.0;
        a.pose.offset.y = -13.0;
        a.pose.offset.z = 1.0;
    }

    let scaled_age = age * 0.1;
    let wobble_cos = scaled_age.cos();
    let wobble_sin = scaled_age.sin();
    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.z += 0.06 * wobble_cos;
        h.pose.rotation.x += 0.06 * wobble_sin;
    }
    if let Some(b) = tree.part_mut("body") {
        b.pose.rotation.z += 0.025 * wobble_sin;
        b.pose.rotation.x += 0.025 * wobble_cos;
    }
}

pub fn build_warden_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_warden_model_tree();
    tree.reset_pose();
    apply_warden_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
