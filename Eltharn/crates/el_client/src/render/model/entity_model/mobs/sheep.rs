use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_sheep_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("head", PartPose::offset(0.0, 6.0, -8.0)).cube(cube(
                [0.0, 0.0],
                [-3.0, -4.0, -6.0],
                [6.0, 6.0, 8.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "body",
                PartPose::offset_and_rotation(0.0, 5.0, 2.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([28.0, 8.0], [-4.0, -10.0, -7.0], [8.0, 16.0, 6.0])),
        )
        .child(
            NamedPartSpec::new("right_hind_leg", PartPose::offset(-3.0, 12.0, 7.0)).cube(
                cube_mirrored([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0]),
            ),
        )
        .child(
            NamedPartSpec::new("left_hind_leg", PartPose::offset(3.0, 12.0, 7.0)).cube(cube(
                [0.0, 16.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_front_leg", PartPose::offset(-3.0, 12.0, -5.0)).cube(
                cube_mirrored([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0]),
            ),
        )
        .child(
            NamedPartSpec::new("left_front_leg", PartPose::offset(3.0, 12.0, -5.0)).cube(cube(
                [0.0, 16.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_sheep_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_sheep_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn apply_sheep_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.offset.y += state.head_eat_position_scale * 9.0 * state.age_scale;
        h.pose.rotation.x = state.head_eat_angle_scale;
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
}

pub fn build_sheep_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_sheep_model_tree();
    tree.reset_pose();
    apply_sheep_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn build_sheep_fur_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 6.0, -8.0)).cube(cube_grown(
                    [0.0, 0.0],
                    [-3.0, -4.0, -4.0],
                    [6.0, 6.0, 6.0],
                    0.6,
                )),
            )
            .child(
                NamedPartSpec::new(
                    "body",
                    PartPose::offset_and_rotation(0.0, 5.0, 2.0, 1.570796, 0.0, 0.0),
                )
                .cube(cube_grown(
                    [28.0, 8.0],
                    [-4.0, -10.0, -7.0],
                    [8.0, 16.0, 6.0],
                    1.75,
                )),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-3.0, 12.0, 7.0)).cube(
                    cube_grown([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], 0.5),
                ),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(3.0, 12.0, 7.0)).cube(
                    cube_grown([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], 0.5),
                ),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-3.0, 12.0, -5.0)).cube(
                    cube_grown([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], 0.5),
                ),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(3.0, 12.0, -5.0)).cube(
                    cube_grown([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], 0.5),
                ),
            );

    JavaModelPartTree::from_root(root)
}

pub fn build_sheep_wool_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_sheep_fur_model_tree();
    tree.reset_pose();
    apply_sheep_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
