use super::ClientSession;
use crate::network::backend_mappers::map_backend_item_stack;
use el_core::ItemStack;
use el_inventory::menu::Slot;
use el_protocol_backend::BackendNetworkEvent;

fn trace_authoritative_item_quantity(slot: usize, previous: &ItemStack, item: &ItemStack) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static EMITTED: AtomicUsize = AtomicUsize::new(0);
    if !el_protocol_backend::diagnostics::file_enabled()
        || item.is_empty()
        || (previous.item.location == item.item.location && previous.count == item.count)
        || EMITTED.load(Ordering::Relaxed) >= 256
        || EMITTED.fetch_add(1, Ordering::Relaxed) >= 256
    {
        return;
    }
    el_protocol_backend::diagnostics::write_line(&format!(
        "[ItemQuantity] stage=authoritative_player_slot slot={slot} item={} count={} previous_count={}",
        item.item.location, item.count, previous.count,
    ));
}

#[derive(Clone, Copy)]
pub struct InventoryMenuContext<'a> {
    pub container_id: i32,
    pub creative: bool,
    pub slots: &'a [Slot],
}

#[derive(Default, Debug)]
pub struct InventoryUiEffects {
    pub state_id: Option<i32>,
    pub carried_item: Option<ItemStack>,
    pub slots: Vec<(usize, ItemStack)>,
    pub refresh_creative_hotbar: bool,
}

pub fn inventory_menu_slot_to_player_slot(menu_slot: usize) -> Option<usize> {
    match menu_slot {
        0 => Some(45),
        1..=4 => Some(40 + menu_slot),
        5..=8 => Some(44 - menu_slot),
        9..=35 => Some(menu_slot),
        36..=44 => Some(menu_slot - 36),
        45 => Some(40),
        _ => None,
    }
}

pub fn player_slot_to_inventory_menu_slot(player_slot: usize) -> Option<usize> {
    match player_slot {
        45 => Some(0),
        41..=44 => Some(player_slot - 40),
        36..=39 => Some(44 - player_slot),
        9..=35 => Some(player_slot),
        0..=8 => Some(player_slot + 36),
        40 => Some(45),
        _ => None,
    }
}

