use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_llama_model_tree() -> JavaModelPartTree {
    let pi = std::f32::consts::PI;
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 7.0, -6.0))
                    .cube(cube([0.0, 0.0], [-2.0, -14.0, -10.0], [4.0, 4.0, 9.0]))
                    .cube(cube([0.0, 14.0], [-4.0, -16.0, -6.0], [8.0, 18.0, 6.0]))
                    .cube(cube([17.0, 0.0], [-4.0, -19.0, -4.0], [3.0, 3.0, 2.0]))
                    .cube(cube([17.0, 0.0], [1.0, -19.0, -4.0], [3.0, 3.0, 2.0])),
            )
            .child(
                NamedPartSpec::new(
                    "body",
                    PartPose::offset_and_rotation(0.0, 5.0, 2.0, pi / 2.0, 0.0, 0.0),
                )
                .cube(cube([29.0, 0.0], [-6.0, -10.0, -7.0], [12.0, 18.0, 10.0])),
            )
            .child(
                NamedPartSpec::new(
                    "right_chest",
                    PartPose::offset_and_rotation(-8.5, 3.0, 3.0, 0.0, pi / 2.0, 0.0),
                )
                .cube(cube([45.0, 28.0], [-3.0, 0.0, 0.0], [8.0, 8.0, 3.0])),
            )
            .child(
                NamedPartSpec::new(
                    "left_chest",
                    PartPose::offset_and_rotation(5.5, 3.0, 3.0, 0.0, pi / 2.0, 0.0),
                )
                .cube(cube([45.0, 41.0], [-3.0, 0.0, 0.0], [8.0, 8.0, 3.0])),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-3.5, 10.0, 6.0)).cube(cube(
                    [29.0, 29.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 14.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(3.5, 10.0, 6.0)).cube(cube(
                    [29.0, 29.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 14.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-3.5, 10.0, -5.0))
                    .cube(cube([29.0, 29.0], [-2.0, 0.0, -2.0], [4.0, 14.0, 4.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(3.5, 10.0, -5.0)).cube(cube(
                    [29.0, 29.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 14.0, 4.0],
                )),
            );

    JavaModelPartTree::from_root(root)
}

pub fn build_llama_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_llama_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}

pub fn apply_llama_model_setup_anim(
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

    let leg_rot = (anim_pos * 0.6662).cos() * 1.4 * anim_speed;
    let leg_rot_opp = (anim_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * anim_speed;

    if let Some(l) = tree.part_mut("right_hind_leg") {
        l.pose.rotation.x = leg_rot;
    }
    if let Some(l) = tree.part_mut("left_hind_leg") {
        l.pose.rotation.x = leg_rot_opp;
    }
    if let Some(l) = tree.part_mut("right_front_leg") {
        l.pose.rotation.x = leg_rot_opp;
    }
    if let Some(l) = tree.part_mut("left_front_leg") {
        l.pose.rotation.x = leg_rot;
    }

    if let Some(rc) = tree.part_mut("right_chest") {
        rc.visible = state.has_chest;
    }
    if let Some(lc) = tree.part_mut("left_chest") {
        lc.visible = state.has_chest;
    }
}

pub fn build_llama_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_llama_model_tree();
    tree.reset_pose();
    apply_llama_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
