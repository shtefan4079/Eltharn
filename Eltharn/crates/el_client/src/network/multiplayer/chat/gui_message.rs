#![allow(dead_code)]

use crate::network::multiplayer::chat::{GuiMessageSource, GuiMessageTag};

#[derive(Debug, Clone)]
pub struct GuiMessage {
    pub added_time: i32,
    pub content: String,
    pub signature: Option<Vec<u8>>,
    pub source: GuiMessageSource,
    pub tag: Option<GuiMessageTag>,
}

impl GuiMessage {
    pub fn new(
        added_time: i32,
        content: impl Into<String>,
        signature: Option<Vec<u8>>,
        source: GuiMessageSource,
        tag: Option<GuiMessageTag>,
    ) -> Self {
        GuiMessage {
            added_time,
            content: content.into(),
            signature,
            source,
            tag,
        }
    }

    const TAG_MARGIN_LEFT: i32 = 4;

    pub fn icon_adjusted_width(&self, base_width: i32) -> i32 {
        if let Some(tag) = &self.tag {
            if let Some(icon) = &tag.icon {
                return base_width - icon.width() as i32 - Self::TAG_MARGIN_LEFT - 2;
            }
        }
        base_width
    }
}

#[derive(Debug, Clone)]
pub struct GuiMessageLine {
    pub parent_added_time: i32,
    pub content: String,
    pub end_of_entry: bool,
    pub tag: Option<GuiMessageTag>,
}
