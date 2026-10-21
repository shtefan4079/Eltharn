use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_phantom_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("body", PartPose::rotation(-0.1, 0.0, 0.0))
            .cube(cube([0.0, 8.0], [-3.0, -2.0, -8.0], [5.0, 3.0, 9.0]))
            .child(
                NamedPartSpec::new(
                    "head",
                    PartPose::offset_and_rotation(0.0, 1.0, -7.0, 0.2, 0.0, 0.0),
                )
                .cube(cube([0.0, 0.0], [-4.0, -2.0, -5.0], [7.0, 3.0, 5.0])),
            )
            .child(
                NamedPartSpec::new("tail_base", PartPose::offset(0.0, -2.0, 1.0))
                    .cube(cube([3.0, 20.0], [-2.0, 0.0, 0.0], [3.0, 2.0, 6.0]))
                    .child(
                        NamedPartSpec::new("tail_tip", PartPose::offset(0.0, 0.5, 6.0)).cube(cube(
                            [4.0, 29.0],
                            [-1.0, 0.0, 0.0],
                            [1.0, 1.0, 6.0],
                        )),
                    ),
            )
            .child(
                NamedPartSpec::new(
                    "left_wing_base",
                    PartPose::offset_and_rotation(2.0, -2.0, -8.0, 0.0, 0.0, 0.1),
                )
                .cube(cube([23.0, 12.0], [0.0, 0.0, 0.0], [6.0, 2.0, 9.0]))
                .child(
                    NamedPartSpec::new(
                        "left_wing_tip",
                        PartPose::offset_and_rotation(6.0, 0.0, 0.0, 0.0, 0.0, 0.1),
                    )
                    .cube(cube(
                        [16.0, 24.0],
                        [0.0, 0.0, 0.0],
                        [13.0, 1.0, 9.0],
                    )),
                ),
            )
            .child(
                NamedPartSpec::new(
                    "right_wing_base",
                    PartPose::offset_and_rotation(-3.0, -2.0, -8.0, 0.0, 0.0, -0.1),
                )
                .cube(cube_mirrored(
                    [23.0, 12.0],
                    [-6.0, 0.0, 0.0],
                    [6.0, 2.0, 9.0],
                ))
                .child(
                    NamedPartSpec::new(
                        "right_wing_tip",
                        PartPose::offset_and_rotation(-6.0, 0.0, 0.0, 0.0, 0.0, -0.1),
                    )
                    .cube(cube_mirrored(
                        [16.0, 24.0],
                        [-13.0, 0.0, 0.0],
                        [13.0, 1.0, 9.0],
                    )),
                ),
            ),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_phantom_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_phantom_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

pub fn apply_phantom_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim = state.age_in_ticks * 7.448451 * (std::f32::consts::PI / 180.0);
    let wing_z = anim.cos() * 16.0 * (std::f32::consts::PI / 180.0);

    if let Some(lwb) = tree.part_mut("left_wing_base") {
        lwb.pose.rotation.z = wing_z;
    }
    if let Some(lwt) = tree.part_mut("left_wing_tip") {
        lwt.pose.rotation.z = wing_z;
    }
    if let Some(rwb) = tree.part_mut("right_wing_base") {
        rwb.pose.rotation.z = -wing_z;
    }
    if let Some(rwt) = tree.part_mut("right_wing_tip") {
        rwt.pose.rotation.z = -wing_z;
    }

    let tail_x = -(5.0 + (anim * 2.0).cos() * 5.0) * (std::f32::consts::PI / 180.0);
    if let Some(tb) = tree.part_mut("tail_base") {
        tb.pose.rotation.x = tail_x;
    }
    if let Some(tt) = tree.part_mut("tail_tip") {
        tt.pose.rotation.x = tail_x;
    }
}

pub fn build_phantom_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_phantom_model_tree();
    tree.reset_pose();
    apply_phantom_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
