use crate::FontWidthMeasurer;
use crate::components::click_sound_sink;
use crate::components::{
    Button, GuiEventListener, Renderable, TabOrderedElement, WidgetClickSound,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{NarratableEntry, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{
    CharacterEvent, CursorType, KeyEvent, MouseButtonEvent, MouseButtonInfo,
};
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackCompatibility {
    Unknown,
    TooOld,
    Compatible,
    TooNew,
}
impl PackCompatibility {
    pub fn is_compatible(self) -> bool {
        self == Self::Compatible
    }
    pub fn description_key(self) -> &'static str {
        match self {
            Self::Unknown => "pack.incompatible.unknown",
            Self::TooOld => "pack.incompatible.old",
            Self::Compatible => "",
            Self::TooNew => "pack.incompatible.new",
        }
    }
    pub fn confirmation_key(self) -> &'static str {
        match self {
            Self::Unknown => "pack.incompatible.confirm.unknown",
            Self::TooOld => "pack.incompatible.confirm.old",
            Self::TooNew => "pack.incompatible.confirm.new",
            Self::Compatible => "",
        }
    }
    pub fn unknown() -> Self {
        Self::Unknown
    }
}
pub fn pack_compatibility_for_formats(min_format: u32, max_format: u32) -> PackCompatibility {
    const CURRENT_FORMAT: u32 = 88;
    if max_format < CURRENT_FORMAT {
        PackCompatibility::TooOld
    } else if min_format > CURRENT_FORMAT {
        PackCompatibility::TooNew
    } else {
        PackCompatibility::Compatible
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackSource {
    Local,
    BuiltIn,
}
impl PackSource {
    pub fn decorate(self, description: &str) -> String {
        match self {
            Self::BuiltIn => {
                let source = crate::translate("pack.source.builtin");
                crate::translate("pack.nameAndSource")
                    .replacen("%s", description, 1)
                    .replacen("%s", &source, 1)
            }
            Self::Local => description.to_string(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackEntry {
    pub id: String,
    pub title: String,
    pub description: String,
    pub compatibility: PackCompatibility,
    pub source: PackSource,
    pub icon_path: String,
    pub has_icon: bool,
    pub fixed_position: bool,
    pub required: bool,
}
impl PackEntry {
    pub fn extended_description(&self) -> String {
        if self.compatibility.is_compatible() {
            self.source.decorate(&self.description)
        } else {
            let compat_desc = crate::translate(self.compatibility.description_key());
            format!(
                "{}\n{}",
                self.source.decorate(&self.description),
                compat_desc
            )
        }
    }
    pub fn can_select(&self) -> bool {
        true
    }
    pub fn can_unselect(&self) -> bool {
        !self.required
    }
}
pub const LIST_WIDTH: i32 = 200;
pub const ENTRY_HEIGHT: i32 = 36;
pub const LIST_TOP_PADDING: i32 = 2;
pub const LIST_HEADER_HEIGHT: i32 = 13;
pub const ICON_SIZE: i32 = 32;
pub const HEADER_HEIGHT: i32 = 49;
pub const FOOTER_HEIGHT: i32 = 33;
pub const BUTTON_H: i32 = 20;
pub const FOOTER_BUTTON_WIDTH: i32 = 150;
pub const FOOTER_BUTTON_GAP: i32 = 8;
pub const SCROLL_SPEED: f32 = 36.0;
struct FooterButtonHandle {
    button: Rc<RefCell<Button<ResourcePacksAction>>>,
}
impl FooterButtonHandle {
    fn new(button: Rc<RefCell<Button<ResourcePacksAction>>>) -> Self {
        Self { button }
    }
}
impl Renderable for FooterButtonHandle {
    fn extract_render_state(
        &mut self,
        _graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
    }
}
impl TabOrderedElement for FooterButtonHandle {
    fn tab_order_group(&self) -> i32 {
        self.button.borrow().tab_order_group()
    }
}
impl GuiEventListener for FooterButtonHandle {
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.button.borrow().is_mouse_over(mouse_x, mouse_y)
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.button.borrow_mut().mouse_clicked(event, double_click)
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        self.button.borrow_mut().mouse_released(event)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        self.button.borrow_mut().key_pressed(event)
    }
    fn set_focused(&mut self, focused: bool) {
        self.button.borrow_mut().set_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.button.borrow().is_focused()
    }
    fn rectangle(&self) -> ScreenRectangle {
        GuiEventListener::rectangle(&*self.button.borrow())
    }
    fn next_focus_path(&self, event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.button.borrow().next_focus_path(event)
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.button.borrow().play_down_sound()
    }
}
impl LayoutElement for FooterButtonHandle {
    fn set_x(&mut self, x: i32) {
        self.button.borrow_mut().set_x(x);
    }
    fn set_y(&mut self, y: i32) {
        self.button.borrow_mut().set_y(y);
    }
    fn x(&self) -> i32 {
        self.button.borrow().x()
    }
    fn y(&self) -> i32 {
        self.button.borrow().y()
    }
    fn width(&self) -> i32 {
        self.button.borrow().width()
    }
    fn height(&self) -> i32 {
        self.button.borrow().height()
    }
}
impl NarratableEntry for FooterButtonHandle {
    fn narration_priority(&self) -> NarrationPriority {
        self.button.borrow().narration_priority()
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        self.button.borrow().update_narration(output);
    }
}
pub struct ResourcePacksScreen {
    base: ScreenBase,
    open_folder_button: Rc<RefCell<Button<ResourcePacksAction>>>,
    done_button: Rc<RefCell<Button<ResourcePacksAction>>>,
    pub font: Arc<dyn FontWidthMeasurer>,
    pub in_world: bool,
    pub available: Vec<PackEntry>,
    pub selected: Vec<PackEntry>,
    pub search: String,
    pub scroll_available: f32,
    pub scroll_selected: f32,
    dragging_scrollbar: Option<bool>,
    cursor_ticks: u32,
    pending_action: Option<ResourcePacksAction>,
    last_width: i32,
    last_height: i32,
}
impl std::fmt::Debug for ResourcePacksScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourcePacksScreen")
            .field("in_world", &self.in_world)
            .field("available", &self.available)
            .field("selected", &self.selected)
            .field("search", &self.search)
            .field("scroll_available", &self.scroll_available)
            .field("scroll_selected", &self.scroll_selected)
            .finish()
    }
}
impl Clone for ResourcePacksScreen {
    fn clone(&self) -> Self {
        let mut s = Self::new(
            self.in_world,
            self.available.clone(),
            self.selected.clone(),
            Arc::clone(&self.font),
        );
        s.search = self.search.clone();
        s.scroll_available = self.scroll_available;
        s.scroll_selected = self.scroll_selected;
        s.dragging_scrollbar = self.dragging_scrollbar;
        s.cursor_ticks = self.cursor_ticks;
        s.pending_action = self.pending_action.clone();
        s
    }
}
impl ResourcePacksScreen {
    pub fn new(
        in_world: bool,
        available: Vec<PackEntry>,
        selected: Vec<PackEntry>,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let open_btn = Button::builder(Component::literal(&crate::translate("pack.openFolder")))
            .bounds(0, 0, FOOTER_BUTTON_WIDTH, BUTTON_H)
            .with_action(ResourcePacksAction::OpenFolder)
            .with_font(Arc::clone(&font))
            .build();
        let done_btn = Button::builder(Component::literal(&crate::translate("gui.done")))
            .bounds(0, 0, FOOTER_BUTTON_WIDTH, BUTTON_H)
            .with_action(ResourcePacksAction::Done)
            .with_font(Arc::clone(&font))
            .build();
        Self {
            base: ScreenBase::new(Component::literal(&crate::translate("resourcePack.title"))),
            open_folder_button: Rc::new(RefCell::new(open_btn)),
            done_button: Rc::new(RefCell::new(done_btn)),
            font,
            in_world,
            available,
            selected,
            search: String::new(),
            scroll_available: 0.0,
            scroll_selected: 0.0,
            dragging_scrollbar: None,
            cursor_ticks: 0,
            pending_action: None,
            last_width: 0,
            last_height: 0,
        }
    }
    pub fn take_action(&mut self) -> Option<ResourcePacksAction> {
        if let Some(action) = self.open_folder_button.borrow_mut().take_press_action() {
            return Some(action);
        }
        if let Some(action) = self.done_button.borrow_mut().take_press_action() {
            return Some(action);
        }
        self.pending_action.take()
    }
    fn ensure_layout(&mut self, width: i32, height: i32) {
        self.base.width = width;
        self.base.height = height;
        if self.last_width == width && self.last_height == height {
            self.done_button.borrow_mut().base.active = !self.selected.is_empty();
            return;
        }
        self.last_width = width;
        self.last_height = height;
        self.base.clear_widgets();
        let open_x = width / 2 - 4 - FOOTER_BUTTON_WIDTH;
        let done_x = width / 2 + 4;
        let btn_y = height - FOOTER_HEIGHT + 6;
        {
            let mut o = self.open_folder_button.borrow_mut();
            o.set_position(open_x, btn_y);
            o.base.set_width(FOOTER_BUTTON_WIDTH);
            o.base.active = true;
        }
        {
            let mut d = self.done_button.borrow_mut();
            d.set_position(done_x, btn_y);
            d.base.set_width(FOOTER_BUTTON_WIDTH);
            d.base.active = !self.selected.is_empty();
        }
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.open_folder_button)));
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.done_button)));
    }
    pub fn tick(&mut self) {
        self.cursor_ticks = self.cursor_ticks.wrapping_add(1);
    }
    pub fn key_char(&mut self, ch: char) {
        self.search.push(ch);
        self.cursor_ticks = 0;
        self.scroll_available = 0.0;
        self.scroll_selected = 0.0;
    }
    pub fn key_backspace(&mut self) {
        self.search.pop();
        self.cursor_ticks = 0;
        self.scroll_available = 0.0;
        self.scroll_selected = 0.0;
    }
    pub fn key_enter(&self) -> Option<ResourcePacksAction> {
        (!self.selected.is_empty()).then_some(ResourcePacksAction::Done)
    }
    pub fn select_pack_by_id(&mut self, id: &str) {
        if let Some(pos) = self.available.iter().position(|pack| pack.id == id) {
            let entry = self.available.remove(pos);
            self.selected.insert(0, entry);
            self.done_button.borrow_mut().base.active = !self.selected.is_empty();
        }
    }
    pub fn mouse_released(&mut self) {
        self.dragging_scrollbar = None;
    }
    pub fn scroll_available(&mut self, delta: f32, screen_h: i32) {
        let n = self.filtered_count(&self.available.clone());
        let max = self.max_scroll(n, screen_h);
        self.scroll_available = (self.scroll_available - delta * SCROLL_SPEED).clamp(0.0, max);
    }
    pub fn scroll_selected(&mut self, delta: f32, screen_h: i32) {
        let n = self.filtered_count(&self.selected.clone());
        let max = self.max_scroll(n, screen_h);
        self.scroll_selected = (self.scroll_selected - delta * SCROLL_SPEED).clamp(0.0, max);
    }
    pub fn available_list_x(&self, screen_w: i32) -> i32 {
        screen_w / 2 - 15 - LIST_WIDTH
    }
    pub fn selected_list_x(&self, screen_w: i32) -> i32 {
        screen_w / 2 + 15
    }
    pub fn available_list_x_pub(&self, screen_w: i32) -> i32 {
        self.available_list_x(screen_w)
    }
    pub fn selected_list_x_pub(&self, screen_w: i32) -> i32 {
        self.selected_list_x(screen_w)
    }
    fn list_h(&self, screen_h: i32) -> i32 {
        (screen_h - HEADER_HEIGHT - FOOTER_HEIGHT).max(0)
    }
    fn filtered_count(&self, list: &[PackEntry]) -> usize {
        if self.search.is_empty() {
            return list.len();
        }
        let q = self.search.to_lowercase();
        list.iter()
            .filter(|p| {
                p.id.to_lowercase().contains(&q)
                    || p.title.to_lowercase().contains(&q)
                    || p.description.to_lowercase().contains(&q)
            })
            .count()
    }
    fn max_scroll(&self, count: usize, screen_h: i32) -> f32 {
        ((LIST_HEADER_HEIGHT + count as i32 * ENTRY_HEIGHT + 4 - self.list_h(screen_h)).max(0))
            as f32
    }
    pub fn mouse_clicked(
        &mut self,
        mouse_x: i32,
        mouse_y: i32,
        screen_w: i32,
        screen_h: i32,
    ) -> Option<ResourcePacksAction> {
        let event =
            MouseButtonEvent::new(mouse_x as f64, mouse_y as f64, MouseButtonInfo::new(0, 0));
        self.ensure_layout(screen_w, screen_h);
        let handled = Screen::mouse_clicked(self, event, false);
        if handled {
            if let Some(action) = self.take_action() {
                match action {
                    ResourcePacksAction::Done => {
                        crate::components::click_sound_sink::record_button_click();
                        return Some(ResourcePacksAction::Done);
                    }
                    ResourcePacksAction::OpenFolder => {
                        crate::components::click_sound_sink::record_button_click();
                        return Some(ResourcePacksAction::OpenFolder);
                    }
                    ResourcePacksAction::ConfirmIncompatible(pack_id) => {
                        crate::components::click_sound_sink::record_button_click();
                        return Some(ResourcePacksAction::ConfirmIncompatible(pack_id));
                    }
                }
            }
        }
        None
    }
    pub fn render_screen(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
    ) {
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        let mouse_x = graphics.mouse_x();
        let mouse_y = graphics.mouse_y();
        self.ensure_layout(width, height);
        self.base
            .extract_background_backdrop(graphics, self.in_world);
        graphics.draw_menu_background(self.in_world, width, HEADER_HEIGHT);
        graphics.pose_mut().push_pose();
        graphics
            .pose_mut()
            .translate(0.0, (height - FOOTER_HEIGHT) as f32);
        graphics.draw_menu_background(self.in_world, width, FOOTER_HEIGHT);
        graphics.pose_mut().pop_pose();
        let title = crate::translate("resourcePack.title");
        let title_w = font.measure_width(&title, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            title,
            (width - title_w) / 2,
            4,
            0xFFFF_FFFFu32 as i32,
        );
        let subtitle = crate::translate("pack.dropInfo");
        let subtitle_w = font.measure_width(&subtitle, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            subtitle,
            (width - subtitle_w) / 2,
            4 + 9 + 4,
            0xFFA0_A0A0u32 as i32,
        );
        let search_y = 4 + 9 + 4 + 9 + 4;
        let search_x = width / 2 - 100;
        graphics.draw_edit_box(
            font,
            search_x,
            search_y,
            200,
            15,
            &self.search,
            Some(&crate::translate("gui.packSelection.search")),
            true,
            (self.cursor_ticks / 6).is_multiple_of(2),
            self.search.chars().count(),
        );
        let list_top = HEADER_HEIGHT;
        let list_h = self.list_h(height);
        let list_bottom = list_top + list_h;
        let ax = self.available_list_x(width);
        let sx = self.selected_list_x(width);
        self.draw_list_background(graphics, ax, list_top, list_h, self.scroll_available);
        graphics.enable_scissor(ax, list_top, ax + LIST_WIDTH, list_bottom);
        self.draw_pack_list(
            graphics,
            font,
            mouse_x,
            mouse_y,
            ax,
            list_top,
            list_h,
            &self.available.clone(),
            self.scroll_available,
            false,
        );
        graphics.disable_scissor();
        self.draw_scrollbar(
            graphics,
            ax,
            list_top,
            list_h,
            self.scroll_available,
            self.max_scroll(self.filtered_count(&self.available), height),
        );
        self.draw_list_separators(graphics, ax, list_top, list_bottom);
        self.draw_list_background(graphics, sx, list_top, list_h, self.scroll_selected);
        graphics.enable_scissor(sx, list_top, sx + LIST_WIDTH, list_bottom);
        self.draw_pack_list(
            graphics,
            font,
            mouse_x,
            mouse_y,
            sx,
            list_top,
            list_h,
            &self.selected.clone(),
            self.scroll_selected,
            true,
        );
        graphics.disable_scissor();
        self.draw_scrollbar(
            graphics,
            sx,
            list_top,
            list_h,
            self.scroll_selected,
            self.max_scroll(self.filtered_count(&self.selected), height),
        );
        self.draw_list_separators(graphics, sx, list_top, list_bottom);
        let open_x = screen_w_footer(width);
        let done_x = width / 2 + 4;
        let btn_y = height - FOOTER_HEIGHT + 6;
        let done_active = !self.selected.is_empty();
        let open_focused = self.open_folder_button.borrow().is_focused();
        let done_focused = self.done_button.borrow().is_focused();
        self.draw_button(
            graphics,
            font,
            "pack.openFolder",
            open_x,
            btn_y,
            150,
            true,
            open_focused,
        );
        self.draw_button(
            graphics,
            font,
            "gui.done",
            done_x,
            btn_y,
            150,
            done_active,
            done_focused,
        );
    }
    #[allow(clippy::too_many_arguments)]
    fn draw_pack_list(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
        mouse_x: i32,
        mouse_y: i32,
        list_x: i32,
        list_top: i32,
        list_h: i32,
        entries: &[PackEntry],
        scroll: f32,
        is_selected_list: bool,
    ) {
        let q = self.search.to_lowercase();
        let filtered: Vec<&PackEntry> = if q.is_empty() {
            entries.iter().collect()
        } else {
            entries
                .iter()
                .filter(|p| {
                    p.id.to_lowercase().contains(&q)
                        || p.title.to_lowercase().contains(&q)
                        || p.description.to_lowercase().contains(&q)
                })
                .collect()
        };
        let title_key = if is_selected_list {
            "pack.selected.title"
        } else {
            "pack.available.title"
        };
        let title = crate::translate(title_key);
        let title_w = font.measure_width(&title, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            title,
            list_x + (LIST_WIDTH - title_w) / 2,
            list_top + LIST_TOP_PADDING + 2,
            0xFFFF_FFFFu32 as i32,
        );
        for (i, pack) in filtered.iter().enumerate() {
            let entry_y =
                list_top + LIST_TOP_PADDING + LIST_HEADER_HEIGHT + (i as i32 * ENTRY_HEIGHT)
                    - scroll as i32;
            if entry_y + ENTRY_HEIGHT < list_top || entry_y > list_top + list_h {
                continue;
            }
            let row_x = list_x + 2;
            let row_width = LIST_WIDTH - 4;
            let content_x = row_x + 2;
            let scrollable = LIST_HEADER_HEIGHT + 4 + filtered.len() as i32 * ENTRY_HEIGHT > list_h;
            let content_width = row_width - if scrollable { 6 } else { 0 } - 4;
            let hovered = mouse_x >= row_x
                && mouse_x < row_x + row_width
                && mouse_y >= entry_y
                && mouse_y < entry_y + ENTRY_HEIGHT;
            if !pack.compatibility.is_compatible() {
                graphics.fill(
                    content_x - 1,
                    entry_y - 1,
                    content_x + content_width + 1,
                    entry_y + ENTRY_HEIGHT + 1,
                    0xFF78_0000u32 as i32,
                );
            }
            let icon = if pack.icon_path.is_empty() {
                ResourceLocation::from_string("minecraft:misc/unknown_pack")
            } else {
                ResourceLocation::from_string(&pack.icon_path)
            };
            graphics.blit(icon, content_x, entry_y + 2, ICON_SIZE, ICON_SIZE);
            let show_incompatible_warning = hovered && !pack.compatibility.is_compatible();
            let title_text = if show_incompatible_warning {
                crate::translate("pack.incompatible").to_string()
            } else {
                pack.title.clone()
            };
            let description_text = if show_incompatible_warning {
                crate::translate(pack.compatibility.description_key()).to_string()
            } else {
                pack.extended_description()
            };
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                title_text,
                content_x + ICON_SIZE + 2,
                entry_y + 1,
                0xFFFF_FFFFu32 as i32,
            );
            let desc = description_text;
            let max_w = (157 - if scrollable { 6 } else { 0 }) as f32;
            let desc_lines = crate::wrap_text(&desc, max_w, font);
            for (li, line) in desc_lines.iter().take(2).enumerate() {
                graphics.text(
                    ResourceLocation::from_string("minecraft:default"),
                    line.clone(),
                    content_x + ICON_SIZE + 2,
                    entry_y + 12 + li as i32 * 9,
                    0xFF80_8080u32 as i32,
                );
            }
            if hovered && (!pack.fixed_position || !pack.required) {
                graphics.fill(
                    content_x,
                    entry_y + 2,
                    content_x + ICON_SIZE,
                    entry_y + 2 + ICON_SIZE,
                    0xA032_3232u32 as i32,
                );
                let rel_x = mouse_x - content_x;
                let rel_y = mouse_y - (entry_y + 2);
                if !is_selected_list && pack.can_select() {
                    let over_icon =
                        rel_x >= 0 && rel_x < ICON_SIZE && rel_y >= 0 && rel_y < ICON_SIZE;
                    let sprite = if over_icon {
                        "minecraft:transferable_list/select_highlighted"
                    } else {
                        "minecraft:transferable_list/select"
                    };
                    graphics.blit(
                        ResourceLocation::from_string(sprite),
                        content_x,
                        entry_y + 2,
                        ICON_SIZE,
                        ICON_SIZE,
                    );
                    graphics.request_cursor(CursorType::PointingHand);
                } else if is_selected_list && pack.can_unselect() {
                    let in_left = rel_x < ICON_SIZE / 2 && rel_y >= 0 && rel_y < ICON_SIZE;
                    let sprite = if in_left {
                        "minecraft:transferable_list/unselect_highlighted"
                    } else {
                        "minecraft:transferable_list/unselect"
                    };
                    graphics.blit(
                        ResourceLocation::from_string(sprite),
                        content_x,
                        entry_y + 2,
                        ICON_SIZE,
                        ICON_SIZE,
                    );
                    let pack_pos = entries.iter().position(|p| p.id == pack.id).unwrap_or(0);
                    let can_move_up = pack_pos > 0
                        && !entries
                            .get(pack_pos.saturating_sub(1))
                            .is_some_and(|neighbor| neighbor.fixed_position);
                    if can_move_up {
                        let in_top_right =
                            rel_x >= ICON_SIZE / 2 && rel_y >= 0 && rel_y < ICON_SIZE / 2;
                        let sprite = if in_top_right {
                            "minecraft:transferable_list/move_up_highlighted"
                        } else {
                            "minecraft:transferable_list/move_up"
                        };
                        graphics.blit(
                            ResourceLocation::from_string(sprite),
                            content_x,
                            entry_y + 2,
                            ICON_SIZE,
                            ICON_SIZE,
                        );
                    }
                    let can_move_down = pack_pos + 1 < entries.len()
                        && !entries
                            .get(pack_pos + 1)
                            .is_some_and(|neighbor| neighbor.fixed_position);
                    if can_move_down {
                        let in_bot_right =
                            rel_x >= ICON_SIZE / 2 && rel_y >= ICON_SIZE / 2 && rel_y < ICON_SIZE;
                        let sprite = if in_bot_right {
                            "minecraft:transferable_list/move_down_highlighted"
                        } else {
                            "minecraft:transferable_list/move_down"
                        };
                        graphics.blit(
                            ResourceLocation::from_string(sprite),
                            content_x,
                            entry_y + 2,
                            ICON_SIZE,
                            ICON_SIZE,
                        );
                    }
                    graphics.request_cursor(CursorType::PointingHand);
                }
            }
        }
    }
    fn draw_list_background(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        x: i32,
        y: i32,
        height: i32,
        scroll: f32,
    ) {
        let texture = ResourceLocation::from_string(if self.in_world {
            "minecraft:gui/inworld_menu_list_background"
        } else {
            "minecraft:gui/menu_list_background"
        });
        let mut yy = y;
        while yy < y + height {
            let tile_h = 32.min(y + height - yy);
            graphics.blit_region(
                texture.clone(),
                x,
                yy,
                LIST_WIDTH,
                tile_h,
                0.0,
                (yy as f32 + scroll) / 32.0,
                LIST_WIDTH as f32 / 32.0,
                (yy as f32 + scroll + tile_h as f32) / 32.0,
                -1,
            );
            yy += tile_h;
        }
    }
    fn draw_list_separators(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        x: i32,
        top: i32,
        bottom: i32,
    ) {
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/header_separator"),
            x,
            top - 2,
            LIST_WIDTH,
            2,
        );
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/footer_separator"),
            x,
            bottom,
            LIST_WIDTH,
            2,
        );
    }
    fn draw_scrollbar(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        list_x: i32,
        list_top: i32,
        list_h: i32,
        scroll: f32,
        max_scroll: f32,
    ) {
        if max_scroll <= 0.0 {
            return;
        }
        let bar_h = if list_h <= 0 {
            0
        } else {
            let upper = list_h.max(1);
            let lower = 32.min(upper);
            ((list_h as f32 * list_h as f32 / (list_h as f32 + max_scroll)) as i32)
                .clamp(lower, upper)
        };
        let travel = list_h - bar_h;
        let bar_y = list_top
            + if max_scroll > 0.0 {
                (scroll.clamp(0.0, max_scroll) * travel as f32 / max_scroll).round() as i32
            } else {
                0
            };
        let x = list_x + LIST_WIDTH - 6;
        graphics.blit(
            ResourceLocation::from_string("minecraft:widget/scroller_background"),
            x,
            list_top,
            6,
            list_h,
        );
        graphics.blit(
            ResourceLocation::from_string("minecraft:widget/scroller"),
            x,
            bar_y,
            6,
            bar_h,
        );
    }
    fn draw_button(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
        label_key: &str,
        x: i32,
        y: i32,
        w: i32,
        active: bool,
        focused: bool,
    ) {
        graphics.draw_button_with_focus(
            font,
            x,
            y,
            w,
            BUTTON_H,
            &crate::translate(label_key),
            active,
            focused,
        );
        let mouse_over = graphics.mouse_x() >= x
            && graphics.mouse_x() < x + w
            && graphics.mouse_y() >= y
            && graphics.mouse_y() < y + BUTTON_H;
        if mouse_over && !active {
            graphics.request_cursor(CursorType::NotAllowed);
        }
    }
}
fn screen_w_footer(width: i32) -> i32 {
    width / 2 - 4 - 150
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourcePacksAction {
    Done,
    OpenFolder,
    ConfirmIncompatible(String),
}
impl Screen for ResourcePacksScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn should_close_on_esc(&self) -> bool {
        true
    }
    fn init(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        self.last_width = 0;
        self.last_height = 0;
        self.ensure_layout(width, height);
    }
    fn reposition_elements(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        self.ensure_layout(width, height);
    }
    fn tick(&mut self) {
        self.cursor_ticks = self.cursor_ticks.wrapping_add(1);
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if event.is_escape() {
            self.pending_action = Some(ResourcePacksAction::Done);
            return true;
        }
        let focused_idx = self.base_ref().focused_index();
        if let Some(index) = focused_idx {
            if index == 0 {
                if event.is_selection() {
                    click_sound_sink::record_button_click();
                    self.pending_action = Some(ResourcePacksAction::OpenFolder);
                    return true;
                }
            } else if index == 1 {
                if event.is_selection() && !self.selected.is_empty() {
                    click_sound_sink::record_button_click();
                    self.pending_action = Some(ResourcePacksAction::Done);
                    return true;
                }
            }
        }
        if event.key == KeyEvent::KEY_ENTER && !self.selected.is_empty() {
            click_sound_sink::record_button_click();
            self.pending_action = Some(ResourcePacksAction::Done);
            return true;
        }
        if event.key == KeyEvent::KEY_BACKSPACE {
            self.search.pop();
            self.cursor_ticks = 0;
            self.scroll_available = 0.0;
            self.scroll_selected = 0.0;
            return true;
        }
        let handled = crate::framework::screen::screen_key_pressed(self, event);
        if handled || (event.key == KeyEvent::KEY_TAB && self.base_ref().focused_index().is_some())
        {
            return true;
        }
        false
    }
    fn char_typed(&mut self, event: CharacterEvent) -> bool {
        let ch = event.codepoint;
        if !ch.is_control() {
            self.search.push(ch);
            self.cursor_ticks = 0;
            self.scroll_available = 0.0;
            self.scroll_selected = 0.0;
            return true;
        }
        false
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        let screen_w = self.base_ref().width;
        let screen_h = self.base_ref().height;
        if screen_w > 0 && screen_h > 0 {
            self.ensure_layout(screen_w, screen_h);
        }
        let mouse_x = event.x as i32;
        let mouse_y = event.y as i32;
        let list_top = HEADER_HEIGHT;
        let list_h = self.list_h(screen_h);
        let list_bottom = list_top + list_h;
        for (selected_list, list_x, count) in [
            (
                false,
                self.available_list_x(screen_w),
                self.filtered_count(&self.available),
            ),
            (
                true,
                self.selected_list_x(screen_w),
                self.filtered_count(&self.selected),
            ),
        ] {
            let max = self.max_scroll(count, screen_h);
            if max > 0.0
                && mouse_x >= list_x + LIST_WIDTH - 6
                && mouse_x < list_x + LIST_WIDTH
                && mouse_y >= list_top
                && mouse_y < list_bottom
            {
                self.dragging_scrollbar = Some(selected_list);
                return true;
            }
        }
        let ax = self.available_list_x(screen_w);
        if mouse_x >= ax
            && mouse_x < ax + LIST_WIDTH
            && mouse_y >= list_top
            && mouse_y < list_bottom
        {
            let q = self.search.to_lowercase();
            let filtered_ids: Vec<String> = self
                .available
                .iter()
                .filter(|p| {
                    q.is_empty()
                        || p.id.to_lowercase().contains(&q)
                        || p.title.to_lowercase().contains(&q)
                        || p.description.to_lowercase().contains(&q)
                })
                .map(|p| p.id.clone())
                .collect();
            let rel_y = mouse_y - list_top - LIST_TOP_PADDING + self.scroll_available as i32;
            let idx = (rel_y - LIST_HEADER_HEIGHT) / ENTRY_HEIGHT;
            if idx >= 0 && (idx as usize) < filtered_ids.len() {
                let rel_x = mouse_x - (ax + 4);
                let rel_y_in = rel_y - LIST_HEADER_HEIGHT - idx * ENTRY_HEIGHT - 2;
                if rel_x < ICON_SIZE && rel_y_in >= 0 && rel_y_in < ICON_SIZE {
                    let pack_id = filtered_ids[idx as usize].clone();
                    if let Some(entry) = self.available.iter().find(|p| p.id == pack_id) {
                        if !entry.compatibility.is_compatible() {
                            click_sound_sink::record_button_click();
                            self.pending_action =
                                Some(ResourcePacksAction::ConfirmIncompatible(pack_id));
                            return true;
                        }
                    }
                    click_sound_sink::record_button_click();
                    self.select_pack_by_id(&pack_id);
                    let n = self.filtered_count(&self.available.clone());
                    let max = self.max_scroll(n, screen_h);
                    self.scroll_available = self.scroll_available.clamp(0.0, max);
                    self.ensure_layout(screen_w, screen_h);
                    return true;
                }
            }
        }
        let sx = self.selected_list_x(screen_w);
        if mouse_x >= sx
            && mouse_x < sx + LIST_WIDTH
            && mouse_y >= list_top
            && mouse_y < list_bottom
        {
            let q = self.search.to_lowercase();
            let filtered_ids: Vec<String> = self
                .selected
                .iter()
                .filter(|p| {
                    q.is_empty()
                        || p.id.to_lowercase().contains(&q)
                        || p.title.to_lowercase().contains(&q)
                        || p.description.to_lowercase().contains(&q)
                })
                .map(|p| p.id.clone())
                .collect();
            let rel_y = mouse_y - list_top - LIST_TOP_PADDING + self.scroll_selected as i32;
            let idx = (rel_y - LIST_HEADER_HEIGHT) / ENTRY_HEIGHT;
            if idx >= 0 && (idx as usize) < filtered_ids.len() {
                let rel_x = mouse_x - (sx + 4);
                let rel_y_in = rel_y - LIST_HEADER_HEIGHT - idx * ENTRY_HEIGHT - 2;
                let pack_id = filtered_ids[idx as usize].clone();
                if let Some(pos) = self.selected.iter().position(|p| p.id == pack_id) {
                    let entry = &self.selected[pos];
                    if entry.can_unselect()
                        && rel_x < ICON_SIZE / 2
                        && rel_y_in >= 0
                        && rel_y_in < ICON_SIZE
                    {
                        click_sound_sink::record_button_click();
                        let e = self.selected.remove(pos);
                        self.available.push(e);
                        let n = self.filtered_count(&self.selected.clone());
                        let max = self.max_scroll(n, screen_h);
                        self.scroll_selected = self.scroll_selected.clamp(0.0, max);
                        self.ensure_layout(screen_w, screen_h);
                        return true;
                    } else if entry.can_unselect()
                        && pos > 0
                        && !self.selected[pos - 1].fixed_position
                        && rel_x >= ICON_SIZE / 2
                        && rel_x < ICON_SIZE
                        && rel_y_in >= 0
                        && rel_y_in < ICON_SIZE / 2
                    {
                        click_sound_sink::record_button_click();
                        self.selected.swap(pos, pos - 1);
                        return true;
                    } else if entry.can_unselect()
                        && pos + 1 < self.selected.len()
                        && !self.selected[pos + 1].fixed_position
                        && rel_x >= ICON_SIZE / 2
                        && rel_x < ICON_SIZE
                        && rel_y_in >= 0
                        && rel_y_in >= ICON_SIZE / 2
                    {
                        click_sound_sink::record_button_click();
                        self.selected.swap(pos, pos + 1);
                        return true;
                    }
                }
            }
        }
        let handled = crate::framework::screen::screen_mouse_clicked(self, event, _double_click);
        if !handled {
            return false;
        }
        if let Some(action) = self.open_folder_button.borrow_mut().take_press_action() {
            click_sound_sink::record_button_click();
            self.pending_action = Some(action);
            return true;
        }
        if let Some(action) = self.done_button.borrow_mut().take_press_action() {
            click_sound_sink::record_button_click();
            self.pending_action = Some(action);
            return true;
        }
        true
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        self.dragging_scrollbar = None;
        crate::framework::screen::screen_mouse_released(self, event)
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, _dx: f64, _dy: f64) -> bool {
        let screen_w = self.base_ref().width;
        let screen_h = self.base_ref().height;
        let mouse_x = event.x as i32;
        let mouse_y = event.y as i32;
        let Some(selected_list) = self.dragging_scrollbar else {
            return false;
        };
        let list_x = if selected_list {
            self.selected_list_x(screen_w)
        } else {
            self.available_list_x(screen_w)
        };
        let list_top = HEADER_HEIGHT;
        let list_h = self.list_h(screen_h);
        let count = if selected_list {
            self.filtered_count(&self.selected)
        } else {
            self.filtered_count(&self.available)
        };
        let max = self.max_scroll(count, screen_h);
        if max <= 0.0 || mouse_x < list_x + LIST_WIDTH - 8 || mouse_x > list_x + LIST_WIDTH {
            return true;
        }
        let bar_h = if list_h <= 0 {
            0
        } else {
            let upper = list_h.max(1);
            let lower = 32.min(upper);
            ((list_h as f32 * list_h as f32 / (list_h as f32 + max)) as i32).clamp(lower, upper)
        };
        let travel = (list_h - bar_h).max(1);
        let value = ((mouse_y - list_top - bar_h / 2) as f32 / travel as f32 * max).clamp(0.0, max);
        if selected_list {
            self.scroll_selected = value;
        } else {
            self.scroll_available = value;
        }
        true
    }
    fn mouse_scrolled(
        &mut self,
        mouse_x: f64,
        _mouse_y: f64,
        _scroll_x: f64,
        scroll_y: f64,
    ) -> bool {
        let screen_w = self.base_ref().width;
        let screen_h = self.base_ref().height;
        let mx = mouse_x as i32;
        let ax = self.available_list_x(screen_w);
        let sx = self.selected_list_x(screen_w);
        if mx >= ax && mx < ax + LIST_WIDTH {
            self.scroll_available(scroll_y as f32, screen_h);
            true
        } else if mx >= sx && mx < sx + LIST_WIDTH {
            self.scroll_selected(scroll_y as f32, screen_h);
            true
        } else {
            false
        }
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_ticks: f32,
    ) {
        let font = Arc::clone(&self.font);
        self.render_screen(graphics, font.as_ref());
    }
}
