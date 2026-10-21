use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_enderman_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, -13.0, 0.0))
                    .cube(cube([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]))
                    .child(NamedPartSpec::new("hat", PartPose::ZERO).cube(cube_grown(
                        [0.0, 16.0],
                        [-4.0, -8.0, -4.0],
                        [8.0, 8.0, 8.0],
                        -0.5,
                    ))),
            )
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, -14.0, 0.0)).cube(cube(
                    [32.0, 16.0],
                    [-4.0, 0.0, -2.0],
                    [8.0, 12.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_arm", PartPose::offset(-5.0, -12.0, 0.0)).cube(cube(
                    [56.0, 0.0],
                    [-1.0, -2.0, -1.0],
                    [2.0, 30.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_arm", PartPose::offset(5.0, -12.0, 0.0)).cube(
                    cube_mirrored([56.0, 0.0], [-1.0, -2.0, -1.0], [2.0, 30.0, 2.0]),
                ),
            )
            .child(
                NamedPartSpec::new("right_leg", PartPose::offset(-2.0, -5.0, 0.0)).cube(cube(
                    [56.0, 0.0],
                    [-1.0, 0.0, -1.0],
                    [2.0, 30.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_leg", PartPose::offset(2.0, -5.0, 0.0)).cube(
                    cube_mirrored([56.0, 0.0], [-1.0, 0.0, -1.0], [2.0, 30.0, 2.0]),
                ),
            );

    JavaModelPartTree::from_root(root)
}

pub fn build_enderman_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_enderman_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn apply_enderman_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    }

    let right_leg_x = ((anim_pos * 0.6662).cos() * 1.4 * anim_speed * 0.5).clamp(-0.4, 0.4);
    let left_leg_x = ((anim_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * anim_speed * 0.5)
        .clamp(-0.4, 0.4);

    let right_arm_x = ((anim_pos * 0.6662 + std::f32::consts::PI).cos() * 2.0 * anim_speed * 0.25)
        .clamp(-0.4, 0.4);
    let left_arm_x = ((anim_pos * 0.6662).cos() * 2.0 * anim_speed * 0.25).clamp(-0.4, 0.4);

    if let Some(l) = tree.part_mut("right_leg") {
        l.pose.rotation.x = right_leg_x;
    }
    if let Some(l) = tree.part_mut("left_leg") {
        l.pose.rotation.x = left_leg_x;
    }

    if let Some(a) = tree.part_mut("right_arm") {
        a.pose.rotation.x = right_arm_x;
    }
    if let Some(a) = tree.part_mut("left_arm") {
        a.pose.rotation.x = left_arm_x;
    }

    if state.is_creepy {
        if let Some(h) = tree.part_mut("head") {
            h.pose.offset.y -= 5.0;
        }
        if let Some(hat) = tree.part_mut("hat") {
            hat.pose.offset.y += 5.0;
        }
    }
}

pub fn build_enderman_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_enderman_model_tree();
    tree.reset_pose();
    apply_enderman_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
