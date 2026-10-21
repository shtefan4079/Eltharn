use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_armadillo_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, 21.0, 4.0))
                    .cube(cube_grown(
                        [0.0, 20.0],
                        [-4.0, -7.0, -10.0],
                        [8.0, 8.0, 12.0],
                        0.3,
                    ))
                    .cube(cube([0.0, 40.0], [-4.0, -7.0, -10.0], [8.0, 8.0, 12.0]))
                    .child(
                        NamedPartSpec::new(
                            "tail",
                            PartPose::offset_and_rotation(0.0, -3.0, 1.0, 0.5061, 0.0, 0.0),
                        )
                        .cube(cube(
                            [44.0, 53.0],
                            [-0.5, -0.0865, 0.0933],
                            [1.0, 6.0, 1.0],
                        )),
                    )
                    .child(
                        NamedPartSpec::new("head", PartPose::offset(0.0, -2.0, -11.0))
                            .child(
                                NamedPartSpec::new(
                                    "head_cube",
                                    PartPose::offset_and_rotation(0.0, 0.0, 0.0, -0.3927, 0.0, 0.0),
                                )
                                .cube(cube(
                                    [43.0, 15.0],
                                    [-1.5, -1.0, -1.0],
                                    [3.0, 5.0, 2.0],
                                )),
                            )
                            .child(
                                NamedPartSpec::new("right_ear", PartPose::offset(-1.0, -1.0, 0.0))
                                    .child(
                                        NamedPartSpec::new(
                                            "right_ear_cube",
                                            PartPose::offset_and_rotation(
                                                -0.5, 0.0, -0.6, 0.1886, -0.3864, -0.0718,
                                            ),
                                        )
                                        .cube(cube(
                                            [43.0, 10.0],
                                            [-2.0, -3.0, 0.0],
                                            [2.0, 5.0, 0.0],
                                        )),
                                    ),
                            )
                            .child(
                                NamedPartSpec::new("left_ear", PartPose::offset(1.0, -2.0, 0.0))
                                    .child(
                                        NamedPartSpec::new(
                                            "left_ear_cube",
                                            PartPose::offset_and_rotation(
                                                0.5, 1.0, -0.6, 0.1886, 0.3864, 0.0718,
                                            ),
                                        )
                                        .cube(cube(
                                            [47.0, 10.0],
                                            [0.0, -3.0, 0.0],
                                            [2.0, 5.0, 0.0],
                                        )),
                                    ),
                            ),
                    ),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-2.0, 21.0, 4.0)).cube(cube(
                    [51.0, 31.0],
                    [-1.0, 0.0, -1.0],
                    [2.0, 3.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(2.0, 21.0, 4.0)).cube(cube(
                    [42.0, 31.0],
                    [-1.0, 0.0, -1.0],
                    [2.0, 3.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-2.0, 21.0, -4.0))
                    .cube(cube([51.0, 43.0], [-1.0, 0.0, -1.0], [2.0, 3.0, 2.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(2.0, 21.0, -4.0)).cube(cube(
                    [42.0, 43.0],
                    [-1.0, 0.0, -1.0],
                    [2.0, 3.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("cube", PartPose::offset(0.0, 24.0, 0.0))
                    .visible(false)
                    .cube(cube([0.0, 0.0], [-5.0, -10.0, -6.0], [10.0, 10.0, 10.0])),
            );

    JavaModelPartTree::from_root(root)
}

pub fn build_armadillo_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_armadillo_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

pub fn apply_armadillo_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let deg2rad = std::f32::consts::PI / 180.0;
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;

    if state.is_hiding_in_shell {
        if let Some(b) = tree.part_mut("body") {
            b.skip_draw = true;
        }
        if let Some(l) = tree.part_mut("left_hind_leg") {
            l.visible = false;
        }
        if let Some(l) = tree.part_mut("right_hind_leg") {
            l.visible = false;
        }
        if let Some(l) = tree.part_mut("left_front_leg") {
            l.visible = false;
        }
        if let Some(l) = tree.part_mut("right_front_leg") {
            l.visible = false;
        }
        if let Some(t) = tree.part_mut("tail") {
            t.visible = false;
        }
        if let Some(c) = tree.part_mut("cube") {
            c.visible = true;
        }
    } else {
        if let Some(b) = tree.part_mut("body") {
            b.skip_draw = false;
        }
        if let Some(l) = tree.part_mut("left_hind_leg") {
            l.visible = true;
        }
        if let Some(l) = tree.part_mut("right_hind_leg") {
            l.visible = true;
        }
        if let Some(l) = tree.part_mut("left_front_leg") {
            l.visible = true;
        }
        if let Some(l) = tree.part_mut("right_front_leg") {
            l.visible = true;
        }
        if let Some(t) = tree.part_mut("tail") {
            t.visible = true;
        }
        if let Some(c) = tree.part_mut("cube") {
            c.visible = false;
        }

        if let Some(h) = tree.part_mut("head") {
            h.pose.rotation.x = state.x_rot_degrees.clamp(-22.5, 25.0) * deg2rad;
            h.pose.rotation.y = state.y_rot_degrees.clamp(-32.5, 32.5) * deg2rad;
        }
        if let Some(l) = tree.part_mut("left_hind_leg") {
            l.pose.rotation.x = (anim_pos * 0.6662).cos() * anim_speed;
        }
        if let Some(l) = tree.part_mut("right_hind_leg") {
            l.pose.rotation.x = (anim_pos * 0.6662 + std::f32::consts::PI).cos() * anim_speed;
        }
    }
}

pub fn build_armadillo_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_armadillo_model_tree();
    tree.reset_pose();
    apply_armadillo_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
