use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
pub const TOUCH_CONTROL_PROFILE_FILE: &str = "touch-controls.json";
pub const TOUCH_CONTROL_PROFILE_VERSION: u32 = 1;
pub const MAX_TOUCH_PROFILE_BYTES: usize = 16 * 1024;
pub const MAX_TOUCH_BUTTONS: usize = 16;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TouchPreset {
    #[default]
    Classic,
    Compact,
    Disabled,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TouchHandedness {
    #[default]
    Right,
    Left,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TouchAction {
    Forward,
    Back,
    Left,
    Right,
    Attack,
    Use,
    Jump,
    Sneak,
    Sprint,
    Inventory,
    SwapHands,
    Drop,
    Chat,
    Pause,
    DebugInfo,
}
impl TouchAction {
    pub fn glyph_segments(self) -> &'static [[f32; 4]] {
        match self {
            Self::Forward | Self::Jump => &[
                [-0.65, 0.15, 0.0, -0.5],
                [0.0, -0.5, 0.65, 0.15],
                [0.0, -0.5, 0.0, 0.65],
            ],
            Self::Back | Self::Sneak | Self::Drop => &[
                [-0.65, -0.15, 0.0, 0.5],
                [0.0, 0.5, 0.65, -0.15],
                [0.0, -0.65, 0.0, 0.5],
            ],
            Self::Left => &[
                [0.1, -0.65, -0.55, 0.0],
                [-0.55, 0.0, 0.1, 0.65],
                [-0.55, 0.0, 0.65, 0.0],
            ],
            Self::Right => &[
                [-0.1, -0.65, 0.55, 0.0],
                [0.55, 0.0, -0.1, 0.65],
                [-0.65, 0.0, 0.55, 0.0],
            ],
            Self::Attack => &[
                [-0.6, 0.6, 0.55, -0.55],
                [0.1, -0.6, 0.6, -0.6],
                [0.6, -0.6, 0.6, -0.1],
                [-0.6, -0.05, 0.05, 0.6],
                [-0.75, 0.45, -0.45, 0.75],
            ],
            Self::Use => &[
                [-0.65, -0.65, 0.1, -0.65],
                [-0.65, -0.65, -0.65, 0.65],
                [-0.65, 0.65, 0.1, 0.65],
                [-0.05, 0.0, 0.7, 0.0],
                [0.3, -0.4, 0.7, 0.0],
                [0.7, 0.0, 0.3, 0.4],
            ],
            Self::Sprint => &[
                [-0.65, -0.05, 0.0, -0.65],
                [0.0, -0.65, 0.65, -0.05],
                [-0.65, 0.6, 0.0, 0.0],
                [0.0, 0.0, 0.65, 0.6],
            ],
            Self::Inventory => &[
                [-0.65, -0.65, 0.65, -0.65],
                [0.65, -0.65, 0.65, 0.65],
                [0.65, 0.65, -0.65, 0.65],
                [-0.65, 0.65, -0.65, -0.65],
                [-0.65, 0.0, 0.65, 0.0],
                [0.0, -0.65, 0.0, 0.65],
            ],
            Self::SwapHands => &[
                [-0.7, -0.3, 0.6, -0.3],
                [0.2, -0.7, 0.6, -0.3],
                [0.6, -0.3, 0.2, 0.1],
                [0.7, 0.3, -0.6, 0.3],
                [-0.2, -0.1, -0.6, 0.3],
                [-0.6, 0.3, -0.2, 0.7],
            ],
            Self::Chat => &[
                [-0.65, -0.6, 0.65, -0.6],
                [0.65, -0.6, 0.65, 0.35],
                [0.65, 0.35, -0.1, 0.35],
                [-0.1, 0.35, -0.55, 0.7],
                [-0.55, 0.7, -0.55, 0.35],
                [-0.55, 0.35, -0.65, 0.35],
                [-0.65, 0.35, -0.65, -0.6],
            ],
            Self::Pause => &[[-0.3, -0.65, -0.3, 0.65], [0.3, -0.65, 0.3, 0.65]],
            Self::DebugInfo => &[
                [-0.6, 0.6, -0.6, 0.1],
                [0.0, 0.6, 0.0, -0.6],
                [0.6, 0.6, 0.6, -0.2],
            ],
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TouchPlacement {
    pub center: [f32; 2],
    pub radius: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TouchButtonPlacement {
    pub action: TouchAction,
    pub placement: TouchPlacement,
    pub visible: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TouchControlProfile {
    pub version: u32,
    pub enabled: bool,
    pub handedness: TouchHandedness,
    pub scale: f32,
    pub opacity: f32,
    pub dead_zone: f32,
    pub look_sensitivity: f32,
    pub joystick: TouchPlacement,
    pub buttons: Vec<TouchButtonPlacement>,
}
impl Default for TouchControlProfile {
    fn default() -> Self {
        Self::for_preset(TouchPreset::Classic)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TouchProfileError(pub String);
impl fmt::Display for TouchProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl std::error::Error for TouchProfileError {}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TouchCircle {
    pub center: [f32; 2],
    pub radius: f32,
}
impl TouchCircle {
    pub fn contains(self, point: [f32; 2]) -> bool {
        (point[0] - self.center[0]).hypot(point[1] - self.center[1]) <= self.radius
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedTouchButton {
    pub action: TouchAction,
    pub circle: TouchCircle,
    pub glyph: Vec<[f32; 4]>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedTouchLayout {
    pub enabled: bool,
    pub opacity: f32,
    pub joystick: TouchCircle,
    pub buttons: Vec<ResolvedTouchButton>,
}
impl TouchControlProfile {
    pub fn for_preset(preset: TouchPreset) -> Self {
        let button = |action, x, y, radius| TouchButtonPlacement {
            action,
            placement: TouchPlacement {
                center: [x, y],
                radius,
            },
            visible: true,
        };
        Self {
            version: TOUCH_CONTROL_PROFILE_VERSION,
            enabled: preset != TouchPreset::Disabled,
            handedness: TouchHandedness::Right,
            scale: if preset == TouchPreset::Compact {
                0.8
            } else {
                1.0
            },
            opacity: 0.72,
            dead_zone: 0.20,
            look_sensitivity: 1.0,
            joystick: TouchPlacement {
                center: [0.18, 0.73],
                radius: 0.15,
            },
            buttons: vec![
                button(TouchAction::Attack, 0.82, 0.69, 0.067),
                button(TouchAction::Use, 0.94, 0.52, 0.058),
                button(TouchAction::Jump, 0.72, 0.49, 0.058),
                button(TouchAction::Sneak, 0.94, 0.84, 0.052),
                button(TouchAction::Sprint, 0.66, 0.81, 0.052),
                button(TouchAction::Inventory, 0.40, 0.85, 0.045),
                button(TouchAction::SwapHands, 0.52, 0.85, 0.045),
                button(TouchAction::Chat, 0.07, 0.10, 0.042),
                button(TouchAction::Pause, 0.94, 0.10, 0.042),
            ],
        }
    }
    pub fn from_launcher_value(value: &str) -> Result<Self, TouchProfileError> {
        if value.len() > MAX_TOUCH_PROFILE_BYTES {
            return Err(TouchProfileError("Touch profile exceeds 16 KiB".into()));
        }
        let profile = match value.trim() {
            "classic" | "1" | "" => Self::for_preset(TouchPreset::Classic),
            "compact" | "2" => Self::for_preset(TouchPreset::Compact),
            "off" | "none" | "disabled" | "0" => Self::for_preset(TouchPreset::Disabled),
            json => serde_json::from_value(
                el_strict_json::strict_json(json.as_bytes(), MAX_TOUCH_PROFILE_BYTES)
                    .map_err(TouchProfileError)?,
            )
            .map_err(|error| TouchProfileError(format!("Invalid touch profile: {error}")))?,
        };
        profile.validate()?;
        Ok(profile)
    }
    pub fn validate(&self) -> Result<(), TouchProfileError> {
        let bounded = |name: &str, value: f32, min: f32, max: f32| {
            if value.is_finite() && (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(TouchProfileError(format!(
                    "{name} must be finite and between {min} and {max}"
                )))
            }
        };
        if self.version != TOUCH_CONTROL_PROFILE_VERSION {
            return Err(TouchProfileError(
                "Unsupported touch profile version".into(),
            ));
        }
        bounded("scale", self.scale, 0.6, 1.5)?;
        bounded("opacity", self.opacity, 0.15, 1.0)?;
        bounded("dead_zone", self.dead_zone, 0.05, 0.6)?;
        bounded("look_sensitivity", self.look_sensitivity, 0.25, 3.0)?;
        let placement = |value: TouchPlacement| {
            bounded("center.x", value.center[0], 0.0, 1.0)?;
            bounded("center.y", value.center[1], 0.0, 1.0)?;
            bounded("radius", value.radius, 0.025, 0.25)
        };
        placement(self.joystick)?;
        if self.buttons.len() > MAX_TOUCH_BUTTONS {
            return Err(TouchProfileError(
                "Touch profile has too many buttons".into(),
            ));
        }
        let mut actions = BTreeSet::new();
        for button in &self.buttons {
            placement(button.placement)?;
            if !actions.insert(button.action) {
                return Err(TouchProfileError(
                    "Touch profile contains duplicate actions".into(),
                ));
            }
        }
        Ok(())
    }
    pub fn resolve(
        &self,
        width: f32,
        height: f32,
    ) -> Result<ResolvedTouchLayout, TouchProfileError> {
        self.resolve_safe_viewport(width, height, [0.0; 4])
    }
    pub fn resolve_safe_viewport(
        &self,
        width: f32,
        height: f32,
        insets: [f32; 4],
    ) -> Result<ResolvedTouchLayout, TouchProfileError> {
        self.validate()?;
        if !width.is_finite()
            || !height.is_finite()
            || width <= 1.0
            || height <= 1.0
            || insets
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(TouchProfileError("Invalid touch viewport".into()));
        }
        let usable_width = width - insets[0] - insets[2];
        let usable_height = height - insets[1] - insets[3];
        if usable_width <= 1.0 || usable_height <= 1.0 {
            return Err(TouchProfileError("Touch viewport has no safe area".into()));
        }
        let side = usable_width.min(usable_height);
        let circle = |placement: TouchPlacement| {
            let x = if self.handedness == TouchHandedness::Left {
                1.0 - placement.center[0]
            } else {
                placement.center[0]
            };
            let radius = (placement.radius * side * self.scale).min(side * 0.45);
            TouchCircle {
                center: [
                    insets[0] + (x * usable_width).clamp(radius, usable_width - radius),
                    insets[1]
                        + (placement.center[1] * usable_height)
                            .clamp(radius, usable_height - radius),
                ],
                radius,
            }
        };
        Ok(ResolvedTouchLayout {
            enabled: self.enabled,
            opacity: self.opacity,
            joystick: circle(self.joystick),
            buttons: self
                .buttons
                .iter()
                .filter(|button| button.visible)
                .map(|button| ResolvedTouchButton {
                    action: button.action,
                    circle: circle(button.placement),
                    glyph: button.action.glyph_segments().to_vec(),
                })
                .collect(),
        })
    }
}
