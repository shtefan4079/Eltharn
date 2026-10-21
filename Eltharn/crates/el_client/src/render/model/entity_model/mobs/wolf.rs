use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_wolf_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("head", PartPose::offset(-1.0, 13.5, -7.0)).child(
                NamedPartSpec::new("real_head", PartPose::ZERO)
                    .cube(cube([0.0, 0.0], [-2.0, -3.0, -2.0], [6.0, 6.0, 4.0]))
                    .cube(cube([16.0, 14.0], [-2.0, -5.0, 0.0], [2.0, 2.0, 1.0]))
                    .cube(cube([16.0, 14.0], [2.0, -5.0, 0.0], [2.0, 2.0, 1.0]))
                    .cube(cube([0.0, 10.0], [-0.5, -0.001, -5.0], [3.0, 3.0, 4.0])),
            ),
        )
        .child(
            NamedPartSpec::new(
                "body",
                PartPose::offset_and_rotation(0.0, 14.0, 2.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([18.0, 14.0], [-3.0, -2.0, -3.0], [6.0, 9.0, 6.0])),
        )
        .child(
            NamedPartSpec::new(
                "upper_body",
                PartPose::offset_and_rotation(-1.0, 14.0, -3.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([21.0, 0.0], [-3.0, -3.0, -3.0], [8.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new("right_hind_leg", PartPose::offset(-2.5, 16.0, 7.0)).cube(
                cube_mirrored([0.0, 18.0], [0.0, 0.0, -1.0], [2.0, 8.0, 2.0]),
            ),
        )
        .child(
            NamedPartSpec::new("left_hind_leg", PartPose::offset(0.5, 16.0, 7.0)).cube(cube(
                [0.0, 18.0],
                [0.0, 0.0, -1.0],
                [2.0, 8.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_front_leg", PartPose::offset(-2.5, 16.0, -4.0)).cube(
                cube_mirrored([0.0, 18.0], [0.0, 0.0, -1.0], [2.0, 8.0, 2.0]),
            ),
        )
        .child(
            NamedPartSpec::new("left_front_leg", PartPose::offset(0.5, 16.0, -4.0)).cube(cube(
                [0.0, 18.0],
                [0.0, 0.0, -1.0],
                [2.0, 8.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "tail",
                PartPose::offset_and_rotation(-1.0, 12.0, 8.0, 0.6283185, 0.0, 0.0),
            )
            .child(NamedPartSpec::new("real_tail", PartPose::ZERO).cube(cube(
                [9.0, 18.0],
                [0.0, 0.0, -1.0],
                [2.0, 8.0, 2.0],
            ))),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_wolf_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_wolf_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn apply_wolf_model_setup_anim(tree: &mut JavaModelPartTree, state: EntityModelAnimationState) {
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;

    if let Some(t) = tree.part_mut("tail") {
        if state.is_angry {
            t.pose.rotation.y = 0.0;
        } else {
            t.pose.rotation.y = (anim_pos * 0.6662).cos() * 1.4 * anim_speed;
        }
        t.pose.rotation.x = state.tail_angle;
    }

    if state.is_sitting {
        let age_scale = state.age_scale;
        if let Some(b) = tree.part_mut("body") {
            b.pose.offset.y += 4.0 * age_scale;
            b.pose.offset.z -= 2.0 * age_scale;
            b.pose.rotation.x = std::f32::consts::PI / 4.0;
        }
        if let Some(ub) = tree.part_mut("upper_body") {
            ub.pose.offset.y += 2.0;
            ub.pose.rotation.x = std::f32::consts::PI * 2.0 / 5.0;
            ub.pose.rotation.y = 0.0;
        }
        if let Some(t) = tree.part_mut("tail") {
            t.pose.offset.y += 9.0 * age_scale;
            t.pose.offset.z -= 2.0 * age_scale;
        }
        if let Some(l) = tree.part_mut("right_hind_leg") {
            l.pose.offset.y += 6.7 * age_scale;
            l.pose.offset.z -= 5.0 * age_scale;
            l.pose.rotation.x = std::f32::consts::PI * 1.5;
        }
        if let Some(l) = tree.part_mut("left_hind_leg") {
            l.pose.offset.y += 6.7 * age_scale;
            l.pose.offset.z -= 5.0 * age_scale;
            l.pose.rotation.x = std::f32::consts::PI * 1.5;
        }
        if let Some(l) = tree.part_mut("right_front_leg") {
            l.pose.rotation.x = 5.811947;
            l.pose.offset.x += 0.01 * age_scale;
            l.pose.offset.y += 1.0 * age_scale;
        }
        if let Some(l) = tree.part_mut("left_front_leg") {
            l.pose.rotation.x = 5.811947;
            l.pose.offset.x -= 0.01 * age_scale;
            l.pose.offset.y += 1.0 * age_scale;
        }
    } else {
        if let Some(l) = tree.part_mut("right_hind_leg") {
            l.pose.rotation.x = (anim_pos * 0.6662).cos() * 1.4 * anim_speed;
        }
        if let Some(l) = tree.part_mut("left_hind_leg") {
            l.pose.rotation.x = (anim_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * anim_speed;
        }
        if let Some(l) = tree.part_mut("right_front_leg") {
            l.pose.rotation.x = (anim_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * anim_speed;
        }
        if let Some(l) = tree.part_mut("left_front_leg") {
            l.pose.rotation.x = (anim_pos * 0.6662).cos() * 1.4 * anim_speed;
        }
    }

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    }
    if let Some(rh) = tree.part_mut("real_head") {
        let body_roll_0 = (state.shake_anim * std::f32::consts::PI * 2.0).sin();
        rh.pose.rotation.z = state.head_roll_angle + body_roll_0;
    }
    if let Some(ub) = tree.part_mut("upper_body") {
        if state.shake_anim > 0.0 {
            let body_roll_neg = ((state.shake_anim - 0.08) * std::f32::consts::PI * 2.0).sin();
            ub.pose.rotation.z = body_roll_neg;
        }
    }
}

pub fn build_wolf_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_wolf_model_tree();
    tree.reset_pose();
    apply_wolf_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
