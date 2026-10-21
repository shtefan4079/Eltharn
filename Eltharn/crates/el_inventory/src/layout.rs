use crate::menu::MenuType;
use el_core::resource_location::ResourceLocation;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotPosition {
    pub x: i32,
    pub y: i32,
}
impl SlotPosition {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChestBackgroundSplit {
    pub top_height: i32,
    pub bottom_source_y: i32,
    pub bottom_height: i32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InventoryEntityLayout {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    pub size: i32,
    pub y_offset: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ScreenLayout {
    pub image_width: i32,
    pub image_height: i32,
    pub title_label_x: i32,
    pub title_label_y: i32,
    pub inventory_label_x: i32,
    pub inventory_label_y: i32,
    pub background: ResourceLocation,
    pub slot_positions: Vec<SlotPosition>,
    pub chest_background_split: Option<ChestBackgroundSplit>,
    pub inventory_entity: Option<InventoryEntityLayout>,
}
impl ScreenLayout {
    pub fn for_menu(menu_type: MenuType, slots_len: usize) -> Self {
        let (image_width, image_height) = image_size(menu_type, slots_len);
        let background = background_texture(menu_type);
        let slot_positions = slot_positions(menu_type, slots_len);
        let title_label_x = match menu_type {
            MenuType::Inventory => 97,
            MenuType::Merchant => 116,
            _ => 8,
        };
        let title_label_y = 6;
        let inventory_label_x = match menu_type {
            MenuType::Merchant => 108 + 8,
            MenuType::Beacon => 36,
            _ => 8,
        };
        let inventory_label_y = image_height - 94;
        let chest_background_split = match menu_type {
            MenuType::Chest
            | MenuType::Chest1Row
            | MenuType::Chest2Rows
            | MenuType::Chest3Rows
            | MenuType::Chest4Rows
            | MenuType::Chest5Rows
            | MenuType::Chest6Rows => {
                let chest_slots = slots_len.saturating_sub(36);
                let rows = chest_rows_from_slots(chest_slots);
                Some(ChestBackgroundSplit {
                    top_height: rows as i32 * 18 + 17,
                    bottom_source_y: 126,
                    bottom_height: 96,
                })
            }
            _ => None,
        };
        let inventory_entity = if menu_type == MenuType::Inventory {
            Some(InventoryEntityLayout {
                x0: 26,
                y0: 8,
                x1: 75,
                y1: 78,
                size: 30,
                y_offset: 0.0625,
            })
        } else {
            None
        };
        Self {
            image_width,
            image_height,
            title_label_x,
            title_label_y,
            inventory_label_x,
            inventory_label_y,
            background,
            slot_positions,
            chest_background_split,
            inventory_entity,
        }
    }
}
pub fn image_size(menu_type: MenuType, slots_len: usize) -> (i32, i32) {
    match menu_type {
        MenuType::Chest
        | MenuType::Chest1Row
        | MenuType::Chest2Rows
        | MenuType::Chest3Rows
        | MenuType::Chest4Rows
        | MenuType::Chest5Rows
        | MenuType::Chest6Rows => {
            let chest_slots = slots_len.saturating_sub(36);
            let rows = chest_rows_from_slots(chest_slots);
            (176, 114 + rows as i32 * 18)
        }
        MenuType::Creative => (195, 136),
        MenuType::Hopper => (176, 133),
        MenuType::Beacon => (230, 219),
        MenuType::Merchant => (276, 166),
        _ => (176, 166),
    }
}
pub fn background_texture(menu_type: MenuType) -> ResourceLocation {
    match menu_type {
        MenuType::Inventory => ResourceLocation::new("minecraft", "gui/container/inventory"),
        MenuType::Crafting => ResourceLocation::new("minecraft", "gui/container/crafting_table"),
        MenuType::Furnace => ResourceLocation::new("minecraft", "gui/container/furnace"),
        MenuType::Smoker => ResourceLocation::new("minecraft", "gui/container/smoker"),
        MenuType::BlastFurnace => ResourceLocation::new("minecraft", "gui/container/blast_furnace"),
        MenuType::Chest
        | MenuType::Chest1Row
        | MenuType::Chest2Rows
        | MenuType::Chest3Rows
        | MenuType::Chest4Rows
        | MenuType::Chest5Rows
        | MenuType::Chest6Rows => ResourceLocation::new("minecraft", "gui/container/generic_54"),
        MenuType::Dispenser => ResourceLocation::new("minecraft", "gui/container/dispenser"),
        MenuType::Crafter => ResourceLocation::new("minecraft", "gui/container/crafter"),
        MenuType::Anvil => ResourceLocation::new("minecraft", "gui/container/anvil"),
        MenuType::Beacon => ResourceLocation::new("minecraft", "gui/container/beacon"),
        MenuType::BrewingStand => ResourceLocation::new("minecraft", "gui/container/brewing_stand"),
        MenuType::Hopper => ResourceLocation::new("minecraft", "gui/container/hopper"),
        MenuType::Lectern => ResourceLocation::new("minecraft", "gui/container/lectern"),
        MenuType::Loom => ResourceLocation::new("minecraft", "gui/container/loom"),
        MenuType::Merchant => ResourceLocation::new("minecraft", "gui/container/villager"),
        MenuType::ShulkerBox => ResourceLocation::new("minecraft", "gui/container/shulker_box"),
        MenuType::Smithing => ResourceLocation::new("minecraft", "gui/container/smithing"),
        MenuType::Grindstone => ResourceLocation::new("minecraft", "gui/container/grindstone"),
        MenuType::CartographyTable => {
            ResourceLocation::new("minecraft", "gui/container/cartography_table")
        }
        MenuType::Stonecutter => ResourceLocation::new("minecraft", "gui/container/stonecutter"),
        MenuType::Enchantment => {
            ResourceLocation::new("minecraft", "gui/container/enchanting_table")
        }
        MenuType::Creative => {
            ResourceLocation::new("minecraft", "gui/container/creative_inventory/tab_items")
        }
        MenuType::Mount => ResourceLocation::new("minecraft", "gui/container/horse"),
    }
}
pub fn slot_positions(menu_type: MenuType, slots_len: usize) -> Vec<SlotPosition> {
    let mut coords = Vec::with_capacity(slots_len);
    match menu_type {
        MenuType::Inventory => {
            coords.push(SlotPosition::new(154, 28));
            coords.push(SlotPosition::new(98, 18));
            coords.push(SlotPosition::new(116, 18));
            coords.push(SlotPosition::new(98, 36));
            coords.push(SlotPosition::new(116, 36));
            coords.push(SlotPosition::new(8, 8));
            coords.push(SlotPosition::new(8, 26));
            coords.push(SlotPosition::new(8, 44));
            coords.push(SlotPosition::new(8, 62));
            push_player_inventory(&mut coords, 8, 84, 8, 142);
            coords.push(SlotPosition::new(77, 62));
        }
        MenuType::Crafting => {
            coords.push(SlotPosition::new(124, 35));
            for row in 0..3 {
                for col in 0..3 {
                    coords.push(SlotPosition::new(30 + col * 18, 17 + row * 18));
                }
            }
            push_player_inventory(&mut coords, 8, 84, 8, 142);
        }
        MenuType::Furnace | MenuType::Smoker | MenuType::BlastFurnace => {
            coords.push(SlotPosition::new(56, 17));
            coords.push(SlotPosition::new(56, 53));
            coords.push(SlotPosition::new(116, 35));
            push_player_inventory(&mut coords, 8, 84, 8, 142);
        }
        MenuType::Chest
        | MenuType::Chest1Row
        | MenuType::Chest2Rows
        | MenuType::Chest3Rows
        | MenuType::Chest4Rows
        | MenuType::Chest5Rows
        | MenuType::Chest6Rows
        | MenuType::ShulkerBox => {
            let chest_slots = slots_len.saturating_sub(36);
            let rows = chest_rows_from_slots(chest_slots);
            for i in 0..chest_slots {
                let row = (i / 9) as i32;
                let col = (i % 9) as i32;
                coords.push(SlotPosition::new(8 + col * 18, 18 + row * 18));
            }
            let player_inventory_y = 18 + rows as i32 * 18 + 13;
            push_player_inventory(
                &mut coords,
                8,
                player_inventory_y,
                8,
                player_inventory_y + 58,
            );
        }
        MenuType::Mount => {
            let mount_slots = slots_len.saturating_sub(38);
            let columns = mount_slots.div_ceil(3).max(1);
            coords.push(SlotPosition::new(8, 18));
            coords.push(SlotPosition::new(8, 36));
            for y in 0..3 {
                for x in 0..columns {
                    let idx = x + y * columns;
                    if idx >= mount_slots {
                        break;
                    }
                    coords.push(SlotPosition::new(80 + x as i32 * 18, 18 + y as i32 * 18));
                }
            }
            push_player_inventory(&mut coords, 8, 84, 8, 142);
        }
        MenuType::Crafter => {
            for row in 0..3 {
                for col in 0..3 {
                    coords.push(SlotPosition::new(26 + col * 18, 17 + row * 18));
                }
            }
            push_player_inventory(&mut coords, 8, 84, 8, 142);
            coords.push(SlotPosition::new(134, 35));
        }
        MenuType::Dispenser => {
            for row in 0..3 {
                for col in 0..3 {
                    coords.push(SlotPosition::new(62 + col * 18, 17 + row * 18));
                }
            }
            push_player_inventory(&mut coords, 8, 84, 8, 142);
        }
        MenuType::Anvil => {
            coords.push(SlotPosition::new(27, 47));
            coords.push(SlotPosition::new(76, 47));
            coords.push(SlotPosition::new(134, 47));
            push_player_inventory(&mut coords, 8, 84, 8, 142);
        }
        MenuType::Beacon => {
            coords.push(SlotPosition::new(136, 110));
            push_player_inventory(&mut coords, 36, 137, 36, 195);
        }
        MenuType::BrewingStand => {
            coords.push(SlotPosition::new(56, 51));
            coords.push(SlotPosition::new(79, 58));
            coords.push(SlotPosition::new(102, 51));
            coords.push(SlotPosition::new(79, 17));
            coords.push(SlotPosition::new(17, 17));
            push_player_inventory(&mut coords, 8, 84, 8, 142);
        }
        MenuType::Hopper => {
            for col in 0..5 {
                coords.push(SlotPosition::new(44 + col * 18, 20));
            }
            push_player_inventory(&mut coords, 8, 51, 8, 109);
        }
        MenuType::Lectern => {
            coords.push(SlotPosition::new(0, 0));
            push_player_inventory(&mut coords, 8, 84, 8, 142);
        }
        MenuType::Loom => {
            coords.push(SlotPosition::new(13, 26));
            coords.push(SlotPosition::new(33, 26));
            coords.push(SlotPosition::new(23, 45));
            coords.push(SlotPosition::new(143, 53));
            push_player_inventory(&mut coords, 8, 84, 8, 142);
        }
        MenuType::Merchant => {
            coords.push(SlotPosition::new(136, 37));
            coords.push(SlotPosition::new(162, 37));
            coords.push(SlotPosition::new(220, 37));
            push_player_inventory(&mut coords, 108, 84, 108, 142);
        }
        MenuType::Smithing => {
            coords.push(SlotPosition::new(8, 48));
            coords.push(SlotPosition::new(26, 48));
            coords.push(SlotPosition::new(44, 48));
            coords.push(SlotPosition::new(98, 48));
            push_player_inventory(&mut coords, 8, 84, 8, 142);
        }
        MenuType::Grindstone => {
            coords.push(SlotPosition::new(49, 19));
            coords.push(SlotPosition::new(49, 40));
            coords.push(SlotPosition::new(129, 34));
            push_player_inventory(&mut coords, 8, 84, 8, 142);
        }
        MenuType::CartographyTable => {
            coords.push(SlotPosition::new(15, 15));
            coords.push(SlotPosition::new(15, 52));
            coords.push(SlotPosition::new(145, 39));
            push_player_inventory(&mut coords, 8, 84, 8, 142);
        }
        MenuType::Stonecutter => {
            coords.push(SlotPosition::new(20, 33));
            coords.push(SlotPosition::new(143, 33));
            push_player_inventory(&mut coords, 8, 84, 8, 142);
        }
        MenuType::Enchantment => {
            coords.push(SlotPosition::new(15, 47));
            coords.push(SlotPosition::new(35, 47));
            push_player_inventory(&mut coords, 8, 84, 8, 142);
        }
        MenuType::Creative => {
            for row in 0..5 {
                for col in 0..9 {
                    coords.push(SlotPosition::new(9 + col * 18, 18 + row * 18));
                }
            }
            for col in 0..9 {
                coords.push(SlotPosition::new(9 + col * 18, 112));
            }
        }
    }
    coords.truncate(slots_len);
    coords
}
fn push_player_inventory(
    coords: &mut Vec<SlotPosition>,
    inventory_x: i32,
    inventory_y: i32,
    hotbar_x: i32,
    hotbar_y: i32,
) {
    for row in 0..3 {
        for col in 0..9 {
            coords.push(SlotPosition::new(
                inventory_x + col * 18,
                inventory_y + row * 18,
            ));
        }
    }
    for col in 0..9 {
        coords.push(SlotPosition::new(hotbar_x + col * 18, hotbar_y));
    }
}
fn chest_rows_from_slots(chest_slots: usize) -> usize {
    ((chest_slots + 8) / 9).max(1)
}
