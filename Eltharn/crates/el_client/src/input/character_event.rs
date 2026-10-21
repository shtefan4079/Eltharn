#![allow(dead_code)]

pub struct CharacterEvent {
    pub codepoint: u32,
}

impl CharacterEvent {
    pub fn new(codepoint: u32) -> Self {
        Self { codepoint }
    }

    pub fn codepoint_as_string(&self) -> String {
        if let Some(c) = char::from_u32(self.codepoint) {
            c.to_string()
        } else {
            String::new()
        }
    }

    pub fn is_allowed_chat_character(&self) -> bool {
        self.codepoint != 167 && self.codepoint >= 32 && self.codepoint != 127
    }
}
