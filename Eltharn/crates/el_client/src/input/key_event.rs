#![allow(dead_code)]

use crate::input::input_with_modifiers::InputWithModifiers;

#[allow(dead_code)]
pub struct KeyEvent {
    pub key: i32,
    pub scancode: i32,
    pub modifiers: u32,
}

impl KeyEvent {
    pub fn new(key: i32, scancode: i32, modifiers: u32) -> Self {
        Self {
            key,
            scancode,
            modifiers,
        }
    }
}

impl InputWithModifiers for KeyEvent {
    fn input(&self) -> i32 {
        self.key
    }

    fn _modifiers(&self) -> u32 {
        self.modifiers
    }
}
