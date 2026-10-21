use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_fox_model_tree() -> JavaModelPartTree {
    let pi = std::f32::consts::PI;
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("head", PartPose::offset(-1.0, 16.5, -3.0))
                .cube(cube([1.0, 5.0], [-3.0, -2.0, -5.0], [8.0, 6.0, 6.0]))
                .child(NamedPartSpec::new("right_ear", PartPose::ZERO).cube(cube(
                    [8.0, 1.0],
                    [-3.0, -4.0, -4.0],
                    [2.0, 2.0, 1.0],
                )))
                .child(NamedPartSpec::new("left_ear", PartPose::ZERO).cube(cube(
                    [15.0, 1.0],
                    [3.0, -4.0, -4.0],
                    [2.0, 2.0, 1.0],
                )))
                .child(NamedPartSpec::new("nose", PartPose::ZERO).cube(cube(
                    [6.0, 18.0],
                    [-1.0, 2.01, -8.0],
                    [4.0, 2.0, 3.0],
                ))),
        )
        .child(
            NamedPartSpec::new(
                "body",
                PartPose::offset_and_rotation(0.0, 16.0, -6.0, pi / 2.0, 0.0, 0.0),
            )
            .cube(cube([24.0, 15.0], [-3.0, 3.999, -3.5], [6.0, 11.0, 6.0]))
            .child(
                NamedPartSpec::new(
                    "tail",
                    PartPose::offset_and_rotation(-4.0, 15.0, -1.0, -0.05235988, 0.0, 0.0),
                )
                .cube(cube([30.0, 0.0], [2.0, 0.0, -1.0], [4.0, 9.0, 5.0])),
            ),
        )
        .child(
            NamedPartSpec::new("right_hind_leg", PartPose::offset(-5.0, 17.5, 7.0)).cube(
                cube_grown([13.0, 24.0], [2.0, 0.5, -1.0], [2.0, 6.0, 2.0], 0.001),
            ),
        )
        .child(
            NamedPartSpec::new("left_hind_leg", PartPose::offset(-1.0, 17.5, 7.0)).cube(
                cube_grown([4.0, 24.0], [2.0, 0.5, -1.0], [2.0, 6.0, 2.0], 0.001),
            ),
        )
        .child(
            NamedPartSpec::new("right_front_leg", PartPose::offset(-5.0, 17.5, 0.0)).cube(
                cube_grown([13.0, 24.0], [2.0, 0.5, -1.0], [2.0, 6.0, 2.0], 0.001),
            ),
        )
        .child(
            NamedPartSpec::new("left_front_leg", PartPose::offset(-1.0, 17.5, 0.0)).cube(
                cube_grown([4.0, 24.0], [2.0, 0.5, -1.0], [2.0, 6.0, 2.0], 0.001),
            ),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_fox_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_fox_model_tree().mesh(ModelTextureSize {
        width: 48.0,
        height: 32.0,
    })
}

