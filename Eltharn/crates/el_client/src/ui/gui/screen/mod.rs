pub mod container;

use el_protocol::packets::game::ContainerInput;

#[derive(Debug, Clone, Copy)]
pub struct InventoryEntityPreview {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub size: f32,
    pub offset_y: f32,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainerClickAction {
    pub slot_num: i16,
    pub button_num: i8,
    pub input: ContainerInput,
    pub sync_changes: bool,
}

#[allow(dead_code)]
impl ContainerClickAction {
    pub fn new(slot_num: i16, button_num: i8, input: ContainerInput) -> Self {
        Self {
            slot_num,
            button_num,
            input,
            sync_changes: true,
        }
    }

    pub fn without_slot_sync(slot_num: i16, button_num: i8, input: ContainerInput) -> Self {
        Self {
            slot_num,
            button_num,
            input,
            sync_changes: false,
        }
    }
}

pub fn normalize_chat_message(message: &str) -> String {
    let normalized = message.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_utf16(&normalized, 256)
}

pub fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

pub fn truncate_utf16(text: &str, max_units: usize) -> String {
    let mut used = 0usize;
    text.chars()
        .take_while(|ch| {
            let width = ch.len_utf16();
            let fits = used.saturating_add(width) <= max_units;
            if fits {
                used = used.saturating_add(width);
            }
            fits
        })
        .collect()
}

#[allow(dead_code)]
pub fn get_item_abbreviation_and_color(item_path: &str) -> (&'static str, [f32; 4]) {
    match item_path {
        "diamond" => ("D", [0.25, 0.9, 1.0, 1.0]),
        "emerald" => ("E", [0.1, 0.9, 0.35, 1.0]),
        "gold_ingot" => ("Au", [1.0, 0.82, 0.2, 1.0]),
        "iron_ingot" => ("Fe", [0.85, 0.85, 0.85, 1.0]),
        "redstone" => ("R", [1.0, 0.15, 0.1, 1.0]),
        "lapis_lazuli" => ("L", [0.2, 0.35, 1.0, 1.0]),
        "coal" => ("C", [0.15, 0.15, 0.15, 1.0]),
        "stick" => ("St", [0.65, 0.42, 0.2, 1.0]),
        _ => ("?", [1.0, 1.0, 1.0, 1.0]),
    }
}
