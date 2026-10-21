use crate::gameplay::camera::Camera;
use glam::{Mat4, Vec3};
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct CameraRenderState {
    pub pos: Vec3,
    pub x_rot: f32,
    pub y_rot: f32,
    pub initialized: bool,
    pub is_panoramic_mode: bool,
    pub hud_fov: f32,
    pub depth_far: f32,
    pub projection_matrix: Mat4,
    pub view_rotation_matrix: Mat4,
}
impl CameraRenderState {
    pub fn extract(camera: &Camera) -> Self {
        let view_rotation_matrix = camera.render_view_rotation_matrix();
        let depth_far = camera.depth_far();
        let projection_matrix = camera.projection_matrix();
        Self {
            pos: camera.position,
            x_rot: camera.pitch,
            y_rot: camera.yaw,
            initialized: camera.is_initialized(),
            is_panoramic_mode: camera.is_panoramic_mode(),
            hud_fov: camera.hud_fov(),
            depth_far,
            projection_matrix,
            view_rotation_matrix,
        }
    }
}
