use crate::gameplay::player::local_player::{
    ActiveMobEffect, LocalPlayer, compare_mob_effects, mob_effect_path,
};
use crate::input::{BoundInput, KeyAction, VanillaKeyMappings};
use crate::ui::chat::chat_component::{
    draw_formatted_chat_text_ex, formatted_chat_width, split_formatted_chat_lines,
};
use crate::ui::gui::font::FontRenderer;
use crate::ui::gui::item_renderer::{draw_gui_item, draw_gui_item_decorations_with_count_text};
use crate::ui::gui::renderer::GuiRenderer;
use el_core::item::{DataComponent, ItemStack, is_vanilla_bundle_item_path};
use el_core::registry::BuiltInRegistries;
use el_core::resource_location::ResourceLocation;
use el_inventory::creative::{
    CREATIVE_TAB_HEIGHT, CREATIVE_TAB_WIDTH, CreativeInventoryState, CreativeTabKind,
    CreativeTabRow,
};
use el_inventory::layout::ScreenLayout;
use el_inventory::menu::{ClickType, Menu, MenuType, SlotType};
use el_nbt::tag::Tag;
use el_protocol::packets::game::{
    ContainerInput, ServerboundContainerClickPacket, ServerboundSetCreativeModeSlotPacket,
};
use std::collections::{BTreeMap, HashSet};
use winit::event::{ElementState, Ime, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
#[path = "container/item_attribute_tooltips.rs"]
mod item_attribute_tooltips;
#[cfg(feature = "extensions")]
#[path = "container/mod_tooltip.rs"]
mod mod_tooltip;
const TOOLTIP_TEXT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const TOOLTIP_CATEGORY: [f32; 4] = [0.333, 0.333, 1.0, 1.0];
const TOOLTIP_DARK_PURPLE: [f32; 4] = [0.667, 0.0, 0.667, 1.0];
const QUICK_CRAFT_FILL: [f32; 4] = [1.0, 1.0, 1.0, 128.0 / 255.0];
const QUICK_CRAFT_COUNT_YELLOW: [f32; 4] = [1.0, 1.0, 0.333, 1.0];
const VANILLA_CONTAINER_TITLE_COLOR_ARGB: i32 = -12566464;
const EDIT_BOX_CURSOR_HEIGHT: i32 = 11;
fn util_get_millis() -> u64 {
    static START: std::sync::OnceLock<web_time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(web_time::Instant::now)
        .elapsed()
        .as_millis() as u64
}
fn quick_craft_place_count(
    quick_craft_slots_size: usize,
    quick_crafting_type: i32,
    carried: &ItemStack,
) -> i32 {
    match quick_crafting_type {
        0 => (carried.count as f32 / quick_craft_slots_size as f32).floor() as i32,
        1 => 1,
        2 => carried.get_max_stack_size(),
        _ => carried.count,
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlotContourKind {
    SlotStream,
    ScreenBackground,
}
fn slot_no_item_icon(
    slot_type: SlotType,
    slot_idx: usize,
    mount_entity_kind: Option<&str>,
) -> Option<(ResourceLocation, SlotContourKind)> {
    let (path, kind) = match slot_type {
        SlotType::Armor => (
            match slot_idx {
                39 => "container/slot/helmet",
                38 => "container/slot/chestplate",
                37 => "container/slot/leggings",
                36 => "container/slot/boots",
                _ => return None,
            },
            SlotContourKind::SlotStream,
        ),
        SlotType::Offhand => ("container/slot/shield", SlotContourKind::SlotStream),
        SlotType::Lapis => ("container/slot/lapis_lazuli", SlotContourKind::SlotStream),
        SlotType::BrewingFuel => ("container/slot/brewing_fuel", SlotContourKind::SlotStream),
        SlotType::BrewingBottle => ("container/slot/potion", SlotContourKind::SlotStream),
        SlotType::MountSaddle => ("container/slot/saddle", SlotContourKind::SlotStream),
        SlotType::MountArmor => {
            let path = {
                let kind = mount_entity_kind.unwrap_or("");
                if kind.contains("llama") {
                    "container/slot/llama_armor"
                } else if kind.contains("nautilus") {
                    "container/slot/nautilus_armor_inventory"
                } else {
                    "container/slot/horse_armor"
                }
            };
            (path, SlotContourKind::SlotStream)
        }
        SlotType::Banner => ("container/slot/banner", SlotContourKind::ScreenBackground),
        SlotType::Dye => ("container/slot/dye", SlotContourKind::ScreenBackground),
        SlotType::BannerPattern => (
            "container/slot/banner_pattern",
            SlotContourKind::ScreenBackground,
        ),
        _ => return None,
    };
    Some((ResourceLocation::new("minecraft", path), kind))
}
fn edit_box_argb_to_rgba(argb: i32) -> [f32; 4] {
    let argb = argb as u32;
    [
        ((argb >> 16) & 0xFF) as f32 / 255.0,
        ((argb >> 8) & 0xFF) as f32 / 255.0,
        (argb & 0xFF) as f32 / 255.0,
        ((argb >> 24) & 0xFF) as f32 / 255.0,
    ]
}
#[derive(Clone, Debug)]
struct TooltipLine {
    text: String,
    color: [f32; 4],
    italic: bool,
    styled: Option<String>,
    color_override: bool,
}
#[derive(Clone, Debug)]
enum TooltipImage {
    Bundle(BundleTooltipContents),
    #[cfg(feature = "extensions")]
    Extension(mod_tooltip::Presentation),
}
impl TooltipImage {
    fn width(&self, _font: &FontRenderer) -> i32 {
        match self {
            Self::Bundle(_) => BUNDLE_TOOLTIP_WIDTH,
            #[cfg(feature = "extensions")]
            Self::Extension(view) => view.inline_size().0,
        }
    }
    fn height(&self, font: &FontRenderer) -> i32 {
        match self {
            Self::Bundle(contents) => contents.tooltip_height(font),
            #[cfg(feature = "extensions")]
            Self::Extension(view) => view.inline_size().1,
        }
    }
    fn show_tooltip_with_item_in_hand(&self) -> bool {
        matches!(self, Self::Bundle(_))
    }
}
#[derive(Clone, Debug)]
enum TooltipComponent {
    Text(TooltipLine),
    Image(TooltipImage),
}
impl TooltipComponent {
    fn width(&self, font: &FontRenderer) -> i32 {
        match self {
            Self::Text(line) => formatted_chat_width(font, &line.styled_text(), 1.0).ceil() as i32,
            Self::Image(image) => image.width(font),
        }
    }
    fn height(&self, font: &FontRenderer) -> i32 {
        match self {
            Self::Text(_) => JAVA_TOOLTIP_TEXT_LINE_HEIGHT,
            Self::Image(image) => image.height(font),
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct DeferredTooltip {
    components: Vec<TooltipComponent>,
    style: Option<ResourceLocation>,
    #[cfg(feature = "extensions")]
    fallback: Option<Box<DeferredTooltip>>,
}
impl DeferredTooltip {
    fn new(
        lines: Vec<TooltipLine>,
        image: Option<TooltipImage>,
        style: Option<ResourceLocation>,
    ) -> Option<Self> {
        let mut components: Vec<_> = lines.into_iter().map(TooltipComponent::Text).collect();
        if let Some(image) = image {
            let image_index = usize::from(!components.is_empty());
            components.insert(image_index, TooltipComponent::Image(image));
        }
        (!components.is_empty()).then_some(Self {
            components,
            style,
            #[cfg(feature = "extensions")]
            fallback: None,
        })
    }
    fn text(lines: Vec<TooltipLine>) -> Option<Self> {
        Self::new(lines, None, None)
    }
    fn width(&self, font: &FontRenderer) -> i32 {
        self.components
            .iter()
            .map(|component| component.width(font))
            .max()
            .unwrap_or(0)
    }
    fn height(&self, font: &FontRenderer) -> i32 {
        let sum = self
            .components
            .iter()
            .map(|component| component.height(font))
            .sum::<i32>();
        if self.components.len() == 1 {
            sum - JAVA_TOOLTIP_FIRST_LINE_GAP
        } else {
            sum
        }
    }
}
#[derive(Default)]
struct DeferredTooltipSlot {
    tooltip: Option<DeferredTooltip>,
}
impl DeferredTooltipSlot {
    fn set_for_next_frame(&mut self, tooltip: Option<DeferredTooltip>, replace_existing: bool) {
        let Some(tooltip) = tooltip else {
            return;
        };
        if self.tooltip.is_none() || replace_existing {
            self.tooltip = Some(tooltip);
        }
    }
    fn take(&mut self) -> Option<DeferredTooltip> {
        self.tooltip.take()
    }
}
const BUNDLE_TOOLTIP_WIDTH: i32 = 96;
const BUNDLE_TOOLTIP_SLOT_SIZE: i32 = 24;
const BUNDLE_TOOLTIP_GRID_COLUMNS: usize = 4;
const BUNDLE_TOOLTIP_MAX_SLOTS: usize = 12;
const BUNDLE_TOOLTIP_PROGRESS_HEIGHT: i32 = 13;
const BUNDLE_TOOLTIP_PROGRESS_FILL_MAX: i32 = 94;
const BUNDLE_TOOLTIP_PROGRESS_MARGIN_Y: i32 = 4;
#[derive(Clone, Debug)]
struct BundleTooltipContents {
    items: Vec<ItemStack>,
    selected_item: Option<usize>,
}
impl BundleTooltipContents {
    fn from_component(item: &ItemStack) -> Option<Self> {
        let DataComponent::Generic {
            value: Tag::List(entries),
            ..
        } = item.components.get("minecraft:bundle_contents")?
        else {
            return None;
        };
        let mut items = Vec::with_capacity(entries.len());
        for entry in entries {
            items.push(bundle_item_stack_template_from_component_tag(entry)?);
        }
        Some(Self {
            items,
            selected_item: None,
        })
    }
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    fn number_of_items_to_show(&self) -> usize {
        let number_of_item_stacks = self.items.len();
        let available_items_to_show = if number_of_item_stacks > BUNDLE_TOOLTIP_MAX_SLOTS {
            BUNDLE_TOOLTIP_MAX_SLOTS - 1
        } else {
            BUNDLE_TOOLTIP_MAX_SLOTS
        };
        let items_on_non_full_row = number_of_item_stacks % BUNDLE_TOOLTIP_GRID_COLUMNS;
        let empty_space_on_non_full_row = if items_on_non_full_row == 0 {
            0
        } else {
            BUNDLE_TOOLTIP_GRID_COLUMNS - items_on_non_full_row
        };
        number_of_item_stacks.min(available_items_to_show - empty_space_on_non_full_row)
    }
    fn grid_rows(&self) -> usize {
        self.items
            .len()
            .min(BUNDLE_TOOLTIP_MAX_SLOTS)
            .div_ceil(BUNDLE_TOOLTIP_GRID_COLUMNS)
    }
    fn item_grid_height(&self) -> i32 {
        i32::try_from(self.grid_rows()).unwrap_or(i32::MAX) * BUNDLE_TOOLTIP_SLOT_SIZE
    }
    fn empty_description_lines(&self, font: &FontRenderer) -> Vec<String> {
        split_formatted_chat_lines(
            font,
            &crate::ui::i18n::translate("item.minecraft.bundle.empty.description"),
            BUNDLE_TOOLTIP_WIDTH as f32,
        )
    }
    fn tooltip_height(&self, font: &FontRenderer) -> i32 {
        if self.is_empty() {
            let description_height =
                i32::try_from(self.empty_description_lines(font).len()).unwrap_or(i32::MAX) * 9;
            description_height + BUNDLE_TOOLTIP_PROGRESS_HEIGHT + 8
        } else {
            self.item_grid_height() + BUNDLE_TOOLTIP_PROGRESS_HEIGHT + 8
        }
    }
    fn weight(&self) -> Option<BundleWeight> {
        self.items
            .iter()
            .try_fold(BundleWeight::ZERO, |weight, item| {
                let item_weight = bundle_item_weight(item)?;
                let total_item_weight = item_weight.multiply_by_count(item.count)?;
                weight.checked_add(total_item_weight)
            })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BundleWeight {
    numerator: u64,
    denominator: u64,
}
impl BundleWeight {
    const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };
    const ONE: Self = Self {
        numerator: 1,
        denominator: 1,
    };
    fn one_over(denominator: i32) -> Option<Self> {
        let denominator = u64::try_from(denominator).ok()?;
        (denominator > 0).then_some(Self {
            numerator: 1,
            denominator,
        })
    }
    fn checked_add(self, other: Self) -> Option<Self> {
        let divisor = gcd(self.denominator, other.denominator);
        let left_factor = other.denominator / divisor;
        let right_factor = self.denominator / divisor;
        let numerator = self
            .numerator
            .checked_mul(left_factor)?
            .checked_add(other.numerator.checked_mul(right_factor)?)?;
        let denominator = self.denominator.checked_mul(left_factor)?;
        Some(Self::reduced(numerator, denominator))
    }
    fn multiply_by_count(self, count: i32) -> Option<Self> {
        let count = u64::try_from(count).ok()?;
        Some(Self::reduced(
            self.numerator.checked_mul(count)?,
            self.denominator,
        ))
    }
    fn plus_bundle_container_weight(self) -> Option<Self> {
        self.checked_add(Self::one_over(16)?)
    }
    fn is_zero(self) -> bool {
        self.numerator == 0
    }
    fn is_full_or_more(self) -> bool {
        self.numerator >= self.denominator
    }
    fn multiply_and_truncate(self, factor: i32) -> Option<i32> {
        let factor = u64::try_from(factor).ok()?;
        let value = self.numerator.checked_mul(factor)? / self.denominator;
        i32::try_from(value).ok()
    }
    fn reduced(numerator: u64, denominator: u64) -> Self {
        debug_assert!(denominator > 0);
        let divisor = gcd(numerator, denominator);
        Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        }
    }
}
fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}
fn bundle_item_weight(item: &ItemStack) -> Option<BundleWeight> {
    if let Some(contents) = BundleTooltipContents::from_component(item) {
        return contents.weight()?.plus_bundle_container_weight();
    }
    if bundle_item_has_bees(item) {
        return Some(BundleWeight::ONE);
    }
    BundleWeight::one_over(item.get_max_stack_size())
}
fn bundle_item_has_bees(item: &ItemStack) -> bool {
    matches!(
        item.components.get("minecraft:bees"), Some(DataComponent::Generic { value :
        Tag::List(occupants), .. }) if ! occupants.is_empty()
    )
}
#[allow(dead_code)]
fn bundle_tooltip_image(item: &ItemStack) -> Option<TooltipImage> {
    bundle_tooltip_image_with_selected(item, None)
}
fn bundle_tooltip_image_with_selected(
    item: &ItemStack,
    selected_item: Option<usize>,
) -> Option<TooltipImage> {
    if item.item.location.namespace != "minecraft"
        || !is_vanilla_bundle_item_path(&item.item.location.path)
        || tooltip_display_hides(item)
        || tooltip_component_hidden(item, "minecraft:bundle_contents")
    {
        return None;
    }
    let mut contents = bundle_contents_for_item(item)?;
    contents.selected_item = selected_item.filter(|&index| index < contents.items.len());
    Some(TooltipImage::Bundle(contents))
}
fn bundle_contents_for_item(item: &ItemStack) -> Option<BundleTooltipContents> {
    if item.item.location.namespace != "minecraft"
        || !is_vanilla_bundle_item_path(&item.item.location.path)
    {
        return None;
    }
    BundleTooltipContents::from_component(item)
}
fn java_scroll_signum(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}
fn next_bundle_scroll_selection(wheel: i32, current_selected: i32, limit: usize) -> i32 {
    let limit = i32::try_from(limit).expect("bundle tooltip item count fits i32");
    debug_assert!(limit > 0);
    let mut selected = (current_selected - wheel.signum()).max(-1);
    while selected < 0 {
        selected += limit;
    }
    while selected >= limit {
        selected -= limit;
    }
    selected
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TooltipLayout {
    text_x: i32,
    text_y: i32,
    text_width: i32,
    text_height: i32,
    background_x: i32,
    background_y: i32,
    background_width: i32,
    background_height: i32,
}
const JAVA_TOOLTIP_MOUSE_OFFSET: i32 = 12;
const JAVA_TOOLTIP_PADDING: i32 = 3;
const JAVA_TOOLTIP_MARGIN: i32 = 9;
const JAVA_TOOLTIP_TEXT_LINE_HEIGHT: i32 = 10;
const JAVA_TOOLTIP_FIRST_LINE_GAP: i32 = 2;
#[allow(dead_code)]
fn java_tooltip_text_height(line_count: usize) -> i32 {
    let line_count = i32::try_from(line_count).unwrap_or(i32::MAX);
    if line_count == 1 {
        JAVA_TOOLTIP_TEXT_LINE_HEIGHT - JAVA_TOOLTIP_FIRST_LINE_GAP
    } else {
        line_count.saturating_mul(JAVA_TOOLTIP_TEXT_LINE_HEIGHT)
    }
}
fn default_tooltip_layout(
    mouse_x: i32,
    mouse_y: i32,
    screen_width: i32,
    screen_height: i32,
    text_width: i32,
    text_height: i32,
) -> TooltipLayout {
    let mut text_x = mouse_x + JAVA_TOOLTIP_MOUSE_OFFSET;
    let mut text_y = mouse_y - JAVA_TOOLTIP_MOUSE_OFFSET;
    if text_x + text_width > screen_width {
        text_x = (text_x - 2 * JAVA_TOOLTIP_MOUSE_OFFSET - text_width).max(4);
    }
    let padded_height = text_height + JAVA_TOOLTIP_PADDING;
    if text_y + padded_height > screen_height {
        text_y = screen_height - padded_height;
    }
    let frame_padding = JAVA_TOOLTIP_PADDING + JAVA_TOOLTIP_MARGIN;
    TooltipLayout {
        text_x,
        text_y,
        text_width,
        text_height,
        background_x: text_x - frame_padding,
        background_y: text_y - frame_padding,
        background_width: text_width + 2 * frame_padding,
        background_height: text_height + 2 * frame_padding,
    }
}
fn tooltip_background_bounds(origin: [i32; 2], size: [i32; 2]) -> [i32; 4] {
    let margin = JAVA_TOOLTIP_PADDING + JAVA_TOOLTIP_MARGIN;
    [
        origin[0] - margin,
        origin[1] - margin,
        size[0] + 2 * margin,
        size[1] + 2 * margin,
    ]
}
fn render_tooltip_background(
    renderer: &mut GuiRenderer,
    origin: [i32; 2],
    size: [i32; 2],
    style: Option<&ResourceLocation>,
) {
    let [x, y, width, height] = tooltip_background_bounds(origin, size);
    for part in ["background", "frame"] {
        renderer.draw_sprite(
            x as f32,
            y as f32,
            width as f32,
            height as f32,
            &tooltip_sprite(style, part),
            [1.0; 4],
        );
    }
}
#[derive(Debug)]
pub enum ContainerKeyboardAction {
    Unhandled,
    Handled,
    Close,
    Clicks(Vec<ServerboundContainerClickPacket>),
}
impl TooltipLine {
    fn new(text: String, color: [f32; 4]) -> Self {
        Self {
            text,
            color,
            italic: false,
            styled: None,
            color_override: false,
        }
    }
    fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
    fn with_styled_text(mut self, styled: String) -> Self {
        self.styled = (!styled.is_empty()).then_some(styled);
        self
    }
    #[cfg(feature = "extensions")]
    fn with_color_override(mut self) -> Self {
        self.color_override = true;
        self
    }
    fn styled_text(&self) -> String {
        if let Some(styled) = &self.styled {
            return styled.clone();
        }
        let red = (self.color[0].clamp(0.0, 1.0) * 255.0).round() as u8;
        let green = (self.color[1].clamp(0.0, 1.0) * 255.0).round() as u8;
        let blue = (self.color[2].clamp(0.0, 1.0) * 255.0).round() as u8;
        let mut text = format!("\u{1b}[38;2;{red};{green};{blue}m");
        if self.italic {
            text.push_str("\u{1b}[3m");
        }
        text.push_str(&self.text);
        text.push_str("\u{1b}[m");
        text
    }
}
#[derive(Clone, Debug)]
pub struct ContainerScreenState {
    #[cfg(feature = "extensions")]
    pub tooltip_modifiers: [bool; 3],
    pub container_id: i32,
    pub menu_type: MenuType,
    pub title: String,
    pub menu: Menu,
    pub state_id: i32,
    pub attribute_base_values: Option<(f64, f64)>,
    pub image_width: i32,
    pub image_height: i32,
    pub left_pos: i32,
    pub top_pos: i32,
    pub slot_coords: Vec<(i32, i32)>,
    pub hovered_slot_idx: Option<usize>,
    pub last_click_slot_idx: Option<usize>,
    pub double_click: bool,
    pub last_quick_moved: ItemStack,
    pub skip_next_release: bool,
    pub carried_item: ItemStack,
    pub is_quick_crafting: bool,
    pub quick_craft_slots: HashSet<usize>,
    pub quick_craft_type: i32,
    pub quick_craft_button: MouseButton,
    pub quick_crafting_remainder: i32,
    pub background_texture: ResourceLocation,
    pub mount_entity_kind: Option<String>,
    pub creative_scroll_row: i32,
    pub pending_creative_clicks: Vec<ServerboundSetCreativeModeSlotPacket>,
    pub pending_slot_state_changes: Vec<(u32, i32, bool)>,
    pub is_dragging_scrollbar: bool,
    pub creative_state: Option<CreativeInventoryState>,
    pub creative_search_box: Option<el_gui::components::edit_box::EditBox>,
    creative_search_tab_sync_pending: bool,
    creative_search_tab_changed: bool,
    pub pressed_creative_tab: Option<String>,
    pub data: BTreeMap<usize, i32>,
    bundle_selected_items: BTreeMap<usize, usize>,
    bundle_scroll_accum_x: f64,
    bundle_scroll_accum_y: f64,
    pending_bundle_item_selections: Vec<(i32, u32)>,
    bundle_item_selection_connection_available: bool,
    deferred_tooltip: Option<DeferredTooltip>,
}
impl ContainerScreenState {
    pub fn new(
        container_id: i32,
        menu_type: MenuType,
        title: String,
        mut menu: Menu,
        state_id: i32,
        carried_item: ItemStack,
    ) -> Self {
        let mut creative_state = None;
        if menu_type == MenuType::Creative {
            let state = CreativeInventoryState::new(false, false);
            state.apply_to_menu(&mut menu);
            creative_state = Some(state);
        }
        let layout = ScreenLayout::for_menu(menu_type, menu.slots.len());
        let image_width = layout.image_width;
        let image_height = layout.image_height;
        let background_texture = creative_state
            .as_ref()
            .map(CreativeInventoryState::selected_background_texture)
            .unwrap_or_else(|| layout.background.clone());
        let slot_coords = if menu_type == MenuType::Creative {
            menu.slot_positions
                .iter()
                .map(|pos| (pos.x, pos.y))
                .collect()
        } else {
            layout
                .slot_positions
                .iter()
                .map(|pos| (pos.x, pos.y))
                .collect()
        };
        let mut state = Self {
            #[cfg(feature = "extensions")]
            tooltip_modifiers: [false; 3],
            container_id,
            menu_type,
            title,
            menu,
            state_id,
            image_width,
            image_height,
            left_pos: 0,
            top_pos: 0,
            slot_coords,
            hovered_slot_idx: None,
            attribute_base_values: None,
            last_click_slot_idx: None,
            double_click: false,
            last_quick_moved: ItemStack::empty(),
            skip_next_release: false,
            carried_item,
            is_quick_crafting: false,
            quick_craft_slots: HashSet::new(),
            quick_craft_type: 0,
            quick_craft_button: MouseButton::Left,
            quick_crafting_remainder: 0,
            background_texture,
            mount_entity_kind: None,
            creative_scroll_row: 0,
            pending_creative_clicks: Vec::new(),
            pending_slot_state_changes: Vec::new(),
            is_dragging_scrollbar: false,
            creative_state,
            creative_search_box: None,
            creative_search_tab_sync_pending: false,
            creative_search_tab_changed: false,
            pressed_creative_tab: None,
            data: BTreeMap::new(),
            bundle_selected_items: BTreeMap::new(),
            bundle_scroll_accum_x: 0.0,
            bundle_scroll_accum_y: 0.0,
            pending_bundle_item_selections: Vec::new(),
            bundle_item_selection_connection_available: false,
            deferred_tooltip: None,
        };
        if menu_type == MenuType::Creative {
            state.refresh_creative_layout();
        }
        state
    }
    pub fn set_data(&mut self, id: usize, value: i32) {
        if self.menu_type == MenuType::Crafter && id < 9 {
            self.menu.set_slot_enabled(id, value == 0);
        }
        self.data.insert(id, value);
    }
    pub fn take_pending_slot_state_changes(&mut self) -> Vec<(u32, i32, bool)> {
        std::mem::take(&mut self.pending_slot_state_changes)
    }
    pub fn update_layout(&mut self, logical_w: f32, logical_h: f32) {
        self.left_pos = (logical_w as i32 - self.image_width) / 2;
        self.top_pos = (logical_h as i32 - self.image_height) / 2;
    }
    fn request_creative_search_box_tab_sync(&mut self, tab_changed: bool) {
        self.creative_search_tab_sync_pending = true;
        self.creative_search_tab_changed |= tab_changed;
    }
    pub fn handle_mouse_move(&mut self, mouse_x: f32, mouse_y: f32) {
        self.refresh_hovered_slot(mouse_x, mouse_y);
        if self.is_quick_crafting && !self.carried_item.is_empty() {
            if let Some(slot_idx) = self.hovered_slot_idx {
                if self.should_add_slot_to_quick_craft(slot_idx)
                    && self.quick_craft_slots.insert(slot_idx)
                {
                    self.recalculate_quick_craft_remaining();
                }
            }
        }
        if self.menu_type == MenuType::Creative && self.is_dragging_scrollbar {
            if let Some(creative) = self.creative_state.as_mut() {
                creative.scroll_to_mouse_y(mouse_y, self.top_pos, &mut self.menu);
                self.creative_scroll_row = creative.row_index_for_scroll(creative.scroll_offs);
                self.refresh_slot_coords_from_menu();
            }
        }
    }
    pub fn set_bundle_item_selection_connection_available(&mut self, available: bool) {
        self.bundle_item_selection_connection_available = available;
    }
    fn slot_no_item_icon(
        &self,
        slot_idx: usize,
        draw_kind: SlotContourKind,
    ) -> Option<ResourceLocation> {
        let slot = self.menu.slots.get(slot_idx)?;
        slot_no_item_icon(
            slot.slot_type,
            slot.slot_idx,
            self.mount_entity_kind.as_deref(),
        )
        .and_then(|(icon, contour_kind)| (contour_kind == draw_kind).then_some(icon))
    }
    pub fn on_close(&mut self) {
        if let Some(hovered_slot) = self.hovered_slot_idx {
            self.unselect_bundle_item(hovered_slot);
        }
    }
    fn should_add_slot_to_quick_craft(&self, slot_idx: usize) -> bool {
        if slot_idx >= self.menu.slots.len() {
            return false;
        }
        let slot = &self.menu.slots[slot_idx];
        if slot.slot_type == el_inventory::menu::SlotType::CraftingResult {
            return false;
        }
        if self.quick_craft_type != 2
            && self.carried_item.count <= self.quick_craft_slots.len() as i32
        {
            return false;
        }
        let slot_item = self.menu.get_slot_item(slot_idx);
        if !slot_item.is_empty()
            && (slot_item.item.location != self.carried_item.item.location
                || slot_item.components != self.carried_item.components)
        {
            return false;
        }
        true
    }
    fn recalculate_quick_craft_remaining(&mut self) {
        if self.carried_item.is_empty() || !self.is_quick_crafting {
            return;
        }
        if self.quick_craft_type == 2 {
            self.quick_crafting_remainder = self.carried_item.get_max_stack_size();
            return;
        }
        let mut remainder = self.carried_item.count;
        for &slot_idx in &self.quick_craft_slots {
            let slot_item = self.menu.get_slot_item(slot_idx);
            let carry = if slot_item.is_empty() {
                0
            } else {
                slot_item.count
            };
            let max_size = self.quick_craft_slot_max_size(slot_idx);
            let place = quick_craft_place_count(
                self.quick_craft_slots.len(),
                self.quick_craft_type,
                &self.carried_item,
            );
            let new_count = (place + carry).min(max_size);
            remainder -= new_count - carry;
        }
        self.quick_crafting_remainder = remainder;
    }
    fn quick_craft_slot_max_size(&self, slot_idx: usize) -> i32 {
        let carried_max = self.carried_item.get_max_stack_size();
        let Some(slot) = self.menu.slots.get(slot_idx) else {
            return carried_max;
        };
        let slot_max = if matches!(
            slot.slot_type,
            SlotType::MountSaddle
                | SlotType::MountArmor
                | SlotType::BeaconPayment
                | SlotType::BrewingBottle
                | SlotType::SingleItem
                | SlotType::NonInteractive
        ) {
            1
        } else {
            carried_max
        };
        carried_max.min(slot_max)
    }
    fn can_item_quick_replace(&self, slot_idx: usize) -> bool {
        let slot_item = self.menu.get_slot_item(slot_idx);
        if slot_item.is_empty() {
            return true;
        }
        slot_item.item.location == self.carried_item.item.location
            && slot_item.components == self.carried_item.components
            && slot_item.count <= self.carried_item.get_max_stack_size()
    }
    fn menu_can_drag_to(&self, slot_idx: usize) -> bool {
        if self.menu_type != MenuType::Creative {
            return true;
        }
        let Some(slot) = self.menu.slots.get(slot_idx) else {
            return false;
        };
        slot.container_idx != 1 && slot.slot_type != SlotType::Trash
    }
    pub fn handle_mouse_click(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        button: MouseButton,
        is_double_click: bool,
        has_shift: bool,
    ) -> Option<Vec<ServerboundContainerClickPacket>> {
        let clicked_slot = self.get_slot_at(mouse_x, mouse_y);
        let clicked_outside = self.has_clicked_outside(mouse_x, mouse_y);
        if self.menu_type == MenuType::Crafter
            && matches!(button, MouseButton::Left | MouseButton::Right)
        {
            if let Some(slot_idx) = clicked_slot.filter(|slot_idx| *slot_idx < 9) {
                if self.menu.get_slot_item(slot_idx).is_empty() {
                    let new_state = if self.menu.is_slot_disabled(slot_idx) {
                        Some(true)
                    } else if self.carried_item.is_empty() {
                        Some(false)
                    } else {
                        None
                    };
                    if let Some(enabled) = new_state {
                        self.menu.set_slot_enabled(slot_idx, enabled);
                        self.data.insert(slot_idx, i32::from(!enabled));
                        self.pending_slot_state_changes.push((
                            slot_idx as u32,
                            self.container_id,
                            enabled,
                        ));
                    }
                }
            }
        }
        if self.menu_type == MenuType::Creative {
            if button == MouseButton::Left {
                if let Some(creative) = self.creative_state.as_mut() {
                    if let Some(tab) =
                        creative.tab_at(mouse_x, mouse_y, self.left_pos, self.top_pos)
                    {
                        self.pressed_creative_tab = Some(tab.id.to_string());
                        return None;
                    }
                    if creative.inside_scrollbar(mouse_x, mouse_y, self.left_pos, self.top_pos) {
                        self.is_dragging_scrollbar = creative.can_scroll();
                        creative.scroll_to_mouse_y(mouse_y, self.top_pos, &mut self.menu);
                        self.creative_scroll_row =
                            creative.row_index_for_scroll(creative.scroll_offs);
                        self.refresh_slot_coords_from_menu();
                        return None;
                    }
                }
            }
        }
        if self.menu_type == MenuType::Inventory {
            let button_x = self.left_pos as f32 + 104.0;
            let button_y = self.top_pos as f32 + self.image_height as f32 / 2.0 - 22.0;
            if mouse_x >= button_x
                && mouse_x < button_x + 20.0
                && mouse_y >= button_y
                && mouse_y < button_y + 18.0
            {
                return None;
            }
        }
        let mut slot_id = -1i16;
        if let Some(idx) = clicked_slot {
            slot_id = idx as i16;
        } else if clicked_outside {
            slot_id = -999;
        }
        self.double_click =
            clicked_slot.is_some() && self.last_click_slot_idx == clicked_slot && is_double_click;
        self.skip_next_release = false;
        let mut packets = Vec::new();
        if self.menu_type == MenuType::Creative {
            if self.double_click && clicked_slot.is_some() && button == MouseButton::Left {
                let idx = clicked_slot.unwrap();
                if has_shift {
                    if !self.last_quick_moved.is_empty() {
                        for i in 0..self.menu.slots.len() {
                            let target_item = self.menu.get_slot_item(i);
                            if !target_item.is_empty()
                                && target_item.item.location == self.last_quick_moved.item.location
                                && target_item.components == self.last_quick_moved.components
                            {
                                let _ = self.perform_slot_click_prediction(
                                    i as i16,
                                    0,
                                    ContainerInput::QuickMove,
                                );
                            }
                        }
                    }
                } else {
                    let _ = self.perform_slot_click_prediction(
                        idx as i16,
                        0,
                        ContainerInput::PickupAll,
                    );
                }
                self.double_click = false;
                self.skip_next_release = true;
                self.last_click_slot_idx = clicked_slot;
                return None;
            }
            if let Some(idx) = clicked_slot {
                if button == MouseButton::Middle {
                    let _ =
                        self.perform_slot_click_prediction(idx as i16, 2, ContainerInput::Clone);
                    self.skip_next_release = true;
                    self.last_click_slot_idx = clicked_slot;
                    return None;
                }
                let slot_type = self.menu.slots[idx].slot_type;
                if matches!(slot_type, SlotType::Creative | SlotType::Trash) {
                    if has_shift {
                        self.last_quick_moved = self.menu.get_slot_item(idx).clone();
                    }
                    let container_input = if has_shift && slot_type == SlotType::Trash {
                        ContainerInput::QuickMove
                    } else if has_shift {
                        ContainerInput::QuickMove
                    } else {
                        ContainerInput::Pickup
                    };
                    let button_num = match button {
                        MouseButton::Left => 0i8,
                        MouseButton::Right => 1i8,
                        _ => 0i8,
                    };
                    let _ =
                        self.perform_slot_click_prediction(idx as i16, button_num, container_input);
                    self.skip_next_release = true;
                    self.last_click_slot_idx = clicked_slot;
                    return None;
                } else if has_shift && button == MouseButton::Left {
                    if let Some(creative) = &self.creative_state {
                        if creative.selected_kind() != CreativeTabKind::Inventory {
                            self.menu.set_slot_item(idx, ItemStack::empty());
                            if let Some(slot_num) =
                                el_inventory::creative::creative_packet_slot(&self.menu.slots[idx])
                            {
                                self.pending_creative_clicks.push(
                                    ServerboundSetCreativeModeSlotPacket {
                                        slot_num,
                                        item_stack: ItemStack::empty(),
                                    },
                                );
                            }
                            self.skip_next_release = true;
                            self.last_click_slot_idx = clicked_slot;
                            return None;
                        }
                    }
                }
            }
        }
        let cloning =
            button == MouseButton::Middle && self.menu_type == MenuType::Creative && slot_id != -1;
        if button == MouseButton::Left || button == MouseButton::Right || cloning {
            if slot_id != -1 {
                if self.carried_item.is_empty() {
                    let container_input = if cloning {
                        ContainerInput::Clone
                    } else if has_shift && slot_id != -999 {
                        if let Some(idx) = clicked_slot {
                            let item = self.menu.get_slot_item(idx);
                            self.last_quick_moved = item.clone();
                        }
                        ContainerInput::QuickMove
                    } else if slot_id == -999 {
                        ContainerInput::Throw
                    } else {
                        ContainerInput::Pickup
                    };
                    let button_num = match button {
                        MouseButton::Left => 0i8,
                        MouseButton::Right => 1i8,
                        MouseButton::Middle => 2i8,
                        _ => 0i8,
                    };
                    if let Some(packet) =
                        self.perform_slot_click_prediction(slot_id, button_num, container_input)
                    {
                        packets.push(packet);
                    }
                    self.skip_next_release = true;
                } else {
                    self.is_quick_crafting = true;
                    self.quick_craft_button = button;
                    self.quick_craft_slots.clear();
                    self.quick_craft_type = match button {
                        MouseButton::Left => 0,
                        MouseButton::Right => 1,
                        MouseButton::Middle => 2,
                        _ => 0,
                    };
                    if let Some(idx) = clicked_slot {
                        if self.should_add_slot_to_quick_craft(idx)
                            && self.quick_craft_slots.insert(idx)
                        {
                            self.recalculate_quick_craft_remaining();
                        }
                    }
                }
            }
        }
        self.last_click_slot_idx = clicked_slot;
        if packets.is_empty() {
            None
        } else {
            Some(packets)
        }
    }
    pub fn handle_mouse_release(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        button: MouseButton,
        has_shift: bool,
    ) -> Option<Vec<ServerboundContainerClickPacket>> {
        if self.menu_type == MenuType::Creative {
            self.is_dragging_scrollbar = false;
            if button == MouseButton::Left {
                let mut search_box_tab_changed = None;
                if let Some(creative) = self.creative_state.as_mut() {
                    let clicked_tab =
                        creative.tab_at(mouse_x, mouse_y, self.left_pos, self.top_pos);
                    if let Some(tab) = clicked_tab {
                        if self.pressed_creative_tab.as_deref() == Some(tab.id) {
                            let tab_changed = creative.selected_tab_id() != tab.id;
                            if creative.select_tab(tab.id, &mut self.menu) {
                                search_box_tab_changed = Some(tab_changed);
                            }
                            self.creative_scroll_row =
                                creative.row_index_for_scroll(creative.scroll_offs);
                            self.refresh_creative_layout();
                            self.pressed_creative_tab = None;
                            if let Some(tab_changed) = search_box_tab_changed {
                                self.request_creative_search_box_tab_sync(tab_changed);
                            }
                            return None;
                        }
                    }
                }
                if self.pressed_creative_tab.take().is_some() {
                    return None;
                }
            }
        }
        let clicked_slot = self.get_slot_at(mouse_x, mouse_y);
        let clicked_outside = self.has_clicked_outside(mouse_x, mouse_y);
        let mut slot_id = -1i16;
        if let Some(idx) = clicked_slot {
            slot_id = idx as i16;
        } else if clicked_outside {
            slot_id = -999;
        }
        let mut packets = Vec::new();
        if self.double_click && clicked_slot.is_some() && button == MouseButton::Left {
            if has_shift {
                if !self.last_quick_moved.is_empty() {
                    for i in 0..self.menu.slots.len() {
                        let target_item = self.menu.get_slot_item(i);
                        if !target_item.is_empty()
                            && target_item.item.location == self.last_quick_moved.item.location
                            && target_item.components == self.last_quick_moved.components
                        {
                            if let Some(packet) = self.perform_slot_click_prediction(
                                i as i16,
                                0,
                                ContainerInput::QuickMove,
                            ) {
                                packets.push(packet);
                            }
                        }
                    }
                }
            } else {
                if let Some(packet) =
                    self.perform_slot_click_prediction(slot_id, 0, ContainerInput::PickupAll)
                {
                    packets.push(packet);
                }
            }
            self.double_click = false;
        } else {
            if self.is_quick_crafting && self.quick_craft_button == button {
                let should_quickcraft = self.quick_craft_slots.len() > 1
                    || (self.quick_craft_type == 2
                        && self.menu_type == MenuType::Creative
                        && !self.quick_craft_slots.is_empty());
                if should_quickcraft {
                    let start_button = self.quick_craft_type << 2;
                    packets.push(ServerboundContainerClickPacket {
                        container_id: self.container_id,
                        state_id: self.state_id,
                        slot_num: -999,
                        button_num: start_button as i8,
                        container_input: ContainerInput::QuickCraft,
                        changed_slots: BTreeMap::new(),
                        carried_item: self.carried_item.clone(),
                    });
                    for &slot_idx in &self.quick_craft_slots {
                        let add_button = 1 | (self.quick_craft_type << 2);
                        packets.push(ServerboundContainerClickPacket {
                            container_id: self.container_id,
                            state_id: self.state_id,
                            slot_num: slot_idx as i16,
                            button_num: add_button as i8,
                            container_input: ContainerInput::QuickCraft,
                            changed_slots: BTreeMap::new(),
                            carried_item: self.carried_item.clone(),
                        });
                    }
                    let end_button = 2 | (self.quick_craft_type << 2);
                    let slots_before: Vec<ItemStack> = (0..self.menu.slots.len())
                        .map(|i| self.menu.get_slot_item(i).clone())
                        .collect();
                    self.apply_quick_craft_locally();
                    let mut changed_slots = BTreeMap::new();
                    for i in 0..self.menu.slots.len() {
                        let item_after = self.menu.get_slot_item(i);
                        if item_after != &slots_before[i] {
                            changed_slots.insert(i as i16, item_after.clone());
                        }
                    }
                    packets.push(ServerboundContainerClickPacket {
                        container_id: self.container_id,
                        state_id: self.state_id,
                        slot_num: -999,
                        button_num: end_button as i8,
                        container_input: ContainerInput::QuickCraft,
                        changed_slots,
                        carried_item: self.carried_item.clone(),
                    });
                } else if slot_id != -1 {
                    let button_num = match button {
                        MouseButton::Left => 0i8,
                        MouseButton::Right => 1i8,
                        _ => 0i8,
                    };
                    if let Some(packet) = self.perform_slot_click_prediction(
                        slot_id,
                        button_num,
                        ContainerInput::Pickup,
                    ) {
                        packets.push(packet);
                    }
                }
                self.is_quick_crafting = false;
                self.quick_craft_slots.clear();
            } else if !self.carried_item.is_empty() {
                if self.skip_next_release {
                    self.skip_next_release = false;
                } else if slot_id != -1 {
                    let container_input = if has_shift {
                        if let Some(idx) = clicked_slot {
                            self.last_quick_moved = self.menu.get_slot_item(idx).clone();
                        }
                        ContainerInput::QuickMove
                    } else {
                        ContainerInput::Pickup
                    };
                    let button_num = match button {
                        MouseButton::Left => 0i8,
                        MouseButton::Right => 1i8,
                        _ => 0i8,
                    };
                    if let Some(packet) =
                        self.perform_slot_click_prediction(slot_id, button_num, container_input)
                    {
                        packets.push(packet);
                    }
                }
            }
        }
        if packets.is_empty() {
            None
        } else {
            Some(packets)
        }
    }
    fn perform_slot_click_prediction(
        &mut self,
        slot_id: i16,
        button_num: i8,
        container_input: ContainerInput,
    ) -> Option<ServerboundContainerClickPacket> {
        if self.menu_type == MenuType::Creative && slot_id == -999 {
            if !self.carried_item.is_empty()
                && matches!(
                    container_input,
                    ContainerInput::Pickup | ContainerInput::Throw
                )
            {
                let dropped = if button_num == 0 {
                    let stack = self.carried_item.clone();
                    self.carried_item = ItemStack::empty();
                    stack
                } else {
                    let stack = self.carried_item.copy_with_count(1);
                    self.carried_item.count -= 1;
                    if self.carried_item.count <= 0 {
                        self.carried_item = ItemStack::empty();
                    }
                    stack
                };
                if !dropped.is_empty() {
                    self.pending_creative_clicks
                        .push(ServerboundSetCreativeModeSlotPacket {
                            slot_num: -1,
                            item_stack: dropped,
                        });
                }
            }
            return None;
        }
        if slot_id == -999 {
            let mut packet = ServerboundContainerClickPacket {
                container_id: self.container_id,
                state_id: self.state_id,
                slot_num: -999,
                button_num,
                container_input,
                changed_slots: BTreeMap::new(),
                carried_item: ItemStack::empty(),
            };
            if container_input == ContainerInput::Throw {
                if button_num == 0 {
                    if self.carried_item.count > 0 {
                        self.carried_item.count -= 1;
                        if self.carried_item.count == 0 {
                            self.carried_item = ItemStack::empty();
                        }
                    }
                } else {
                    self.carried_item = ItemStack::empty();
                }
                packet.carried_item = self.carried_item.clone();
            }
            return Some(packet);
        }
        let slot_idx = slot_id as usize;
        if slot_idx >= self.menu.slots.len() {
            return None;
        }
        if matches!(
            container_input,
            ContainerInput::QuickMove | ContainerInput::Swap
        ) {
            self.unselect_bundle_item(slot_idx);
        }
        let slots_before: Vec<ItemStack> = (0..self.menu.slots.len())
            .map(|i| self.menu.get_slot_item(i).clone())
            .collect();
        let click_type = match container_input {
            ContainerInput::QuickMove => ClickType::QuickMove,
            ContainerInput::Swap => ClickType::Swap {
                inventory_slot: button_num.max(0) as usize,
            },
            ContainerInput::Clone => ClickType::Clone,
            ContainerInput::Throw => ClickType::Throw {
                button: button_num as i32,
            },
            ContainerInput::PickupAll => ClickType::PickupAll,
            _ => ClickType::Pickup {
                button: button_num as i32,
            },
        };
        if self.menu_type == MenuType::Creative {
            if self.menu.slots[slot_idx].slot_type == SlotType::Trash
                && click_type == ClickType::QuickMove
            {
                for slot_num in 5..=45 {
                    self.pending_creative_clicks
                        .push(ServerboundSetCreativeModeSlotPacket {
                            slot_num,
                            item_stack: ItemStack::empty(),
                        });
                }
            }
            let creative_drop = if container_input == ContainerInput::Throw {
                let clicked = self.menu.get_slot_item(slot_idx);
                if clicked.is_empty() {
                    None
                } else if self.menu.slots[slot_idx].slot_type == SlotType::Creative {
                    Some(clicked.copy_with_count(if button_num == 0 {
                        1
                    } else {
                        clicked.get_max_stack_size()
                    }))
                } else {
                    Some(clicked.copy_with_count(if button_num == 0 { 1 } else { clicked.count }))
                }
            } else {
                None
            };
            let _ = self
                .menu
                .click_slot(slot_idx, click_type, &mut self.carried_item);
            if let Some(item_stack) = creative_drop {
                self.pending_creative_clicks
                    .push(ServerboundSetCreativeModeSlotPacket {
                        slot_num: -1,
                        item_stack,
                    });
            }
            for i in 0..self.menu.slots.len() {
                let item_after = self.menu.get_slot_item(i);
                if item_after != &slots_before[i] {
                    if let Some(slot_num) =
                        el_inventory::creative::creative_packet_slot(&self.menu.slots[i])
                    {
                        self.pending_creative_clicks
                            .push(ServerboundSetCreativeModeSlotPacket {
                                slot_num,
                                item_stack: item_after.clone(),
                            });
                    }
                }
            }
            return None;
        }
        let result = self
            .menu
            .click_slot(slot_idx, click_type, &mut self.carried_item);
        if result.is_ok() {
            let mut changed_slots = BTreeMap::new();
            for i in 0..self.menu.slots.len() {
                let item_after = self.menu.get_slot_item(i);
                if item_after != &slots_before[i] {
                    changed_slots.insert(i as i16, item_after.clone());
                }
            }
            Some(ServerboundContainerClickPacket {
                container_id: self.container_id,
                state_id: self.state_id,
                slot_num: slot_id,
                button_num,
                container_input,
                changed_slots,
                carried_item: self.carried_item.clone(),
            })
        } else {
            None
        }
    }
    fn apply_quick_craft_locally(&mut self) {
        if self.carried_item.is_empty() || self.quick_craft_slots.is_empty() {
            return;
        }
        let num_slots = self.quick_craft_slots.len();
        if self.quick_craft_type == 0 {
            let to_distribute = self.carried_item.count;
            let count_per_slot = to_distribute / num_slots as i32;
            let mut remainder = to_distribute % num_slots as i32;
            if count_per_slot > 0 {
                for &slot_idx in &self.quick_craft_slots {
                    let slot_item = self.menu.get_slot_item(slot_idx).clone();
                    let max_stack = self.carried_item.get_max_stack_size();
                    let carry = if slot_item.is_empty() {
                        0
                    } else {
                        slot_item.count
                    };
                    let mut added = count_per_slot;
                    let space = max_stack - carry;
                    if added > space {
                        added = space;
                        remainder += count_per_slot - space;
                    }
                    let mut new_stack = self.carried_item.clone();
                    new_stack.count = carry + added;
                    self.menu.set_slot_item(slot_idx, new_stack);
                }
            }
            self.carried_item.count = remainder;
            if self.carried_item.count <= 0 {
                self.carried_item = ItemStack::empty();
            }
        } else if self.quick_craft_type == 1 {
            for &slot_idx in &self.quick_craft_slots {
                if self.carried_item.is_empty() {
                    break;
                }
                let slot_item = self.menu.get_slot_item(slot_idx).clone();
                let max_stack = self.carried_item.get_max_stack_size();
                let carry = if slot_item.is_empty() {
                    0
                } else {
                    slot_item.count
                };
                if carry < max_stack {
                    let mut new_stack = self.carried_item.clone();
                    new_stack.count = carry + 1;
                    self.menu.set_slot_item(slot_idx, new_stack);
                    self.carried_item.count -= 1;
                }
            }
            if self.carried_item.count <= 0 {
                self.carried_item = ItemStack::empty();
            }
        } else if self.quick_craft_type == 2 && self.menu_type == MenuType::Creative {
            for &slot_idx in &self.quick_craft_slots {
                let max_stack = self.carried_item.get_max_stack_size();
                self.menu
                    .set_slot_item(slot_idx, self.carried_item.copy_with_count(max_stack));
            }
        }
    }
    fn menu_slot_is_active(&self, slot_idx: usize) -> bool {
        let _ = slot_idx;
        true
    }
    fn get_slot_at(&self, mouse_x: f32, mouse_y: f32) -> Option<usize> {
        let mx = mouse_x as i32 - self.left_pos;
        let my = mouse_y as i32 - self.top_pos;
        for (idx, &(x, y)) in self.slot_coords.iter().enumerate() {
            if mx >= x - 1
                && mx < x + 17
                && my >= y - 1
                && my < y + 17
                && self.menu_slot_is_active(idx)
            {
                return Some(idx);
            }
        }
        None
    }
    fn refresh_hovered_slot(&mut self, mouse_x: f32, mouse_y: f32) {
        let previously_hovered = self.hovered_slot_idx;
        let hovered = self.get_slot_at(mouse_x, mouse_y);
        self.hovered_slot_idx = hovered;
        if let Some(previously_hovered) = previously_hovered {
            if Some(previously_hovered) != hovered {
                self.unselect_bundle_item(previously_hovered);
            }
        }
    }
    fn has_clicked_outside(&self, mouse_x: f32, mouse_y: f32) -> bool {
        let mx = mouse_x as i32;
        let my = mouse_y as i32;
        let outside = mx < self.left_pos
            || my < self.top_pos
            || mx >= self.left_pos + self.image_width
            || my >= self.top_pos + self.image_height;
        if outside && self.menu_type == MenuType::Creative {
            if let Some(creative) = self.creative_state.as_ref() {
                return creative
                    .tab_at(mouse_x, mouse_y, self.left_pos, self.top_pos)
                    .is_none();
            }
        }
        outside
    }
    fn render_active_effects(
        &self,
        renderer: &mut GuiRenderer,
        font: &FontRenderer,
        player: &LocalPlayer,
        logical_w: f32,
    ) {
        let x0 = self.left_pos + self.image_width + 2;
        let available_width = logical_w.round() as i32 - x0;
        if player.active_effects.is_empty() || available_width < 32 {
            return;
        }
        let max_width = if available_width >= 120 {
            available_width - 7
        } else {
            32
        };
        let y_step = if player.active_effects.len() > 5 {
            132 / (player.active_effects.len() as i32 - 1)
        } else {
            33
        };
        let mut effects: Vec<(u32, ActiveMobEffect)> = player
            .active_effects
            .iter()
            .filter_map(|(&effect_id, &effect)| {
                mob_effect_path(effect_id).map(|_| (effect_id, effect))
            })
            .collect();
        effects.sort_by(|(left_id, left), (right_id, right)| {
            compare_mob_effects(*left_id, *left, *right_id, *right)
        });
        let mut y0 = self.top_pos;
        for (effect_id, effect) in effects {
            let path = mob_effect_path(effect_id).expect("filtered above");
            let name_key = format!("effect.minecraft.{path}");
            let mut name = crate::ui::i18n::translate(&name_key).to_string();
            if effect.amplifier >= 1 && effect.amplifier <= 9 {
                let level_key = format!("enchantment.level.{}", effect.amplifier + 1);
                name.push(' ');
                name.push_str(&crate::ui::i18n::translate(&level_key));
            }
            let duration = LocalPlayer::format_effect_duration(effect.duration);
            let name_width = 32 + font.measure_width(&name, 1.0).ceil() as i32 + 7;
            let duration_width = 32 + font.measure_width(&duration, 1.0).ceil() as i32 + 7;
            let texture_width = name_width.max(duration_width).min(max_width);
            let background = if effect.ambient {
                "container/inventory/effect_background_ambient"
            } else {
                "container/inventory/effect_background"
            };
            let background = ResourceLocation::new("minecraft", background);
            renderer.draw_sprite(
                x0 as f32,
                y0 as f32,
                texture_width as f32,
                32.0,
                &background,
                [1.0; 4],
            );
            let icon_path = format!("mob_effect/{path}");
            let icon = ResourceLocation::new("minecraft", &icon_path);
            renderer.draw_sprite(
                x0 as f32 + 7.0,
                y0 as f32 + 7.0,
                18.0,
                18.0,
                &icon,
                [1.0; 4],
            );
            let max_text_width = texture_width - 32 - 7;
            if max_text_width > 0 {
                let display_name = clip_effect_text(&name, font, max_text_width);
                renderer.draw_text_top(
                    font,
                    &display_name,
                    x0 as f32 + 32.0,
                    y0 as f32 + 7.0,
                    [1.0, 1.0, 1.0, 1.0],
                    1.0,
                );
                renderer.draw_text_top(
                    font,
                    &duration,
                    x0 as f32 + 32.0,
                    y0 as f32 + 16.0,
                    [0.5019608, 0.5019608, 0.5019608, 1.0],
                    1.0,
                );
            }
            y0 += y_step;
        }
    }
    pub fn render(
        &mut self,
        renderer: &mut GuiRenderer,
        font: &FontRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        resource_manager: &crate::resources::manager::ClientResourceManager,
        logical_w: f32,
        logical_h: f32,
        mouse_x: f32,
        mouse_y: f32,
        defer_tooltip: bool,
        player: &LocalPlayer,
    ) {
        self.update_layout(logical_w, logical_h);
        if self.menu_type == MenuType::Creative {
            self.sync_creative_search_box_if_needed(font, util_get_millis());
        }
        renderer.draw_gradient_rect(
            0.0,
            0.0,
            logical_w,
            logical_h,
            [16.0 / 255.0, 16.0 / 255.0, 16.0 / 255.0, 192.0 / 255.0],
            [16.0 / 255.0, 16.0 / 255.0, 16.0 / 255.0, 208.0 / 255.0],
        );
        if self.menu_type == MenuType::Creative {
            self.refresh_creative_layout();
            self.render_creative_tabs(renderer, font, false, mouse_x, mouse_y);
        }
        self.refresh_hovered_slot(mouse_x, mouse_y);
        let tex_idx = renderer.get_or_load_custom_texture(
            device,
            queue,
            texture_bind_group_layout,
            resource_manager,
            &self.background_texture,
        );
        let layout = ScreenLayout::for_menu(self.menu_type, self.menu.slots.len());
        if let Some(split) = layout.chest_background_split {
            let u_max = self.image_width as f32 / 256.0;
            renderer.draw_custom_texture_region(
                self.left_pos as f32,
                self.top_pos as f32,
                self.image_width as f32,
                split.top_height as f32,
                0.0,
                0.0,
                u_max,
                split.top_height as f32 / 256.0,
                tex_idx,
                [1.0, 1.0, 1.0, 1.0],
            );
            renderer.draw_custom_texture_region(
                self.left_pos as f32,
                (self.top_pos + split.top_height) as f32,
                self.image_width as f32,
                split.bottom_height as f32,
                0.0,
                split.bottom_source_y as f32 / 256.0,
                u_max,
                (split.bottom_source_y + split.bottom_height) as f32 / 256.0,
                tex_idx,
                [1.0, 1.0, 1.0, 1.0],
            );
        } else {
            let u_max = self.image_width as f32 / 256.0;
            let v_max = self.image_height as f32 / 256.0;
            renderer.draw_custom_texture_region(
                self.left_pos as f32,
                self.top_pos as f32,
                self.image_width as f32,
                self.image_height as f32,
                0.0,
                0.0,
                u_max,
                v_max,
                tex_idx,
                [1.0, 1.0, 1.0, 1.0],
            );
        }
        if self.menu_type == MenuType::Creative {
            self.render_creative_search_box(renderer, font, util_get_millis());
            self.render_creative_scroller(renderer);
            self.render_creative_tabs(renderer, font, true, mouse_x, mouse_y);
        }
        if self.menu_type == MenuType::Inventory {
            let button_x = self.left_pos as f32 + 104.0;
            let button_y = logical_h / 2.0 - 22.0;
            let is_hovered = mouse_x >= button_x
                && mouse_x < button_x + 20.0
                && mouse_y >= button_y
                && mouse_y < button_y + 18.0;
            let sprite_id = if is_hovered {
                ResourceLocation::new("minecraft", "recipe_book/button_highlighted")
            } else {
                ResourceLocation::new("minecraft", "recipe_book/button")
            };
            renderer.draw_sprite(
                button_x,
                button_y,
                20.0,
                18.0,
                &sprite_id,
                [1.0, 1.0, 1.0, 1.0],
            );
        }
        let title_color = vanilla_argb_color(VANILLA_CONTAINER_TITLE_COLOR_ARGB);
        if self.menu_type == MenuType::Creative {
            if let Some(creative) = self.creative_state.as_ref() {
                let tab = creative.selected_tab();
                if tab.show_title {
                    let title = crate::ui::i18n::translate(tab.translation_key);
                    renderer.draw_text_top(
                        font,
                        &title,
                        self.left_pos as f32 + layout.title_label_x as f32,
                        self.top_pos as f32 + layout.title_label_y as f32,
                        title_color,
                        1.0,
                    );
                }
            }
        } else {
            renderer.draw_text_top(
                font,
                &self.title,
                self.left_pos as f32 + layout.title_label_x as f32,
                self.top_pos as f32 + layout.title_label_y as f32,
                title_color,
                1.0,
            );
        }
        if self.menu_type != MenuType::Creative
            && self.menu_type != MenuType::Beacon
            && self.menu_type != MenuType::Inventory
        {
            let inv_title = crate::ui::i18n::translate("container.inventory");
            renderer.draw_text_top(
                font,
                &inv_title,
                self.left_pos as f32 + layout.inventory_label_x as f32,
                self.top_pos as f32 + layout.inventory_label_y as f32,
                title_color,
                1.0,
            );
        }
        if self.menu_type == MenuType::Inventory {
            self.render_active_effects(renderer, font, player, logical_w);
        }
        for i in 0..self.menu.slots.len() {
            if i >= self.slot_coords.len() {
                break;
            }
            let (sx, sy) = self.slot_coords[i];
            let slot_x = self.left_pos as f32 + sx as f32;
            let slot_y = self.top_pos as f32 + sy as f32;
            let mut quick_craft_ghost: Option<(ItemStack, Option<String>)> = None;
            let mut skip_slot_item = false;
            if self.is_quick_crafting
                && self.quick_craft_slots.contains(&i)
                && !self.carried_item.is_empty()
            {
                if self.quick_craft_slots.len() == 1 {
                    skip_slot_item = true;
                } else if self.can_item_quick_replace(i) && self.menu_can_drag_to(i) {
                    let slot_item = self.menu.get_slot_item(i);
                    let carry = if slot_item.is_empty() {
                        0
                    } else {
                        slot_item.count
                    };
                    let place = quick_craft_place_count(
                        self.quick_craft_slots.len(),
                        self.quick_craft_type,
                        &self.carried_item,
                    );
                    let mut new_count = place + carry;
                    let mut overflow: Option<String> = None;
                    let max_size = self.quick_craft_slot_max_size(i);
                    if new_count > max_size {
                        new_count = max_size;
                        overflow = Some(max_size.to_string());
                    }
                    let ghost = self.carried_item.copy_with_count(new_count);
                    quick_craft_ghost = Some((ghost, overflow));
                } else {
                    self.quick_craft_slots.remove(&i);
                    self.recalculate_quick_craft_remaining();
                }
            }
            let item = self.menu.get_slot_item(i);
            if self.menu_type == MenuType::Crafter && i < 9 && self.menu.is_slot_disabled(i) {
                renderer.draw_sprite(
                    slot_x - 1.0,
                    slot_y - 1.0,
                    18.0,
                    18.0,
                    &ResourceLocation::new("minecraft", "container/crafter/disabled_slot"),
                    [1.0; 4],
                );
                continue;
            }
            let background_icon = if item.is_empty() {
                self.slot_no_item_icon(i, SlotContourKind::ScreenBackground)
            } else {
                None
            };
            if let Some(icon) = &background_icon {
                renderer.draw_sprite(slot_x, slot_y, 16.0, 16.0, icon, [1.0; 4]);
            }
            let no_item_icon = if item.is_empty()
                && !skip_slot_item
                && quick_craft_ghost.is_none()
                && !self.menu.is_slot_disabled(i)
            {
                self.slot_no_item_icon(i, SlotContourKind::SlotStream)
            } else {
                None
            };
            if Some(i) == self.hovered_slot_idx {
                renderer.draw_sprite(
                    slot_x - 4.0,
                    slot_y - 4.0,
                    24.0,
                    24.0,
                    &ResourceLocation::new("minecraft", "container/slot_highlight_back"),
                    [1.0, 1.0, 1.0, 1.0],
                );
            }
            if let Some((ghost, overflow)) = &quick_craft_ghost {
                renderer.draw_rect(slot_x, slot_y, 16.0, 16.0, QUICK_CRAFT_FILL);
                let cooldown = player.item_cooldown_percent(ghost, 0.0);
                match overflow {
                    Some(max_size) => {
                        draw_gui_item(renderer, font, ghost, slot_x, slot_y, cooldown, false);
                        draw_gui_item_decorations_with_count_text(
                            renderer,
                            font,
                            ghost,
                            slot_x,
                            slot_y,
                            cooldown,
                            max_size,
                            QUICK_CRAFT_COUNT_YELLOW,
                        );
                    }
                    None => draw_gui_item(renderer, font, ghost, slot_x, slot_y, cooldown, true),
                }
            } else if let Some(icon) = &no_item_icon {
                renderer.draw_sprite(slot_x, slot_y, 16.0, 16.0, icon, [1.0; 4]);
            } else if !skip_slot_item {
                let cooldown = player.item_cooldown_percent(item, 0.0);
                draw_gui_item(renderer, font, item, slot_x, slot_y, cooldown, true);
            }
            if Some(i) == self.hovered_slot_idx {
                renderer.draw_sprite(
                    slot_x - 4.0,
                    slot_y - 4.0,
                    24.0,
                    24.0,
                    &ResourceLocation::new("minecraft", "container/slot_highlight_front"),
                    [1.0, 1.0, 1.0, 1.0],
                );
            }
        }
        if self.menu_type == MenuType::Crafter {
            let redstone_sprite = if self.data.get(&9) == Some(&1) {
                "container/crafter/powered_redstone"
            } else {
                "container/crafter/unpowered_redstone"
            };
            renderer.draw_sprite(
                self.left_pos as f32 + 97.0,
                self.top_pos as f32 + 35.0,
                16.0,
                16.0,
                &ResourceLocation::new("minecraft", redstone_sprite),
                [1.0; 4],
            );
        }
        if !self.carried_item.is_empty() {
            let cx = mouse_x - 8.0;
            let cy = mouse_y - 8.0;
            if self.is_quick_crafting && self.quick_craft_slots.len() > 1 {
                let remainder = self.quick_crafting_remainder.max(0);
                let carried_draw = self.carried_item.copy_with_count(remainder);
                if carried_draw.is_empty() {
                    draw_gui_item_decorations_with_count_text(
                        renderer,
                        font,
                        &carried_draw,
                        cx,
                        cy,
                        0.0,
                        "0",
                        QUICK_CRAFT_COUNT_YELLOW,
                    );
                } else {
                    let cooldown = player.item_cooldown_percent(&carried_draw, 0.0);
                    draw_gui_item(renderer, font, &carried_draw, cx, cy, cooldown, true);
                }
            } else {
                let cooldown = player.item_cooldown_percent(&self.carried_item, 0.0);
                draw_gui_item(renderer, font, &self.carried_item, cx, cy, cooldown, true);
            }
        }
        let mut deferred_tooltip = DeferredTooltipSlot::default();
        if let Some(hovered_idx) = self.hovered_slot_idx {
            let item = self.menu.get_slot_item(hovered_idx);
            if !item.is_empty() {
                let image = self.bundle_tooltip_image_for_slot(hovered_idx, item);
                let can_show_with_carried_item = image
                    .as_ref()
                    .is_some_and(TooltipImage::show_tooltip_with_item_in_hand);
                if self.carried_item.is_empty() || can_show_with_carried_item {
                    let tooltip = DeferredTooltip::new(
                        self.tooltip_lines_for_item(hovered_idx, item),
                        image,
                        tooltip_style(item),
                    );
                    #[cfg(feature = "extensions")]
                    let tooltip = mod_tooltip::compose(
                        tooltip,
                        item,
                        self.tooltip_modifiers,
                        renderer,
                        device,
                        queue,
                        texture_bind_group_layout,
                        resource_manager,
                    );
                    deferred_tooltip.set_for_next_frame(tooltip, false);
                }
            }
        }
        if self.menu_type == MenuType::Creative {
            if let Some(creative) = self.creative_state.as_ref()
                && let Some(tab) =
                    creative.tab_hovered_at(mouse_x, mouse_y, self.left_pos, self.top_pos)
            {
                deferred_tooltip.set_for_next_frame(
                    DeferredTooltip::text(vec![TooltipLine::new(
                        crate::ui::i18n::translate(tab.translation_key).to_string(),
                        TOOLTIP_TEXT,
                    )]),
                    false,
                );
            }
            if let Some(hovered_idx) = self.hovered_slot_idx
                && self
                    .menu
                    .slots
                    .get(hovered_idx)
                    .is_some_and(|slot| slot.slot_type == SlotType::Trash)
            {
                deferred_tooltip.set_for_next_frame(
                    DeferredTooltip::text(vec![TooltipLine::new(
                        crate::ui::i18n::translate("inventory.binSlot").to_string(),
                        TOOLTIP_TEXT,
                    )]),
                    false,
                );
            }
        }
        self.deferred_tooltip = deferred_tooltip.take();
        if !defer_tooltip {
            if let Some(tooltip) = self.deferred_tooltip.take() {
                render_tooltip(
                    renderer, font, &tooltip, mouse_x, mouse_y, logical_w, logical_h,
                );
            }
        }
    }
    pub(crate) fn deferred_tooltip(&self) -> Option<&DeferredTooltip> {
        self.deferred_tooltip.as_ref()
    }
    pub fn update_creative_catalog(&mut self) {
        if self.menu_type != MenuType::Creative {
            return;
        }
        if let Some(creative) = self.creative_state.as_ref() {
            creative.apply_to_menu(&mut self.menu);
        } else {
            el_inventory::menu::fill_creative_grid(&mut self.menu, self.creative_scroll_row);
        }
        self.refresh_creative_layout();
    }
    pub fn mouse_scrolled(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        scroll_x: f32,
        scroll_y: f32,
    ) -> bool {
        self.refresh_hovered_slot(mouse_x, mouse_y);
        let shown_bundle_items = self.hovered_slot_idx.and_then(|slot| {
            bundle_contents_for_item(self.menu.get_slot_item(slot))
                .map(|contents| (slot, contents.number_of_items_to_show()))
        });
        if let Some((hovered_slot, amount_of_shown_items)) = shown_bundle_items {
            if amount_of_shown_items > 0 {
                let (wheel_x, wheel_y) =
                    self.consume_bundle_scroll(scroll_x as f64, scroll_y as f64);
                let wheel = if wheel_y == 0 { -wheel_x } else { wheel_y };
                if wheel != 0 {
                    let selected_item = self
                        .bundle_selected_items
                        .get(&hovered_slot)
                        .copied()
                        .and_then(|index| i32::try_from(index).ok())
                        .unwrap_or(-1);
                    let updated_selected_item =
                        next_bundle_scroll_selection(wheel, selected_item, amount_of_shown_items);
                    if selected_item != updated_selected_item {
                        self.select_bundle_item(hovered_slot, updated_selected_item);
                    }
                }
                return true;
            }
        }
        if self.menu_type == MenuType::Creative {
            if let Some(creative) = self.creative_state.as_mut() {
                if creative.can_scroll() {
                    creative.subtract_input_from_scroll(scroll_y, &mut self.menu);
                    self.creative_scroll_row = creative.row_index_for_scroll(creative.scroll_offs);
                    self.refresh_creative_layout();
                    return true;
                }
            }
        }
        false
    }
    pub fn take_pending_bundle_item_selections(&mut self) -> Vec<(i32, u32)> {
        std::mem::take(&mut self.pending_bundle_item_selections)
    }
    fn bundle_tooltip_image_for_slot(&self, slot: usize, item: &ItemStack) -> Option<TooltipImage> {
        bundle_tooltip_image_with_selected(item, self.bundle_selected_items.get(&slot).copied())
    }
    fn consume_bundle_scroll(&mut self, scaled_x: f64, scaled_y: f64) -> (i32, i32) {
        if self.bundle_scroll_accum_x != 0.0
            && java_scroll_signum(scaled_x) != java_scroll_signum(self.bundle_scroll_accum_x)
        {
            self.bundle_scroll_accum_x = 0.0;
        }
        if self.bundle_scroll_accum_y != 0.0
            && java_scroll_signum(scaled_y) != java_scroll_signum(self.bundle_scroll_accum_y)
        {
            self.bundle_scroll_accum_y = 0.0;
        }
        self.bundle_scroll_accum_x += scaled_x;
        self.bundle_scroll_accum_y += scaled_y;
        let wheel_x = self.bundle_scroll_accum_x as i32;
        let wheel_y = self.bundle_scroll_accum_y as i32;
        if wheel_x != 0 || wheel_y != 0 {
            self.bundle_scroll_accum_x -= wheel_x as f64;
            self.bundle_scroll_accum_y -= wheel_y as f64;
        }
        (wheel_x, wheel_y)
    }
    fn select_bundle_item(&mut self, slot: usize, selected_item: i32) {
        let shown_items = bundle_contents_for_item(self.menu.get_slot_item(slot))
            .map(|contents| contents.number_of_items_to_show())
            .unwrap_or(0);
        let Ok(selected_item) = usize::try_from(selected_item) else {
            return;
        };
        if selected_item >= shown_items {
            return;
        }
        if !self.bundle_item_selection_connection_available {
            return;
        }
        self.bundle_selected_items.insert(slot, selected_item);
        self.pending_bundle_item_selections
            .push((slot as i32, selected_item as u32));
    }
    fn unselect_bundle_item(&mut self, slot: usize) {
        let is_bundle = bundle_contents_for_item(self.menu.get_slot_item(slot)).is_some();
        if self.bundle_item_selection_connection_available {
            self.bundle_selected_items.remove(&slot);
        }
        if self.bundle_item_selection_connection_available && is_bundle {
            self.pending_bundle_item_selections
                .push((slot as i32, u32::MAX));
        }
    }
    pub fn handle_keyboard_event(
        &mut self,
        event: &WindowEvent,
        controls: &VanillaKeyMappings,
        control_down: bool,
        shift_down: bool,
        font: &FontRenderer,
        clipboard: &mut dyn el_gui::components::edit_box::Clipboard,
    ) -> ContainerKeyboardAction {
        let WindowEvent::KeyboardInput { event, .. } = event else {
            return ContainerKeyboardAction::Unhandled;
        };
        if event.state != ElementState::Pressed {
            return ContainerKeyboardAction::Unhandled;
        }
        let input = pressed_bound_input(event);
        self.handle_pressed_key(
            event.physical_key,
            event.text.as_deref(),
            input,
            controls,
            control_down,
            shift_down,
            font,
            clipboard,
        )
    }
    pub fn handle_ime_event(&mut self, event: &Ime, font: &FontRenderer) -> bool {
        let Some(edit_box) = self.creative_search_box.as_mut() else {
            return false;
        };
        if !edit_box.is_focused() {
            return false;
        }
        match event {
            Ime::Preedit(text, cursor) => {
                let preedit = if text.is_empty() {
                    None
                } else {
                    let caret_position = cursor
                        .map(|(start, _)| text.get(..start).unwrap_or(text).chars().count())
                        .unwrap_or(0);
                    Some(el_gui::input::events::PreeditEvent::new(
                        text.clone(),
                        caret_position,
                        vec![text.clone()],
                        0,
                    ))
                };
                edit_box.preedit_updated(font, preedit)
            }
            Ime::Commit(text) => {
                for character in text.chars() {
                    edit_box.char_typed(font, character);
                }
                self.push_creative_search_query();
                true
            }
            Ime::Disabled => edit_box.preedit_updated(font, None),
            Ime::Enabled => false,
        }
    }
    fn handle_pressed_key(
        &mut self,
        physical_key: PhysicalKey,
        text: Option<&str>,
        input: Option<BoundInput>,
        controls: &VanillaKeyMappings,
        control_down: bool,
        shift_down: bool,
        font: &FontRenderer,
        clipboard: &mut dyn el_gui::components::edit_box::Clipboard,
    ) -> ContainerKeyboardAction {
        let search_tab_active = self.menu_type == MenuType::Creative
            && self
                .creative_state
                .as_ref()
                .is_some_and(|creative| creative.selected_kind() == CreativeTabKind::Search);
        if search_tab_active {
            let do_quick_swap = self
                .hovered_slot_idx
                .is_some_and(|idx| self.menu.slots[idx].slot_type != SlotType::Creative)
                || self
                    .hovered_slot_idx
                    .is_some_and(|idx| !self.menu.get_slot_item(idx).is_empty());
            if do_quick_swap {
                if let Some(action) = self.handle_inventory_keybind(input, controls, control_down) {
                    return action;
                }
            }
            if matches!(physical_key, PhysicalKey::Code(KeyCode::Escape)) {
                return ContainerKeyboardAction::Unhandled;
            }
            use el_gui::components::edit_box::EditBoxKey;
            let edit_box_key = match physical_key {
                PhysicalKey::Code(KeyCode::Backspace) => Some(EditBoxKey::Backspace),
                PhysicalKey::Code(KeyCode::Delete) => Some(EditBoxKey::Delete),
                PhysicalKey::Code(KeyCode::ArrowLeft) => Some(EditBoxKey::Left),
                PhysicalKey::Code(KeyCode::ArrowRight) => Some(EditBoxKey::Right),
                PhysicalKey::Code(KeyCode::Home) => Some(EditBoxKey::Home),
                PhysicalKey::Code(KeyCode::End) => Some(EditBoxKey::End),
                _ => None,
            };
            let needs_tab_sync = self.creative_search_box.is_none()
                || self.creative_search_tab_sync_pending
                || self
                    .creative_search_box
                    .as_ref()
                    .is_some_and(|edit_box| !edit_box.is_visible() || !edit_box.is_focused());
            if needs_tab_sync {
                let tab_changed = std::mem::take(&mut self.creative_search_tab_changed);
                self.creative_search_tab_sync_pending = false;
                self.sync_creative_search_box_to_tab(font, tab_changed, util_get_millis());
            } else {
                self.ensure_creative_search_box(font);
            }
            if let Some(key) = edit_box_key {
                if let Some(edit_box) = self.creative_search_box.as_mut() {
                    edit_box.key_pressed(font, key, control_down, shift_down);
                }
                self.push_creative_search_query();
                return ContainerKeyboardAction::Handled;
            }
            if control_down {
                if let PhysicalKey::Code(code) = physical_key {
                    let consumed = match code {
                        KeyCode::KeyA => self
                            .creative_search_box
                            .as_mut()
                            .is_some_and(|b| b.select_all(font)),
                        KeyCode::KeyC => self
                            .creative_search_box
                            .as_ref()
                            .is_some_and(|b| b.copy(clipboard)),
                        KeyCode::KeyV => self
                            .creative_search_box
                            .as_mut()
                            .is_some_and(|b| b.paste(font, clipboard)),
                        KeyCode::KeyX => self
                            .creative_search_box
                            .as_mut()
                            .is_some_and(|b| b.cut(font, clipboard)),
                        _ => false,
                    };
                    if consumed {
                        self.push_creative_search_query();
                        return ContainerKeyboardAction::Handled;
                    }
                }
            }
            if let Some(text) = text {
                if let Some(edit_box) = self.creative_search_box.as_mut() {
                    for ch in text.chars() {
                        edit_box.char_typed(font, ch);
                    }
                }
                self.push_creative_search_query();
            }
            return ContainerKeyboardAction::Handled;
        }
        if input.is_some_and(|input| controls.matches_input(KeyAction::Inventory, input)) {
            return ContainerKeyboardAction::Close;
        }
        if let Some(action) = self.handle_inventory_keybind(input, controls, control_down) {
            return action;
        }
        if self.menu_type != MenuType::Creative {
            return ContainerKeyboardAction::Unhandled;
        }
        let Some(creative) = self.creative_state.as_mut() else {
            return ContainerKeyboardAction::Unhandled;
        };
        match physical_key {
            PhysicalKey::Code(KeyCode::KeyT)
                if creative.selected_kind() != CreativeTabKind::Search =>
            {
                creative.select_search_tab(&mut self.menu);
                self.request_creative_search_box_tab_sync(true);
                self.refresh_creative_layout();
                return ContainerKeyboardAction::Handled;
            }
            PhysicalKey::Code(KeyCode::Backspace)
                if creative.selected_kind() == CreativeTabKind::Search =>
            {
                let handled = creative.backspace_search(&mut self.menu);
                self.refresh_creative_layout();
                return if handled {
                    ContainerKeyboardAction::Handled
                } else {
                    ContainerKeyboardAction::Unhandled
                };
            }
            PhysicalKey::Code(KeyCode::Escape) => {
                return ContainerKeyboardAction::Unhandled;
            }
            _ => {}
        }
        ContainerKeyboardAction::Unhandled
    }
    fn handle_inventory_keybind(
        &mut self,
        input: Option<BoundInput>,
        controls: &VanillaKeyMappings,
        control_down: bool,
    ) -> Option<ContainerKeyboardAction> {
        let input = input?;
        let hovered_idx = self.hovered_slot_idx?;
        if self.carried_item.is_empty() {
            if controls.matches_input(KeyAction::SwapOffhand, input) {
                return Some(self.click_for_keyboard(hovered_idx, 40, ContainerInput::Swap));
            }
            for hotbar_slot in 0..9 {
                if controls.matches_input(KeyAction::HotbarSlot(hotbar_slot), input) {
                    return Some(self.click_for_keyboard(
                        hovered_idx,
                        hotbar_slot as i8,
                        ContainerInput::Swap,
                    ));
                }
            }
        }
        let hovered_item = self.menu.get_slot_item(hovered_idx);
        if hovered_item.is_empty() {
            return None;
        }
        if controls.matches_input(KeyAction::PickItem, input)
            && self.menu_type == MenuType::Creative
        {
            return Some(self.click_for_keyboard(hovered_idx, 0, ContainerInput::Clone));
        }
        if controls.matches_input(KeyAction::Drop, input) && self.carried_item.is_empty() {
            let button = if control_down { 1 } else { 0 };
            return Some(self.click_for_keyboard(hovered_idx, button, ContainerInput::Throw));
        }
        None
    }
    fn click_for_keyboard(
        &mut self,
        slot_idx: usize,
        button_num: i8,
        input: ContainerInput,
    ) -> ContainerKeyboardAction {
        if let Some(packet) = self.perform_slot_click_prediction(slot_idx as i16, button_num, input)
        {
            ContainerKeyboardAction::Clicks(vec![packet])
        } else if !self.pending_creative_clicks.is_empty() {
            ContainerKeyboardAction::Handled
        } else {
            ContainerKeyboardAction::Unhandled
        }
    }
    fn tooltip_lines_for_item(&self, hovered_idx: usize, item: &ItemStack) -> Vec<TooltipLine> {
        let is_creative_slot = self.menu_type == MenuType::Creative
            && self
                .menu
                .slots
                .get(hovered_idx)
                .is_some_and(|slot| slot.slot_type == SlotType::Creative);
        let mut lines = item_tooltip_lines(item, is_creative_slot, self.attribute_base_values);
        if tooltip_display_hides(item) {
            return lines;
        }
        if lines.is_empty() {
            return lines;
        }
        let item_path = item.item.location.path.as_str();
        if self.menu_type != MenuType::Creative || hovered_idx >= self.menu.slots.len() {
            return lines;
        }
        let Some(creative) = self.creative_state.as_ref() else {
            return lines;
        };
        let selected_kind = creative.selected_kind();
        if selected_kind == CreativeTabKind::Category && is_creative_slot {
            return lines;
        }
        if selected_kind == CreativeTabKind::Search && is_creative_slot {
            for tag in &creative.visible_tags {
                if el_inventory::creative::item_is_in_tag(item, tag) {
                    lines.insert(
                        1,
                        TooltipLine::new(format!("#{}", tag), TOOLTIP_DARK_PURPLE),
                    );
                }
            }
        }
        let mut i = 1;
        for tab in creative.visible_tabs() {
            if tab.kind != CreativeTabKind::Search && tab.items.contains(&item_path) {
                lines.insert(
                    i,
                    TooltipLine::new(
                        crate::ui::i18n::translate(tab.translation_key).to_string(),
                        TOOLTIP_CATEGORY,
                    ),
                );
                i += 1;
            }
        }
        lines
    }
    fn refresh_slot_coords_from_menu(&mut self) {
        self.slot_coords = self
            .menu
            .slot_positions
            .iter()
            .map(|pos| (pos.x, pos.y))
            .collect();
    }
    fn refresh_creative_layout(&mut self) {
        if let Some(creative) = self.creative_state.as_ref() {
            self.background_texture = creative.selected_background_texture();
        }
        self.refresh_slot_coords_from_menu();
    }
    fn render_creative_tabs(
        &self,
        renderer: &mut GuiRenderer,
        font: &FontRenderer,
        selected_only: bool,
        _mouse_x: f32,
        _mouse_y: f32,
    ) {
        let Some(creative) = self.creative_state.as_ref() else {
            return;
        };
        for tab in creative.visible_tabs() {
            let selected = tab.id == creative.selected_tab_id();
            if selected != selected_only {
                continue;
            }
            let is_top = tab.row == CreativeTabRow::Top;
            let pos = tab.column.clamp(0, 6) + 1;
            let row_name = if is_top { "top" } else { "bottom" };
            let selected_name = if selected { "selected" } else { "unselected" };
            let sprite = ResourceLocation::new(
                "minecraft",
                &format!(
                    "container/creative_inventory/tab_{}_{}_{}",
                    row_name, selected_name, pos
                ),
            );
            let x = self.left_pos + creative.tab_x(tab);
            let y = creative.tab_render_y(tab, self.top_pos);
            renderer.draw_sprite(
                x as f32,
                y as f32,
                CREATIVE_TAB_WIDTH as f32,
                CREATIVE_TAB_HEIGHT as f32,
                &sprite,
                [1.0, 1.0, 1.0, 1.0],
            );
            if let Some(icon) = stack_from_item_path(tab.icon_item) {
                let icon_x = x as f32 + 5.0;
                let icon_y = y as f32 + if is_top { 9.0 } else { 7.0 };
                draw_gui_item(renderer, font, &icon, icon_x, icon_y, 0.0, false);
            }
        }
    }
    pub fn ensure_creative_search_box(&mut self, font: &FontRenderer) {
        let x = self.left_pos + 82;
        let y = self.top_pos + 6;
        match self.creative_search_box.as_mut() {
            Some(edit_box) => {
                if edit_box.x() != x || edit_box.y() != y {
                    edit_box.set_position(font, x, y);
                }
            }
            None => {
                let mut edit_box = el_gui::components::edit_box::EditBox::new(font, x, y, 80, 9);
                edit_box.set_max_length(font, 50);
                edit_box.set_bordered(font, false);
                edit_box.set_text_color(-1);
                edit_box.set_invert_highlighted_text_color(false);
                edit_box.set_visible(false);
                self.creative_search_box = Some(edit_box);
            }
        }
    }
    pub fn sync_creative_search_box_to_tab(
        &mut self,
        font: &FontRenderer,
        tab_changed: bool,
        now_ms: u64,
    ) {
        self.ensure_creative_search_box(font);
        let is_search = self
            .creative_state
            .as_ref()
            .is_some_and(|creative| creative.selected_kind() == CreativeTabKind::Search);
        let Some(edit_box) = self.creative_search_box.as_mut() else {
            return;
        };
        if is_search {
            edit_box.set_visible(true);
            edit_box.set_can_lose_focus(false);
            edit_box.set_focused(true, now_ms);
            if tab_changed {
                edit_box.set_value(font, "");
            }
        } else {
            edit_box.set_visible(false);
            edit_box.set_can_lose_focus(true);
            edit_box.set_focused(false, now_ms);
            edit_box.set_value(font, "");
        }
    }
    fn sync_creative_search_box_if_needed(&mut self, font: &FontRenderer, now_ms: u64) {
        if self.menu_type != MenuType::Creative {
            return;
        }
        let widget_missing = self.creative_search_box.is_none();
        if widget_missing || self.creative_search_tab_sync_pending {
            let tab_changed = std::mem::take(&mut self.creative_search_tab_changed);
            self.creative_search_tab_sync_pending = false;
            self.sync_creative_search_box_to_tab(font, tab_changed, now_ms);
        } else {
            self.ensure_creative_search_box(font);
        }
    }
    fn push_creative_search_query(&mut self) {
        let Some(query) = self
            .creative_search_box
            .as_ref()
            .map(el_gui::components::edit_box::EditBox::value)
        else {
            return;
        };
        if let Some(creative) = self.creative_state.as_mut() {
            if creative.search_query != query {
                creative.set_search_query(&query, &mut self.menu);
            }
        }
        self.refresh_creative_layout();
    }
    fn render_creative_search_box(
        &self,
        renderer: &mut GuiRenderer,
        font: &FontRenderer,
        now_ms: u64,
    ) {
        let Some(edit_box) = self.creative_search_box.as_ref() else {
            return;
        };
        if !edit_box.is_visible() {
            return;
        }
        use el_gui::components::edit_box::EditBoxDraw;
        let plan = edit_box.render_plan(font, now_ms);
        let preedit_underline = plan.preedit_underline;
        for draw in plan.draws {
            match draw {
                EditBoxDraw::Sprite { .. } => {}
                EditBoxDraw::Text {
                    text,
                    x,
                    y,
                    color,
                    shadow,
                } => {
                    let rgba = edit_box_argb_to_rgba(color);
                    if shadow {
                        renderer
                            .draw_text_top_with_shadow(font, &text, x as f32, y as f32, rgba, 1.0);
                    } else {
                        renderer.draw_text_top(font, &text, x as f32, y as f32, rgba, 1.0);
                    }
                }
                EditBoxDraw::Highlight {
                    x0,
                    y0,
                    x1,
                    y1,
                    invert,
                } => {
                    let (left, right) = (x0.min(x1), x0.max(x1));
                    renderer.draw_text_highlight_rect(
                        left as f32,
                        y0 as f32,
                        (right - left) as f32,
                        (y1 - y0) as f32,
                        invert,
                    );
                }
                EditBoxDraw::InsertCursor { x, y, color } => {
                    renderer.draw_rect(
                        x as f32,
                        (y - 1) as f32,
                        1.0,
                        EDIT_BOX_CURSOR_HEIGHT as f32,
                        edit_box_argb_to_rgba(color),
                    );
                }
                EditBoxDraw::AppendCursor {
                    x,
                    y,
                    color,
                    shadow,
                } => {
                    let glyph = el_gui::components::edit_box::EditBox::append_cursor_character();
                    let rgba = edit_box_argb_to_rgba(color);
                    if shadow {
                        renderer
                            .draw_text_top_with_shadow(font, glyph, x as f32, y as f32, rgba, 1.0);
                    } else {
                        renderer.draw_text_top(font, glyph, x as f32, y as f32, rgba, 1.0);
                    }
                }
            }
        }
        if let Some(underline) = preedit_underline {
            if underline.x1 > underline.x0 {
                renderer.draw_rect(
                    underline.x0 as f32,
                    underline.y as f32,
                    (underline.x1 - underline.x0) as f32,
                    1.0,
                    edit_box_argb_to_rgba(underline.color),
                );
            }
        }
    }
    fn render_creative_scroller(&self, renderer: &mut GuiRenderer) {
        let Some(creative) = self.creative_state.as_ref() else {
            return;
        };
        if !creative.selected_tab().can_scroll {
            return;
        }
        let sprite = if creative.can_scroll() {
            ResourceLocation::new("minecraft", "container/creative_inventory/scroller")
        } else {
            ResourceLocation::new(
                "minecraft",
                "container/creative_inventory/scroller_disabled",
            )
        };
        renderer.draw_sprite(
            self.left_pos as f32 + 175.0,
            self.top_pos as f32 + creative.scroller_y() as f32,
            12.0,
            15.0,
            &sprite,
            [1.0, 1.0, 1.0, 1.0],
        );
    }
}
fn clip_effect_text(text: &str, font: &FontRenderer, max_width: i32) -> String {
    if font.measure_width(text, 1.0) <= max_width as f32 {
        return text.to_owned();
    }
    let ellipsis = "…";
    let ellipsis_width = font.measure_width(ellipsis, 1.0);
    let mut clipped = String::new();
    for ch in text.chars() {
        let candidate_width = font.measure_width(&format!("{clipped}{ch}"), 1.0) + ellipsis_width;
        if candidate_width > max_width as f32 {
            break;
        }
        clipped.push(ch);
    }
    clipped.push('…');
    clipped
}
fn item_tooltip_lines(
    item: &ItemStack,
    is_creative_slot: bool,
    attribute_base_values: Option<(f64, f64)>,
) -> Vec<TooltipLine> {
    let hide_tooltip = tooltip_display_hides(item);
    if hide_tooltip && !is_creative_slot {
        return Vec::new();
    }
    let name_color = tooltip_item_name_color(item);
    let name_line = tooltip_name_line(item, name_color);
    let mut lines = vec![name_line];
    if hide_tooltip {
        return lines;
    }
    let item_path = item.item.location.path.as_str();
    if item.components.get("minecraft:bees").is_some()
        && !tooltip_component_hidden(item, "minecraft:bees")
    {
        add_bees_tooltip_lines(item, &mut lines);
    }
    if item.components.get("minecraft:container_loot").is_some()
        && !tooltip_component_hidden(item, "minecraft:container_loot")
    {
        add_seeded_container_loot_tooltip_lines(&mut lines);
    }
    if item.components.get("minecraft:container").is_some()
        && !tooltip_component_hidden(item, "minecraft:container")
    {
        add_container_contents_tooltip_lines(item, &mut lines);
    }
    if item.components.get("minecraft:potion_contents").is_some()
        && !tooltip_component_hidden(item, "minecraft:potion_contents")
    {
        add_potion_tooltip_lines(item, &mut lines);
    }
    if item_path == "ominous_bottle"
        && !tooltip_component_hidden(item, "minecraft:ominous_bottle_amplifier")
    {
        add_ominous_bottle_tooltip_lines(item, &mut lines);
    }
    if !tooltip_component_hidden(item, "minecraft:stored_enchantments") {
        add_stored_enchantment_tooltip_lines(item, &mut lines);
    }
    if !tooltip_component_hidden(item, "minecraft:enchantments") {
        add_item_enchantment_tooltip_lines(item, &mut lines);
    }
    if item_path == "goat_horn" && !tooltip_component_hidden(item, "minecraft:instrument") {
        add_goat_horn_tooltip_lines(item, &mut lines);
    }
    if item_path == "suspicious_stew"
        && !tooltip_component_hidden(item, "minecraft:suspicious_stew_effects")
    {
        add_suspicious_stew_tooltip_lines(item, &mut lines);
    }
    if item_path == "painting" && !tooltip_component_hidden(item, "minecraft:painting/variant") {
        add_painting_tooltip_lines(item, &mut lines);
    }
    if item.components.get("minecraft:dyed_color").is_some()
        && !tooltip_component_hidden(item, "minecraft:dyed_color")
    {
        lines.push(
            TooltipLine::new(
                crate::ui::i18n::translate("item.dyed").to_string(),
                [0.62, 0.62, 0.62, 1.0],
            )
            .italic(),
        );
    }
    if !tooltip_component_hidden(item, "minecraft:lore") {
        add_lore_tooltip_lines(item, &mut lines);
    }
    if !tooltip_component_hidden(item, "minecraft:attribute_modifiers") {
        item_attribute_tooltips::add_item_attribute_tooltip_lines(
            item,
            attribute_base_values,
            &mut lines,
        );
    }
    if item.components.get("minecraft:unbreakable").is_some()
        && !tooltip_component_hidden(item, "minecraft:unbreakable")
    {
        lines.push(TooltipLine::new(
            crate::ui::i18n::translate("item.unbreakable").to_string(),
            [0.62, 0.62, 0.62, 1.0],
        ));
    }
    lines
}
pub(crate) fn chat_item_tooltip(
    item: &ItemStack,
    advanced: bool,
    attribute_base_values: Option<(f64, f64)>,
) -> Option<DeferredTooltip> {
    let mut lines = item_tooltip_lines(item, false, attribute_base_values);
    if lines.is_empty() {
        return None;
    }
    if advanced {
        if item.is_damaged() && !tooltip_component_hidden(item, "minecraft:damage") {
            lines.push(TooltipLine::new(
                crate::ui::i18n::format(
                    "item.durability",
                    &[
                        &(item.get_max_damage() - item.get_damage_value()),
                        &item.get_max_damage(),
                    ],
                ),
                TOOLTIP_TEXT,
            ));
        }
        let gray = [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0, 1.0];
        lines.push(TooltipLine::new(item.item.location.to_string(), gray));
        let names: std::collections::HashSet<_> = item
            .components
            .prototype
            .keys()
            .chain(item.components.patch.map.keys())
            .collect();
        let count = names
            .into_iter()
            .filter(|name| !name.starts_with("eltharn:") && item.components.get(name).is_some())
            .count();
        if count > 0 {
            lines.push(TooltipLine::new(
                crate::ui::i18n::format("item.components", &[&count]),
                gray,
            ));
        }
    }
    DeferredTooltip::new(lines, bundle_tooltip_image(item), tooltip_style(item))
}
fn tooltip_display_hides(item: &ItemStack) -> bool {
    let Some(DataComponent::Generic {
        value: Tag::Compound(fields),
        ..
    }) = item.components.get("minecraft:tooltip_display")
    else {
        return false;
    };
    match compound_get(fields, "hide_tooltip") {
        Some(Tag::Byte(value)) => *value != 0,
        Some(Tag::Int(value)) => *value != 0,
        Some(Tag::Long(value)) => *value != 0,
        _ => false,
    }
}
fn tooltip_component_hidden(item: &ItemStack, component: &str) -> bool {
    let Some(DataComponent::Generic {
        value: Tag::Compound(fields),
        ..
    }) = item.components.get("minecraft:tooltip_display")
    else {
        return false;
    };
    let Some(Tag::List(hidden_components)) = compound_get(fields, "hidden_components") else {
        return false;
    };
    let short_component = component.strip_prefix("minecraft:").unwrap_or(component);
    hidden_components.iter().any(|tag| {
        let Tag::String(hidden) = tag else {
            return false;
        };
        hidden == component
            || hidden.strip_prefix("minecraft:").unwrap_or(hidden) == short_component
    })
}
fn tooltip_style(item: &ItemStack) -> Option<ResourceLocation> {
    let DataComponent::Generic {
        value: Tag::String(style),
        ..
    } = item.components.get("minecraft:tooltip_style")?
    else {
        return None;
    };
    ResourceLocation::try_from_string(style)
}
fn tooltip_sprite(style: Option<&ResourceLocation>, suffix: &str) -> ResourceLocation {
    match style {
        Some(style) => ResourceLocation::new(
            &style.namespace,
            &format!("tooltip/{}_{}", style.path, suffix),
        ),
        None => ResourceLocation::new("minecraft", &format!("tooltip/{suffix}")),
    }
}
fn add_lore_tooltip_lines(item: &ItemStack, lines: &mut Vec<TooltipLine>) {
    let mut append = |raw: &str| {
        if let Some((text, styled)) =
            styled_text_from_raw_component_with_style(raw, lore_text_component_style())
        {
            if !text.is_empty() {
                lines.push(
                    TooltipLine::new(text, [0.6667, 0.0, 0.6667, 1.0])
                        .italic()
                        .with_styled_text(styled),
                );
            }
        } else {
            let text = parse_chat_component(raw);
            if !text.is_empty() {
                lines.push(TooltipLine::new(text, [0.6667, 0.0, 0.6667, 1.0]).italic());
            }
        }
    };
    match item.components.get("minecraft:lore") {
        Some(DataComponent::Lore(lore)) => {
            for line in lore {
                append(line);
            }
        }
        Some(DataComponent::Generic {
            value: Tag::List(entries),
            ..
        }) => {
            for entry in entries {
                match entry {
                    Tag::String(raw) => append(raw),
                    Tag::Compound(_) | Tag::List(_) => {
                        if let Some(json) = tag_to_json(entry) {
                            let raw = serde_json::to_string(&json).unwrap_or_default();
                            append(&raw);
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    if !item.components.get("minecraft:lore").is_some()
        && let Some(display) = legacy_display_compound(item)
        && let Some(Tag::List(entries)) = compound_get(display, "Lore")
    {
        for entry in entries {
            if let Tag::String(raw) = entry {
                append(raw);
            }
        }
    }
}
fn tooltip_name_line(item: &ItemStack, color: [f32; 4]) -> TooltipLine {
    let inherited = item_hover_name_text_component_style(item);
    let component = item
        .components
        .get("minecraft:custom_name")
        .or_else(|| item.components.get("minecraft:item_name"))
        .and_then(|component| component_tag_for_item_name(component, inherited));
    if let Some((text, styled)) = component {
        return TooltipLine::new(text, color).with_styled_text(styled);
    }
    if let Some(display) = legacy_display_compound(item)
        && let Some(name) = compound_get(display, "Name")
        && let Some((text, styled)) = styled_text_from_tag_with_style(
            name,
            TextComponentStyle {
                italic: true,
                ..inherited
            },
        )
    {
        return TooltipLine::new(text, color).with_styled_text(styled);
    }
    TooltipLine::new(translated_item_name(item), color)
}
fn tooltip_item_name_color(item: &ItemStack) -> [f32; 4] {
    let color = crate::ui::gui::hud::item_name_color(item);
    [
        ((color >> 16) & 0xff) as f32 / 255.0,
        ((color >> 8) & 0xff) as f32 / 255.0,
        (color & 0xff) as f32 / 255.0,
        1.0,
    ]
}
fn java_26_2_tooltip_translation(key: &str) -> std::borrow::Cow<'_, str> {
    crate::ui::i18n::translate(key)
}
fn format_java_tooltip_translation(
    template: impl AsRef<str>,
    arguments: &[String],
    inherited: TextComponentStyle,
) -> String {
    let template = template.as_ref();
    let bytes = template.as_bytes();
    let mut result = String::new();
    let mut literal_start = 0usize;
    let mut replacement_index = 0usize;
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if bytes[cursor] != b'%' {
            cursor += 1;
            continue;
        }
        let Ok(format) = parse_json_translation_format(template, cursor) else {
            return styled_text_run(template, inherited);
        };
        if literal_start < cursor {
            result.push_str(&styled_text_run(
                &template[literal_start..cursor],
                inherited,
            ));
        }
        match format {
            JsonTranslationFormat::Percent { end } => {
                result.push_str(&styled_text_run("%", inherited));
                cursor = end;
            }
            JsonTranslationFormat::String { position, end } => {
                let index = position.unwrap_or_else(|| {
                    let current = replacement_index;
                    replacement_index += 1;
                    current
                });
                let Some(argument) = arguments.get(index) else {
                    return styled_text_run(template, inherited);
                };
                result.push_str(argument);
                cursor = end;
            }
        }
        literal_start = cursor;
    }
    if literal_start < template.len() {
        result.push_str(&styled_text_run(&template[literal_start..], inherited));
    }
    result
}
fn default_tooltip_text_style() -> TextComponentStyle {
    TextComponentStyle {
        color: Some(0xFF_FF_FF),
        ..TextComponentStyle::default()
    }
}
fn gray_tooltip_text_style() -> TextComponentStyle {
    TextComponentStyle {
        color: crate::ui::scoreboard::chat_color_rgb(7),
        ..TextComponentStyle::default()
    }
}
fn item_hover_name_for_nested_tooltip(item: &ItemStack) -> String {
    let inherited = default_tooltip_text_style();
    if let Some(component) = item
        .components
        .get("minecraft:custom_name")
        .or_else(|| item.components.get("minecraft:item_name"))
        && let Some((_, styled)) = component_tag_for_item_name(component, inherited)
    {
        return styled;
    }
    styled_text_run(&translated_item_name(item), inherited)
}
fn known_non_air_tooltip_item(identifier: &str) -> Option<std::sync::Arc<el_core::item::Item>> {
    let location = ResourceLocation::try_from_string(identifier)?;
    let item = BuiltInRegistries::items()
        .get_by_location(&location)
        .clone();
    (item.location == location && item.location.path != "air").then_some(item)
}
fn item_stack_template_from_compound_tag(
    tag: &Tag,
    require_persistent_count_range: bool,
) -> Option<ItemStack> {
    let Tag::Compound(fields) = tag else {
        return None;
    };
    let Tag::String(identifier) = compound_get(fields, "id")? else {
        return None;
    };
    let count = compound_get(fields, "count")
        .map(tag_i32)
        .unwrap_or(Some(1))?;
    if require_persistent_count_range {
        if !(1..=99).contains(&count) {
            return None;
        }
    } else if count <= 0 {
        return None;
    }
    if let Some(components) = compound_get(fields, "components")
        && !matches!(components, Tag::Compound(_))
    {
        return None;
    }
    let item = known_non_air_tooltip_item(identifier)?;
    let stack = ItemStack::from_nbt(tag)?;
    (stack.item.location == item.location && !stack.is_empty()).then_some(stack)
}
fn item_stack_template_from_persistent_codec_tag(tag: &Tag) -> Option<ItemStack> {
    match tag {
        Tag::String(identifier) => Some(ItemStack::new(known_non_air_tooltip_item(identifier)?, 1)),
        Tag::Compound(_) => item_stack_template_from_compound_tag(tag, true),
        _ => None,
    }
}
fn item_stack_template_from_stream_codec_tag(tag: &Tag) -> Option<ItemStack> {
    item_stack_template_from_compound_tag(tag, false)
}
fn bundle_item_stack_template_from_component_tag(tag: &Tag) -> Option<ItemStack> {
    match tag {
        Tag::String(_) => item_stack_template_from_persistent_codec_tag(tag),
        Tag::Compound(_) => item_stack_template_from_stream_codec_tag(tag),
        _ => None,
    }
}
fn looks_like_persistent_container_slot(tag: &Tag) -> bool {
    let Tag::Compound(fields) = tag else {
        return false;
    };
    compound_get(fields, "slot").is_some() && compound_get(fields, "item").is_some()
}
fn persistent_container_contents_for_tooltip(entries: &[Tag]) -> Option<Vec<ItemStack>> {
    persistent_container_slots_for_tooltip(entries)
        .map(|slots| slots.into_iter().filter(|item| !item.is_empty()).collect())
}
fn persistent_container_slots_for_tooltip(entries: &[Tag]) -> Option<Vec<ItemStack>> {
    let mut slots: Vec<Option<ItemStack>> = Vec::new();
    for entry in entries {
        let Tag::Compound(fields) = entry else {
            return None;
        };
        let slot = tag_i32(compound_get(fields, "slot")?)?;
        if !(0..256).contains(&slot) {
            return None;
        }
        let stack = item_stack_template_from_persistent_codec_tag(compound_get(fields, "item")?)?;
        let slot = usize::try_from(slot).ok()?;
        if slots.len() <= slot {
            slots.resize_with(slot + 1, || None);
        }
        slots[slot] = Some(stack);
    }
    Some(
        slots
            .into_iter()
            .map(|item| item.unwrap_or_else(ItemStack::empty))
            .collect(),
    )
}
fn streamed_container_contents_for_tooltip(entries: &[Tag]) -> Option<Vec<ItemStack>> {
    streamed_container_slots_for_tooltip(entries)
        .map(|slots| slots.into_iter().filter(|item| !item.is_empty()).collect())
}
fn streamed_container_slots_for_tooltip(entries: &[Tag]) -> Option<Vec<ItemStack>> {
    entries
        .iter()
        .map(|entry| match entry {
            Tag::End => Some(None),
            _ => item_stack_template_from_stream_codec_tag(entry).map(Some),
        })
        .collect::<Option<Vec<_>>>()
        .map(|slots| {
            slots
                .into_iter()
                .map(|item| item.unwrap_or_else(ItemStack::empty))
                .collect()
        })
}
fn container_contents_for_tooltip(item: &ItemStack) -> Option<Vec<ItemStack>> {
    let DataComponent::Generic {
        value: Tag::List(entries),
        ..
    } = item.components.get("minecraft:container")?
    else {
        return None;
    };
    if entries.len() > 256 {
        return None;
    }
    if entries.iter().all(looks_like_persistent_container_slot) {
        persistent_container_contents_for_tooltip(entries)
    } else {
        streamed_container_contents_for_tooltip(entries)
    }
}
fn add_container_contents_tooltip_lines(item: &ItemStack, lines: &mut Vec<TooltipLine>) {
    let Some(contents) = container_contents_for_tooltip(item) else {
        return;
    };
    let mut line_count = 0usize;
    let mut item_count = 0usize;
    for contained_item in contents {
        item_count += 1;
        if line_count <= 4 {
            line_count += 1;
            let styled = format_java_tooltip_translation(
                java_26_2_tooltip_translation("item.container.item_count"),
                &[
                    item_hover_name_for_nested_tooltip(&contained_item),
                    styled_text_run(
                        &contained_item.count.to_string(),
                        default_tooltip_text_style(),
                    ),
                ],
                default_tooltip_text_style(),
            );
            let text = crate::ui::chat::chat_component::plain_chat_text(&styled);
            lines.push(TooltipLine::new(text, TOOLTIP_TEXT).with_styled_text(styled));
        }
    }
    if item_count.saturating_sub(line_count) > 0 {
        let styled = format_java_tooltip_translation(
            java_26_2_tooltip_translation("item.container.more_items"),
            &[styled_text_run(
                &item_count.saturating_sub(line_count).to_string(),
                default_tooltip_text_style(),
            )],
            TextComponentStyle {
                italic: true,
                ..default_tooltip_text_style()
            },
        );
        let text = crate::ui::chat::chat_component::plain_chat_text(&styled);
        lines.push(
            TooltipLine::new(text, TOOLTIP_TEXT)
                .italic()
                .with_styled_text(styled),
        );
    }
}
fn add_seeded_container_loot_tooltip_lines(lines: &mut Vec<TooltipLine>) {
    let styled = styled_text_run(
        &java_26_2_tooltip_translation("item.container.loot_table.unknown"),
        default_tooltip_text_style(),
    );
    let text = crate::ui::chat::chat_component::plain_chat_text(&styled);
    lines.push(TooltipLine::new(text, TOOLTIP_TEXT).with_styled_text(styled));
}
fn add_bees_tooltip_lines(item: &ItemStack, lines: &mut Vec<TooltipLine>) {
    let Some(DataComponent::Generic {
        value: Tag::List(bees),
        ..
    }) = item.components.get("minecraft:bees")
    else {
        return;
    };
    let count = bees.len().to_string();
    let styled = format_java_tooltip_translation(
        &crate::ui::i18n::translate("container.beehive.bees"),
        &[
            styled_text_run(&count, gray_tooltip_text_style()),
            styled_text_run("3", gray_tooltip_text_style()),
        ],
        gray_tooltip_text_style(),
    );
    let text = crate::ui::chat::chat_component::plain_chat_text(&styled);
    lines.push(TooltipLine::new(text, [0.6667, 0.6667, 0.6667, 1.0]).with_styled_text(styled));
}
impl TooltipLine {
    #[allow(dead_code)]
    fn italic_if_custom(self, item: &ItemStack) -> Self {
        if item.components.get("minecraft:custom_name").is_some() {
            self.italic()
        } else {
            self
        }
    }
}
fn component_tag_for_item_name(
    component: &DataComponent,
    inherited: TextComponentStyle,
) -> Option<(String, String)> {
    match component {
        DataComponent::CustomName(raw) => styled_text_from_raw_component_with_style(raw, inherited),
        DataComponent::Generic { value, .. } => styled_text_from_tag_with_style(value, inherited),
        _ => None,
    }
}
pub(crate) fn item_name_styled_ansi(item: &ItemStack) -> Option<String> {
    let inherited = item_hover_name_text_component_style(item);
    item.components
        .get("minecraft:custom_name")
        .or_else(|| item.components.get("minecraft:item_name"))
        .and_then(|component| component_tag_for_item_name(component, inherited))
        .map(|(_, styled)| styled)
        .or_else(|| {
            legacy_display_compound(item)
                .and_then(|fields| compound_get(fields, "Name"))
                .and_then(|name| {
                    styled_text_from_tag_with_style(
                        name,
                        TextComponentStyle {
                            italic: true,
                            ..inherited
                        },
                    )
                })
                .map(|(_, styled)| styled)
        })
}
fn styled_text_from_raw_component_with_style(
    raw: &str,
    inherited: TextComponentStyle,
) -> Option<(String, String)> {
    let value = serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|value| {
            if let serde_json::Value::String(nested) = &value {
                serde_json::from_str::<serde_json::Value>(nested)
                    .ok()
                    .or(Some(value))
            } else {
                Some(value)
            }
        })
        .unwrap_or_else(|| serde_json::Value::String(raw.to_string()));
    styled_text_from_json_value_with_style(value, inherited)
}
fn styled_text_from_tag_with_style(
    tag: &Tag,
    inherited: TextComponentStyle,
) -> Option<(String, String)> {
    if let Tag::String(raw) = tag {
        return styled_text_from_raw_component_with_style(raw, inherited);
    }
    styled_text_from_json_value_with_style(tag_to_json(tag)?, inherited)
}
fn styled_text_from_json_value_with_style(
    value: serde_json::Value,
    inherited: TextComponentStyle,
) -> Option<(String, String)> {
    let styled = json_component_to_ansi(&value, inherited);
    let text = crate::ui::chat::chat_component::plain_chat_text(&styled);
    Some((text, styled))
}
#[derive(Clone, Copy, Default)]
struct TextComponentStyle {
    color: Option<u32>,
    bold: bool,
    italic: bool,
    underlined: bool,
    strikethrough: bool,
    obfuscated: bool,
}
fn item_hover_name_text_component_style(item: &ItemStack) -> TextComponentStyle {
    TextComponentStyle {
        color: Some(crate::ui::gui::hud::item_name_color(item)),
        italic: item.components.get("minecraft:custom_name").is_some(),
        ..TextComponentStyle::default()
    }
}
fn lore_text_component_style() -> TextComponentStyle {
    TextComponentStyle {
        color: crate::ui::scoreboard::chat_color_rgb(5),
        italic: true,
        ..TextComponentStyle::default()
    }
}
fn json_component_to_ansi(value: &serde_json::Value, inherited: TextComponentStyle) -> String {
    match value {
        serde_json::Value::Array(values) => values
            .iter()
            .map(|value| json_component_to_ansi(value, inherited))
            .collect(),
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if (trimmed.starts_with('{') || trimmed.starts_with('['))
                && let Ok(nested) = serde_json::from_str::<serde_json::Value>(trimmed)
                && matches!(
                    nested,
                    serde_json::Value::Object(_) | serde_json::Value::Array(_)
                )
            {
                return json_component_to_ansi(&nested, inherited);
            }
            styled_text_run(text, inherited)
        }
        serde_json::Value::Object(object) => {
            let style = text_component_style(object, inherited);
            let mut content = if let Some(text) = object.get("text").and_then(|v| v.as_str()) {
                styled_text_run(text, style)
            } else if let Some(key) = object.get("translate").and_then(|v| v.as_str()) {
                json_translated_component_to_ansi(object, key, style)
            } else if let Some(value) = object.get("score") {
                let score = value
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                styled_text_run(score, style)
            } else if let Some(selector) = object.get("selector").and_then(|v| v.as_str()) {
                styled_text_run(selector, style)
            } else if let Some(keybind) = object.get("keybind").and_then(|v| v.as_str()) {
                styled_text_run(keybind, style)
            } else {
                String::new()
            };
            if let Some(extra) = object.get("extra") {
                content.push_str(&json_component_to_ansi(extra, style));
            }
            content
        }
        _ => styled_text_run(&value.to_string(), inherited),
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum JsonTranslationFormat {
    Percent { end: usize },
    String { position: Option<usize>, end: usize },
}
fn json_translated_component_to_ansi(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    inherited: TextComponentStyle,
) -> String {
    let translated = crate::ui::i18n::translate(key);
    let template = if translated == key {
        object
            .get("fallback")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(&translated)
    } else {
        &translated
    };
    let arguments = object
        .get("with")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    json_decompose_translation_template(template, arguments, inherited)
        .unwrap_or_else(|_| styled_text_run(template, inherited))
}
fn json_decompose_translation_template(
    template: &str,
    arguments: &[serde_json::Value],
    inherited: TextComponentStyle,
) -> Result<String, ()> {
    let bytes = template.as_bytes();
    let mut result = String::new();
    let mut literal_start = 0usize;
    let mut replacement_index = 0usize;
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if bytes[cursor] != b'%' {
            cursor += 1;
            continue;
        }
        let format = parse_json_translation_format(template, cursor)?;
        if literal_start < cursor {
            result.push_str(&styled_text_run(
                &template[literal_start..cursor],
                inherited,
            ));
        }
        match format {
            JsonTranslationFormat::Percent { end } => {
                result.push_str(&styled_text_run("%", inherited));
                cursor = end;
            }
            JsonTranslationFormat::String { position, end } => {
                let index = position.unwrap_or_else(|| {
                    let current = replacement_index;
                    replacement_index += 1;
                    current
                });
                let argument = arguments.get(index).ok_or(())?;
                result.push_str(&json_component_to_ansi(argument, inherited));
                cursor = end;
            }
        }
        literal_start = cursor;
    }
    if literal_start < template.len() {
        result.push_str(&styled_text_run(&template[literal_start..], inherited));
    }
    Ok(result)
}
fn parse_json_translation_format(
    template: &str,
    start: usize,
) -> Result<JsonTranslationFormat, ()> {
    let bytes = template.as_bytes();
    debug_assert_eq!(bytes.get(start), Some(&b'%'));
    let after_percent = start + 1;
    if after_percent == bytes.len() {
        return Err(());
    }
    let mut digits_end = after_percent;
    while matches!(bytes.get(digits_end), Some(b'0'..=b'9')) {
        digits_end += 1;
    }
    let (position, type_index) =
        if digits_end > after_percent && bytes.get(digits_end) == Some(&b'$') {
            let one_based = template[after_percent..digits_end]
                .parse::<i32>()
                .map_err(|_| ())?;
            let zero_based = one_based.checked_sub(1).ok_or(())?;
            (
                Some(usize::try_from(zero_based).map_err(|_| ())?),
                digits_end + 1,
            )
        } else {
            (None, after_percent)
        };
    match bytes.get(type_index) {
        Some(b'%') if position.is_none() => Ok(JsonTranslationFormat::Percent {
            end: type_index + 1,
        }),
        Some(b's') => Ok(JsonTranslationFormat::String {
            position,
            end: type_index + 1,
        }),
        _ => Err(()),
    }
}
fn text_component_style(
    object: &serde_json::Map<String, serde_json::Value>,
    inherited: TextComponentStyle,
) -> TextComponentStyle {
    let mut style = inherited;
    if object.get("reset").and_then(|v| v.as_bool()) == Some(true) {
        style = TextComponentStyle::default();
    }
    if let Some(color) = object.get("color") {
        style.color = match color {
            serde_json::Value::String(value) => text_color(value),
            _ => None,
        };
    }
    for (key, target) in [
        ("bold", &mut style.bold),
        ("italic", &mut style.italic),
        ("underlined", &mut style.underlined),
        ("strikethrough", &mut style.strikethrough),
        ("obfuscated", &mut style.obfuscated),
    ] {
        if let Some(value) = object.get(key).and_then(|value| value.as_bool()) {
            *target = value;
        }
    }
    style
}
fn text_color(value: &str) -> Option<u32> {
    if let Some(hex) = value.strip_prefix('#') {
        return u32::from_str_radix(hex, 16).ok().filter(|_| hex.len() == 6);
    }
    let ordinal = match value {
        "black" => 0,
        "dark_blue" => 1,
        "dark_green" => 2,
        "dark_aqua" => 3,
        "dark_red" => 4,
        "dark_purple" => 5,
        "gold" => 6,
        "gray" => 7,
        "dark_gray" => 8,
        "blue" => 9,
        "green" => 10,
        "aqua" => 11,
        "red" => 12,
        "light_purple" => 13,
        "yellow" => 14,
        "white" => 15,
        _ => return None,
    };
    crate::ui::scoreboard::chat_color_rgb(ordinal)
}
fn styled_text_run(text: &str, style: TextComponentStyle) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut codes = Vec::new();
    if let Some(color) = style.color {
        codes.push(format!(
            "38;2;{};{};{}",
            (color >> 16) & 0xff,
            (color >> 8) & 0xff,
            color & 0xff
        ));
    }
    if style.bold {
        codes.push("1".to_string());
    }
    if style.italic {
        codes.push("3".to_string());
    }
    if style.underlined {
        codes.push("4".to_string());
    }
    if style.strikethrough {
        codes.push("9".to_string());
    }
    if style.obfuscated {
        codes.push("8".to_string());
    }
    if codes.is_empty() {
        return text.to_string();
    }
    format!("\u{1b}[{}m{}\u{1b}[m", codes.join(";"), text)
}
fn custom_data_compound(item: &ItemStack) -> Option<&[(String, Tag)]> {
    let DataComponent::Generic {
        value: Tag::Compound(fields),
        ..
    } = item.components.get("minecraft:custom_data")?
    else {
        return None;
    };
    Some(fields)
}
fn legacy_display_compound(item: &ItemStack) -> Option<&[(String, Tag)]> {
    let fields = custom_data_compound(item)?;
    match compound_get(fields, "display") {
        Some(Tag::Compound(display)) => Some(display),
        _ => Some(fields),
    }
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
fn translated_item_name(item: &ItemStack) -> String {
    if let Some(DataComponent::CustomName(name)) = item.components.get("minecraft:custom_name") {
        return parse_chat_component(name);
    }
    if let Some(DataComponent::Generic { value, .. }) = item.components.get("minecraft:custom_name")
        && let Some(name) = formatted_text_from_tag(value)
        && !name.is_empty()
    {
        return name;
    }
    let namespace = item.item.location.namespace.as_str();
    let path = item.item.location.path.as_str();
    let is_potion_item = path == "potion"
        || path == "splash_potion"
        || path == "lingering_potion"
        || path == "tipped_arrow";
    if is_potion_item {
        if let Some(DataComponent::Generic {
            value: Tag::Compound(fields),
            ..
        }) = item.components.get("minecraft:potion_contents")
        {
            let potion_name = compound_string(fields, "custom_name")
                .or_else(|| {
                    compound_string(fields, "potion").map(|potion_id| {
                        let potion_path = potion_id
                            .strip_prefix("minecraft:")
                            .unwrap_or(potion_id.as_str());
                        potion_registry_id_to_name(potion_path).to_owned()
                    })
                })
                .unwrap_or_else(|| "empty".to_string());
            let desc_id = format!("item.{}.{}", namespace, path);
            let effect_key = format!("{}.effect.{}", desc_id, potion_name);
            return crate::ui::i18n::translate(&effect_key).to_string();
        }
    }
    if path == "ominous_bottle" {
        let item_key = format!("item.{}.{}", namespace, path);
        let item_name = crate::ui::i18n::translate(&item_key);
        if item_name != item_key {
            if let Some(DataComponent::Generic {
                value: Tag::Int(amp),
                ..
            }) = item.components.get("minecraft:ominous_bottle_amplifier")
            {
                let roman = match amp {
                    0 => "I",
                    1 => "II",
                    2 => "III",
                    3 => "IV",
                    4 => "V",
                    _ => "",
                };
                if !roman.is_empty() {
                    return format!("{} {}", item_name, roman);
                }
            }
            return item_name.to_string();
        }
    }
    if let Some(DataComponent::Generic { value, .. }) = item.components.get("minecraft:item_name") {
        if let Some(name) = formatted_text_from_tag(value) {
            if !name.is_empty() {
                return name;
            }
        }
    }
    let item_key = format!("item.{}.{}", namespace, path);
    let item_name = crate::ui::i18n::translate(&item_key);
    if item_name != item_key {
        return item_name.to_string();
    }
    let block_key = format!("block.{}.{}", namespace, path);
    let block_name = crate::ui::i18n::translate(&block_key);
    if block_name != block_key {
        return block_name.to_string();
    }
    path.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
fn potion_registry_id_to_name(registry_id: &str) -> &str {
    match registry_id {
        "water" => "water",
        "mundane" => "mundane",
        "thick" => "thick",
        "awkward" => "awkward",
        "night_vision" | "long_night_vision" => "night_vision",
        "invisibility" | "long_invisibility" => "invisibility",
        "leaping" | "long_leaping" | "strong_leaping" => "leaping",
        "fire_resistance" | "long_fire_resistance" => "fire_resistance",
        "swiftness" | "long_swiftness" | "strong_swiftness" => "swiftness",
        "slowness" | "long_slowness" | "strong_slowness" => "slowness",
        "turtle_master" | "long_turtle_master" | "strong_turtle_master" => "turtle_master",
        "water_breathing" | "long_water_breathing" => "water_breathing",
        "healing" | "strong_healing" => "healing",
        "harming" | "strong_harming" => "harming",
        "poison" | "long_poison" | "strong_poison" => "poison",
        "regeneration" | "long_regeneration" | "strong_regeneration" => "regeneration",
        "strength" | "long_strength" | "strong_strength" => "strength",
        "weakness" | "long_weakness" => "weakness",
        "luck" => "luck",
        "slow_falling" | "long_slow_falling" => "slow_falling",
        "wind_charged" => "wind_charged",
        "weaving" => "weaving",
        "oozing" => "oozing",
        "infested" => "infested",
        other => other,
    }
}
fn add_potion_tooltip_lines(item: &ItemStack, lines: &mut Vec<TooltipLine>) {
    let Some(DataComponent::Generic {
        value: Tag::Compound(fields),
        ..
    }) = item.components.get("minecraft:potion_contents")
    else {
        let no_effect = crate::ui::i18n::translate("effect.none");
        lines.push(TooltipLine::new(
            no_effect.to_string(),
            [0.5, 0.5, 0.5, 1.0],
        ));
        return;
    };
    let duration_scale = item
        .components
        .get("minecraft:potion_duration_scale")
        .and_then(|component| match component {
            DataComponent::Generic { value, .. } => tag_f32(value),
            _ => None,
        })
        .unwrap_or(1.0);
    let base_effects = compound_string(fields, "potion")
        .map(|potion_id| {
            let potion_path = potion_id
                .strip_prefix("minecraft:")
                .unwrap_or(potion_id.as_str());
            potion_effects_for_id(potion_path)
        })
        .unwrap_or_default();
    let custom_effects = custom_potion_effects(fields);
    if base_effects.is_empty() && custom_effects.is_empty() {
        let no_effect = crate::ui::i18n::translate("effect.none");
        lines.push(TooltipLine::new(
            no_effect.to_string(),
            [0.5, 0.5, 0.5, 1.0],
        ));
        return;
    }
    for effect in &base_effects {
        lines.push(potion_effect_tooltip_line(
            effect.effect_id,
            effect.amplifier,
            effect.duration_ticks,
            effect.is_beneficial,
            duration_scale,
        ));
    }
    for effect in &custom_effects {
        lines.push(potion_effect_tooltip_line(
            &effect.effect_id,
            effect.amplifier,
            effect.duration_ticks,
            effect.is_beneficial,
            duration_scale,
        ));
    }
    let modifiers: Vec<PotionAttributeModifier> = base_effects
        .iter()
        .flat_map(|effect| potion_effect_attribute_modifiers(effect.effect_id, effect.amplifier))
        .chain(custom_effects.iter().flat_map(|effect| {
            potion_effect_attribute_modifiers(&effect.effect_id, effect.amplifier)
        }))
        .collect();
    if !modifiers.is_empty() {
        lines.push(TooltipLine::new(String::new(), TOOLTIP_TEXT));
        lines.push(TooltipLine::new(
            crate::ui::i18n::translate("potion.whenDrank").to_string(),
            [2.0 / 3.0, 0.0, 2.0 / 3.0, 1.0],
        ));
        for modifier in modifiers {
            lines.push(modifier.tooltip_line());
        }
    }
}
fn custom_potion_effects(fields: &[(String, Tag)]) -> Vec<CustomPotionEffectInfo> {
    let Some(Tag::List(entries)) = compound_get(fields, "custom_effects") else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| {
            let Tag::Compound(effect_fields) = entry else {
                return None;
            };
            let effect_id = compound_string(effect_fields, "id")
                .or_else(|| compound_string(effect_fields, "effect"))?;
            let effect_id = effect_id
                .strip_prefix("minecraft:")
                .unwrap_or(effect_id.as_str())
                .to_owned();
            let amplifier = compound_get(effect_fields, "amplifier")
                .and_then(tag_i32)
                .unwrap_or(0)
                .clamp(0, 255);
            let duration_ticks = compound_get(effect_fields, "duration")
                .and_then(tag_i32)
                .unwrap_or(0);
            Some(CustomPotionEffectInfo {
                is_beneficial: potion_effect_is_beneficial(&effect_id),
                effect_id,
                amplifier,
                duration_ticks,
            })
        })
        .collect()
}
fn potion_effect_tooltip_line(
    effect_id: &str,
    amplifier: i32,
    duration_ticks: i32,
    is_beneficial: bool,
    duration_scale: f32,
) -> TooltipLine {
    let effect_name_key = format!("effect.minecraft.{effect_id}");
    let effect_name = crate::ui::i18n::translate(&effect_name_key);
    let mut line_text = if amplifier > 0 {
        let potency_key = format!("potion.potency.{amplifier}");
        let potency = crate::ui::i18n::translate(&potency_key);
        format!("{effect_name} {potency}")
    } else {
        effect_name.to_string()
    };
    if duration_ticks == -1 {
        let infinite = crate::ui::i18n::translate("effect.duration.infinite");
        line_text = format!("{line_text} ({infinite})");
    } else if duration_ticks > 20 {
        let scaled_ticks = (duration_ticks as f32 * duration_scale).floor() as i32;
        let total_seconds = scaled_ticks / 20;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        line_text = format!("{line_text} ({minutes}:{seconds:02})");
    }
    let color = if is_beneficial {
        [1.0 / 3.0, 1.0 / 3.0, 1.0, 1.0]
    } else {
        [1.0, 1.0 / 3.0, 1.0 / 3.0, 1.0]
    };
    TooltipLine::new(line_text, color)
}
fn add_ominous_bottle_tooltip_lines(item: &ItemStack, lines: &mut Vec<TooltipLine>) {
    let amp = if let Some(DataComponent::Generic {
        value: Tag::Int(a), ..
    }) = item.components.get("minecraft:ominous_bottle_amplifier")
    {
        *a
    } else {
        0
    };
    let effect_name = crate::ui::i18n::translate("effect.minecraft.bad_omen");
    let line_text = if amp > 0 {
        let potency_key = format!("potion.potency.{}", amp);
        let potency = crate::ui::i18n::translate(&potency_key);
        format!("{} {}", effect_name, potency)
    } else {
        effect_name.to_string()
    };
    let duration_ticks = 120000;
    let total_seconds = duration_ticks / 20;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let duration_str = format!("({}:{:02})", minutes, seconds);
    let full_text = format!("{} {}", line_text, duration_str);
    lines.push(TooltipLine::new(
        full_text,
        [1.0 / 3.0, 1.0 / 3.0, 1.0, 1.0],
    ));
}
struct PotionEffectInfo {
    effect_id: &'static str,
    amplifier: i32,
    duration_ticks: i32,
    is_beneficial: bool,
}
struct CustomPotionEffectInfo {
    effect_id: String,
    amplifier: i32,
    duration_ticks: i32,
    is_beneficial: bool,
}
#[derive(Clone, Copy)]
struct PotionAttributeModifier {
    attribute_key: &'static str,
    amount: f64,
    operation_id: i32,
}
impl PotionAttributeModifier {
    fn tooltip_line(self) -> TooltipLine {
        let display_amount = if self.operation_id == 1 || self.operation_id == 2 {
            self.amount * 100.0
        } else {
            self.amount
        };
        let (translation_key, amount, color) = if self.amount > 0.0 {
            (
                format!("attribute.modifier.plus.{}", self.operation_id),
                java_attribute_modifier_format(display_amount),
                [1.0 / 3.0, 1.0 / 3.0, 1.0, 1.0],
            )
        } else {
            (
                format!("attribute.modifier.take.{}", self.operation_id),
                java_attribute_modifier_format(-display_amount),
                [1.0, 1.0 / 3.0, 1.0 / 3.0, 1.0],
            )
        };
        let attribute_name = crate::ui::i18n::translate(self.attribute_key);
        let template = crate::ui::i18n::translate(&translation_key);
        TooltipLine::new(
            format_two_percent_args(&template, &amount, &attribute_name),
            color,
        )
    }
}
fn potion_effect_attribute_modifiers(
    effect_id: &str,
    amplifier: i32,
) -> Vec<PotionAttributeModifier> {
    let factor = (amplifier.clamp(0, 255) + 1) as f64;
    let effect_id = effect_id.strip_prefix("minecraft:").unwrap_or(effect_id);
    let modifier = |attribute_key, amount, operation_id| PotionAttributeModifier {
        attribute_key,
        amount: amount * factor,
        operation_id,
    };
    match effect_id {
        "speed" => vec![modifier("attribute.name.movement_speed", 0.2, 2)],
        "slowness" => vec![modifier("attribute.name.movement_speed", -0.15, 2)],
        "haste" => vec![modifier("attribute.name.attack_speed", 0.1, 2)],
        "mining_fatigue" => vec![modifier("attribute.name.attack_speed", -0.1, 2)],
        "strength" => vec![modifier("attribute.name.attack_damage", 3.0, 0)],
        "jump_boost" => vec![modifier("attribute.name.safe_fall_distance", 1.0, 0)],
        "invisibility" => {
            vec![modifier("attribute.name.waypoint_transmit_range", -1.0, 2)]
        }
        "weakness" => vec![modifier("attribute.name.attack_damage", -4.0, 0)],
        "health_boost" => vec![modifier("attribute.name.max_health", 4.0, 0)],
        "absorption" => vec![modifier("attribute.name.max_absorption", 4.0, 0)],
        "luck" => vec![modifier("attribute.name.luck", 1.0, 0)],
        "unluck" => vec![modifier("attribute.name.luck", -1.0, 0)],
        _ => Vec::new(),
    }
}
fn java_attribute_modifier_format(value: f64) -> String {
    let mut formatted = format!("{value:.2}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    formatted
}
fn format_two_percent_args(template: &str, first: &str, second: &str) -> String {
    template
        .replacen("%s", first, 1)
        .replacen("%s", second, 1)
        .replace("%%", "%")
}
fn potion_effect_is_beneficial(effect_id: &str) -> bool {
    !matches!(
        effect_id,
        "slowness"
            | "mining_fatigue"
            | "instant_damage"
            | "nausea"
            | "blindness"
            | "hunger"
            | "weakness"
            | "poison"
            | "wither"
            | "unluck"
            | "levitation"
            | "darkness"
            | "wind_charged"
            | "weaving"
            | "oozing"
            | "infested"
    )
}
fn potion_effects_for_id(potion_id: &str) -> Vec<PotionEffectInfo> {
    match potion_id {
        "water" | "mundane" | "thick" | "awkward" => vec![],
        "night_vision" => {
            vec![PotionEffectInfo {
                effect_id: "night_vision",
                amplifier: 0,
                duration_ticks: 3600,
                is_beneficial: true,
            }]
        }
        "long_night_vision" => {
            vec![PotionEffectInfo {
                effect_id: "night_vision",
                amplifier: 0,
                duration_ticks: 9600,
                is_beneficial: true,
            }]
        }
        "invisibility" => {
            vec![PotionEffectInfo {
                effect_id: "invisibility",
                amplifier: 0,
                duration_ticks: 3600,
                is_beneficial: true,
            }]
        }
        "long_invisibility" => {
            vec![PotionEffectInfo {
                effect_id: "invisibility",
                amplifier: 0,
                duration_ticks: 9600,
                is_beneficial: true,
            }]
        }
        "leaping" => {
            vec![PotionEffectInfo {
                effect_id: "jump_boost",
                amplifier: 0,
                duration_ticks: 3600,
                is_beneficial: true,
            }]
        }
        "long_leaping" => {
            vec![PotionEffectInfo {
                effect_id: "jump_boost",
                amplifier: 0,
                duration_ticks: 9600,
                is_beneficial: true,
            }]
        }
        "strong_leaping" => {
            vec![PotionEffectInfo {
                effect_id: "jump_boost",
                amplifier: 1,
                duration_ticks: 1800,
                is_beneficial: true,
            }]
        }
        "fire_resistance" => {
            vec![PotionEffectInfo {
                effect_id: "fire_resistance",
                amplifier: 0,
                duration_ticks: 3600,
                is_beneficial: true,
            }]
        }
        "long_fire_resistance" => {
            vec![PotionEffectInfo {
                effect_id: "fire_resistance",
                amplifier: 0,
                duration_ticks: 9600,
                is_beneficial: true,
            }]
        }
        "swiftness" => {
            vec![PotionEffectInfo {
                effect_id: "speed",
                amplifier: 0,
                duration_ticks: 3600,
                is_beneficial: true,
            }]
        }
        "long_swiftness" => {
            vec![PotionEffectInfo {
                effect_id: "speed",
                amplifier: 0,
                duration_ticks: 9600,
                is_beneficial: true,
            }]
        }
        "strong_swiftness" => {
            vec![PotionEffectInfo {
                effect_id: "speed",
                amplifier: 1,
                duration_ticks: 1800,
                is_beneficial: true,
            }]
        }
        "slowness" => {
            vec![PotionEffectInfo {
                effect_id: "slowness",
                amplifier: 0,
                duration_ticks: 1800,
                is_beneficial: false,
            }]
        }
        "long_slowness" => {
            vec![PotionEffectInfo {
                effect_id: "slowness",
                amplifier: 0,
                duration_ticks: 4800,
                is_beneficial: false,
            }]
        }
        "strong_slowness" => {
            vec![PotionEffectInfo {
                effect_id: "slowness",
                amplifier: 3,
                duration_ticks: 400,
                is_beneficial: false,
            }]
        }
        "turtle_master" => {
            vec![
                PotionEffectInfo {
                    effect_id: "slowness",
                    amplifier: 3,
                    duration_ticks: 400,
                    is_beneficial: false,
                },
                PotionEffectInfo {
                    effect_id: "resistance",
                    amplifier: 2,
                    duration_ticks: 400,
                    is_beneficial: true,
                },
            ]
        }
        "long_turtle_master" => {
            vec![
                PotionEffectInfo {
                    effect_id: "slowness",
                    amplifier: 3,
                    duration_ticks: 800,
                    is_beneficial: false,
                },
                PotionEffectInfo {
                    effect_id: "resistance",
                    amplifier: 2,
                    duration_ticks: 800,
                    is_beneficial: true,
                },
            ]
        }
        "strong_turtle_master" => {
            vec![
                PotionEffectInfo {
                    effect_id: "slowness",
                    amplifier: 5,
                    duration_ticks: 400,
                    is_beneficial: false,
                },
                PotionEffectInfo {
                    effect_id: "resistance",
                    amplifier: 3,
                    duration_ticks: 400,
                    is_beneficial: true,
                },
            ]
        }
        "water_breathing" => {
            vec![PotionEffectInfo {
                effect_id: "water_breathing",
                amplifier: 0,
                duration_ticks: 3600,
                is_beneficial: true,
            }]
        }
        "long_water_breathing" => {
            vec![PotionEffectInfo {
                effect_id: "water_breathing",
                amplifier: 0,
                duration_ticks: 9600,
                is_beneficial: true,
            }]
        }
        "healing" => {
            vec![PotionEffectInfo {
                effect_id: "instant_health",
                amplifier: 0,
                duration_ticks: 1,
                is_beneficial: true,
            }]
        }
        "strong_healing" => {
            vec![PotionEffectInfo {
                effect_id: "instant_health",
                amplifier: 1,
                duration_ticks: 1,
                is_beneficial: true,
            }]
        }
        "harming" => {
            vec![PotionEffectInfo {
                effect_id: "instant_damage",
                amplifier: 0,
                duration_ticks: 1,
                is_beneficial: false,
            }]
        }
        "strong_harming" => {
            vec![PotionEffectInfo {
                effect_id: "instant_damage",
                amplifier: 1,
                duration_ticks: 1,
                is_beneficial: false,
            }]
        }
        "poison" => {
            vec![PotionEffectInfo {
                effect_id: "poison",
                amplifier: 0,
                duration_ticks: 900,
                is_beneficial: false,
            }]
        }
        "long_poison" => {
            vec![PotionEffectInfo {
                effect_id: "poison",
                amplifier: 0,
                duration_ticks: 1800,
                is_beneficial: false,
            }]
        }
        "strong_poison" => {
            vec![PotionEffectInfo {
                effect_id: "poison",
                amplifier: 1,
                duration_ticks: 432,
                is_beneficial: false,
            }]
        }
        "regeneration" => {
            vec![PotionEffectInfo {
                effect_id: "regeneration",
                amplifier: 0,
                duration_ticks: 900,
                is_beneficial: true,
            }]
        }
        "long_regeneration" => {
            vec![PotionEffectInfo {
                effect_id: "regeneration",
                amplifier: 0,
                duration_ticks: 1800,
                is_beneficial: true,
            }]
        }
        "strong_regeneration" => {
            vec![PotionEffectInfo {
                effect_id: "regeneration",
                amplifier: 1,
                duration_ticks: 450,
                is_beneficial: true,
            }]
        }
        "strength" => {
            vec![PotionEffectInfo {
                effect_id: "strength",
                amplifier: 0,
                duration_ticks: 3600,
                is_beneficial: true,
            }]
        }
        "long_strength" => {
            vec![PotionEffectInfo {
                effect_id: "strength",
                amplifier: 0,
                duration_ticks: 9600,
                is_beneficial: true,
            }]
        }
        "strong_strength" => {
            vec![PotionEffectInfo {
                effect_id: "strength",
                amplifier: 1,
                duration_ticks: 1800,
                is_beneficial: true,
            }]
        }
        "weakness" => {
            vec![PotionEffectInfo {
                effect_id: "weakness",
                amplifier: 0,
                duration_ticks: 1800,
                is_beneficial: false,
            }]
        }
        "long_weakness" => {
            vec![PotionEffectInfo {
                effect_id: "weakness",
                amplifier: 0,
                duration_ticks: 4800,
                is_beneficial: false,
            }]
        }
        "luck" => {
            vec![PotionEffectInfo {
                effect_id: "luck",
                amplifier: 0,
                duration_ticks: 6000,
                is_beneficial: true,
            }]
        }
        "slow_falling" => {
            vec![PotionEffectInfo {
                effect_id: "slow_falling",
                amplifier: 0,
                duration_ticks: 1800,
                is_beneficial: true,
            }]
        }
        "long_slow_falling" => {
            vec![PotionEffectInfo {
                effect_id: "slow_falling",
                amplifier: 0,
                duration_ticks: 4800,
                is_beneficial: true,
            }]
        }
        "wind_charged" => {
            vec![PotionEffectInfo {
                effect_id: "wind_charged",
                amplifier: 0,
                duration_ticks: 3600,
                is_beneficial: false,
            }]
        }
        "weaving" => {
            vec![PotionEffectInfo {
                effect_id: "weaving",
                amplifier: 0,
                duration_ticks: 3600,
                is_beneficial: false,
            }]
        }
        "oozing" => {
            vec![PotionEffectInfo {
                effect_id: "oozing",
                amplifier: 0,
                duration_ticks: 3600,
                is_beneficial: false,
            }]
        }
        "infested" => {
            vec![PotionEffectInfo {
                effect_id: "infested",
                amplifier: 0,
                duration_ticks: 3600,
                is_beneficial: false,
            }]
        }
        _ => vec![],
    }
}
fn compound_get<'a>(fields: &'a [(String, Tag)], key: &str) -> Option<&'a Tag> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}
fn formatted_text_from_tag(tag: &Tag) -> Option<String> {
    match tag {
        Tag::String(raw) => Some(parse_chat_component(raw)),
        Tag::Compound(_) | Tag::List(_) => {
            let json = tag_to_json(tag)?;
            Some(flatten_text_component(&json))
        }
        _ => None,
    }
}
fn tag_to_json(tag: &Tag) -> Option<serde_json::Value> {
    match tag {
        Tag::End => Some(serde_json::Value::Null),
        Tag::Byte(v) => Some(serde_json::json!(*v)),
        Tag::Short(v) => Some(serde_json::json!(*v)),
        Tag::Int(v) => Some(serde_json::json!(*v)),
        Tag::Long(v) => Some(serde_json::json!(*v)),
        Tag::Float(v) => Some(serde_json::json!(*v)),
        Tag::Double(v) => Some(serde_json::json!(*v)),
        Tag::String(v) => Some(serde_json::Value::String(v.clone())),
        Tag::ByteArray(values) => Some(serde_json::Value::Array(
            values.iter().map(|v| serde_json::json!(*v)).collect(),
        )),
        Tag::IntArray(values) => Some(serde_json::Value::Array(
            values.iter().map(|v| serde_json::json!(*v)).collect(),
        )),
        Tag::LongArray(values) => Some(serde_json::Value::Array(
            values.iter().map(|v| serde_json::json!(*v)).collect(),
        )),
        Tag::List(values) => Some(serde_json::Value::Array(
            values.iter().filter_map(tag_to_json).collect(),
        )),
        Tag::Compound(fields) => Some(serde_json::Value::Object(
            fields
                .iter()
                .filter_map(|(key, value)| Some((key.clone(), tag_to_json(value)?)))
                .collect(),
        )),
    }
}
fn vanilla_argb_color(value: i32) -> [f32; 4] {
    let argb = value as u32;
    [
        ((argb >> 16) & 0xFF) as f32 / 255.0,
        ((argb >> 8) & 0xFF) as f32 / 255.0,
        (argb & 0xFF) as f32 / 255.0,
        ((argb >> 24) & 0xFF) as f32 / 255.0,
    ]
}
pub(crate) fn render_tooltip(
    renderer: &mut GuiRenderer,
    font: &FontRenderer,
    tooltip: &DeferredTooltip,
    mouse_x: f32,
    mouse_y: f32,
    screen_w: f32,
    screen_h: f32,
) {
    let measure = |tooltip: &DeferredTooltip| {
        default_tooltip_layout(
            mouse_x as i32,
            mouse_y as i32,
            screen_w as i32,
            screen_h as i32,
            tooltip.width(font),
            tooltip.height(font),
        )
    };
    let layout = measure(tooltip);
    #[cfg(feature = "extensions")]
    let (tooltip, layout, placements) = match mod_tooltip::overlay_placements(
        tooltip,
        font,
        renderer,
        layout,
        [mouse_x as i32, mouse_y as i32],
        [screen_w as i32, screen_h as i32],
    ) {
        Some(placements) => (tooltip, layout, placements),
        None => {
            let original = tooltip.fallback.as_deref().unwrap_or(tooltip);
            (original, measure(original), BTreeMap::new())
        }
    };
    render_tooltip_background(
        renderer,
        [layout.text_x, layout.text_y],
        [layout.text_width, layout.text_height],
        tooltip.style.as_ref(),
    );
    let mut component_y = layout.text_y;
    for (index, component) in tooltip.components.iter().enumerate() {
        if let TooltipComponent::Text(line) = component {
            if line.color_override {
                crate::ui::chat::chat_component::draw_formatted_chat_text_override_color(
                    renderer,
                    font,
                    &line.styled_text(),
                    layout.text_x as f32,
                    component_y as f32,
                    1.0,
                    line.color,
                    true,
                );
            } else {
                draw_formatted_chat_text_ex(
                    renderer,
                    font,
                    &line.styled_text(),
                    layout.text_x as f32,
                    component_y as f32,
                    1.0,
                    1.0,
                    true,
                );
            }
        }
        component_y += component.height(font);
        if index == 0 {
            component_y += JAVA_TOOLTIP_FIRST_LINE_GAP;
        }
    }
    component_y = layout.text_y;
    for (index, component) in tooltip.components.iter().enumerate() {
        #[cfg(feature = "extensions")]
        if let TooltipComponent::Image(TooltipImage::Extension(view)) = component {
            if view.canvas.flow == el_mod_content::tooltip::Flow::Overlay {
                if let Some(placement) = placements.get(&index) {
                    if placement.background {
                        render_tooltip_background(
                            renderer,
                            placement.origin,
                            [view.canvas.width as i32, view.canvas.height as i32],
                            None,
                        );
                    }
                    view.render_at(renderer, font, placement.origin[0], placement.origin[1]);
                }
            } else {
                view.render(
                    renderer,
                    font,
                    layout.text_x,
                    component_y,
                    layout.text_width,
                );
            }
        }
        if let TooltipComponent::Image(TooltipImage::Bundle(contents)) = component {
            render_bundle_tooltip_image(
                renderer,
                font,
                contents,
                layout.text_x,
                component_y,
                layout.text_width,
                screen_w,
                screen_h,
            );
        }
        component_y += component.height(font);
        if index == 0 {
            component_y += JAVA_TOOLTIP_FIRST_LINE_GAP;
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BundleTooltipGridCell {
    Surplus {
        x: i32,
        y: i32,
        hidden_item_count: i32,
    },
    Item {
        x: i32,
        y: i32,
        item_index: usize,
        selected: bool,
    },
}
impl BundleTooltipContents {
    fn grid_cells(
        &self,
        tooltip_x: i32,
        tooltip_y: i32,
        tooltip_width: i32,
    ) -> Vec<BundleTooltipGridCell> {
        let is_overflowing = self.items.len() > BUNDLE_TOOLTIP_MAX_SLOTS;
        let shown_item_count = self.number_of_items_to_show();
        let x_start = tooltip_x + (tooltip_width - BUNDLE_TOOLTIP_WIDTH) / 2 + BUNDLE_TOOLTIP_WIDTH;
        let y_start = tooltip_y + self.item_grid_height();
        let hidden_item_count = self.items[shown_item_count..]
            .iter()
            .fold(0_i32, |sum, item| sum.wrapping_add(item.count));
        let mut cells = Vec::new();
        let mut slot_number = 1usize;
        for row_number in 1..=self.grid_rows() {
            for column_number in 1..=BUNDLE_TOOLTIP_GRID_COLUMNS {
                let x = x_start
                    - i32::try_from(column_number).unwrap_or(i32::MAX) * BUNDLE_TOOLTIP_SLOT_SIZE;
                let y = y_start
                    - i32::try_from(row_number).unwrap_or(i32::MAX) * BUNDLE_TOOLTIP_SLOT_SIZE;
                if is_overflowing && column_number * row_number == 1 {
                    cells.push(BundleTooltipGridCell::Surplus {
                        x,
                        y,
                        hidden_item_count,
                    });
                } else if shown_item_count >= slot_number {
                    let item_index = shown_item_count - slot_number;
                    cells.push(BundleTooltipGridCell::Item {
                        x,
                        y,
                        item_index,
                        selected: self.selected_item == Some(item_index),
                    });
                    slot_number += 1;
                }
            }
        }
        cells
    }
}
fn render_bundle_tooltip_image(
    renderer: &mut GuiRenderer,
    font: &FontRenderer,
    contents: &BundleTooltipContents,
    x: i32,
    y: i32,
    tooltip_width: i32,
    screen_width: f32,
    screen_height: f32,
) {
    let Some(weight) = contents.weight() else {
        return;
    };
    let content_x = x + (tooltip_width - BUNDLE_TOOLTIP_WIDTH) / 2;
    if contents.is_empty() {
        let description_lines = contents.empty_description_lines(font);
        for (line_index, line) in description_lines.iter().enumerate() {
            renderer.draw_text_top_with_shadow(
                font,
                line,
                content_x as f32,
                (y + i32::try_from(line_index).unwrap_or(i32::MAX) * 9) as f32,
                vanilla_argb_color(-5592406),
                1.0,
            );
        }
        let description_height = i32::try_from(description_lines.len()).unwrap_or(i32::MAX) * 9;
        render_bundle_progress_bar(
            renderer,
            font,
            content_x,
            y + description_height + BUNDLE_TOOLTIP_PROGRESS_MARGIN_Y,
            weight,
        );
        return;
    }
    for cell in contents.grid_cells(x, y, tooltip_width) {
        match cell {
            BundleTooltipGridCell::Surplus {
                x,
                y,
                hidden_item_count,
            } => {
                let text = format!("+{hidden_item_count}");
                let text_width = font.measure_width(&text, 1.0).ceil() as i32;
                renderer.draw_text_top_with_shadow(
                    font,
                    &text,
                    (x + BUNDLE_TOOLTIP_SLOT_SIZE / 2 - text_width / 2) as f32,
                    (y + 10) as f32,
                    [1.0, 1.0, 1.0, 1.0],
                    1.0,
                );
            }
            BundleTooltipGridCell::Item {
                x,
                y,
                item_index,
                selected,
            } => {
                let background = if selected {
                    ResourceLocation::new("minecraft", "container/bundle/slot_highlight_back")
                } else {
                    ResourceLocation::new("minecraft", "container/bundle/slot_background")
                };
                renderer.draw_sprite(
                    x as f32,
                    y as f32,
                    BUNDLE_TOOLTIP_SLOT_SIZE as f32,
                    BUNDLE_TOOLTIP_SLOT_SIZE as f32,
                    &background,
                    [1.0, 1.0, 1.0, 1.0],
                );
                draw_gui_item(
                    renderer,
                    font,
                    &contents.items[item_index],
                    (x + 4) as f32,
                    (y + 4) as f32,
                    0.0,
                    true,
                );
                if selected {
                    renderer.draw_sprite(
                        x as f32,
                        y as f32,
                        BUNDLE_TOOLTIP_SLOT_SIZE as f32,
                        BUNDLE_TOOLTIP_SLOT_SIZE as f32,
                        &ResourceLocation::new(
                            "minecraft",
                            "container/bundle/slot_highlight_front",
                        ),
                        [1.0, 1.0, 1.0, 1.0],
                    );
                }
            }
        }
    }
    render_bundle_selected_item_tooltip(
        renderer,
        font,
        contents,
        x,
        y,
        tooltip_width,
        screen_width,
        screen_height,
    );
    render_bundle_progress_bar(
        renderer,
        font,
        content_x,
        y + contents.item_grid_height() + BUNDLE_TOOLTIP_PROGRESS_MARGIN_Y,
        weight,
    );
}
fn bundle_selected_item_tooltip_origin(
    bundle_tooltip_x: i32,
    bundle_tooltip_y: i32,
    bundle_tooltip_width: i32,
    selected_item_text_width: i32,
) -> (i32, i32) {
    let center_tooltip = bundle_tooltip_x + bundle_tooltip_width / 2 - 12;
    (
        center_tooltip - selected_item_text_width / 2,
        bundle_tooltip_y - 15,
    )
}
fn render_bundle_selected_item_tooltip(
    renderer: &mut GuiRenderer,
    font: &FontRenderer,
    contents: &BundleTooltipContents,
    bundle_tooltip_x: i32,
    bundle_tooltip_y: i32,
    bundle_tooltip_width: i32,
    screen_width: f32,
    screen_height: f32,
) {
    let Some((selected_tooltip, tooltip_x, tooltip_y)) = bundle_selected_item_tooltip_request(
        contents,
        bundle_tooltip_x,
        bundle_tooltip_y,
        bundle_tooltip_width,
        font,
    ) else {
        return;
    };
    render_tooltip(
        renderer,
        font,
        &selected_tooltip,
        tooltip_x as f32,
        tooltip_y as f32,
        screen_width,
        screen_height,
    );
}
fn bundle_selected_item_tooltip_request(
    contents: &BundleTooltipContents,
    bundle_tooltip_x: i32,
    bundle_tooltip_y: i32,
    bundle_tooltip_width: i32,
    font: &FontRenderer,
) -> Option<(DeferredTooltip, i32, i32)> {
    let Some(selected_index) = contents.selected_item else {
        return None;
    };
    let Some(item) = contents.items.get(selected_index) else {
        return None;
    };
    let Some(selected_tooltip) = DeferredTooltip::new(
        vec![tooltip_name_line(item, tooltip_item_name_color(item))],
        None,
        tooltip_style(item),
    ) else {
        return None;
    };
    let (tooltip_x, tooltip_y) = bundle_selected_item_tooltip_origin(
        bundle_tooltip_x,
        bundle_tooltip_y,
        bundle_tooltip_width,
        selected_tooltip.width(font),
    );
    Some((selected_tooltip, tooltip_x, tooltip_y))
}
fn render_bundle_progress_bar(
    renderer: &mut GuiRenderer,
    font: &FontRenderer,
    x: i32,
    y: i32,
    weight: BundleWeight,
) {
    let fill_width = weight
        .multiply_and_truncate(BUNDLE_TOOLTIP_PROGRESS_FILL_MAX)
        .unwrap_or(BUNDLE_TOOLTIP_PROGRESS_FILL_MAX)
        .clamp(0, BUNDLE_TOOLTIP_PROGRESS_FILL_MAX);
    if fill_width > 0 {
        let fill_sprite = if weight.is_full_or_more() {
            ResourceLocation::new("minecraft", "container/bundle/bundle_progressbar_full")
        } else {
            ResourceLocation::new("minecraft", "container/bundle/bundle_progressbar_fill")
        };
        renderer.draw_sprite(
            (x + 1) as f32,
            y as f32,
            fill_width as f32,
            BUNDLE_TOOLTIP_PROGRESS_HEIGHT as f32,
            &fill_sprite,
            [1.0, 1.0, 1.0, 1.0],
        );
    }
    renderer.draw_sprite(
        x as f32,
        y as f32,
        BUNDLE_TOOLTIP_WIDTH as f32,
        BUNDLE_TOOLTIP_PROGRESS_HEIGHT as f32,
        &ResourceLocation::new("minecraft", "container/bundle/bundle_progressbar_border"),
        [1.0, 1.0, 1.0, 1.0],
    );
    let fill_text = if weight.is_zero() {
        Some(crate::ui::i18n::translate("item.minecraft.bundle.empty"))
    } else if weight.is_full_or_more() {
        Some(crate::ui::i18n::translate("item.minecraft.bundle.full"))
    } else {
        None
    };
    if let Some(fill_text) = fill_text {
        let text_width = font.measure_width(&fill_text, 1.0).ceil() as i32;
        renderer.draw_text_top_with_shadow(
            font,
            &fill_text,
            (x + BUNDLE_TOOLTIP_WIDTH / 2 - text_width / 2) as f32,
            (y + 3) as f32,
            [1.0, 1.0, 1.0, 1.0],
            1.0,
        );
    }
}
fn pressed_bound_input(event: &winit::event::KeyEvent) -> Option<BoundInput> {
    if event.repeat || event.state != ElementState::Pressed {
        return None;
    }
    match event.physical_key {
        PhysicalKey::Code(code) => Some(BoundInput::Key(code)),
        _ => None,
    }
}
#[allow(dead_code)]
pub fn generate_slot_coords(menu_type: MenuType, slots_len: usize) -> Vec<(i32, i32)> {
    el_inventory::layout::slot_positions(menu_type, slots_len)
        .into_iter()
        .map(|pos| (pos.x, pos.y))
        .collect()
}
fn add_stored_enchantment_tooltip_lines(item: &ItemStack, lines: &mut Vec<TooltipLine>) {
    let metadata = enchantment_tooltip_metadata(item);
    let modern = match item.components.get("minecraft:stored_enchantments") {
        Some(DataComponent::Generic { value, .. }) => Some(value),
        _ => None,
    };
    if let Some(value) = modern {
        append_enchantment_entries(value, &metadata, lines);
    } else if let Some(fields) = custom_data_compound(item) {
        if let Some(value) = compound_get(fields, "StoredEnchantments") {
            append_enchantment_entries(value, &metadata, lines);
        }
    }
}
fn add_item_enchantment_tooltip_lines(item: &ItemStack, lines: &mut Vec<TooltipLine>) {
    let metadata = enchantment_tooltip_metadata(item);
    let modern = match item.components.get("minecraft:enchantments") {
        Some(DataComponent::Generic { value, .. }) => Some(value),
        _ => None,
    };
    if let Some(value) = modern {
        append_enchantment_entries(value, &metadata, lines);
    } else if let Some(fields) = custom_data_compound(item) {
        if let Some(value) = compound_get(fields, "Enchantments") {
            append_enchantment_entries(value, &metadata, lines);
        }
    }
}
const ENCHANTMENT_TOOLTIP_METADATA_COMPONENT: &str = "rustore:enchantment_tooltip_metadata";
#[derive(Clone, Debug, Default)]
struct EnchantmentTooltipMetadata {
    definitions: BTreeMap<String, EnchantmentTooltipDefinition>,
    tooltip_order: Vec<String>,
    curses: HashSet<String>,
    has_synced_tooltip_order: bool,
    has_synced_curse_tag: bool,
}
#[derive(Clone, Debug)]
struct EnchantmentTooltipDefinition {
    description: serde_json::Value,
    max_level: Option<i32>,
}
fn enchantment_tooltip_metadata(item: &ItemStack) -> EnchantmentTooltipMetadata {
    let Some(DataComponent::Generic {
        value: Tag::Compound(fields),
        ..
    }) = item.components.get(ENCHANTMENT_TOOLTIP_METADATA_COMPONENT)
    else {
        return EnchantmentTooltipMetadata::default();
    };
    let mut metadata = EnchantmentTooltipMetadata {
        has_synced_tooltip_order: compound_get(fields, "has_tooltip_order")
            .and_then(tag_bool)
            .unwrap_or(false),
        has_synced_curse_tag: compound_get(fields, "has_curse_tag")
            .and_then(tag_bool)
            .unwrap_or(false),
        ..EnchantmentTooltipMetadata::default()
    };
    if let Some(Tag::List(order)) = compound_get(fields, "tooltip_order") {
        metadata
            .tooltip_order
            .extend(order.iter().filter_map(|entry| {
                let Tag::String(id) = entry else {
                    return None;
                };
                Some(id.clone())
            }));
    }
    if let Some(Tag::List(curses)) = compound_get(fields, "curses") {
        metadata.curses.extend(curses.iter().filter_map(|entry| {
            let Tag::String(id) = entry else {
                return None;
            };
            Some(id.clone())
        }));
    }
    if let Some(Tag::Compound(definitions)) = compound_get(fields, "definitions") {
        for (id, definition) in definitions {
            let Tag::Compound(definition) = definition else {
                continue;
            };
            let Some(description) = compound_get(definition, "description").and_then(tag_to_json)
            else {
                continue;
            };
            metadata.definitions.insert(
                id.clone(),
                EnchantmentTooltipDefinition {
                    description,
                    max_level: compound_get(definition, "max_level").and_then(tag_i32),
                },
            );
        }
    }
    metadata
}
fn append_enchantment_entries(
    value: &Tag,
    metadata: &EnchantmentTooltipMetadata,
    lines: &mut Vec<TooltipLine>,
) {
    match value {
        Tag::Compound(fields) => {
            if compound_get(fields, "show_in_tooltip")
                .and_then(tag_bool)
                .is_some_and(|show| !show)
            {
                return;
            }
            if let Some(levels) = compound_get(fields, "levels") {
                append_enchantment_entries(levels, metadata, lines);
                return;
            }
            append_ordered_enchantment_entries(
                fields
                    .iter()
                    .filter(|(ench_id, _)| ench_id != "show_in_tooltip" && ench_id != "levels")
                    .filter_map(|(ench_id, level_tag)| {
                        let level = tag_i32(level_tag).unwrap_or(1);
                        (level > 0).then(|| (ench_id.clone(), level))
                    })
                    .collect(),
                metadata,
                lines,
            );
        }
        Tag::List(entries) => {
            let mut decoded = Vec::new();
            for entry in entries {
                let Tag::Compound(fields) = entry else {
                    continue;
                };
                let Some(id) = compound_string(fields, "id") else {
                    continue;
                };
                let level = compound_get(fields, "lvl")
                    .or_else(|| compound_get(fields, "level"))
                    .and_then(tag_i32)
                    .unwrap_or(1);
                if level > 0 {
                    decoded.push((id, level));
                }
            }
            append_ordered_enchantment_entries(decoded, metadata, lines);
        }
        _ => {}
    }
}
const VANILLA_ENCHANTMENT_TOOLTIP_ORDER: &[&str] = &[
    "minecraft:binding_curse",
    "minecraft:vanishing_curse",
    "minecraft:riptide",
    "minecraft:channeling",
    "minecraft:wind_burst",
    "minecraft:frost_walker",
    "minecraft:lunge",
    "minecraft:sharpness",
    "minecraft:smite",
    "minecraft:bane_of_arthropods",
    "minecraft:impaling",
    "minecraft:power",
    "minecraft:density",
    "minecraft:breach",
    "minecraft:piercing",
    "minecraft:sweeping_edge",
    "minecraft:multishot",
    "minecraft:fire_aspect",
    "minecraft:flame",
    "minecraft:knockback",
    "minecraft:punch",
    "minecraft:protection",
    "minecraft:blast_protection",
    "minecraft:fire_protection",
    "minecraft:projectile_protection",
    "minecraft:feather_falling",
    "minecraft:fortune",
    "minecraft:looting",
    "minecraft:silk_touch",
    "minecraft:luck_of_the_sea",
    "minecraft:efficiency",
    "minecraft:quick_charge",
    "minecraft:lure",
    "minecraft:respiration",
    "minecraft:aqua_affinity",
    "minecraft:soul_speed",
    "minecraft:swift_sneak",
    "minecraft:depth_strider",
    "minecraft:thorns",
    "minecraft:loyalty",
    "minecraft:unbreaking",
    "minecraft:infinity",
    "minecraft:mending",
];
fn append_ordered_enchantment_entries(
    mut entries: Vec<(String, i32)>,
    metadata: &EnchantmentTooltipMetadata,
    lines: &mut Vec<TooltipLine>,
) {
    if metadata.has_synced_tooltip_order {
        entries.sort_by_key(|(ench_id, _)| {
            metadata
                .tooltip_order
                .iter()
                .position(|known| known == ench_id)
                .unwrap_or(usize::MAX)
        });
    } else {
        entries.sort_by_key(|(ench_id, _)| {
            VANILLA_ENCHANTMENT_TOOLTIP_ORDER
                .iter()
                .position(|known| *known == ench_id)
                .unwrap_or(usize::MAX)
        });
    }
    for (ench_id, level) in entries {
        append_one_enchantment(&ench_id, level, metadata, lines);
    }
}
const ENCHANTMENT_GRAY_RGB: u32 = 0xAAAAAA;
const ENCHANTMENT_CURSE_RED_RGB: u32 = 0xFF5555;
fn append_one_enchantment(
    ench_id: &str,
    level: i32,
    metadata: &EnchantmentTooltipMetadata,
    lines: &mut Vec<TooltipLine>,
) {
    let normalized = ench_id.strip_prefix("minecraft:").unwrap_or(ench_id);
    let is_curse = if metadata.has_synced_curse_tag {
        metadata.curses.contains(ench_id)
    } else {
        normalized.contains("curse")
    };
    let color_rgb = if is_curse {
        ENCHANTMENT_CURSE_RED_RGB
    } else {
        ENCHANTMENT_GRAY_RGB
    };
    let color = [
        ((color_rgb >> 16) & 0xff) as f32 / 255.0,
        ((color_rgb >> 8) & 0xff) as f32 / 255.0,
        (color_rgb & 0xff) as f32 / 255.0,
        1.0,
    ];
    let max_level = metadata
        .definitions
        .get(ench_id)
        .and_then(|definition| definition.max_level)
        .unwrap_or_else(|| {
            if vanilla_enchantment_max_level_is_one(normalized) {
                1
            } else {
                i32::MAX
            }
        });
    let show_level = level != 1 || max_level != 1;
    let level_suffix = show_level
        .then(|| crate::ui::i18n::translate(&format!("enchantment.level.{level}")).to_string());
    if let Some(definition) = metadata.definitions.get(ench_id) {
        let inherited = TextComponentStyle {
            color: Some(color_rgb),
            ..TextComponentStyle::default()
        };
        let mut styled = json_component_to_ansi(&definition.description, inherited);
        let mut text = crate::ui::chat::chat_component::plain_chat_text(&styled);
        if let Some(level_suffix) = level_suffix {
            text.push(' ');
            text.push_str(&level_suffix);
            styled.push_str(&styled_text_run(&format!(" {level_suffix}"), inherited));
        }
        lines.push(TooltipLine::new(text, color).with_styled_text(styled));
        return;
    }
    let key = match normalized {
        "curse_of_binding" => "enchantment.minecraft.binding_curse".to_string(),
        "curse_of_vanishing" => "enchantment.minecraft.vanishing_curse".to_string(),
        _ => format!("enchantment.minecraft.{}", normalized),
    };
    let name = crate::ui::i18n::translate(&key);
    let text = level_suffix
        .map(|suffix| format!("{} {}", name, suffix))
        .unwrap_or_else(|| name.into_owned());
    lines.push(TooltipLine::new(text, color));
}
fn vanilla_enchantment_max_level_is_one(id: &str) -> bool {
    matches!(
        id,
        "aqua_affinity"
            | "binding_curse"
            | "channeling"
            | "flame"
            | "infinity"
            | "mending"
            | "multishot"
            | "silk_touch"
            | "vanishing_curse"
    )
}
fn tag_i32(tag: &Tag) -> Option<i32> {
    match tag {
        Tag::Byte(v) => Some(i32::from(*v)),
        Tag::Short(v) => Some(i32::from(*v)),
        Tag::Int(v) => Some(*v),
        Tag::Long(v) => i32::try_from(*v).ok(),
        _ => None,
    }
}
fn tag_f32(tag: &Tag) -> Option<f32> {
    match tag {
        Tag::Byte(v) => Some(f32::from(*v)),
        Tag::Short(v) => Some(f32::from(*v)),
        Tag::Int(v) => Some(*v as f32),
        Tag::Long(v) => Some(*v as f32),
        Tag::Float(v) => Some(*v),
        Tag::Double(v) => Some(*v as f32),
        _ => None,
    }
}
fn tag_bool(tag: &Tag) -> Option<bool> {
    match tag {
        Tag::Byte(value) => Some(*value != 0),
        Tag::Short(value) => Some(*value != 0),
        Tag::Int(value) => Some(*value != 0),
        Tag::Long(value) => Some(*value != 0),
        Tag::String(value) => match value.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        },
        _ => None,
    }
}
fn compound_string(fields: &[(String, Tag)], key: &str) -> Option<String> {
    match compound_get(fields, key)? {
        Tag::String(value) => Some(value.clone()),
        Tag::Compound(nested) => compound_string(nested, "id")
            .or_else(|| compound_string(nested, "name"))
            .or_else(|| compound_string(nested, "asset_id")),
        _ => None,
    }
}
fn add_goat_horn_tooltip_lines(item: &ItemStack, lines: &mut Vec<TooltipLine>) {
    if let Some(DataComponent::Generic {
        value: Tag::String(inst_name),
        ..
    }) = item.components.get("minecraft:instrument")
    {
        let inst_key = format!(
            "instrument.{}",
            inst_name.strip_prefix("minecraft:").unwrap_or(inst_name)
        );
        let name = crate::ui::i18n::translate(&inst_key);
        lines.push(TooltipLine::new(name.to_string(), [0.62, 0.62, 0.62, 1.0]));
    }
}
fn add_suspicious_stew_tooltip_lines(item: &ItemStack, lines: &mut Vec<TooltipLine>) {
    if let Some(DataComponent::Generic {
        value: Tag::List(effects),
        ..
    }) = item.components.get("minecraft:suspicious_stew_effects")
    {
        for tag in effects {
            if let Tag::Compound(m) = tag {
                let effect_name = match compound_get(m, "id") {
                    Some(Tag::String(s)) => s.as_str(),
                    _ => "",
                };
                let duration = match compound_get(m, "duration") {
                    Some(Tag::Int(d)) => *d,
                    Some(Tag::Byte(d)) => *d as i32,
                    Some(Tag::Short(d)) => *d as i32,
                    _ => 160,
                };
                let effect_key = format!(
                    "effect.{}",
                    effect_name
                        .strip_prefix("minecraft:")
                        .unwrap_or(effect_name)
                );
                let name = crate::ui::i18n::translate(&effect_key);
                let seconds = duration / 20;
                let mins = seconds / 60;
                let secs = seconds % 60;
                let line_text = format!("{} ({}:{:02})", name, mins, secs);
                lines.push(TooltipLine::new(line_text, [0.62, 0.62, 0.62, 1.0]));
            }
        }
    }
}
fn parse_chat_component(raw_json: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw_json) {
        flatten_text_component(&value)
    } else {
        raw_json.to_string()
    }
}
pub(crate) fn flatten_text_component(value: &serde_json::Value) -> String {
    crate::ui::chat::chat_component::plain_chat_text(&json_component_to_ansi(
        value,
        TextComponentStyle::default(),
    ))
}
pub(crate) fn styled_text_component(value: &serde_json::Value) -> String {
    json_component_to_ansi(value, TextComponentStyle::default())
}
fn add_painting_tooltip_lines(item: &ItemStack, lines: &mut Vec<TooltipLine>) {
    if let Some(DataComponent::Generic {
        value: Tag::String(variant_name),
        ..
    }) = item.components.get("minecraft:painting/variant")
    {
        let variant_path = variant_name
            .strip_prefix("minecraft:")
            .unwrap_or(variant_name.as_str());
        let title_key = format!("painting.minecraft.{}.title", variant_path);
        let title_text = crate::ui::i18n::translate(&title_key);
        lines.push(TooltipLine::new(
            title_text.to_string(),
            [1.0, 1.0, 0.33, 1.0],
        ));
        let author_key = format!("painting.minecraft.{}.author", variant_path);
        let author_text = crate::ui::i18n::translate(&author_key);
        if author_text != author_key {
            lines.push(TooltipLine::new(
                author_text.to_string(),
                [0.62, 0.62, 0.62, 1.0],
            ));
        }
        let (w, h) = get_painting_dimensions(variant_path);
        lines.push(TooltipLine::new(
            format!("{}x{}", w, h),
            [0.62, 0.62, 0.62, 1.0],
        ));
    }
}
fn get_painting_dimensions(variant: &str) -> (i32, i32) {
    match variant {
        "kebab" | "aztec" | "alban" | "aztec2" | "bomb" | "plant" | "wasteland" | "meditative" => {
            (1, 1)
        }
        "pool" | "courbet" | "sea" | "sunset" | "creebet" => (2, 1),
        "wanderer" | "graham" | "prairie_ride" => (1, 2),
        "match" | "bust" | "stage" | "void" | "skull_and_roses" | "wither" | "earth" | "wind"
        | "water" | "fire" | "baroque" | "humble" => (2, 2),
        "fighters" | "changing" | "finding" | "lowmist" | "passage" => (4, 2),
        "pointer" | "pigscene" | "burning_skull" | "unpacked" | "orb" => (4, 4),
        "skeleton" | "donkey_kong" => (4, 3),
        "backyard" | "pond" => (3, 4),
        "bouquet" | "cavebird" | "cotan" | "endboss" | "fern" | "owlemons" | "sunflowers"
        | "tides" | "dennis" => (3, 3),
        _ => (1, 1),
    }
}
