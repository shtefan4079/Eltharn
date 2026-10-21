#![allow(dead_code)]
use crate::gameplay::camera_type::CameraType;
use crate::input::{
    CharacterEvent, InputWithModifiers, KeyAction, KeyEvent, PreeditEvent, VanillaKeyMappings,
};
pub const DEBUG_CRASH_TIME_MS: u64 = 10_000;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPressAction {
    Release,
    Press,
    Repeat,
}
impl KeyPressAction {
    pub const fn from_glfw_action(action: i32) -> Option<Self> {
        match action {
            0 => Some(Self::Release),
            1 => Some(Self::Press),
            2 => Some(Self::Repeat),
            _ => None,
        }
    }
    pub const fn is_down_transition(self) -> bool {
        matches!(self, Self::Press | Self::Repeat)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerspectiveChange {
    pub previous: CameraType,
    pub current: CameraType,
    pub crossed_first_person_boundary: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VanillaKeybindOutcome {
    pub perspective_change: Option<PerspectiveChange>,
    pub smooth_camera_toggled: bool,
    pub gui_toggled: bool,
    pub spectator_shader_effects_toggled: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardInputContext {
    pub handles_game_input: bool,
    pub handles_global_input: bool,
    pub debug_modifier_down: bool,
    pub debug_overlay_modifier_same: bool,
    pub profiler_chart_shown: bool,
}
impl Default for KeyboardInputContext {
    fn default() -> Self {
        Self {
            handles_game_input: true,
            handles_global_input: true,
            debug_modifier_down: false,
            debug_overlay_modifier_same: false,
            profiler_chart_shown: false,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyPressOutcome {
    pub mapping_updated: bool,
    pub toggle_debug_overlay: bool,
    pub pause_requested: bool,
    pub did_debug_action: bool,
    pub debug_action: Option<KeyAction>,
    pub profiler_digit: Option<i32>,
    pub consumed: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugCrashTick {
    Idle,
    Prompt,
    Warning { seconds_remaining: u64 },
    TriggerCrash,
}
#[allow(dead_code)]
pub struct KeyboardHandler {
    debug_crash_key_time_ms: Option<u64>,
    debug_crash_key_reported_time_ms: u64,
    debug_crash_key_reported_count: u64,
    used_debug_key_as_modifier: bool,
    last_preedit_event: Option<PreeditEvent>,
    clipboard: String,
}
impl Default for KeyboardHandler {
    fn default() -> Self {
        Self::new()
    }
}
#[allow(dead_code)]
impl KeyboardHandler {
    pub fn new() -> Self {
        Self {
            debug_crash_key_time_ms: None,
            debug_crash_key_reported_time_ms: 0,
            debug_crash_key_reported_count: 0,
            used_debug_key_as_modifier: false,
            last_preedit_event: None,
            clipboard: String::new(),
        }
    }
    pub fn consume_vanilla_keybinds(
        mappings: &mut VanillaKeyMappings,
        camera_type: &mut CameraType,
        smooth_camera: &mut bool,
        hud_hidden: &mut bool,
    ) -> VanillaKeybindOutcome {
        let mut outcome = VanillaKeybindOutcome::default();
        while let Some(change) = Self::consume_perspective_change(mappings, *camera_type) {
            *camera_type = change.current;
            outcome.perspective_change = Some(change);
        }
        while Self::consume_smooth_camera_toggle(mappings, smooth_camera) {
            outcome.smooth_camera_toggled = true;
        }
        while mappings.consume_click(KeyAction::ToggleGui) {
            *hud_hidden = !*hud_hidden;
            outcome.gui_toggled = true;
        }
        while mappings.consume_click(KeyAction::ToggleSpectatorShaderEffects) {
            outcome.spectator_shader_effects_toggled = true;
        }
        outcome
    }
    pub fn consume_perspective_change(
        mappings: &mut VanillaKeyMappings,
        current: CameraType,
    ) -> Option<PerspectiveChange> {
        if !mappings.consume_click(KeyAction::TogglePerspective) {
            return None;
        }
        let next = current.cycle();
        Some(PerspectiveChange {
            previous: current,
            current: next,
            crossed_first_person_boundary: current.is_first_person() != next.is_first_person(),
        })
    }
    pub fn consume_smooth_camera_toggle(
        mappings: &mut VanillaKeyMappings,
        smooth_camera: &mut bool,
    ) -> bool {
        if !mappings.consume_click(KeyAction::SmoothCamera) {
            return false;
        }
        *smooth_camera = !*smooth_camera;
        true
    }
    pub fn key_press(
        &mut self,
        mappings: &mut VanillaKeyMappings,
        action: KeyPressAction,
        key_action: Option<KeyAction>,
        event: &KeyEvent,
        context: KeyboardInputContext,
    ) -> KeyPressOutcome {
        let mut outcome = KeyPressOutcome::default();
        if context.debug_overlay_modifier_same
            && key_action == Some(KeyAction::DebugModifier)
            && action == KeyPressAction::Release
        {
            if self.used_debug_key_as_modifier {
                self.used_debug_key_as_modifier = false;
            } else {
                outcome.toggle_debug_overlay = true;
                outcome.consumed = true;
            }
        } else if !context.debug_overlay_modifier_same
            && key_action == Some(KeyAction::DebugOverlay)
            && action == KeyPressAction::Press
        {
            outcome.toggle_debug_overlay = true;
            outcome.consumed = true;
        }
        if action == KeyPressAction::Release {
            if let Some(key_action) = key_action {
                mappings.set_action_down(key_action, false);
                outcome.mapping_updated = true;
            }
            return outcome;
        }
        if context.handles_global_input && event.is_escape() {
            outcome.pause_requested = true;
            outcome.did_debug_action = context.debug_modifier_down;
            outcome.consumed = true;
        } else if context.debug_modifier_down {
            if let Some(key_action) = key_action.filter(|action| is_debug_action(*action)) {
                outcome.did_debug_action = true;
                outcome.debug_action = Some(key_action);
                outcome.consumed = true;
            }
        }
        if context.debug_overlay_modifier_same {
            self.used_debug_key_as_modifier |= outcome.did_debug_action;
        }
        if context.profiler_chart_shown && !context.debug_modifier_down {
            let digit = event.get_digit();
            if digit != crate::input::input_with_modifiers::NOT_DIGIT {
                outcome.profiler_digit = Some(digit);
                outcome.consumed = true;
            }
        }
        if let Some(key_action) = key_action {
            if context.handles_game_input {
                if outcome.did_debug_action {
                    mappings.set_action_down(key_action, false);
                } else {
                    mappings.set_action_down(key_action, true);
                    mappings.click_action(key_action);
                }
                outcome.mapping_updated = true;
            } else if key_action == KeyAction::DebugModifier {
                mappings.set_action_down(key_action, !outcome.did_debug_action);
                outcome.mapping_updated = true;
            }
        }
        outcome
    }
    pub fn update_debug_crash_keys(
        &mut self,
        now_ms: u64,
        debug_crash_down: bool,
        debug_modifier_down: bool,
        modifier_and_overlay_is_same: bool,
    ) {
        if self.debug_crash_key_time_ms.is_some() {
            if !debug_crash_down || !debug_modifier_down {
                self.debug_crash_key_time_ms = None;
            }
        } else if debug_crash_down && debug_modifier_down {
            self.used_debug_key_as_modifier = modifier_and_overlay_is_same;
            self.debug_crash_key_time_ms = Some(now_ms);
            self.debug_crash_key_reported_time_ms = now_ms;
            self.debug_crash_key_reported_count = 0;
        }
    }
    pub fn tick_debug_crash(&mut self, now_ms: u64) -> DebugCrashTick {
        let Some(start_ms) = self.debug_crash_key_time_ms else {
            return DebugCrashTick::Idle;
        };
        let elapsed = now_ms.saturating_sub(start_ms);
        if elapsed > DEBUG_CRASH_TIME_MS {
            return DebugCrashTick::TriggerCrash;
        }
        let reported_elapsed = now_ms.saturating_sub(self.debug_crash_key_reported_time_ms);
        if reported_elapsed < 1_000 {
            return DebugCrashTick::Idle;
        }
        self.debug_crash_key_reported_time_ms = now_ms;
        let report = if self.debug_crash_key_reported_count == 0 {
            DebugCrashTick::Prompt
        } else {
            DebugCrashTick::Warning {
                seconds_remaining: (DEBUG_CRASH_TIME_MS - elapsed).div_ceil(1_000),
            }
        };
        self.debug_crash_key_reported_count += 1;
        report
    }
    pub fn cancel_debug_crash(&mut self) {
        self.debug_crash_key_time_ms = None;
    }
    pub fn debug_crash_armed(&self) -> bool {
        self.debug_crash_key_time_ms.is_some()
    }
    pub fn char_typed(event: &CharacterEvent) -> Option<String> {
        event
            .is_allowed_chat_character()
            .then(|| event.codepoint_as_string())
            .filter(|text| !text.is_empty())
    }
    pub fn preedit_callback(&mut self, event: Option<PreeditEvent>) -> Option<&PreeditEvent> {
        self.last_preedit_event = event;
        self.last_preedit_event.as_ref()
    }
    pub fn last_preedit_event(&self) -> Option<&PreeditEvent> {
        self.last_preedit_event.as_ref()
    }
    pub fn get_clipboard(&self) -> &str {
        &self.clipboard
    }
    pub fn set_clipboard(&mut self, clipboard: impl Into<String>) -> bool {
        let clipboard = clipboard.into();
        if clipboard.is_empty() {
            return false;
        }
        self.clipboard = clipboard;
        true
    }
}
fn is_debug_action(action: KeyAction) -> bool {
    matches!(
        action,
        KeyAction::DebugReloadChunk
            | KeyAction::DebugShowHitboxes
            | KeyAction::DebugClearChat
            | KeyAction::DebugShowChunkBorders
            | KeyAction::DebugShowAdvancedTooltips
            | KeyAction::DebugCopyRecreateCommand
            | KeyAction::DebugSpectate
            | KeyAction::DebugSwitchGameMode
            | KeyAction::DebugDebugOptions
            | KeyAction::DebugFocusPause
            | KeyAction::DebugDumpDynamicTextures
            | KeyAction::DebugReloadResourcePacks
            | KeyAction::DebugProfiling
            | KeyAction::DebugCopyLocation
            | KeyAction::DebugDumpVersion
            | KeyAction::DebugProfilingChart
            | KeyAction::DebugFpsCharts
            | KeyAction::DebugNetworkCharts
            | KeyAction::DebugLightmapTexture
    )
}
