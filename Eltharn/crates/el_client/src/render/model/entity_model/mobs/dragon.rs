use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_ender_dragon_model_tree() -> JavaModelPartTree {
    let mut root = NamedPartSpec::new("root", PartPose::ZERO);

    root = root.child(
        NamedPartSpec::new("head", PartPose::offset(0.0, 20.0, -62.0))
            .cube(cube([176.0, 44.0], [-6.0, -1.0, -24.0], [12.0, 5.0, 16.0]))
            .cube(cube([112.0, 30.0], [-8.0, -8.0, -10.0], [16.0, 16.0, 16.0]))
            .cube(cube_mirrored(
                [0.0, 0.0],
                [-5.0, -12.0, -4.0],
                [2.0, 4.0, 6.0],
            ))
            .cube(cube_mirrored(
                [112.0, 0.0],
                [-5.0, -3.0, -22.0],
                [2.0, 2.0, 4.0],
            ))
            .cube(cube_mirrored(
                [0.0, 0.0],
                [3.0, -12.0, -4.0],
                [2.0, 4.0, 6.0],
            ))
            .cube(cube_mirrored(
                [112.0, 0.0],
                [3.0, -3.0, -22.0],
                [2.0, 2.0, 4.0],
            ))
            .child(
                NamedPartSpec::new("jaw", PartPose::offset(0.0, 4.0, -8.0)).cube(cube(
                    [176.0, 65.0],
                    [-6.0, 0.0, -16.0],
                    [12.0, 4.0, 16.0],
                )),
            ),
    );

    const NECK_NAMES: [&str; 5] = ["neck0", "neck1", "neck2", "neck3", "neck4"];
    for i in 0..5 {
        root = root.child(
            NamedPartSpec::new(
                NECK_NAMES[i],
                PartPose::offset(0.0, 20.0, -12.0 - i as f32 * 10.0),
            )
            .cube(cube([192.0, 104.0], [-5.0, -5.0, -5.0], [10.0, 10.0, 10.0]))
            .cube(cube([48.0, 0.0], [-1.0, -9.0, -3.0], [2.0, 4.0, 6.0])),
        );
    }

    const TAIL_NAMES: [&str; 12] = [
        "tail0", "tail1", "tail2", "tail3", "tail4", "tail5", "tail6", "tail7", "tail8", "tail9",
        "tail10", "tail11",
    ];
    for i in 0..12 {
        root = root.child(
            NamedPartSpec::new(
                TAIL_NAMES[i],
                PartPose::offset(0.0, 10.0, 60.0 + i as f32 * 10.0),
            )
            .cube(cube([192.0, 104.0], [-5.0, -5.0, -5.0], [10.0, 10.0, 10.0]))
            .cube(cube([48.0, 0.0], [-1.0, -9.0, -3.0], [2.0, 4.0, 6.0])),
        );
    }

    let body = NamedPartSpec::new("body", PartPose::offset(0.0, 3.0, 8.0))
        .cube(cube([0.0, 0.0], [-12.0, 1.0, -16.0], [24.0, 24.0, 64.0]))
        .cube(cube([220.0, 53.0], [-1.0, -5.0, -10.0], [2.0, 6.0, 12.0]))
        .cube(cube([220.0, 53.0], [-1.0, -5.0, 10.0], [2.0, 6.0, 12.0]))
        .cube(cube([220.0, 53.0], [-1.0, -5.0, 30.0], [2.0, 6.0, 12.0]))
        .child(
            NamedPartSpec::new("left_wing", PartPose::offset(12.0, 2.0, -6.0))
                .cube(cube_mirrored(
                    [112.0, 88.0],
                    [0.0, -4.0, -4.0],
                    [56.0, 8.0, 8.0],
                ))
                .cube(cube_mirrored(
                    [-56.0, 88.0],
                    [0.0, 0.0, 2.0],
                    [56.0, 0.0, 56.0],
                ))
                .child(
                    NamedPartSpec::new("left_wing_tip", PartPose::offset(56.0, 0.0, 0.0))
                        .cube(cube_mirrored(
                            [112.0, 136.0],
                            [0.0, -2.0, -2.0],
                            [56.0, 4.0, 4.0],
                        ))
                        .cube(cube_mirrored(
                            [-56.0, 144.0],
                            [0.0, 0.0, 2.0],
                            [56.0, 0.0, 56.0],
                        )),
                ),
        )
        .child(
            NamedPartSpec::new("right_wing", PartPose::offset(-12.0, 2.0, -6.0))
                .cube(cube([112.0, 88.0], [-56.0, -4.0, -4.0], [56.0, 8.0, 8.0]))
                .cube(cube([-56.0, 88.0], [-56.0, 0.0, 2.0], [56.0, 0.0, 56.0]))
                .child(
                    NamedPartSpec::new("right_wing_tip", PartPose::offset(-56.0, 0.0, 0.0))
                        .cube(cube([112.0, 136.0], [-56.0, -2.0, -2.0], [56.0, 4.0, 4.0]))
                        .cube(cube([-56.0, 144.0], [-56.0, 0.0, 2.0], [56.0, 0.0, 56.0])),
                ),
        )
        .child(
            NamedPartSpec::new(
                "left_front_leg",
                PartPose::offset_and_rotation(12.0, 17.0, -6.0, 1.3, 0.0, 0.0),
            )
            .cube(cube([112.0, 104.0], [-4.0, -4.0, -4.0], [8.0, 24.0, 8.0]))
            .child(
                NamedPartSpec::new(
                    "left_front_leg_tip",
                    PartPose::offset_and_rotation(0.0, 20.0, -1.0, -0.5, 0.0, 0.0),
                )
                .cube(cube([226.0, 138.0], [-3.0, -1.0, -3.0], [6.0, 24.0, 6.0]))
                .child(
                    NamedPartSpec::new(
                        "left_front_foot",
                        PartPose::offset_and_rotation(0.0, 23.0, 0.0, 0.75, 0.0, 0.0),
                    )
                    .cube(cube(
                        [144.0, 104.0],
                        [-4.0, 0.0, -12.0],
                        [8.0, 4.0, 16.0],
                    )),
                ),
            ),
        )
        .child(
            NamedPartSpec::new(
                "right_front_leg",
                PartPose::offset_and_rotation(-12.0, 17.0, -6.0, 1.3, 0.0, 0.0),
            )
            .cube(cube([112.0, 104.0], [-4.0, -4.0, -4.0], [8.0, 24.0, 8.0]))
            .child(
                NamedPartSpec::new(
                    "right_front_leg_tip",
                    PartPose::offset_and_rotation(0.0, 20.0, -1.0, -0.5, 0.0, 0.0),
                )
                .cube(cube([226.0, 138.0], [-3.0, -1.0, -3.0], [6.0, 24.0, 6.0]))
                .child(
                    NamedPartSpec::new(
                        "right_front_foot",
                        PartPose::offset_and_rotation(0.0, 23.0, 0.0, 0.75, 0.0, 0.0),
                    )
                    .cube(cube(
                        [144.0, 104.0],
                        [-4.0, 0.0, -12.0],
                        [8.0, 4.0, 16.0],
                    )),
                ),
            ),
        )
        .child(
            NamedPartSpec::new(
                "left_hind_leg",
                PartPose::offset_and_rotation(16.0, 13.0, 34.0, 1.0, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -4.0, -8.0], [16.0, 32.0, 16.0]))
            .child(
                NamedPartSpec::new(
                    "left_hind_leg_tip",
                    PartPose::offset_and_rotation(0.0, 32.0, -4.0, 0.5, 0.0, 0.0),
                )
                .cube(cube([196.0, 0.0], [-6.0, -2.0, 0.0], [12.0, 32.0, 12.0]))
                .child(
                    NamedPartSpec::new(
                        "left_hind_foot",
                        PartPose::offset_and_rotation(0.0, 31.0, 4.0, 0.75, 0.0, 0.0),
                    )
                    .cube(cube(
                        [112.0, 0.0],
                        [-9.0, 0.0, -20.0],
                        [18.0, 6.0, 24.0],
                    )),
                ),
            ),
        )
        .child(
            NamedPartSpec::new(
                "right_hind_leg",
                PartPose::offset_and_rotation(-16.0, 13.0, 34.0, 1.0, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -4.0, -8.0], [16.0, 32.0, 16.0]))
            .child(
                NamedPartSpec::new(
                    "right_hind_leg_tip",
                    PartPose::offset_and_rotation(0.0, 32.0, -4.0, 0.5, 0.0, 0.0),
                )
                .cube(cube([196.0, 0.0], [-6.0, -2.0, 0.0], [12.0, 32.0, 12.0]))
                .child(
                    NamedPartSpec::new(
                        "right_hind_foot",
                        PartPose::offset_and_rotation(0.0, 31.0, 4.0, 0.75, 0.0, 0.0),
                    )
                    .cube(cube(
                        [112.0, 0.0],
                        [-9.0, 0.0, -20.0],
                        [18.0, 6.0, 24.0],
                    )),
                ),
            ),
        );

    root = root.child(body);
    JavaModelPartTree::from_root(root)
}

pub fn build_ender_dragon_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_ender_dragon_model_tree().mesh(ModelTextureSize {
        width: 256.0,
        height: 256.0,
    })
}

pub fn apply_ender_dragon_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let flap_time = state.age_in_ticks * (std::f32::consts::PI * 2.0);
    if let Some(j) = tree.part_mut("jaw") {
        j.pose.rotation.x = (flap_time.sin() + 1.0) * 0.2;
    }
}

pub fn build_ender_dragon_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_ender_dragon_model_tree();
    tree.reset_pose();
    apply_ender_dragon_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 256.0,
        height: 256.0,
    })
}
