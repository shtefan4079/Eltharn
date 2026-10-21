#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::LazyLock;

use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;
use crate::render::model::animation::{
    AnimationChannel, AnimationDefinition, AnimationInterpolation, AnimationTarget, Keyframe,
    degree_vec, pos_vec, scale_vec,
};

pub fn build_copper_golem_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::offset(0.0, 24.0, 0.0))
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, -5.0, 0.0))
                    .cube(cube([0.0, 15.0], [-4.0, -6.0, -3.0], [8.0, 6.0, 6.0]))
                    .child(
                        NamedPartSpec::new("head", PartPose::offset(0.0, -6.0, 0.0))
                            .cube(cube_grown(
                                [0.0, 0.0],
                                [-4.0, -5.0, -5.0],
                                [8.0, 5.0, 10.0],
                                0.015,
                            ))
                            .cube(cube([56.0, 0.0], [-1.0, -2.0, -6.0], [2.0, 3.0, 2.0]))
                            .cube(cube_grown(
                                [37.0, 8.0],
                                [-1.0, -9.0, -1.0],
                                [2.0, 4.0, 2.0],
                                -0.015,
                            ))
                            .cube(cube_grown(
                                [37.0, 0.0],
                                [-2.0, -13.0, -2.0],
                                [4.0, 4.0, 4.0],
                                -0.015,
                            )),
                    )
                    .child(
                        NamedPartSpec::new("right_arm", PartPose::offset(-4.0, -6.0, 0.0))
                            .cube(cube([36.0, 16.0], [-3.0, -1.0, -2.0], [3.0, 10.0, 4.0])),
                    )
                    .child(
                        NamedPartSpec::new("left_arm", PartPose::offset(4.0, -6.0, 0.0))
                            .cube(cube([50.0, 16.0], [0.0, -1.0, -2.0], [3.0, 10.0, 4.0])),
                    ),
            )
            .child(
                NamedPartSpec::new("right_leg", PartPose::offset(0.0, -5.0, 0.0)).cube(cube(
                    [0.0, 27.0],
                    [-4.0, 0.0, -2.0],
                    [4.0, 5.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_leg", PartPose::offset(0.0, -5.0, 0.0)).cube(cube(
                    [16.0, 27.0],
                    [0.0, 0.0, -2.0],
                    [4.0, 5.0, 4.0],
                )),
            );

    JavaModelPartTree::from_root(root)
}

