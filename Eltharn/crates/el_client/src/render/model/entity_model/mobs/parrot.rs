use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_parrot_model_tree() -> JavaModelPartTree {
    let pi = std::f32::consts::PI;
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new(
                    "body",
                    PartPose::offset_and_rotation(0.0, 16.5, -3.0, 0.4937, 0.0, 0.0),
                )
                .cube(cube([2.0, 8.0], [-1.5, 0.0, -1.5], [3.0, 6.0, 3.0])),
            )
            .child(
                NamedPartSpec::new(
                    "tail",
                    PartPose::offset_and_rotation(0.0, 21.07, 1.16, 1.015, 0.0, 0.0),
                )
                .cube(cube([22.0, 1.0], [-1.5, -1.0, -1.0], [3.0, 4.0, 1.0])),
            )
            .child(
                NamedPartSpec::new(
                    "left_wing",
                    PartPose::offset_and_rotation(1.5, 16.94, -2.76, -0.6981, -pi, 0.0),
                )
                .cube(cube([19.0, 8.0], [-0.5, 0.0, -1.5], [1.0, 5.0, 3.0])),
            )
            .child(
                NamedPartSpec::new(
                    "right_wing",
                    PartPose::offset_and_rotation(-1.5, 16.94, -2.76, -0.6981, -pi, 0.0),
                )
                .cube(cube([19.0, 8.0], [-0.5, 0.0, -1.5], [1.0, 5.0, 3.0])),
            )
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 15.69, -2.76))
                    .cube(cube([2.0, 2.0], [-1.0, -1.5, -1.0], [2.0, 3.0, 2.0]))
                    .child(
                        NamedPartSpec::new("head2", PartPose::offset(0.0, -2.0, -1.0)).cube(cube(
                            [10.0, 0.0],
                            [-1.0, -0.5, -2.0],
                            [2.0, 1.0, 4.0],
                        )),
                    )
                    .child(
                        NamedPartSpec::new("beak1", PartPose::offset(0.0, -0.5, -1.5)).cube(cube(
                            [11.0, 7.0],
                            [-0.5, -1.0, -0.5],
                            [1.0, 2.0, 1.0],
                        )),
                    )
                    .child(
                        NamedPartSpec::new("beak2", PartPose::offset(0.0, -1.75, -2.45))
                            .cube(cube([16.0, 7.0], [-0.5, 0.0, -0.5], [1.0, 2.0, 1.0])),
                    )
                    .child(
                        NamedPartSpec::new(
                            "feather",
                            PartPose::offset_and_rotation(0.0, -2.15, 0.15, -0.2214, 0.0, 0.0),
                        )
                        .cube(cube(
                            [2.0, 18.0],
                            [0.0, -4.0, -2.0],
                            [0.0, 5.0, 4.0],
                        )),
                    ),
            )
            .child(
                NamedPartSpec::new(
                    "left_leg",
                    PartPose::offset_and_rotation(1.0, 22.0, -1.05, -0.0299, 0.0, 0.0),
                )
                .cube(cube([14.0, 18.0], [-0.5, 0.0, -0.5], [1.0, 2.0, 1.0])),
            )
            .child(
                NamedPartSpec::new(
                    "right_leg",
                    PartPose::offset_and_rotation(-1.0, 22.0, -1.05, -0.0299, 0.0, 0.0),
                )
                .cube(cube([14.0, 18.0], [-0.5, 0.0, -0.5], [1.0, 2.0, 1.0])),
            );

    JavaModelPartTree::from_root(root)
}

pub fn build_parrot_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_parrot_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParrotPose {
    Flying,
    Standing,
    Sitting,
    Party,
}

impl ParrotPose {
    fn from_state(state: &EntityModelAnimationState) -> Self {
        if state.is_dancing {
            Self::Party
        } else if state.is_sitting {
            Self::Sitting
        } else if !state.is_on_ground {
            Self::Flying
        } else {
            Self::Standing
        }
    }
}

