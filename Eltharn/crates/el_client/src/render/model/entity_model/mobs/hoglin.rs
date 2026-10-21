use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_hoglin_model_tree() -> JavaModelPartTree {
    let pi = std::f32::consts::PI;
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, 7.0, 0.0))
                    .cube(cube([1.0, 1.0], [-8.0, -7.0, -13.0], [16.0, 14.0, 26.0]))
                    .child(
                        NamedPartSpec::new("mane", PartPose::offset(0.0, -14.0, -7.0)).cube(
                            cube_grown([90.0, 33.0], [0.0, 0.0, -9.0], [0.0, 10.0, 19.0], 0.001),
                        ),
                    ),
            )
            .child(
                NamedPartSpec::new(
                    "head",
                    PartPose::offset_and_rotation(0.0, 2.0, -12.0, 0.87266463, 0.0, 0.0),
                )
                .cube(cube([61.0, 1.0], [-7.0, -3.0, -19.0], [14.0, 6.0, 19.0]))
                .child(
                    NamedPartSpec::new(
                        "right_ear",
                        PartPose::offset_and_rotation(-6.0, -2.0, -3.0, 0.0, 0.0, -pi * 2.0 / 9.0),
                    )
                    .cube(cube(
                        [1.0, 1.0],
                        [-6.0, -1.0, -2.0],
                        [6.0, 1.0, 4.0],
                    )),
                )
                .child(
                    NamedPartSpec::new(
                        "left_ear",
                        PartPose::offset_and_rotation(6.0, -2.0, -3.0, 0.0, 0.0, pi * 2.0 / 9.0),
                    )
                    .cube(cube([1.0, 6.0], [0.0, -1.0, -2.0], [6.0, 1.0, 4.0])),
                )
                .child(
                    NamedPartSpec::new("right_horn", PartPose::offset(-7.0, 2.0, -12.0))
                        .cube(cube([10.0, 13.0], [-1.0, -11.0, -1.0], [2.0, 11.0, 2.0])),
                )
                .child(
                    NamedPartSpec::new("left_horn", PartPose::offset(7.0, 2.0, -12.0)).cube(cube(
                        [1.0, 13.0],
                        [-1.0, -11.0, -1.0],
                        [2.0, 11.0, 2.0],
                    )),
                ),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-4.0, 10.0, -8.5))
                    .cube(cube([66.0, 42.0], [-3.0, 0.0, -3.0], [6.0, 14.0, 6.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(4.0, 10.0, -8.5)).cube(cube(
                    [41.0, 42.0],
                    [-3.0, 0.0, -3.0],
                    [6.0, 14.0, 6.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-5.0, 13.0, 10.0))
                    .cube(cube([21.0, 45.0], [-2.5, 0.0, -2.5], [5.0, 11.0, 5.0])),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(5.0, 13.0, 10.0)).cube(cube(
                    [0.0, 45.0],
                    [-2.5, 0.0, -2.5],
                    [5.0, 11.0, 5.0],
                )),
            );

    JavaModelPartTree::from_root(root)
}

pub fn build_hoglin_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_hoglin_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}

pub fn apply_hoglin_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let pi = std::f32::consts::PI;
    let deg2rad = pi / 180.0;
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;

    if let Some(re) = tree.part_mut("right_ear") {
        re.pose.rotation.z = (-pi * 2.0 / 9.0) - anim_speed * anim_pos.sin();
    }
    if let Some(le) = tree.part_mut("left_ear") {
        le.pose.rotation.z = (pi * 2.0 / 9.0) + anim_speed * anim_pos.sin();
    }
    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.y = state.y_rot_degrees * deg2rad;
    }

    let leg_rot = anim_pos.cos() * 1.2 * anim_speed;
    let leg_rot_opp = (anim_pos + pi).cos() * 1.2 * anim_speed;

    if let Some(l) = tree.part_mut("right_front_leg") {
        l.pose.rotation.x = leg_rot;
    }
    if let Some(l) = tree.part_mut("left_front_leg") {
        l.pose.rotation.x = leg_rot_opp;
    }
    if let Some(l) = tree.part_mut("right_hind_leg") {
        l.pose.rotation.x = leg_rot_opp;
    }
    if let Some(l) = tree.part_mut("left_hind_leg") {
        l.pose.rotation.x = leg_rot;
    }
}

pub fn build_hoglin_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_hoglin_model_tree();
    tree.reset_pose();
    apply_hoglin_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
