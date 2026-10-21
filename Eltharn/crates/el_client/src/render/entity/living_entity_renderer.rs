use glam::{Mat4, Vec3};
const EYE_BED_OFFSET: f32 = 0.1;
const MAX_RIDING_HEAD_DIFF: f32 = 85.0;
const BODY_CATCH_UP_HEAD_DIFF: f32 = 50.0;
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Down,
    Up,
    North,
    South,
    West,
    East,
}
impl Direction {
    pub const fn step_x(self) -> i32 {
        match self {
            Self::West => -1,
            Self::East => 1,
            _ => 0,
        }
    }
    pub const fn step_z(self) -> i32 {
        match self {
            Self::North => -1,
            Self::South => 1,
            _ => 0,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityPose {
    Standing,
    Sleeping,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LivingEntityVisibility {
    pub invisible: bool,
    pub invisible_to_player: bool,
    pub glowing: bool,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LivingEntityRotationState {
    pub body_rot: f32,
    pub x_rot: f32,
    pub age_in_ticks: f32,
    pub death_time: f32,
    pub bounding_box_height: f32,
    pub pose: EntityPose,
    pub bed_orientation: Option<Direction>,
    pub is_auto_spin_attack: bool,
    pub is_upside_down: bool,
    pub is_fully_frozen: bool,
}
impl Default for LivingEntityRotationState {
    fn default() -> Self {
        Self {
            body_rot: 0.0,
            x_rot: 0.0,
            age_in_ticks: 0.0,
            death_time: 0.0,
            bounding_box_height: 1.8,
            pose: EntityPose::Standing,
            bed_orientation: None,
            is_auto_spin_attack: false,
            is_upside_down: false,
            is_fully_frozen: false,
        }
    }
}
pub struct LivingEntityRenderer;
#[allow(dead_code)]
impl LivingEntityRenderer {
    pub const fn should_submit_body(visibility: LivingEntityVisibility) -> bool {
        !visibility.invisible || !visibility.invisible_to_player || visibility.glowing
    }
    pub const fn should_force_translucent(visibility: LivingEntityVisibility) -> bool {
        visibility.invisible && !visibility.invisible_to_player
    }
    pub const fn should_render_layers(_visibility: LivingEntityVisibility) -> bool {
        true
    }
    pub const fn is_body_visible(visibility: LivingEntityVisibility) -> bool {
        !visibility.invisible
    }
    pub const fn sleep_direction_to_rotation(direction: Direction) -> f32 {
        match direction {
            Direction::South => 90.0,
            Direction::West => 0.0,
            Direction::North => 270.0,
            Direction::East => 180.0,
            Direction::Down | Direction::Up => 0.0,
        }
    }
    pub fn is_upside_down_name(name: &str) -> bool {
        name == "Dinnerbone" || name == "Grumm"
    }
    pub fn sleeping_eye_offset(eye_height: f32, bed_orientation: Direction) -> Vec3 {
        let head_offset = eye_height - EYE_BED_OFFSET;
        Vec3::new(
            -(bed_orientation.step_x() as f32) * head_offset,
            0.0,
            -(bed_orientation.step_z() as f32) * head_offset,
        )
    }
    pub fn solve_body_rot(
        head_rot: f32,
        body_rot: f32,
        riding_body_rot: Option<f32>,
        partial_ticks: f32,
        old_body_rot: f32,
    ) -> f32 {
        if let Some(riding_body_rot) = riding_body_rot {
            let mut solved_body_rot =
                rot_lerp_degrees(partial_ticks, old_body_rot, riding_body_rot);
            let head_diff = wrap_degrees(head_rot - solved_body_rot)
                .clamp(-MAX_RIDING_HEAD_DIFF, MAX_RIDING_HEAD_DIFF);
            solved_body_rot = head_rot - head_diff;
            if head_diff.abs() > BODY_CATCH_UP_HEAD_DIFF {
                solved_body_rot += head_diff * 0.2;
            }
            solved_body_rot
        } else {
            rot_lerp_degrees(partial_ticks, old_body_rot, body_rot)
        }
    }
    pub fn setup_rotations_matrix(
        state: LivingEntityRotationState,
        entity_scale: f32,
        flip_degrees: f32,
    ) -> Mat4 {
        let mut body_rot = state.body_rot;
        if state.is_fully_frozen {
            body_rot += (state.age_in_ticks.floor() * 3.25).cos() * std::f32::consts::PI * 0.4;
        }
        let mut matrix = Mat4::IDENTITY;
        if state.pose != EntityPose::Sleeping {
            matrix *= Mat4::from_rotation_y((180.0 - body_rot).to_radians());
        }
        if state.death_time > 0.0 {
            let fall = (((state.death_time - 1.0) / 20.0) * 1.6).sqrt().min(1.0);
            matrix *= Mat4::from_rotation_z((fall * flip_degrees).to_radians());
        } else if state.is_auto_spin_attack {
            matrix *= Mat4::from_rotation_x((-90.0 - state.x_rot).to_radians());
            matrix *= Mat4::from_rotation_y((state.age_in_ticks * -75.0).to_radians());
        } else if state.pose == EntityPose::Sleeping {
            let angle = state
                .bed_orientation
                .map(Self::sleep_direction_to_rotation)
                .unwrap_or(body_rot);
            matrix *= Mat4::from_rotation_y(angle.to_radians());
            matrix *= Mat4::from_rotation_z(flip_degrees.to_radians());
            matrix *= Mat4::from_rotation_y(270.0_f32.to_radians());
        } else if state.is_upside_down {
            matrix *= Mat4::from_translation(Vec3::new(
                0.0,
                (state.bounding_box_height + EYE_BED_OFFSET) / entity_scale.max(f32::EPSILON),
                0.0,
            ));
            matrix *= Mat4::from_rotation_z(180.0_f32.to_radians());
        }
        matrix
    }
    pub fn overlay_coords(red_overlay: bool, white_overlay_progress: f32) -> (i32, i32) {
        (
            overlay_u(white_overlay_progress),
            if red_overlay { 3 } else { 10 },
        )
    }
}
fn overlay_u(white_overlay_progress: f32) -> i32 {
    (white_overlay_progress.clamp(0.0, 1.0) * 15.0).round() as i32
}
fn rot_lerp_degrees(partial_ticks: f32, old: f32, new: f32) -> f32 {
    old + partial_ticks.clamp(0.0, 1.0) * wrap_degrees(new - old)
}
fn wrap_degrees(value: f32) -> f32 {
    (value + 180.0).rem_euclid(360.0) - 180.0
}
