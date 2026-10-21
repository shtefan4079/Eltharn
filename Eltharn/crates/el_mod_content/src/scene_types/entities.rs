use serde::{Deserialize, Serialize};

pub const MAX_ENTITIES: usize = 128;
pub const MAX_PAGE: usize = 8;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityObservation {
    pub id: i32,
    pub kind: String,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub health: f32,
    pub invisible: bool,
    pub crouching: bool,
    pub alive: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntitySnapshot {
    pub version: u32,
    pub epoch: String,
    pub sequence: String,
    pub total_loaded: usize,
    pub truncated: bool,
    pub entities: Vec<EntityObservation>,
}
impl EntitySnapshot {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.entities.len() > MAX_ENTITIES
            || self.total_loaded < self.entities.len()
            || self.truncated != (self.total_loaded > self.entities.len())
            || [&self.epoch, &self.sequence].iter().any(|s| {
                s.parse::<u64>()
                    .ok()
                    .is_none_or(|n| n == 0 || n.to_string() != s.as_str())
            })
        {
            return Err("Invalid entity snapshot identity/bounds".into());
        }
        let mut previous = None;
        for entity in &self.entities {
            if previous.is_some_and(|id| id >= entity.id)
                || entity.kind.parse::<crate::ContentId>().is_err()
                || entity.kind.len() > 128
                || entity
                    .position
                    .iter()
                    .any(|v| !v.is_finite() || v.abs() > 32_000_000.0)
                || entity.velocity.iter().any(|v| !v.is_finite())
                || !entity.yaw_degrees.is_finite()
                || !entity.pitch_degrees.is_finite()
                || !entity.health.is_finite()
            {
                return Err("Invalid entity observation".into());
            }
            previous = Some(entity.id);
        }
        Ok(())
    }
}