impl ClientSession {
    pub fn apply_inventory_event(
        &mut self,
        event: BackendNetworkEvent,
        menu: Option<InventoryMenuContext<'_>>,
    ) -> Result<InventoryUiEffects, BackendNetworkEvent> {
        let mut effects = InventoryUiEffects::default();
        match event {
            BackendNetworkEvent::InventoryContent { items } => {
                self.apply_inventory_menu_items(
                    0,
                    items.into_iter().map(map_backend_item_stack),
                    menu,
                    &mut effects,
                );
            }
            BackendNetworkEvent::InventorySlot {
                slot,
                item,
                direct_player_slot,
            } => {
                let slot = slot as usize;
                let item = map_backend_item_stack(item);
                let player_slot = if direct_player_slot {
                    Some(slot)
                } else {
                    inventory_menu_slot_to_player_slot(slot)
                };
                if let Some(active) = menu.filter(|active| active.container_id == 0) {
                    let menu_slot = if direct_player_slot {
                        player_slot.and_then(player_slot_to_inventory_menu_slot)
                    } else {
                        Some(slot)
                    };
                    if let Some(menu_slot) = menu_slot {
                        if active.creative {
                            effects.refresh_creative_hotbar = true;
                        } else {
                            effects.slots.push((menu_slot, item.clone()));
                        }
                    }
                }
                if let Some(player_slot) = player_slot {
                    self.apply_inventory_player_slot(player_slot, item, menu, &mut effects);
                }
            }
            BackendNetworkEvent::ContainerContent {
                container_id,
                state_id,
                items,
                carried_item,
            } => {
                let carried_item = map_backend_item_stack(carried_item);
                if crate::gameplay::player::LocalPlayer::movement_diagnostics_enabled() {
                    println!(
                        "[MenuDebug] CLIENT_CONTAINER_CONTENT_EVENT container={} state_id={} items={} carried={}x{}",
                        container_id,
                        state_id,
                        items.len(),
                        carried_item.item.location,
                        carried_item.count
                    );
                }
                if container_id == 0 {
                    self.player_inventory_state_id = state_id as i32;
                    self.player_inventory_carried_item = carried_item.clone();
                }
                if menu.is_some_and(|active| active.container_id == container_id) {
                    effects.state_id = Some(state_id as i32);
                    effects.carried_item = Some(carried_item);
                }
                if let Some(active) = menu.filter(|active| active.container_id == container_id) {
                    if crate::gameplay::player::LocalPlayer::movement_diagnostics_enabled() {
                        println!(
                            "[MenuDebug] CLIENT_CONTAINER_CONTENT_APPLY matched_screen=true container={} menu_slots={} incoming_items={}",
                            container_id,
                            active.slots.len(),
                            items.len()
                        );
                    }
                } else if container_id == 0 {
                    if crate::gameplay::player::LocalPlayer::movement_diagnostics_enabled() {
                        println!(
                            "[MenuDebug] CLIENT_CONTAINER_CONTENT_APPLY matched_screen=false container=0 fallback=player_inventory incoming_items={}",
                            items.len()
                        );
                    }
                } else if crate::gameplay::player::LocalPlayer::movement_diagnostics_enabled() {
                    println!(
                        "[MenuDebug] CLIENT_CONTAINER_CONTENT_APPLY matched_screen=false container={} ignored_non_player incoming_items={}",
                        container_id,
                        items.len()
                    );
                }
                self.apply_inventory_menu_items(
                    container_id,
                    items.into_iter().map(map_backend_item_stack),
                    menu,
                    &mut effects,
                );
            }
            BackendNetworkEvent::ContainerSlot {
                container_id,
                state_id,
                slot,
                item,
            } => {
                let item = map_backend_item_stack(item);
                if crate::gameplay::player::LocalPlayer::movement_diagnostics_enabled() {
                    println!(
                        "[MenuDebug] CLIENT_CONTAINER_SLOT_EVENT container={} state_id={} slot={} item={}x{}",
                        container_id, state_id, slot, item.item.location, item.count
                    );
                }
                if container_id == 0 {
                    self.player_inventory_state_id = state_id as i32;
                }
                if let Some(active) = menu.filter(|active| active.container_id == container_id) {
                    effects.state_id = Some(state_id as i32);
                    if crate::gameplay::player::LocalPlayer::movement_diagnostics_enabled() {
                        println!(
                            "[MenuDebug] CLIENT_CONTAINER_SLOT_APPLY matched_screen=true container={} slot={} menu_slots={}",
                            container_id,
                            slot,
                            active.slots.len()
                        );
                    }
                } else if container_id == 0 {
                    if crate::gameplay::player::LocalPlayer::movement_diagnostics_enabled() {
                        println!(
                            "[MenuDebug] CLIENT_CONTAINER_SLOT_APPLY matched_screen=false container=0 fallback=player_inventory slot={}",
                            slot
                        );
                    }
                } else if crate::gameplay::player::LocalPlayer::movement_diagnostics_enabled() {
                    println!(
                        "[MenuDebug] CLIENT_CONTAINER_SLOT_APPLY matched_screen=false container={} ignored_non_player slot={}",
                        container_id, slot
                    );
                }
                if let Some(player_slot) = self.inventory_menu_item_target(
                    container_id,
                    slot as usize,
                    &item,
                    menu,
                    &mut effects,
                ) {
                    self.apply_inventory_player_slot(player_slot, item, menu, &mut effects);
                }
            }
            BackendNetworkEvent::SetCursorItem { item } => {
                let item = map_backend_item_stack(item);
                if crate::gameplay::player::LocalPlayer::movement_diagnostics_enabled() {
                    println!(
                        "[MenuDebug] CLIENT_SET_CURSOR_ITEM_EVENT item={}x{}",
                        item.item.location, item.count
                    );
                }
                effects = self.apply_inventory_cursor_item(item, menu);
            }
            BackendNetworkEvent::HeldSlot { slot } => {
                if crate::gameplay::player::LocalPlayer::movement_diagnostics_enabled() {
                    if slot > 8 {
                        println!(
                            "[HeldSlotDebug] disposition=ignored_invalid raw_slot={} active_before={} carried_index={}",
                            slot, self.active_hotbar_slot, self.game_mode.carried_index
                        );
                    } else {
                        println!(
                            "[HeldSlotDebug] disposition=applied raw_slot={} active_before={} active_after={} carried_index={}",
                            slot, self.active_hotbar_slot, slot, self.game_mode.carried_index
                        );
                    }
                }
                if slot <= 8 {
                    self.active_hotbar_slot = slot as usize;
                }
            }
            event => return Err(event),
        }
        Ok(effects)
    }