pub static COPPER_GOLEM_WALK: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(10.0, 15.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.2083,
                    degree_vec(10.0, -1.87, -10.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.4167,
                    degree_vec(10.0, -15.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.625,
                    degree_vec(10.0, -0.82, 10.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.8333,
                    degree_vec(10.0, 15.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("head")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.2083,
                    degree_vec(-10.0, 1.87, 10.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.4167,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.625,
                    degree_vec(-10.0, 0.82, -10.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.8333,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_arm")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(70.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.4167,
                    degree_vec(-80.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.8333,
                    degree_vec(70.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_arm")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-80.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.4167,
                    degree_vec(70.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.8333,
                    degree_vec(-80.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-60.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.4167,
                    degree_vec(60.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.8333,
                    degree_vec(-60.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(60.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.4167,
                    degree_vec(-60.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.8333,
                    degree_vec(60.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    AnimationDefinition::new(0.8333, true, map)
});

pub static COPPER_GOLEM_IDLE: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(0.0, -35.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    degree_vec(0.0, -35.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.625,
                    degree_vec(0.0, 35.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2083,
                    degree_vec(0.0, 35.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.7083,
                    degree_vec(0.0, 35.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    3.0,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    3.5,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("head")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.625,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2083,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    degree_vec(0.0, 300.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.6667,
                    degree_vec(0.0, 300.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.75,
                    degree_vec(-25.0, 300.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.7083,
                    degree_vec(-25.0, 300.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    3.0,
                    degree_vec(0.0, 360.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    3.5,
                    degree_vec(0.0, 360.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    AnimationDefinition::new(3.5, false, map)
});

pub static COPPER_GOLEM_WALK_ITEM: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(10.0, 7.5, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.2083,
                    degree_vec(10.0, -1.87, -5.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.4167,
                    degree_vec(10.0, -7.5, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.625,
                    degree_vec(10.0, -0.82, 5.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.8333,
                    degree_vec(10.0, 7.5, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("head")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.2083,
                    degree_vec(-10.0, 1.87, 10.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.4167,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.625,
                    degree_vec(-10.0, 0.82, -10.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.8333,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_arm")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![Keyframe::new(
                0.0,
                degree_vec(-59.78638, -6.49053, -3.76613),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("left_arm")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![Keyframe::new(
                0.0,
                degree_vec(-59.78638, 6.49053, 3.76613),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("left_arm")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![Keyframe::new(
                0.0,
                pos_vec(-0.21129, -0.0212, -0.07004),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("right_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-30.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.4167,
                    degree_vec(30.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.8333,
                    degree_vec(-30.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(30.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.4167,
                    degree_vec(-30.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.8333,
                    degree_vec(30.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    AnimationDefinition::new(0.8333, true, map)
});

pub static COPPER_GOLEM_CHEST_INTERACTION_ITEM_DROP: LazyLock<AnimationDefinition> =
    LazyLock::new(|| {
        let mut map = HashMap::new();
        map.entry("body")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        degree_vec(18.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(24.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(15.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(12.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(12.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.75,
                        degree_vec(12.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.8333,
                        degree_vec(14.72765, -31.63886, -7.85085),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.9167,
                        degree_vec(14.72765, -31.63886, -7.85085),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.0417,
                        degree_vec(14.72765, -31.63886, -7.85085),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.125,
                        degree_vec(12.40525, -4.4E-4, 0.00829),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        degree_vec(12.40525, -4.4E-4, 0.00829),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        degree_vec(13.92716, 26.80536, 6.38918),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.625,
                        degree_vec(13.93, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        degree_vec(21.43, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.7917,
                        degree_vec(21.43, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        degree_vec(13.93, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        degree_vec(13.93, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.125,
                        degree_vec(12.40725, 0.0, 0.00783),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.375,
                        degree_vec(-7.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4167,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.625,
                        degree_vec(15.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.6667,
                        degree_vec(17.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7083,
                        degree_vec(22.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.875,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("body")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.6, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        pos_vec(0.0, 0.5, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.625,
                        pos_vec(0.0, 0.4, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        pos_vec(-0.01805, 0.88303, -0.09783),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.7917,
                        pos_vec(-0.01805, 0.88303, -0.09783),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        pos_vec(0.0, 0.6, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        pos_vec(0.0, 0.6, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.375,
                        pos_vec(0.0, 0.0, -1.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        pos_vec(0.0, 0.0, -1.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.875,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        map.entry("head")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        degree_vec(-20.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(-20.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.25,
                        degree_vec(-2.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(-5.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.6667,
                        degree_vec(0.0, -20.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.75,
                        degree_vec(0.0, -20.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.8333,
                        degree_vec(0.0, 10.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.0417,
                        degree_vec(0.0, 10.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.1667,
                        degree_vec(0.0, 10.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        degree_vec(0.0, 27.5, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4167,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4583,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5,
                        degree_vec(0.0, -2.5, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.625,
                        degree_vec(0.0, -2.5, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        degree_vec(9.73588, -1.93433, -3.73384),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.7083,
                        degree_vec(9.73588, -1.93433, -3.73384),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.7917,
                        degree_vec(10.15255, -1.93433, -3.73384),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        degree_vec(17.86088, -1.93433, -3.73384),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.875,
                        degree_vec(17.23588, -1.93433, -3.73384),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.9167,
                        degree_vec(17.23588, -1.93433, -3.73384),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        degree_vec(17.23588, -1.93433, -3.73384),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0833,
                        degree_vec(-0.26, -1.93, -3.73),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        degree_vec(-15.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.6667,
                        degree_vec(-15.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7083,
                        degree_vec(-5.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.875,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("head")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        pos_vec(0.0, -0.15451, 0.47553),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(0.25, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.4167,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.0417,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.1667,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4167,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4583,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(1.5, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        1.625,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        pos_vec(-0.22438, 0.82319, -1.27252),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.875,
                        pos_vec(-0.22438, 0.82319, -1.27252),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.9167,
                        pos_vec(-0.22438, 0.82319, -1.27252),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        pos_vec(-0.22438, 0.82319, -1.27252),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0833,
                        pos_vec(-0.39, 0.52, -2.21),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.6667,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.875,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        map.entry("right_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        degree_vec(-7.38733, 1.29876, 9.91615),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(-7.38733, 1.29876, 9.91615),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(10.0, 0.0, 32.5),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(-34.55418, 11.73507, 36.8361),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4583,
                        degree_vec(-117.82767, 2.94538, 0.22703),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5417,
                        degree_vec(-97.7902, 0.73403, 1.39387),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(-92.79, 0.73, 1.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.75,
                        degree_vec(-92.79, 0.73, 1.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.8333,
                        degree_vec(-95.83405, 33.18639, -0.40081),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5,
                        degree_vec(-95.83, 33.19, -0.4),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5833,
                        degree_vec(-44.60123, 10.14454, 8.66307),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        degree_vec(-4.31506, 6.54961, 13.21388),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0833,
                        degree_vec(-4.31506, 6.54961, 13.21388),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.125,
                        degree_vec(-4.31506, 6.54961, 13.21388),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2083,
                        degree_vec(-113.7629, 21.38835, 15.48184),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        degree_vec(-113.76, 21.39, 15.48),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        degree_vec(-39.99304, 7.3511, 14.05666),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.625,
                        degree_vec(68.07913, -3.61348, 1.39182),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.6667,
                        degree_vec(193.16708, -1.90441, -0.43495),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7083,
                        degree_vec(250.66708, -1.90441, -0.43495),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7917,
                        degree_vec(264.19006, -4.41519, -0.66792),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.8333,
                        degree_vec(270.53693, 16.03493, 0.47968),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.875,
                        degree_vec(319.31668, 17.97846, 1.34328),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9167,
                        degree_vec(0.0, -5.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("right_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(0.75, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.8333,
                        pos_vec(0.25358, -0.20153, 2.21248),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5,
                        pos_vec(0.25, -0.2, 2.21),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5417,
                        pos_vec(-0.79739, -0.10573, 1.70592),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5833,
                        pos_vec(-0.26323, -1.46323, 0.66566),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        pos_vec(-0.51052, -0.38088, 0.79745),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0833,
                        pos_vec(-0.51052, -0.38088, 0.79745),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.125,
                        pos_vec(-0.51052, -0.38088, 0.79745),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        pos_vec(-0.51, -0.38, 0.8),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.875,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        map.entry("left_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(25.0, 0.0, -37.5),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.25,
                        degree_vec(-21.59341, -12.60837, -45.69252),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(-120.7755, -5.21988, -2.02064),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(-98.27419, -1.79323, -1.15048),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(-93.27, -1.79, -1.15),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5417,
                        degree_vec(-93.27419, -1.79323, -1.15048),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(-93.55693, -22.3224, 3.64383),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.7083,
                        degree_vec(-93.55693, -22.3224, 3.64383),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.1667,
                        degree_vec(-93.55693, -22.3224, 3.64383),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        degree_vec(-95.75, -2.42, 5.97),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.25,
                        degree_vec(-98.4029, -17.39503, 6.85104),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        degree_vec(-101.24523, -29.87096, 7.69993),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5833,
                        degree_vec(-101.25, -29.87, 7.7),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.25,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        degree_vec(-88.58526, -17.10045, 11.7676),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        degree_vec(-88.59, -17.1, 11.77),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        degree_vec(-46.59531, -16.13694, -3.85578),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.875,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("left_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5833,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        pos_vec(-0.00677, -0.76064, 3.19059),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        pos_vec(0.0512, -0.76176, 3.12882),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.875,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        map.entry("right_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(7.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.125,
                        degree_vec(7.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.375,
                        degree_vec(2.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        degree_vec(5.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        degree_vec(5.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5833,
                        degree_vec(5.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.625,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.875,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("right_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.375,
                        pos_vec(0.0, 0.0909, -0.10834),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        pos_vec(0.0, 0.09, -0.11),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        pos_vec(0.0, 0.09, -0.11),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.875,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        map.entry("left_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.125,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.375,
                        degree_vec(2.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        degree_vec(5.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        degree_vec(5.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5833,
                        degree_vec(5.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.625,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("left_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.375,
                        pos_vec(0.0, 0.0909, -0.10834),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        pos_vec(0.0, 0.09, -0.11),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        pos_vec(0.0, 0.09, -0.11),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        AnimationDefinition::new(3.0, true, map)
    });

pub static COPPER_GOLEM_CHEST_INTERACTION_ITEM_NODROP: LazyLock<AnimationDefinition> =
    LazyLock::new(|| {
        let mut map = HashMap::new();
        map.entry("body")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        degree_vec(18.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(24.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(15.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(12.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(12.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.75,
                        degree_vec(12.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.8333,
                        degree_vec(14.72765, -31.63886, -7.85085),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.9167,
                        degree_vec(14.72765, -31.63886, -7.85085),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.0417,
                        degree_vec(14.72765, -31.63886, -7.85085),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.125,
                        degree_vec(12.40525, -4.4E-4, 0.00829),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        degree_vec(12.40525, -4.4E-4, 0.00829),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        degree_vec(13.92716, 26.80536, 6.38918),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.625,
                        degree_vec(13.93, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        degree_vec(21.43, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.7917,
                        degree_vec(21.43, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        degree_vec(13.93, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        degree_vec(13.93, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        degree_vec(12.40725, 0.0, 0.00783),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.25,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        degree_vec(-2.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.875,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("body")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.6, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        pos_vec(0.0, 0.5, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.625,
                        pos_vec(0.0, 0.4, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        pos_vec(-0.01805, 0.88303, -0.09783),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.7917,
                        pos_vec(-0.01805, 0.88303, -0.09783),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        pos_vec(0.0, 0.6, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        pos_vec(0.0, 0.6, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(2.25, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        2.3333,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(2.5, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        2.875,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        map.entry("head")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        degree_vec(-20.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(-20.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.25,
                        degree_vec(-2.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(-5.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.6667,
                        degree_vec(0.0, -20.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.75,
                        degree_vec(0.0, -20.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.8333,
                        degree_vec(0.0, 10.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.0417,
                        degree_vec(0.0, 10.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.1667,
                        degree_vec(0.0, 10.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        degree_vec(0.0, 27.5, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4167,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4583,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5,
                        degree_vec(0.0, -2.5, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.625,
                        degree_vec(0.0, -2.5, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        degree_vec(9.73588, -1.93433, -3.73384),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.7083,
                        degree_vec(9.73588, -1.93433, -3.73384),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.7917,
                        degree_vec(10.15255, -1.93433, -3.73384),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        degree_vec(17.86088, -1.93433, -3.73384),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.875,
                        degree_vec(17.23588, -1.93433, -3.73384),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.9167,
                        degree_vec(17.23588, -1.93433, -3.73384),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        degree_vec(17.23588, -1.93433, -3.73384),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0833,
                        degree_vec(-0.26, -1.93, -3.73),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.25,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        degree_vec(1.25, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        degree_vec(2.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.6667,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        degree_vec(0.0, -360.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("head")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        pos_vec(0.0, -0.15451, 0.47553),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(0.25, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.4167,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.0417,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.1667,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4167,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4583,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(1.5, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        1.625,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        pos_vec(-0.22438, 0.82319, -1.27252),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.875,
                        pos_vec(-0.22438, 0.82319, -1.27252),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.9167,
                        pos_vec(-0.22438, 0.82319, -1.27252),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        pos_vec(-0.22438, 0.82319, -1.27252),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0833,
                        pos_vec(-0.39, 0.52, -2.21),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(2.25, pos_vec(0.0, 0.0, 0.5), AnimationInterpolation::Linear),
                    Keyframe::new(
                        2.2917,
                        pos_vec(0.0, -0.01091, -0.02988),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        pos_vec(0.0, -0.01, -0.03),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.6667,
                        pos_vec(0.0, -0.01, -0.03),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        pos_vec(0.0, -0.01, -0.03),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        map.entry("right_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        degree_vec(-7.38733, 1.29876, 9.91615),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(-7.38733, 1.29876, 9.91615),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(10.0, 0.0, 32.5),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(-34.55418, 11.73507, 36.8361),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4583,
                        degree_vec(-117.82767, 2.94538, 0.22703),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5417,
                        degree_vec(-97.7902, 0.73403, 1.39387),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(-92.79, 0.73, 1.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.75,
                        degree_vec(-92.79, 0.73, 1.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.8333,
                        degree_vec(-95.83405, 33.18639, -0.40081),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5,
                        degree_vec(-95.83, 33.19, -0.4),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5833,
                        degree_vec(-44.60123, 10.14454, 8.66307),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        degree_vec(-4.31506, 6.54961, 13.21388),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0833,
                        degree_vec(-4.31506, 6.54961, 13.21388),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        degree_vec(-4.31506, 6.54961, 13.21388),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.25,
                        degree_vec(-6.53898, 13.96898, 14.34786),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        degree_vec(3.50393, -4.70737, 8.3608),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        degree_vec(3.50393, -4.70737, 8.3608),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5,
                        degree_vec(3.90089, -4.3843, 3.35549),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        degree_vec(3.9, -4.38, 3.36),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9167,
                        degree_vec(3.9, -4.38, 3.36),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        degree_vec(3.90089, -4.3843, 3.35549),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("right_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(0.75, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.8333,
                        pos_vec(0.25358, -0.20153, 2.21248),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5,
                        pos_vec(0.25, -0.2, 2.21),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5417,
                        pos_vec(-0.79739, -0.10573, 1.70592),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5833,
                        pos_vec(-0.26323, -1.46323, 0.66566),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        pos_vec(-0.51052, -0.38088, 0.79745),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0833,
                        pos_vec(-0.51052, -0.38088, 0.79745),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        pos_vec(-0.51052, -0.38088, 0.79745),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.25,
                        pos_vec(-0.46, -0.34, 0.72),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        pos_vec(-0.46, 0.1159, -0.30086),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        pos_vec(-0.46, 0.1159, -0.30086),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5,
                        pos_vec(-0.46, 0.1159, -0.30086),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        pos_vec(-0.46, -0.88, -0.3),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9167,
                        pos_vec(-0.46, -0.88, -0.3),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        pos_vec(-0.46, 0.1159, -0.30086),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        map.entry("left_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(25.0, 0.0, -37.5),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.25,
                        degree_vec(-21.59341, -12.60837, -45.69252),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(-120.7755, -5.21988, -2.02064),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(-98.27419, -1.79323, -1.15048),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(-93.27, -1.79, -1.15),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5417,
                        degree_vec(-93.27419, -1.79323, -1.15048),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(-93.55693, -22.3224, 3.64383),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.7083,
                        degree_vec(-93.55693, -22.3224, 3.64383),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.1667,
                        degree_vec(-93.55693, -22.3224, 3.64383),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        degree_vec(-95.75, -2.42, 5.97),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.25,
                        degree_vec(-98.4029, -17.39503, 6.85104),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        degree_vec(-101.24523, -29.87096, 7.69993),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5833,
                        degree_vec(-101.25, -29.87, 7.7),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.25,
                        degree_vec(0.0, 0.0, -20.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        degree_vec(2.47864, -0.32621, -12.50706),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        degree_vec(2.47864, -0.32621, -12.50706),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5,
                        degree_vec(2.41492, -0.64686, -5.01363),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        degree_vec(2.41, -0.65, -5.01),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9167,
                        degree_vec(2.41, -0.65, -5.01),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        degree_vec(2.41492, -0.64686, -5.01363),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("left_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5833,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        pos_vec(-0.00677, -0.76064, 3.19059),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        pos_vec(-0.00677, -0.76064, 3.19059),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        pos_vec(-0.00677, -0.76064, 3.19059),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.25,
                        pos_vec(0.03, -0.76, 0.45),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        pos_vec(0.03, -0.28229, -0.07133),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        pos_vec(0.03, -0.28229, -0.07133),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5,
                        pos_vec(0.03, -0.28229, -0.07133),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        pos_vec(0.03, -1.28, -0.07),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9167,
                        pos_vec(0.03, -1.28, -0.07),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        pos_vec(0.03, -0.28229, -0.07133),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        map.entry("right_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(7.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        degree_vec(7.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.25,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.875,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("right_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(2.25, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        2.3333,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(2.5, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        2.875,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        map.entry("left_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.25,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("left_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(2.25, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        map.entry("left_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Scale,
                vec![
                    Keyframe::new(
                        0.0,
                        scale_vec(1.0, 1.0, 1.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        scale_vec(1.0, 1.0, 1.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        AnimationDefinition::new(3.0, true, map)
    });

pub static COPPER_GOLEM_CHEST_INTERACTION_NOITEM_GET: LazyLock<AnimationDefinition> =
    LazyLock::new(|| {
        let mut map = HashMap::new();
        map.entry("body")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        degree_vec(18.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(24.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(15.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(12.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(12.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.75,
                        degree_vec(12.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.8333,
                        degree_vec(14.72765, -31.63886, -7.85085),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.9167,
                        degree_vec(14.72765, -31.63886, -7.85085),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.0417,
                        degree_vec(14.72765, -31.63886, -7.85085),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.125,
                        degree_vec(12.40525, -4.4E-4, 0.00829),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        degree_vec(12.40525, -4.4E-4, 0.00829),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        degree_vec(13.92716, 26.80536, 6.38918),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.625,
                        degree_vec(13.93, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        degree_vec(21.43, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.7917,
                        degree_vec(21.43, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        degree_vec(13.93, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        degree_vec(13.93, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.125,
                        degree_vec(12.40725, 0.0, 0.00783),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.25,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.375,
                        degree_vec(15.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4167,
                        degree_vec(17.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        degree_vec(22.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.625,
                        degree_vec(24.14867, -20.70481, -9.00717),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7083,
                        degree_vec(24.14867, -20.70481, -9.00717),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.75,
                        degree_vec(22.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7917,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("body")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.6, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        pos_vec(0.0, 0.5, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.625,
                        pos_vec(0.0, 0.4, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        pos_vec(-0.01805, 0.88303, -0.09783),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.7917,
                        pos_vec(-0.01805, 0.88303, -0.09783),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        pos_vec(0.0, 0.6, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        pos_vec(0.0, 0.6, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(2.25, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        2.2917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.625,
                        pos_vec(0.0, 0.46194, -0.19134),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7083,
                        pos_vec(0.0, 0.46194, -0.19134),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("head")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        degree_vec(-20.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(-20.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.25,
                        degree_vec(-2.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(-5.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.6667,
                        degree_vec(0.0, -20.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.75,
                        degree_vec(0.0, -20.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.8333,
                        degree_vec(0.0, 10.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.0417,
                        degree_vec(0.0, 10.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.1667,
                        degree_vec(0.0, 10.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        degree_vec(0.0, 27.5, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4167,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4583,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5833,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.625,
                        degree_vec(0.0, -2.5, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        degree_vec(10.16381, -16.71134, -6.35306),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        degree_vec(10.16381, -16.71134, -6.35306),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.9167,
                        degree_vec(10.16381, -16.71134, -6.35306),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.9583,
                        degree_vec(10.16381, -16.71134, -6.35306),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0,
                        degree_vec(5.16381, -16.71134, -6.35306),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        degree_vec(0.16381, -16.71134, -6.35306),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0833,
                        degree_vec(0.15732, -4.21139, -6.31751),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.125,
                        degree_vec(0.07901, 5.3943, -3.15187),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        degree_vec(0.0, 7.5, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        degree_vec(4.53867, 7.47675, 0.59181),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        degree_vec(-2.53852, 9.99038, -0.44067),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4167,
                        degree_vec(-12.68664, 9.76061, -2.18558),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.625,
                        degree_vec(-15.19938, 22.36971, -3.52259),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.6667,
                        degree_vec(-3.02173, 22.37156, -2.41802),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7083,
                        degree_vec(-0.52173, 22.37156, -2.41802),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.75,
                        degree_vec(-12.40598, -0.4674, -1.79838),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.8333,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("head")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        pos_vec(0.0, -0.15451, 0.47553),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(0.25, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.4167,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.0417,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.1667,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4167,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4583,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(1.5, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        1.5833,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.625,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        pos_vec(-0.22438, 0.82319, -1.27252),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        pos_vec(-0.22438, 0.82319, -1.27252),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.9167,
                        pos_vec(-0.22438, 0.82319, -1.27252),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.9583,
                        pos_vec(-0.52521, 0.96725, -0.32978),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0,
                        pos_vec(-0.52521, 0.96725, -0.32978),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        pos_vec(-0.5345, 1.16541, -0.37206),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0833,
                        pos_vec(-0.5345, 1.16541, -0.37206),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.625,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.8333,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("right_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        degree_vec(-7.38733, 1.29876, 9.91615),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(-7.38733, 1.29876, 9.91615),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(10.0, 0.0, 32.5),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(-34.55418, 11.73507, 36.8361),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4583,
                        degree_vec(-82.47403, 17.82361, 2.17224),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5,
                        degree_vec(-85.08388, 14.26971, 1.99595),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5417,
                        degree_vec(-85.16266, 13.19102, 2.43976),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(-92.79, 0.73, 1.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.75,
                        degree_vec(-92.79, 0.73, 1.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.8333,
                        degree_vec(-95.83405, 33.18639, -0.40081),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.25,
                        degree_vec(-95.83, 33.19, -0.4),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        degree_vec(-98.33, 33.19, -0.4),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5417,
                        degree_vec(-56.46674, 3.3853, 14.45894),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        degree_vec(-56.46674, 3.3853, 14.45894),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        degree_vec(-56.46674, 3.3853, 14.45894),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0,
                        degree_vec(-56.46674, 3.3853, 14.45894),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        degree_vec(-56.46674, 3.3853, 14.45894),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        degree_vec(-84.12204, 8.95753, 14.11779),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2083,
                        degree_vec(-84.12204, 8.95753, 14.11779),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.25,
                        degree_vec(-93.6065, 13.90544, 15.98524),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        degree_vec(-124.48661, 66.29146, -7.28605),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.375,
                        degree_vec(-129.4866, 66.29146, -7.28605),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4167,
                        degree_vec(-108.91607, 1.79762, 20.93924),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5,
                        degree_vec(-102.18303, 4.35881, 17.40962),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        degree_vec(-98.33642, -0.70114, 4.09322),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.625,
                        degree_vec(-98.39385, 6.71929, 3.00137),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.6667,
                        degree_vec(-98.33981, 1.77244, 3.7307),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7083,
                        degree_vec(-100.70987, 3.48829, 7.1138),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.75,
                        degree_vec(-97.95, 6.92, 13.88),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7917,
                        degree_vec(-87.95, 6.92, 13.88),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.8333,
                        degree_vec(-97.95, 6.92, 13.88),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.875,
                        degree_vec(-102.95, 6.92, 13.88),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9167,
                        degree_vec(-76.475, 3.46, 6.94),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        degree_vec(-26.475, 3.46, 6.94),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("right_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(0.75, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.8333,
                        pos_vec(0.25358, -0.20153, 2.21248),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.25,
                        pos_vec(0.25, -0.2, 2.21),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        pos_vec(0.25, -0.2, 2.21),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5417,
                        pos_vec(-0.26323, -1.46323, 0.66566),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        pos_vec(-0.26323, -1.46323, 0.66566),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        pos_vec(-0.26323, -1.46323, 0.66566),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0,
                        pos_vec(-0.26323, -1.46323, 0.66566),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        pos_vec(-0.26323, -1.46323, 0.66566),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.25,
                        pos_vec(-0.51, -0.38, 0.8),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        pos_vec(-0.51, -0.38, 0.8),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.375,
                        pos_vec(-0.51, -0.38, 0.8),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4167,
                        pos_vec(-2.14094, 0.69619, 1.23422),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        pos_vec(-0.97932, 0.38244, 0.12884),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        pos_vec(-1.55232, 1.79904, 0.37956),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.625,
                        pos_vec(-1.53125, 1.64598, 1.41168),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.6667,
                        pos_vec(-1.57256, 1.05375, 1.32469),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.75,
                        pos_vec(-1.33, 0.16, 1.02),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7917,
                        pos_vec(-1.33, 0.16, 1.02),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.8333,
                        pos_vec(-1.33, 0.16, 1.02),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.875,
                        pos_vec(-1.33, 0.16, 1.02),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9167,
                        pos_vec(-0.5748, 0.38848, 1.45646),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        pos_vec(-0.67, 0.08, 0.51),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        map.entry("left_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(25.0, 0.0, -37.5),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.25,
                        degree_vec(-21.59341, -12.60837, -45.69252),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(-120.7755, -5.21988, -2.02064),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(-98.27419, -1.79323, -1.15048),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(-93.27, -1.79, -1.15),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5417,
                        degree_vec(-93.27419, -1.79323, -1.15048),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(-93.55693, -22.3224, 3.64383),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.7083,
                        degree_vec(-93.55693, -22.3224, 3.64383),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.1667,
                        degree_vec(-93.55693, -22.3224, 3.64383),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        degree_vec(-95.75, -2.42, 5.97),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.25,
                        degree_vec(-98.4029, -17.39503, 6.85104),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        degree_vec(-101.24523, -29.87096, 7.69993),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5833,
                        degree_vec(-101.25, -29.87, 7.7),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.9583,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2083,
                        degree_vec(-88.58526, -17.10045, 11.7676),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        degree_vec(-88.59, -17.1, 11.77),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4167,
                        degree_vec(-46.59531, -16.13694, -3.85578),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5,
                        degree_vec(-24.5317, -19.0214, -13.70805),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.8333,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("left_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5833,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        pos_vec(-0.00677, -0.76064, 3.19059),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        pos_vec(0.0512, -0.76176, 3.12882),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.8333,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("right_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(7.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        degree_vec(7.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7083,
                        degree_vec(7.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7917,
                        degree_vec(5.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.8333,
                        degree_vec(5.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.875,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("right_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.7917,
                        pos_vec(0.0, 0.09, -0.11),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.8333,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("left_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.25,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4167,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.625,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("left_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(2.25, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        2.3333,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4167,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(2.5, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        2.625,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        AnimationDefinition::new(3.0, true, map)
    });

pub static COPPER_GOLEM_CHEST_INTERACTION_NOITEM_NOGET: LazyLock<AnimationDefinition> =
    LazyLock::new(|| {
        let mut map = HashMap::new();
        map.entry("body")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        degree_vec(18.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(24.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(15.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(12.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(12.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.75,
                        degree_vec(12.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.8333,
                        degree_vec(14.72765, -31.63886, -7.85085),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.9167,
                        degree_vec(14.72765, -31.63886, -7.85085),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.0417,
                        degree_vec(14.72765, -31.63886, -7.85085),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.125,
                        degree_vec(12.40525, -4.4E-4, 0.00829),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        degree_vec(12.40525, -4.4E-4, 0.00829),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        degree_vec(13.92716, 26.80536, 6.38918),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.625,
                        degree_vec(13.93, 26.81, 6.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.7083,
                        degree_vec(12.40725, 0.00444, 0.00783),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        degree_vec(12.40725, 0.00444, 0.00783),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.125,
                        degree_vec(12.40725, 0.0, 0.00783),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.25,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.6667,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("body")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.6, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        pos_vec(0.0, 0.5, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.625,
                        pos_vec(0.0, 0.4, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.7083,
                        pos_vec(0.0, 0.34, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        pos_vec(0.0, 0.34, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(2.25, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        2.4583,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(2.5, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        2.6667,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("head")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        degree_vec(-20.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(-20.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.25,
                        degree_vec(-2.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(-5.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.6667,
                        degree_vec(0.0, -20.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.75,
                        degree_vec(0.0, -20.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.8333,
                        degree_vec(0.0, 10.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.0417,
                        degree_vec(0.0, 10.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.1667,
                        degree_vec(0.0, 10.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        degree_vec(0.0, 27.5, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4167,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4583,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5833,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.625,
                        degree_vec(0.0, -2.5, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.7083,
                        degree_vec(0.57, -1.25, 0.07),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.75,
                        degree_vec(0.89798, -18.12465, -0.16276),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.7917,
                        degree_vec(1.21328, -21.15422, -0.2148),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.875,
                        degree_vec(1.21328, -21.15422, -0.2148),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0,
                        degree_vec(1.21328, -21.15422, -0.2148),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        degree_vec(2.56546, 0.76525, 0.57246),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        degree_vec(4.53867, 7.47675, 0.59181),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5,
                        degree_vec(4.53867, 7.47675, 0.59181),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        degree_vec(0.0, -360.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("head")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        pos_vec(0.0, -0.15451, 0.47553),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(0.25, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.4167,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.0417,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.1667,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4167,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.4583,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(1.5, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        1.5833,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.625,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.7083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        pos_vec(0.0, -0.01, -0.03),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        map.entry("right_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.1667,
                        degree_vec(-7.38733, 1.29876, 9.91615),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(-7.38733, 1.29876, 9.91615),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(10.0, 0.0, 32.5),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(-34.55418, 11.73507, 36.8361),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4583,
                        degree_vec(-82.47403, 17.82361, 2.17224),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5,
                        degree_vec(-85.08388, 14.26971, 1.99595),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5417,
                        degree_vec(-85.16266, 13.19102, 2.43976),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(-92.79, 0.73, 1.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.75,
                        degree_vec(-92.79, 0.73, 1.39),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.8333,
                        degree_vec(-95.83405, 33.18639, -0.40081),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.25,
                        degree_vec(-95.83, 33.19, -0.4),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        degree_vec(-98.33, 33.19, -0.4),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5417,
                        degree_vec(-56.46674, 3.3853, 14.45894),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        degree_vec(-56.46674, 3.3853, 14.45894),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        degree_vec(-56.46674, 3.3853, 14.45894),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0,
                        degree_vec(-56.46674, 3.3853, 14.45894),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        degree_vec(-56.46674, 3.3853, 14.45894),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        degree_vec(3.9, -4.38, 3.36),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9167,
                        degree_vec(3.9, -4.38, 3.36),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        degree_vec(3.90089, -4.3843, 3.35549),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("right_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(0.75, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.8333,
                        pos_vec(0.25358, -0.20153, 2.21248),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.25,
                        pos_vec(0.25, -0.2, 2.21),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        pos_vec(0.25, -0.2, 2.21),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5417,
                        pos_vec(-0.26323, -1.46323, 0.66566),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        pos_vec(-0.26323, -1.46323, 0.66566),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        pos_vec(-0.26323, -1.46323, 0.66566),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0,
                        pos_vec(-0.26323, -1.46323, 0.66566),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        pos_vec(-0.26323, -1.46323, 0.66566),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(2.5, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        2.5417,
                        pos_vec(-0.46, -0.88, -0.3),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9167,
                        pos_vec(-0.46, -0.88, -0.3),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        pos_vec(-0.46, 0.1159, -0.30086),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        map.entry("left_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(-2.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(25.0, 0.0, -37.5),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.25,
                        degree_vec(-21.59341, -12.60837, -45.69252),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2917,
                        degree_vec(-120.7755, -5.21988, -2.02064),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.375,
                        degree_vec(-98.27419, -1.79323, -1.15048),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        degree_vec(-93.27, -1.79, -1.15),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5417,
                        degree_vec(-93.27419, -1.79323, -1.15048),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.5833,
                        degree_vec(-93.55693, -22.3224, 3.64383),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.7083,
                        degree_vec(-93.55693, -22.3224, 3.64383),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.1667,
                        degree_vec(-93.55693, -22.3224, 3.64383),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        degree_vec(-95.75, -2.42, 5.97),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.25,
                        degree_vec(-98.4029, -17.39503, 6.85104),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2917,
                        degree_vec(-101.24523, -29.87096, 7.69993),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5833,
                        degree_vec(-101.25, -29.87, 7.7),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.8333,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0833,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.1667,
                        degree_vec(-88.17772, -42.09094, 10.96195),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2083,
                        degree_vec(-88.58526, -17.10045, 11.7676),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        degree_vec(-88.59, -17.1, 11.77),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4167,
                        degree_vec(-46.59531, -16.13694, -3.85578),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        degree_vec(-24.5317, -19.0214, -13.70805),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5,
                        degree_vec(-24.5317, -19.0214, -13.70805),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        degree_vec(2.41, -0.65, -5.01),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9167,
                        degree_vec(2.41, -0.65, -5.01),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        degree_vec(2.41492, -0.64686, -5.01363),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        3.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("left_arm")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.4167,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.5833,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        1.6667,
                        pos_vec(-0.00677, -0.76064, 3.19059),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        pos_vec(0.0512, -0.76176, 3.12882),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.4583,
                        pos_vec(0.03, -0.51, 2.09),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5,
                        pos_vec(0.03, -0.51, 2.09),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.5417,
                        pos_vec(0.03, -1.28, -0.07),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9167,
                        pos_vec(0.03, -1.28, -0.07),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        pos_vec(0.03, -0.28229, -0.07133),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(3.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                ],
            ));
        map.entry("right_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(7.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        degree_vec(7.5, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("right_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("left_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Rotation,
                vec![
                    Keyframe::new(
                        0.0,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.125,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        degree_vec(-10.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        degree_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        map.entry("left_leg")
            .or_insert_with(Vec::new)
            .push(AnimationChannel::new(
                AnimationTarget::Position,
                vec![
                    Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                    Keyframe::new(
                        0.125,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        0.2083,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.0417,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.2917,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.3333,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                    Keyframe::new(
                        2.9583,
                        pos_vec(0.0, 0.0, 0.0),
                        AnimationInterpolation::Linear,
                    ),
                ],
            ));
        AnimationDefinition::new(3.0, true, map)
    });

pub fn apply_copper_golem_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    }
    if state.main_hand_item_empty && state.off_hand_item_empty {
        COPPER_GOLEM_WALK.apply_walk(
            tree,
            state.walk_animation_pos,
            state.walk_animation_speed,
            2.0,
            2.5,
        );
    } else {
        COPPER_GOLEM_WALK_ITEM.apply_walk(
            tree,
            state.walk_animation_pos,
            state.walk_animation_speed,
            2.0,
            2.5,
        );
        if let Some(arm) = tree.part_mut("right_arm") {
            arm.pose.rotation.x = arm.pose.rotation.x.min(-0.87266463);
            arm.pose.rotation.y = arm.pose.rotation.y.min(-0.1134464);
            arm.pose.rotation.z = arm.pose.rotation.z.min(-0.064577185);
        }
        if let Some(arm) = tree.part_mut("left_arm") {
            arm.pose.rotation.x = arm.pose.rotation.x.min(-0.87266463);
            arm.pose.rotation.y = arm.pose.rotation.y.max(0.1134464);
            arm.pose.rotation.z = arm.pose.rotation.z.max(0.064577185);
        }
    }
    COPPER_GOLEM_IDLE.apply_state(
        tree,
        &state.copper_golem_idle_animation_state,
        state.age_in_ticks,
    );
    COPPER_GOLEM_CHEST_INTERACTION_NOITEM_GET.apply_state(
        tree,
        &state.copper_golem_interaction_get_item_animation_state,
        state.age_in_ticks,
    );
    COPPER_GOLEM_CHEST_INTERACTION_NOITEM_NOGET.apply_state(
        tree,
        &state.copper_golem_interaction_get_no_item_animation_state,
        state.age_in_ticks,
    );
    COPPER_GOLEM_CHEST_INTERACTION_ITEM_DROP.apply_state(
        tree,
        &state.copper_golem_interaction_drop_item_animation_state,
        state.age_in_ticks,
    );
    COPPER_GOLEM_CHEST_INTERACTION_ITEM_NODROP.apply_state(
        tree,
        &state.copper_golem_interaction_drop_no_item_animation_state,
        state.age_in_ticks,
    );
}

pub fn build_copper_golem_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_copper_golem_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

pub fn build_copper_golem_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_copper_golem_model_tree();
    tree.reset_pose();
    apply_copper_golem_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
