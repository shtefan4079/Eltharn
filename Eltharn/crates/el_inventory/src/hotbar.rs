use crate::layout::SlotPosition;
use el_core::item::ItemStack;
#[derive(Debug, Clone, PartialEq)]
pub struct Hotbar {
    pub items: Vec<ItemStack>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotbarLayout {
    pub background: HotbarRect,
    pub selection: HotbarRect,
    pub slots: [SlotPosition; Hotbar::SIZE],
    pub offhand_left_background: HotbarRect,
    pub offhand_left_slot: SlotPosition,
    pub offhand_right_background: HotbarRect,
    pub offhand_right_slot: SlotPosition,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HotbarRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
impl HotbarRect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}
impl Default for Hotbar {
    fn default() -> Self {
        Self::new()
    }
}
impl Hotbar {
    pub const SIZE: usize = 9;
    pub const BACKGROUND_WIDTH: i32 = 182;
    pub const BACKGROUND_HEIGHT: i32 = 22;
    pub const HALF_BACKGROUND_WIDTH: i32 = 91;
    pub const SELECTION_WIDTH: i32 = 24;
    pub const SELECTION_HEIGHT: i32 = 23;
    pub const SLOT_STRIDE: i32 = 20;
    pub const ITEM_SIZE: i32 = 16;
    pub fn new() -> Self {
        Self {
            items: vec![ItemStack::empty(); Self::SIZE],
        }
    }
    pub fn load(&self) -> Vec<ItemStack> {
        self.items.clone()
    }
    pub fn store_from(&mut self, inventory: &[ItemStack]) {
        for i in 0..Self::SIZE {
            if i < inventory.len() {
                self.items[i] = inventory[i].clone();
            } else {
                self.items[i] = ItemStack::empty();
            }
        }
    }
    pub fn is_empty(&self) -> bool {
        self.items.iter().all(|item| item.is_empty())
    }
}
impl HotbarLayout {
    pub fn for_screen(screen_width: i32, screen_height: i32, selected_slot: usize) -> Self {
        let screen_center = screen_width / 2;
        let background_x = screen_center - Hotbar::HALF_BACKGROUND_WIDTH;
        let background_y = screen_height - Hotbar::BACKGROUND_HEIGHT;
        let selected_slot = selected_slot.min(Hotbar::SIZE.saturating_sub(1));
        let slots = std::array::from_fn(|i| {
            SlotPosition::new(
                screen_center - 90 + i as i32 * Hotbar::SLOT_STRIDE + 2,
                screen_height - Hotbar::ITEM_SIZE - 3,
            )
        });
        Self {
            background: HotbarRect::new(
                background_x,
                background_y,
                Hotbar::BACKGROUND_WIDTH,
                Hotbar::BACKGROUND_HEIGHT,
            ),
            selection: HotbarRect::new(
                background_x - 1 + selected_slot as i32 * Hotbar::SLOT_STRIDE,
                background_y - 1,
                Hotbar::SELECTION_WIDTH,
                Hotbar::SELECTION_HEIGHT,
            ),
            slots,
            offhand_left_background: HotbarRect::new(background_x - 29, screen_height - 23, 29, 24),
            offhand_left_slot: SlotPosition::new(background_x - 26, screen_height - 19),
            offhand_right_background: HotbarRect::new(
                background_x + Hotbar::BACKGROUND_WIDTH,
                screen_height - 23,
                29,
                24,
            ),
            offhand_right_slot: SlotPosition::new(
                background_x + Hotbar::BACKGROUND_WIDTH + 10,
                screen_height - 19,
            ),
        }
    }
}
