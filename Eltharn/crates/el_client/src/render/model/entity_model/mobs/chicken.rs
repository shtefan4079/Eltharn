use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_chicken_model_tree() -> JavaModelPartTree {
    let pi = std::f32::consts::PI;
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("head", PartPose::offset(0.0, 15.0, -4.0))
                .cube(cube([0.0, 0.0], [-2.0, -6.0, -2.0], [4.0, 6.0, 3.0]))
                .child(NamedPartSpec::new("beak", PartPose::ZERO).cube(cube(
                    [14.0, 0.0],
                    [-2.0, -4.0, -4.0],
                    [4.0, 2.0, 2.0],
                )))
                .child(NamedPartSpec::new("red_thing", PartPose::ZERO).cube(cube(
                    [14.0, 4.0],
                    [-1.0, -2.0, -3.0],
                    [2.0, 2.0, 2.0],
                ))),
        )
        .child(
            NamedPartSpec::new(
                "body",
                PartPose::offset_and_rotation(0.0, 16.0, 0.0, pi / 2.0, 0.0, 0.0),
            )
            .cube(cube([0.0, 9.0], [-3.0, -4.0, -3.0], [6.0, 8.0, 6.0])),
        )
        .child(
            NamedPartSpec::new("right_leg", PartPose::offset(-2.0, 19.0, 1.0)).cube(cube(
                [26.0, 0.0],
                [-1.0, 0.0, -3.0],
                [3.0, 5.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_leg", PartPose::offset(1.0, 19.0, 1.0)).cube(cube(
                [26.0, 0.0],
                [-1.0, 0.0, -3.0],
                [3.0, 5.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_wing", PartPose::offset(-4.0, 13.0, 0.0)).cube(cube(
                [24.0, 13.0],
                [0.0, 0.0, -3.0],
                [1.0, 4.0, 6.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_wing", PartPose::offset(4.0, 13.0, 0.0)).cube(cube(
                [24.0, 13.0],
                [-1.0, 0.0, -3.0],
                [1.0, 4.0, 6.0],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_chicken_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_chicken_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn apply_chicken_model_setup_anim(
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

    let flap_angle = (state.flap.sin() + 1.0) * state.flap_speed;
    if let Some(l) = tree.part_mut("right_leg") {
        l.pose.rotation.x = (anim_pos * 0.6662).cos() * 1.4 * anim_speed;
    }
    if let Some(l) = tree.part_mut("left_leg") {
        l.pose.rotation.x = (anim_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * anim_speed;
    }
    if let Some(w) = tree.part_mut("right_wing") {
        w.pose.rotation.z = flap_angle;
    }
    if let Some(w) = tree.part_mut("left_wing") {
        w.pose.rotation.z = -flap_angle;
    }
}

pub fn build_chicken_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_chicken_model_tree();
    tree.reset_pose();
    apply_chicken_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
