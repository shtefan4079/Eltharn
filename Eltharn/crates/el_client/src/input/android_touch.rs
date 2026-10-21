use el_launcher_core::touch_controls::{TouchAction, TouchControlProfile};
use std::collections::BTreeSet;
use std::collections::HashMap;
use winit::event::{Touch, TouchPhase};
pub fn key_action(action: TouchAction) -> Option<super::key_mapping::KeyAction> {
    use super::key_mapping::KeyAction;
    Some(match action {
        TouchAction::Forward => KeyAction::Forward,
        TouchAction::Back => KeyAction::Back,
        TouchAction::Left => KeyAction::Left,
        TouchAction::Right => KeyAction::Right,
        TouchAction::Attack => KeyAction::Attack,
        TouchAction::Use => KeyAction::Use,
        TouchAction::Jump => KeyAction::Jump,
        TouchAction::Sneak => KeyAction::Sneak,
        TouchAction::Sprint => KeyAction::Sprint,
        TouchAction::Inventory => KeyAction::Inventory,
        TouchAction::SwapHands => KeyAction::SwapOffhand,
        TouchAction::Drop => KeyAction::Drop,
        TouchAction::Chat => KeyAction::Chat,
        TouchAction::Pause | TouchAction::DebugInfo => return None,
    })
}
#[cfg(target_os = "android")]
mod platform_events {
    use std::collections::VecDeque;
    use std::sync::{Mutex, OnceLock};
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum AndroidHardwareEvent {
        MouseMotion { dx: f64, dy: f64 },
        MouseWheel { x: f64, y: f64 },
        MouseButton { button: u16, pressed: bool },
    }
    static HARDWARE_EVENTS: OnceLock<Mutex<VecDeque<AndroidHardwareEvent>>> = OnceLock::new();
    fn hardware_events() -> &'static Mutex<VecDeque<AndroidHardwareEvent>> {
        HARDWARE_EVENTS.get_or_init(|| Mutex::new(VecDeque::with_capacity(64)))
    }
    pub fn submit_hardware_event(event: AndroidHardwareEvent) {
        if let Ok(mut events) = hardware_events().lock() {
            if events.len() >= 512 {
                events.pop_front();
            }
            events.push_back(event);
        }
    }
    pub fn drain_hardware_events() -> Vec<AndroidHardwareEvent> {
        let Ok(mut events) = hardware_events().lock() else {
            return Vec::new();
        };
        events.drain(..).collect()
    }
    static TEXT_INPUT_EVENTS: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();
    fn text_input_events() -> &'static Mutex<VecDeque<String>> {
        TEXT_INPUT_EVENTS.get_or_init(|| Mutex::new(VecDeque::with_capacity(32)))
    }
    pub fn submit_text_input(text: String) {
        if let Ok(mut events) = text_input_events().lock() {
            if events.len() >= 256 {
                events.pop_front();
            }
            events.push_back(text);
        }
    }
    pub fn drain_text_input() -> Vec<String> {
        let Ok(mut events) = text_input_events().lock() else {
            return Vec::new();
        };
        events.drain(..).collect()
    }
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum AndroidKeyAction {
        Enter,
        Escape,
    }
    static KEY_ACTION_EVENTS: OnceLock<Mutex<VecDeque<AndroidKeyAction>>> = OnceLock::new();
    fn key_action_events() -> &'static Mutex<VecDeque<AndroidKeyAction>> {
        KEY_ACTION_EVENTS.get_or_init(|| Mutex::new(VecDeque::with_capacity(16)))
    }
    pub fn submit_key_action(action: AndroidKeyAction) {
        if let Ok(mut events) = key_action_events().lock() {
            if events.len() >= 32 {
                events.pop_front();
            }
            events.push_back(action);
        }
    }
    pub fn drain_key_actions() -> Vec<AndroidKeyAction> {
        let Ok(mut events) = key_action_events().lock() else {
            return Vec::new();
        };
        events.drain(..).collect()
    }
}
#[cfg(target_os = "android")]
pub use platform_events::*;
#[derive(Debug, Default)]
pub struct AndroidMenuPointer {
    owner: Option<u64>,
}
impl AndroidMenuPointer {
    pub fn transition(&mut self, id: u64, phase: TouchPhase) -> Option<TouchPhase> {
        match phase {
            TouchPhase::Started if self.owner.is_none() => self.owner = Some(id),
            TouchPhase::Moved if self.owner == Some(id) => {}
            TouchPhase::Ended | TouchPhase::Cancelled if self.owner == Some(id) => {
                self.owner = None;
            }
            _ => return None,
        }
        Some(phase)
    }
    pub fn release(&mut self) -> bool {
        self.owner.take().is_some()
    }
}
pub fn content_rect_insets(width: f32, height: f32, rect: [i32; 4]) -> [f32; 4] {
    let [left, top, right, bottom] = rect.map(|value| value as f32);
    if !width.is_finite()
        || !height.is_finite()
        || width <= 0.0
        || height <= 0.0
        || right <= left
        || bottom <= top
    {
        return [0.0; 4];
    }
    let left = left.clamp(0.0, width);
    let top = top.clamp(0.0, height);
    let right = right.clamp(left, width);
    let bottom = bottom.clamp(top, height);
    if right - left <= 1.0 || bottom - top <= 1.0 {
        return [0.0; 4];
    }
    [left, top, width - right, height - bottom]
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TouchZone {
    Joystick,
    Look,
    Button(TouchAction),
    Unclaimed,
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct TrackedTouch {
    zone: TouchZone,
    point: [f32; 2],
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TouchVisualState {
    pub joystick_offset: [f32; 2],
    pub joystick_active: bool,
    pub pressed: BTreeSet<TouchAction>,
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AndroidTouchUpdate {
    pub changes: Vec<(TouchAction, bool)>,
    pub look_delta: (f64, f64),
}
#[derive(Debug, Default)]
pub struct AndroidTouchController {
    touches: HashMap<u64, TrackedTouch>,
    joystick_pointer: Option<u64>,
    look_pointer: Option<u64>,
    visual: TouchVisualState,
}
impl AndroidTouchController {
    pub fn visual_state(&self) -> &TouchVisualState {
        &self.visual
    }
    pub fn release_all(&mut self) -> AndroidTouchUpdate {
        let changes = self
            .visual
            .pressed
            .iter()
            .map(|action| (*action, false))
            .collect();
        self.touches.clear();
        self.joystick_pointer = None;
        self.look_pointer = None;
        self.visual = TouchVisualState::default();
        AndroidTouchUpdate {
            changes,
            look_delta: (0.0, 0.0),
        }
    }
    pub fn handle_touch_insets(
        &mut self,
        touch: &Touch,
        width: f32,
        height: f32,
        insets: [f32; 4],
        profile: &TouchControlProfile,
    ) -> AndroidTouchUpdate {
        let Ok(layout) = profile.resolve_safe_viewport(width, height, insets) else {
            return self.release_all();
        };
        if !layout.enabled {
            return self.release_all();
        }
        let point = [touch.location.x as f32, touch.location.y as f32];
        if point.iter().any(|value| !value.is_finite()) {
            return self.release_all();
        }
        let mut update = AndroidTouchUpdate::default();
        match touch.phase {
            TouchPhase::Started => {
                if self.touches.contains_key(&touch.id) || self.touches.len() >= 16 {
                    return update;
                }
                let zone = if let Some(button) = layout
                    .buttons
                    .iter()
                    .rev()
                    .find(|button| button.circle.contains(point))
                {
                    TouchZone::Button(button.action)
                } else if layout.joystick.contains(point) && self.joystick_pointer.is_none() {
                    self.joystick_pointer = Some(touch.id);
                    TouchZone::Joystick
                } else if layout.joystick.contains(point) || self.look_pointer.is_some() {
                    TouchZone::Unclaimed
                } else {
                    self.look_pointer = Some(touch.id);
                    TouchZone::Look
                };
                self.touches.insert(touch.id, TrackedTouch { zone, point });
            }
            TouchPhase::Moved => {
                let Some(tracked) = self.touches.get_mut(&touch.id) else {
                    return update;
                };
                if tracked.zone == TouchZone::Look {
                    update.look_delta = (
                        f64::from(point[0] - tracked.point[0])
                            * f64::from(profile.look_sensitivity),
                        f64::from(point[1] - tracked.point[1])
                            * f64::from(profile.look_sensitivity),
                    );
                }
                tracked.point = point;
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if self.touches.remove(&touch.id).is_none() {
                    return update;
                }
                if self.joystick_pointer == Some(touch.id) {
                    self.joystick_pointer = None;
                }
                if self.look_pointer == Some(touch.id) {
                    self.look_pointer = None;
                }
            }
        }
        let previous = self.visual.pressed.clone();
        self.visual = TouchVisualState::default();
        for tracked in self.touches.values() {
            if let TouchZone::Button(action) = tracked.zone {
                self.visual.pressed.insert(action);
            }
        }
        if let Some(pointer) = self.joystick_pointer.and_then(|id| self.touches.get(&id)) {
            let dx = (pointer.point[0] - layout.joystick.center[0]) / layout.joystick.radius;
            let dy = (pointer.point[1] - layout.joystick.center[1]) / layout.joystick.radius;
            let distance = dx.hypot(dy);
            self.visual.joystick_active = true;
            let divisor = distance.max(1.0);
            self.visual.joystick_offset = [dx / divisor, dy / divisor];
            if distance > profile.dead_zone {
                let threshold = profile.dead_zone;
                if dx < -threshold {
                    self.visual.pressed.insert(TouchAction::Left);
                }
                if dx > threshold {
                    self.visual.pressed.insert(TouchAction::Right);
                }
                if dy < -threshold {
                    self.visual.pressed.insert(TouchAction::Forward);
                }
                if dy > threshold {
                    self.visual.pressed.insert(TouchAction::Back);
                }
            }
        }
        update.changes.extend(
            previous
                .difference(&self.visual.pressed)
                .map(|action| (*action, false)),
        );
        update.changes.extend(
            self.visual
                .pressed
                .difference(&previous)
                .map(|action| (*action, true)),
        );
        update
    }
}
