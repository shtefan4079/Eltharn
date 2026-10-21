use crate::container::{Container, SimpleContainer};
use crate::creative_tabs_catalog::{
    GENERATED_CREATIVE_TABS, GeneratedCreativeTab, GeneratedCreativeTabKind,
    GeneratedCreativeTabRow,
};
use crate::layout::SlotPosition;
use crate::menu::{Menu, MenuType, Slot, SlotType};
use el_core::item::{DataComponent, ItemStack};
use el_core::registry::BuiltInRegistries;
use el_core::resource_location::ResourceLocation;
use el_nbt::Tag;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock, RwLock};
pub const CREATIVE_GRID_SIZE: usize = 45;
pub const CREATIVE_GRID_COLS: usize = 9;
pub const CREATIVE_GRID_ROWS: usize = 5;
pub const CREATIVE_HOTBAR_SIZE: usize = 9;
pub const CREATIVE_IMAGE_WIDTH: i32 = 195;
pub const CREATIVE_IMAGE_HEIGHT: i32 = 136;
pub const CREATIVE_TAB_WIDTH: i32 = 26;
pub const CREATIVE_TAB_HEIGHT: i32 = 32;
pub const CREATIVE_TAB_SPACING: i32 = 27;
pub const CREATIVE_SCROLLBAR_X: i32 = 175;
pub const CREATIVE_SCROLLBAR_Y: i32 = 18;
pub const CREATIVE_SCROLLBAR_WIDTH: i32 = 14;
pub const CREATIVE_SCROLLBAR_HEIGHT: i32 = 112;
pub const CREATIVE_SCROLLER_HEIGHT: i32 = 15;
pub const CREATIVE_SCROLLER_TRAVEL: i32 = 95;
pub const CREATIVE_SCROLLER_DRAG_TRAVEL: i32 = 97;
pub const CREATIVE_TRASH_SLOT_INDEX: usize = 46;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CreativeTabKind {
    Category,
    Hotbar,
    Search,
    Inventory,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CreativeTabRow {
    Top,
    Bottom,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreativeTab {
    pub id: &'static str,
    pub translation_key: &'static str,
    pub icon_item: &'static str,
    pub background: &'static str,
    pub row: CreativeTabRow,
    pub column: i32,
    pub kind: CreativeTabKind,
    pub aligned_right: bool,
    pub show_title: bool,
    pub can_scroll: bool,
    pub items: &'static [&'static str],
}
#[derive(Clone, Debug)]
pub struct CreativeInventoryState {
    selected_tab: &'static str,
    pub scroll_offs: f32,
    pub scrolling: bool,
    pub search_query: String,
    pub visible_tags: Vec<ResourceLocation>,
    pub display_operator_tab: bool,
    pub has_permissions: bool,
    items: Vec<ItemStack>,
}
impl CreativeInventoryState {
    pub fn new(display_operator_tab: bool, has_permissions: bool) -> Self {
        let mut state = Self {
            selected_tab: default_tab().id,
            scroll_offs: 0.0,
            scrolling: false,
            search_query: String::new(),
            visible_tags: Vec::new(),
            display_operator_tab,
            has_permissions,
            items: Vec::new(),
        };
        state.rebuild_items();
        state
    }
    pub fn selected_tab_id(&self) -> &'static str {
        self.selected_tab
    }
    pub fn selected_tab(&self) -> CreativeTab {
        creative_tab(self.selected_tab).unwrap_or_else(default_tab)
    }
    pub fn selected_kind(&self) -> CreativeTabKind {
        self.selected_tab().kind
    }
    pub fn visible_tabs(&self) -> Vec<CreativeTab> {
        creative_tabs()
            .into_iter()
            .filter(|tab| self.should_display_tab(*tab))
            .collect()
    }
    pub fn select_tab(&mut self, id: &str, menu: &mut Menu) -> bool {
        let Some(tab) = creative_tab(id) else {
            return false;
        };
        if !self.should_display_tab(tab) {
            return false;
        }
        let old = self.selected_tab;
        self.selected_tab = tab.id;
        if old != tab.id || tab.kind != CreativeTabKind::Search {
            self.search_query.clear();
        }
        self.scroll_offs = 0.0;
        self.rebuild_items();
        self.apply_to_menu(menu);
        true
    }
    pub fn select_search_tab(&mut self, menu: &mut Menu) {
        let _ = self.select_tab("search", menu);
    }
    pub fn set_search_query(&mut self, query: impl Into<String>, menu: &mut Menu) {
        if self.selected_kind() != CreativeTabKind::Search {
            return;
        }
        self.search_query = query.into();
        if self.search_query.len() > 50 {
            self.search_query.truncate(50);
        }
        self.scroll_offs = 0.0;
        self.rebuild_items();
        self.apply_to_menu(menu);
    }
    pub fn push_search_char(&mut self, ch: char, menu: &mut Menu) -> bool {
        if self.selected_kind() != CreativeTabKind::Search
            || self.search_query.len() >= 50
            || ch.is_control()
        {
            return false;
        }
        self.search_query.push(ch);
        self.rebuild_items();
        self.apply_to_menu(menu);
        true
    }
    pub fn backspace_search(&mut self, menu: &mut Menu) -> bool {
        if self.selected_kind() != CreativeTabKind::Search {
            return false;
        }
        let removed = self.search_query.pop().is_some();
        if removed {
            self.rebuild_items();
            self.apply_to_menu(menu);
        }
        removed
    }
    pub fn selected_background_texture(&self) -> ResourceLocation {
        ResourceLocation::new(
            "minecraft",
            &format!(
                "gui/container/creative_inventory/tab_{}",
                self.selected_tab().background
            ),
        )
    }
    pub fn can_scroll(&self) -> bool {
        self.selected_tab().can_scroll && self.calculate_row_count() > 0
    }
    pub fn calculate_row_count(&self) -> i32 {
        let rows = (self.items.len() + CREATIVE_GRID_COLS - 1) / CREATIVE_GRID_COLS;
        rows.saturating_sub(CREATIVE_GRID_ROWS) as i32
    }
    pub fn row_index_for_scroll(&self, scroll_offs: f32) -> i32 {
        ((scroll_offs * self.calculate_row_count() as f32) + 0.5)
            .floor()
            .max(0.0) as i32
    }
    pub fn scroll_for_row_index(&self, row_index: i32) -> f32 {
        let row_count = self.calculate_row_count();
        if row_count <= 0 {
            0.0
        } else {
            (row_index as f32 / row_count as f32).clamp(0.0, 1.0)
        }
    }
    pub fn subtract_input_from_scroll(&mut self, input: f32, menu: &mut Menu) {
        let row_count = self.calculate_row_count();
        if row_count <= 0 {
            return;
        }
        self.scroll_offs = (self.scroll_offs - input / row_count as f32).clamp(0.0, 1.0);
        self.apply_to_menu(menu);
    }
    pub fn scroll_to_mouse_y(&mut self, mouse_y: f32, top_pos: i32, menu: &mut Menu) {
        let yscr = top_pos as f32 + CREATIVE_SCROLLBAR_Y as f32;
        self.scroll_offs =
            ((mouse_y - yscr - 7.5) / CREATIVE_SCROLLER_DRAG_TRAVEL as f32).clamp(0.0, 1.0);
        self.apply_to_menu(menu);
    }
    pub fn scroller_y(&self) -> i32 {
        CREATIVE_SCROLLBAR_Y + (CREATIVE_SCROLLER_TRAVEL as f32 * self.scroll_offs) as i32
    }
    pub fn inside_scrollbar(
        &self,
        mouse_x: f32,
        mouse_y: f32,
        left_pos: i32,
        top_pos: i32,
    ) -> bool {
        let x = mouse_x as i32 - left_pos;
        let y = mouse_y as i32 - top_pos;
        x >= CREATIVE_SCROLLBAR_X
            && y >= CREATIVE_SCROLLBAR_Y
            && x < CREATIVE_SCROLLBAR_X + CREATIVE_SCROLLBAR_WIDTH
            && y < CREATIVE_SCROLLBAR_Y + CREATIVE_SCROLLBAR_HEIGHT
            && self.selected_tab().can_scroll
    }
    pub fn tab_x(&self, tab: CreativeTab) -> i32 {
        let mut x = CREATIVE_TAB_SPACING * tab.column;
        if tab.aligned_right {
            x = CREATIVE_IMAGE_WIDTH - CREATIVE_TAB_SPACING * (7 - tab.column) + 1;
        }
        x
    }
    pub fn tab_y(&self, tab: CreativeTab) -> i32 {
        match tab.row {
            CreativeTabRow::Top => -CREATIVE_TAB_HEIGHT,
            CreativeTabRow::Bottom => CREATIVE_IMAGE_HEIGHT,
        }
    }
    pub fn tab_render_y(&self, tab: CreativeTab, top_pos: i32) -> i32 {
        match tab.row {
            CreativeTabRow::Top => top_pos - 28,
            CreativeTabRow::Bottom => top_pos + CREATIVE_IMAGE_HEIGHT - 4,
        }
    }
    pub fn tab_at(
        &self,
        mouse_x: f32,
        mouse_y: f32,
        left_pos: i32,
        top_pos: i32,
    ) -> Option<CreativeTab> {
        let x = mouse_x as i32 - left_pos;
        let y = mouse_y as i32 - top_pos;
        self.visible_tabs().into_iter().find(|tab| {
            let tx = self.tab_x(*tab);
            let ty = self.tab_y(*tab);
            x >= tx && x <= tx + CREATIVE_TAB_WIDTH && y >= ty && y <= ty + CREATIVE_TAB_HEIGHT
        })
    }
    pub fn tab_hovered_at(
        &self,
        mouse_x: f32,
        mouse_y: f32,
        left_pos: i32,
        top_pos: i32,
    ) -> Option<CreativeTab> {
        let x = mouse_x as i32 - left_pos;
        let y = mouse_y as i32 - top_pos;
        self.visible_tabs().into_iter().find(|tab| {
            let tx = self.tab_x(*tab);
            let ty = self.tab_y(*tab);
            x >= tx + 3 && x < tx + 3 + 21 && y >= ty + 3 && y < ty + 3 + 27
        })
    }
    pub fn apply_to_menu(&self, menu: &mut Menu) {
        if menu.menu_type != MenuType::Creative {
            return;
        }
        if self.selected_kind() == CreativeTabKind::Inventory {
            apply_inventory_tab_slots(menu);
            return;
        }
        apply_item_picker_slots(menu);
        if menu.containers.len() < 2 {
            menu.containers
                .push(SimpleContainer::new(CREATIVE_GRID_SIZE));
        }
        let start_idx = (self.row_index_for_scroll(self.scroll_offs) as usize) * CREATIVE_GRID_COLS;
        for slot_idx in 0..CREATIVE_GRID_SIZE {
            let item_idx = start_idx + slot_idx;
            let stack = self
                .items
                .get(item_idx)
                .cloned()
                .unwrap_or_else(ItemStack::empty);
            menu.containers[1].set_item(slot_idx, stack);
        }
    }
    pub fn current_items(&self) -> &[ItemStack] {
        &self.items
    }
    fn rebuild_items(&mut self) {
        self.visible_tags.clear();
        self.items = match self.selected_kind() {
            CreativeTabKind::Category => stacks_from_item_paths(self.selected_tab().items),
            CreativeTabKind::Search => self.search_items(),
            CreativeTabKind::Hotbar => self.hotbar_placeholders(),
            CreativeTabKind::Inventory => Vec::new(),
        };
    }
    fn search_items(&mut self) -> Vec<ItemStack> {
        let query = self.search_query.to_lowercase();
        let search_tags = query.strip_prefix('#');
        let matching_tags = search_tags.map(|tag_query| {
            let tags: Vec<ResourceLocation> = matching_item_tags(tag_query);
            self.visible_tags = tags.clone();
            tags
        });
        let mut seen = HashSet::new();
        let mut items = Vec::new();
        for tab in creative_tabs() {
            if matches!(
                tab.kind,
                CreativeTabKind::Search | CreativeTabKind::Inventory | CreativeTabKind::Hotbar
            ) || !self.should_display_tab(tab)
            {
                continue;
            }
            for path in tab.items {
                if !seen.insert(*path) {
                    continue;
                }
                let mut candidates = Vec::new();
                expand_item_path(path, &mut candidates);
                for stack in candidates {
                    if let Some(tags) = &matching_tags {
                        if tags.iter().any(|tag| item_is_in_tag(&stack, tag)) {
                            items.push(stack);
                        }
                    } else {
                        push_matching_stack(&mut items, path, stack, &query);
                    }
                }
            }
        }
        items
    }
    fn hotbar_placeholders(&self) -> Vec<ItemStack> {
        let paper = stack_from_item_path("paper").unwrap_or_else(ItemStack::empty);
        let mut items = vec![ItemStack::empty(); 81];
        for hotbar in 0..9 {
            items[hotbar * 9 + hotbar] = paper.clone();
        }
        items
    }
    fn should_display_tab(&self, tab: CreativeTab) -> bool {
        if tab.id == "op_blocks" {
            return self.display_operator_tab && self.has_permissions;
        }
        tab.kind != CreativeTabKind::Category || !tab.items.is_empty()
    }
}
pub fn creative_tabs() -> Vec<CreativeTab> {
    GENERATED_CREATIVE_TABS
        .iter()
        .copied()
        .map(convert_tab)
        .collect()
}
pub fn creative_tab(id: &str) -> Option<CreativeTab> {
    GENERATED_CREATIVE_TABS
        .iter()
        .copied()
        .find(|tab| tab.id == id)
        .map(convert_tab)
}
pub fn default_tab() -> CreativeTab {
    creative_tab("building_blocks").expect("generated creative tabs must include building_blocks")
}
pub fn apply_item_picker_slots(menu: &mut Menu) {
    menu.slots.clear();
    for slot_idx in 0..CREATIVE_GRID_SIZE {
        menu.slots.push(Slot::new(1, slot_idx, SlotType::Creative));
    }
    for slot_idx in 0..CREATIVE_HOTBAR_SIZE {
        menu.slots.push(Slot::new(0, slot_idx, SlotType::Hotbar));
    }
    menu.slot_positions = item_picker_slot_positions();
}
pub fn apply_inventory_tab_slots(menu: &mut Menu) {
    if menu.containers.len() < 3 {
        menu.containers.push(SimpleContainer::new(1));
    }
    menu.slots.clear();
    for inv_menu_slot in 0..46 {
        let (container_slot, slot_type) = match inv_menu_slot {
            5 => (39, SlotType::Armor),
            6 => (38, SlotType::Armor),
            7 => (37, SlotType::Armor),
            8 => (36, SlotType::Armor),
            9..=35 => (inv_menu_slot, SlotType::PlayerInventory),
            36..=44 => (inv_menu_slot - 36, SlotType::Hotbar),
            45 => (40, SlotType::Offhand),
            _ => (inv_menu_slot, SlotType::Generic),
        };
        menu.slots.push(Slot::new(0, container_slot, slot_type));
    }
    menu.slots.push(Slot::new(2, 0, SlotType::Trash));
    menu.slot_positions = inventory_tab_slot_positions();
}
pub fn item_picker_slot_positions() -> Vec<SlotPosition> {
    let mut positions = Vec::with_capacity(54);
    for row in 0..5 {
        for col in 0..9 {
            positions.push(SlotPosition::new(9 + col * 18, 18 + row * 18));
        }
    }
    for col in 0..9 {
        positions.push(SlotPosition::new(9 + col * 18, 112));
    }
    positions
}
pub fn inventory_tab_slot_positions() -> Vec<SlotPosition> {
    let mut positions = Vec::with_capacity(47);
    for i in 0..46 {
        let pos = match i {
            0..=4 => SlotPosition::new(-2000, -2000),
            5..=8 => {
                let pos = i - 5;
                let col = pos / 2;
                let row = pos % 2;
                SlotPosition::new(54 + col * 54, 6 + row * 27)
            }
            45 => SlotPosition::new(35, 20),
            _ => {
                let pos = i - 9;
                let col = pos % 9;
                let row = pos / 9;
                let y = if i >= 36 { 112 } else { 54 + row * 18 };
                SlotPosition::new(9 + col * 18, y)
            }
        };
        positions.push(pos);
    }
    positions.push(SlotPosition::new(173, 112));
    positions
}
pub fn creative_packet_slot(slot: &Slot) -> Option<i16> {
    if slot.container_idx != 0 {
        return None;
    }
    match slot.slot_type {
        SlotType::Hotbar => Some((36 + slot.slot_idx) as i16),
        SlotType::PlayerInventory => Some(slot.slot_idx as i16),
        SlotType::Armor => match slot.slot_idx {
            39 => Some(5),
            38 => Some(6),
            37 => Some(7),
            36 => Some(8),
            _ => None,
        },
        SlotType::Offhand => Some(45),
        _ => None,
    }
}
fn convert_tab(tab: GeneratedCreativeTab) -> CreativeTab {
    CreativeTab {
        id: tab.id,
        translation_key: tab.translation_key,
        icon_item: tab.icon_item,
        background: tab.background,
        row: match tab.row {
            GeneratedCreativeTabRow::Top => CreativeTabRow::Top,
            GeneratedCreativeTabRow::Bottom => CreativeTabRow::Bottom,
        },
        column: tab.column,
        kind: match tab.kind {
            GeneratedCreativeTabKind::Category => CreativeTabKind::Category,
            GeneratedCreativeTabKind::Hotbar => CreativeTabKind::Hotbar,
            GeneratedCreativeTabKind::Search => CreativeTabKind::Search,
            GeneratedCreativeTabKind::Inventory => CreativeTabKind::Inventory,
        },
        aligned_right: tab.aligned_right,
        show_title: tab.show_title,
        can_scroll: tab.can_scroll,
        items: tab.items,
    }
}
const POTION_EFFECTS: &[&str] = &[
    "minecraft:water",
    "minecraft:mundane",
    "minecraft:thick",
    "minecraft:awkward",
    "minecraft:night_vision",
    "minecraft:long_night_vision",
    "minecraft:invisibility",
    "minecraft:long_invisibility",
    "minecraft:leaping",
    "minecraft:long_leaping",
    "minecraft:strong_leaping",
    "minecraft:fire_resistance",
    "minecraft:long_fire_resistance",
    "minecraft:swiftness",
    "minecraft:long_swiftness",
    "minecraft:strong_swiftness",
    "minecraft:slowness",
    "minecraft:long_slowness",
    "minecraft:strong_slowness",
    "minecraft:water_breathing",
    "minecraft:long_water_breathing",
    "minecraft:healing",
    "minecraft:strong_healing",
    "minecraft:harming",
    "minecraft:strong_harming",
    "minecraft:poison",
    "minecraft:long_poison",
    "minecraft:strong_poison",
    "minecraft:regeneration",
    "minecraft:long_regeneration",
    "minecraft:strong_regeneration",
    "minecraft:strength",
    "minecraft:long_strength",
    "minecraft:strong_strength",
    "minecraft:weakness",
    "minecraft:long_weakness",
    "minecraft:slow_falling",
    "minecraft:long_slow_falling",
    "minecraft:turtle_master",
    "minecraft:long_turtle_master",
    "minecraft:strong_turtle_master",
    "minecraft:infestation",
    "minecraft:oozing",
    "minecraft:weaving",
    "minecraft:wind_charged",
];
fn compound_get<'a>(fields: &'a [(String, Tag)], key: &str) -> Option<&'a Tag> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}
fn expand_item_path(path: &str, out: &mut Vec<ItemStack>) {
    if path == "potion"
        || path == "splash_potion"
        || path == "lingering_potion"
        || path == "tipped_arrow"
    {
        if let Some(base_stack) = stack_from_item_path(path) {
            for effect in POTION_EFFECTS {
                let mut stack = base_stack.clone();
                let compound_value = Tag::Compound(vec![(
                    "potion".to_string(),
                    Tag::String(effect.to_string()),
                )]);
                stack.components.set(
                    "minecraft:potion_contents",
                    DataComponent::Generic {
                        type_name: "minecraft:potion_contents".to_string(),
                        value: compound_value,
                    },
                );
                out.push(stack);
            }
        }
    } else if path == "firework_rocket" {
        if let Some(base_stack) = stack_from_item_path(path) {
            for flight_duration in 1..=3 {
                let mut stack = base_stack.clone();
                stack.components.set(
                    "minecraft:fireworks",
                    DataComponent::Generic {
                        type_name: "minecraft:fireworks".to_string(),
                        value: Tag::Compound(vec![
                            ("flight_duration".to_string(), Tag::Int(flight_duration)),
                            ("explosions".to_string(), Tag::List(Vec::new())),
                        ]),
                    },
                );
                out.push(stack);
            }
        }
    } else if path == "ominous_bottle" {
        if let Some(base_stack) = stack_from_item_path(path) {
            for amp in 0..5 {
                let mut stack = base_stack.clone();
                stack.components.set(
                    "minecraft:ominous_bottle_amplifier",
                    DataComponent::Generic {
                        type_name: "minecraft:ominous_bottle_amplifier".to_string(),
                        value: Tag::Int(amp),
                    },
                );
                out.push(stack);
            }
        }
    } else if path == "enchanted_book" {
        if let Some(base_stack) = stack_from_item_path(path) {
            let enchantments = [
                ("minecraft:channeling", 1),
                ("minecraft:quick_charge", 3),
                ("minecraft:silk_touch", 1),
                ("minecraft:protection", 4),
                ("minecraft:feather_falling", 4),
                ("minecraft:sweeping_edge", 3),
                ("minecraft:impaling", 5),
                ("minecraft:wind_burst", 3),
                ("minecraft:sharpness", 5),
                ("minecraft:power", 5),
                ("minecraft:lure", 3),
                ("minecraft:unbreaking", 3),
                ("minecraft:depth_strider", 3),
                ("minecraft:loyalty", 3),
                ("minecraft:smite", 5),
                ("minecraft:fire_aspect", 2),
                ("minecraft:breach", 4),
                ("minecraft:mending", 1),
                ("minecraft:binding_curse", 1),
                ("minecraft:frost_walker", 2),
                ("minecraft:soul_speed", 3),
                ("minecraft:piercing", 4),
                ("minecraft:vanishing_curse", 1),
                ("minecraft:looting", 3),
                ("minecraft:projectile_protection", 4),
                ("minecraft:blast_protection", 4),
                ("minecraft:respiration", 3),
                ("minecraft:multishot", 1),
                ("minecraft:thorns", 3),
                ("minecraft:riptide", 3),
                ("minecraft:flame", 1),
                ("minecraft:infinity", 1),
                ("minecraft:aqua_affinity", 1),
                ("minecraft:efficiency", 5),
                ("minecraft:knockback", 2),
                ("minecraft:fire_protection", 4),
                ("minecraft:fortune", 3),
                ("minecraft:punch", 2),
                ("minecraft:swift_sneak", 3),
                ("minecraft:bane_of_arthropods", 5),
                ("minecraft:density", 5),
                ("minecraft:lunge", 1),
                ("minecraft:luck_of_the_sea", 3),
            ];
            for (ench, max_lvl) in enchantments {
                let mut stack = base_stack.clone();
                let compound_value = Tag::Compound(vec![(ench.to_string(), Tag::Int(max_lvl))]);
                stack.components.set(
                    "minecraft:stored_enchantments",
                    DataComponent::Generic {
                        type_name: "minecraft:stored_enchantments".to_string(),
                        value: compound_value,
                    },
                );
                out.push(stack);
            }
        }
    } else if path == "goat_horn" {
        if let Some(base_stack) = stack_from_item_path(path) {
            let instruments = [
                "minecraft:ponder_goat_horn",
                "minecraft:sing_goat_horn",
                "minecraft:seek_goat_horn",
                "minecraft:feel_goat_horn",
                "minecraft:admire_goat_horn",
                "minecraft:call_goat_horn",
                "minecraft:yearn_goat_horn",
                "minecraft:dream_goat_horn",
            ];
            for inst in instruments {
                let mut stack = base_stack.clone();
                stack.components.set(
                    "minecraft:instrument",
                    DataComponent::Generic {
                        type_name: "minecraft:instrument".to_string(),
                        value: Tag::String(inst.to_string()),
                    },
                );
                out.push(stack);
            }
        }
    } else if path == "suspicious_stew" {
        if let Some(base_stack) = stack_from_item_path(path) {
            let stews = [
                ("minecraft:night_vision", 100),
                ("minecraft:jump_boost", 120),
                ("minecraft:weakness", 160),
                ("minecraft:blindness", 160),
                ("minecraft:poison", 240),
                ("minecraft:saturation", 7),
                ("minecraft:fire_resistance", 80),
                ("minecraft:regeneration", 160),
                ("minecraft:wither", 160),
            ];
            for (effect, duration) in stews {
                let mut stack = base_stack.clone();
                let effect_compound = Tag::Compound(vec![
                    ("id".to_string(), Tag::String(effect.to_string())),
                    ("duration".to_string(), Tag::Int(duration)),
                ]);
                stack.components.set(
                    "minecraft:suspicious_stew_effects",
                    DataComponent::Generic {
                        type_name: "minecraft:suspicious_stew_effects".to_string(),
                        value: Tag::List(vec![effect_compound]),
                    },
                );
                out.push(stack);
            }
        }
    } else {
        if let Some(stack) = stack_from_item_path(path) {
            out.push(stack);
        }
    }
}
fn stacks_from_item_paths(paths: &[&str]) -> Vec<ItemStack> {
    let mut result = Vec::new();
    for path in paths {
        expand_item_path(path, &mut result);
    }
    result
}
fn stack_from_item_path(path: &str) -> Option<ItemStack> {
    let loc = ResourceLocation::new("minecraft", path);
    let item = BuiltInRegistries::items().get_by_location(&loc).clone();
    if item.location.path == "air" && path != "air" {
        None
    } else {
        Some(ItemStack::new(item, 1))
    }
}
fn push_matching_stack(items: &mut Vec<ItemStack>, path: &str, stack: ItemStack, query: &str) {
    let is_match = if query.is_empty() {
        true
    } else {
        let path_matches = path.to_lowercase().contains(query);
        if path_matches {
            true
        } else if let Some(DataComponent::Generic {
            value: Tag::Compound(fields),
            ..
        }) = stack.components.get("minecraft:potion_contents")
        {
            if let Some(Tag::String(potion_id)) = compound_get(fields, "potion") {
                potion_id.to_lowercase().contains(query)
            } else {
                false
            }
        } else {
            false
        }
    };
    if is_match {
        items.push(stack);
    }
}
#[derive(Clone, Debug, Default)]
pub struct ItemTagIndex {
    entries: HashMap<ResourceLocation, Vec<ResourceLocation>>,
}
impl ItemTagIndex {
    pub fn from_documents<I, B>(documents: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = (ResourceLocation, B)>,
        B: AsRef<[u8]>,
    {
        let mut raw = HashMap::new();
        for (id, bytes) in documents {
            let document: serde_json::Value = serde_json::from_slice(bytes.as_ref())
                .map_err(|error| format!("Item tag {id}: {error}"))?;
            let values = document
                .get("values")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| format!("Item tag {id}: missing values array"))?;
            let mut entries = Vec::with_capacity(values.len());
            for value in values {
                entries.push(
                    parse_item_tag_entry(value)
                        .ok_or_else(|| format!("Item tag {id}: invalid entry"))?,
                );
            }
            if raw.insert(id.clone(), entries).is_some() {
                return Err(format!("Duplicate item tag {id}"));
            }
        }
        let mut entries = HashMap::new();
        for id in raw.keys() {
            flatten_item_tag(&raw, id, &mut HashSet::new(), &mut entries);
        }
        Ok(Self { entries })
    }
    pub fn contains(&self, item: &ResourceLocation, tag: &ResourceLocation) -> bool {
        self.entries
            .get(tag)
            .is_some_and(|entries| entries.contains(item))
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    fn matching(&self, query: &str) -> Vec<ResourceLocation> {
        let mut tags: Vec<_> = self
            .entries
            .keys()
            .filter(|id| id.path.contains(query) || id.to_string().contains(query))
            .cloned()
            .collect();
        tags.sort_by_key(ResourceLocation::to_string);
        tags
    }
}
static ITEM_TAG_INDEX: OnceLock<RwLock<Arc<ItemTagIndex>>> = OnceLock::new();
fn item_tag_index() -> Arc<ItemTagIndex> {
    ITEM_TAG_INDEX
        .get_or_init(|| RwLock::new(Arc::default()))
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}
pub fn replace_item_tags(index: ItemTagIndex) {
    *ITEM_TAG_INDEX
        .get_or_init(|| RwLock::new(Arc::default()))
        .write()
        .unwrap_or_else(|error| error.into_inner()) = Arc::new(index);
}
fn matching_item_tags(query: &str) -> Vec<ResourceLocation> {
    item_tag_index().matching(query)
}
pub fn item_is_in_tag(item: &ItemStack, tag: &ResourceLocation) -> bool {
    item_tag_index().contains(&item.item.location, tag)
}
enum ItemTagEntry {
    Item(ResourceLocation),
    Tag(ResourceLocation),
}
fn parse_item_tag_entry(value: &serde_json::Value) -> Option<ItemTagEntry> {
    let raw = value.as_str().map(ToOwned::to_owned).or_else(|| {
        value
            .as_object()
            .and_then(|object| object.get("id"))
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
    })?;
    if let Some(tag) = raw.strip_prefix('#') {
        ResourceLocation::try_from_string(tag).map(ItemTagEntry::Tag)
    } else {
        ResourceLocation::try_from_string(&raw).map(ItemTagEntry::Item)
    }
}
fn flatten_item_tag(
    raw: &HashMap<ResourceLocation, Vec<ItemTagEntry>>,
    tag_id: &ResourceLocation,
    visiting: &mut HashSet<String>,
    index: &mut HashMap<ResourceLocation, Vec<ResourceLocation>>,
) -> Vec<ResourceLocation> {
    if let Some(cached) = index.get(tag_id) {
        return cached.clone();
    }
    if !visiting.insert(tag_id.to_string()) {
        return Vec::new();
    }
    let mut items: Vec<ResourceLocation> = Vec::new();
    for entry in raw.get(tag_id).map(Vec::as_slice).unwrap_or(&[]) {
        match entry {
            ItemTagEntry::Item(item) => items.push(item.clone()),
            ItemTagEntry::Tag(nested) => {
                items.extend(flatten_item_tag(raw, nested, visiting, index));
            }
        }
    }
    visiting.remove(&tag_id.to_string());
    let mut seen = HashSet::new();
    items.retain(|item| seen.insert(item.clone()));
    index.insert(tag_id.clone(), items.clone());
    items
}