    fn apply_inventory_menu_items(
        &mut self,
        container_id: i32,
        items: impl Iterator<Item = ItemStack>,
        menu: Option<InventoryMenuContext<'_>>,
        effects: &mut InventoryUiEffects,
    ) {
        let mut player_updates = Vec::new();
        for (slot, item) in items.enumerate() {
            if let Some(player_slot) =
                self.inventory_menu_item_target(container_id, slot, &item, menu, effects)
            {
                player_updates.push((player_slot, item));
            }
        }
        for (slot, item) in player_updates {
            self.apply_inventory_player_slot(slot, item, menu, effects);
        }
        if menu.is_some_and(|active| active.container_id == container_id && active.creative) {
            effects.refresh_creative_hotbar = true;
        }
    }

    fn inventory_menu_item_target(
        &self,
        container_id: i32,
        slot: usize,
        item: &ItemStack,
        menu: Option<InventoryMenuContext<'_>>,
        effects: &mut InventoryUiEffects,
    ) -> Option<usize> {
        if let Some(active) = menu.filter(|active| active.container_id == container_id) {
            if active.creative {
                effects.refresh_creative_hotbar = true;
                inventory_menu_slot_to_player_slot(slot)
            } else {
                effects.slots.push((slot, item.clone()));
                active
                    .slots
                    .get(slot)
                    .filter(|slot| slot.container_idx == 0)
                    .map(|slot| slot.slot_idx)
            }
        } else if container_id == 0 {
            inventory_menu_slot_to_player_slot(slot)
        } else {
            None
        }
    }

    pub fn apply_inventory_cursor_item(
        &mut self,
        item: ItemStack,
        menu: Option<InventoryMenuContext<'_>>,
    ) -> InventoryUiEffects {
        let mut effects = InventoryUiEffects::default();
        if let Some(active) = menu {
            if active.creative {
                return effects;
            }
            if crate::gameplay::player::LocalPlayer::movement_diagnostics_enabled() {
                println!(
                    "[MenuDebug] CLIENT_SET_CURSOR_ITEM_APPLY matched_screen=true container={} item={}x{}",
                    active.container_id, item.item.location, item.count
                );
            }
            effects.carried_item = Some(item.clone());
        } else if crate::gameplay::player::LocalPlayer::movement_diagnostics_enabled() {
            println!(
                "[MenuDebug] CLIENT_SET_CURSOR_ITEM_APPLY matched_screen=false item={}x{}",
                item.item.location, item.count
            );
        }
        if menu.is_none_or(|active| active.container_id == 0) {
            self.player_inventory_carried_item = item;
        }
        effects
    }

    pub fn set_inventory_player_slot(
        &mut self,
        slot: usize,
        item: ItemStack,
        menu: Option<InventoryMenuContext<'_>>,
    ) -> InventoryUiEffects {
        let mut effects = InventoryUiEffects::default();
        self.apply_inventory_player_slot(slot, item, menu, &mut effects);
        effects
    }

    fn apply_inventory_player_slot(
        &mut self,
        slot: usize,
        item: ItemStack,
        menu: Option<InventoryMenuContext<'_>>,
        effects: &mut InventoryUiEffects,
    ) {
        let Some(player_item) = self.player.inventory.get_mut(slot) else {
            return;
        };
        trace_authoritative_item_quantity(slot, player_item, &item);
        *player_item = item.clone();
        if let Some(active) = menu {
            if active.creative {
                effects.refresh_creative_hotbar = true;
            } else {
                for (menu_slot, info) in active.slots.iter().enumerate() {
                    if info.container_idx == 0 && info.slot_idx == slot {
                        effects.slots.push((menu_slot, item.clone()));
                    }
                }
            }
        }
    }
}
