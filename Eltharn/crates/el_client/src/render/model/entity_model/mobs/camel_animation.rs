use crate::render::model::animation::{
    AnimationChannel, AnimationDefinition, AnimationInterpolation, AnimationTarget, Keyframe,
    degree_vec, pos_vec,
};
use std::collections::HashMap;
use std::sync::LazyLock;

pub static CAMEL_WALK: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("root")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, 0.0, 2.5),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(0.0, 0.0, -2.5),
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
                    degree_vec(2.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(-2.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(2.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.125,
                    degree_vec(-2.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(22.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(-22.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.4583,
                    pos_vec(0.0, 4.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.75,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-22.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(22.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.75,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.2083,
                    pos_vec(0.0, 4.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-20.4, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(22.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.375,
                    degree_vec(-22.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(0.0, -0.21, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.75,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.0833,
                    pos_vec(0.0, 4.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.375,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(22.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.625,
                    degree_vec(-22.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    pos_vec(0.0, 4.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.625,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(0.0, 0.0, -22.5),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.125,
                    degree_vec(0.0, 0.0, -22.5),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(0.0, 0.0, 22.5),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.125,
                    degree_vec(0.0, 0.0, 22.5),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("tail")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(15.94102, -8.42106, 20.94102),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(15.94102, 8.42106, -20.94102),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    AnimationDefinition::new(1.5, true, map)
});

pub static CAMEL_SIT: LazyLock<AnimationDefinition> = LazyLock::new(|| {
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
                    1.3,
                    degree_vec(30.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.8,
                    degree_vec(24.0, 0.0, 0.0),
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
                Keyframe::new(1.3, pos_vec(0.0, 0.0, 1.0), AnimationInterpolation::Linear),
                Keyframe::new(1.8, pos_vec(0.0, -6.0, 1.0), AnimationInterpolation::Linear),
            ],
        ));
    map.entry("right_front_leg")
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
                    1.0,
                    degree_vec(-30.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    degree_vec(-30.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    1.0,
                    pos_vec(0.0, -2.0, 11.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    pos_vec(0.0, -2.0, 11.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    pos_vec(0.0, -8.4, 11.4),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_front_leg")
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
                    1.0,
                    degree_vec(-30.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    degree_vec(-30.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    1.0,
                    pos_vec(0.0, -2.0, 11.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    pos_vec(0.0, -2.0, 11.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    pos_vec(0.0, -8.4, 11.4),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_hind_leg")
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
                    0.5,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    degree_vec(-15.0, -3.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    degree_vec(-65.0, -9.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(0.5, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(1.5, pos_vec(0.0, 0.0, 1.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    1.7,
                    pos_vec(1.0, -0.62, 0.25),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    pos_vec(0.5, -11.25, 2.5),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_hind_leg")
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
                    0.5,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    degree_vec(-15.0, 3.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    degree_vec(-65.0, 9.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(0.5, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(1.5, pos_vec(0.0, 0.0, 1.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    1.7,
                    pos_vec(-1.0, -0.62, 0.25),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    pos_vec(-0.5, -11.25, 2.5),
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
                    0.7,
                    degree_vec(-27.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    degree_vec(-21.25, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("tail")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(5.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    degree_vec(5.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    degree_vec(80.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    AnimationDefinition::new(2.0, false, map)
});

pub static CAMEL_SIT_POSE: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![Keyframe::new(
                0.0,
                degree_vec(0.0, 0.0, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![Keyframe::new(
                0.0,
                pos_vec(0.0, -19.9, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![Keyframe::new(
                0.0,
                degree_vec(-90.0, 10.0, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![Keyframe::new(
                0.0,
                pos_vec(0.0, -20.6, 12.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![Keyframe::new(
                0.0,
                degree_vec(-90.0, -10.0, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![Keyframe::new(
                0.0,
                pos_vec(0.0, -20.6, 12.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![Keyframe::new(
                0.0,
                degree_vec(-90.0, -15.0, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![Keyframe::new(
                0.0,
                pos_vec(1.0, -20.5, 5.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![Keyframe::new(
                0.0,
                degree_vec(-90.0, 15.0, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![Keyframe::new(
                0.0,
                pos_vec(-1.0, -20.5, 5.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("head")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![Keyframe::new(
                0.0,
                degree_vec(0.0, 0.0, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("tail")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![Keyframe::new(
                0.0,
                degree_vec(50.0, 0.0, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    AnimationDefinition::new(1.0, false, map)
});

pub static CAMEL_STANDUP: LazyLock<AnimationDefinition> = LazyLock::new(|| {
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
                    0.7,
                    degree_vec(-17.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.8,
                    degree_vec(-17.83, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.3,
                    degree_vec(-5.83, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(0.0, -19.9, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7,
                    pos_vec(0.0, -19.9, -3.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    pos_vec(0.0, -12.76, -4.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.8,
                    pos_vec(0.0, -10.1, -4.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.3,
                    pos_vec(0.0, -2.9, -2.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-90.0, 10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    degree_vec(-90.0, 10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    degree_vec(-49.06, 10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.8,
                    degree_vec(-22.5, 10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.3,
                    degree_vec(-25.0, 10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(0.0, -20.6, 12.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    pos_vec(0.0, -20.6, 8.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    pos_vec(0.0, -7.14, 4.42),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.8,
                    pos_vec(0.0, -1.27, -1.33),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.3,
                    pos_vec(0.0, -1.27, -0.33),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-90.0, -10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    degree_vec(-90.0, -10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    degree_vec(-49.06, -10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.8,
                    degree_vec(-22.5, -10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.3,
                    degree_vec(-25.0, -10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(0.0, -20.6, 12.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    pos_vec(0.0, -20.6, 8.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    pos_vec(0.0, -7.14, 4.42),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.8,
                    pos_vec(0.0, -1.27, -1.33),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.3,
                    pos_vec(0.0, -1.27, -0.33),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-90.0, -15.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.3,
                    degree_vec(-90.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.6,
                    degree_vec(-90.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    degree_vec(-60.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    degree_vec(35.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.2,
                    degree_vec(30.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(1.0, -20.5, 5.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.3,
                    pos_vec(-2.0, -20.5, 3.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.6,
                    pos_vec(-2.0, -20.5, 3.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    pos_vec(-2.0, -10.5, 2.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    pos_vec(-2.0, -0.4, -3.9),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    pos_vec(-2.0, -4.3, -9.8),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.2,
                    pos_vec(-1.0, -2.5, -5.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-90.0, 15.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.3,
                    degree_vec(-90.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.6,
                    degree_vec(-90.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    degree_vec(-60.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    degree_vec(35.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.2,
                    degree_vec(30.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(-1.0, -20.5, 5.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.3,
                    pos_vec(2.0, -20.5, 3.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.6,
                    pos_vec(2.0, -20.5, 3.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    pos_vec(2.0, -10.5, 2.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    pos_vec(2.0, -0.4, -3.9),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    pos_vec(2.0, -4.3, -9.8),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.2,
                    pos_vec(1.0, -2.5, -5.0),
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
                    0.3,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.8,
                    degree_vec(55.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.0,
                    degree_vec(65.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("tail")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(50.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.4,
                    degree_vec(55.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    degree_vec(55.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    degree_vec(17.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    AnimationDefinition::new(2.6, false, map)
});

pub static CAMEL_DASH: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![Keyframe::new(
                0.0,
                degree_vec(5.0, 0.0, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("tail")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(67.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(112.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(67.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(112.5, 0.0, 0.0),
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
                    degree_vec(10.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(10.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(44.97272, 1.76749, -1.76833),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(-90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(44.97272, 1.76749, -1.76833),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(-90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(44.97272, -1.76749, 1.76833),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(-90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(44.97272, -1.76749, 1.76833),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(-45.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(-45.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-45.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(-45.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![Keyframe::new(
                0.0,
                degree_vec(0.0, -67.5, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("right_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![Keyframe::new(
                0.0,
                degree_vec(0.0, 67.5, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    AnimationDefinition::new(0.5, true, map)
});

pub static CAMEL_IDLE: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("tail")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(5.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(4.98107, 0.43523, -4.98107),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    3.0,
                    degree_vec(4.9872, -0.29424, 3.36745),
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
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.0,
                    degree_vec(-2.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    2.5,
                    degree_vec(0.0, 0.0, -45.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.625,
                    degree_vec(0.0, 0.0, 22.5),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.75,
                    degree_vec(0.0, 0.0, -45.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.875,
                    degree_vec(0.0, 0.0, 22.5),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    2.5,
                    degree_vec(0.0, 0.0, 45.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.625,
                    degree_vec(0.0, 0.0, -22.5),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.75,
                    degree_vec(0.0, 0.0, 45.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.875,
                    degree_vec(0.0, 0.0, -22.5),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    AnimationDefinition::new(4.0, false, map)
});

pub static CAMEL_BABY_WALK: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("root")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, 0.0, 2.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(0.0, 0.0, -2.5),
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
                    degree_vec(2.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(-2.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(2.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.125,
                    degree_vec(-2.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("head")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![Keyframe::new(
                0.0,
                pos_vec(0.0, 0.0, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-22.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(22.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(0.075, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.75,
                    pos_vec(0.075, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.2083,
                    pos_vec(0.075, 4.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(22.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(-22.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(0.0, pos_vec(-0.1, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    0.4583,
                    pos_vec(-0.1, 4.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.75,
                    pos_vec(-0.1, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(22.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(-9.49, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.5833,
                    degree_vec(-17.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.2083,
                    degree_vec(7.38, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(0.0, pos_vec(-0.1, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    0.25,
                    pos_vec(-0.1, 5.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.5833,
                    pos_vec(-0.1, 0.0, -0.1),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-15.83, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(22.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(-7.38, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.25,
                    degree_vec(-21.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(0.0, pos_vec(0.1, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    0.6667,
                    pos_vec(0.1, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.0,
                    pos_vec(0.1, 4.0, 0.17),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.2083,
                    pos_vec(0.1, 0.0, -0.11),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_ear")
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
                    0.375,
                    degree_vec(0.0, 0.0, 22.5),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.125,
                    degree_vec(0.0, 0.0, 22.5),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_ear")
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
                    0.375,
                    degree_vec(0.0, 0.0, -22.5),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.125,
                    degree_vec(0.0, 0.0, -22.5),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("tail")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(15.94, -8.42, 20.94),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.75,
                    degree_vec(15.94, 8.42, -20.94),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![Keyframe::new(
                0.0,
                pos_vec(0.0, -0.6, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    AnimationDefinition::new(1.5, true, map)
});

pub static CAMEL_BABY_STANDUP: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(2.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2,
                    degree_vec(2.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    degree_vec(-12.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(-12.6, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.3,
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
                Keyframe::new(
                    0.0,
                    pos_vec(0.0, -13.25, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2,
                    pos_vec(0.0, -13.25, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.3,
                    pos_vec(0.0, -11.52, -0.85),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    pos_vec(0.0, -9.4335, -1.6246),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    pos_vec(0.0, -9.15, -1.58),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2,
                    pos_vec(0.0, -8.5, -1.3),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    pos_vec(0.0, -1.7, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(2.3, pos_vec(0.0, -0.1, 0.0), AnimationInterpolation::Linear),
            ],
        ));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-90.0, 10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2,
                    degree_vec(-75.21, 9.37, 2.45),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.3,
                    degree_vec(-70.21, 9.37, 2.45),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.8,
                    degree_vec(-40.55, 8.68, 1.23),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(-40.55, 8.68, 1.23),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2,
                    degree_vec(-20.45, 7.61, 0.74),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    degree_vec(-20.45, 7.61, 0.74),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.6,
                    degree_vec(-20.45, 7.61, 0.74),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.1,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.3,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(0.1, -11.0, 6.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(0.2, pos_vec(0.1, -8.0, 6.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    0.3,
                    pos_vec(-0.12, -7.18, 5.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.8,
                    pos_vec(-0.9942, -2.3474, 2.2929),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    pos_vec(-0.9942, -2.3474, 2.2929),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2,
                    pos_vec(-0.72, -0.6, 1.71),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    pos_vec(-0.72, -0.6, 1.71),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.6,
                    pos_vec(-0.72, -0.6, 1.71),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    pos_vec(-0.21, -0.55, 1.05),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.8,
                    pos_vec(-0.21, 0.73, 0.52),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(1.9, pos_vec(0.05, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(2.1, pos_vec(0.05, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(2.2, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(2.3, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
            ],
        ));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-90.0, -10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2,
                    degree_vec(-74.2, -9.3, -2.45),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.3,
                    degree_vec(-69.2, -9.32, -2.4),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.8,
                    degree_vec(-46.75, -10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(-46.75, -10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2,
                    degree_vec(-24.31, -9.2, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    degree_vec(-24.31, -9.2, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    degree_vec(-24.31, -9.2, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    degree_vec(-12.15, -4.6, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.0,
                    degree_vec(-12.15, -4.6, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.2,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.5,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(0.0, -11.0, 6.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2,
                    pos_vec(0.0, -7.89, 4.96),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.3,
                    pos_vec(0.0, -6.89, 4.96),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.8,
                    pos_vec(0.65, -2.98, 3.56),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    pos_vec(0.65, -2.98, 3.56),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2,
                    pos_vec(0.5, -0.69, 2.47),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    pos_vec(0.5, -0.69, 2.47),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    pos_vec(0.5, -0.69, 2.47),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(1.9, pos_vec(0.4, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(2.1, pos_vec(0.2, 1.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    2.2,
                    pos_vec(-0.025, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.3,
                    pos_vec(-0.02, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(2.5, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
            ],
        ));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-90.0, -12.0, -0.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2,
                    degree_vec(-90.0, -12.0, -0.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    degree_vec(-89.79, -17.4, -0.8),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.8,
                    degree_vec(-89.38, -11.59, -0.99),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    degree_vec(-89.09, -7.49, -0.98),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(-88.5, -4.3, -1.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2,
                    degree_vec(-79.5, -4.3, -1.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    degree_vec(-66.11, -4.19, -1.05),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    degree_vec(-47.51, -4.19, -1.05),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    degree_vec(-35.11, -4.19, -1.05),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.8,
                    degree_vec(-20.63, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    degree_vec(5.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.1,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(1.5, -10.949, 1.9507),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2,
                    pos_vec(1.5, -10.949, 1.9507),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    pos_vec(1.0, -11.0, 1.75),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.8,
                    pos_vec(0.16, -11.11, 0.49),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    pos_vec(-0.81, -10.95, -0.02),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    pos_vec(-0.77, -10.8, -0.94),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2,
                    pos_vec(-0.77, -8.7, -0.94),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    pos_vec(-1.6, -6.43, -1.59),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    pos_vec(-1.36, -3.37, -1.65),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    pos_vec(-1.2, -1.33, -1.69),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.8,
                    pos_vec(-0.6, 2.0, -0.3),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    pos_vec(-0.2, 1.9, -1.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(2.1, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
            ],
        ));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-90.0, 12.0, 0.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2,
                    degree_vec(-90.0, 12.0, 0.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    degree_vec(-90.0, 17.5, 1.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.8,
                    degree_vec(-90.0, 8.5, 0.73),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    degree_vec(-90.0, 3.0, 0.37),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(-75.38, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    degree_vec(-56.38, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    degree_vec(-30.15, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.6,
                    degree_vec(3.6, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.1,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(-1.3, -11.0, 1.95),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2,
                    pos_vec(-1.3, -11.0, 1.95),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    pos_vec(-0.7, -10.9, 1.8),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.8,
                    pos_vec(-0.06, -11.2, 0.21),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    pos_vec(0.39, -11.2, -0.23),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    pos_vec(0.24, -8.0, -1.27),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    pos_vec(0.34, -4.53, -0.8),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    pos_vec(0.3, -1.88, -0.42),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    pos_vec(0.25, 0.18, 0.3),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.6,
                    pos_vec(0.25, 1.48, -1.75),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(1.7, pos_vec(0.1, 0.0, -0.4), AnimationInterpolation::Linear),
                Keyframe::new(1.9, pos_vec(0.1, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(2.1, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
            ],
        ));
    map.entry("head")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-4.75, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2,
                    degree_vec(5.65, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    degree_vec(40.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    degree_vec(46.68, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.6,
                    degree_vec(37.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.0,
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
                    0.5,
                    pos_vec(0.0, 0.8368, 1.9553),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.8,
                    pos_vec(0.0, -0.95, 1.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.3,
                    pos_vec(0.0, -0.95, 1.42),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("tail")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(46.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2,
                    degree_vec(46.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.3,
                    degree_vec(39.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    degree_vec(40.75, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7,
                    degree_vec(47.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.8,
                    degree_vec(49.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    degree_vec(52.75, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(50.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    degree_vec(42.75, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    degree_vec(1.75, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    degree_vec(7.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.8,
                    degree_vec(6.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.3,
                    degree_vec(5.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("tail")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(0.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(0.2, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(0.3, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(0.7, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(0.8, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(1.0, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(2.3, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
            ],
        ));
    map.entry("right_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, 0.0, 15.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(0.0, 0.0, 15.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    degree_vec(0.0, 0.0, 15.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.6,
                    degree_vec(0.0, 0.0, 5.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    degree_vec(0.0, 0.0, -23.75),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.2,
                    degree_vec(0.0, 0.0, -23.75),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.3,
                    degree_vec(0.0, 0.0, 12.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.4,
                    degree_vec(0.0, 0.0, 67.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.5,
                    degree_vec(0.0, 0.0, 45.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(2.3, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(2.4, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(2.5, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
            ],
        ));
    map.entry("left_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, 0.0, -22.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(0.0, 0.0, -22.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    degree_vec(0.0, 0.0, -22.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.6,
                    degree_vec(0.0, 0.0, -7.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.9,
                    degree_vec(0.0, 0.0, 22.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.2,
                    degree_vec(0.0, 0.0, 22.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.3,
                    degree_vec(0.0, 0.0, -22.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.4,
                    degree_vec(0.0, 0.0, -55.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    2.5,
                    degree_vec(0.0, 0.0, -45.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(2.3, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(2.4, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(2.5, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
            ],
        ));
    AnimationDefinition::new(2.6, true, map)
});

pub static CAMEL_BABY_DASH: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![Keyframe::new(
                0.0,
                degree_vec(5.0, 0.0, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("tail")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(67.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(112.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(67.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(112.5, 0.0, 0.0),
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
                    degree_vec(5.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(5.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("head")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![Keyframe::new(
                0.0,
                pos_vec(0.0, 0.0, 1.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(44.9727, 1.7675, -1.7683),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(-90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(44.9727, 1.7675, -1.7683),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(-90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(44.9727, -1.7675, 1.7683),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(-90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(44.9727, -1.7675, 1.7683),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(-45.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(-45.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(-45.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(-45.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(90.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("left_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![Keyframe::new(
                0.0,
                degree_vec(0.0, -67.5, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    map.entry("right_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![Keyframe::new(
                0.0,
                degree_vec(0.0, 67.5, 0.0),
                AnimationInterpolation::Linear,
            )],
        ));
    AnimationDefinition::new(0.5, true, map)
});

pub static CAMEL_BABY_IDLE: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("tail")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(5.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(4.9811, 0.4352, -4.9811),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    3.0,
                    degree_vec(4.9872, -0.2942, 3.3674),
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
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.0,
                    degree_vec(-2.5, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("left_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, 0.0, -45.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.5,
                    degree_vec(0.0, 0.0, -45.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.625,
                    degree_vec(0.0, 0.0, 22.5),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.75,
                    degree_vec(0.0, 0.0, -45.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.875,
                    degree_vec(0.0, 0.0, 22.5),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    3.0,
                    degree_vec(0.0, 0.0, -45.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("right_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, 0.0, 45.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.5,
                    degree_vec(0.0, 0.0, 45.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.625,
                    degree_vec(0.0, 0.0, -22.5),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.75,
                    degree_vec(0.0, 0.0, 45.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    2.875,
                    degree_vec(0.0, 0.0, -22.5),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    3.0,
                    degree_vec(0.0, 0.0, 45.0),
                    AnimationInterpolation::CatmullRom,
                ),
            ],
        ));
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    AnimationDefinition::new(4.0, true, map)
});

pub static CAMEL_BABY_SIT: LazyLock<AnimationDefinition> = LazyLock::new(|| {
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
                    0.4,
                    degree_vec(9.26, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.6,
                    degree_vec(18.15, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    degree_vec(30.0, 0.0, 0.0),
                    AnimationInterpolation::CatmullRom,
                ),
                Keyframe::new(
                    1.2,
                    degree_vec(21.14, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    degree_vec(15.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    degree_vec(2.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.6,
                    degree_vec(2.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    degree_vec(2.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(0.0, pos_vec(0.0, -0.1, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(0.2, pos_vec(0.0, -1.5, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    0.4,
                    pos_vec(0.0, -2.21, 0.04),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.6,
                    pos_vec(0.0, -2.21, 0.04),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    pos_vec(0.0, -5.23, 0.33),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2,
                    pos_vec(0.0, -6.06, 0.17),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    pos_vec(0.0, -11.7, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    pos_vec(0.0, -13.35, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.6,
                    pos_vec(0.0, -13.25, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    pos_vec(0.0, -13.25, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_front_leg")
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
                    0.5,
                    degree_vec(-2.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7,
                    degree_vec(-23.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(-30.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    degree_vec(-44.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    degree_vec(-73.25, 6.5, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    degree_vec(-90.0, 10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    degree_vec(-90.0, 10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(0.125, -0.1, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    pos_vec(0.125, -0.1, 0.4),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.6,
                    pos_vec(0.12, -0.15, 2.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7,
                    pos_vec(0.12, -0.6, 4.6),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    pos_vec(0.12, -0.87, 5.97),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    pos_vec(0.125, -1.0, 6.9),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    pos_vec(0.325, -2.6, 7.6),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    pos_vec(-0.28, -7.45, 8.06),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    pos_vec(0.1, -11.0, 6.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    pos_vec(0.1, -11.0, 6.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_front_leg")
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
                    0.5,
                    degree_vec(-2.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7,
                    degree_vec(-23.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    degree_vec(-30.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    degree_vec(-45.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    degree_vec(-73.5, -6.5, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    degree_vec(-90.0, -10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    degree_vec(-90.0, -10.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(
                    0.0,
                    pos_vec(-0.125, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.5,
                    pos_vec(-0.125, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.6,
                    pos_vec(-0.13, -0.18, 2.35),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.7,
                    pos_vec(-0.13, -0.57, 4.7),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    pos_vec(-0.13, -0.86, 6.07),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.0,
                    pos_vec(-0.125, -1.0, 6.8),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    pos_vec(-0.22, -2.7, 7.8),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    pos_vec(0.19, -7.5, 8.01),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    pos_vec(0.0, -11.0, 6.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.7,
                    pos_vec(0.0, -10.9, 6.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_hind_leg")
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
                    0.5,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    degree_vec(-31.47, -0.45, 1.29),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2,
                    degree_vec(-49.78, -10.29, -1.15),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    degree_vec(-79.21, -13.72, -1.53),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    degree_vec(-90.0, -11.88, -0.54),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(0.0, pos_vec(-0.1, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(0.5, pos_vec(-0.1, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(0.9, pos_vec(-0.1, 0.0, 1.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    1.1,
                    pos_vec(0.56, -3.49, 2.18),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    pos_vec(1.22, -8.72, 3.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    pos_vec(1.4, -10.95, 1.95),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_hind_leg")
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
                    0.5,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    degree_vec(-10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    degree_vec(-31.13, 0.56, -1.3),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2,
                    degree_vec(-49.8, 10.3, 1.15),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    degree_vec(-79.21, 13.72, 1.53),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    degree_vec(-90.0, 11.89, 0.54),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(0.0, pos_vec(0.1, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(0.5, pos_vec(0.1, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(0.9, pos_vec(0.1, 0.0, 1.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    1.1,
                    pos_vec(0.2, -0.75, 1.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2,
                    pos_vec(-0.51, -3.49, 2.18),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    pos_vec(-1.22, -8.72, 3.1),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    pos_vec(-1.33, -11.0, 1.95),
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
                    0.1,
                    degree_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.4,
                    degree_vec(-20.42, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    degree_vec(-31.25, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    degree_vec(-26.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2,
                    degree_vec(-19.25, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    degree_vec(-14.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    degree_vec(-4.75, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.6,
                    degree_vec(-4.75, 0.0, 0.0),
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
                Keyframe::new(0.1, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    0.4,
                    pos_vec(0.0, 0.38, 0.44),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    pos_vec(0.0, 0.35, 0.88),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.1,
                    pos_vec(0.0, -0.3, 1.21),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    pos_vec(0.0, -0.3, 1.22),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("tail")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(5.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.9,
                    degree_vec(5.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2,
                    degree_vec(25.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    degree_vec(52.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.6,
                    degree_vec(62.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.8,
                    degree_vec(52.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("tail")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Position,
            vec![
                Keyframe::new(1.4, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(1.8, pos_vec(0.0, 0.0, 0.0), AnimationInterpolation::Linear),
            ],
        ));
    map.entry("left_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, 0.0, -45.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2,
                    degree_vec(0.0, 0.0, -45.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    degree_vec(0.0, 0.0, -62.5),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    degree_vec(0.0, 0.0, -45.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    degree_vec(0.0, 0.0, 20.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.6,
                    degree_vec(0.0, 0.0, -22.5),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("right_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, 0.0, 45.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.2,
                    degree_vec(0.0, 0.0, 45.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.3,
                    degree_vec(0.0, 0.0, 60.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.4,
                    degree_vec(0.0, 0.0, 45.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.5,
                    degree_vec(0.0, 0.0, -20.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    1.6,
                    degree_vec(0.0, 0.0, 15.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    AnimationDefinition::new(2.0, true, map)
});

pub static CAMEL_BABY_SIT_POSE: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    map.entry("right_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    map.entry("left_front_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    map.entry("left_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    map.entry("right_hind_leg")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("head")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    map.entry("head")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("tail")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    map.entry("tail")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("left_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    map.entry("left_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("right_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    map.entry("right_ear")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("root")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("root")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    AnimationDefinition::new(0.0, true, map)
});
