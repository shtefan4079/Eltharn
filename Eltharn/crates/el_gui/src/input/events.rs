#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseButtonInfo {
    pub button: i32,
    pub modifiers: u32,
}

impl MouseButtonInfo {
    pub fn new(button: i32, modifiers: u32) -> Self {
        Self { button, modifiers }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MouseButtonEvent {
    pub x: f64,
    pub y: f64,
    pub button_info: MouseButtonInfo,
}

impl MouseButtonEvent {
    pub fn new(x: f64, y: f64, button_info: MouseButtonInfo) -> Self {
        Self { x, y, button_info }
    }

    pub fn button(&self) -> i32 {
        self.button_info.button
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: i32,
    pub scancode: i32,
    pub modifiers: u32,
    pub repeat: bool,
}

impl KeyEvent {
    pub const KEY_ESCAPE: i32 = 256;
    pub const KEY_TAB: i32 = 258;
    pub const KEY_ENTER: i32 = 257;
    pub const KEY_SPACE: i32 = 32;
    pub const KEY_BACKSPACE: i32 = 259;
    pub const KEY_DELETE: i32 = 261;
    pub const KEY_HOME: i32 = 268;
    pub const KEY_END: i32 = 269;
    pub const KEY_LEFT: i32 = 263;
    pub const KEY_RIGHT: i32 = 262;
    pub const KEY_UP: i32 = 265;
    pub const KEY_DOWN: i32 = 264;
    pub const KEY_A: i32 = 65;
    pub const KEY_C: i32 = 67;
    pub const KEY_V: i32 = 86;
    pub const KEY_X: i32 = 88;

    pub const MOD_SHIFT: u32 = 1;
    pub const MOD_CONTROL: u32 = 2;
    pub const MOD_ALT: u32 = 4;
    pub const MOD_SUPER: u32 = 8;

    pub fn new(key: i32, scancode: i32, modifiers: u32) -> Self {
        Self {
            key,
            scancode,
            modifiers,
            repeat: false,
        }
    }

    pub fn repeated(mut self, repeat: bool) -> Self {
        self.repeat = repeat;
        self
    }

    pub fn is_escape(&self) -> bool {
        self.key == Self::KEY_ESCAPE
    }

    pub fn has_shift_down(&self) -> bool {
        self.modifiers & Self::MOD_SHIFT != 0
    }

    pub fn has_control_down(&self) -> bool {
        self.modifiers & Self::MOD_CONTROL != 0
    }

    pub fn is_selection(&self) -> bool {
        self.key == Self::KEY_ENTER || self.key == Self::KEY_SPACE
    }

    pub fn is_copy(&self) -> bool {
        self.has_control_down() && self.key == Self::KEY_C
    }

    pub fn is_paste(&self) -> bool {
        self.has_control_down() && self.key == Self::KEY_V
    }

    pub fn is_cut(&self) -> bool {
        self.has_control_down() && self.key == Self::KEY_X
    }

    pub fn is_select_all(&self) -> bool {
        self.has_control_down() && self.key == Self::KEY_A
    }

    pub fn get_digit(&self) -> Option<i32> {
        ('0' as i32..='9' as i32)
            .contains(&self.key)
            .then(|| self.key - '0' as i32)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CharacterEvent {
    pub codepoint: char,
}

impl CharacterEvent {
    pub fn new(codepoint: char) -> Self {
        Self { codepoint }
    }

    pub fn codepoint_as_string(&self) -> String {
        self.codepoint.to_string()
    }

    pub fn is_allowed_chat_character(&self) -> bool {
        !self.codepoint.is_control() && self.codepoint != '\u{7f}'
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PreeditEvent {
    pub full_text: String,
    pub caret_position: usize,
    pub blocks: Vec<String>,
    pub focused_block: usize,
}

impl PreeditEvent {
    pub fn new(
        full_text: String,
        caret_position: usize,
        blocks: Vec<String>,
        focused_block: usize,
    ) -> Self {
        Self {
            full_text,
            caret_position,
            blocks,
            focused_block,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CursorType {
    #[default]
    Default,
    PointingHand,
    NotAllowed,
    Text,
    ResizeNs,
}
