use crate::gameplay::camera_type::CameraType;
use crate::input::VanillaKeyMappings;

#[derive(Debug, Clone)]
pub struct Options {
    pub key_mappings: VanillaKeyMappings,
    camera_type: CameraType,
    pub smooth_camera: bool,
    pub sensitivity: f64,
    pub invert_mouse_x: bool,
    pub invert_mouse_y: bool,
    pub discrete_mouse_scroll: bool,
    pub mouse_wheel_sensitivity: f32,
    pub raw_mouse_input: bool,
    pub fov_degrees: f32,
    pub fov_effect_scale: f32,
    pub screen_effect_scale: f32,
    pub damage_tilt_strength: f64,
    pub bob_view: bool,
    pub render_distance_chunks: u32,
    pub server_render_distance_chunks: u32,
    pub cloud_range_chunks: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            key_mappings: VanillaKeyMappings::new(),
            camera_type: CameraType::FirstPerson,
            smooth_camera: false,
            sensitivity: 0.5,
            invert_mouse_x: false,
            invert_mouse_y: false,
            discrete_mouse_scroll: false,
            mouse_wheel_sensitivity: 1.0,
            raw_mouse_input: true,
            fov_degrees: 70.0,
            fov_effect_scale: 1.0,
            screen_effect_scale: 1.0,
            damage_tilt_strength: 1.0,
            bob_view: true,
            render_distance_chunks: 12,
            server_render_distance_chunks: 0,
            cloud_range_chunks: 128,
        }
    }
}

impl Options {
    pub const fn camera_type(&self) -> CameraType {
        self.camera_type
    }

    pub fn set_camera_type(&mut self, camera_type: CameraType) {
        self.camera_type = camera_type;
    }

    pub fn effective_render_distance(&self) -> u32 {
        if self.server_render_distance_chunks == 0 {
            self.render_distance_chunks
        } else {
            self.render_distance_chunks
                .min(self.server_render_distance_chunks)
        }
    }

    pub fn set_server_render_distance(&mut self, distance: u32) {
        self.server_render_distance_chunks = distance;
    }
}
