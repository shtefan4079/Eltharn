use crate::render::model::animation::{
    AnimationChannel, AnimationDefinition, AnimationInterpolation, AnimationTarget, Keyframe,
    degree_vec, pos_vec,
};
use std::collections::HashMap;
use std::sync::LazyLock;

pub static BAT_RESTING: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.entry("head")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    map.entry("head")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("feet")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    map.entry("right_wing")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    map.entry("right_wing")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("right_wing_tip")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    map.entry("left_wing")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    map.entry("left_wing")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Position, vec![]));
    map.entry("left_wing_tip")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(AnimationTarget::Rotation, vec![]));
    AnimationDefinition::new(0.5, true, map)
});

pub static BAT_FLYING: LazyLock<AnimationDefinition> = LazyLock::new(|| {
    let mut map = HashMap::new();
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
                    degree_vec(20.0, 0.0, 0.0),
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
                    pos_vec(0.0, 2.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(0.25, pos_vec(0.0, 1.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    0.375,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.4583,
                    pos_vec(0.0, -1.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("body")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(40.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(52.5, 0.0, 0.0),
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
                    pos_vec(0.0, 2.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(0.25, pos_vec(0.0, 1.0, 0.0), AnimationInterpolation::Linear),
                Keyframe::new(
                    0.375,
                    pos_vec(0.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.4583,
                    pos_vec(0.0, -1.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("feet")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(10.0, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(-21.25, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(-12.5, 0.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_wing")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, 85.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(0.0, -55.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(0.0, 50.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(0.0, 70.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("right_wing_tip")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, 10.5, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.0417,
                    degree_vec(0.0, 65.5, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2083,
                    degree_vec(0.0, -135.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_wing")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, -85.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.125,
                    degree_vec(0.0, 55.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.25,
                    degree_vec(0.0, -50.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.375,
                    degree_vec(0.0, -70.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    map.entry("left_wing_tip")
        .or_insert_with(Vec::new)
        .push(AnimationChannel::new(
            AnimationTarget::Rotation,
            vec![
                Keyframe::new(
                    0.0,
                    degree_vec(0.0, -10.5, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.0417,
                    degree_vec(0.0, -65.5, 0.0),
                    AnimationInterpolation::Linear,
                ),
                Keyframe::new(
                    0.2083,
                    degree_vec(0.0, 135.0, 0.0),
                    AnimationInterpolation::Linear,
                ),
            ],
        ));
    AnimationDefinition::new(0.5, true, map)
});
