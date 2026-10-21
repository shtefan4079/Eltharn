use el_gui::input::events::{KeyEvent, MouseButtonEvent, MouseButtonInfo};
use winit::event::{ElementState, MouseButton, WindowEvent};
#[derive(Clone, Copy, Default)]
pub(crate) struct HeldModifiers {
    pub shift: bool,
    pub control: bool,
}
pub(crate) fn modifier_bits(modifiers: HeldModifiers) -> u32 {
    let mut bits = 0;
    if modifiers.shift {
        bits |= KeyEvent::MOD_SHIFT;
    }
    if modifiers.control {
        bits |= KeyEvent::MOD_CONTROL;
    }
    bits
}
pub(crate) fn key_event(
    event: &WindowEvent,
    modifiers: HeldModifiers,
    repeat: bool,
) -> Option<(KeyEvent, ElementState)> {
    let WindowEvent::KeyboardInput {
        event: key_event, ..
    } = event
    else {
        return None;
    };
    let code = match key_event.physical_key {
        winit::keyboard::PhysicalKey::Code(code) => code,
        _ => return None,
    };
    let key = glfw_key(code)?;
    let bits = modifier_bits(modifiers);
    let mut modelled = KeyEvent::new(key, 0, bits);
    if key_event.state == ElementState::Pressed && (repeat || key_event.repeat) {
        modelled = modelled.repeated(true);
    }
    Some((modelled, key_event.state))
}
pub(crate) fn mouse_event(
    event: &WindowEvent,
    mouse_x: f64,
    mouse_y: f64,
    modifiers: HeldModifiers,
) -> Option<MouseButtonEvent> {
    let WindowEvent::MouseInput { button, .. } = event else {
        return None;
    };
    let button = match button {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
        MouseButton::Back => 3,
        MouseButton::Forward => 4,
        MouseButton::Other(code) => *code as i32,
    };
    let bits = modifier_bits(modifiers);
    Some(MouseButtonEvent::new(
        mouse_x,
        mouse_y,
        MouseButtonInfo::new(button, bits),
    ))
}
pub(crate) fn mouse_drag_event(
    mouse_x: f64,
    mouse_y: f64,
    modifiers: HeldModifiers,
) -> MouseButtonEvent {
    MouseButtonEvent::new(
        mouse_x,
        mouse_y,
        MouseButtonInfo::new(0, modifier_bits(modifiers)),
    )
}
pub(crate) fn wheel_delta(event: &WindowEvent) -> Option<(f64, f64)> {
    match event {
        WindowEvent::MouseWheel { delta, .. } => match delta {
            winit::event::MouseScrollDelta::LineDelta(x, y) => Some((f64::from(*x), f64::from(*y))),
            winit::event::MouseScrollDelta::PixelDelta(position) => Some((position.x, position.y)),
        },
        _ => None,
    }
}
pub(crate) fn character_event(event: &WindowEvent) -> Option<char> {
    let WindowEvent::KeyboardInput {
        event: key_event, ..
    } = event
    else {
        return None;
    };
    if key_event.state != ElementState::Pressed {
        return None;
    }
    match &key_event.logical_key {
        winit::keyboard::Key::Character(text) => {
            let mut chars = text.chars();
            let first = chars.next()?;
            chars.next().is_none().then_some(first)
        }
        _ => None,
    }
}
fn glfw_key(code: KeyCode) -> Option<i32> {
    use winit::keyboard::KeyCode;
    Some(match code {
        KeyCode::Escape => 256,
        KeyCode::Enter | KeyCode::NumpadEnter => 257,
        KeyCode::Tab => 258,
        KeyCode::Backspace => 259,
        KeyCode::Space => 32,
        KeyCode::Delete => 261,
        KeyCode::Home => 268,
        KeyCode::End => 269,
        KeyCode::ArrowLeft => 263,
        KeyCode::ArrowRight => 262,
        KeyCode::ArrowUp => 265,
        KeyCode::ArrowDown => 264,
        KeyCode::KeyA => 65,
        KeyCode::KeyB => 66,
        KeyCode::KeyC => 67,
        KeyCode::KeyD => 68,
        KeyCode::KeyE => 69,
        KeyCode::KeyF => 70,
        KeyCode::KeyG => 71,
        KeyCode::KeyH => 72,
        KeyCode::KeyI => 73,
        KeyCode::KeyJ => 74,
        KeyCode::KeyK => 75,
        KeyCode::KeyL => 76,
        KeyCode::KeyM => 77,
        KeyCode::KeyN => 78,
        KeyCode::KeyO => 79,
        KeyCode::KeyP => 80,
        KeyCode::KeyQ => 81,
        KeyCode::KeyR => 82,
        KeyCode::KeyS => 83,
        KeyCode::KeyT => 84,
        KeyCode::KeyU => 85,
        KeyCode::KeyV => 86,
        KeyCode::KeyW => 87,
        KeyCode::KeyX => 88,
        KeyCode::KeyY => 89,
        KeyCode::KeyZ => 90,
        KeyCode::Digit0 => 48,
        KeyCode::Digit1 => 49,
        KeyCode::Digit2 => 50,
        KeyCode::Digit3 => 51,
        KeyCode::Digit4 => 52,
        KeyCode::Digit5 => 53,
        KeyCode::Digit6 => 54,
        KeyCode::Digit7 => 55,
        KeyCode::Digit8 => 56,
        KeyCode::Digit9 => 57,
        _ => return None,
    })
}
use winit::keyboard::KeyCode;
