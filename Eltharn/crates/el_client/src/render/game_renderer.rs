use crate::gameplay::camera::{Camera, setup_camera_from_entity};
use crate::gameplay::camera_type::CameraType;
use crate::render::state::level::camera_render_state::CameraRenderState;
use crate::world::client_level::ClientLevel;
use glam::Vec3;

#[allow(dead_code)]
pub struct GameRenderer {
    main_camera: Camera,
}

#[allow(dead_code)]
impl GameRenderer {
    pub fn new(main_camera: Camera) -> Self {
        Self { main_camera }
    }

    pub fn main_camera(&self) -> &Camera {
        &self.main_camera
    }

    pub fn main_camera_mut(&mut self) -> &mut Camera {
        &mut self.main_camera
    }

    pub fn align_camera_with_entity(
        &mut self,
        camera_type: CameraType,
        eye: Vec3,
        entity_yaw: f32,
        entity_pitch: f32,
        level: Option<&ClientLevel>,
    ) {
        let (position, yaw, pitch) =
            setup_camera_from_entity(camera_type, eye, entity_yaw, entity_pitch, level);
        self.main_camera.position = position;
        self.main_camera.yaw = yaw;
        self.main_camera.pitch = pitch;
        self.main_camera.set_detached(camera_type.is_detached());
    }

    pub fn extract_camera_state(&self) -> CameraRenderState {
        CameraRenderState::extract(&self.main_camera)
    }
}
