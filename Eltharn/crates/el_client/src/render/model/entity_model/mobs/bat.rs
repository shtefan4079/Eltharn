use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_bat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 17.0, 0.0))
                .cube(cube([0.0, 0.0], [-1.5, 0.0, -1.0], [3.0, 5.0, 2.0]))
                .child(
                    NamedPartSpec::new("right_wing", PartPose::offset(-1.5, 0.0, 0.0))
                        .cube(cube([12.0, 0.0], [-2.0, -2.0, 0.0], [2.0, 7.0, 0.0]))
                        .child(
                            NamedPartSpec::new("right_wing_tip", PartPose::offset(-2.0, 0.0, 0.0))
                                .cube(cube([16.0, 0.0], [-6.0, -2.0, 0.0], [6.0, 8.0, 0.0])),
                        ),
                )
                .child(
                    NamedPartSpec::new("left_wing", PartPose::offset(1.5, 0.0, 0.0))
                        .cube(cube([12.0, 7.0], [0.0, -2.0, 0.0], [2.0, 7.0, 0.0]))
                        .child(
                            NamedPartSpec::new("left_wing_tip", PartPose::offset(2.0, 0.0, 0.0))
                                .cube(cube([16.0, 8.0], [0.0, -2.0, 0.0], [6.0, 8.0, 0.0])),
                        ),
                )
                .child(
                    NamedPartSpec::new("feet", PartPose::offset(0.0, 5.0, 0.0)).cube(cube(
                        [16.0, 16.0],
                        [-1.5, 0.0, 0.0],
                        [3.0, 2.0, 0.0],
                    )),
                ),
        )
        .child(
            NamedPartSpec::new("head", PartPose::offset(0.0, 17.0, 0.0))
                .cube(cube([0.0, 7.0], [-2.0, -3.0, -1.0], [4.0, 3.0, 2.0]))
                .child(
                    NamedPartSpec::new("right_ear", PartPose::offset(-1.5, -2.0, 0.0)).cube(cube(
                        [1.0, 15.0],
                        [-2.5, -4.0, 0.0],
                        [3.0, 5.0, 0.0],
                    )),
                )
                .child(
                    NamedPartSpec::new("left_ear", PartPose::offset(1.1, -3.0, 0.0)).cube(cube(
                        [8.0, 15.0],
                        [-0.1, -3.0, 0.0],
                        [3.0, 5.0, 0.0],
                    )),
                ),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_bat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_bat_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}

pub fn apply_bat_model_setup_anim(tree: &mut JavaModelPartTree, state: EntityModelAnimationState) {
    let deg2rad = std::f32::consts::PI / 180.0;
    if state.is_resting {
        if let Some(h) = tree.part_mut("head") {
            h.pose.rotation.y = state.y_rot_degrees * deg2rad;
        }
    }

    super::bat_animation::BAT_FLYING.apply_state(
        tree,
        &state.bat_fly_animation_state,
        state.age_in_ticks,
    );
    super::bat_animation::BAT_RESTING.apply_state(
        tree,
        &state.bat_rest_animation_state,
        state.age_in_ticks,
    );
}

pub fn build_bat_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_bat_model_tree();
    tree.reset_pose();
    apply_bat_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
