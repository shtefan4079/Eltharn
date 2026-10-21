use crate::FontWidthMeasurer;
use crate::components::click_sound_sink;
use crate::components::{
    Button, DEFAULT_HEIGHT, GuiEventListener, Renderable, TabOrderedElement, WidgetClickSound,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{NarratableEntry, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{KeyEvent, MouseButtonEvent, MouseButtonInfo};
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
pub const HEADER_HEIGHT: i32 = 33;
pub const FOOTER_HEIGHT: i32 = 33;
pub const LIST_TOP_INSET: i32 = 2;
pub const ITEM_HEIGHT: i32 = 20;
pub const ROW_WIDTH: i32 = 340;
pub const SCROLLBAR_WIDTH: i32 = 6;
pub const SCROLLBAR_GAP: i32 = 2;
pub const BUTTON_GAP: i32 = 5;
pub const RESET_GAP: i32 = 10;
pub const FOOTER_BUTTON_WIDTH: i32 = 150;
pub const FOOTER_BUTTON_GAP: i32 = 8;
pub const FOOTER_BUTTON_Y_OFFSET: i32 = 26;
pub const CHANGE_BUTTON_WIDTH: i32 = 75;
pub const RESET_BUTTON_WIDTH: i32 = 50;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyBindEntry {
    pub id: usize,
    pub name_key: String,
    pub name_label: Option<String>,
    pub category_key: String,
    pub category_label: Option<String>,
    pub category_order: Option<usize>,
    pub order: i32,
    pub binding_name: String,
    pub is_default: bool,
    pub is_duplicate: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyBindsAction {
    Done,
    ResetAll,
    SelectKey { id: usize },
    ResetKey { id: usize },
}
#[derive(Clone, Debug)]
pub enum KeyBindsListItem {
    CategoryHeader { category_key: String },
    KeyRow { entry: KeyBindEntry },
}
#[derive(Clone, Debug)]
pub struct KeyBindsLayoutItem {
    pub kind: KeyBindsListItem,
    pub relative_y: i32,
    pub height: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyRowButtonKind {
    Change,
    Reset,
}
struct RowButtonWidget {
    rect: ScreenRectangle,
    entry_id: usize,
    kind: KeyRowButtonKind,
    active: bool,
    focused: bool,
    pressed: bool,
}
impl RowButtonWidget {
    fn new(rect: ScreenRectangle, entry_id: usize, kind: KeyRowButtonKind, active: bool) -> Self {
        Self {
            rect,
            entry_id,
            kind,
            active,
            focused: false,
            pressed: false,
        }
    }
    fn takes_press(&mut self) -> bool {
        let pressed = self.pressed;
        self.pressed = false;
        pressed
    }
}
#[derive(Clone)]
struct RowButtonHandle {
    button: Rc<RefCell<RowButtonWidget>>,
}
impl RowButtonHandle {
    fn new(button: Rc<RefCell<RowButtonWidget>>) -> Self {
        Self { button }
    }
    fn with_button<R>(&self, f: impl FnOnce(&RowButtonWidget) -> R, fallback: R) -> R {
        self.button.try_borrow().map(|b| f(&b)).unwrap_or(fallback)
    }
}
impl Renderable for RowButtonHandle {
    fn extract_render_state(
        &mut self,
        _graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
    }
}
impl TabOrderedElement for RowButtonHandle {
    fn tab_order_group(&self) -> i32 {
        0
    }
}
impl NarratableEntry for RowButtonHandle {
    fn narration_priority(&self) -> NarrationPriority {
        NarrationPriority::None
    }
    fn update_narration(&self, _output: &mut NarrationElementOutput) {}
}
impl GuiEventListener for RowButtonHandle {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        if event.button() != 0 {
            return false;
        }
        let Ok(mut btn) = self.button.try_borrow_mut() else {
            return false;
        };
        if btn.active && btn.rect.contains_point(event.x as i32, event.y as i32) {
            btn.pressed = true;
            return true;
        }
        false
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.with_button(
            |b| b.rect.contains_point(mouse_x as i32, mouse_y as i32),
            false,
        )
    }
    fn set_focused(&mut self, focused: bool) {
        if let Ok(mut btn) = self.button.try_borrow_mut() {
            btn.focused = focused;
        }
    }
    fn is_focused(&self) -> bool {
        self.with_button(|b| b.focused, false)
    }
    fn rectangle(&self) -> ScreenRectangle {
        self.with_button(|b| b.rect, ScreenRectangle::empty())
    }
    fn next_focus_path(&self, _event: &FocusNavigationEvent) -> Option<ComponentPath> {
        Some(ComponentPath::leaf(0))
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.with_button(
            |b| {
                if b.active {
                    Some(WidgetClickSound::UiButtonClick)
                } else {
                    None
                }
            },
            Some(WidgetClickSound::UiButtonClick),
        )
    }
}
impl LayoutElement for RowButtonHandle {
    fn set_x(&mut self, x: i32) {
        if let Ok(mut b) = self.button.try_borrow_mut() {
            b.rect.position.x = x;
        }
    }
    fn set_y(&mut self, y: i32) {
        if let Ok(mut b) = self.button.try_borrow_mut() {
            b.rect.position.y = y;
        }
    }
    fn x(&self) -> i32 {
        self.with_button(|b| b.rect.position.x, 0)
    }
    fn y(&self) -> i32 {
        self.with_button(|b| b.rect.position.y, 0)
    }
    fn width(&self) -> i32 {
        self.with_button(|b| b.rect.width, 0)
    }
    fn height(&self) -> i32 {
        self.with_button(|b| b.rect.height, 0)
    }
}
struct FooterButtonHandle {
    button: Rc<RefCell<Button<KeyBindsAction>>>,
}
impl FooterButtonHandle {
    fn new(button: Rc<RefCell<Button<KeyBindsAction>>>) -> Self {
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
pub struct KeyBindsScreen {
    base: ScreenBase,
    reset_all_button: Rc<RefCell<Button<KeyBindsAction>>>,
    done_button: Rc<RefCell<Button<KeyBindsAction>>>,
    row_buttons: Vec<Rc<RefCell<RowButtonWidget>>>,
    pub entries: Vec<KeyBindEntry>,
    pub scroll_y: f32,
    pub dragging_scrollbar: bool,
    pub in_world: bool,
    pub last_width: i32,
    pub last_height: i32,
    font: Arc<dyn FontWidthMeasurer>,
    pending_action: Option<KeyBindsAction>,
}
fn category_sort_order(category_key: &str) -> usize {
    match category_key {
        "key.categories.movement" => 0,
        "key.categories.misc" => 1,
        "key.categories.multiplayer" => 2,
        "key.categories.gameplay" => 3,
        "key.categories.inventory" => 4,
        "key.categories.creative" => 5,
        "key.categories.spectator" => 6,
        "key.categories.debug" => 7,
        _ => usize::MAX,
    }
}
impl KeyBindsScreen {
    pub fn new(
        in_world: bool,
        entries: Vec<KeyBindEntry>,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let reset_all_btn =
            Button::builder(Component::literal(&crate::translate("controls.resetAll")))
                .bounds(0, 0, FOOTER_BUTTON_WIDTH, DEFAULT_HEIGHT)
                .with_action(KeyBindsAction::ResetAll)
                .with_font(Arc::clone(&font))
                .build();
        let done_btn = Button::builder(Component::literal(&crate::translate("gui.done")))
            .bounds(0, 0, FOOTER_BUTTON_WIDTH, DEFAULT_HEIGHT)
            .with_action(KeyBindsAction::Done)
            .with_font(Arc::clone(&font))
            .build();
        Self {
            base: ScreenBase::new(Component::literal(&crate::translate(
                "controls.keybinds.title",
            ))),
            reset_all_button: Rc::new(RefCell::new(reset_all_btn)),
            done_button: Rc::new(RefCell::new(done_btn)),
            row_buttons: Vec::new(),
            entries,
            scroll_y: 0.0,
            dragging_scrollbar: false,
            in_world,
            last_width: 0,
            last_height: 0,
            font,
            pending_action: None,
        }
    }
    pub fn take_action(&mut self) -> Option<KeyBindsAction> {
        self.pending_action.take()
    }
    pub fn replace_entries(&mut self, entries: Vec<KeyBindEntry>) {
        if self.entries == entries {
            return;
        }
        let focused_index = self.base.focused_index();
        let focused_action = focused_index
            .and_then(|index| index.checked_sub(2))
            .and_then(|index| self.row_buttons.get(index))
            .and_then(|button| {
                let button = button.borrow();
                self.entries
                    .iter()
                    .find(|entry| entry.id == button.entry_id)
                    .map(|entry| (entry.name_key.clone(), button.kind))
            });
        self.entries = entries;
        self.pending_action = None;
        self.dragging_scrollbar = false;
        self.row_buttons.clear();
        self.last_width = 0;
        self.last_height = 0;
        let width = self.base.width;
        let height = self.base.height;
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_amount(height));
        if width > 0 && height > 0 {
            self.ensure_layout(width, height);
            let next_focus = if let Some((name, kind)) = focused_action {
                self.row_buttons
                    .iter()
                    .position(|button| {
                        let button = button.borrow();
                        button.kind == kind
                            && self
                                .entries
                                .iter()
                                .any(|entry| entry.id == button.entry_id && entry.name_key == name)
                    })
                    .map(|index| index + 2)
            } else {
                focused_index.filter(|index| *index < 2)
            };
            self.base.set_focused(next_focus);
        }
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn row_rects(&self) -> Vec<ScreenRectangle> {
        self.row_buttons.iter().map(|b| b.borrow().rect).collect()
    }
    pub fn get_layout_items(&self) -> Vec<KeyBindsLayoutItem> {
        let mut items = Vec::new();
        let mut current_y = 0;
        let mut sorted_entries = self.entries.clone();
        sorted_entries.sort_by(|a, b| {
            a.category_order
                .unwrap_or_else(|| category_sort_order(&a.category_key))
                .cmp(
                    &b.category_order
                        .unwrap_or_else(|| category_sort_order(&b.category_key)),
                )
                .then_with(|| a.order.cmp(&b.order))
                .then_with(|| {
                    a.name_label
                        .clone()
                        .unwrap_or_else(|| crate::translate(&a.name_key))
                        .cmp(
                            &b.name_label
                                .clone()
                                .unwrap_or_else(|| crate::translate(&b.name_key)),
                        )
                })
                .then_with(|| a.name_key.cmp(&b.name_key))
                .then_with(|| a.id.cmp(&b.id))
        });
        let mut categories: Vec<String> = Vec::new();
        for entry in &sorted_entries {
            if !categories.contains(&entry.category_key) {
                categories.push(entry.category_key.clone());
            }
        }
        for cat in categories {
            items.push(KeyBindsLayoutItem {
                kind: KeyBindsListItem::CategoryHeader {
                    category_key: cat.clone(),
                },
                relative_y: current_y,
                height: ITEM_HEIGHT,
            });
            current_y += ITEM_HEIGHT;
            for entry in &sorted_entries {
                if entry.category_key == cat {
                    items.push(KeyBindsLayoutItem {
                        kind: KeyBindsListItem::KeyRow {
                            entry: entry.clone(),
                        },
                        relative_y: current_y,
                        height: ITEM_HEIGHT,
                    });
                    current_y += ITEM_HEIGHT;
                }
            }
        }
        items
    }
    pub fn max_scroll_amount(&self, height: i32) -> f32 {
        let items = self.get_layout_items();
        if let Some(last) = items.last() {
            let total_height = last.relative_y + last.height + 4;
            let visible_height = height - HEADER_HEIGHT - FOOTER_HEIGHT;
            if visible_height <= 0 {
                return 0.0;
            }
            (total_height - visible_height).max(0) as f32
        } else {
            0.0
        }
    }
    pub fn scrollbar_height(&self, height: i32) -> i32 {
        let scroll_area_h = height - HEADER_HEIGHT - FOOTER_HEIGHT;
        if scroll_area_h <= 0 {
            return 0;
        }
        let items = self.get_layout_items();
        let total_content_height = if let Some(last) = items.last() {
            last.relative_y + last.height + 4
        } else {
            1
        };
        let bar_h =
            ((scroll_area_h * scroll_area_h) as f32 / total_content_height as f32).round() as i32;
        let upper = (scroll_area_h - 8).max(1);
        bar_h.clamp(1, upper).min(scroll_area_h)
    }
    pub fn can_reset_all(&self) -> bool {
        self.entries.iter().any(|entry| !entry.is_default)
    }
    pub fn scrollbar_x(width: i32) -> i32 {
        width / 2 + ROW_WIDTH / 2 + SCROLLBAR_WIDTH + SCROLLBAR_GAP
    }
    pub fn reset_button_x(width: i32) -> i32 {
        Self::scrollbar_x(width) - RESET_BUTTON_WIDTH - RESET_GAP
    }
    pub fn change_button_x(width: i32) -> i32 {
        Self::reset_button_x(width) - BUTTON_GAP - CHANGE_BUTTON_WIDTH
    }
    pub fn mouse_clicked(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        width: i32,
        height: i32,
    ) -> Option<KeyBindsAction> {
        let event =
            MouseButtonEvent::new(mouse_x as f64, mouse_y as f64, MouseButtonInfo::new(0, 0));
        self.ensure_layout(width, height);
        let handled = Screen::mouse_clicked(self, event, false);
        if handled {
            if let Some(action) = self.take_action() {
                match action {
                    KeyBindsAction::Done => {
                        click_sound_sink::record_button_click();
                        return Some(KeyBindsAction::Done);
                    }
                    KeyBindsAction::ResetAll => {
                        click_sound_sink::record_button_click();
                        return Some(KeyBindsAction::ResetAll);
                    }
                    KeyBindsAction::SelectKey { id } => {
                        click_sound_sink::record_button_click();
                        return Some(KeyBindsAction::SelectKey { id });
                    }
                    KeyBindsAction::ResetKey { id } => {
                        click_sound_sink::record_button_click();
                        return Some(KeyBindsAction::ResetKey { id });
                    }
                }
            }
        }
        None
    }
    pub fn mouse_released(&mut self) {
        self.dragging_scrollbar = false;
    }
    fn ensure_layout(&mut self, width: i32, height: i32) {
        if self.last_width == width && self.last_height == height && !self.row_buttons.is_empty() {
            self.reset_all_button.borrow_mut().base.active = self.can_reset_all();
            for r in &self.row_buttons {
                if let Ok(mut btn) = r.try_borrow_mut() {
                    if btn.kind == KeyRowButtonKind::Reset {
                        if let Some(entry) = self.entries.iter().find(|e| e.id == btn.entry_id) {
                            btn.active = !entry.is_default;
                        }
                    }
                }
            }
            return;
        }
        self.last_width = width;
        self.last_height = height;
        self.base.clear_widgets();
        let reset_all_x = width / 2 - FOOTER_BUTTON_WIDTH - FOOTER_BUTTON_GAP / 2;
        let done_x = width / 2 + FOOTER_BUTTON_GAP / 2;
        let footer_y = height - FOOTER_BUTTON_Y_OFFSET;
        {
            let mut r = self.reset_all_button.borrow_mut();
            r.set_position(reset_all_x, footer_y);
            r.base.set_width(FOOTER_BUTTON_WIDTH);
            r.base.active = self.can_reset_all();
        }
        {
            let mut d = self.done_button.borrow_mut();
            d.set_position(done_x, footer_y);
            d.base.set_width(FOOTER_BUTTON_WIDTH);
            d.base.active = true;
        }
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.reset_all_button)));
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.done_button)));
        self.row_buttons.clear();
        let items = self.get_layout_items();
        for item in items {
            let item_y = HEADER_HEIGHT + LIST_TOP_INSET + item.relative_y - self.scroll_y as i32;
            if let KeyBindsListItem::KeyRow { entry } = item.kind {
                let change_btn_x = Self::change_button_x(width);
                let reset_btn_x = Self::reset_button_x(width);
                let change_rect =
                    ScreenRectangle::new(change_btn_x, item_y, CHANGE_BUTTON_WIDTH, ITEM_HEIGHT);
                let reset_rect =
                    ScreenRectangle::new(reset_btn_x, item_y, RESET_BUTTON_WIDTH, ITEM_HEIGHT);
                let change_btn = Rc::new(RefCell::new(RowButtonWidget::new(
                    change_rect,
                    entry.id,
                    KeyRowButtonKind::Change,
                    true,
                )));
                let reset_btn = Rc::new(RefCell::new(RowButtonWidget::new(
                    reset_rect,
                    entry.id,
                    KeyRowButtonKind::Reset,
                    !entry.is_default,
                )));
                self.row_buttons.push(Rc::clone(&change_btn));
                self.row_buttons.push(Rc::clone(&reset_btn));
            }
        }
        for btn in &self.row_buttons {
            self.base.add_widget(RowButtonHandle::new(Rc::clone(btn)));
        }
    }
    fn update_row_rects(&mut self) {
        let width = self.base_ref().width;
        let items = self.get_layout_items();
        let mut row_idx = 0;
        for item in items {
            if let KeyBindsListItem::KeyRow { entry } = item.kind {
                let item_y =
                    HEADER_HEIGHT + LIST_TOP_INSET + item.relative_y - self.scroll_y as i32;
                let change_btn_x = Self::change_button_x(width);
                let reset_btn_x = Self::reset_button_x(width);
                if let Some(btn) = self.row_buttons.get(row_idx) {
                    if let Ok(mut b) = btn.try_borrow_mut() {
                        b.rect = ScreenRectangle::new(
                            change_btn_x,
                            item_y,
                            CHANGE_BUTTON_WIDTH,
                            ITEM_HEIGHT,
                        );
                        b.entry_id = entry.id;
                    }
                }
                row_idx += 1;
                if let Some(btn) = self.row_buttons.get(row_idx) {
                    if let Ok(mut b) = btn.try_borrow_mut() {
                        b.rect = ScreenRectangle::new(
                            reset_btn_x,
                            item_y,
                            RESET_BUTTON_WIDTH,
                            ITEM_HEIGHT,
                        );
                        b.entry_id = entry.id;
                        b.active = !entry.is_default;
                    }
                }
                row_idx += 1;
            }
        }
    }
    fn render_screen(&mut self, graphics: &mut GuiGraphicsExtractor, font: &dyn FontWidthMeasurer) {
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        let mouse_x = graphics.mouse_x();
        let mouse_y = graphics.mouse_y();
        self.ensure_layout(width, height);
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_amount(height));
        self.base
            .extract_background_backdrop(graphics, self.in_world);
        let list_bg_tex = if self.in_world {
            "minecraft:gui/inworld_menu_list_background"
        } else {
            "minecraft:gui/menu_list_background"
        };
        let list_background = ResourceLocation::from_string(list_bg_tex);
        let mut y = HEADER_HEIGHT;
        while y < height - FOOTER_HEIGHT {
            let tile_h = 32.min(height - FOOTER_HEIGHT - y);
            let mut x = 0;
            while x < width {
                let tile_w = 32.min(width - x);
                graphics.blit_region(
                    list_background.clone(),
                    x,
                    y,
                    tile_w,
                    tile_h,
                    0.0,
                    (y as f32 + self.scroll_y) / 32.0,
                    tile_w as f32 / 32.0,
                    tile_h as f32 / 32.0,
                    0xFFFF_FFFFu32 as i32,
                );
                x += 32;
            }
            y += tile_h;
        }
        graphics.enable_scissor(0, HEADER_HEIGHT, width, height - FOOTER_HEIGHT);
        let layout_items = self.get_layout_items();
        for item in layout_items {
            let item_y = HEADER_HEIGHT + LIST_TOP_INSET + item.relative_y - self.scroll_y as i32;
            match item.kind {
                KeyBindsListItem::CategoryHeader { category_key } => {
                    let text = self
                        .entries
                        .iter()
                        .find(|entry| entry.category_key == category_key)
                        .and_then(|entry| entry.category_label.clone())
                        .unwrap_or_else(|| crate::translate(&category_key));
                    let text_w = font.measure_width(&text, 1.0).round() as i32;
                    graphics.text(
                        ResourceLocation::from_string("minecraft:default"),
                        text,
                        width / 2 - text_w / 2,
                        item_y + 5,
                        0xFFFF_FFFFu32 as i32,
                    );
                }
                KeyBindsListItem::KeyRow { entry } => {
                    let label = entry
                        .name_label
                        .clone()
                        .unwrap_or_else(|| crate::translate(&entry.name_key));
                    graphics.text(
                        ResourceLocation::from_string("minecraft:default"),
                        label,
                        width / 2 - ROW_WIDTH / 2 + 2,
                        item_y + 6,
                        0xFFFF_FFFFu32 as i32,
                    );
                    let key_btn_x = Self::change_button_x(width);
                    let key_hovered = mouse_x >= key_btn_x
                        && mouse_x < key_btn_x + CHANGE_BUTTON_WIDTH
                        && mouse_y >= item_y
                        && mouse_y < item_y + 20
                        && graphics.contains_point_in_scissor(mouse_x, mouse_y);
                    let key_sprite = if key_hovered {
                        "minecraft:widget/button_highlighted"
                    } else {
                        "minecraft:widget/button"
                    };
                    graphics.blit(
                        ResourceLocation::from_string(key_sprite),
                        key_btn_x,
                        item_y,
                        CHANGE_BUTTON_WIDTH,
                        20,
                    );
                    let is_selected =
                        entry.binding_name.starts_with("> ") && entry.binding_name.ends_with(" <");
                    let key_label = if is_selected {
                        entry.binding_name.clone()
                    } else if entry.is_duplicate {
                        format!("[ {} ]", entry.binding_name)
                    } else {
                        entry.binding_name.clone()
                    };
                    let key_text_color = if is_selected || entry.is_duplicate {
                        0xFFFF_FF55u32 as i32
                    } else {
                        0xFFFF_FFFFu32 as i32
                    };
                    graphics.draw_scrolling_text_with_default_center(
                        font,
                        &key_label,
                        key_btn_x + 2,
                        key_btn_x + CHANGE_BUTTON_WIDTH - 2,
                        item_y,
                        item_y + ITEM_HEIGHT,
                        key_text_color,
                    );
                    let reset_btn_x = Self::reset_button_x(width);
                    let reset_text = crate::translate("controls.reset");
                    graphics.draw_button(
                        font,
                        reset_btn_x,
                        item_y,
                        RESET_BUTTON_WIDTH,
                        ITEM_HEIGHT,
                        &reset_text,
                        !entry.is_default,
                    );
                }
            }
        }
        graphics.disable_scissor();
        graphics.draw_menu_background(self.in_world, width, HEADER_HEIGHT);
        graphics.pose_mut().push_pose();
        graphics
            .pose_mut()
            .translate(0.0, (height - FOOTER_HEIGHT) as f32);
        graphics.draw_menu_background(self.in_world, width, FOOTER_HEIGHT);
        graphics.pose_mut().pop_pose();
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/header_separator"),
            0,
            31,
            width,
            2,
        );
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/footer_separator"),
            0,
            height - FOOTER_HEIGHT,
            width,
            2,
        );
        let title_text = crate::translate("controls.keybinds.title");
        let title_w = font.measure_width(&title_text, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            title_text,
            (width - title_w) / 2,
            12,
            0xFFFF_FFFFu32 as i32,
        );
        let max_scroll = self.max_scroll_amount(height);
        if max_scroll > 0.0 {
            let scrollbar_x = Self::scrollbar_x(width);
            let scroll_area_h = height - HEADER_HEIGHT - FOOTER_HEIGHT;
            let bar_h = self.scrollbar_height(height);
            let bar_y = HEADER_HEIGHT
                + (self.scroll_y * (scroll_area_h - bar_h) as f32 / max_scroll).round() as i32;
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller_background"),
                scrollbar_x,
                HEADER_HEIGHT,
                SCROLLBAR_WIDTH,
                scroll_area_h,
            );
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller"),
                scrollbar_x,
                bar_y,
                SCROLLBAR_WIDTH,
                bar_h,
            );
        }
        let reset_all_x = width / 2 - FOOTER_BUTTON_WIDTH - FOOTER_BUTTON_GAP / 2;
        let reset_all_y = height - FOOTER_BUTTON_Y_OFFSET;
        graphics.draw_button(
            font,
            reset_all_x,
            reset_all_y,
            FOOTER_BUTTON_WIDTH,
            20,
            &crate::translate("controls.resetAll"),
            self.can_reset_all(),
        );
        let done_x = width / 2 + FOOTER_BUTTON_GAP / 2;
        let done_y = height - FOOTER_BUTTON_Y_OFFSET;
        graphics.draw_button(
            font,
            done_x,
            done_y,
            FOOTER_BUTTON_WIDTH,
            20,
            &crate::translate("gui.done"),
            true,
        );
    }
}
impl Screen for KeyBindsScreen {
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
        self.row_buttons.clear();
        self.ensure_layout(width, height);
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if event.is_escape() {
            self.pending_action = Some(KeyBindsAction::Done);
            return true;
        }
        let focused_idx = self.base_ref().focused_index();
        if let Some(index) = focused_idx {
            if index == 0 {
                if event.is_selection() && self.can_reset_all() {
                    click_sound_sink::record_button_click();
                    self.pending_action = Some(KeyBindsAction::ResetAll);
                    return true;
                }
            } else if index == 1 {
                if event.is_selection() {
                    click_sound_sink::record_button_click();
                    self.pending_action = Some(KeyBindsAction::Done);
                    return true;
                }
            } else if let Some(btn) = self.row_buttons.get(index - 2) {
                let (entry_id, kind, active) = {
                    let b = btn.borrow();
                    (b.entry_id, b.kind, b.active)
                };
                if active && event.is_selection() {
                    click_sound_sink::record_button_click();
                    match kind {
                        KeyRowButtonKind::Change => {
                            self.pending_action = Some(KeyBindsAction::SelectKey { id: entry_id });
                        }
                        KeyRowButtonKind::Reset => {
                            self.pending_action = Some(KeyBindsAction::ResetKey { id: entry_id });
                        }
                    }
                    return true;
                }
            }
        }
        let handled = crate::framework::screen::screen_key_pressed(self, event);
        if handled || (event.key == KeyEvent::KEY_TAB && self.base_ref().focused_index().is_some())
        {
            return true;
        }
        false
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        let width = self.base_ref().width;
        let height = self.base_ref().height;
        let scrollbar_x = Self::scrollbar_x(width);
        let bar_h = self.scrollbar_height(height);
        let max_scroll = self.max_scroll_amount(height);
        let bar_y = if max_scroll == 0.0 {
            HEADER_HEIGHT
        } else {
            let scroll_area_h = height - (HEADER_HEIGHT + FOOTER_HEIGHT);
            HEADER_HEIGHT
                + (self.scroll_y * (scroll_area_h - bar_h) as f32 / max_scroll).round() as i32
        };
        let (mx, my) = (event.x as i32, event.y as i32);
        if event.button() == 0
            && mx >= scrollbar_x
            && mx < scrollbar_x + SCROLLER_WIDTH
            && my >= HEADER_HEIGHT
            && my < height - FOOTER_HEIGHT
        {
            self.dragging_scrollbar = true;
            if my < bar_y {
                self.scroll_y = (self.scroll_y - 22.0).max(0.0);
            } else if my > bar_y + bar_h {
                self.scroll_y = (self.scroll_y + 22.0).min(max_scroll);
            }
            self.update_row_rects();
            return true;
        }
        let handled = crate::framework::screen::screen_mouse_clicked(self, event, double_click);
        if !handled {
            return false;
        }
        if let Some(action) = self.reset_all_button.borrow_mut().take_press_action() {
            click_sound_sink::record_button_click();
            self.pending_action = Some(action);
            return true;
        }
        if let Some(action) = self.done_button.borrow_mut().take_press_action() {
            click_sound_sink::record_button_click();
            self.pending_action = Some(action);
            return true;
        }
        let pressed: Vec<(usize, KeyRowButtonKind, bool)> = self
            .row_buttons
            .iter()
            .filter_map(|btn| {
                let mut b = btn.borrow_mut();
                if b.takes_press() {
                    Some((b.entry_id, b.kind, b.active))
                } else {
                    None
                }
            })
            .collect();
        for (entry_id, kind, active) in pressed {
            if !active {
                continue;
            }
            match kind {
                KeyRowButtonKind::Change => {
                    click_sound_sink::record_button_click();
                    self.pending_action = Some(KeyBindsAction::SelectKey { id: entry_id });
                    return true;
                }
                KeyRowButtonKind::Reset => {
                    click_sound_sink::record_button_click();
                    self.pending_action = Some(KeyBindsAction::ResetKey { id: entry_id });
                    return true;
                }
            }
        }
        true
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        let handled = crate::framework::screen::screen_mouse_released(self, event);
        self.dragging_scrollbar = false;
        handled
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        let handled = crate::framework::screen::screen_mouse_dragged(self, event, dx, dy);
        if self.dragging_scrollbar {
            let height = self.base_ref().height;
            let max_scroll = self.max_scroll_amount(height);
            let bar_h = self.scrollbar_height(height);
            let scroll_area_h = height - HEADER_HEIGHT - FOOTER_HEIGHT;
            let y_drag_scale = if scroll_area_h - bar_h > 0 {
                (max_scroll / (scroll_area_h - bar_h) as f32).max(1.0)
            } else {
                1.0
            };
            self.scroll_y = (self.scroll_y + dy as f32 * y_drag_scale).clamp(0.0, max_scroll);
            self.update_row_rects();
            true
        } else {
            handled
        }
    }
    fn mouse_scrolled(&mut self, _x: f64, _y: f64, _scroll_x: f64, scroll_y: f64) -> bool {
        let height = self.base_ref().height;
        let max_scroll = self.max_scroll_amount(height);
        self.scroll_y = (self.scroll_y - scroll_y as f32 * 12.0).clamp(0.0, max_scroll);
        self.update_row_rects();
        true
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
        let font = Arc::clone(&self.font);
        self.render_screen(graphics, font.as_ref());
    }
}
const SCROLLER_WIDTH: i32 = SCROLLBAR_WIDTH;
