#![allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TranslucencyPointOfView {
    pub x: i8,
    pub y: i8,
    pub z: i8,
}
impl TranslucencyPointOfView {
    pub fn of(camera: glam::Vec3, section_x: i32, section_y: i32, section_z: i32) -> Self {
        Self {
            x: coordinate(camera.x, section_x),
            y: coordinate(camera.y, section_y),
            z: coordinate(camera.z, section_z),
        }
    }
    pub const fn is_axis_aligned(self) -> bool {
        self.x == 0 || self.y == 0 || self.z == 0
    }
}
fn coordinate(camera: f32, section: i32) -> i8 {
    ((camera.floor() as i32 >> 4) - section).clamp(-1, 1) as i8
}
