use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_axolotl_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("body", PartPose::offset(0.0, 19.5, 5.0))
            .cube(cube([0.0, 11.0], [-4.0, -2.0, -9.0], [8.0, 4.0, 10.0]))
            .cube(cube([2.0, 17.0], [0.0, -3.0, -8.0], [0.0, 5.0, 9.0]))
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 0.0, -9.0))
                    .cube(cube_grown(
                        [0.0, 1.0],
                        [-4.0, -3.0, -5.0],
                        [8.0, 5.0, 5.0],
                        0.001,
                    ))
                    .child(
                        NamedPartSpec::new("top_gills", PartPose::offset(0.0, -3.0, -1.0)).cube(
                            cube_grown([3.0, 37.0], [-4.0, -3.0, 0.0], [8.0, 3.0, 0.0], 0.001),
                        ),
                    )
                    .child(
                        NamedPartSpec::new("left_gills", PartPose::offset(-4.0, 0.0, -1.0)).cube(
                            cube_grown([0.0, 40.0], [-3.0, -5.0, 0.0], [3.0, 7.0, 0.0], 0.001),
                        ),
                    )
                    .child(
                        NamedPartSpec::new("right_gills", PartPose::offset(4.0, 0.0, -1.0)).cube(
                            cube_grown([11.0, 40.0], [0.0, -5.0, 0.0], [3.0, 7.0, 0.0], 0.001),
                        ),
                    ),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-3.5, 1.0, -1.0)).cube(
                    cube_grown([2.0, 13.0], [-2.0, 0.0, 0.0], [3.0, 5.0, 0.0], 0.001),
                ),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(3.5, 1.0, -1.0)).cube(
                    cube_grown([2.0, 13.0], [-1.0, 0.0, 0.0], [3.0, 5.0, 0.0], 0.001),
                ),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-3.5, 1.0, -8.0)).cube(
                    cube_grown([2.0, 13.0], [-2.0, 0.0, 0.0], [3.0, 5.0, 0.0], 0.001),
                ),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(3.5, 1.0, -8.0)).cube(
                    cube_grown([2.0, 13.0], [-1.0, 0.0, 0.0], [3.0, 5.0, 0.0], 0.001),
                ),
            )
            .child(
                NamedPartSpec::new("tail", PartPose::offset(0.0, 0.0, 1.0)).cube(cube(
                    [2.0, 19.0],
                    [0.0, -3.0, 0.0],
                    [0.0, 5.0, 12.0],
                )),
            ),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_axolotl_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_axolotl_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

pub fn apply_axolotl_model_setup_anim(
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
    if let Some(l) = tree.part_mut("left_hind_leg") {
        l.pose.rotation.x = (anim_pos * 0.6662).cos() * anim_speed;
    }
    if let Some(l) = tree.part_mut("right_hind_leg") {
        l.pose.rotation.x = (anim_pos * 0.6662 + std::f32::consts::PI).cos() * anim_speed;
    }
}

pub fn build_axolotl_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_axolotl_model_tree();
    tree.reset_pose();
    apply_axolotl_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
