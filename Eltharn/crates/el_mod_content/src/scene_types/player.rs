use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerContext {
    pub version: u32,
    pub world: String,
    pub epoch: String,
    pub dimension: Option<String>,
    pub position: [f64; 3],
    pub yaw_degrees: f64,
    pub pitch_degrees: f64,
}
impl PlayerContext {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || [&self.world, &self.epoch].iter().any(|s| {
                s.parse::<u64>()
                    .ok()
                    .is_none_or(|n| n == 0 || n.to_string() != s.as_str())
            })
            || self
                .position
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 32_000_000.0)
            || !self.yaw_degrees.is_finite()
            || !self.pitch_degrees.is_finite()
            || self
                .dimension
                .as_ref()
                .is_some_and(|d| d.len() > 256 || d.parse::<crate::ContentId>().is_err())
        {
            return Err("invalid local player context".into());
        }
        Ok(())
    }
}
