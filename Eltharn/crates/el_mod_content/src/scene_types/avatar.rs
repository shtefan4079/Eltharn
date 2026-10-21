use serde::{Deserialize, Serialize};
pub const CAPE_ROWS: usize = 17;
pub const CAPE_OFFSET_PIXEL_MILLI_MAX: i32 = 8000;
pub type CapeRows = [[i32; 3]; CAPE_ROWS];
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalAvatarMotion {
    pub player_yaw_radians: f64,
    pub head_pitch_radians: f64,
    pub is_under_water: bool,
    pub is_visually_swimming: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvatarPose {
    Standing,
    Crouching,
    Swimming,
    FallFlying,
    SpinAttack,
    Sleeping,
    Dying,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalAvatarContext {
    pub version: u32,
    pub handle: String,
    pub world: String,
    pub epoch: String,
    pub position_previous: [f64; 3],
    pub position: [f64; 3],
    pub velocity_previous: [f64; 3],
    pub velocity: [f64; 3],
    pub body_yaw_previous_radians: f64,
    pub body_yaw_radians: f64,
    pub head_yaw_previous_radians: f64,
    pub head_yaw_radians: f64,
    pub pose: AvatarPose,
    pub on_ground: bool,
    pub is_crouching: bool,
    pub is_fall_flying: bool,
    pub cloak_previous: [f64; 3],
    pub cloak: [f64; 3],
    pub bob_previous: f64,
    pub bob: f64,
    pub walk_distance_previous: f64,
    pub walk_distance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion: Option<LocalAvatarMotion>,
}
pub fn local_handle(world: u64, epoch: u64) -> String {
    format!("local/{world}/{epoch}")
}
impl LocalAvatarContext {
    pub fn validate(&self) -> Result<(), String> {
        if self
            .motion
            .as_ref()
            .is_some_and(|m| !m.player_yaw_radians.is_finite() || !m.head_pitch_radians.is_finite())
        {
            return Err("invalid local avatar motion".into());
        }
        let identity = |s: &str| {
            s.parse::<u64>()
                .ok()
                .filter(|n| *n > 0 && n.to_string() == s)
        };
        let (Some(world), Some(epoch)) = (identity(&self.world), identity(&self.epoch)) else {
            return Err("invalid local avatar identity".into());
        };
        if self.version != 1 || self.handle != local_handle(world, epoch) {
            return Err("invalid local avatar handle/version".into());
        }
        for vector in [
            self.position_previous,
            self.position,
            self.velocity_previous,
            self.velocity,
            self.cloak_previous,
            self.cloak,
        ] {
            if vector
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 32_000_000.0)
            {
                return Err("invalid local avatar vector".into());
            }
        }
        if [
            self.body_yaw_previous_radians,
            self.body_yaw_radians,
            self.head_yaw_previous_radians,
            self.head_yaw_radians,
            self.bob_previous,
            self.bob,
            self.walk_distance_previous,
            self.walk_distance,
        ]
        .iter()
        .any(|v| !v.is_finite())
        {
            return Err("invalid local avatar scalar".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapeDeformation {
    pub version: u32,
    pub handle: String,
    pub offsets_pixel_milli: CapeRows,
}
impl CapeDeformation {
    pub fn identity(&self) -> Option<(u64, u64)> {
        let mut parts = self.handle.split('/');
        if parts.next()? != "local" {
            return None;
        }
        let parse = |s: &str| {
            s.parse::<u64>()
                .ok()
                .filter(|n| *n > 0 && n.to_string() == s)
        };
        let identity = (parse(parts.next()?)?, parse(parts.next()?)?);
        if parts.next().is_some() {
            return None;
        }
        Some(identity)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.identity().is_none()
            || self.offsets_pixel_milli[0] != [0; 3]
            || self
                .offsets_pixel_milli
                .iter()
                .flatten()
                .any(|v| v.unsigned_abs() > CAPE_OFFSET_PIXEL_MILLI_MAX as u32)
        {
            return Err("invalid local cape deformation".into());
        }
        Ok(())
    }
    pub fn is_neutral(&self) -> bool {
        self.offsets_pixel_milli.iter().flatten().all(|v| *v == 0)
    }
}