pub fn apply_parrot_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;
    let flap_angle = (state.flap.sin() + 1.0) * state.flap_speed;

    let pose = ParrotPose::from_state(&state);

    match pose {
        ParrotPose::Flying => {
            if let Some(l) = tree.part_mut("left_leg") {
                l.pose.rotation.x += std::f32::consts::PI * 2.0 / 9.0;
            }
            if let Some(r) = tree.part_mut("right_leg") {
                r.pose.rotation.x += std::f32::consts::PI * 2.0 / 9.0;
            }
        }
        ParrotPose::Sitting => {
            if let Some(h) = tree.part_mut("head") {
                h.pose.offset.y += 1.0;
            }
            if let Some(t) = tree.part_mut("tail") {
                t.pose.rotation.x += std::f32::consts::PI / 6.0;
                t.pose.offset.y += 1.0;
            }
            if let Some(b) = tree.part_mut("body") {
                b.pose.offset.y += 1.0;
            }
            if let Some(l) = tree.part_mut("left_wing") {
                l.pose.rotation.z = -0.0873;
                l.pose.offset.y += 1.0;
            }
            if let Some(r) = tree.part_mut("right_wing") {
                r.pose.rotation.z = 0.0873;
                r.pose.offset.y += 1.0;
            }
            if let Some(l) = tree.part_mut("left_leg") {
                l.pose.offset.y += 1.0;
                l.pose.rotation.x += 1.0;
            }
            if let Some(r) = tree.part_mut("right_leg") {
                r.pose.offset.y += 1.0;
                r.pose.rotation.x += 1.0;
            }
        }
        ParrotPose::Party => {
            if let Some(l) = tree.part_mut("left_leg") {
                l.pose.rotation.z = -std::f32::consts::PI / 9.0;
            }
            if let Some(r) = tree.part_mut("right_leg") {
                r.pose.rotation.z = std::f32::consts::PI / 9.0;
            }
        }
        ParrotPose::Standing => {}
    }

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    }

    match pose {
        ParrotPose::Party => {
            let x_pos = state.age_in_ticks.cos();
            let y_pos = state.age_in_ticks.sin();
            if let Some(h) = tree.part_mut("head") {
                h.pose.offset.x += x_pos;
                h.pose.offset.y += y_pos;
                h.pose.rotation.x = 0.0;
                h.pose.rotation.y = 0.0;
                h.pose.rotation.z = state.age_in_ticks.sin() * 0.4;
            }
            if let Some(b) = tree.part_mut("body") {
                b.pose.offset.x += x_pos;
                b.pose.offset.y += y_pos;
            }
            if let Some(l) = tree.part_mut("left_wing") {
                l.pose.rotation.z = -0.0873 - flap_angle;
                l.pose.offset.x += x_pos;
                l.pose.offset.y += y_pos;
            }
            if let Some(r) = tree.part_mut("right_wing") {
                r.pose.rotation.z = 0.0873 + flap_angle;
                r.pose.offset.x += x_pos;
                r.pose.offset.y += y_pos;
            }
            if let Some(t) = tree.part_mut("tail") {
                t.pose.offset.x += x_pos;
                t.pose.offset.y += y_pos;
            }
        }
        _ => {
            if pose == ParrotPose::Standing {
                if let Some(l) = tree.part_mut("left_leg") {
                    l.pose.rotation.x += (anim_pos * 0.6662).cos() * 1.4 * anim_speed;
                }
                if let Some(r) = tree.part_mut("right_leg") {
                    r.pose.rotation.x +=
                        (anim_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * anim_speed;
                }
            }
            if pose == ParrotPose::Standing || pose == ParrotPose::Flying {
                let bobbing_body = flap_angle * 0.3;
                if let Some(h) = tree.part_mut("head") {
                    h.pose.offset.y += bobbing_body;
                }
                if let Some(t) = tree.part_mut("tail") {
                    t.pose.rotation.x += (anim_pos * 0.6662).cos() * 0.3 * anim_speed;
                    t.pose.offset.y += bobbing_body;
                }
                if let Some(b) = tree.part_mut("body") {
                    b.pose.offset.y += bobbing_body;
                }
                if let Some(l) = tree.part_mut("left_wing") {
                    l.pose.rotation.z = -0.0873 - flap_angle;
                    l.pose.offset.y += bobbing_body;
                }
                if let Some(r) = tree.part_mut("right_wing") {
                    r.pose.rotation.z = 0.0873 + flap_angle;
                    r.pose.offset.y += bobbing_body;
                }
                if let Some(l) = tree.part_mut("left_leg") {
                    l.pose.offset.y += bobbing_body;
                }
                if let Some(r) = tree.part_mut("right_leg") {
                    r.pose.offset.y += bobbing_body;
                }
            }
        }
    }
}

pub fn build_parrot_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_parrot_model_tree();
    tree.reset_pose();
    apply_parrot_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
