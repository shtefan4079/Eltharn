use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_sniffer_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("bone", PartPose::offset(0.0, 5.0, 0.0))
            .child(
                NamedPartSpec::new("body", PartPose::ZERO)
                    .cube(cube(
                        [62.0, 68.0],
                        [-12.5, -14.0, -20.0],
                        [25.0, 29.0, 40.0],
                    ))
                    .cube(cube_grown(
                        [62.0, 0.0],
                        [-12.5, -14.0, -20.0],
                        [25.0, 24.0, 40.0],
                        0.5,
                    ))
                    .cube(cube([87.0, 68.0], [-12.5, 12.0, -20.0], [25.0, 0.0, 40.0]))
                    .child(
                        NamedPartSpec::new("head", PartPose::offset(0.0, 6.5, -19.48))
                            .cube(cube([8.0, 15.0], [-6.5, -7.5, -11.5], [13.0, 18.0, 11.0]))
                            .cube(cube([8.0, 4.0], [-6.5, 7.5, -11.5], [13.0, 0.0, 11.0]))
                            .child(
                                NamedPartSpec::new("left_ear", PartPose::offset(6.51, -7.5, -4.51))
                                    .cube(cube([2.0, 0.0], [0.0, 0.0, -3.0], [1.0, 19.0, 7.0])),
                            )
                            .child(
                                NamedPartSpec::new(
                                    "right_ear",
                                    PartPose::offset(-6.51, -7.5, -4.51),
                                )
                                .cube(cube(
                                    [48.0, 0.0],
                                    [-1.0, 0.0, -3.0],
                                    [1.0, 19.0, 7.0],
                                )),
                            )
                            .child(
                                NamedPartSpec::new("nose", PartPose::offset(0.0, -4.5, -11.5))
                                    .cube(cube([10.0, 45.0], [-6.5, -2.0, -9.0], [13.0, 2.0, 9.0])),
                            )
                            .child(
                                NamedPartSpec::new("lower_beak", PartPose::offset(0.0, 2.5, -12.5))
                                    .cube(cube(
                                        [10.0, 57.0],
                                        [-6.5, -7.0, -8.0],
                                        [13.0, 12.0, 9.0],
                                    )),
                            ),
                    ),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-7.5, 10.0, -15.0))
                    .cube(cube([32.0, 87.0], [-3.5, -1.0, -4.0], [7.0, 10.0, 8.0])),
            )
            .child(
                NamedPartSpec::new("right_mid_leg", PartPose::offset(-7.5, 10.0, 0.0)).cube(cube(
                    [32.0, 105.0],
                    [-3.5, -1.0, -4.0],
                    [7.0, 10.0, 8.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-7.5, 10.0, 15.0))
                    .cube(cube([32.0, 123.0], [-3.5, -1.0, -4.0], [7.0, 10.0, 8.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(7.5, 10.0, -15.0))
                    .cube(cube([0.0, 87.0], [-3.5, -1.0, -4.0], [7.0, 10.0, 8.0])),
            )
            .child(
                NamedPartSpec::new("left_mid_leg", PartPose::offset(7.5, 10.0, 0.0)).cube(cube(
                    [0.0, 105.0],
                    [-3.5, -1.0, -4.0],
                    [7.0, 10.0, 8.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(7.5, 10.0, 15.0)).cube(cube(
                    [0.0, 123.0],
                    [-3.5, -1.0, -4.0],
                    [7.0, 10.0, 8.0],
                )),
            ),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_sniffer_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_sniffer_model_tree().mesh(ModelTextureSize {
        width: 192.0,
        height: 192.0,
    })
}

pub fn apply_sniffer_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    }
    let leg_rot = (anim_pos * 0.6662).cos() * 1.4 * anim_speed;
    if let Some(l) = tree.part_mut("right_front_leg") {
        l.pose.rotation.x = leg_rot;
    }
    if let Some(l) = tree.part_mut("left_front_leg") {
        l.pose.rotation.x = -leg_rot;
    }
    if let Some(l) = tree.part_mut("right_mid_leg") {
        l.pose.rotation.x = -leg_rot;
    }
    if let Some(l) = tree.part_mut("left_mid_leg") {
        l.pose.rotation.x = leg_rot;
    }
    if let Some(l) = tree.part_mut("right_hind_leg") {
        l.pose.rotation.x = leg_rot;
    }
    if let Some(l) = tree.part_mut("left_hind_leg") {
        l.pose.rotation.x = -leg_rot;
    }
}

pub fn build_sniffer_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_sniffer_model_tree();
    tree.reset_pose();
    apply_sniffer_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 192.0,
        height: 192.0,
    })
}
