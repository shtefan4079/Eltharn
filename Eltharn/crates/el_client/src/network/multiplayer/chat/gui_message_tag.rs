#![allow(dead_code)]

pub const CHAT_MODIFIED_SPRITE: &str = "icon/chat_modified";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuiMessageTagIcon {
    ChatModified,
}

impl GuiMessageTagIcon {
    pub const fn width(&self) -> u32 {
        match self {
            GuiMessageTagIcon::ChatModified => 9,
        }
    }

    pub const fn height(&self) -> u32 {
        match self {
            GuiMessageTagIcon::ChatModified => 9,
        }
    }

    pub fn sprite(&self) -> &'static str {
        match self {
            GuiMessageTagIcon::ChatModified => CHAT_MODIFIED_SPRITE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GuiMessageTag {
    pub indicator_color: u32,
    pub icon: Option<GuiMessageTagIcon>,
    pub text: Option<String>,
    pub log_tag: Option<&'static str>,
}

impl GuiMessageTag {
    const COLOR_NOT_SECURE: u32 = 13_684_944;
    const COLOR_MODIFIED: u32 = 6_316_128;
    const COLOR_ERROR: u32 = 16_733_525;

    pub fn system() -> Self {
        GuiMessageTag {
            indicator_color: Self::COLOR_NOT_SECURE,
            icon: None,
            text: Some("chat.tag.system".to_owned()),
            log_tag: Some("System"),
        }
    }

    pub fn system_single_player() -> Self {
        GuiMessageTag {
            indicator_color: Self::COLOR_NOT_SECURE,
            icon: None,
            text: Some("chat.tag.system_single_player".to_owned()),
            log_tag: Some("System"),
        }
    }

    pub fn chat_not_secure() -> Self {
        GuiMessageTag {
            indicator_color: Self::COLOR_NOT_SECURE,
            icon: None,
            text: Some("chat.tag.not_secure".to_owned()),
            log_tag: Some("Not Secure"),
        }
    }

    pub fn chat_modified(original_content: impl Into<String>) -> Self {
        let original = original_content.into();
        let text = format!("chat.tag.modified\n{}", original);
        GuiMessageTag {
            indicator_color: Self::COLOR_MODIFIED,
            icon: Some(GuiMessageTagIcon::ChatModified),
            text: Some(text),
            log_tag: Some("Modified"),
        }
    }

    pub fn chat_error() -> Self {
        GuiMessageTag {
            indicator_color: Self::COLOR_ERROR,
            icon: None,
            text: Some("chat.tag.error".to_owned()),
            log_tag: Some("Chat Error"),
        }
    }
}
