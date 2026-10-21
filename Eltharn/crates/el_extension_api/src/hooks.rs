use crate::services::{
    CLIENT_AVATAR_HOOK_CAPABILITY, CLIENT_CAMERA_HOOK_CAPABILITY, CLIENT_INPUT_HOOK_CAPABILITY,
    CLIENT_RENDER_HOOK_CAPABILITY, CLIENT_TOOLTIP_HOOK_CAPABILITY,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
pub const HOOK_VERSION: u32 = 1;
pub const MAX_HOOK_BINDINGS: usize = 16;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HookPoint {
    #[serde(rename = "ui.item.tooltip")]
    ItemTooltip,
    #[serde(rename = "ui.item.tooltip.layout")]
    ItemTooltipLayout,
    #[serde(rename = "camera.transform")]
    CameraTransform,
    #[serde(rename = "camera.hurt")]
    CameraHurt,
    #[serde(rename = "hud.health.blink")]
    HudHealthBlink,
    #[serde(rename = "avatar.cape.geometry")]
    CapeGeometry,
    #[serde(rename = "hud.visibility")]
    HudVisibility,
    #[serde(rename = "hand.visibility")]
    HandVisibility,
    #[serde(rename = "ui.world.visibility")]
    WorldUiVisibility,
    #[serde(rename = "ui.menu.visibility")]
    MenuUiVisibility,
    #[serde(rename = "input.action")]
    InputAction,
}
impl HookPoint {
    pub const fn name(self) -> &'static str {
        match self {
            Self::ItemTooltip => "ui.item.tooltip",
            Self::ItemTooltipLayout => "ui.item.tooltip.layout",
            Self::CameraTransform => "camera.transform",
            Self::CameraHurt => "camera.hurt",
            Self::HudHealthBlink => "hud.health.blink",
            Self::CapeGeometry => "avatar.cape.geometry",
            Self::HudVisibility => "hud.visibility",
            Self::HandVisibility => "hand.visibility",
            Self::WorldUiVisibility => "ui.world.visibility",
            Self::MenuUiVisibility => "ui.menu.visibility",
            Self::InputAction => "input.action",
        }
    }
    pub const fn capability(self) -> &'static str {
        match self {
            Self::ItemTooltip | Self::ItemTooltipLayout => CLIENT_TOOLTIP_HOOK_CAPABILITY,
            Self::CameraTransform | Self::CameraHurt => CLIENT_CAMERA_HOOK_CAPABILITY,
            Self::CapeGeometry => CLIENT_AVATAR_HOOK_CAPABILITY,
            Self::HudHealthBlink
            | Self::HudVisibility
            | Self::HandVisibility
            | Self::WorldUiVisibility
            | Self::MenuUiVisibility => CLIENT_RENDER_HOOK_CAPABILITY,
            Self::InputAction => CLIENT_INPUT_HOOK_CAPABILITY,
        }
    }
    pub const fn allows(self, mode: HookMode) -> bool {
        match self {
            Self::ItemTooltipLayout => matches!(mode, HookMode::Modify),
            Self::InputAction => {
                matches!(mode, HookMode::Before | HookMode::After | HookMode::Cancel)
            }
            _ => true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookMode {
    Before,
    After,
    Modify,
    Cancel,
    Replace,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookBinding {
    pub point: HookPoint,
    pub mode: HookMode,
    pub function: String,
    #[serde(default)]
    pub priority: i16,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookModule {
    pub version: u32,
    pub script: String,
    pub bindings: Vec<HookBinding>,
}
impl HookModule {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != HOOK_VERSION {
            return Err("Unsupported source hook version".into());
        }
        if self.script.is_empty()
            || self.script.len() > 240
            || !self.script.ends_with(".rhai")
            || !self
                .script
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b))
            || self
                .script
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == "..")
        {
            return Err("Hook source must name a packaged .rhai source".into());
        }
        if self.bindings.is_empty() || self.bindings.len() > MAX_HOOK_BINDINGS {
            return Err("Hook binding count must be 1..16".into());
        }
        let mut seen = BTreeSet::new();
        for binding in &self.bindings {
            let mut bytes = binding.function.bytes();
            if binding.function.len() > 64
                || !bytes
                    .next()
                    .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                || !bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
            {
                return Err("Invalid hook function name".into());
            }
            if !(-1000..=1000).contains(&binding.priority) {
                return Err("Hook priority must be -1000..1000".into());
            }
            if !binding.point.allows(binding.mode) {
                return Err(format!(
                    "Unsupported {:?} at {}",
                    binding.mode,
                    binding.point.name()
                ));
            }
            if !seen.insert((binding.point, binding.mode, binding.function.as_str())) {
                return Err("Duplicate hook binding".into());
            }
        }
        Ok(())
    }
}
