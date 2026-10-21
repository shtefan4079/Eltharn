#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CameraType {
    #[default]
    FirstPerson,
    ThirdPersonBack,
    ThirdPersonFront,
}
impl CameraType {
    pub const VALUES: [Self; 3] = [
        Self::FirstPerson,
        Self::ThirdPersonBack,
        Self::ThirdPersonFront,
    ];
    pub fn cycle(self) -> Self {
        Self::VALUES[(self.ordinal() + 1) % Self::VALUES.len()]
    }
    pub const fn ordinal(self) -> usize {
        match self {
            Self::FirstPerson => 0,
            Self::ThirdPersonBack => 1,
            Self::ThirdPersonFront => 2,
        }
    }
    pub const fn is_first_person(self) -> bool {
        matches!(self, Self::FirstPerson)
    }
    pub const fn is_mirrored(self) -> bool {
        matches!(self, Self::ThirdPersonFront)
    }
    pub const fn is_detached(self) -> bool {
        !self.is_first_person()
    }
    pub fn view_rotation(self, player_yaw: f32, player_pitch: f32) -> (f32, f32) {
        if self.is_mirrored() {
            (player_yaw + std::f32::consts::PI, -player_pitch)
        } else {
            (player_yaw, player_pitch)
        }
    }
}
