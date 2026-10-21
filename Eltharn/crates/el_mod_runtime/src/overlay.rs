use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const CAPABILITY: &str = el_extension_api::services::CLIENT_OVERLAY_WRITE_CAPABILITY;
pub const WRITE_OPCODE: i32 = el_extension_api::services::CLIENT_OVERLAY_WRITE_OPCODE;
pub const MAX_REQUEST_BYTES: usize = 4096;
pub const MAX_SEGMENTS: usize = 16;
pub const MAX_SEGMENT_BYTES: usize = 256;
pub const MAX_TOTAL_BYTES: usize = 1024;
pub const COLORS: [&str; 16] = [
    "black",
    "dark_blue",
    "dark_green",
    "dark_aqua",
    "dark_red",
    "dark_purple",
    "gold",
    "gray",
    "dark_gray",
    "blue",
    "green",
    "aqua",
    "red",
    "light_purple",
    "yellow",
    "white",
];
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Segment {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underlined: bool,
    #[serde(default)]
    pub strikethrough: bool,
    #[serde(default)]
    pub obfuscated: bool,
}
pub fn validate(segments: &[Segment]) -> Result<(), String> {
    if segments.is_empty() || segments.len() > MAX_SEGMENTS {
        return Err("Overlay segment count limit".into());
    }
    let mut total = 0usize;
    for segment in segments {
        if segment.text.len() > MAX_SEGMENT_BYTES || segment.text.chars().any(char::is_control) {
            return Err("Overlay segment text limit".into());
        }
        if let Some(color) = &segment.color {
            if !COLORS.contains(&color.as_str()) {
                return Err("Unsupported overlay color".into());
            }
        }
        total += segment.text.len();
    }
    if total == 0 || total > MAX_TOTAL_BYTES {
        return Err("Overlay message byte limit".into());
    }
    Ok(())
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub sequence: u64,
    pub epoch: u64,
    pub segments: Vec<Segment>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub messages: BTreeMap<String, Record>,
}
#[derive(Clone, Default)]
pub(crate) struct State {
    report: Report,
    sequence: u64,
}
impl State {
    pub fn publish(&mut self, owner: &str, epoch: u64, segments: &[Segment]) -> Result<(), String> {
        validate(segments)?;
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or("Overlay message sequence overflow")?;
        self.report.messages.insert(
            owner.into(),
            Record {
                sequence: self.sequence,
                epoch,
                segments: segments.to_vec(),
            },
        );
        Ok(())
    }
    pub fn report(&self) -> Report {
        self.report.clone()
    }
    pub fn clear(&mut self) {
        self.report.messages.clear();
    }
    pub fn remove(&mut self, owner: &str) {
        self.report.messages.remove(owner);
    }
}
