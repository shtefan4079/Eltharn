#![allow(dead_code)]

use super::entity_model::tree::JavaModelPartTree;
use glam::Vec3;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationState {
    pub start_tick: Option<f32>,
}

impl AnimationState {
    pub const fn new() -> Self {
        Self { start_tick: None }
    }

    pub fn start(&mut self, tick_count: f32) {
        self.start_tick = Some(tick_count);
    }

    pub fn start_if_stopped(&mut self, tick_count: f32) {
        if self.start_tick.is_none() {
            self.start(tick_count);
        }
    }

    pub fn animate_when(&mut self, condition: bool, tick_count: f32) {
        if condition {
            self.start_if_stopped(tick_count);
        } else {
            self.stop();
        }
    }

    pub fn stop(&mut self) {
        self.start_tick = None;
    }

    pub fn get_time_in_millis(&self, age_in_ticks: f32) -> Option<f32> {
        self.start_tick.map(|start| (age_in_ticks - start) * 50.0)
    }

    pub fn is_started(&self) -> bool {
        self.start_tick.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationTarget {
    Position,
    Rotation,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationInterpolation {
    Linear,
    CatmullRom,
}

#[derive(Debug, Clone)]
pub struct Keyframe {
    pub timestamp_sec: f32,
    pub pre_target: Vec3,
    pub post_target: Vec3,
    pub interpolation: AnimationInterpolation,
}

impl Keyframe {
    pub fn new(
        timestamp_sec: f32,
        post_target: Vec3,
        interpolation: AnimationInterpolation,
    ) -> Self {
        Self {
            timestamp_sec,
            pre_target: post_target,
            post_target,
            interpolation,
        }
    }

    pub fn new_split(
        timestamp_sec: f32,
        pre_target: Vec3,
        post_target: Vec3,
        interpolation: AnimationInterpolation,
    ) -> Self {
        Self {
            timestamp_sec,
            pre_target,
            post_target,
            interpolation,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnimationChannel {
    pub target: AnimationTarget,
    pub keyframes: Vec<Keyframe>,
}

impl AnimationChannel {
    pub fn new(target: AnimationTarget, keyframes: Vec<Keyframe>) -> Self {
        Self { target, keyframes }
    }
}

#[derive(Debug, Clone)]
pub struct AnimationDefinition {
    pub length_in_seconds: f32,
    pub looping: bool,
    pub bone_animations: HashMap<&'static str, Vec<AnimationChannel>>,
}

impl AnimationDefinition {
    pub fn new(
        length_in_seconds: f32,
        looping: bool,
        bone_animations: HashMap<&'static str, Vec<AnimationChannel>>,
    ) -> Self {
        Self {
            length_in_seconds,
            looping,
            bone_animations,
        }
    }

    pub fn apply_walk(
        &self,
        tree: &mut JavaModelPartTree,
        animation_pos: f32,
        animation_speed: f32,
        speed_factor: f32,
        scale_factor: f32,
    ) {
        let elapsed_millis = animation_pos * 50.0 * speed_factor;
        let target_scale = (animation_speed * scale_factor).min(1.0);
        apply_animation(tree, self, elapsed_millis, target_scale);
    }

    pub fn apply_state(
        &self,
        tree: &mut JavaModelPartTree,
        state: &AnimationState,
        age_in_ticks: f32,
    ) {
        if let Some(elapsed_millis) = state.get_time_in_millis(age_in_ticks) {
            apply_animation(tree, self, elapsed_millis, 1.0);
        }
    }
}

pub fn apply_animation(
    tree: &mut JavaModelPartTree,
    definition: &AnimationDefinition,
    elapsed_millis: f32,
    target_scale: f32,
) {
    let mut seconds_since_start = elapsed_millis / 1000.0;
    if definition.looping {
        seconds_since_start %= definition.length_in_seconds;
    }

    for (bone_name, channels) in &definition.bone_animations {
        let Some(part) = tree.part_mut(bone_name) else {
            continue;
        };

        for channel in channels {
            let keyframes = &channel.keyframes;
            if keyframes.is_empty() {
                continue;
            }

            let mut prev_idx = 0;
            let mut found = false;
            for i in 0..keyframes.len() {
                if seconds_since_start <= keyframes[i].timestamp_sec {
                    prev_idx = i.saturating_sub(1);
                    found = true;
                    break;
                }
            }
            if !found {
                prev_idx = keyframes.len() - 1;
            }

            let next_idx = (prev_idx + 1).min(keyframes.len() - 1);
            let prev_frame = &keyframes[prev_idx];
            let next_frame = &keyframes[next_idx];
            let time_delta = seconds_since_start - prev_frame.timestamp_sec;

            let lerp_alpha = if next_idx != prev_idx {
                (time_delta / (next_frame.timestamp_sec - prev_frame.timestamp_sec)).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let interpolated = match next_frame.interpolation {
                AnimationInterpolation::Linear => {
                    let p0 = prev_frame.post_target;
                    let p1 = next_frame.pre_target;
                    p0.lerp(p1, lerp_alpha) * target_scale
                }
                AnimationInterpolation::CatmullRom => {
                    let p0 = keyframes[prev_idx.saturating_sub(1)].post_target;
                    let p1 = prev_frame.post_target;
                    let p2 = next_frame.post_target;
                    let p3 = keyframes[(next_idx + 1).min(keyframes.len() - 1)].post_target;
                    Vec3::new(
                        catmullrom(lerp_alpha, p0.x, p1.x, p2.x, p3.x) * target_scale,
                        catmullrom(lerp_alpha, p0.y, p1.y, p2.y, p3.y) * target_scale,
                        catmullrom(lerp_alpha, p0.z, p1.z, p2.z, p3.z) * target_scale,
                    )
                }
            };

            match channel.target {
                AnimationTarget::Position => part.offset_pos(interpolated),
                AnimationTarget::Rotation => part.offset_rotation(interpolated),
                AnimationTarget::Scale => part.offset_scale(interpolated),
            }
        }
    }
}

fn catmullrom(alpha: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
    let alpha_sq = alpha * alpha;
    let alpha_cub = alpha_sq * alpha;
    0.5 * ((2.0 * p1)
        + (p2 - p0) * alpha
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * alpha_sq
        + (3.0 * p1 - p0 - 3.0 * p2 + p3) * alpha_cub)
}

pub fn pos_vec(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, -y, z)
}

pub fn degree_vec(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(
        x * (std::f32::consts::PI / 180.0),
        y * (std::f32::consts::PI / 180.0),
        z * (std::f32::consts::PI / 180.0),
    )
}

pub fn scale_vec(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x - 1.0, y - 1.0, z - 1.0)
}
