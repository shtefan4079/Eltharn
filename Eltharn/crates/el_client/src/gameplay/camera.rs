#![allow(dead_code)]
pub use crate::gameplay::camera_type::CameraType;
use crate::world::client_level::ClientLevel;
use el_core::CollisionContext;
use el_core::fluid_state::FluidState;
use el_math::{Vec3 as MathVec3, block_pos::BlockPos};
use glam::{DVec3, EulerRot, Mat4, Quat, Vec3};
use std::f32::consts::FRAC_PI_2;
pub const PROJECTION_Z_NEAR: f32 = 0.05;
pub const BASE_HUD_FOV: f32 = 70.0;
fn java_hurt_axis_rotation(degrees: f32, around_y: bool) -> Mat4 {
    let half_angle = degrees.to_radians() * 0.5;
    let sine = f64::from(half_angle).sin() as f32;
    let cosine = java_joml_cos_from_sin(sine, half_angle);
    let quaternion = if around_y {
        Quat::from_xyzw(0.0, sine, 0.0, cosine)
    } else {
        Quat::from_xyzw(0.0, 0.0, sine, cosine)
    };
    Mat4::from_quat(quaternion)
}
fn java_joml_cos_from_sin(sine: f32, angle: f32) -> f32 {
    let magnitude = (1.0 - sine * sine).sqrt();
    let shifted = angle + FRAC_PI_2;
    let mut wrapped =
        shifted - ((shifted / std::f32::consts::TAU) as i32 as f32) * std::f32::consts::TAU;
    if wrapped < 0.0 {
        wrapped += std::f32::consts::TAU;
    }
    if wrapped >= std::f32::consts::PI {
        -magnitude
    } else {
        magnitude
    }
}
#[inline]
pub(crate) fn java_reverse_z_perspective_rh(
    fov_y_radians: f32,
    aspect_ratio: f32,
    z_near: f32,
    z_far: f32,
) -> Mat4 {
    Mat4::perspective_rh(fov_y_radians, aspect_ratio, z_far, z_near)
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FogType {
    #[default]
    None,
    Water,
    Lava,
    PowderSnow,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearPlane {
    pub forward: Vec3,
    pub left: Vec3,
    pub up: Vec3,
}
impl NearPlane {
    pub fn top_left(self) -> Vec3 {
        self.forward + self.up + self.left
    }
    pub fn top_right(self) -> Vec3 {
        self.forward + self.up - self.left
    }
    pub fn bottom_left(self) -> Vec3 {
        self.forward - self.up + self.left
    }
    pub fn bottom_right(self) -> Vec3 {
        self.forward - self.up - self.left
    }
    pub fn point_on_plane(self, x: f32, y: f32) -> Vec3 {
        self.forward + self.up * y - self.left * x
    }
}
#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    pub sensitivity: f64,
    initialized: bool,
    detached: bool,
    eye_height: f32,
    eye_height_old: f32,
    projection_width: f32,
    projection_height: f32,
    projection_z_near: f32,
    depth_far: f32,
    hud_fov: f32,
    fov_modifier: f32,
    old_fov_modifier: f32,
    death_tilt: f32,
    hurt_tilt: f32,
    hurt_dir: f32,
    mod_render_rotation: Mat4,
    mod_render_fov_scale: f32,
    panoramic_mode: bool,
    panoramic_forwards: Vec3,
}
impl Camera {
    pub fn new<V: Into<Vec3>>(position: V, yaw: f32, pitch: f32) -> Self {
        Self {
            position: position.into(),
            yaw,
            pitch,
            fov: 70.0,
            sensitivity: 0.5,
            initialized: false,
            detached: false,
            eye_height: 0.0,
            eye_height_old: 0.0,
            projection_width: 1.0,
            projection_height: 1.0,
            projection_z_near: PROJECTION_Z_NEAR,
            depth_far: 1000.0,
            hud_fov: BASE_HUD_FOV,
            fov_modifier: 1.0,
            old_fov_modifier: 1.0,
            death_tilt: 0.0,
            hurt_tilt: 0.0,
            hurt_dir: 0.0,
            mod_render_rotation: Mat4::IDENTITY,
            mod_render_fov_scale: 1.0,
            panoramic_mode: false,
            panoramic_forwards: Vec3::Z,
        }
    }
    pub fn tick(&mut self, target_eye_height: Option<f32>, target_fov_modifier: f32) {
        if let Some(target_eye_height) = target_eye_height {
            self.eye_height_old = self.eye_height;
            self.eye_height += (target_eye_height - self.eye_height) * 0.5;
        }
        self.tick_fov(target_fov_modifier);
    }
    pub fn tick_fov(&mut self, target_fov_modifier: f32) {
        self.old_fov_modifier = self.fov_modifier;
        self.fov_modifier += (target_fov_modifier - self.fov_modifier) * 0.5;
        self.fov_modifier = self.fov_modifier.clamp(0.1, 1.5);
    }
    pub fn update_projection(
        &mut self,
        partial_tick: f32,
        base_fov: f32,
        fov_effect_scale: f32,
        death_time: Option<f32>,
        hurt: Option<(f32, f32, f32)>,
        damage_tilt_strength: f64,
        fluid: FogType,
        width: f32,
        height: f32,
        depth_far: f32,
    ) {
        self.mod_render_rotation = Mat4::IDENTITY;
        self.mod_render_fov_scale = 1.0;
        self.projection_width = width.max(1.0);
        self.projection_height = height.max(1.0);
        self.depth_far = depth_far;
        self.fov = self.calculate_fov(partial_tick, base_fov, fov_effect_scale, death_time, fluid);
        self.hud_fov = self.modify_fov_for_death_or_fluid(
            partial_tick,
            BASE_HUD_FOV,
            fov_effect_scale,
            death_time,
            fluid,
        );
        if let Some(death_time) = death_time {
            let duration = (death_time + partial_tick).min(20.0);
            self.death_tilt = 40.0 - 8000.0 / (duration + 200.0);
        } else {
            self.death_tilt = 0.0;
        }
        if let Some((hurt_time, hurt_duration, hurt_dir)) = hurt {
            let mut hurt = hurt_time - partial_tick;
            if hurt < 0.0 || hurt_duration <= 0.0 {
                self.hurt_tilt = 0.0;
                self.hurt_dir = 0.0;
            } else {
                hurt /= hurt_duration;
                hurt = el_math::mth::sin(hurt * hurt * hurt * hurt * std::f32::consts::PI);
                self.hurt_tilt = (-f64::from(hurt) * 14.0 * damage_tilt_strength) as f32;
                self.hurt_dir = hurt_dir;
            }
        } else {
            self.hurt_tilt = 0.0;
            self.hurt_dir = 0.0;
        }
        self.initialized = true;
    }
    pub(crate) fn hurt_render_angles(&self) -> [f32; 3] {
        [self.death_tilt, self.hurt_tilt, self.hurt_dir]
    }
    pub(crate) fn set_hurt_render_angles(
        &mut self,
        enabled: bool,
        angles_degrees: [f32; 3],
    ) -> bool {
        if !enabled {
            self.death_tilt = 0.0;
            self.hurt_tilt = 0.0;
            self.hurt_dir = 0.0;
            return true;
        }
        if !angles_degrees.into_iter().all(f32::is_finite) {
            return false;
        }
        [self.death_tilt, self.hurt_tilt, self.hurt_dir] = angles_degrees;
        true
    }
    pub fn calculate_fov(
        &self,
        partial_tick: f32,
        base_fov: f32,
        fov_effect_scale: f32,
        death_time: Option<f32>,
        fluid: FogType,
    ) -> f32 {
        if self.panoramic_mode {
            return 90.0;
        }
        let modifier =
            self.old_fov_modifier + partial_tick * (self.fov_modifier - self.old_fov_modifier);
        self.modify_fov_for_death_or_fluid(
            partial_tick,
            base_fov * modifier,
            fov_effect_scale,
            death_time,
            fluid,
        )
    }
    pub fn modify_fov_for_death_or_fluid(
        &self,
        partial_tick: f32,
        mut fov: f32,
        fov_effect_scale: f32,
        death_time: Option<f32>,
        fluid: FogType,
    ) -> f32 {
        if let Some(death_time) = death_time {
            let duration = (death_time + partial_tick).min(20.0);
            fov /= (1.0 - 500.0 / (duration + 500.0)) * 2.0 + 1.0;
        }
        if matches!(fluid, FogType::Lava | FogType::Water) {
            fov *= 1.0 + fov_effect_scale * (0.857_142_87 - 1.0);
        }
        fov
    }
    pub fn build_view_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        let view = self.view_rotation_matrix() * Mat4::from_translation(-self.position);
        let proj = java_reverse_z_perspective_rh(
            self.render_fov().to_radians(),
            aspect_ratio,
            PROJECTION_Z_NEAR,
            self.depth_far,
        );
        proj * self.bob_hurt_matrix() * view
    }
    pub fn build_outline_view_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        const VIEW_SHRINK: f32 = 1.0 - 1.0 / 256.0;
        const PERSPECTIVE_LAYER_SCALE: f32 = 1.0 - 1.0 / 4096.0;
        let view_rotation = self.view_rotation_matrix();
        let projection = java_reverse_z_perspective_rh(
            self.render_fov().to_radians(),
            aspect_ratio,
            PROJECTION_Z_NEAR,
            self.depth_far,
        );
        projection
            * self.bob_hurt_matrix()
            * Mat4::from_scale(Vec3::splat(VIEW_SHRINK))
            * view_rotation
            * Mat4::from_scale(Vec3::splat(PERSPECTIVE_LAYER_SCALE))
    }
    pub fn build_culling_view_projection_matrix(&self, aspect_ratio: f32, base_fov: f32) -> Mat4 {
        let view = self.render_view_rotation_matrix() * Mat4::from_translation(-self.position);
        let proj = Mat4::perspective_rh(
            self.render_fov().max(base_fov).to_radians(),
            aspect_ratio,
            PROJECTION_Z_NEAR,
            self.depth_far,
        );
        proj * view
    }
    pub fn forward(&self) -> Vec3 {
        self.rotation() * -Vec3::Z
    }
    pub fn right(&self) -> Vec3 {
        self.rotation() * Vec3::X
    }
    pub fn left(&self) -> Vec3 {
        self.rotation() * -Vec3::X
    }
    pub fn up(&self) -> Vec3 {
        self.rotation() * Vec3::Y
    }
    pub fn rotation(&self) -> Quat {
        Quat::from_euler(
            EulerRot::YXZ,
            std::f32::consts::PI - self.yaw,
            -self.pitch,
            0.0,
        )
    }
    pub(crate) fn render_rotation(&self) -> Quat {
        if self.mod_render_rotation == Mat4::IDENTITY {
            self.rotation()
        } else {
            Quat::from_mat4(&self.render_view_rotation_matrix()).conjugate()
        }
    }
    pub(crate) fn render_right(&self) -> Vec3 {
        self.render_rotation() * Vec3::X
    }
    pub(crate) fn render_up(&self) -> Vec3 {
        self.render_rotation() * Vec3::Y
    }
    pub(crate) fn render_forward(&self) -> Vec3 {
        self.render_rotation() * -Vec3::Z
    }
    pub(crate) fn render_yaw_pitch(&self) -> (f32, f32) {
        if self.mod_render_rotation == Mat4::IDENTITY {
            (self.yaw, self.pitch)
        } else {
            let (y, x, _) = self.render_rotation().to_euler(EulerRot::YXZ);
            (std::f32::consts::PI - y, -x)
        }
    }
    pub fn view_rotation_matrix(&self) -> Mat4 {
        Mat4::from_quat(self.rotation().conjugate())
    }
    pub(crate) fn render_view_rotation_matrix(&self) -> Mat4 {
        if self.mod_render_rotation == Mat4::IDENTITY {
            self.view_rotation_matrix()
        } else {
            self.mod_render_rotation * self.view_rotation_matrix()
        }
    }
    fn bob_hurt_matrix(&self) -> Mat4 {
        let mut matrix = Mat4::IDENTITY;
        if self.death_tilt != 0.0 {
            matrix = matrix * java_hurt_axis_rotation(self.death_tilt, false);
        }
        if self.hurt_tilt != 0.0 || self.hurt_dir != 0.0 {
            matrix = matrix
                * java_hurt_axis_rotation(-self.hurt_dir, true)
                * java_hurt_axis_rotation(self.hurt_tilt, false)
                * java_hurt_axis_rotation(self.hurt_dir, true);
        }
        if self.mod_render_rotation == Mat4::IDENTITY {
            matrix
        } else {
            matrix * self.mod_render_rotation
        }
    }
    pub(crate) fn set_mod_render_transform(&mut self, radians: [f32; 3], fov_scale_milli: u16) {
        self.mod_render_rotation = if radians == [0.0; 3] {
            Mat4::IDENTITY
        } else {
            Mat4::from_rotation_y(radians[0])
                * Mat4::from_rotation_x(radians[1])
                * Mat4::from_rotation_z(radians[2])
        };
        self.mod_render_fov_scale = if fov_scale_milli == 1000 {
            1.0
        } else {
            f32::from(fov_scale_milli) / 1000.0
        };
    }
    fn render_fov(&self) -> f32 {
        if self.mod_render_fov_scale == 1.0 {
            self.fov
        } else {
            (self.fov * self.mod_render_fov_scale).clamp(1.0, 179.0)
        }
    }
    pub fn render_view_rotation_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        let proj = java_reverse_z_perspective_rh(
            self.render_fov().to_radians(),
            aspect_ratio,
            PROJECTION_Z_NEAR,
            self.depth_far,
        );
        proj * self.bob_hurt_matrix() * self.view_rotation_matrix()
    }
    pub fn view_rotation_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        let proj = java_reverse_z_perspective_rh(
            self.render_fov().to_radians(),
            aspect_ratio,
            PROJECTION_Z_NEAR,
            self.depth_far,
        );
        proj * self.render_view_rotation_matrix()
    }
    pub fn projection_matrix(&self) -> Mat4 {
        java_reverse_z_perspective_rh(
            self.render_fov().to_radians(),
            self.projection_width / self.projection_height,
            self.projection_z_near,
            self.depth_far,
        )
    }
    pub fn move_relative(&mut self, forwards: f32, up: f32, right: f32) {
        self.position += self.forward() * forwards + self.up() * up + self.right() * right;
    }
    pub fn set_rotation(&mut self, yaw: f32, pitch: f32) {
        self.yaw = yaw;
        self.pitch = pitch;
    }
    pub fn set_position<V: Into<Vec3>>(&mut self, position: V) {
        self.position = position.into();
    }
    pub fn block_position(&self) -> BlockPos {
        BlockPos::new(
            self.position.x.floor() as i32,
            self.position.y.floor() as i32,
            self.position.z.floor() as i32,
        )
    }
    pub fn wrapped_yaw_degrees(&self) -> f32 {
        let degrees = self.yaw.to_degrees();
        (degrees + 180.0).rem_euclid(360.0) - 180.0
    }
    pub const fn is_initialized(&self) -> bool {
        self.initialized
    }
    pub const fn is_detached(&self) -> bool {
        self.detached
    }
    pub fn set_detached(&mut self, detached: bool) {
        self.detached = detached;
    }
    pub const fn is_panoramic_mode(&self) -> bool {
        self.panoramic_mode
    }
    pub const fn hud_fov(&self) -> f32 {
        self.hud_fov
    }
    pub const fn depth_far(&self) -> f32 {
        self.depth_far
    }
    pub const fn eye_height(&self, partial_tick: f32) -> f32 {
        self.eye_height_old + (self.eye_height - self.eye_height_old) * partial_tick
    }
    pub fn near_plane(&self, fov: f32) -> NearPlane {
        let aspect_ratio = self.projection_width / self.projection_height;
        let plane_height = (fov.to_radians() / 2.0).tan() * self.projection_z_near;
        let plane_width = plane_height * aspect_ratio;
        NearPlane {
            forward: self.forward() * self.projection_z_near,
            left: self.left() * plane_width,
            up: self.up() * plane_height,
        }
    }
    pub fn fluid_in_camera(&self, level: &ClientLevel, fov: f32) -> FogType {
        if !self.initialized {
            return FogType::None;
        }
        let current_pos = self.block_position();
        let current = level.get_block_state(current_pos);
        let current_fluid = current.get_fluid_state();
        if current_fluid.is_water()
            && self.position.y
                < current_pos.y as f32 + Self::fluid_height(level, current_pos, current_fluid)
        {
            return FogType::Water;
        }
        let plane = self.near_plane(fov);
        for point in [
            plane.forward,
            plane.top_left(),
            plane.top_right(),
            plane.bottom_left(),
            plane.bottom_right(),
        ] {
            let pos = self.position + point;
            let check_pos = BlockPos::new(
                pos.x.floor() as i32,
                pos.y.floor() as i32,
                pos.z.floor() as i32,
            );
            let state = level.get_block_state(check_pos);
            let fluid = state.get_fluid_state();
            if fluid.is_lava() {
                if pos.y <= check_pos.y as f32 + Self::fluid_height(level, check_pos, fluid) {
                    return FogType::Lava;
                }
            } else if state.block.path == "powder_snow" {
                return FogType::PowderSnow;
            }
        }
        FogType::None
    }
    fn fluid_height(level: &ClientLevel, pos: BlockPos, fluid: FluidState) -> f32 {
        let above = level.get_block_state(pos.above()).get_fluid_state();
        Self::fluid_height_from_states(fluid, above)
    }
    fn fluid_height_from_states(fluid: FluidState, above: FluidState) -> f32 {
        fluid.height_with_same_fluid_above(above)
    }
    pub fn enable_panoramic_mode(&mut self) {
        self.panoramic_mode = true;
        self.panoramic_forwards = self.forward();
    }
    pub fn disable_panoramic_mode(&mut self) {
        self.panoramic_mode = false;
    }
    pub const fn panoramic_forwards(&self) -> Vec3 {
        self.panoramic_forwards
    }
    pub fn reset(&mut self) {
        self.initialized = false;
        self.detached = false;
    }
    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        let yaw_delta = vanilla_mouse_delta_radians(self.sensitivity, mouse_dx);
        let pitch_delta = vanilla_mouse_delta_radians(self.sensitivity, mouse_dy);
        self.yaw += yaw_delta;
        self.pitch += pitch_delta;
        const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;
        self.pitch = self.pitch.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);
    }
    pub fn process_keyboard(&mut self, forward: f32, right: f32, up: f32, speed: f32) {
        let forward_dir = self.forward();
        let flat_forward = Vec3::new(forward_dir.x, 0.0, forward_dir.z).normalize_or_zero();
        let flat_right = self.right();
        self.position += flat_forward * forward * speed;
        self.position += flat_right * right * speed;
        self.position.y += up * speed;
    }
}
pub fn third_person_camera_position(eye: Vec3, forward: Vec3, distance: f32) -> Vec3 {
    eye - forward.normalize_or_zero() * distance
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraEntitySettings {
    pub scale: f32,
    pub distance: f32,
}
impl Default for CameraEntitySettings {
    fn default() -> Self {
        Self {
            scale: 1.0,
            distance: crate::gameplay::attributes::CAMERA_DISTANCE.default_value as f32,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum CameraSleepingState {
    #[default]
    NotSleeping,
    Sleeping {
        bed_yaw_degrees: Option<f32>,
    },
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraSetupSettings {
    pub entity: CameraEntitySettings,
    pub mount: Option<CameraEntitySettings>,
    pub sleeping: CameraSleepingState,
}
pub fn setup_camera_from_entity(
    camera_type: CameraType,
    eye: Vec3,
    player_yaw: f32,
    player_pitch: f32,
    level: Option<&ClientLevel>,
) -> (Vec3, f32, f32) {
    setup_camera_from_entity_with_settings(
        camera_type,
        eye,
        player_yaw,
        player_pitch,
        level,
        CameraSetupSettings::default(),
    )
}
pub fn setup_camera_from_entity_with_settings(
    camera_type: CameraType,
    eye: Vec3,
    player_yaw: f32,
    player_pitch: f32,
    level: Option<&ClientLevel>,
    settings: CameraSetupSettings,
) -> (Vec3, f32, f32) {
    let collision_context = CollisionContext::empty();
    setup_camera_from_entity_with_settings_and_collision_context(
        camera_type,
        eye,
        player_yaw,
        player_pitch,
        level,
        settings,
        &collision_context,
    )
}
pub fn setup_camera_from_entity_with_settings_and_collision_context(
    camera_type: CameraType,
    eye: Vec3,
    player_yaw: f32,
    player_pitch: f32,
    level: Option<&ClientLevel>,
    settings: CameraSetupSettings,
    collision_context: &CollisionContext,
) -> (Vec3, f32, f32) {
    let (mut yaw, mut pitch) = camera_type.view_rotation(player_yaw, player_pitch);
    if !camera_type.is_detached() {
        if let CameraSleepingState::Sleeping { bed_yaw_degrees } = settings.sleeping {
            yaw = bed_yaw_degrees
                .map(|bed_yaw_degrees| (bed_yaw_degrees - 180.0).to_radians())
                .unwrap_or(0.0);
            pitch = 0.0;
            return (eye + Vec3::Y * 0.15, yaw, pitch);
        }
        return (eye, yaw, pitch);
    }
    let camera = Camera::new(eye, yaw, pitch);
    let mount = settings.mount.unwrap_or(settings.entity);
    let max_distance =
        (settings.entity.scale * settings.entity.distance).max(mount.scale * mount.distance);
    let distance = level
        .map(|level| {
            clipped_third_person_camera_distance_with_collision_context(
                level,
                eye,
                camera.forward(),
                max_distance,
                collision_context,
            )
        })
        .unwrap_or(max_distance);
    (
        third_person_camera_position(eye, camera.forward(), distance),
        yaw,
        pitch,
    )
}
pub fn setup_camera_from_entity_with_settings_and_collision_context_d(
    camera_type: CameraType,
    eye: DVec3,
    player_yaw: f32,
    player_pitch: f32,
    level: Option<&ClientLevel>,
    settings: CameraSetupSettings,
    collision_context: &CollisionContext,
) -> (DVec3, f32, f32) {
    let (mut yaw, mut pitch) = camera_type.view_rotation(player_yaw, player_pitch);
    if !camera_type.is_detached() {
        if let CameraSleepingState::Sleeping { bed_yaw_degrees } = settings.sleeping {
            yaw = bed_yaw_degrees
                .map(|bed_yaw_degrees| (bed_yaw_degrees - 180.0).to_radians())
                .unwrap_or(0.0);
            pitch = 0.0;
            return (eye + DVec3::Y * f64::from(0.15_f32), yaw, pitch);
        }
        return (eye, yaw, pitch);
    }
    let camera = Camera::new(Vec3::ZERO, yaw, pitch);
    let mount = settings.mount.unwrap_or(settings.entity);
    let max_distance =
        (settings.entity.scale * settings.entity.distance).max(mount.scale * mount.distance);
    let distance = level
        .map(|level| {
            clipped_third_person_camera_distance_with_collision_context_d(
                level,
                eye,
                camera.forward(),
                max_distance,
                collision_context,
            )
        })
        .unwrap_or(max_distance);
    let local_offset = third_person_camera_position(Vec3::ZERO, camera.forward(), distance);
    (eye + local_offset.as_dvec3(), yaw, pitch)
}
pub fn clipped_third_person_camera_distance(
    level: &ClientLevel,
    eye: Vec3,
    forward: Vec3,
    max_distance: f32,
) -> f32 {
    let collision_context = CollisionContext::empty();
    clipped_third_person_camera_distance_with_collision_context(
        level,
        eye,
        forward,
        max_distance,
        &collision_context,
    )
}
pub fn clipped_third_person_camera_distance_with_collision_context(
    level: &ClientLevel,
    eye: Vec3,
    forward: Vec3,
    max_distance: f32,
    collision_context: &CollisionContext,
) -> f32 {
    let eye = MathVec3::new(f64::from(eye.x), f64::from(eye.y), f64::from(eye.z));
    let forward = MathVec3::new(
        f64::from(forward.x),
        f64::from(forward.y),
        f64::from(forward.z),
    );
    let mut camera_distance = max_distance;
    for i in 0..8 {
        let offset_x = (((i & 1) * 2) as f32 - 1.0) * 0.1_f32;
        let offset_y = ((((i >> 1) & 1) * 2) as f32 - 1.0) * 0.1_f32;
        let offset_z = ((((i >> 2) & 1) * 2) as f32 - 1.0) * 0.1_f32;
        let from = eye.add(
            f64::from(offset_x),
            f64::from(offset_y),
            f64::from(offset_z),
        );
        let to = from.add(
            -forward.x * f64::from(camera_distance),
            -forward.y * f64::from(camera_distance),
            -forward.z * f64::from(camera_distance),
        );
        if let Some(hit) = clip_visual_block(level, from, to, collision_context) {
            let dist_sq = eye.distance_to_sq(&hit) as f32;
            if dist_sq < camera_distance * camera_distance {
                camera_distance = dist_sq.sqrt();
            }
        }
    }
    camera_distance
}
pub fn clipped_third_person_camera_distance_with_collision_context_d(
    level: &ClientLevel,
    eye: DVec3,
    forward: Vec3,
    max_distance: f32,
    collision_context: &CollisionContext,
) -> f32 {
    let eye = MathVec3::new(eye.x, eye.y, eye.z);
    let forward = MathVec3::new(
        f64::from(forward.x),
        f64::from(forward.y),
        f64::from(forward.z),
    );
    let mut camera_distance = max_distance;
    for i in 0..8 {
        let offset_x = (((i & 1) * 2) as f32 - 1.0) * 0.1_f32;
        let offset_y = ((((i >> 1) & 1) * 2) as f32 - 1.0) * 0.1_f32;
        let offset_z = ((((i >> 2) & 1) * 2) as f32 - 1.0) * 0.1_f32;
        let from = eye.add(
            f64::from(offset_x),
            f64::from(offset_y),
            f64::from(offset_z),
        );
        let to = from.add(
            -forward.x * f64::from(camera_distance),
            -forward.y * f64::from(camera_distance),
            -forward.z * f64::from(camera_distance),
        );
        if let Some(hit) = clip_visual_block(level, from, to, collision_context) {
            let dist_sq = eye.distance_to_sq(&hit) as f32;
            if dist_sq < camera_distance * camera_distance {
                camera_distance = dist_sq.sqrt();
            }
        }
    }
    camera_distance
}
fn clip_visual_block(
    level: &ClientLevel,
    from: MathVec3,
    to: MathVec3,
    collision_context: &CollisionContext,
) -> Option<MathVec3> {
    if from == to {
        return None;
    }
    const TRAVERSE_EPSILON: f64 = -1.0e-7;
    let to_x = java_lerp(TRAVERSE_EPSILON, to.x, from.x);
    let to_y = java_lerp(TRAVERSE_EPSILON, to.y, from.y);
    let to_z = java_lerp(TRAVERSE_EPSILON, to.z, from.z);
    let from_x = java_lerp(TRAVERSE_EPSILON, from.x, to.x);
    let from_y = java_lerp(TRAVERSE_EPSILON, from.y, to.y);
    let from_z = java_lerp(TRAVERSE_EPSILON, from.z, to.z);
    let mut current_x = from_x.floor() as i32;
    let mut current_y = from_y.floor() as i32;
    let mut current_z = from_z.floor() as i32;
    if let Some(hit) = clip_visual_block_at(
        level,
        BlockPos::new(current_x, current_y, current_z),
        from,
        to,
        collision_context,
    ) {
        return Some(hit);
    }
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    let dz = to_z - from_z;
    let sign_x = java_sign(dx);
    let sign_y = java_sign(dy);
    let sign_z = java_sign(dz);
    let t_delta_x = if sign_x == 0 {
        f64::MAX
    } else {
        f64::from(sign_x) / dx
    };
    let t_delta_y = if sign_y == 0 {
        f64::MAX
    } else {
        f64::from(sign_y) / dy
    };
    let t_delta_z = if sign_z == 0 {
        f64::MAX
    } else {
        f64::from(sign_z) / dz
    };
    let mut t_x = t_delta_x
        * if sign_x > 0 {
            1.0 - java_frac(from_x)
        } else {
            java_frac(from_x)
        };
    let mut t_y = t_delta_y
        * if sign_y > 0 {
            1.0 - java_frac(from_y)
        } else {
            java_frac(from_y)
        };
    let mut t_z = t_delta_z
        * if sign_z > 0 {
            1.0 - java_frac(from_z)
        } else {
            java_frac(from_z)
        };
    while t_x <= 1.0 || t_y <= 1.0 || t_z <= 1.0 {
        if t_x < t_y {
            if t_x < t_z {
                current_x = current_x.wrapping_add(sign_x);
                t_x += t_delta_x;
            } else {
                current_z = current_z.wrapping_add(sign_z);
                t_z += t_delta_z;
            }
        } else if t_y < t_z {
            current_y = current_y.wrapping_add(sign_y);
            t_y += t_delta_y;
        } else {
            current_z = current_z.wrapping_add(sign_z);
            t_z += t_delta_z;
        }
        if let Some(hit) = clip_visual_block_at(
            level,
            BlockPos::new(current_x, current_y, current_z),
            from,
            to,
            collision_context,
        ) {
            return Some(hit);
        }
    }
    None
}
fn clip_visual_block_at(
    level: &ClientLevel,
    pos: BlockPos,
    from: MathVec3,
    to: MathVec3,
    collision_context: &CollisionContext,
) -> Option<MathVec3> {
    level
        .get_block_state(pos)
        .get_visual_shape_with_context(pos, collision_context)
        .clip_location(from, to, pos)
}
#[inline]
fn java_lerp(delta: f64, start: f64, end: f64) -> f64 {
    start + delta * (end - start)
}
#[inline]
fn java_frac(value: f64) -> f64 {
    value - value.floor()
}
#[inline]
fn java_sign(value: f64) -> i32 {
    if value > 0.0 {
        1
    } else if value < 0.0 {
        -1
    } else {
        0
    }
}
pub fn vanilla_mouse_delta_radians(sensitivity: f64, delta: f64) -> f32 {
    let ss = sensitivity * f64::from(0.6_f32) + f64::from(0.2_f32);
    let sensitivity_mod = ss * ss * ss;
    let xo = delta * sensitivity_mod * 8.0;
    (xo as f32 * 0.15_f32).to_radians()
}
