use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_breeze_body_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("body", PartPose::ZERO)
            .child(
                NamedPartSpec::new("rods", PartPose::offset(0.0, 8.0, 0.0))
                    .child(
                        NamedPartSpec::new(
                            "rod_1",
                            PartPose::offset_and_rotation(
                                2.5981, -3.0, 1.5, -2.7489, -1.0472, 3.1416,
                            ),
                        )
                        .cube(cube(
                            [0.0, 17.0],
                            [-1.0, 0.0, -3.0],
                            [2.0, 8.0, 2.0],
                        )),
                    )
                    .child(
                        NamedPartSpec::new(
                            "rod_2",
                            PartPose::offset_and_rotation(
                                -2.5981, -3.0, 1.5, -2.7489, 1.0472, 3.1416,
                            ),
                        )
                        .cube(cube(
                            [0.0, 17.0],
                            [-1.0, 0.0, -3.0],
                            [2.0, 8.0, 2.0],
                        )),
                    )
                    .child(
                        NamedPartSpec::new(
                            "rod_3",
                            PartPose::offset_and_rotation(0.0, -3.0, -3.0, 0.3927, 0.0, 0.0),
                        )
                        .cube(cube(
                            [0.0, 17.0],
                            [-1.0, 0.0, -3.0],
                            [2.0, 8.0, 2.0],
                        )),
                    ),
            )
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 4.0, 0.0))
                    .cube(cube([4.0, 24.0], [-5.0, -5.0, -4.2], [10.0, 3.0, 4.0]))
                    .cube(cube([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]))
                    .child(
                        NamedPartSpec::new("eyes", PartPose::ZERO)
                            .cube(cube([4.0, 24.0], [-5.0, -5.0, -4.2], [10.0, 3.0, 4.0]))
                            .cube(cube([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0])),
                    ),
            ),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_breeze_wind_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("wind_body", PartPose::ZERO).child(
            NamedPartSpec::new("wind_bottom", PartPose::offset(0.0, 24.0, 0.0))
                .cube(cube([1.0, 83.0], [-2.5, -7.0, -2.5], [5.0, 7.0, 5.0]))
                .child(
                    NamedPartSpec::new("wind_mid", PartPose::offset(0.0, -7.0, 0.0))
                        .cube(cube([74.0, 28.0], [-6.0, -6.0, -6.0], [12.0, 6.0, 12.0]))
                        .cube(cube([78.0, 32.0], [-4.0, -6.0, -4.0], [8.0, 6.0, 8.0]))
                        .cube(cube([49.0, 71.0], [-2.5, -6.0, -2.5], [5.0, 6.0, 5.0]))
                        .child(
                            NamedPartSpec::new("wind_top", PartPose::offset(0.0, -6.0, 0.0))
                                .cube(cube([0.0, 0.0], [-9.0, -8.0, -9.0], [18.0, 8.0, 18.0]))
                                .cube(cube([6.0, 6.0], [-6.0, -8.0, -6.0], [12.0, 8.0, 12.0]))
                                .cube(cube([105.0, 57.0], [-2.5, -8.0, -2.5], [5.0, 8.0, 5.0])),
                        ),
                ),
        ),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_breeze_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let (mut body_v, mut body_i) = build_breeze_body_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    });
    let (wind_v, wind_i) = build_breeze_wind_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    });
    let offset = body_v.len() as u32;
    body_v.extend(wind_v);
    body_i.extend(wind_i.into_iter().map(|idx| idx + offset));
    (body_v, body_i)
}

pub fn apply_breeze_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    }
}

pub fn build_breeze_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_breeze_body_model_tree();
    tree.reset_pose();
    apply_breeze_model_setup_anim(&mut tree, state);
    let (mut body_v, mut body_i) = tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    });
    let (wind_v, wind_i) = build_breeze_wind_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    });
    let offset = body_v.len() as u32;
    body_v.extend(wind_v);
    body_i.extend(wind_i.into_iter().map(|idx| idx + offset));
    (body_v, body_i)
}
