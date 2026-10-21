use std::{cell::RefCell, collections::BTreeSet};
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    keyboard::PhysicalKey,
};
pub(crate) const MAX_PRESSED_BUTTONS: usize = 64;
#[derive(Debug)]
pub(crate) struct ButtonState {
    pressed: BTreeSet<String>,
    focused: bool,
    overflowed: bool,
}
impl Default for ButtonState {
    fn default() -> Self {
        Self {
            pressed: BTreeSet::new(),
            focused: true,
            overflowed: false,
        }
    }
}
impl ButtonState {
    pub(crate) fn observe(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                self.key(event.physical_key, event.state);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.set(mouse_name(*button), *state);
            }
            WindowEvent::Focused(focused) => {
                self.focused = *focused;
                if !focused {
                    self.clear();
                }
            }
            _ => {}
        }
    }
    fn key(&mut self, key: PhysicalKey, state: ElementState) {
        if let Some(name) = key_name(key) {
            self.set(name, state);
        }
    }
    fn set(&mut self, name: String, state: ElementState) {
        if self.overflowed {
            return;
        }
        match state {
            ElementState::Released => {
                self.pressed.remove(&name);
            }
            ElementState::Pressed if self.focused => {
                if !self.pressed.contains(&name) && self.pressed.len() == MAX_PRESSED_BUTTONS {
                    self.pressed.clear();
                    self.overflowed = true;
                } else {
                    self.pressed.insert(name);
                }
            }
            ElementState::Pressed => {}
        }
    }
    pub(crate) fn snapshot(&self) -> Vec<String> {
        self.pressed.iter().cloned().collect()
    }
    pub(crate) fn overflowed(&self) -> bool {
        self.overflowed
    }
    pub(crate) fn clear(&mut self) {
        self.pressed.clear();
        self.overflowed = false;
    }
}
pub(crate) fn key_name(key: PhysicalKey) -> Option<String> {
    let PhysicalKey::Code(code) = key else {
        return None;
    };
    let name = format!("{code:?}");
    (!name.is_empty() && name.len() <= 48 && name.bytes().all(|byte| byte.is_ascii_alphanumeric()))
        .then_some(name)
}
pub(crate) fn mouse_name(button: MouseButton) -> String {
    match button {
        MouseButton::Left => "MouseLeft".into(),
        MouseButton::Right => "MouseRight".into(),
        MouseButton::Middle => "MouseMiddle".into(),
        MouseButton::Back => "MouseBack".into(),
        MouseButton::Forward => "MouseForward".into(),
        MouseButton::Other(value) => format!("MouseOther{value}"),
    }
}
thread_local! {
    static BUTTONS : RefCell < ButtonState > = RefCell::new(ButtonState::default());
}
pub(crate) fn observe(event: &WindowEvent) {
    BUTTONS.with(|cell| cell.borrow_mut().observe(event));
}
pub(crate) fn snapshot() -> Vec<String> {
    BUTTONS.with(|cell| cell.borrow().snapshot())
}
pub(crate) fn overflowed() -> bool {
    BUTTONS.with(|cell| cell.borrow().overflowed())
}
pub(crate) fn reset(focused: bool) {
    BUTTONS.with(|cell| {
        *cell.borrow_mut() = ButtonState {
            focused,
            ..Default::default()
        }
    });
}