pub fn apply_fox_model_setup_anim(tree: &mut JavaModelPartTree, state: EntityModelAnimationState) {
    let pi = std::f32::consts::PI;
    let deg2rad = pi / 180.0;
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;
    let age_scale = state.age_scale;

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.z = state.head_roll_angle;
    }
    if let Some(l) = tree.part_mut("right_hind_leg") {
        l.pose.rotation.x = (anim_pos * 0.6662).cos() * 1.4 * anim_speed;
    }
    if let Some(l) = tree.part_mut("left_hind_leg") {
        l.pose.rotation.x = (anim_pos * 0.6662 + pi).cos() * 1.4 * anim_speed;
    }
    if let Some(l) = tree.part_mut("right_front_leg") {
        l.pose.rotation.x = (anim_pos * 0.6662 + pi).cos() * 1.4 * anim_speed;
    }
    if let Some(l) = tree.part_mut("left_front_leg") {
        l.pose.rotation.x = (anim_pos * 0.6662).cos() * 1.4 * anim_speed;
    }

    if state.is_crouching {
        if let Some(b) = tree.part_mut("body") {
            b.pose.rotation.x += 0.10471976;
            b.pose.offset.y += state.crouch_amount;
            let wiggle = (state.age_in_ticks).cos() * 0.05;
            b.pose.rotation.y = wiggle;
        }
        if let Some(h) = tree.part_mut("head") {
            h.pose.offset.y += state.crouch_amount * age_scale;
        }
        let wiggle = (state.age_in_ticks).cos() * 0.05;
        if let Some(l) = tree.part_mut("right_hind_leg") {
            l.pose.rotation.z = wiggle;
        }
        if let Some(l) = tree.part_mut("left_hind_leg") {
            l.pose.rotation.z = wiggle;
        }
        if let Some(l) = tree.part_mut("right_front_leg") {
            l.pose.rotation.z = wiggle / 2.0;
        }
        if let Some(l) = tree.part_mut("left_front_leg") {
            l.pose.rotation.z = wiggle / 2.0;
        }
    } else if state.is_sleeping {
        if let Some(b) = tree.part_mut("body") {
            b.pose.rotation.z = -pi / 2.0;
            b.pose.offset.y += 5.0;
        }
        if let Some(t) = tree.part_mut("tail") {
            t.pose.rotation.x = -pi * 5.0 / 6.0;
        }
        if let Some(h) = tree.part_mut("head") {
            h.pose.offset.x += 2.0;
            h.pose.offset.y += 2.99;
            h.pose.rotation.x = 0.0;
            h.pose.rotation.y = -pi * 2.0 / 3.0;
            h.pose.rotation.z = (state.age_in_ticks * 0.027).cos() / 22.0;
        }
    } else if state.is_sitting {
        if let Some(b) = tree.part_mut("body") {
            b.pose.rotation.x = pi / 6.0;
            b.pose.offset.y -= 7.0;
            b.pose.offset.z += 3.0;
        }
        if let Some(t) = tree.part_mut("tail") {
            t.pose.rotation.x = pi / 4.0;
            t.pose.offset.z -= 1.0;
        }
        if let Some(h) = tree.part_mut("head") {
            h.pose.rotation.x = 0.0;
            h.pose.rotation.y = 0.0;
            h.pose.offset.y -= 6.5;
            h.pose.offset.z += 2.75;
        }
        if let Some(l) = tree.part_mut("right_front_leg") {
            l.pose.rotation.x = -pi / 12.0;
        }
        if let Some(l) = tree.part_mut("left_front_leg") {
            l.pose.rotation.x = -pi / 12.0;
        }
        if let Some(l) = tree.part_mut("right_hind_leg") {
            l.pose.rotation.x = -pi * 5.0 / 12.0;
            l.pose.offset.y += 4.0;
            l.pose.offset.z -= 0.25;
        }
        if let Some(l) = tree.part_mut("left_hind_leg") {
            l.pose.rotation.x = -pi * 5.0 / 12.0;
            l.pose.offset.y += 4.0;
            l.pose.offset.z -= 0.25;
        }
    }

    if state.is_pouncing {
        let crouch = state.crouch_amount / 2.0;
        if let Some(b) = tree.part_mut("body") {
            b.pose.offset.y -= crouch;
        }
        if let Some(h) = tree.part_mut("head") {
            h.pose.offset.y -= crouch;
        }
    }

    if !state.is_sleeping && !state.is_faceplanted && !state.is_crouching {
        if let Some(h) = tree.part_mut("head") {
            h.pose.rotation.x = state.x_rot_degrees * deg2rad;
            h.pose.rotation.y = state.y_rot_degrees * deg2rad;
        }
    }
}

pub fn build_fox_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_fox_model_tree();
    tree.reset_pose();
    apply_fox_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 48.0,
        height: 32.0,
    })
}
