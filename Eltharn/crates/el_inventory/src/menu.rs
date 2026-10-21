use crate::container::{Container, SimpleContainer};
use crate::layout::{self, SlotPosition};
use el_core::item::ItemStack;
use std::collections::HashSet;
use std::sync::LazyLock;
static EMPTY_STACK: LazyLock<ItemStack> = LazyLock::new(ItemStack::empty);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuType {
    Inventory,
    Crafting,
    Furnace,
    Chest,
    Creative,
    Chest1Row,
    Chest2Rows,
    Chest3Rows,
    Chest4Rows,
    Chest5Rows,
    Chest6Rows,
    Dispenser,
    Crafter,
    Anvil,
    Beacon,
    BrewingStand,
    Hopper,
    Lectern,
    Loom,
    Merchant,
    ShulkerBox,
    Smithing,
    Smoker,
    BlastFurnace,
    Grindstone,
    CartographyTable,
    Stonecutter,
    Enchantment,
    Mount,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotType {
    Generic,
    CraftingInput,
    CraftingResult,
    Armor,
    Offhand,
    PlayerInventory,
    Hotbar,
    Fuel,
    FurnaceResult,
    MountSaddle,
    MountArmor,
    CrafterInput,
    NonInteractive,
    SingleItem,
    Lapis,
    BeaconPayment,
    BrewingBottle,
    BrewingFuel,
    BrewingIngredient,
    LecternBook,
    Banner,
    Dye,
    BannerPattern,
    SmithingTemplate,
    MapInput,
    Stonecuttable,
    Creative,
    Trash,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    pub container_idx: usize,
    pub slot_idx: usize,
    pub slot_type: SlotType,
}
impl Slot {
    pub const fn new(container_idx: usize, slot_idx: usize, slot_type: SlotType) -> Self {
        Self {
            container_idx,
            slot_idx,
            slot_type,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClickType {
    Pickup { button: i32 },
    QuickMove,
    Swap { inventory_slot: usize },
    Clone,
    Throw { button: i32 },
    PickupAll,
}
#[derive(Clone, Debug)]
pub struct Menu {
    pub menu_type: MenuType,
    pub slots: Vec<Slot>,
    pub containers: Vec<SimpleContainer>,
    pub slot_positions: Vec<SlotPosition>,
    quickcraft_type: i32,
    quickcraft_status: i32,
    quickcraft_slots: HashSet<usize>,
    disabled_slots: HashSet<usize>,
}
impl Menu {
    pub fn new(menu_type: MenuType, slots: Vec<Slot>, containers: Vec<SimpleContainer>) -> Self {
        let slot_positions = layout::slot_positions(menu_type, slots.len());
        Self {
            menu_type,
            slots,
            containers,
            slot_positions,
            quickcraft_type: 0,
            quickcraft_status: 0,
            quickcraft_slots: HashSet::new(),
            disabled_slots: HashSet::new(),
        }
    }
    pub fn set_slot_enabled(&mut self, slot_idx: usize, enabled: bool) {
        if enabled {
            self.disabled_slots.remove(&slot_idx);
        } else {
            self.disabled_slots.insert(slot_idx);
        }
    }
    pub fn is_slot_disabled(&self, slot_idx: usize) -> bool {
        self.disabled_slots.contains(&slot_idx)
    }
    pub fn get_slot_item(&self, slot_idx: usize) -> &ItemStack {
        let Some(slot) = self.slots.get(slot_idx) else {
            return &EMPTY_STACK;
        };
        let Some(container) = self.containers.get(slot.container_idx) else {
            return &EMPTY_STACK;
        };
        container.get_item(slot.slot_idx)
    }
    pub fn set_slot_item(&mut self, slot_idx: usize, stack: ItemStack) {
        let Some(slot) = self.slots.get(slot_idx).cloned() else {
            return;
        };
        let Some(container) = self.containers.get_mut(slot.container_idx) else {
            return;
        };
        container.set_item(slot.slot_idx, stack);
    }
    pub fn slot_position(&self, slot_idx: usize) -> Option<SlotPosition> {
        self.slot_positions.get(slot_idx).copied()
    }
    pub fn click_slot(
        &mut self,
        slot_idx: usize,
        click_type: ClickType,
        carried: &mut ItemStack,
    ) -> Result<(), String> {
        if slot_idx >= self.slots.len() {
            return Err(format!("slot {} outside menu", slot_idx));
        }
        if self.menu_type == MenuType::Creative {
            match self.slots[slot_idx].slot_type {
                SlotType::Creative => {
                    return self.do_creative_pick(slot_idx, click_type, carried);
                }
                SlotType::Trash => {
                    self.do_creative_trash(click_type, carried);
                    return Ok(());
                }
                _ => {}
            }
        }
        match click_type {
            ClickType::Pickup { button } => self.do_pickup(slot_idx, button, carried),
            ClickType::QuickMove => {
                self.quick_move_stack(slot_idx);
                Ok(())
            }
            ClickType::Swap { inventory_slot } => self.do_swap(slot_idx, inventory_slot),
            ClickType::Clone => self.do_clone(slot_idx, carried),
            ClickType::Throw { button } => {
                self.throw_from_slot(slot_idx, button);
                Ok(())
            }
            ClickType::PickupAll => {
                self.pickup_all(slot_idx, carried);
                Ok(())
            }
        }
    }
    pub fn click_outside(
        &mut self,
        click_type: ClickType,
        carried: &mut ItemStack,
    ) -> Result<(), String> {
        match click_type {
            ClickType::Pickup { button } | ClickType::Throw { button } => {
                if !carried.is_empty() {
                    if button == 0 {
                        *carried = ItemStack::empty();
                    } else {
                        carried.count -= 1;
                        normalize_stack(carried);
                    }
                }
                Ok(())
            }
            ClickType::Swap { .. } | ClickType::Clone => Ok(()),
            _ => Ok(()),
        }
    }
    pub fn start_quickcraft(&mut self, drag_type: i32) {
        self.quickcraft_type = drag_type;
        self.quickcraft_status = 1;
        self.quickcraft_slots.clear();
    }
    pub fn add_quickcraft_slot(&mut self, slot_idx: usize, carried: &ItemStack) -> bool {
        if self.quickcraft_status != 1 || !self.can_drag_to(slot_idx, carried) {
            return false;
        }
        if self.quickcraft_type != 2 && carried.count <= self.quickcraft_slots.len() as i32 {
            return false;
        }
        self.quickcraft_slots.insert(slot_idx)
    }
    pub fn finish_quickcraft(&mut self, carried: &mut ItemStack) {
        if self.quickcraft_status != 1 || carried.is_empty() || self.quickcraft_slots.is_empty() {
            self.reset_quickcraft();
            return;
        }
        let slots: Vec<usize> = self.quickcraft_slots.iter().copied().collect();
        match self.quickcraft_type {
            0 => {
                let mut remaining = carried.count;
                let slot_count = slots.len() as i32;
                let per_slot = remaining / slot_count;
                if per_slot > 0 {
                    for slot_idx in slots {
                        if remaining <= 0 {
                            break;
                        }
                        let current = self.get_slot_item(slot_idx).clone();
                        let existing = if current.is_empty() { 0 } else { current.count };
                        let max = self.slot_max_stack_size(slot_idx, carried);
                        let to_add = per_slot.min(max - existing).max(0).min(remaining);
                        if to_add > 0 {
                            let mut target = if current.is_empty() {
                                carried.copy_with_count(0)
                            } else {
                                current
                            };
                            target.count = existing + to_add;
                            self.set_slot_item(slot_idx, target);
                            remaining -= to_add;
                        }
                    }
                }
                carried.count = remaining;
                normalize_stack(carried);
            }
            1 => {
                for slot_idx in slots {
                    if carried.is_empty() {
                        break;
                    }
                    let current = self.get_slot_item(slot_idx).clone();
                    let existing = if current.is_empty() { 0 } else { current.count };
                    let max = self.slot_max_stack_size(slot_idx, carried);
                    if existing < max {
                        let mut target = if current.is_empty() {
                            carried.copy_with_count(0)
                        } else {
                            current
                        };
                        target.count = existing + 1;
                        self.set_slot_item(slot_idx, target);
                        carried.count -= 1;
                        normalize_stack(carried);
                    }
                }
            }
            2 => {
                for slot_idx in slots {
                    let current = self.get_slot_item(slot_idx).clone();
                    let existing = if current.is_empty() { 0 } else { current.count };
                    let max = self.slot_max_stack_size(slot_idx, carried);
                    if existing < max {
                        let mut target = if current.is_empty() {
                            carried.copy_with_count(0)
                        } else {
                            current
                        };
                        target.count = max;
                        self.set_slot_item(slot_idx, target);
                    }
                }
            }
            _ => {}
        }
        self.reset_quickcraft();
    }
    pub fn reset_quickcraft(&mut self) {
        self.quickcraft_type = 0;
        self.quickcraft_status = 0;
        self.quickcraft_slots.clear();
    }
    pub fn quick_move_stack(&mut self, slot_idx: usize) -> ItemStack {
        let Some(slot) = self.slots.get(slot_idx).cloned() else {
            return ItemStack::empty();
        };
        if slot.slot_type == SlotType::NonInteractive {
            return ItemStack::empty();
        }
        let original = self.get_slot_item(slot_idx).clone();
        if original.is_empty() {
            return ItemStack::empty();
        }
        let mut moved = original.clone();
        let changed = match self.menu_type {
            MenuType::Inventory => self.quick_move_inventory(slot_idx, &slot, &mut moved),
            MenuType::Crafting => self.quick_move_crafting(slot_idx, &slot, &mut moved),
            MenuType::Furnace | MenuType::Smoker | MenuType::BlastFurnace => {
                self.quick_move_furnace(slot_idx, &slot, &mut moved)
            }
            MenuType::Chest
            | MenuType::Chest1Row
            | MenuType::Chest2Rows
            | MenuType::Chest3Rows
            | MenuType::Chest4Rows
            | MenuType::Chest5Rows
            | MenuType::Chest6Rows
            | MenuType::Dispenser
            | MenuType::Anvil
            | MenuType::Beacon
            | MenuType::Hopper
            | MenuType::Lectern
            | MenuType::Loom
            | MenuType::Merchant
            | MenuType::ShulkerBox
            | MenuType::Smithing
            | MenuType::Grindstone
            | MenuType::CartographyTable
            | MenuType::Stonecutter
            | MenuType::Enchantment => self.quick_move_chest(slot_idx, &mut moved),
            MenuType::Mount => self.quick_move_mount(slot_idx, &mut moved),
            MenuType::BrewingStand => self.quick_move_brewing(slot_idx, &mut moved),
            MenuType::Crafter => self.quick_move_crafter(slot_idx, &mut moved),
            MenuType::Creative => self.quick_move_creative(slot_idx, &slot, &mut moved),
        };
        if !changed {
            return ItemStack::empty();
        }
        if moved.is_empty() {
            self.set_slot_item(slot_idx, ItemStack::empty());
        } else {
            self.set_slot_item(slot_idx, moved);
        }
        original
    }
    fn do_pickup(
        &mut self,
        slot_idx: usize,
        button: i32,
        carried: &mut ItemStack,
    ) -> Result<(), String> {
        if button != 0 && button != 1 {
            return Ok(());
        }
        if !self.may_pickup(slot_idx) {
            return Ok(());
        }
        let slot_item = self.get_slot_item(slot_idx).clone();
        if slot_item.is_empty() {
            if !carried.is_empty() && self.may_place(slot_idx, carried) {
                let amount = if button == 0 { carried.count } else { 1 };
                let max = self.slot_max_stack_size(slot_idx, carried);
                let amount = amount.min(max).min(carried.count);
                if amount > 0 {
                    self.set_slot_item(slot_idx, carried.copy_with_count(amount));
                    carried.count -= amount;
                    normalize_stack(carried);
                }
            }
            return Ok(());
        }
        if carried.is_empty() {
            let amount = if button == 0 {
                slot_item.count
            } else {
                (slot_item.count + 1) / 2
            };
            let mut remainder = slot_item.clone();
            let taken = remainder.split(amount);
            self.set_slot_item(slot_idx, remainder);
            *carried = taken;
            normalize_stack(carried);
            return Ok(());
        }
        if same_item_same_components(&slot_item, carried) {
            if self.may_place(slot_idx, carried) {
                let max = self.slot_max_stack_size(slot_idx, &slot_item);
                let available = max - slot_item.count;
                let requested = if button == 0 { carried.count } else { 1 };
                let to_move = requested.min(available).min(carried.count);
                if to_move > 0 {
                    let mut merged = slot_item;
                    merged.count += to_move;
                    self.set_slot_item(slot_idx, merged);
                    carried.count -= to_move;
                    normalize_stack(carried);
                }
            }
            return Ok(());
        }
        if self.may_place(slot_idx, carried)
            && carried.count <= self.slot_max_stack_size(slot_idx, carried)
        {
            self.set_slot_item(slot_idx, carried.clone());
            *carried = slot_item;
        }
        Ok(())
    }
    fn do_creative_pick(
        &mut self,
        slot_idx: usize,
        click_type: ClickType,
        carried: &mut ItemStack,
    ) -> Result<(), String> {
        let clicked = self.get_slot_item(slot_idx).clone();
        match click_type {
            ClickType::QuickMove => {
                if !clicked.is_empty() {
                    if let Some(player_inventory) = self.containers.get_mut(0) {
                        let mut target_slot = None;
                        for hotbar_slot in 0..9 {
                            if player_inventory.get_item(hotbar_slot).is_empty() {
                                target_slot = Some(hotbar_slot);
                                break;
                            }
                        }
                        let slot_to_set = target_slot.unwrap_or(0);
                        player_inventory.set_item(
                            slot_to_set,
                            clicked.copy_with_count(clicked.get_max_stack_size()),
                        );
                    }
                }
            }
            ClickType::Clone => {
                if !clicked.is_empty() && carried.is_empty() {
                    *carried = clicked.copy_with_count(clicked.get_max_stack_size());
                }
            }
            ClickType::Pickup { button } => {
                if clicked.is_empty() {
                    if !carried.is_empty() {
                        if button == 0 {
                            *carried = ItemStack::empty();
                        } else {
                            carried.count -= 1;
                            normalize_stack(carried);
                        }
                    }
                    return Ok(());
                }
                if same_item_same_components(carried, &clicked) {
                    if button == 0 {
                        if carried.count < carried.get_max_stack_size() {
                            carried.count += 1;
                        }
                    } else {
                        carried.count -= 1;
                        normalize_stack(carried);
                    }
                } else if carried.is_empty() {
                    *carried = clicked.copy_with_count(clicked.count);
                } else if button == 0 {
                    *carried = ItemStack::empty();
                } else {
                    carried.count -= 1;
                    normalize_stack(carried);
                }
            }
            ClickType::Throw { .. } => {}
            ClickType::PickupAll => {}
            ClickType::Swap { inventory_slot } => {
                if !clicked.is_empty() && inventory_slot < 9 {
                    if let Some(player_inventory) = self.containers.get_mut(0) {
                        player_inventory.set_item(
                            inventory_slot,
                            clicked.copy_with_count(clicked.get_max_stack_size()),
                        );
                    }
                }
            }
        }
        Ok(())
    }
    fn do_creative_trash(&mut self, click_type: ClickType, carried: &mut ItemStack) {
        match click_type {
            ClickType::QuickMove => {
                if let Some(player_inventory) = self.containers.get_mut(0) {
                    player_inventory.clear_content();
                }
                *carried = ItemStack::empty();
            }
            ClickType::Pickup { .. }
            | ClickType::Swap { .. }
            | ClickType::Clone
            | ClickType::Throw { .. }
            | ClickType::PickupAll => *carried = ItemStack::empty(),
        }
    }
    fn do_swap(&mut self, slot_idx: usize, inventory_slot: usize) -> Result<(), String> {
        if slot_idx >= self.slots.len() || self.containers.is_empty() {
            return Ok(());
        }
        let target_slot = self.slots[slot_idx].clone();
        if target_slot.container_idx == 0 && target_slot.slot_idx == inventory_slot {
            return Ok(());
        }
        let source = self.containers[0].get_item(inventory_slot).clone();
        let target = self.get_slot_item(slot_idx).clone();
        if source.is_empty() && target.is_empty() {
            return Ok(());
        }
        if source.is_empty() {
            if self.may_pickup(slot_idx) {
                self.containers[0].set_item(inventory_slot, target);
                self.set_slot_item(slot_idx, ItemStack::empty());
            }
            return Ok(());
        }
        if target.is_empty() {
            if self.may_place(slot_idx, &source) {
                let max = self.slot_max_stack_size(slot_idx, &source);
                if source.count > max {
                    self.set_slot_item(slot_idx, source.copy_with_count(max));
                    self.containers[0].set_item(
                        inventory_slot,
                        source.copy_with_count(source.count.saturating_sub(max)),
                    );
                } else {
                    self.set_slot_item(slot_idx, source);
                    self.containers[0].set_item(inventory_slot, ItemStack::empty());
                }
            }
            return Ok(());
        }
        if self.may_pickup(slot_idx) && self.may_place(slot_idx, &source) {
            let max = self.slot_max_stack_size(slot_idx, &source);
            if source.count <= max {
                self.containers[0].set_item(inventory_slot, target);
                self.set_slot_item(slot_idx, source);
            } else {
                let mut remainder = source.clone();
                let moved = remainder.split(max);
                self.set_slot_item(slot_idx, moved);
                self.containers[0].set_item(inventory_slot, remainder);
                let mut displaced = target;
                let _ = self.move_item_stack_to(&mut displaced, 9, 45, false);
            }
        }
        Ok(())
    }
    fn do_clone(&mut self, slot_idx: usize, carried: &mut ItemStack) -> Result<(), String> {
        if !carried.is_empty() {
            return Ok(());
        }
        let clicked = self.get_slot_item(slot_idx).clone();
        if !clicked.is_empty() {
            *carried = clicked.copy_with_count(clicked.get_max_stack_size());
        }
        Ok(())
    }
    fn throw_from_slot(&mut self, slot_idx: usize, button: i32) {
        let current = self.get_slot_item(slot_idx).clone();
        if current.is_empty() || !self.may_pickup(slot_idx) {
            return;
        }
        if button == 0 {
            let mut remaining = current;
            remaining.count -= 1;
            normalize_stack(&mut remaining);
            self.set_slot_item(slot_idx, remaining);
        } else {
            self.set_slot_item(slot_idx, ItemStack::empty());
        }
    }
    fn pickup_all(&mut self, _slot_idx: usize, carried: &mut ItemStack) {
        if carried.is_empty() || carried.count >= carried.get_max_stack_size() {
            return;
        }
        for idx in (0..self.slots.len()).rev() {
            if carried.count >= carried.get_max_stack_size() {
                break;
            }
            if !self.may_pickup(idx) {
                continue;
            }
            let stack = self.get_slot_item(idx).clone();
            if !same_item_same_components(&stack, carried) {
                continue;
            }
            let needed = carried.get_max_stack_size() - carried.count;
            let moved = needed.min(stack.count);
            if moved <= 0 {
                continue;
            }
            carried.count += moved;
            let mut remaining = stack;
            remaining.count -= moved;
            self.set_slot_item(idx, remaining);
        }
        normalize_stack(carried);
    }
    fn quick_move_inventory(
        &mut self,
        slot_idx: usize,
        slot: &Slot,
        stack: &mut ItemStack,
    ) -> bool {
        if slot_idx == 0 {
            return self.move_item_stack_to(stack, 9, 45, true);
        }
        match slot.slot_type {
            SlotType::CraftingInput | SlotType::CraftingResult | SlotType::Armor => {
                self.move_item_stack_to(stack, 9, 45, false)
            }
            SlotType::PlayerInventory => self.move_item_stack_to(stack, 36, 45, false),
            SlotType::Hotbar => self.move_item_stack_to(stack, 9, 36, false),
            SlotType::Offhand => self.move_item_stack_to(stack, 9, 45, false),
            _ => self.move_item_stack_to(stack, 9, 45, false),
        }
    }
    fn quick_move_crafting(
        &mut self,
        slot_idx: usize,
        _slot: &Slot,
        stack: &mut ItemStack,
    ) -> bool {
        if slot_idx == 0 {
            return self.move_item_stack_to(stack, 10, 46, true);
        }
        if (1..10).contains(&slot_idx) {
            return self.move_item_stack_to(stack, 10, 46, false);
        }
        if (10..37).contains(&slot_idx) {
            return self.move_item_stack_to(stack, 37, 46, false);
        }
        if (37..46).contains(&slot_idx) {
            return self.move_item_stack_to(stack, 10, 37, false);
        }
        false
    }
    fn quick_move_furnace(&mut self, slot_idx: usize, _slot: &Slot, stack: &mut ItemStack) -> bool {
        if slot_idx == 2 {
            return self.move_item_stack_to(stack, 3, 39, true);
        }
        if (3..30).contains(&slot_idx) {
            return self.move_item_stack_to(stack, 30, 39, false);
        }
        if (30..39).contains(&slot_idx) {
            return self.move_item_stack_to(stack, 3, 30, false);
        }
        self.move_item_stack_to(stack, 3, 39, false)
    }
    fn quick_move_chest(&mut self, slot_idx: usize, stack: &mut ItemStack) -> bool {
        let chest_slots = self.slots.len().saturating_sub(36);
        if slot_idx < chest_slots {
            self.move_item_stack_to(stack, chest_slots, self.slots.len(), true)
        } else {
            self.move_item_stack_to(stack, 0, chest_slots, false)
        }
    }
    fn quick_move_mount(&mut self, slot_idx: usize, stack: &mut ItemStack) -> bool {
        let player_start = self.slots.len().saturating_sub(36);
        if slot_idx < player_start {
            return self.move_item_stack_to(stack, player_start, self.slots.len(), true);
        }
        if self.get_slot_item(1).is_empty()
            && self.may_place(1, stack)
            && self.move_item_stack_to(stack, 1, 2, false)
        {
            return true;
        }
        if self.get_slot_item(0).is_empty()
            && self.may_place(0, stack)
            && self.move_item_stack_to(stack, 0, 1, false)
        {
            return true;
        }
        if player_start > 2 && self.move_item_stack_to(stack, 2, player_start, false) {
            return true;
        }
        let player_inventory_end = player_start + 27;
        if slot_idx >= player_inventory_end {
            self.move_item_stack_to(stack, player_start, player_inventory_end, false)
        } else {
            self.move_item_stack_to(stack, player_inventory_end, self.slots.len(), false)
        }
    }
    fn quick_move_crafter(&mut self, slot_idx: usize, stack: &mut ItemStack) -> bool {
        if slot_idx < 9 {
            self.move_item_stack_to(stack, 9, 45, true)
        } else {
            self.move_item_stack_to(stack, 0, 9, false)
        }
    }
    fn quick_move_brewing(&mut self, slot_idx: usize, stack: &mut ItemStack) -> bool {
        if slot_idx < 5 {
            return self.move_item_stack_to(stack, 5, self.slots.len(), true);
        }
        if is_vanilla_brewing_fuel(stack)
            && self.get_slot_item(4).is_empty()
            && self.move_item_stack_to(stack, 4, 5, false)
        {
            return true;
        }
        if is_vanilla_brewing_ingredient(stack) && self.move_item_stack_to(stack, 3, 4, false) {
            return true;
        }
        if is_vanilla_brewing_bottle(stack) && self.move_item_stack_to(stack, 0, 3, false) {
            return true;
        }
        let main_inventory_end = 5 + 27;
        if slot_idx >= main_inventory_end {
            self.move_item_stack_to(stack, 5, main_inventory_end, false)
        } else {
            self.move_item_stack_to(stack, main_inventory_end, self.slots.len(), false)
        }
    }
    fn quick_move_creative(&mut self, slot_idx: usize, slot: &Slot, stack: &mut ItemStack) -> bool {
        if self
            .slots
            .iter()
            .any(|s| s.slot_type == SlotType::PlayerInventory)
        {
            return self.quick_move_inventory(slot_idx, slot, stack);
        }
        if slot.slot_type == SlotType::Creative {
            *stack = ItemStack::empty();
            return true;
        }
        if slot.slot_type == SlotType::Hotbar || (45..54).contains(&slot_idx) {
            self.set_slot_item(slot_idx, ItemStack::empty());
            *stack = ItemStack::empty();
            return true;
        }
        false
    }
    pub fn move_item_stack_to(
        &mut self,
        stack: &mut ItemStack,
        start_index: usize,
        end_index: usize,
        reverse_direction: bool,
    ) -> bool {
        if stack.is_empty() || start_index >= end_index || start_index >= self.slots.len() {
            return false;
        }
        let end_index = end_index.min(self.slots.len());
        let mut changed = false;
        if stack.is_stackable() {
            let mut index = if reverse_direction {
                end_index.saturating_sub(1)
            } else {
                start_index
            };
            while !stack.is_empty() && in_range(index, start_index, end_index) {
                if self.may_place(index, stack) {
                    let existing = self.get_slot_item(index).clone();
                    if same_item_same_components(&existing, stack) {
                        let max = self.slot_max_stack_size(index, &existing);
                        let total = existing.count + stack.count;
                        if total <= max {
                            let mut new_stack = existing;
                            new_stack.count = total;
                            self.set_slot_item(index, new_stack);
                            stack.count = 0;
                            changed = true;
                        } else if existing.count < max {
                            let move_count = max - existing.count;
                            let mut new_stack = existing;
                            new_stack.count = max;
                            self.set_slot_item(index, new_stack);
                            stack.count -= move_count;
                            changed = true;
                        }
                        normalize_stack(stack);
                    }
                }
                if reverse_direction {
                    if index == 0 {
                        break;
                    }
                    index -= 1;
                } else {
                    index += 1;
                }
            }
        }
        if !stack.is_empty() {
            let mut index = if reverse_direction {
                end_index.saturating_sub(1)
            } else {
                start_index
            };
            while in_range(index, start_index, end_index) {
                if self.may_place(index, stack) && self.get_slot_item(index).is_empty() {
                    let max = self.slot_max_stack_size(index, stack);
                    let move_count = stack.count.min(max);
                    self.set_slot_item(index, stack.copy_with_count(move_count));
                    stack.count -= move_count;
                    normalize_stack(stack);
                    changed = true;
                    break;
                }
                if reverse_direction {
                    if index == 0 {
                        break;
                    }
                    index -= 1;
                } else {
                    index += 1;
                }
            }
        }
        changed
    }
    fn can_drag_to(&self, slot_idx: usize, carried: &ItemStack) -> bool {
        if slot_idx >= self.slots.len() || carried.is_empty() {
            return false;
        }
        if !self.may_place(slot_idx, carried) {
            return false;
        }
        let stack = self.get_slot_item(slot_idx);
        stack.is_empty() || same_item_same_components(stack, carried)
    }
    fn may_place(&self, slot_idx: usize, stack: &ItemStack) -> bool {
        let Some(slot) = self.slots.get(slot_idx) else {
            return false;
        };
        if matches!(
            slot.slot_type,
            SlotType::CraftingResult
                | SlotType::FurnaceResult
                | SlotType::NonInteractive
                | SlotType::Creative
                | SlotType::Trash
        ) {
            return false;
        }
        if slot.slot_type == SlotType::CrafterInput && self.is_slot_disabled(slot_idx) {
            return false;
        }
        if slot.slot_type == SlotType::MountSaddle {
            return item_path(stack) == Some("saddle");
        }
        if slot.slot_type == SlotType::MountArmor {
            return item_path(stack).is_some_and(|path| {
                path.ends_with("_horse_armor")
                    || path.ends_with("_nautilus_armor")
                    || path == "turtle_scute"
            });
        }
        if slot.slot_type == SlotType::BeaconPayment {
            return matches!(
                item_path(stack),
                Some("netherite_ingot" | "emerald" | "diamond" | "gold_ingot" | "iron_ingot")
            );
        }
        if slot.slot_type == SlotType::Lapis {
            return item_path(stack) == Some("lapis_lazuli");
        }
        if slot.slot_type == SlotType::BrewingBottle {
            return is_vanilla_brewing_bottle(stack);
        }
        if slot.slot_type == SlotType::BrewingFuel {
            return is_vanilla_brewing_fuel(stack);
        }
        if slot.slot_type == SlotType::BrewingIngredient {
            return is_vanilla_brewing_ingredient(stack);
        }
        if slot.slot_type == SlotType::LecternBook {
            return matches!(item_path(stack), Some("written_book" | "writable_book"));
        }
        if slot.slot_type == SlotType::Banner {
            return item_path(stack).is_some_and(|path| path.ends_with("_banner"));
        }
        if slot.slot_type == SlotType::Dye {
            return item_path(stack).is_some_and(|path| path.ends_with("_dye"));
        }
        if slot.slot_type == SlotType::BannerPattern {
            return matches!(
                item_path(stack),
                Some(
                    "flower_banner_pattern"
                        | "creeper_banner_pattern"
                        | "skull_banner_pattern"
                        | "mojang_banner_pattern"
                        | "globe_banner_pattern"
                        | "piglin_banner_pattern"
                        | "flow_banner_pattern"
                        | "guster_banner_pattern"
                        | "field_masoned_banner_pattern"
                        | "bordure_indented_banner_pattern"
                )
            );
        }
        if slot.slot_type == SlotType::SmithingTemplate {
            return item_path(stack).is_some_and(|path| path.ends_with("_smithing_template"));
        }
        if slot.slot_type == SlotType::MapInput {
            return if slot.slot_idx == 0 {
                stack.components.get("minecraft:map_id").is_some()
            } else {
                matches!(item_path(stack), Some("paper" | "map" | "glass_pane"))
            };
        }
        let Some(container) = self.containers.get(slot.container_idx) else {
            return false;
        };
        container.can_place_item(slot.slot_idx, stack)
    }
    fn may_pickup(&self, slot_idx: usize) -> bool {
        self.slots
            .get(slot_idx)
            .is_some_and(|slot| slot.slot_type != SlotType::NonInteractive)
    }
    fn slot_max_stack_size(&self, slot_idx: usize, stack: &ItemStack) -> i32 {
        let Some(slot) = self.slots.get(slot_idx) else {
            return stack.get_max_stack_size();
        };
        if matches!(
            slot.slot_type,
            SlotType::MountSaddle
                | SlotType::MountArmor
                | SlotType::BeaconPayment
                | SlotType::BrewingBottle
                | SlotType::SingleItem
                | SlotType::NonInteractive
        ) {
            return 1;
        }
        let container_max = self
            .containers
            .get(slot.container_idx)
            .map(|container| container.get_max_stack_size_for(stack))
            .unwrap_or_else(|| stack.get_max_stack_size());
        container_max.min(stack.get_max_stack_size())
    }
}
pub fn new_inventory_menu(player_inventory: SimpleContainer) -> Menu {
    let mut slots = Vec::with_capacity(46);
    slots.push(Slot::new(1, 0, SlotType::CraftingResult));
    for slot_idx in 0..4 {
        slots.push(Slot::new(2, slot_idx, SlotType::CraftingInput));
    }
    for slot_idx in [39, 38, 37, 36] {
        slots.push(Slot::new(0, slot_idx, SlotType::Armor));
    }
    add_player_inventory_slots(&mut slots, 0);
    slots.push(Slot::new(0, 40, SlotType::Offhand));
    Menu::new(
        MenuType::Inventory,
        slots,
        vec![
            player_inventory,
            SimpleContainer::new(1),
            SimpleContainer::new(4),
        ],
    )
}
pub fn new_crafting_menu(player_inventory: SimpleContainer) -> Menu {
    let mut slots = Vec::with_capacity(46);
    slots.push(Slot::new(1, 0, SlotType::CraftingResult));
    for slot_idx in 0..9 {
        slots.push(Slot::new(2, slot_idx, SlotType::CraftingInput));
    }
    add_player_inventory_slots(&mut slots, 0);
    Menu::new(
        MenuType::Crafting,
        slots,
        vec![
            player_inventory,
            SimpleContainer::new(1),
            SimpleContainer::new(9),
        ],
    )
}
pub fn new_furnace_menu(player_inventory: SimpleContainer, furnace: SimpleContainer) -> Menu {
    let mut slots = Vec::with_capacity(39);
    slots.push(Slot::new(1, 0, SlotType::Generic));
    slots.push(Slot::new(1, 1, SlotType::Fuel));
    slots.push(Slot::new(1, 2, SlotType::CraftingResult));
    add_player_inventory_slots(&mut slots, 0);
    Menu::new(MenuType::Furnace, slots, vec![player_inventory, furnace])
}
pub fn new_chest_menu(player_inventory: SimpleContainer, chest: SimpleContainer) -> Menu {
    let chest_size = chest.get_container_size();
    let mut slots = Vec::with_capacity(chest_size + 36);
    for slot_idx in 0..chest_size {
        slots.push(Slot::new(1, slot_idx, SlotType::Generic));
    }
    add_player_inventory_slots(&mut slots, 0);
    Menu::new(MenuType::Chest, slots, vec![player_inventory, chest])
}
pub fn new_generic_menu(
    player_inventory: SimpleContainer,
    container_size: usize,
    menu_type: MenuType,
) -> Menu {
    let mut slots = Vec::with_capacity(container_size + 36);
    for slot_idx in 0..container_size {
        slots.push(Slot::new(1, slot_idx, SlotType::Generic));
    }
    add_player_inventory_slots(&mut slots, 0);
    Menu::new(
        menu_type,
        slots,
        vec![player_inventory, SimpleContainer::new(container_size)],
    )
}
pub fn new_mount_menu(player_inventory: SimpleContainer, columns: usize) -> Menu {
    let mut slots = Vec::with_capacity(38 + columns * 3);
    slots.push(Slot::new(1, 0, SlotType::MountSaddle));
    slots.push(Slot::new(1, 1, SlotType::MountArmor));
    for slot_idx in 0..columns.saturating_mul(3) {
        slots.push(Slot::new(1, slot_idx + 2, SlotType::Generic));
    }
    add_player_inventory_slots(&mut slots, 0);
    Menu::new(
        MenuType::Mount,
        slots,
        vec![player_inventory, SimpleContainer::new(2 + columns * 3)],
    )
}
pub fn new_crafter_menu(player_inventory: SimpleContainer) -> Menu {
    let mut slots = Vec::with_capacity(46);
    for slot_idx in 0..9 {
        slots.push(Slot::new(1, slot_idx, SlotType::CrafterInput));
    }
    add_player_inventory_slots(&mut slots, 0);
    slots.push(Slot::new(2, 0, SlotType::NonInteractive));
    Menu::new(
        MenuType::Crafter,
        slots,
        vec![
            player_inventory,
            SimpleContainer::new(9),
            SimpleContainer::new(1),
        ],
    )
}
fn item_path(stack: &ItemStack) -> Option<&str> {
    (!stack.is_empty() && stack.item.location.namespace == "minecraft")
        .then_some(stack.item.location.path.as_str())
}
fn is_vanilla_brewing_bottle(stack: &ItemStack) -> bool {
    matches!(
        item_path(stack),
        Some("potion" | "splash_potion" | "lingering_potion" | "glass_bottle")
    )
}
fn is_vanilla_brewing_fuel(stack: &ItemStack) -> bool {
    item_path(stack) == Some("blaze_powder")
}
fn is_vanilla_brewing_ingredient(stack: &ItemStack) -> bool {
    matches!(
        item_path(stack),
        Some(
            "glowstone_dust"
                | "redstone"
                | "nether_wart"
                | "breeze_rod"
                | "slime_block"
                | "stone"
                | "cobweb"
                | "gunpowder"
                | "dragon_breath"
                | "golden_carrot"
                | "fermented_spider_eye"
                | "magma_cream"
                | "rabbit_foot"
                | "turtle_helmet"
                | "pufferfish"
                | "glistering_melon_slice"
                | "spider_eye"
                | "ghast_tear"
                | "blaze_powder"
                | "sugar"
                | "phantom_membrane"
        )
    )
}
pub fn new_dispenser_menu(player_inventory: SimpleContainer) -> Menu {
    new_generic_menu(player_inventory, 9, MenuType::Dispenser)
}
pub fn new_anvil_menu(player_inventory: SimpleContainer) -> Menu {
    new_special_menu(
        player_inventory,
        MenuType::Anvil,
        &[
            SlotType::Generic,
            SlotType::Generic,
            SlotType::CraftingResult,
        ],
    )
}
pub fn new_beacon_menu(player_inventory: SimpleContainer) -> Menu {
    new_special_menu(
        player_inventory,
        MenuType::Beacon,
        &[SlotType::BeaconPayment],
    )
}
pub fn new_brewing_stand_menu(player_inventory: SimpleContainer) -> Menu {
    new_special_menu(
        player_inventory,
        MenuType::BrewingStand,
        &[
            SlotType::BrewingBottle,
            SlotType::BrewingBottle,
            SlotType::BrewingBottle,
            SlotType::BrewingIngredient,
            SlotType::BrewingFuel,
        ],
    )
}
pub fn new_hopper_menu(player_inventory: SimpleContainer) -> Menu {
    new_generic_menu(player_inventory, 5, MenuType::Hopper)
}
pub fn new_lectern_menu(player_inventory: SimpleContainer) -> Menu {
    new_special_menu(
        player_inventory,
        MenuType::Lectern,
        &[SlotType::LecternBook],
    )
}
pub fn new_loom_menu(player_inventory: SimpleContainer) -> Menu {
    new_special_menu(
        player_inventory,
        MenuType::Loom,
        &[
            SlotType::Banner,
            SlotType::Dye,
            SlotType::BannerPattern,
            SlotType::CraftingResult,
        ],
    )
}
pub fn new_merchant_menu(player_inventory: SimpleContainer) -> Menu {
    new_special_menu(
        player_inventory,
        MenuType::Merchant,
        &[
            SlotType::Generic,
            SlotType::Generic,
            SlotType::CraftingResult,
        ],
    )
}
pub fn new_shulker_box_menu(player_inventory: SimpleContainer) -> Menu {
    new_generic_menu(player_inventory, 27, MenuType::ShulkerBox)
}
pub fn new_smithing_menu(player_inventory: SimpleContainer) -> Menu {
    new_special_menu(
        player_inventory,
        MenuType::Smithing,
        &[
            SlotType::SmithingTemplate,
            SlotType::Generic,
            SlotType::Generic,
            SlotType::CraftingResult,
        ],
    )
}
pub fn new_grindstone_menu(player_inventory: SimpleContainer) -> Menu {
    new_special_menu(
        player_inventory,
        MenuType::Grindstone,
        &[
            SlotType::Generic,
            SlotType::Generic,
            SlotType::CraftingResult,
        ],
    )
}
pub fn new_cartography_table_menu(player_inventory: SimpleContainer) -> Menu {
    new_special_menu(
        player_inventory,
        MenuType::CartographyTable,
        &[
            SlotType::MapInput,
            SlotType::Generic,
            SlotType::CraftingResult,
        ],
    )
}
pub fn new_stonecutter_menu(player_inventory: SimpleContainer) -> Menu {
    new_special_menu(
        player_inventory,
        MenuType::Stonecutter,
        &[SlotType::Stonecuttable, SlotType::CraftingResult],
    )
}
pub fn new_enchantment_menu(player_inventory: SimpleContainer) -> Menu {
    new_special_menu(
        player_inventory,
        MenuType::Enchantment,
        &[SlotType::SingleItem, SlotType::Lapis],
    )
}
fn new_special_menu(
    player_inventory: SimpleContainer,
    menu_type: MenuType,
    upper_slots: &[SlotType],
) -> Menu {
    let mut slots = Vec::with_capacity(upper_slots.len() + 36);
    for (slot_idx, slot_type) in upper_slots.iter().copied().enumerate() {
        slots.push(Slot::new(1, slot_idx, slot_type));
    }
    add_player_inventory_slots(&mut slots, 0);
    Menu::new(
        menu_type,
        slots,
        vec![player_inventory, SimpleContainer::new(upper_slots.len())],
    )
}
pub fn new_creative_menu(player_inventory: SimpleContainer) -> Menu {
    let mut menu = Menu::new(
        MenuType::Creative,
        Vec::new(),
        vec![player_inventory, SimpleContainer::new(45)],
    );
    let creative = crate::creative::CreativeInventoryState::new(false, false);
    creative.apply_to_menu(&mut menu);
    menu
}
pub fn fill_creative_grid(menu: &mut Menu, scroll_row: i32) {
    if menu.menu_type != MenuType::Creative || menu.containers.len() < 2 {
        return;
    }
    let mut creative = crate::creative::CreativeInventoryState::new(false, false);
    creative.scroll_offs = creative.scroll_for_row_index(scroll_row.max(0));
    creative.apply_to_menu(menu);
}
fn add_player_inventory_slots(slots: &mut Vec<Slot>, container_idx: usize) {
    for slot_idx in 9..36 {
        slots.push(Slot::new(
            container_idx,
            slot_idx,
            SlotType::PlayerInventory,
        ));
    }
    for slot_idx in 0..9 {
        slots.push(Slot::new(container_idx, slot_idx, SlotType::Hotbar));
    }
}
fn in_range(index: usize, start_index: usize, end_index: usize) -> bool {
    index >= start_index && index < end_index
}
fn same_item_same_components(left: &ItemStack, right: &ItemStack) -> bool {
    !left.is_empty()
        && !right.is_empty()
        && left.item.location == right.item.location
        && left.components == right.components
}
fn normalize_stack(stack: &mut ItemStack) {
    if stack.count <= 0 || stack.item.location.path == "air" {
        *stack = ItemStack::empty();
    }
}
