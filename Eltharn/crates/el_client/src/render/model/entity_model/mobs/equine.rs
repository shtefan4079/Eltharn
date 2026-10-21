#![allow(dead_code)]

use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_horse_model_tree() -> JavaModelPartTree {
    let pi = std::f32::consts::PI;
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 11.0, 5.0))
                .cube(cube_grown(
                    [0.0, 32.0],
                    [-5.0, -8.0, -17.0],
                    [10.0, 10.0, 22.0],
                    0.05,
                ))
                .child(
                    NamedPartSpec::new(
                        "tail",
                        PartPose::offset_and_rotation(0.0, -5.0, 2.0, pi / 6.0, 0.0, 0.0),
                    )
                    .cube(cube(
                        [42.0, 36.0],
                        [-1.5, 0.0, 0.0],
                        [3.0, 14.0, 4.0],
                    )),
                ),
        )
        .child(
            NamedPartSpec::new(
                "head_parts",
                PartPose::offset_and_rotation(0.0, 4.0, -12.0, pi / 6.0, 0.0, 0.0),
            )
            .cube(cube([0.0, 35.0], [-2.05, -6.0, -2.0], [4.0, 12.0, 7.0]))
            .child(
                NamedPartSpec::new("head", PartPose::ZERO)
                    .cube(cube([0.0, 13.0], [-3.0, -11.0, -2.0], [6.0, 5.0, 7.0]))
                    .child(
                        NamedPartSpec::new("left_ear", PartPose::ZERO).cube(cube_grown(
                            [19.0, 16.0],
                            [0.55, -13.0, 4.0],
                            [2.0, 3.0, 1.0],
                            -0.001,
                        )),
                    )
                    .child(
                        NamedPartSpec::new("right_ear", PartPose::ZERO).cube(cube_grown(
                            [19.0, 16.0],
                            [-2.55, -13.0, 4.0],
                            [2.0, 3.0, 1.0],
                            -0.001,
                        )),
                    ),
            )
            .child(NamedPartSpec::new("mane", PartPose::ZERO).cube(cube(
                [56.0, 36.0],
                [-1.0, -11.0, 5.01],
                [2.0, 16.0, 2.0],
            )))
            .child(NamedPartSpec::new("upper_mouth", PartPose::ZERO).cube(cube(
                [0.0, 25.0],
                [-2.0, -11.0, -7.0],
                [4.0, 5.0, 5.0],
            ))),
        )
        .child(
            NamedPartSpec::new("left_hind_leg", PartPose::offset(4.0, 14.0, 7.0)).cube(
                cube_mirrored([48.0, 21.0], [-3.0, -1.01, -1.0], [4.0, 11.0, 4.0]),
            ),
        )
        .child(
            NamedPartSpec::new("right_hind_leg", PartPose::offset(-4.0, 14.0, 7.0)).cube(cube(
                [48.0, 21.0],
                [-1.0, -1.01, -1.0],
                [4.0, 11.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_front_leg", PartPose::offset(4.0, 14.0, -10.0)).cube(
                cube_mirrored([48.0, 21.0], [-3.0, -1.01, -1.9], [4.0, 11.0, 4.0]),
            ),
        )
        .child(
            NamedPartSpec::new("right_front_leg", PartPose::offset(-4.0, 14.0, -10.0)).cube(cube(
                [48.0, 21.0],
                [-1.0, -1.01, -1.9],
                [4.0, 11.0, 4.0],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_horse_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_horse_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

pub fn apply_horse_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let pi = std::f32::consts::PI;
    let deg2rad = pi / 180.0;
    let clamped_y_rot = state.y_rot_degrees.clamp(-20.0, 20.0);
    let mut head_rot_x_rad = state.x_rot_degrees * deg2rad;
    let animation_speed = state.walk_animation_speed;
    let animation_pos = state.walk_animation_pos;

    if animation_speed > 0.2 {
        head_rot_x_rad += (animation_pos * 0.8).cos() * 0.15 * animation_speed;
    }

    let eating = state.eat_animation;
    let standing = state.stand_animation;
    let i_standing = 1.0 - standing;
    let feeding_anim = state.feeding_animation;
    let animate_tail = state.animate_tail;

    let water_multiplier = if state.is_in_water { 0.2 } else { 1.0 };
    let leg_anim_1 = (water_multiplier * animation_pos * 0.6662 + pi).cos();
    let leg_x_rot_anim = leg_anim_1 * 0.8 * animation_speed;
    let base_head_angle = (1.0 - standing.max(eating))
        * (pi / 6.0 + head_rot_x_rad + feeding_anim * (state.age_in_ticks).sin() * 0.05);

    if let Some(h) = tree.part_mut("head_parts") {
        h.pose.rotation.x = standing * (pi / 12.0 + head_rot_x_rad)
            + eating * (2.1816616 + (state.age_in_ticks).sin() * 0.05)
            + base_head_angle;
        let base_y_rot = clamped_y_rot * deg2rad;
        h.pose.rotation.y = standing * base_y_rot + (1.0 - standing.max(eating)) * base_y_rot;
        let lerp = |t: f32, a: f32, b: f32| a + t * (b - a);
        h.pose.offset.y += lerp(eating, lerp(standing, 0.0, -8.0), 7.0);
        h.pose.offset.z = lerp(standing, h.pose.offset.z, -4.0);
    }

    if let Some(b) = tree.part_mut("body") {
        b.pose.rotation.x = standing * (-pi / 4.0) + i_standing * b.pose.rotation.x;
    }

    if let Some(l) = tree.part_mut("left_front_leg") {
        l.pose.offset.y -= 12.0 * standing;
        l.pose.offset.z += 4.0 * standing;
    }
    if let Some(l) = tree.part_mut("right_front_leg") {
        l.pose.offset.y -= 12.0 * standing;
        l.pose.offset.z += 4.0 * standing;
    }

    let stand_angle = (pi / 12.0) * standing;
    let bob_value = (state.age_in_ticks * 0.6 + pi).cos();
    let leg_standing_x_rot_offset = -pi / 3.0;
    let rleg_rot = (leg_standing_x_rot_offset + bob_value) * standing + leg_x_rot_anim * i_standing;
    let lleg_rot = (leg_standing_x_rot_offset - bob_value) * standing - leg_x_rot_anim * i_standing;

    if let Some(l) = tree.part_mut("left_hind_leg") {
        l.pose.rotation.x = stand_angle - leg_anim_1 * 0.5 * animation_speed * i_standing;
    }
    if let Some(l) = tree.part_mut("right_hind_leg") {
        l.pose.rotation.x = stand_angle + leg_anim_1 * 0.5 * animation_speed * i_standing;
    }
    if let Some(l) = tree.part_mut("left_front_leg") {
        l.pose.rotation.x = rleg_rot;
    }
    if let Some(l) = tree.part_mut("right_front_leg") {
        l.pose.rotation.x = lleg_rot;
    }

    if let Some(t) = tree.part_mut("tail") {
        let age_scale = state.age_scale;
        t.pose.rotation.x = pi / 6.0 + animation_speed * 0.75;
        t.pose.offset.y += animation_speed * age_scale;
        t.pose.offset.z += animation_speed * 2.0 * age_scale;
        if animate_tail {
            t.pose.rotation.y = (state.age_in_ticks * 0.7).cos();
        } else {
            t.pose.rotation.y = 0.0;
        }
    }
}

pub fn build_horse_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_horse_model_tree();
    tree.reset_pose();
    apply_horse_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

pub fn build_donkey_model_tree() -> JavaModelPartTree {
    let mut tree = build_horse_model_tree();
    let pi = std::f32::consts::PI;

    let left_chest_idx = tree.push_part(
        NamedPartSpec::new(
            "left_chest",
            PartPose::offset_and_rotation(6.0, -8.0, 0.0, 0.0, -pi / 2.0, 0.0),
        )
        .cube(cube([26.0, 21.0], [-4.0, 0.0, -2.0], [8.0, 8.0, 3.0])),
    );
    let right_chest_idx = tree.push_part(
        NamedPartSpec::new(
            "right_chest",
            PartPose::offset_and_rotation(-6.0, -8.0, 0.0, 0.0, pi / 2.0, 0.0),
        )
        .cube(cube([26.0, 21.0], [-4.0, 0.0, -2.0], [8.0, 8.0, 3.0])),
    );
    if let Some(body) = tree.part_mut("body") {
        body.children.push(left_chest_idx);
        body.children.push(right_chest_idx);
    }

    let left_ear_idx = tree.push_part(
        NamedPartSpec::new(
            "left_ear",
            PartPose::offset_and_rotation(1.25, -10.0, 4.0, pi / 12.0, 0.0, pi / 12.0),
        )
        .cube(cube([0.0, 12.0], [-1.0, -7.0, 0.0], [2.0, 7.0, 1.0])),
    );
    let right_ear_idx = tree.push_part(
        NamedPartSpec::new(
            "right_ear",
            PartPose::offset_and_rotation(-1.25, -10.0, 4.0, pi / 12.0, 0.0, -pi / 12.0),
        )
        .cube(cube([0.0, 12.0], [-1.0, -7.0, 0.0], [2.0, 7.0, 1.0])),
    );

    let mut old_ears = Vec::new();
    for child_idx in tree
        .part("head")
        .map(|h| &h.children)
        .unwrap_or(&Vec::new())
    {
        let name = tree.nodes[*child_idx].name;
        if name == "left_ear" || name == "right_ear" {
            old_ears.push(*child_idx);
        }
    }

    if let Some(head) = tree.part_mut("head") {
        head.children.retain(|c| !old_ears.contains(c));
        head.children.push(left_ear_idx);
        head.children.push(right_ear_idx);
    }

    tree
}

pub fn build_donkey_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_donkey_model_tree();
    tree.reset_pose();
    apply_horse_model_setup_anim(&mut tree, state);

    let (mut vertices, indices) = tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    });
    for v in &mut vertices {
        v.position[0] *= 0.87;
        v.position[1] *= 0.87;
        v.position[2] *= 0.87;
    }
    (vertices, indices)
}

pub fn build_mule_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_donkey_model_tree();
    tree.reset_pose();
    apply_horse_model_setup_anim(&mut tree, state);
    let (mut vertices, indices) = tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    });
    for v in &mut vertices {
        v.position[0] *= 0.92;
        v.position[1] *= 0.92;
        v.position[2] *= 0.92;
    }
    (vertices, indices)
}
