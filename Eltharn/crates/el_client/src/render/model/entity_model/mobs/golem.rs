use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_iron_golem_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, -7.0, -2.0))
                    .cube(cube([0.0, 0.0], [-4.0, -12.0, -5.5], [8.0, 10.0, 8.0]))
                    .cube(cube([24.0, 0.0], [-1.0, -5.0, -7.5], [2.0, 4.0, 2.0])),
            )
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, -7.0, 0.0))
                    .cube(cube([0.0, 40.0], [-9.0, -2.0, -6.0], [18.0, 12.0, 11.0]))
                    .cube(cube_grown(
                        [0.0, 70.0],
                        [-4.5, 10.0, -3.0],
                        [9.0, 5.0, 6.0],
                        0.5,
                    )),
            )
            .child(
                NamedPartSpec::new("right_arm", PartPose::offset(0.0, -7.0, 0.0)).cube(cube(
                    [60.0, 21.0],
                    [-13.0, -2.5, -3.0],
                    [4.0, 30.0, 6.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_arm", PartPose::offset(0.0, -7.0, 0.0)).cube(cube(
                    [60.0, 58.0],
                    [9.0, -2.5, -3.0],
                    [4.0, 30.0, 6.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_leg", PartPose::offset(-4.0, 11.0, 0.0)).cube(cube(
                    [37.0, 0.0],
                    [-3.5, -3.0, -3.0],
                    [6.0, 16.0, 5.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_leg", PartPose::offset(5.0, 11.0, 0.0)).cube(
                    cube_mirrored([60.0, 0.0], [-3.5, -3.0, -3.0], [6.0, 16.0, 5.0]),
                ),
            );

    JavaModelPartTree::from_root(root)
}

pub fn build_iron_golem_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_iron_golem_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}

fn triangle_wave(x: f32, period: f32) -> f32 {
    let m = (x % period + period) % period;
    let half = period * 0.5;
    (m - half).abs() - half * 0.5
}

pub fn apply_iron_golem_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let deg2rad = std::f32::consts::PI / 180.0;
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.y = state.y_rot_degrees * deg2rad;
        h.pose.rotation.x = state.x_rot_degrees * deg2rad;
    }

    let leg_wave = triangle_wave(anim_pos, 13.0);
    if let Some(ra) = tree.part_mut("right_arm") {
        ra.pose.rotation.x = (-0.2 + 1.5 * leg_wave) * anim_speed;
    }
    if let Some(la) = tree.part_mut("left_arm") {
        la.pose.rotation.x = (-0.2 - 1.5 * leg_wave) * anim_speed;
    }
    if let Some(rl) = tree.part_mut("right_leg") {
        rl.pose.rotation.x = -1.5 * leg_wave * anim_speed;
        rl.pose.rotation.y = 0.0;
    }
    if let Some(ll) = tree.part_mut("left_leg") {
        ll.pose.rotation.x = 1.5 * leg_wave * anim_speed;
        ll.pose.rotation.y = 0.0;
    }
}

pub fn build_iron_golem_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_iron_golem_model_tree();
    tree.reset_pose();
    apply_iron_golem_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}

pub fn build_snow_golem_model_tree() -> JavaModelPartTree {
    let arm_spec = cube_grown([32.0, 0.0], [-1.0, 0.0, -1.0], [12.0, 2.0, 2.0], -0.5);

    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 4.0, 0.0)).cube(cube_grown(
                    [0.0, 0.0],
                    [-4.0, -8.0, -4.0],
                    [8.0, 8.0, 8.0],
                    -0.5,
                )),
            )
            .child(
                NamedPartSpec::new(
                    "left_arm",
                    PartPose::offset_and_rotation(5.0, 6.0, 1.0, 0.0, 0.0, 1.0),
                )
                .cube(arm_spec.clone()),
            )
            .child(
                NamedPartSpec::new(
                    "right_arm",
                    PartPose::offset_and_rotation(-5.0, 6.0, -1.0, 0.0, std::f32::consts::PI, -1.0),
                )
                .cube(arm_spec),
            )
            .child(
                NamedPartSpec::new("upper_body", PartPose::offset(0.0, 13.0, 0.0)).cube(
                    cube_grown([0.0, 16.0], [-5.0, -10.0, -5.0], [10.0, 10.0, 10.0], -0.5),
                ),
            )
            .child(
                NamedPartSpec::new("lower_body", PartPose::offset(0.0, 24.0, 0.0)).cube(
                    cube_grown([0.0, 36.0], [-6.0, -12.0, -6.0], [12.0, 12.0, 12.0], -0.5),
                ),
            );

    JavaModelPartTree::from_root(root)
}

pub fn build_snow_golem_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_snow_golem_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

pub fn apply_snow_golem_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let deg2rad = std::f32::consts::PI / 180.0;
    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.y = state.y_rot_degrees * deg2rad;
        h.pose.rotation.x = state.x_rot_degrees * deg2rad;
    }

    let upper_y_rot = state.y_rot_degrees * deg2rad * 0.25;
    if let Some(ub) = tree.part_mut("upper_body") {
        ub.pose.rotation.y = upper_y_rot;
    }

    let sin = upper_y_rot.sin();
    let cos = upper_y_rot.cos();

    if let Some(la) = tree.part_mut("left_arm") {
        la.pose.rotation.y = upper_y_rot;
        la.pose.offset.x = cos * 5.0;
        la.pose.offset.z = -sin * 5.0;
    }

    if let Some(ra) = tree.part_mut("right_arm") {
        ra.pose.rotation.y = upper_y_rot + std::f32::consts::PI;
        ra.pose.offset.x = -cos * 5.0;
        ra.pose.offset.z = sin * 5.0;
    }
}

pub fn build_snow_golem_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_snow_golem_model_tree();
    tree.reset_pose();
    apply_snow_golem_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
