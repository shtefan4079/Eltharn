use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_cat_model_tree() -> JavaModelPartTree {
    let pi = std::f32::consts::PI;
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 15.0, -9.0))
                    .cube(cube([0.0, 0.0], [-2.5, -2.0, -3.0], [5.0, 4.0, 5.0]))
                    .cube(cube([0.0, 24.0], [-1.5, -0.001, -4.0], [3.0, 2.0, 2.0]))
                    .cube(cube([0.0, 10.0], [-2.0, -3.0, 0.0], [1.0, 1.0, 2.0]))
                    .cube(cube([6.0, 10.0], [1.0, -3.0, 0.0], [1.0, 1.0, 2.0])),
            )
            .child(
                NamedPartSpec::new(
                    "body",
                    PartPose::offset_and_rotation(0.0, 12.0, -10.0, pi / 2.0, 0.0, 0.0),
                )
                .cube(cube([20.0, 0.0], [-2.0, 3.0, -8.0], [4.0, 16.0, 6.0])),
            )
            .child(
                NamedPartSpec::new(
                    "tail1",
                    PartPose::offset_and_rotation(0.0, 15.0, 8.0, 0.9, 0.0, 0.0),
                )
                .cube(cube([0.0, 15.0], [-0.5, 0.0, 0.0], [1.0, 8.0, 1.0])),
            )
            .child(
                NamedPartSpec::new("tail2", PartPose::offset(0.0, 20.0, 14.0)).cube(cube_grown(
                    [4.0, 15.0],
                    [-0.5, 0.0, 0.0],
                    [1.0, 8.0, 1.0],
                    -0.02,
                )),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(1.1, 18.0, 5.0)).cube(cube(
                    [8.0, 13.0],
                    [-1.0, 0.0, 1.0],
                    [2.0, 6.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-1.1, 18.0, 5.0)).cube(cube(
                    [8.0, 13.0],
                    [-1.0, 0.0, 1.0],
                    [2.0, 6.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(1.2, 14.1, -5.0)).cube(cube(
                    [40.0, 0.0],
                    [-1.0, 0.0, 0.0],
                    [2.0, 10.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-1.2, 14.1, -5.0))
                    .cube(cube([40.0, 0.0], [-1.0, 0.0, 0.0], [2.0, 10.0, 2.0])),
            );

    JavaModelPartTree::from_root(root)
}

pub fn build_cat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_cat_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn build_ocelot_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_cat_mesh()
}

pub fn apply_cat_model_setup_anim(tree: &mut JavaModelPartTree, state: EntityModelAnimationState) {
    let pi = std::f32::consts::PI;
    let deg2rad = pi / 180.0;
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;
    let age_scale = state.age_scale;

    if state.is_crouching {
        if let Some(b) = tree.part_mut("body") {
            b.pose.offset.y += 1.0 * age_scale;
        }
        if let Some(h) = tree.part_mut("head") {
            h.pose.offset.y += 2.0 * age_scale;
        }
        if let Some(t1) = tree.part_mut("tail1") {
            t1.pose.offset.y += 1.0 * age_scale;
            t1.pose.rotation.x = pi / 2.0;
        }
        if let Some(t2) = tree.part_mut("tail2") {
            t2.pose.offset.y += -4.0 * age_scale;
            t2.pose.offset.z += 2.0 * age_scale;
            t2.pose.rotation.x = pi / 2.0;
        }
    } else if state.is_sprinting {
        let t1_y = tree.part("tail1").map(|t| t.pose.offset.y).unwrap_or(15.0);
        if let Some(t1) = tree.part_mut("tail1") {
            t1.pose.rotation.x = pi / 2.0;
        }
        if let Some(t2) = tree.part_mut("tail2") {
            t2.pose.offset.y = t1_y;
            t2.pose.offset.z += 2.0 * age_scale;
            t2.pose.rotation.x = pi / 2.0;
        }
    }

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * deg2rad;
        h.pose.rotation.y = state.y_rot_degrees * deg2rad;
    }

    if !state.is_sitting {
        if let Some(b) = tree.part_mut("body") {
            b.pose.rotation.x = pi / 2.0;
        }
        if state.is_sprinting {
            if let Some(l) = tree.part_mut("left_hind_leg") {
                l.pose.rotation.x = (anim_pos * 0.6662).cos() * anim_speed;
            }
            if let Some(l) = tree.part_mut("right_hind_leg") {
                l.pose.rotation.x = (anim_pos * 0.6662 + 0.3).cos() * anim_speed;
            }
            if let Some(l) = tree.part_mut("left_front_leg") {
                l.pose.rotation.x = (anim_pos * 0.6662 + pi + 0.3).cos() * anim_speed;
            }
            if let Some(l) = tree.part_mut("right_front_leg") {
                l.pose.rotation.x = (anim_pos * 0.6662 + pi).cos() * anim_speed;
            }
            if let Some(t2) = tree.part_mut("tail2") {
                t2.pose.rotation.x = 1.7278761 + (pi / 10.0) * anim_pos.cos() * anim_speed;
            }
        } else {
            if let Some(l) = tree.part_mut("left_hind_leg") {
                l.pose.rotation.x = (anim_pos * 0.6662).cos() * anim_speed;
            }
            if let Some(l) = tree.part_mut("right_hind_leg") {
                l.pose.rotation.x = (anim_pos * 0.6662 + pi).cos() * anim_speed;
            }
            if let Some(l) = tree.part_mut("left_front_leg") {
                l.pose.rotation.x = (anim_pos * 0.6662 + pi).cos() * anim_speed;
            }
            if let Some(l) = tree.part_mut("right_front_leg") {
                l.pose.rotation.x = (anim_pos * 0.6662).cos() * anim_speed;
            }
            if let Some(t2) = tree.part_mut("tail2") {
                if !state.is_crouching {
                    t2.pose.rotation.x = 1.7278761 + (pi / 4.0) * anim_pos.cos() * anim_speed;
                } else {
                    t2.pose.rotation.x = 1.7278761 + 0.47123894 * anim_pos.cos() * anim_speed;
                }
            }
        }
    }

    if state.is_sitting {
        if let Some(b) = tree.part_mut("body") {
            b.pose.rotation.x = pi / 4.0;
            b.pose.offset.y += -4.0 * age_scale;
            b.pose.offset.z += 5.0 * age_scale;
        }
        if let Some(h) = tree.part_mut("head") {
            h.pose.offset.y += -3.3 * age_scale;
            h.pose.offset.z += 1.0 * age_scale;
        }
        if let Some(t1) = tree.part_mut("tail1") {
            t1.pose.offset.y += 8.0 * age_scale;
            t1.pose.offset.z += -2.0 * age_scale;
            t1.pose.rotation.x = 1.7278761;
        }
        if let Some(t2) = tree.part_mut("tail2") {
            t2.pose.offset.y += 2.0 * age_scale;
            t2.pose.offset.z += -0.8 * age_scale;
            t2.pose.rotation.x = 2.670354;
        }
        if let Some(l) = tree.part_mut("left_front_leg") {
            l.pose.rotation.x = -pi / 20.0;
            l.pose.offset.y += 2.0 * age_scale;
            l.pose.offset.z -= 2.0 * age_scale;
        }
        if let Some(l) = tree.part_mut("right_front_leg") {
            l.pose.rotation.x = -pi / 20.0;
            l.pose.offset.y += 2.0 * age_scale;
            l.pose.offset.z -= 2.0 * age_scale;
        }
        if let Some(l) = tree.part_mut("left_hind_leg") {
            l.pose.rotation.x = -pi / 2.0;
            l.pose.offset.y += 3.0 * age_scale;
            l.pose.offset.z -= 4.0 * age_scale;
        }
        if let Some(l) = tree.part_mut("right_hind_leg") {
            l.pose.rotation.x = -pi / 2.0;
            l.pose.offset.y += 3.0 * age_scale;
            l.pose.offset.z -= 4.0 * age_scale;
        }
    }

    if state.lie_down_amount > 0.0 {
        let lerp = |amount: f32, old: f32, new: f32| old + amount * (new - old);
        if let Some(h) = tree.part_mut("head") {
            h.pose.rotation.z = lerp(state.lie_down_amount, h.pose.rotation.z, -1.2707963);
            h.pose.rotation.y = lerp(state.lie_down_amount, h.pose.rotation.y, 1.2707963);
        }
        if let Some(l) = tree.part_mut("left_front_leg") {
            l.pose.rotation.x = -1.2707963;
        }
        if let Some(l) = tree.part_mut("right_front_leg") {
            l.pose.rotation.x = -0.47079635;
            l.pose.rotation.z = -0.2;
            l.pose.offset.x += age_scale;
        }
        if let Some(l) = tree.part_mut("left_hind_leg") {
            l.pose.rotation.x = -0.4;
        }
        if let Some(l) = tree.part_mut("right_hind_leg") {
            l.pose.rotation.x = 0.5;
            l.pose.rotation.z = -0.5;
            l.pose.offset.x += 0.8 * age_scale;
            l.pose.offset.y += 2.0 * age_scale;
        }
        if let Some(t1) = tree.part_mut("tail1") {
            t1.pose.rotation.x = lerp(state.lie_down_amount_tail, t1.pose.rotation.x, 0.8);
        }
        if let Some(t2) = tree.part_mut("tail2") {
            t2.pose.rotation.x = lerp(state.lie_down_amount_tail, t2.pose.rotation.x, -0.4);
        }
    }

    if state.relax_state_one_amount > 0.0 {
        let lerp = |amount: f32, old: f32, new: f32| old + amount * (new - old);
        if let Some(h) = tree.part_mut("head") {
            h.pose.rotation.x = lerp(state.relax_state_one_amount, h.pose.rotation.x, -0.58177644);
        }
    }
}

pub fn build_cat_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_cat_model_tree();
    tree.reset_pose();
    apply_cat_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn build_ocelot_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    build_cat_mesh_animated(state)
}
