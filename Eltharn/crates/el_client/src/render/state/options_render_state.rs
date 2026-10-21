use crate::gameplay::camera_type::CameraType;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptionsRenderState {
    pub camera_type: CameraType,
    pub bob_view: bool,
    pub fov_degrees: f32,
    pub render_distance_chunks: u32,
}

impl Default for OptionsRenderState {
    fn default() -> Self {
        Self {
            camera_type: CameraType::FirstPerson,
            bob_view: true,
            fov_degrees: 70.0,
            render_distance_chunks: 12,
        }
    }
}
