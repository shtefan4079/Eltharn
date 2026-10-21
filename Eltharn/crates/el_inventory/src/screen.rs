use crate::layout::ScreenLayout;
use crate::menu::{ClickType, Menu, MenuType, fill_creative_grid};
use el_core::item::ItemStack;
pub const SLOT_CLICKED_OUTSIDE: i16 = -999;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Left,
    Right,
    Middle,
}
impl PointerButton {
    pub const fn pickup_button(self) -> i32 {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Middle => 2,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenClick {
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub button: PointerButton,
    pub shift_down: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SlotChange {
    pub slot_idx: usize,
    pub before: ItemStack,
    pub after: ItemStack,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ScreenClickOutcome {
    pub slot_num: i16,
    pub click_type: ClickType,
    pub carried_item: ItemStack,
    pub changed_slots: Vec<SlotChange>,
}
#[derive(Clone, Debug)]
pub struct InventoryScreenEngine {
    pub container_id: i32,
    pub menu_type: MenuType,
    pub title: String,
    pub menu: Menu,
    pub state_id: i32,
    pub carried_item: ItemStack,
    pub layout: ScreenLayout,
    pub left_pos: i32,
    pub top_pos: i32,
    pub hovered_slot_idx: Option<usize>,
    pub creative_scroll_row: i32,
}
impl InventoryScreenEngine {
    pub fn new(
        container_id: i32,
        menu_type: MenuType,
        title: String,
        menu: Menu,
        state_id: i32,
        carried_item: ItemStack,
    ) -> Self {
        let layout = ScreenLayout::for_menu(menu_type, menu.slots.len());
        Self {
            container_id,
            menu_type,
            title,
            menu,
            state_id,
            carried_item,
            layout,
            left_pos: 0,
            top_pos: 0,
            hovered_slot_idx: None,
            creative_scroll_row: 0,
        }
    }
    pub fn update_layout(&mut self, logical_width: i32, logical_height: i32) {
        self.layout = ScreenLayout::for_menu(self.menu_type, self.menu.slots.len());
        self.left_pos = (logical_width - self.layout.image_width) / 2;
        self.top_pos = (logical_height - self.layout.image_height) / 2;
    }
    pub fn handle_mouse_move(&mut self, mouse_x: i32, mouse_y: i32) {
        self.hovered_slot_idx = self.get_slot_at(mouse_x, mouse_y);
    }
    pub fn get_slot_at(&self, mouse_x: i32, mouse_y: i32) -> Option<usize> {
        let mx = mouse_x - self.left_pos;
        let my = mouse_y - self.top_pos;
        self.layout
            .slot_positions
            .iter()
            .enumerate()
            .find_map(|(idx, pos)| {
                if mx >= pos.x - 1 && mx < pos.x + 17 && my >= pos.y - 1 && my < pos.y + 17 {
                    Some(idx)
                } else {
                    None
                }
            })
    }
    pub fn has_clicked_outside(&self, mouse_x: i32, mouse_y: i32) -> bool {
        mouse_x < self.left_pos
            || mouse_y < self.top_pos
            || mouse_x >= self.left_pos + self.layout.image_width
            || mouse_y >= self.top_pos + self.layout.image_height
    }
    pub fn click(&mut self, click: ScreenClick) -> Option<ScreenClickOutcome> {
        let slot_idx = self.get_slot_at(click.mouse_x, click.mouse_y);
        let clicked_outside = self.has_clicked_outside(click.mouse_x, click.mouse_y);
        let click_type = if click.shift_down && slot_idx.is_some() {
            ClickType::QuickMove
        } else {
            ClickType::Pickup {
                button: click.button.pickup_button(),
            }
        };
        let before = self.snapshot_slots();
        match slot_idx {
            Some(slot_idx) => {
                self.menu
                    .click_slot(slot_idx, click_type, &mut self.carried_item)
                    .ok()?;
                Some(ScreenClickOutcome {
                    slot_num: slot_idx as i16,
                    click_type,
                    carried_item: self.carried_item.clone(),
                    changed_slots: self.changed_slots(before),
                })
            }
            None if clicked_outside => {
                self.menu
                    .click_outside(click_type, &mut self.carried_item)
                    .ok()?;
                Some(ScreenClickOutcome {
                    slot_num: SLOT_CLICKED_OUTSIDE,
                    click_type,
                    carried_item: self.carried_item.clone(),
                    changed_slots: self.changed_slots(before),
                })
            }
            None => None,
        }
    }
    pub fn mouse_scrolled(&mut self, scroll_y: f32) {
        if self.menu_type != MenuType::Creative {
            return;
        }
        let max_scroll_row = self.creative_max_scroll_row();
        if scroll_y > 0.0 {
            self.creative_scroll_row = (self.creative_scroll_row - 1).max(0);
        } else if scroll_y < 0.0 {
            self.creative_scroll_row = (self.creative_scroll_row + 1).min(max_scroll_row);
        }
        fill_creative_grid(&mut self.menu, self.creative_scroll_row);
    }
    pub fn creative_max_scroll_row(&self) -> i32 {
        let items = el_core::registry::BuiltInRegistries::items();
        let mut real_items_count = 0;
        for id in 0..items.size() {
            if items.get_by_id(id).location.path != "air" {
                real_items_count += 1;
            }
        }
        let total_rows = (real_items_count + 8) / 9;
        (total_rows - 5).max(0)
    }
    fn snapshot_slots(&self) -> Vec<ItemStack> {
        (0..self.menu.slots.len())
            .map(|idx| self.menu.get_slot_item(idx).clone())
            .collect()
    }
    fn changed_slots(&self, before: Vec<ItemStack>) -> Vec<SlotChange> {
        before
            .into_iter()
            .enumerate()
            .filter_map(|(slot_idx, before)| {
                let after = self.menu.get_slot_item(slot_idx).clone();
                if before == after {
                    None
                } else {
                    Some(SlotChange {
                        slot_idx,
                        before,
                        after,
                    })
                }
            })
            .collect()
    }
}
