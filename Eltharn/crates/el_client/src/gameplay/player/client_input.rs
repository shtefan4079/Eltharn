use crate::input::PlayerInput;
use glam::Vec2;

pub struct ClientInput {
    pub key_presses: PlayerInput,
    pub move_vector: Vec2,
}

impl Default for ClientInput {
    fn default() -> Self {
        Self {
            key_presses: PlayerInput::default(),
            move_vector: Vec2::ZERO,
        }
    }
}

impl ClientInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self) {}

    pub fn get_move_vector(&self) -> Vec2 {
        self.move_vector
    }

    pub fn has_forward_impulse(&self) -> bool {
        self.move_vector.y > 1e-5
    }

    pub fn make_jump(&mut self) {
        self.key_presses.jump = true;
    }
}
