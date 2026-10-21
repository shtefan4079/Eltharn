use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_strider_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("right_leg", PartPose::offset(-4.0, 8.0, 0.0)).cube(cube(
                [0.0, 32.0],
                [-2.0, 0.0, -2.0],
                [4.0, 16.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_leg", PartPose::offset(4.0, 8.0, 0.0)).cube(cube(
                [0.0, 55.0],
                [-2.0, 0.0, -2.0],
                [4.0, 16.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 1.0, 0.0))
                .cube(cube([0.0, 0.0], [-8.0, -6.0, -8.0], [16.0, 14.0, 16.0]))
                .child(
                    NamedPartSpec::new(
                        "right_bottom_bristle",
                        PartPose::offset_and_rotation(-8.0, 4.0, -8.0, 0.0, 0.0, -1.2217305),
                    )
                    .cube(cube_mirrored(
                        [16.0, 65.0],
                        [-12.0, 0.0, 0.0],
                        [12.0, 0.0, 16.0],
                    )),
                )
                .child(
                    NamedPartSpec::new(
                        "right_middle_bristle",
                        PartPose::offset_and_rotation(-8.0, -1.0, -8.0, 0.0, 0.0, -1.134464),
                    )
                    .cube(cube_mirrored(
                        [16.0, 49.0],
                        [-12.0, 0.0, 0.0],
                        [12.0, 0.0, 16.0],
                    )),
                )
                .child(
                    NamedPartSpec::new(
                        "right_top_bristle",
                        PartPose::offset_and_rotation(-8.0, -5.0, -8.0, 0.0, 0.0, -0.87266463),
                    )
                    .cube(cube_mirrored(
                        [16.0, 33.0],
                        [-12.0, 0.0, 0.0],
                        [12.0, 0.0, 16.0],
                    )),
                )
                .child(
                    NamedPartSpec::new(
                        "left_top_bristle",
                        PartPose::offset_and_rotation(8.0, -6.0, -8.0, 0.0, 0.0, 0.87266463),
                    )
                    .cube(cube(
                        [16.0, 33.0],
                        [0.0, 0.0, 0.0],
                        [12.0, 0.0, 16.0],
                    )),
                )
                .child(
                    NamedPartSpec::new(
                        "left_middle_bristle",
                        PartPose::offset_and_rotation(8.0, -2.0, -8.0, 0.0, 0.0, 1.134464),
                    )
                    .cube(cube(
                        [16.0, 49.0],
                        [0.0, 0.0, 0.0],
                        [12.0, 0.0, 16.0],
                    )),
                )
                .child(
                    NamedPartSpec::new(
                        "left_bottom_bristle",
                        PartPose::offset_and_rotation(8.0, 3.0, -8.0, 0.0, 0.0, 1.2217305),
                    )
                    .cube(cube(
                        [16.0, 65.0],
                        [0.0, 0.0, 0.0],
                        [12.0, 0.0, 16.0],
                    )),
                ),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_strider_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_strider_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 128.0,
    })
}

pub fn apply_strider_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed.min(0.25);

    if let Some(b) = tree.part_mut("body") {
        if !state.is_ridden {
            b.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
            b.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
        } else {
            b.pose.rotation.x = 0.0;
            b.pose.rotation.y = 0.0;
        }
        b.pose.rotation.z = 0.1 * (anim_pos * 1.5).sin() * 4.0 * anim_speed;
        b.pose.offset.y = 2.0 - 2.0 * (anim_pos * 1.5).cos() * 2.0 * anim_speed;
    }

    if let Some(l) = tree.part_mut("left_leg") {
        l.pose.rotation.x = (anim_pos * 1.5 * 0.5).sin() * 2.0 * anim_speed;
        l.pose.rotation.z =
            (std::f32::consts::PI / 18.0) * (anim_pos * 1.5 * 0.5).cos() * anim_speed;
        l.pose.offset.y =
            8.0 + 2.0 * (anim_pos * 1.5 * 0.5 + std::f32::consts::PI).sin() * 2.0 * anim_speed;
    }

    if let Some(r) = tree.part_mut("right_leg") {
        r.pose.rotation.x = (anim_pos * 1.5 * 0.5 + std::f32::consts::PI).sin() * 2.0 * anim_speed;
        r.pose.rotation.z = (std::f32::consts::PI / 18.0)
            * (anim_pos * 1.5 * 0.5 + std::f32::consts::PI).cos()
            * anim_speed;
        r.pose.offset.y = 8.0 + 2.0 * (anim_pos * 1.5 * 0.5).sin() * 2.0 * anim_speed;
    }

    let bristle_flow = (anim_pos * 1.5 + std::f32::consts::PI).cos() * anim_speed;
    let age_ticks = state.age_in_ticks;

    let animate_bristles = |tree: &mut JavaModelPartTree,
                            top: &str,
                            mid: &str,
                            bot: &str,
                            initial_z_rots: (f32, f32, f32)| {
        let (top_rot, mid_rot, bot_rot) = (
            bristle_flow * 0.6 + 0.1 * (age_ticks * 0.4).sin(),
            bristle_flow * 1.2 + 0.1 * (age_ticks * 0.2).sin(),
            bristle_flow * 1.3 + 0.05 * (age_ticks * -0.4).sin(),
        );
        if let Some(p) = tree.part_mut(top) {
            p.pose.rotation.z = initial_z_rots.0 + top_rot;
        }
        if let Some(p) = tree.part_mut(mid) {
            p.pose.rotation.z = initial_z_rots.1 + mid_rot;
        }
        if let Some(p) = tree.part_mut(bot) {
            p.pose.rotation.z = initial_z_rots.2 + bot_rot;
        }
    };

    animate_bristles(
        tree,
        "right_top_bristle",
        "right_middle_bristle",
        "right_bottom_bristle",
        (-0.87266463, -1.134464, -1.2217305),
    );
    animate_bristles(
        tree,
        "left_top_bristle",
        "left_middle_bristle",
        "left_bottom_bristle",
        (0.87266463, 1.134464, 1.2217305),
    );
}

pub fn build_strider_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_strider_model_tree();
    tree.reset_pose();
    apply_strider_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 128.0,
    })
}
