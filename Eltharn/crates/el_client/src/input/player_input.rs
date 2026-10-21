use crate::input::key_mapping::{KeyAction, VanillaKeyMappings};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerInput {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sneak: bool,
    pub sprint: bool,
}

impl PlayerInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_from_key_mappings(&mut self, mappings: &VanillaKeyMappings) {
        self.forward = mappings.is_down(KeyAction::Forward);
        self.backward = mappings.is_down(KeyAction::Back);
        self.left = mappings.is_down(KeyAction::Left);
        self.right = mappings.is_down(KeyAction::Right);
        self.jump = mappings.is_down(KeyAction::Jump);
        self.sneak = mappings.is_down(KeyAction::Sneak);
        self.sprint = mappings.is_down(KeyAction::Sprint);
    }
}
