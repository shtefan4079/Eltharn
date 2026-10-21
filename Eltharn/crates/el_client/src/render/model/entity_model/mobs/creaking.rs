#![allow(dead_code)]
use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;
use crate::render::model::animation::{
    AnimationChannel, AnimationDefinition, AnimationInterpolation, AnimationTarget, Keyframe,
    degree_vec, pos_vec, scale_vec,
};
use std::collections::HashMap;
use std::sync::LazyLock;
pub fn build_creaking_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::offset(0.0, 24.0, 0.0))
        .child(
            NamedPartSpec::new("upper_body", PartPose::offset(-1.0, -19.0, 0.0))
                .child(
                    NamedPartSpec::new("head", PartPose::offset(-3.0, -11.0, 0.0))
                        .cube(cube([0.0, 0.0], [-3.0, -10.0, -3.0], [6.0, 10.0, 6.0]))
                        .cube(cube([28.0, 31.0], [-3.0, -13.0, -3.0], [6.0, 3.0, 6.0]))
                        .cube(cube([12.0, 40.0], [3.0, -13.0, 0.0], [9.0, 14.0, 0.0]))
                        .cube(cube([34.0, 12.0], [-12.0, -14.0, 0.0], [9.0, 14.0, 0.0])),
                )
                .child(
                    NamedPartSpec::new("body", PartPose::offset(0.0, -7.0, 1.0))
                        .cube(cube([0.0, 16.0], [0.0, -3.0, -3.0], [6.0, 13.0, 5.0]))
                        .cube(cube([24.0, 0.0], [-6.0, -4.0, -3.0], [6.0, 7.0, 5.0])),
                )
                .child(
                    NamedPartSpec::new("right_arm", PartPose::offset(-7.0, -9.5, 1.5))
                        .cube(cube([22.0, 13.0], [-2.0, -1.5, -1.5], [3.0, 21.0, 3.0]))
                        .cube(cube([46.0, 0.0], [-2.0, 19.5, -1.5], [3.0, 4.0, 3.0])),
                )
                .child(
                    NamedPartSpec::new("left_arm", PartPose::offset(6.0, -9.0, 0.5))
                        .cube(cube([30.0, 40.0], [0.0, -1.0, -1.5], [3.0, 16.0, 3.0]))
                        .cube(cube([52.0, 12.0], [0.0, -5.0, -1.5], [3.0, 4.0, 3.0]))
                        .cube(cube([52.0, 19.0], [0.0, 15.0, -1.5], [3.0, 4.0, 3.0])),
                ),
        )
        .child(
            NamedPartSpec::new("left_leg", PartPose::offset(1.5, -16.0, 0.5))
                .cube(cube([42.0, 40.0], [-1.5, 0.0, -1.5], [3.0, 16.0, 3.0]))
                .cube(cube([45.0, 55.0], [-1.5, 15.7, -4.5], [5.0, 0.0, 9.0])),
        )
        .child(
            NamedPartSpec::new("right_leg", PartPose::offset(-1.0, -17.5, 0.5))
                .cube(cube([0.0, 34.0], [-3.0, -1.5, -1.5], [3.0, 19.0, 3.0]))
                .cube(cube([45.0, 46.0], [-5.0, 17.2, -4.5], [5.0, 0.0, 9.0]))
                .cube(cube([12.0, 34.0], [-3.0, -4.5, -1.5], [3.0, 3.0, 3.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub static CREAKING_WALK: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("upper_body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(26.8802, -23.399, -9.0616),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(-2.2093, 5.9119, 0.0675),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5417,
                    degree_vec(23.0778, 14.2906, 4.6066),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7083,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.875,
                    degree_vec(7.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.125,
                    degree_vec(26.8802, -23.399, -9.0616),
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
                    0.0417,
                    degree_vec(-17.5, -62.5, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.0833,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.4167,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.4583,
                    degree_vec(0.0, 15.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0417,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0833,
                    degree_vec(-37.1532, 81.1131, -28.3621),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.125,
                    degree_vec(0.0, 0.0, 0.0),
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
                    degree_vec(12.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(-32.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.875,
                    degree_vec(12.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.125,
                    degree_vec(-15.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
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
                    degree_vec(-15.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5417,
                    degree_vec(-25.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(-9.0923, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7917,
                    degree_vec(-15.137, -66.7758, 13.9603),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.8333,
                    degree_vec(-9.0923, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.125,
                    degree_vec(-15.0, 0.0, 0.0),
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
                    0.25,
                    degree_vec(30.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(49.8924, -3.8282, 3.2187),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    degree_vec(17.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.625,
                    degree_vec(-56.5613, -12.2403, -8.7374),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9167,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.125,
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
                Keyframe::new(0.0, pos_vec(0.0, 0.0, 2.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    0.25,
                    pos_vec(0.0, 0.1846, 0.5979),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.375,
                    pos_vec(0.0, -0.0665, -2.2177),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    pos_vec(0.0, 1.3563, -4.3474),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.625,
                    pos_vec(0.0, 0.1047, -1.6556),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9167,
                    pos_vec(0.0, 0.0, -1.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.125,
                    pos_vec(0.0, 0.0, 2.0),
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
                    degree_vec(25.5305, 11.3125, 5.3525),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(-49.5628, 7.3556, 6.7933),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.4583,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9167,
                    degree_vec(30.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0417,
                    degree_vec(55.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.125,
                    degree_vec(25.5305, 11.3125, 5.3525),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(0.0, 0.9674, -3.6578),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.125,
                    pos_vec(0.0, -0.2979, -0.9411),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.25,
                    pos_vec(0.0, -0.3, -0.94),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.4583,
                    pos_vec(0.0, -0.3, 1.06),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.125,
                    pos_vec(0.0, 0.9674, -3.6578),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    AnimationDefinition::new(1.125, true, map)
});
pub static CREAKING_ATTACK: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("upper_body")
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
                    0.0833,
                    degree_vec(0.0, 45.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.1667,
                    degree_vec(-115.0, 67.5, -90.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(67.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5417,
                    degree_vec(0.0, 45.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7083,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("upper_body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    0.0833,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2917,
                    pos_vec(0.0, -2.7716, -1.1481),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.375,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5417,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7083,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("upper_body")
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
                    0.7083,
                    scale_vec(1.0, 1.0, 1.0),
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
                    0.1667,
                    degree_vec(0.0, -45.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(-11.25, -45.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2917,
                    degree_vec(-117.3939, 76.6331, -130.1483),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.4167,
                    degree_vec(-45.0, -45.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    degree_vec(60.0, -45.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5833,
                    degree_vec(60.0, -45.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.625,
                    degree_vec(0.0, -45.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7083,
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
                    0.1667,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.4167,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    pos_vec(0.3827, 0.5133, -0.7682),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5833,
                    pos_vec(0.3827, 0.5133, -0.7682),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.625,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7083,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("head")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Scale,
            vec![
                Keyframe::new(
                    0.1667,
                    scale_vec(1.0, 1.0, 1.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.4167,
                    scale_vec(1.0, 1.0, 1.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    scale_vec(1.0, 1.3, 1.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.625,
                    scale_vec(1.0, 1.0, 1.0),
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
                    0.1667,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(7.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.4583,
                    degree_vec(55.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.625,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7083,
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
                    0.1667,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.625,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7083,
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
                    0.1667,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.625,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7083,
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
                    0.1667,
                    pos_vec(0.0, 0.0, -2.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.625,
                    pos_vec(0.0, 0.0, -2.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7083,
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
                    0.1667,
                    degree_vec(0.0, 45.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.625,
                    degree_vec(0.0, 45.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7083,
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
                    0.1667,
                    pos_vec(0.7071, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.625,
                    pos_vec(0.7071, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7083,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
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
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.1667,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(10.3453, 14.7669, 2.664),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.4583,
                    degree_vec(57.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.625,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7083,
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
                    0.7083,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    AnimationDefinition::new(0.7083, true, map)
});
pub static CREAKING_INVULNERABLE: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("upper_body")
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
                    0.0833,
                    degree_vec(-5.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.1667,
                    degree_vec(5.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(0.0, 0.0, 0.0),
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
                    0.0833,
                    degree_vec(17.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.1667,
                    degree_vec(-15.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
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
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.0833,
                    degree_vec(20.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.1667,
                    degree_vec(-15.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    AnimationDefinition::new(0.2917, false, map)
});
pub static CREAKING_DEATH: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("upper_body")
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
                    0.0833,
                    degree_vec(-40.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.1667,
                    degree_vec(-5.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2917,
                    degree_vec(7.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5833,
                    degree_vec(16.25, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.6667,
                    degree_vec(29.0814, 62.5516, 26.5771),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(12.2115, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(10.25, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0417,
                    degree_vec(-47.64, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.125,
                    degree_vec(21.96, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.25,
                    degree_vec(12.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.25,
                    degree_vec(17.3266, 7.9022, -0.1381),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("upper_body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    0.0833,
                    pos_vec(0.0, 0.557, 1.2659),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.1667,
                    pos_vec(0.0, -2.0889, -0.3493),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2917,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("upper_body")
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
                    0.0833,
                    scale_vec(1.0, 1.1, 1.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.1667,
                    scale_vec(1.0, 0.9, 1.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2917,
                    scale_vec(1.0, 1.0, 1.0),
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
                    0.2917,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.25,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5417,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5833,
                    degree_vec(-12.1479, -34.3927, 6.9326),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.6667,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
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
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2917,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.8333,
                    degree_vec(-4.4444, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.875,
                    degree_vec(-26.7402, -78.831, 26.3025),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9583,
                    degree_vec(-5.5556, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.25,
                    degree_vec(-10.0, 0.0, 0.0),
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
                    0.0833,
                    degree_vec(-5.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2917,
                    degree_vec(10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    degree_vec(2.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5417,
                    degree_vec(5.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5833,
                    degree_vec(-67.4168, -12.9552, -8.0231),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.6667,
                    degree_vec(8.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(10.773, -29.5608, -5.3627),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.25,
                    degree_vec(10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7917,
                    degree_vec(10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.8333,
                    degree_vec(12.9625, 39.2735, 8.2901),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9167,
                    degree_vec(10.0, 0.0, 0.0),
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
                    0.2917,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    AnimationDefinition::new(2.25, false, map)
});
pub fn apply_creaking_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    }
    if state.can_move {
        CREAKING_WALK.apply_walk(
            tree,
            state.walk_animation_pos,
            state.walk_animation_speed,
            1.0,
            1.0,
        );
    }
    CREAKING_ATTACK.apply_state(
        tree,
        &state.creaking_attack_animation_state,
        state.age_in_ticks,
    );
    CREAKING_INVULNERABLE.apply_state(
        tree,
        &state.creaking_invulnerability_animation_state,
        state.age_in_ticks,
    );
    CREAKING_DEATH.apply_state(
        tree,
        &state.creaking_death_animation_state,
        state.age_in_ticks,
    );
}
pub fn build_creaking_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_creaking_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_creaking_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_creaking_model_tree();
    tree.reset_pose();
    apply_creaking_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
