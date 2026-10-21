use crate::FontWidthMeasurer;
use crate::components::click_sound_sink;
use crate::components::{
    Button, DEFAULT_HEIGHT, GuiEventListener, Renderable, TabOrderedElement, WidgetClickSound,
};
use crate::framework::graphics::{GuiGraphicsExtractor, WidgetTooltipPosition};
use crate::framework::layouts::LayoutElement;
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
pub const HEADER_HEIGHT: i32 = 33;
pub const FOOTER_HEIGHT: i32 = 33;
pub const ROW_HEIGHT: i32 = 20;
pub const ROW_SPACING: i32 = 25;
pub const BIG_BUTTON_WIDTH: i32 = 310;
pub const SMALL_BUTTON_WIDTH: i32 = 150;
pub const SMALL_BUTTON_GAP: i32 = 10;
pub const DONE_BUTTON_WIDTH: i32 = 200;
pub const FOOTER_BUTTON_HEIGHT: i32 = 20;
pub const FOOTER_BUTTON_Y_OFFSET: i32 = 26;
#[derive(Clone, Debug)]
pub struct VideoSettingsOption {
    pub key: String,
    pub label: String,
    pub is_slider: bool,
    pub min_val: f32,
    pub max_val: f32,
    pub step: f32,
    pub value: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FullscreenVideoMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate_hz: u32,
    pub bit_depth: u16,
}
impl FullscreenVideoMode {
    pub fn serialize(self) -> String {
        format!(
            "{}x{}@{}:{}",
            self.width, self.height, self.refresh_rate_hz, self.bit_depth
        )
    }
    pub fn parse(value: &str) -> Option<Self> {
        let (size, timing) = value
            .trim()
            .split_once('@')
            .unwrap_or((value.trim(), "60:24"));
        let (width, height) = size.split_once('x')?;
        let (refresh, depth) = timing.split_once(':').unwrap_or((timing, "24"));
        let mode = Self {
            width: width.parse().ok()?,
            height: height.parse().ok()?,
            refresh_rate_hz: refresh.parse().ok()?,
            bit_depth: depth.parse().ok()?,
        };
        (mode.width > 0 && mode.height > 0).then_some(mode)
    }
    pub fn matches_display_mode(self, other: Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.refresh_rate_hz == other.refresh_rate_hz
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicsPreset {
    Fast,
    Fancy,
    Fabulous,
    Custom,
}
impl GraphicsPreset {
    pub const fn slider_value(self) -> f32 {
        match self {
            Self::Fast => 0.0,
            Self::Fancy => 1.0,
            Self::Fabulous => 2.0,
            Self::Custom => 3.0,
        }
    }
    pub const fn serialized_name(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Fancy => "fancy",
            Self::Fabulous => "fabulous",
            Self::Custom => "custom",
        }
    }
    pub const fn from_slider_value(value: f32) -> Self {
        match value as i32 {
            0 => Self::Fast,
            1 => Self::Fancy,
            2 => Self::Fabulous,
            _ => Self::Custom,
        }
    }
    pub fn from_serialized_name(value: &str) -> Option<Self> {
        match value {
            "fast" => Some(Self::Fast),
            "fancy" => Some(Self::Fancy),
            "fabulous" => Some(Self::Fabulous),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphicsPresetCapabilities {
    pub anisotropy_has_known_issues: Option<bool>,
}
impl GraphicsPresetCapabilities {
    pub const UNKNOWN: Self = Self {
        anisotropy_has_known_issues: None,
    };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicsPresetApplyResult {
    Applied,
    NeedsAnisotropyWorkaroundState,
}
fn set_option_value(options: &mut [VideoSettingsOption], key: &str, value: f32) {
    if let Some(option) = options.iter_mut().find(|option| option.key == key) {
        option.value = value.clamp(option.min_val, option.max_val);
    }
}
pub fn apply_java_graphics_preset(
    options: &mut [VideoSettingsOption],
    preset: GraphicsPreset,
    capabilities: GraphicsPresetCapabilities,
) -> GraphicsPresetApplyResult {
    let mut result = GraphicsPresetApplyResult::Applied;
    match preset {
        GraphicsPreset::Fast => {
            set_option_value(options, "biomeBlendRadius", 1.0);
            set_option_value(options, "renderDistance", 8.0);
            set_option_value(options, "prioritizeChunkUpdates", 0.0);
            set_option_value(options, "simulationDistance", 6.0);
            set_option_value(options, "ambientOcclusion", 0.0);
            set_option_value(options, "cloudStatus", 1.0);
            set_option_value(options, "particles", 1.0);
            set_option_value(options, "mipmapLevels", 2.0);
            set_option_value(options, "entityShadows", 0.0);
            set_option_value(options, "entityDistanceScaling", 0.75);
            set_option_value(options, "menuBackgroundBlurriness", 2.0);
            set_option_value(options, "cloudRange", 32.0);
            set_option_value(options, "cutoutLeaves", 0.0);
            set_option_value(options, "improvedTransparency", 0.0);
            set_option_value(options, "weatherRadius", 5.0);
            set_option_value(options, "maxAnisotropyBit", 1.0);
            set_option_value(options, "textureFiltering", 0.0);
        }
        GraphicsPreset::Fancy => {
            set_option_value(options, "biomeBlendRadius", 2.0);
            set_option_value(options, "renderDistance", 16.0);
            set_option_value(options, "prioritizeChunkUpdates", 1.0);
            set_option_value(options, "simulationDistance", 12.0);
            set_option_value(options, "ambientOcclusion", 1.0);
            set_option_value(options, "cloudStatus", 2.0);
            set_option_value(options, "particles", 0.0);
            set_option_value(options, "mipmapLevels", 4.0);
            set_option_value(options, "entityShadows", 1.0);
            set_option_value(options, "entityDistanceScaling", 1.0);
            set_option_value(options, "menuBackgroundBlurriness", 5.0);
            set_option_value(options, "cloudRange", 64.0);
            set_option_value(options, "cutoutLeaves", 1.0);
            set_option_value(options, "improvedTransparency", 0.0);
            set_option_value(options, "weatherRadius", 10.0);
            set_option_value(options, "maxAnisotropyBit", 1.0);
            set_option_value(options, "textureFiltering", 1.0);
        }
        GraphicsPreset::Fabulous => {
            set_option_value(options, "biomeBlendRadius", 2.0);
            set_option_value(options, "renderDistance", 32.0);
            set_option_value(options, "prioritizeChunkUpdates", 1.0);
            set_option_value(options, "simulationDistance", 12.0);
            set_option_value(options, "ambientOcclusion", 1.0);
            set_option_value(options, "cloudStatus", 2.0);
            set_option_value(options, "particles", 0.0);
            set_option_value(options, "mipmapLevels", 4.0);
            set_option_value(options, "entityShadows", 1.0);
            set_option_value(options, "entityDistanceScaling", 1.25);
            set_option_value(options, "menuBackgroundBlurriness", 5.0);
            set_option_value(options, "cloudRange", 128.0);
            set_option_value(options, "cutoutLeaves", 1.0);
            set_option_value(
                options,
                "improvedTransparency",
                if cfg!(target_os = "macos") { 0.0 } else { 1.0 },
            );
            set_option_value(options, "weatherRadius", 10.0);
            set_option_value(options, "maxAnisotropyBit", 2.0);
            match capabilities.anisotropy_has_known_issues {
                Some(true) => set_option_value(options, "textureFiltering", 1.0),
                Some(false) => set_option_value(options, "textureFiltering", 2.0),
                None => {
                    result = GraphicsPresetApplyResult::NeedsAnisotropyWorkaroundState;
                }
            }
        }
        GraphicsPreset::Custom => {}
    }
    result
}
fn is_graphics_preset_child_option(key: &str) -> bool {
    matches!(
        key,
        "biomeBlendRadius"
            | "renderDistance"
            | "prioritizeChunkUpdates"
            | "simulationDistance"
            | "ambientOcclusion"
            | "cloudStatus"
            | "particles"
            | "mipmapLevels"
            | "entityShadows"
            | "entityDistanceScaling"
            | "menuBackgroundBlurriness"
            | "cloudRange"
            | "cutoutLeaves"
            | "improvedTransparency"
            | "weatherRadius"
            | "maxAnisotropyBit"
            | "textureFiltering"
    )
}
pub fn graphics_preset_from_options(extra_options: &HashMap<String, String>) -> GraphicsPreset {
    extra_options
        .get("graphicsPreset")
        .map(|value| value.trim())
        .and_then(GraphicsPreset::from_serialized_name)
        .unwrap_or(GraphicsPreset::Fancy)
}
pub fn cloud_status_from_options(extra_options: &HashMap<String, String>) -> f32 {
    let raw = extra_options
        .get("renderClouds")
        .or_else(|| extra_options.get("cloudStatus"))
        .map(|value| value.trim());
    match raw {
        Some("false") => 0.0,
        Some("fast") => 1.0,
        Some("true") => 2.0,
        Some("0") | Some("0.0000") => 0.0,
        Some("1") | Some("1.0000") => 1.0,
        Some("2") | Some("2.0000") => 2.0,
        _ => 2.0,
    }
}
pub fn java_int_range_from_options(
    extra_options: &HashMap<String, String>,
    key: &str,
    min: i32,
    max: i32,
    default: i32,
) -> i32 {
    let Some(raw) = extra_options.get(key).map(|value| value.trim()) else {
        return default;
    };
    if let Ok(value) = raw.parse::<i32>() {
        return if (min..=max).contains(&value) {
            value
        } else {
            default
        };
    }
    if raw.contains('.') {
        if let Ok(value) = raw.parse::<f32>() {
            if value.is_finite() && (0.0..=1.0).contains(&value) {
                return (min as f32 + value * (max - min) as f32).round() as i32;
            }
        }
    }
    default
}
fn java_legacy_enum_id_from_options(
    extra_options: &HashMap<String, String>,
    key: &str,
    default: i32,
) -> i32 {
    let Some(raw) = extra_options.get(key).map(|value| value.trim()) else {
        return default;
    };
    if let Ok(value) = raw.parse::<i32>() {
        return if (0..=2).contains(&value) {
            value
        } else {
            default
        };
    }
    raw.parse::<f32>()
        .ok()
        .filter(|value| value.is_finite() && (0.0..=2.0).contains(value))
        .and_then(|value| {
            let rounded = value.round();
            ((value - rounded).abs() <= f32::EPSILON).then_some(rounded as i32)
        })
        .unwrap_or(default)
}
pub fn entity_distance_scaling_from_options(extra_options: &HashMap<String, String>) -> f32 {
    let Some(raw) = extra_options
        .get("entityDistanceScaling")
        .map(|value| value.trim())
    else {
        return 1.0;
    };
    let Ok(value) = raw.parse::<f32>() else {
        return 1.0;
    };
    if !value.is_finite() {
        return 1.0;
    }
    if (0.5..=5.0).contains(&value) {
        value
    } else if value >= 0.0 && value < 0.5 && raw.contains('.') {
        0.5 + value * 4.5
    } else {
        1.0
    }
}
pub fn framerate_limit_from_options(extra_options: &HashMap<String, String>) -> i32 {
    if extra_options.contains_key("maxFps") {
        return java_int_range_from_options(extra_options, "maxFps", 10, 260, 120);
    }
    if extra_options.contains_key("framerateLimit") {
        return java_int_range_from_options(extra_options, "framerateLimit", 10, 260, 120);
    }
    120
}
impl VideoSettingsOption {
    pub fn get_display_string(&self) -> String {
        self.get_display_string_with_modes(&[])
    }
    fn get_display_string_with_modes(
        &self,
        fullscreen_video_modes: &[FullscreenVideoMode],
    ) -> String {
        let name = crate::translate(&self.label);
        match self.key.as_str() {
            "enableVsync"
            | "fullscreen"
            | "exclusiveFullscreen"
            | "ambientOcclusion"
            | "entityShadows"
            | "cutoutLeaves"
            | "improvedTransparency"
            | "showAutosaveIndicator"
            | "vignette" => {
                let state = if self.value > 0.5 {
                    crate::translate("options.on")
                } else {
                    crate::translate("options.off")
                };
                format!("{}: {}", name, state)
            }
            "fullscreenOption" => {
                let Some(selected_mode) = (self.value >= 0.0)
                    .then(|| self.value.round() as usize)
                    .and_then(|index| fullscreen_video_modes.get(index))
                else {
                    if self.max_val < 0.0 {
                        return translate_or(
                            "options.fullscreen.unavailable",
                            "Setting unavailable",
                        );
                    }
                    return format!(
                        "{}: {}",
                        name,
                        translate_or("options.fullscreen.current", "Current")
                    );
                };
                let mut entry = translate_or("options.fullscreen.entry", "%sx%s@%s (%sbit)");
                for replacement in [
                    selected_mode.width.to_string(),
                    selected_mode.height.to_string(),
                    selected_mode.refresh_rate_hz.to_string(),
                    selected_mode.bit_depth.to_string(),
                ]
                .iter()
                {
                    entry = entry.replacen("%s", replacement, 1);
                }
                format!("{name}: {entry}")
            }
            "framerateLimit" => {
                let val = self.value.round() as i32;
                if val >= 260 {
                    format!(
                        "{}: {}",
                        name,
                        crate::translate("options.framerateLimit.max")
                    )
                } else {
                    let translation = crate::translate("options.framerate");
                    format!("{}: {}", name, translation.replace("%s", &val.to_string()))
                }
            }
            "inactivityFpsLimit" => {
                let val = self.value.round() as i32;
                let mode = match val {
                    0 => crate::translate("options.inactivityFpsLimit.minimized"),
                    1 => crate::translate("options.inactivityFpsLimit.afk"),
                    _ => "".to_string(),
                };
                format!("{}: {}", name, mode)
            }
            "guiScale" => {
                let val = self.value.round() as i32;
                if val == 0 {
                    format!("{}: {}", name, crate::translate("options.guiScale.auto"))
                } else {
                    format!("{}: {}", name, val)
                }
            }
            "gamma" => {
                let pct = (self.value * 100.0).round() as i32;
                if pct == 0 {
                    format!("{}: {}", name, crate::translate("options.gamma.min"))
                } else if pct == 100 {
                    format!("{}: {}", name, crate::translate("options.gamma.max"))
                } else {
                    format!("{}: {}%", name, pct)
                }
            }
            "preferredGraphicsBackend" => {
                let val = self.value.round() as i32;
                let mode = match val {
                    0 => crate::translate("options.graphicsApi.default"),
                    1 => crate::translate("options.graphicsApi.opengl"),
                    2 => crate::translate("options.graphicsApi.vulkan"),
                    _ => "".to_string(),
                };
                format!("{}: {}", name, mode)
            }
            "graphicsPreset" => {
                let val = self.value.round() as i32;
                let mode = match val {
                    0 => crate::translate("options.graphics.fast"),
                    1 => crate::translate("options.graphics.fancy"),
                    2 => crate::translate("options.graphics.fabulous"),
                    3 => crate::translate("options.graphics.custom"),
                    _ => "".to_string(),
                };
                format!("{}: {}", name, mode)
            }
            "biomeBlendRadius" => {
                let val = self.value.round() as i32;
                let dist = val * 2 + 1;
                let key_str = format!("options.biomeBlendRadius.{}", dist);
                format!("{}: {}", name, crate::translate(&key_str))
            }
            "renderDistance" => {
                let val = self.value.round() as i32;
                let translation = crate::translate("options.chunks");
                format!("{}: {}", name, translation.replace("%s", &val.to_string()))
            }
            "prioritizeChunkUpdates" => {
                let val = self.value.round() as i32;
                let mode = match val {
                    0 => crate::translate("options.prioritizeChunkUpdates.none"),
                    1 => crate::translate("options.prioritizeChunkUpdates.byPlayer"),
                    2 => crate::translate("options.prioritizeChunkUpdates.nearby"),
                    _ => "".to_string(),
                };
                format!("{}: {}", name, mode)
            }
            "simulationDistance" => {
                let val = self.value.round() as i32;
                let translation = crate::translate("options.chunks");
                format!("{}: {}", name, translation.replace("%s", &val.to_string()))
            }
            "cloudStatus" => {
                let val = self.value.round() as i32;
                let mode = match val {
                    0 => crate::translate("options.off"),
                    1 => crate::translate("options.clouds.fast"),
                    2 => crate::translate("options.clouds.fancy"),
                    _ => "".to_string(),
                };
                format!("{}: {}", name, mode)
            }
            "particles" => {
                let val = self.value.round() as i32;
                let mode = match val {
                    0 => crate::translate("options.particles.all"),
                    1 => crate::translate("options.particles.decreased"),
                    2 => crate::translate("options.particles.minimal"),
                    _ => "".to_string(),
                };
                format!("{}: {}", name, mode)
            }
            "mipmapLevels" => {
                let val = self.value.round() as i32;
                if val == 0 {
                    format!("{}: {}", name, crate::translate("options.off"))
                } else {
                    format!("{}: {}", name, val)
                }
            }
            "entityDistanceScaling" => {
                let pct = (self.value * 100.0).round() as i32;
                format!("{}: {}%", name, pct)
            }
            "menuBackgroundBlurriness" => {
                let val = self.value.round() as i32;
                if val == 0 {
                    format!("{}: {}", name, crate::translate("options.off"))
                } else {
                    format!("{}: {}", name, val)
                }
            }
            "cloudRange" => {
                let val = self.value.round() as i32;
                let translation = crate::translate("options.chunks");
                format!("{}: {}", name, translation.replace("%s", &val.to_string()))
            }
            "textureFiltering" => {
                let val = self.value.round() as i32;
                let mode = match val {
                    0 => crate::translate("options.textureFiltering.none"),
                    1 => crate::translate("options.textureFiltering.rgss"),
                    2 => crate::translate("options.textureFiltering.anisotropic"),
                    _ => "".to_string(),
                };
                format!("{}: {}", name, mode)
            }
            "maxAnisotropyBit" => {
                let val = self.value.round() as i32;
                if val == 0 {
                    format!("{}: {}", name, crate::translate("options.off"))
                } else {
                    let translation = crate::translate("options.multiplier");
                    format!(
                        "{}: {}",
                        name,
                        translation.replace("%s", &(1 << val).to_string())
                    )
                }
            }
            "weatherRadius" => {
                let val = self.value.round() as i32;
                let translation = crate::translate("options.blocks");
                format!("{}: {}", name, translation.replace("%s", &val.to_string()))
            }
            "attackIndicator" => {
                let val = self.value.round() as i32;
                let mode = match val {
                    0 => crate::translate("options.off"),
                    1 => crate::translate("options.attack.crosshair"),
                    2 => crate::translate("options.attack.hotbar"),
                    _ => "".to_string(),
                };
                format!("{}: {}", name, mode)
            }
            "chunkSectionFadeInTime" => format!("{}: {:.2}", name, self.value),
            _ => format!("{}: {}", name, self.value),
        }
    }
}
fn translate_or(key: &str, fallback: &str) -> String {
    let translated = crate::translate(key);
    if translated == key {
        fallback.to_string()
    } else {
        translated
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoSettingsAction {
    Done,
    GuiScaleChanged,
    OpenShaderPacks,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoSettingsSliderKey {
    Selection,
    Left,
    Right,
}
#[derive(Clone, Debug)]
pub enum VideoSettingsListItem {
    Header {
        label_key: &'static str,
    },
    Big {
        option_index: usize,
    },
    SmallPair {
        option_index_a: usize,
        option_index_b: Option<usize>,
    },
}
#[derive(Clone, Debug)]
pub struct LayoutItem {
    pub kind: VideoSettingsListItem,
    pub relative_y: i32,
    pub height: i32,
}
struct RowWidget {
    rect: ScreenRectangle,
    option_index: usize,
    is_slider: bool,
    pressed: bool,
    focused: bool,
}
impl RowWidget {
    fn new(rect: ScreenRectangle, option_index: usize, is_slider: bool) -> Self {
        Self {
            rect,
            option_index,
            is_slider,
            pressed: false,
            focused: false,
        }
    }
    fn takes_press(&mut self) -> bool {
        let pressed = self.pressed;
        self.pressed = false;
        pressed
    }
}
struct RowHandle {
    row: Rc<RefCell<RowWidget>>,
}
impl RowHandle {
    fn new(row: Rc<RefCell<RowWidget>>) -> Self {
        Self { row }
    }
    fn with_row<T>(&self, read: impl FnOnce(&RowWidget) -> T, fallback: T) -> T {
        match self.row.try_borrow() {
            Ok(row) => read(&row),
            Err(_) => fallback,
        }
    }
}
impl Renderable for RowHandle {
    fn extract_render_state(
        &mut self,
        _graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
    }
}
impl TabOrderedElement for RowHandle {
    fn tab_order_group(&self) -> i32 {
        0
    }
}
impl GuiEventListener for RowHandle {
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        match self.row.try_borrow() {
            Ok(row) => row.rect.contains_point(mouse_x as i32, mouse_y as i32),
            Err(_) => false,
        }
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        if let Ok(mut row) = self.row.try_borrow_mut() {
            if row.rect.contains_point(event.x as i32, event.y as i32) {
                row.pressed = true;
                return true;
            }
        }
        false
    }
    fn set_focused(&mut self, focused: bool) {
        if let Ok(mut row) = self.row.try_borrow_mut() {
            row.focused = focused;
        }
    }
    fn is_focused(&self) -> bool {
        self.with_row(|row| row.focused, false)
    }
    fn rectangle(&self) -> ScreenRectangle {
        match self.row.try_borrow() {
            Ok(row) => row.rect,
            Err(_) => ScreenRectangle::empty(),
        }
    }
    fn next_focus_path(&self, _event: &FocusNavigationEvent) -> Option<ComponentPath> {
        Some(ComponentPath::leaf(0))
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        match self.row.try_borrow() {
            Ok(row) if row.is_slider => None,
            _ => Some(WidgetClickSound::UiButtonClick),
        }
    }
}
impl crate::framework::narration::NarratableEntry for RowHandle {
    fn narration_priority(&self) -> crate::framework::narration::NarrationPriority {
        crate::framework::narration::NarrationPriority::None
    }
    fn update_narration(&self, _output: &mut crate::framework::narration::NarrationElementOutput) {}
}
impl LayoutElement for RowHandle {
    fn set_x(&mut self, x: i32) {
        if let Ok(mut row) = self.row.try_borrow_mut() {
            row.rect.position.x = x;
        }
    }
    fn set_y(&mut self, y: i32) {
        if let Ok(mut row) = self.row.try_borrow_mut() {
            row.rect.position.y = y;
        }
    }
    fn x(&self) -> i32 {
        self.with_row(|row| row.rect.position.x, 0)
    }
    fn y(&self) -> i32 {
        self.with_row(|row| row.rect.position.y, 0)
    }
    fn width(&self) -> i32 {
        self.with_row(|row| row.rect.width, 0)
    }
    fn height(&self) -> i32 {
        self.with_row(|row| row.rect.height, 0)
    }
}
struct FooterButtonHandle {
    button: Rc<RefCell<Button<VideoSettingsAction>>>,
}
impl FooterButtonHandle {
    fn new(button: Rc<RefCell<Button<VideoSettingsAction>>>) -> Self {
        Self { button }
    }
}
impl Renderable for FooterButtonHandle {
    fn extract_render_state(
        &mut self,
        _graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
    }
}
impl TabOrderedElement for FooterButtonHandle {
    fn tab_order_group(&self) -> i32 {
        self.button.borrow().tab_order_group()
    }
}
impl GuiEventListener for FooterButtonHandle {
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.button.borrow().is_mouse_over(mouse_x, mouse_y)
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.button.borrow_mut().mouse_clicked(event, double_click)
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        self.button.borrow_mut().mouse_released(event)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        self.button.borrow_mut().key_pressed(event)
    }
    fn set_focused(&mut self, focused: bool) {
        self.button.borrow_mut().set_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.button.borrow().is_focused()
    }
    fn rectangle(&self) -> ScreenRectangle {
        GuiEventListener::rectangle(&*self.button.borrow())
    }
    fn next_focus_path(&self, event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.button.borrow().next_focus_path(event)
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.button.borrow().play_down_sound()
    }
}
impl crate::framework::narration::NarratableEntry for FooterButtonHandle {
    fn narration_priority(&self) -> crate::framework::narration::NarrationPriority {
        self.button.borrow().narration_priority()
    }
    fn update_narration(&self, output: &mut crate::framework::narration::NarrationElementOutput) {
        self.button.borrow().update_narration(output);
    }
}
impl LayoutElement for FooterButtonHandle {
    fn set_x(&mut self, x: i32) {
        self.button.borrow_mut().set_x(x);
    }
    fn set_y(&mut self, y: i32) {
        self.button.borrow_mut().set_y(y);
    }
    fn x(&self) -> i32 {
        self.button.borrow().x()
    }
    fn y(&self) -> i32 {
        self.button.borrow().y()
    }
    fn width(&self) -> i32 {
        self.button.borrow().width()
    }
    fn height(&self) -> i32 {
        self.button.borrow().height()
    }
}
pub struct VideoSettingsScreen {
    base: ScreenBase,
    rows: Vec<Rc<RefCell<RowWidget>>>,
    done: Rc<RefCell<Button<VideoSettingsAction>>>,
    shader_packs: Rc<RefCell<Button<VideoSettingsAction>>>,
    pub scroll_y: f32,
    pub dragging_scrollbar: bool,
    pub dragging_slider_index: Option<usize>,
    pub focused_slider_index: Option<usize>,
    pub slider_can_change_value: bool,
    pub last_width: i32,
    pub last_height: i32,
    pub options: Vec<VideoSettingsOption>,
    fullscreen_video_modes: Vec<FullscreenVideoMode>,
    pub last_graphics_preset_apply_result: GraphicsPresetApplyResult,
    pub in_world: bool,
    shaderpacks_enabled: bool,
    footer_focus: Option<bool>,
    font: Arc<dyn FontWidthMeasurer>,
    pending_action: Option<VideoSettingsAction>,
    value_changed: bool,
}
impl std::fmt::Debug for VideoSettingsScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoSettingsScreen")
            .field("scroll_y", &self.scroll_y)
            .field("in_world", &self.in_world)
            .field("options_count", &self.options.len())
            .finish()
    }
}
pub fn biome_blend_radius_from_options_value(value: Option<&str>) -> i32 {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return 2;
    };
    if let Ok(radius) = value.parse::<i32>() {
        return radius.clamp(0, 7);
    }
    if value.contains('.') {
        if let Ok(normalized) = value.parse::<f32>() {
            if normalized.is_finite() && (0.0..=1.0).contains(&normalized) {
                return (normalized * 7.0).round() as i32;
            }
        }
    }
    2
}
pub fn ambient_occlusion_enabled_from_options(extra_options: &HashMap<String, String>) -> bool {
    if let Some(value) = extra_options.get("ao") {
        return match value.trim() {
            "true" => true,
            "false" => false,
            _ => true,
        };
    }
    match extra_options
        .get("ambientOcclusion")
        .map(|value| value.trim())
    {
        Some("true") => true,
        Some("false") => false,
        Some(value) => value
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
            .map(|value| value > 0.5)
            .unwrap_or(true),
        None => true,
    }
}
impl VideoSettingsScreen {
    pub fn new(
        in_world: bool,
        extra_options: &HashMap<String, String>,
        gui_scale_setting: u32,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        Self::new_with_video_modes(in_world, extra_options, gui_scale_setting, &[], font)
    }
    pub fn new_with_video_modes(
        in_world: bool,
        extra_options: &HashMap<String, String>,
        gui_scale_setting: u32,
        fullscreen_video_modes: &[FullscreenVideoMode],
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let mut options = Vec::new();
        let get_java_bool = |key: &str, default: bool| -> bool {
            match extra_options.get(key).map(|value| value.trim()) {
                Some("true") => true,
                Some("false") => false,
                _ => default,
            }
        };
        let get_legacy_unit_slider = |key: &str, default: f32, min_val: f32, max_val: f32| -> f32 {
            if let Some(s) = extra_options.get(key) {
                if let Ok(v) = s.parse::<f32>() {
                    let real_val = if v > 1.0 && max_val > 1.0 {
                        v
                    } else {
                        min_val + v.clamp(0.0, 1.0) * (max_val - min_val)
                    };
                    real_val.clamp(min_val, max_val)
                } else {
                    default
                }
            } else {
                default
            }
        };
        let selected_fullscreen_mode = extra_options
            .get("fullscreenResolution")
            .and_then(|value| FullscreenVideoMode::parse(value))
            .and_then(|preferred| {
                fullscreen_video_modes
                    .iter()
                    .position(|mode| mode.matches_display_mode(preferred))
            });
        options.push(VideoSettingsOption {
            key: "fullscreenOption".to_string(),
            label: "options.fullscreen.resolution".to_string(),
            is_slider: false,
            min_val: -1.0,
            max_val: fullscreen_video_modes.len().saturating_sub(1) as f32,
            step: 1.0,
            value: selected_fullscreen_mode.map_or(-1.0, |index| index as f32),
        });
        if fullscreen_video_modes.is_empty() {
            options[0].max_val = -1.0;
        }
        options.push(VideoSettingsOption {
            key: "framerateLimit".to_string(),
            label: "options.framerateLimit".to_string(),
            is_slider: true,
            min_val: 10.0,
            max_val: 260.0,
            step: 10.0,
            value: framerate_limit_from_options(extra_options) as f32,
        });
        options.push(VideoSettingsOption {
            key: "enableVsync".to_string(),
            label: "options.vsync".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if get_java_bool("enableVsync", true) {
                1.0
            } else {
                0.0
            },
        });
        let inactivity_fps_limit_val =
            match extra_options.get("inactivityFpsLimit").map(|s| s.as_str()) {
                Some("minimized") => 0.0,
                Some("afk") => 1.0,
                _ => 1.0,
            };
        options.push(VideoSettingsOption {
            key: "inactivityFpsLimit".to_string(),
            label: "options.inactivityFpsLimit".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: inactivity_fps_limit_val,
        });
        options.push(VideoSettingsOption {
            key: "guiScale".to_string(),
            label: "options.guiScale".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 4.0,
            step: 1.0,
            value: gui_scale_setting as f32,
        });
        options.push(VideoSettingsOption {
            key: "fullscreen".to_string(),
            label: "options.fullscreen".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if get_java_bool("fullscreen", false) {
                1.0
            } else {
                0.0
            },
        });
        options.push(VideoSettingsOption {
            key: "exclusiveFullscreen".to_string(),
            label: "options.exclusiveFullscreen".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if get_java_bool("exclusiveFullscreen", false) {
                1.0
            } else {
                0.0
            },
        });
        options.push(VideoSettingsOption {
            key: "gamma".to_string(),
            label: "options.gamma".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: get_legacy_unit_slider("gamma", 0.5, 0.0, 1.0),
        });
        let preferred_graphics_backend_val = match extra_options
            .get("preferredGraphicsBackend")
            .map(|s| s.as_str())
        {
            Some("default") => 0.0,
            Some("opengl") => 1.0,
            Some("vulkan") => 2.0,
            _ => 0.0,
        };
        options.push(VideoSettingsOption {
            key: "preferredGraphicsBackend".to_string(),
            label: "options.graphicsApi".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 2.0,
            step: 1.0,
            value: preferred_graphics_backend_val,
        });
        options.push(VideoSettingsOption {
            key: "graphicsPreset".to_string(),
            label: "options.graphics.preset".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 3.0,
            step: 1.0,
            value: graphics_preset_from_options(extra_options).slider_value(),
        });
        options.push(VideoSettingsOption {
            key: "biomeBlendRadius".to_string(),
            label: "options.biomeBlendRadius".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 7.0,
            step: 1.0,
            value: biome_blend_radius_from_options_value(
                extra_options.get("biomeBlendRadius").map(String::as_str),
            ) as f32,
        });
        options.push(VideoSettingsOption {
            key: "renderDistance".to_string(),
            label: "options.renderDistance".to_string(),
            is_slider: true,
            min_val: 2.0,
            max_val: 32.0,
            step: 1.0,
            value: java_int_range_from_options(extra_options, "renderDistance", 2, 32, 12) as f32,
        });
        let prioritize_chunk_updates_val = match extra_options
            .get("prioritizeChunkUpdates")
            .map(|value| value.trim())
        {
            Some("none") => 0.0,
            Some("player_affected") => 1.0,
            Some("nearby") => 2.0,
            _ => {
                java_legacy_enum_id_from_options(extra_options, "prioritizeChunkUpdates", 0) as f32
            }
        };
        options.push(VideoSettingsOption {
            key: "prioritizeChunkUpdates".to_string(),
            label: "options.prioritizeChunkUpdates".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 2.0,
            step: 1.0,
            value: prioritize_chunk_updates_val,
        });
        options.push(VideoSettingsOption {
            key: "simulationDistance".to_string(),
            label: "options.simulationDistance".to_string(),
            is_slider: true,
            min_val: 5.0,
            max_val: 32.0,
            step: 1.0,
            value: java_int_range_from_options(extra_options, "simulationDistance", 5, 32, 12)
                as f32,
        });
        options.push(VideoSettingsOption {
            key: "ambientOcclusion".to_string(),
            label: "options.ao".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if ambient_occlusion_enabled_from_options(extra_options) {
                1.0
            } else {
                0.0
            },
        });
        let cloud_status_val = cloud_status_from_options(extra_options);
        options.push(VideoSettingsOption {
            key: "cloudStatus".to_string(),
            label: "options.renderClouds".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 2.0,
            step: 1.0,
            value: cloud_status_val,
        });
        let particles_val = match extra_options.get("particles").map(|value| value.trim()) {
            Some("all") => 0.0,
            Some("decreased") => 1.0,
            Some("minimal") => 2.0,
            _ => java_legacy_enum_id_from_options(extra_options, "particles", 0) as f32,
        };
        options.push(VideoSettingsOption {
            key: "particles".to_string(),
            label: "options.particles".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 2.0,
            step: 1.0,
            value: particles_val,
        });
        options.push(VideoSettingsOption {
            key: "mipmapLevels".to_string(),
            label: "options.mipmapLevels".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 4.0,
            step: 1.0,
            value: java_int_range_from_options(extra_options, "mipmapLevels", 0, 4, 4) as f32,
        });
        options.push(VideoSettingsOption {
            key: "entityShadows".to_string(),
            label: "options.entityShadows".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if get_java_bool("entityShadows", true) {
                1.0
            } else {
                0.0
            },
        });
        options.push(VideoSettingsOption {
            key: "entityDistanceScaling".to_string(),
            label: "options.entityDistanceScaling".to_string(),
            is_slider: true,
            min_val: 0.5,
            max_val: 5.0,
            step: 0.25,
            value: entity_distance_scaling_from_options(extra_options),
        });
        options.push(VideoSettingsOption {
            key: "menuBackgroundBlurriness".to_string(),
            label: "options.accessibility.menu_background_blurriness".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 10.0,
            step: 1.0,
            value: java_int_range_from_options(extra_options, "menuBackgroundBlurriness", 0, 10, 5)
                as f32,
        });
        options.push(VideoSettingsOption {
            key: "cloudRange".to_string(),
            label: "options.renderCloudsDistance".to_string(),
            is_slider: true,
            min_val: 2.0,
            max_val: 128.0,
            step: 1.0,
            value: java_int_range_from_options(extra_options, "cloudRange", 2, 128, 128) as f32,
        });
        options.push(VideoSettingsOption {
            key: "cutoutLeaves".to_string(),
            label: "options.cutoutLeaves".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if get_java_bool("cutoutLeaves", true) {
                1.0
            } else {
                0.0
            },
        });
        options.push(VideoSettingsOption {
            key: "improvedTransparency".to_string(),
            label: "options.improvedTransparency".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if get_java_bool("improvedTransparency", false) {
                1.0
            } else {
                0.0
            },
        });
        let texture_filtering_val = match extra_options
            .get("textureFiltering")
            .map(|value| value.trim())
        {
            Some("none") => 0.0,
            Some("rgss") => 1.0,
            Some("anisotropic") => 2.0,
            _ => java_legacy_enum_id_from_options(extra_options, "textureFiltering", 0) as f32,
        };
        options.push(VideoSettingsOption {
            key: "textureFiltering".to_string(),
            label: "options.textureFiltering".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 2.0,
            step: 1.0,
            value: texture_filtering_val,
        });
        options.push(VideoSettingsOption {
            key: "maxAnisotropyBit".to_string(),
            label: "options.maxAnisotropy".to_string(),
            is_slider: true,
            min_val: 1.0,
            max_val: 3.0,
            step: 1.0,
            value: java_int_range_from_options(extra_options, "maxAnisotropyBit", 1, 3, 2) as f32,
        });
        options.push(VideoSettingsOption {
            key: "weatherRadius".to_string(),
            label: "options.weatherRadius".to_string(),
            is_slider: true,
            min_val: 3.0,
            max_val: 10.0,
            step: 1.0,
            value: java_int_range_from_options(extra_options, "weatherRadius", 3, 10, 10) as f32,
        });
        options.push(VideoSettingsOption {
            key: "showAutosaveIndicator".to_string(),
            label: "options.autosaveIndicator".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if get_java_bool("showAutosaveIndicator", true) {
                1.0
            } else {
                0.0
            },
        });
        options.push(VideoSettingsOption {
            key: "vignette".to_string(),
            label: "options.vignette".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if get_java_bool("vignette", true) {
                1.0
            } else {
                0.0
            },
        });
        let attack_indicator_val = match extra_options
            .get("attackIndicator")
            .map(|value| value.trim())
        {
            Some("off") => 0.0,
            Some("crosshair") => 1.0,
            Some("hotbar") => 2.0,
            _ => java_legacy_enum_id_from_options(extra_options, "attackIndicator", 1) as f32,
        };
        options.push(VideoSettingsOption {
            key: "attackIndicator".to_string(),
            label: "options.attackIndicator".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 2.0,
            step: 1.0,
            value: attack_indicator_val,
        });
        options.push(VideoSettingsOption {
            key: "chunkSectionFadeInTime".to_string(),
            label: "options.chunkFade".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 2.0,
            step: 0.05,
            value: get_legacy_unit_slider("chunkSectionFadeInTime", 0.75, 0.0, 2.0),
        });
        let last_graphics_preset_apply_result = apply_java_graphics_preset(
            &mut options,
            graphics_preset_from_options(extra_options),
            GraphicsPresetCapabilities::UNKNOWN,
        );
        let done = Rc::new(RefCell::new(
            Button::<VideoSettingsAction>::builder(Component::literal(&crate::translate(
                "gui.done",
            )))
            .bounds(0, 0, DONE_BUTTON_WIDTH, DEFAULT_HEIGHT)
            .with_action(VideoSettingsAction::Done)
            .with_font(Arc::clone(&font))
            .build(),
        ));
        let shader_packs = Rc::new(RefCell::new(
            Button::<VideoSettingsAction>::builder(Component::literal("Shader Packs"))
                .bounds(0, 0, 98, DEFAULT_HEIGHT)
                .with_action(VideoSettingsAction::OpenShaderPacks)
                .with_font(Arc::clone(&font))
                .build(),
        ));
        Self {
            base: ScreenBase::new(Component::translatable("options.videoTitle")),
            rows: Vec::new(),
            done,
            shader_packs,
            scroll_y: 0.0,
            dragging_scrollbar: false,
            dragging_slider_index: None,
            focused_slider_index: None,
            slider_can_change_value: false,
            last_width: 0,
            last_height: 0,
            options,
            fullscreen_video_modes: fullscreen_video_modes.to_vec(),
            last_graphics_preset_apply_result,
            in_world,
            shaderpacks_enabled: false,
            footer_focus: None,
            font,
            pending_action: None,
            value_changed: false,
        }
    }
    pub fn has_fullscreen_video_mode_service(&self) -> bool {
        !self.fullscreen_video_modes.is_empty()
    }
    pub fn preferred_fullscreen_video_mode(&self) -> Option<FullscreenVideoMode> {
        let selected = self
            .options
            .iter()
            .find(|option| option.key == "fullscreenOption")?
            .value
            .round();
        if selected < 0.0 {
            None
        } else {
            self.fullscreen_video_modes.get(selected as usize).copied()
        }
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    fn reposition_rows(&mut self) {
        let scroll = self.scroll_y.round() as i32;
        let mut row_idx = 0;
        for item in self.get_layout_items() {
            let item_y = HEADER_HEIGHT + 4 + item.relative_y - scroll;
            match item.kind {
                VideoSettingsListItem::Header { .. } => {}
                VideoSettingsListItem::Big { .. } => {
                    if let Some(row) = self.rows.get(row_idx) {
                        row.borrow_mut().rect.position.y = item_y;
                        row_idx += 1;
                    }
                }
                VideoSettingsListItem::SmallPair { option_index_b, .. } => {
                    if let Some(row) = self.rows.get(row_idx) {
                        row.borrow_mut().rect.position.y = item_y;
                        row_idx += 1;
                    }
                    if option_index_b.is_some()
                        && let Some(row) = self.rows.get(row_idx)
                    {
                        row.borrow_mut().rect.position.y = item_y;
                        row_idx += 1;
                    }
                }
            }
        }
    }
    fn hovered_row(&self, mouse_x: i32, mouse_y: i32) -> Option<(usize, ScreenRectangle)> {
        self.rows.iter().find_map(|row| {
            let r = row.borrow();
            r.rect
                .contains_point(mouse_x, mouse_y)
                .then_some((r.option_index, r.rect))
        })
    }
    fn focused_row(&self) -> Option<(usize, ScreenRectangle, WidgetTooltipPosition)> {
        self.rows.iter().find_map(|row| {
            let r = row.borrow();
            r.focused
                .then_some((r.option_index, r.rect, WidgetTooltipPosition::BelowOrAbove))
        })
    }
    pub fn set_shaderpacks_enabled(&mut self, enabled: bool) {
        if self.shaderpacks_enabled != enabled {
            self.shaderpacks_enabled = enabled;
            self.footer_focus = None;
            self.last_width = 0;
            self.last_height = 0;
        }
    }
    pub fn take_action(&mut self) -> Option<VideoSettingsAction> {
        if let Some(action) = self.pending_action.take() {
            return Some(action);
        }
        if let Some(action) = self.done.borrow_mut().take_press_action() {
            return Some(action);
        }
        if self.shaderpacks_enabled {
            if let Some(action) = self.shader_packs.borrow_mut().take_press_action() {
                return Some(action);
            }
        }
        None
    }
    pub fn take_value_changed(&mut self) -> bool {
        std::mem::take(&mut self.value_changed)
    }
    pub fn focus_extension_footer(&mut self, reverse: bool) {
        if !self.shaderpacks_enabled {
            return;
        }
        self.footer_focus = Some(match self.footer_focus {
            Some(done) => !done,
            None => reverse,
        });
        self.focused_slider_index = None;
        self.slider_can_change_value = false;
    }
    pub fn activate_extension_footer(&mut self) -> Option<VideoSettingsAction> {
        if !self.shaderpacks_enabled {
            return None;
        }
        let done = self.footer_focus?;
        click_sound_sink::record_button_click();
        Some(if done {
            VideoSettingsAction::Done
        } else {
            VideoSettingsAction::OpenShaderPacks
        })
    }
    pub fn footer_buttons(&self, width: i32, height: i32) -> Vec<([i32; 4], VideoSettingsAction)> {
        if !self.shaderpacks_enabled {
            return vec![(
                [
                    width / 2 - 100,
                    height - FOOTER_BUTTON_Y_OFFSET,
                    DONE_BUTTON_WIDTH,
                    FOOTER_BUTTON_HEIGHT,
                ],
                VideoSettingsAction::Done,
            )];
        }
        let total = 200.min((width - 16).max(40));
        let button_width = (total - 4) / 2;
        let left = (width - total) / 2;
        vec![
            (
                [
                    left,
                    height - FOOTER_BUTTON_Y_OFFSET,
                    button_width,
                    FOOTER_BUTTON_HEIGHT,
                ],
                VideoSettingsAction::OpenShaderPacks,
            ),
            (
                [
                    left + button_width + 4,
                    height - FOOTER_BUTTON_Y_OFFSET,
                    button_width,
                    FOOTER_BUTTON_HEIGHT,
                ],
                VideoSettingsAction::Done,
            ),
        ]
    }
    pub fn get_layout_items(&self) -> Vec<LayoutItem> {
        let list_items = vec![
            VideoSettingsListItem::Header {
                label_key: "options.video.display.header",
            },
            VideoSettingsListItem::Big { option_index: 0 },
            VideoSettingsListItem::SmallPair {
                option_index_a: 1,
                option_index_b: Some(2),
            },
            VideoSettingsListItem::SmallPair {
                option_index_a: 3,
                option_index_b: Some(4),
            },
            VideoSettingsListItem::SmallPair {
                option_index_a: 5,
                option_index_b: Some(6),
            },
            VideoSettingsListItem::SmallPair {
                option_index_a: 7,
                option_index_b: Some(8),
            },
            VideoSettingsListItem::Header {
                label_key: "options.video.quality.header",
            },
            VideoSettingsListItem::Big { option_index: 9 },
            VideoSettingsListItem::SmallPair {
                option_index_a: 10,
                option_index_b: Some(11),
            },
            VideoSettingsListItem::SmallPair {
                option_index_a: 12,
                option_index_b: Some(13),
            },
            VideoSettingsListItem::SmallPair {
                option_index_a: 14,
                option_index_b: Some(15),
            },
            VideoSettingsListItem::SmallPair {
                option_index_a: 16,
                option_index_b: Some(17),
            },
            VideoSettingsListItem::SmallPair {
                option_index_a: 18,
                option_index_b: Some(19),
            },
            VideoSettingsListItem::SmallPair {
                option_index_a: 20,
                option_index_b: Some(21),
            },
            VideoSettingsListItem::SmallPair {
                option_index_a: 22,
                option_index_b: Some(23),
            },
            VideoSettingsListItem::SmallPair {
                option_index_a: 24,
                option_index_b: Some(25),
            },
            VideoSettingsListItem::SmallPair {
                option_index_a: 26,
                option_index_b: None,
            },
            VideoSettingsListItem::Header {
                label_key: "options.video.preferences.header",
            },
            VideoSettingsListItem::SmallPair {
                option_index_a: 27,
                option_index_b: Some(28),
            },
            VideoSettingsListItem::SmallPair {
                option_index_a: 29,
                option_index_b: Some(30),
            },
        ];
        let mut items = Vec::new();
        let mut current_y = 0;
        for (i, kind) in list_items.into_iter().enumerate() {
            let height = match &kind {
                VideoSettingsListItem::Header { .. } => {
                    if i == 0 {
                        13
                    } else {
                        31
                    }
                }
                _ => ROW_SPACING,
            };
            items.push(LayoutItem {
                kind,
                relative_y: current_y,
                height,
            });
            current_y += height;
        }
        items
    }
    pub fn max_scroll_amount(&self, height: i32) -> f32 {
        let items = self.get_layout_items();
        if let Some(last) = items.last() {
            let total_height = last.relative_y + last.height + 4;
            let visible_height = height - 66;
            (total_height - visible_height).max(0) as f32
        } else {
            0.0
        }
    }
    pub fn scrollbar_height(&self, height: i32) -> i32 {
        let scroll_area_h = height - 66;
        if scroll_area_h <= 0 {
            return 0;
        }
        let items = self.get_layout_items();
        let total_content_height = if let Some(last) = items.last() {
            last.relative_y + last.height + 4
        } else {
            1
        };
        let bar_h =
            ((scroll_area_h * scroll_area_h) as f32 / total_content_height as f32).round() as i32;
        let upper = (scroll_area_h - 8).max(1);
        let lower = 32.min(upper);
        bar_h.clamp(lower, upper).min(scroll_area_h)
    }
    pub fn slider_key_pressed(&mut self, key: VideoSettingsSliderKey) -> bool {
        let Some(option_idx) = self.focused_slider_index else {
            return false;
        };
        if !self
            .options
            .get(option_idx)
            .is_some_and(|option| option.is_slider)
        {
            self.clear_slider_focus();
            return false;
        }
        match key {
            VideoSettingsSliderKey::Selection => {
                self.slider_can_change_value = !self.slider_can_change_value;
                true
            }
            VideoSettingsSliderKey::Left | VideoSettingsSliderKey::Right
                if self.slider_can_change_value =>
            {
                let Some(width) = self.slider_width(option_idx) else {
                    return false;
                };
                if width <= 8 {
                    return false;
                }
                let option = &self.options[option_idx];
                let span = option.max_val - option.min_val;
                if span <= 0.0 {
                    return false;
                }
                let direction = if key == VideoSettingsSliderKey::Left {
                    -1.0
                } else {
                    1.0
                };
                let normalized = (option.value - option.min_val) / span;
                self.update_slider_from_normalized(
                    option_idx,
                    normalized + direction / (width - 8) as f32,
                );
                true
            }
            VideoSettingsSliderKey::Left | VideoSettingsSliderKey::Right => false,
        }
    }
    fn update_slider_from_mouse(
        &mut self,
        option_idx: usize,
        mx: i32,
        slider_x: i32,
        slider_w: i32,
    ) {
        let usable = slider_w - 8;
        let pct = if usable > 0 {
            (mx - (slider_x + 4)) as f32 / usable as f32
        } else {
            0.0
        };
        self.update_slider_from_normalized(option_idx, pct);
    }
    fn update_slider_from_normalized(&mut self, option_idx: usize, normalized: f32) {
        let (key, changed) = {
            let opt = &mut self.options[option_idx];
            let clamped = normalized.clamp(0.0, 1.0);
            let raw_val = opt.min_val + clamped * (opt.max_val - opt.min_val);
            let steps = ((raw_val - opt.min_val) / opt.step).round();
            let value = (opt.min_val + steps * opt.step).clamp(opt.min_val, opt.max_val);
            let changed = value != opt.value;
            opt.value = value;
            (opt.key.clone(), changed)
        };
        self.on_option_changed(&key, changed);
    }
    fn focus_slider_from_mouse(&mut self, option_idx: usize) {
        self.focused_slider_index = Some(option_idx);
        self.slider_can_change_value = true;
    }
    fn clear_slider_focus(&mut self) {
        self.focused_slider_index = None;
        self.slider_can_change_value = false;
    }
    fn slider_width(&self, option_idx: usize) -> Option<i32> {
        self.get_layout_items()
            .into_iter()
            .find_map(|item| match item.kind {
                VideoSettingsListItem::Big {
                    option_index: candidate,
                } if candidate == option_idx => Some(BIG_BUTTON_WIDTH),
                VideoSettingsListItem::SmallPair {
                    option_index_a,
                    option_index_b,
                } if option_index_a == option_idx || option_index_b == Some(option_idx) => {
                    Some(SMALL_BUTTON_WIDTH)
                }
                _ => None,
            })
    }
    fn cycle_option(&mut self, option_idx: usize) {
        let (key, changed) = {
            let opt = &mut self.options[option_idx];
            let old_value = opt.value;
            opt.value += opt.step;
            if opt.value > opt.max_val {
                opt.value = opt.min_val;
            }
            (opt.key.clone(), opt.value != old_value)
        };
        self.on_option_changed(&key, changed);
    }
    fn on_option_changed(&mut self, key: &str, changed: bool) {
        if !changed {
            return;
        }
        self.value_changed = true;
        if key == "graphicsPreset" {
            let preset = self
                .options
                .iter()
                .find(|option| option.key == "graphicsPreset")
                .map(|option| GraphicsPreset::from_slider_value(option.value.round()))
                .unwrap_or(GraphicsPreset::Fancy);
            self.last_graphics_preset_apply_result = apply_java_graphics_preset(
                &mut self.options,
                preset,
                GraphicsPresetCapabilities::UNKNOWN,
            );
        } else if is_graphics_preset_child_option(key) {
            if let Some(preset) = self
                .options
                .iter_mut()
                .find(|option| option.key == "graphicsPreset")
            {
                preset.value = GraphicsPreset::Custom.slider_value();
            }
        }
    }
    fn ensure_layout(&mut self, width: i32, height: i32) {
        if self.last_width == width && self.last_height == height && !self.rows.is_empty() {
            return;
        }
        self.last_width = width;
        self.last_height = height;
        self.base.clear_widgets();
        if self.shaderpacks_enabled {
            let total = 200.min((width - 16).max(40));
            let button_width = (total - 4) / 2;
            let left = (width - total) / 2;
            {
                let mut sp = self.shader_packs.borrow_mut();
                sp.set_position(left, height - FOOTER_BUTTON_Y_OFFSET);
                sp.base.set_width(button_width);
            }
            {
                let mut d = self.done.borrow_mut();
                d.set_position(left + button_width + 4, height - FOOTER_BUTTON_Y_OFFSET);
                d.base.set_width(button_width);
            }
            self.base
                .add_widget(FooterButtonHandle::new(Rc::clone(&self.shader_packs)));
            self.base
                .add_widget(FooterButtonHandle::new(Rc::clone(&self.done)));
        } else {
            {
                let mut d = self.done.borrow_mut();
                d.set_position(width / 2 - 100, height - FOOTER_BUTTON_Y_OFFSET);
                d.base.set_width(DONE_BUTTON_WIDTH);
            }
            self.base
                .add_widget(FooterButtonHandle::new(Rc::clone(&self.done)));
        }
        self.rows.clear();
        for item in self.get_layout_items() {
            let item_y = HEADER_HEIGHT + 4 + item.relative_y;
            match item.kind {
                VideoSettingsListItem::Header { .. } => {}
                VideoSettingsListItem::Big { option_index } => {
                    self.push_row(width / 2 - 155, item_y, BIG_BUTTON_WIDTH, option_index);
                }
                VideoSettingsListItem::SmallPair {
                    option_index_a,
                    option_index_b,
                } => {
                    self.push_row(width / 2 - 155, item_y, SMALL_BUTTON_WIDTH, option_index_a);
                    if let Some(option_index_b) = option_index_b {
                        self.push_row(width / 2 + 5, item_y, SMALL_BUTTON_WIDTH, option_index_b);
                    }
                }
            }
        }
        for row in &self.rows {
            self.base.add_widget(RowHandle::new(Rc::clone(row)));
        }
    }
    fn push_row(&mut self, x: i32, y: i32, width: i32, option_index: usize) {
        let is_slider = self
            .options
            .get(option_index)
            .is_some_and(|option| option.is_slider);
        let row = Rc::new(RefCell::new(RowWidget::new(
            ScreenRectangle::new(x, y, width, ROW_HEIGHT),
            option_index,
            is_slider,
        )));
        self.rows.push(row);
    }
    fn draw_button(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
        option_idx: usize,
        x: i32,
        y: i32,
        w: i32,
        _mouse_x: i32,
        _mouse_y: i32,
    ) {
        let option = &self.options[option_idx];
        if option.is_slider {
            let span = option.max_val - option.min_val;
            let pct = if span > 0.0 {
                (option.value - option.min_val) / span
            } else {
                0.0
            };
            graphics.draw_slider_with_interaction_state(
                font,
                x,
                y,
                w,
                ROW_HEIGHT,
                &option.get_display_string_with_modes(&self.fullscreen_video_modes),
                pct,
                true,
                self.focused_slider_index == Some(option_idx),
                self.slider_can_change_value && self.focused_slider_index == Some(option_idx),
                self.dragging_slider_index == Some(option_idx),
            );
        } else {
            graphics.draw_button(
                font,
                x,
                y,
                w,
                ROW_HEIGHT,
                &option.get_display_string_with_modes(&self.fullscreen_video_modes),
                true,
            );
        }
    }
    pub fn mouse_clicked(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        width: i32,
        height: i32,
    ) -> Option<VideoSettingsAction> {
        self.base.width = width;
        self.base.height = height;
        self.ensure_layout(width, height);
        let event = MouseButtonEvent::new(
            mouse_x as f64,
            mouse_y as f64,
            crate::input::events::MouseButtonInfo::new(0, 0),
        );
        <Self as Screen>::mouse_clicked(self, event, false);
        self.take_action()
    }
    pub fn mouse_released(&mut self) {
        let event =
            MouseButtonEvent::new(0.0, 0.0, crate::input::events::MouseButtonInfo::new(0, 0));
        <Self as Screen>::mouse_released(self, event);
    }
}
impl Screen for VideoSettingsScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn should_close_on_esc(&self) -> bool {
        true
    }
    fn init(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        self.last_width = 0;
        self.last_height = 0;
        self.rows.clear();
        self.ensure_layout(width, height);
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if event.is_escape() {
            self.pending_action = Some(VideoSettingsAction::Done);
            return true;
        }
        let footer_offset = if self.shaderpacks_enabled { 2 } else { 1 };
        let focused_idx = self.base_ref().focused_index();
        if let Some(index) = focused_idx {
            if index < footer_offset {
                if event.is_selection() {
                    if index == 0 && self.shaderpacks_enabled {
                        self.pending_action = Some(VideoSettingsAction::OpenShaderPacks);
                    } else {
                        self.pending_action = Some(VideoSettingsAction::Done);
                    }
                    click_sound_sink::record_button_click();
                    return true;
                }
            } else if let Some(row) = self.rows.get(index - footer_offset) {
                let (option_index, is_slider) = {
                    let row = row.borrow();
                    (row.option_index, row.is_slider)
                };
                if is_slider {
                    if event.is_selection() {
                        self.slider_can_change_value = !self.slider_can_change_value;
                        return true;
                    }
                    if self.slider_can_change_value {
                        let dir = match event.key {
                            KeyEvent::KEY_LEFT => Some(-1.0),
                            KeyEvent::KEY_RIGHT => Some(1.0),
                            _ => None,
                        };
                        if let Some(dir) = dir {
                            if let Some(width) = self.slider_width(option_index) {
                                if width > 8 {
                                    let option = &self.options[option_index];
                                    let span = option.max_val - option.min_val;
                                    if span > 0.0 {
                                        let normalized = (option.value - option.min_val) / span;
                                        self.update_slider_from_normalized(
                                            option_index,
                                            normalized + dir / (width - 8) as f32,
                                        );
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                } else if event.is_selection() {
                    click_sound_sink::record_button_click();
                    self.cycle_option(option_index);
                    if self.options[option_index].key == "guiScale" {
                        self.pending_action = Some(VideoSettingsAction::GuiScaleChanged);
                    }
                    return true;
                }
            }
        }
        let handled = crate::framework::screen::screen_key_pressed(self, event);
        if handled || (event.key == KeyEvent::KEY_TAB && self.base_ref().focused_index().is_some())
        {
            return true;
        }
        false
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        let handled = crate::framework::screen::screen_mouse_clicked(self, event, double_click);
        if !handled {
            return false;
        }
        let width = self.base_ref().width;
        let height = self.base_ref().height;
        let scrollbar_x = width / 2 + 166;
        let bar_h = self.scrollbar_height(height);
        let max_scroll = self.max_scroll_amount(height);
        let bar_y = if max_scroll == 0.0 {
            HEADER_HEIGHT
        } else {
            let scroll_area_h = height - 66;
            HEADER_HEIGHT
                + (self.scroll_y * (scroll_area_h - bar_h) as f32 / max_scroll).round() as i32
        };
        let (mx, my) = (event.x as i32, event.y as i32);
        if event.button() == 0
            && mx >= scrollbar_x
            && mx < scrollbar_x + 6
            && my >= HEADER_HEIGHT
            && my < height - FOOTER_HEIGHT
        {
            self.dragging_scrollbar = true;
            if my < bar_y {
                self.scroll_y = (self.scroll_y - 25.0).max(0.0);
            } else if my > bar_y + bar_h {
                self.scroll_y = (self.scroll_y + 25.0).min(max_scroll);
            }
            return true;
        }
        if let Some(action) = self.done.borrow_mut().take_press_action() {
            self.pending_action = Some(action);
            return true;
        }
        if self.shaderpacks_enabled {
            if let Some(action) = self.shader_packs.borrow_mut().take_press_action() {
                self.pending_action = Some(action);
                return true;
            }
        }
        let pressed: Vec<(usize, bool, ScreenRectangle)> = self
            .rows
            .iter()
            .filter_map(|row| {
                let mut row = row.borrow_mut();
                row.takes_press()
                    .then(|| (row.option_index, row.is_slider, row.rect))
            })
            .collect();
        for (option_index, is_slider, rect) in pressed {
            if is_slider {
                click_sound_sink::record(None);
                self.focus_slider_from_mouse(option_index);
                self.dragging_slider_index = Some(option_index);
                self.update_slider_from_mouse(
                    option_index,
                    event.x as i32,
                    rect.left(),
                    rect.width,
                );
            } else {
                self.clear_slider_focus();
                click_sound_sink::record_button_click();
                self.cycle_option(option_index);
                if self.options[option_index].key == "guiScale" {
                    self.pending_action = Some(VideoSettingsAction::GuiScaleChanged);
                }
            }
        }
        true
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        let handled = crate::framework::screen::screen_mouse_released(self, event);
        if self.dragging_slider_index.take().is_some() {
            click_sound_sink::record_button_click();
        }
        self.dragging_scrollbar = false;
        handled
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, _dx: f64, dy: f64) -> bool {
        let handled = crate::framework::screen::screen_mouse_dragged(self, event, _dx, dy);
        let height = self.base_ref().height;
        if self.dragging_scrollbar {
            let max_scroll = self.max_scroll_amount(height);
            let bar_h = self.scrollbar_height(height);
            let scroll_area_h = height - 66;
            let y_drag_scale = if scroll_area_h - bar_h > 0 {
                max_scroll / (scroll_area_h - bar_h) as f32
            } else {
                1.0
            };
            self.scroll_y = (self.scroll_y + dy as f32 * y_drag_scale).clamp(0.0, max_scroll);
            self.reposition_rows();
        } else if let Some(slider_idx) = self.dragging_slider_index {
            let rect = self
                .rows
                .iter()
                .find(|row| row.borrow().option_index == slider_idx)
                .map(|row| row.borrow().rect)
                .unwrap_or_else(ScreenRectangle::empty);
            self.update_slider_from_mouse(slider_idx, event.x as i32, rect.left(), rect.width);
        }
        handled
    }
    fn mouse_scrolled(&mut self, _x: f64, _y: f64, _scroll_x: f64, scroll_y: f64) -> bool {
        let height = self.base_ref().height;
        let max_scroll = self.max_scroll_amount(height);
        self.scroll_y = (self.scroll_y - scroll_y as f32 * 12.0).clamp(0.0, max_scroll);
        self.reposition_rows();
        true
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        self.reposition_rows();
        self.base
            .extract_background_backdrop(graphics, self.in_world);
        let list_bg_tex = if self.in_world {
            "minecraft:gui/inworld_menu_list_background"
        } else {
            "minecraft:gui/menu_list_background"
        };
        let list_background = ResourceLocation::from_string(list_bg_tex);
        let mut y = HEADER_HEIGHT;
        while y < height - FOOTER_HEIGHT {
            let tile_h = 32.min(height - FOOTER_HEIGHT - y);
            let mut x = 0;
            while x < width {
                let tile_w = 32.min(width - x);
                graphics.blit_region(
                    list_background.clone(),
                    x,
                    y,
                    tile_w,
                    tile_h,
                    0.0,
                    (y as f32 + self.scroll_y) / 32.0,
                    tile_w as f32 / 32.0,
                    tile_h as f32 / 32.0,
                    0xFFFF_FFFFu32 as i32,
                );
                x += 32;
            }
            y += tile_h;
        }
        graphics.enable_scissor(0, HEADER_HEIGHT, width, height - FOOTER_HEIGHT);
        let layout_items = self.get_layout_items();
        let mouse_x = graphics.mouse_x();
        let mouse_y = graphics.mouse_y();
        for item in layout_items {
            let item_y = HEADER_HEIGHT + 4 + item.relative_y - self.scroll_y as i32;
            match item.kind {
                VideoSettingsListItem::Header { label_key } => {
                    let header_text = crate::translate(label_key);
                    let text_y = if item.height == 13 {
                        item_y
                    } else {
                        item_y + 18
                    };
                    graphics.text(
                        ResourceLocation::from_string("minecraft:default"),
                        header_text,
                        width / 2 - 155,
                        text_y,
                        0xFFFF_FFFFu32 as i32,
                    );
                }
                VideoSettingsListItem::Big { option_index } => {
                    let btn_x = width / 2 - 155;
                    let btn_y = item_y;
                    self.draw_button(
                        graphics,
                        self.font.as_ref(),
                        option_index,
                        btn_x,
                        btn_y,
                        BIG_BUTTON_WIDTH,
                        mouse_x,
                        mouse_y,
                    );
                }
                VideoSettingsListItem::SmallPair {
                    option_index_a,
                    option_index_b,
                } => {
                    let btn_x_a = width / 2 - 155;
                    let btn_y = item_y;
                    self.draw_button(
                        graphics,
                        self.font.as_ref(),
                        option_index_a,
                        btn_x_a,
                        btn_y,
                        SMALL_BUTTON_WIDTH,
                        mouse_x,
                        mouse_y,
                    );
                    if let Some(option_index_b) = option_index_b {
                        let btn_x_b = width / 2 + 5;
                        self.draw_button(
                            graphics,
                            self.font.as_ref(),
                            option_index_b,
                            btn_x_b,
                            btn_y,
                            SMALL_BUTTON_WIDTH,
                            mouse_x,
                            mouse_y,
                        );
                    }
                }
            }
        }
        graphics.disable_scissor();
        graphics.draw_menu_background(self.in_world, width, HEADER_HEIGHT);
        graphics.pose_mut().push_pose();
        graphics
            .pose_mut()
            .translate(0.0, (height - FOOTER_HEIGHT) as f32);
        graphics.draw_menu_background(self.in_world, width, FOOTER_HEIGHT);
        graphics.pose_mut().pop_pose();
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/header_separator"),
            0,
            31,
            width,
            2,
        );
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/footer_separator"),
            0,
            height - FOOTER_HEIGHT,
            width,
            2,
        );
        let title_text = crate::translate("options.videoTitle");
        let title_w = self.font.measure_width(&title_text, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            title_text,
            (width - title_w) / 2,
            12,
            0xFFFF_FFFFu32 as i32,
        );
        let max_scroll = self.max_scroll_amount(height);
        if max_scroll > 0.0 {
            let scrollbar_x = width / 2 + 166;
            let scroll_area_h = height - 66;
            let bar_h = self.scrollbar_height(height);
            let bar_y = HEADER_HEIGHT
                + (self.scroll_y * (scroll_area_h - bar_h) as f32 / max_scroll).round() as i32;
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller_background"),
                scrollbar_x,
                HEADER_HEIGHT,
                6,
                scroll_area_h,
            );
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller"),
                scrollbar_x,
                bar_y,
                6,
                bar_h,
            );
        }
        for (rect, action) in self.footer_buttons(width, height) {
            let [x, y, w, h] = rect;
            let done = action == VideoSettingsAction::Done;
            let label = if done {
                crate::translate("gui.done")
            } else {
                "Shader Packs".to_owned()
            };
            let focused = if done {
                self.done.borrow().is_focused()
            } else {
                self.shader_packs.borrow().is_focused()
            };
            graphics.draw_button_with_focus(
                self.font.as_ref(),
                x,
                y,
                w,
                h,
                &label,
                true,
                focused || self.footer_focus == Some(done),
            );
        }
        let target = if mouse_y >= HEADER_HEIGHT && mouse_y < height - FOOTER_HEIGHT {
            self.hovered_row(mouse_x, mouse_y)
                .map(|(idx, rect)| (idx, rect, WidgetTooltipPosition::Menu))
        } else {
            None
        }
        .or_else(|| self.focused_row());
        if let Some((option_idx, rect, position)) = target
            && let Some(option) = self.options.get(option_idx)
            && let Some(lines) =
                crate::settings::option_tooltips::video_tooltip(&option.key, option.value)
        {
            graphics.draw_widget_tooltip(
                self.font.as_ref(),
                &lines,
                mouse_x,
                mouse_y,
                rect,
                position,
            );
        }
    }
}
