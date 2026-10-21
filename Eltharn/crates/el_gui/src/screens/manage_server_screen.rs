use crate::FontWidthMeasurer;
use crate::components::cycle_button::CycleButton;
use crate::components::edit_box::{Clipboard, EditBox, EditBoxDraw, EditBoxKey, EditBoxRenderPlan};
use crate::components::{
    Button, GuiEventListener, Renderable, TabOrderedElement, WidgetClickSound,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{
    NarratableEntry, NarratedElementType, NarrationElementOutput, NarrationPriority,
};
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::framework::screen::{
    Screen, ScreenBase, screen_char_typed, screen_key_pressed, screen_key_released,
    screen_mouse_clicked, screen_mouse_dragged, screen_mouse_scrolled, screen_rebuild_widgets,
};
use crate::input::events::{CharacterEvent, CursorType, KeyEvent, MouseButtonEvent, PreeditEvent};
use el_core::{Component, ResourceLocation};
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
pub const FIELD_X_OFFSET: i32 = -100;
pub const NAME_FIELD_Y: i32 = 66;
pub const IP_FIELD_Y: i32 = 106;
pub const FIELD_WIDTH: i32 = 200;
pub const FIELD_HEIGHT: i32 = 20;
pub const PACK_ROW_BASE_Y: i32 = 72;
pub const DONE_BASE_Y: i32 = 96;
pub const ROW_GAP_Y: i32 = 18;
pub const CANCEL_BASE_Y: i32 = 120;
pub const TITLE_Y: i32 = 17;
pub const NAME_LABEL_X_OFFSET: i32 = 1;
pub const NAME_LABEL_Y: i32 = 53;
pub const IP_LABEL_Y: i32 = 94;
pub const LABEL_COLOR: i32 = 0xFFA0_A0A0u32 as i32;
pub const TITLE_COLOR: i32 = 0xFFFF_FFFFu32 as i32;
pub const IP_MAX_LENGTH: usize = 128;
pub const NAME_MAX_LENGTH: usize = 32;
pub const PACK_ROW_NAME_KEY: &str = "manageServer.resourcePack";
pub const DEFAULT_NAME_HINT_KEY: &str = "selectServer.defaultName";
pub const DONE_MESSAGE_KEY: &str = "gui.done";
pub const CANCEL_MESSAGE_KEY: &str = "gui.cancel";
pub const PACK_STATUSES: [ResourcePackStatus; 3] = [
    ResourcePackStatus::Enabled,
    ResourcePackStatus::Disabled,
    ResourcePackStatus::Prompt,
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourcePackStatus {
    Enabled,
    Disabled,
    Prompt,
}
impl ResourcePackStatus {
    pub fn next(self) -> Self {
        match self {
            Self::Enabled => Self::Disabled,
            Self::Disabled => Self::Prompt,
            Self::Prompt => Self::Enabled,
        }
    }
    fn name_key(self) -> &'static str {
        match self {
            Self::Enabled => "manageServer.resourcePack.enabled",
            Self::Disabled => "manageServer.resourcePack.disabled",
            Self::Prompt => "manageServer.resourcePack.prompt",
        }
    }
    pub fn display_string(self) -> String {
        crate::translate(self.name_key())
    }
    pub fn row_label(self) -> String {
        format!(
            "{}: {}",
            crate::translate(PACK_ROW_NAME_KEY),
            self.display_string()
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManageServerAction {
    Cancel,
    Done {
        name: String,
        ip: String,
        pack_status: ResourcePackStatus,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowAction {
    Done,
    Cancel,
}
pub struct ManageServerScreen {
    base: ScreenBase,
    pub in_world: bool,
    pub edit_index: Option<usize>,
    pub server_name: String,
    pub server_ip: String,
    pub pack_status: ResourcePackStatus,
    pub focused_field: u32,
    pub name_cursor: usize,
    pub ip_cursor: usize,
    pub title_key: String,
    font: Arc<dyn FontWidthMeasurer>,
    name_edit: Rc<RefCell<EditBox>>,
    ip_edit: Rc<RefCell<EditBox>>,
    pack_row: Option<Rc<RefCell<CycleButton<ResourcePackStatus>>>>,
    done_row: Rc<RefCell<Button<RowAction>>>,
    cancel_row: Rc<RefCell<Button<RowAction>>>,
    clock_ms: Rc<Cell<i64>>,
    clipboard: Rc<RefCell<Option<Rc<RefCell<dyn Clipboard>>>>>,
}
impl ManageServerScreen {
    pub fn new(
        in_world: bool,
        edit_index: Option<usize>,
        server_name: String,
        server_ip: String,
        pack_status: ResourcePackStatus,
        is_edit: bool,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let title_key = if is_edit {
            "manageServer.edit.title"
        } else {
            "manageServer.add.title"
        }
        .to_string();
        let mut name_edit = EditBox::new(font.as_ref(), 0, 0, FIELD_WIDTH, FIELD_HEIGHT);
        name_edit.set_max_length(font.as_ref(), NAME_MAX_LENGTH);
        name_edit.set_hint(Some(crate::translate(DEFAULT_NAME_HINT_KEY)));
        name_edit.set_value(font.as_ref(), &server_name);
        let mut ip_edit = EditBox::new(font.as_ref(), 0, 0, FIELD_WIDTH, FIELD_HEIGHT);
        ip_edit.set_max_length(font.as_ref(), IP_MAX_LENGTH);
        ip_edit.set_value(font.as_ref(), &server_ip);
        let pack_row = CycleButton::<ResourcePackStatus>::builder(
            |status: &ResourcePackStatus| Component::literal(&status.display_string()),
            move || pack_status,
        )
        .with_values(PACK_STATUSES.to_vec())
        .create_at(
            0,
            0,
            FIELD_WIDTH,
            FIELD_HEIGHT,
            Component::literal(&crate::translate(PACK_ROW_NAME_KEY)),
        );
        let done_row = Rc::new(RefCell::new(
            Button::<RowAction>::builder(Component::literal(&crate::translate(DONE_MESSAGE_KEY)))
                .bounds(0, 0, FIELD_WIDTH, FIELD_HEIGHT)
                .with_action(RowAction::Done)
                .with_font(Arc::clone(&font))
                .build(),
        ));
        let cancel_row = Rc::new(RefCell::new(
            Button::<RowAction>::builder(Component::literal(&crate::translate(CANCEL_MESSAGE_KEY)))
                .bounds(0, 0, FIELD_WIDTH, FIELD_HEIGHT)
                .with_action(RowAction::Cancel)
                .with_font(Arc::clone(&font))
                .build(),
        ));
        let title = crate::translate(&title_key);
        let mut screen = Self {
            base: ScreenBase::new(Component::literal(&title)),
            in_world,
            edit_index,
            server_name,
            server_ip,
            pack_status,
            focused_field: 0,
            name_cursor: 0,
            ip_cursor: 0,
            title_key,
            font,
            name_edit: Rc::new(RefCell::new(name_edit)),
            ip_edit: Rc::new(RefCell::new(ip_edit)),
            pack_row: pack_row.map(|row| Rc::new(RefCell::new(row))),
            done_row,
            cancel_row,
            clock_ms: Rc::new(Cell::new(0)),
            clipboard: Rc::new(RefCell::new(None)),
        };
        screen.update_done_row_active();
        screen
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn edit_index(&self) -> Option<usize> {
        self.edit_index
    }
    pub fn title_key(&self) -> &str {
        &self.title_key
    }
    pub fn row_x(width: i32) -> i32 {
        width / 2 + FIELD_X_OFFSET
    }
    pub fn name_field_y() -> i32 {
        NAME_FIELD_Y
    }
    pub fn ip_field_y() -> i32 {
        IP_FIELD_Y
    }
    pub fn pack_row_y(height: i32) -> i32 {
        height / 4 + PACK_ROW_BASE_Y
    }
    pub fn done_y(height: i32) -> i32 {
        height / 4 + DONE_BASE_Y + ROW_GAP_Y
    }
    pub fn cancel_y(height: i32) -> i32 {
        height / 4 + CANCEL_BASE_Y + ROW_GAP_Y
    }
    fn update_done_row_active(&mut self) {
        let active = crate::is_valid_address(&self.ip_edit.borrow().value());
        self.done_row.borrow_mut().base.active = active;
    }
    fn sync_from_widgets(&mut self) {
        self.server_name = self.name_edit.borrow().value();
        self.server_ip = self.ip_edit.borrow().value();
        self.name_cursor = self.name_edit.borrow().cursor_position();
        self.ip_cursor = self.ip_edit.borrow().cursor_position();
        if let Some(pack_row) = self.pack_row.as_ref() {
            self.pack_status = *pack_row.borrow().value();
        }
        if self.name_edit.borrow().is_focused() {
            self.focused_field = 0;
        } else if self.ip_edit.borrow().is_focused() {
            self.focused_field = 1;
        }
        self.update_done_row_active();
    }
    fn refresh_clock(&mut self) {
        let now = self.base.clock_ms();
        self.clock_ms.set(if now < 0 { 0 } else { now });
    }
    fn now_ms(&self) -> u64 {
        let now = self.clock_ms.get();
        if now < 0 { 0 } else { now as u64 }
    }
    pub fn set_clipboard(&mut self, clipboard: Rc<RefCell<dyn Clipboard>>) {
        *self.clipboard.borrow_mut() = Some(clipboard);
    }
    pub fn take_action(&mut self) -> Option<ManageServerAction> {
        self.sync_from_widgets();
        if self.base.take_close_request() {
            return Some(ManageServerAction::Cancel);
        }
        if self.done_row.borrow_mut().take_press_action().is_some() {
            return Some(ManageServerAction::Done {
                name: self.server_name.clone(),
                ip: self.server_ip.clone(),
                pack_status: self.pack_status,
            });
        }
        self.cancel_row
            .borrow_mut()
            .take_press_action()
            .map(|_| ManageServerAction::Cancel)
    }
    pub fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.refresh_clock();
        let consumed = screen_mouse_clicked(self, event, double_click);
        self.sync_from_widgets();
        consumed
    }
    pub fn name_value(&self) -> String {
        self.name_edit.borrow().value()
    }
    pub fn ip_value(&self) -> String {
        self.ip_edit.borrow().value()
    }
    pub fn name_caret(&self) -> usize {
        self.name_edit.borrow().cursor_position()
    }
    pub fn ip_caret(&self) -> usize {
        self.ip_edit.borrow().cursor_position()
    }
    pub fn pack_status_value(&self) -> ResourcePackStatus {
        match self.pack_row.as_ref() {
            Some(pack_row) => *pack_row.borrow().value(),
            None => self.pack_status,
        }
    }
    pub fn focused_field_value(&self) -> u32 {
        if self.name_edit.borrow().is_focused() {
            0
        } else if self.ip_edit.borrow().is_focused() {
            1
        } else {
            self.focused_field
        }
    }
}
impl Screen for ManageServerScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        let x = Self::row_x(self.base.width);
        let height = self.base.height;
        self.name_edit
            .borrow_mut()
            .set_position(self.font.as_ref(), x, NAME_FIELD_Y);
        self.ip_edit
            .borrow_mut()
            .set_position(self.font.as_ref(), x, IP_FIELD_Y);
        self.ip_edit
            .borrow_mut()
            .set_max_length(self.font.as_ref(), IP_MAX_LENGTH);
        let name = self.server_name.clone();
        let ip = self.server_ip.clone();
        self.name_edit
            .borrow_mut()
            .set_value(self.font.as_ref(), &name);
        self.ip_edit.borrow_mut().set_value(self.font.as_ref(), &ip);
        let pack_y = Self::pack_row_y(height);
        if let Some(pack_row) = self.pack_row.as_ref() {
            let mut pack_row = pack_row.borrow_mut();
            pack_row.base.set_x(x);
            pack_row.base.set_y(pack_y);
            pack_row.base.set_width(FIELD_WIDTH);
            pack_row.base.set_height(FIELD_HEIGHT);
        }
        {
            let mut done = self.done_row.borrow_mut();
            done.base.set_x(x);
            done.base.set_y(Self::done_y(height));
        }
        {
            let mut cancel = self.cancel_row.borrow_mut();
            cancel.base.set_x(x);
            cancel.base.set_y(Self::cancel_y(height));
        }
        self.base.add_widget(EditBoxHandle::new(
            Rc::clone(&self.name_edit),
            Arc::clone(&self.font),
            Rc::clone(&self.clock_ms),
            NAME_LABEL_KEY,
            self.clipboard.clone(),
        ));
        self.base.add_widget(EditBoxHandle::new(
            Rc::clone(&self.ip_edit),
            Arc::clone(&self.font),
            Rc::clone(&self.clock_ms),
            IP_LABEL_KEY,
            self.clipboard.clone(),
        ));
        if let Some(pack_row) = self.pack_row.as_ref() {
            self.base
                .add_widget(CycleButtonHandle::new(Rc::clone(pack_row)));
        }
        self.base
            .add_widget(ButtonHandle::new(Rc::clone(&self.done_row)));
        self.base
            .add_widget(ButtonHandle::new(Rc::clone(&self.cancel_row)));
        self.sync_from_widgets();
    }
    fn tick(&mut self) {
        self.refresh_clock();
        self.sync_from_widgets();
    }
    fn reposition_elements(&mut self) {
        self.sync_from_widgets();
        screen_rebuild_widgets(self);
    }
    fn on_close(&mut self) {
        self.base.request_close();
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        self.refresh_clock();
        self.sync_from_widgets();
        let width = self.base.width;
        let center_x = width / 2;
        self.base.extract_background_layers(graphics, self.in_world);
        self.base
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
        let title = crate::translate(&self.title_key);
        let title_x = center_x - font_width(self.font.as_ref(), &title) / 2;
        graphics.text(
            ResourceLocation::from_string(DEFAULT_FONT),
            title,
            title_x,
            TITLE_Y,
            TITLE_COLOR,
        );
        let label_x = center_x + FIELD_X_OFFSET + NAME_LABEL_X_OFFSET;
        graphics.text(
            ResourceLocation::from_string(DEFAULT_FONT),
            crate::translate(NAME_LABEL_KEY),
            label_x,
            NAME_LABEL_Y,
            LABEL_COLOR,
        );
        graphics.text(
            ResourceLocation::from_string(DEFAULT_FONT),
            crate::translate(IP_LABEL_KEY),
            label_x,
            IP_LABEL_Y,
            LABEL_COLOR,
        );
        for edit_box in [&self.name_edit, &self.ip_edit] {
            let hovered = graphics.contains_point_in_scissor(mouse_x, mouse_y)
                && edit_box.borrow().contains(mouse_x as f64, mouse_y as f64);
            if hovered {
                graphics.request_cursor(CursorType::Text);
            }
            let plan = edit_box
                .borrow()
                .render_plan(self.font.as_ref(), self.now_ms());
            draw_edit_box_plan(graphics, &plan);
        }
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        self.refresh_clock();
        let consumed = screen_key_pressed(self, event);
        self.sync_from_widgets();
        consumed
    }
    fn key_released(&mut self, event: KeyEvent) -> bool {
        self.refresh_clock();
        let consumed = screen_key_released(self, event);
        self.sync_from_widgets();
        consumed
    }
    fn char_typed(&mut self, event: CharacterEvent) -> bool {
        self.refresh_clock();
        let consumed = screen_char_typed(self, event);
        self.sync_from_widgets();
        consumed
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        ManageServerScreen::mouse_clicked(self, event, double_click)
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        self.refresh_clock();
        let consumed = screen_mouse_dragged(self, event, dx, dy);
        self.sync_from_widgets();
        consumed
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        self.refresh_clock();
        let consumed = screen_mouse_scrolled(self, x, y, scroll_x, scroll_y);
        self.sync_from_widgets();
        consumed
    }
    fn mouse_moved(&mut self, _x: f64, _y: f64) {
        self.refresh_clock();
    }
    fn update_narration(&mut self, output: &mut NarrationElementOutput) {
        self.base.update_narration(output);
    }
    fn fill_crash_details(&self, details: &mut Vec<(&'static str, String)>) {
        details.push(("Screen name", "ManageServerScreen".to_string()));
        details.push(("title", self.title_key.clone()));
    }
}
impl std::fmt::Debug for ManageServerScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManageServerScreen")
            .field("in_world", &self.in_world)
            .field("edit_index", &self.edit_index)
            .field("title_key", &self.title_key)
            .field("server_name", &self.server_name)
            .field("server_ip", &self.server_ip)
            .field("pack_status", &self.pack_status)
            .field("focused_field", &self.focused_field)
            .field("name_cursor", &self.name_cursor)
            .field("ip_cursor", &self.ip_cursor)
            .field("width", &self.base.width)
            .field("height", &self.base.height)
            .field("widgets", &self.base.widgets_len())
            .field("focused", &self.base.focused_index())
            .finish()
    }
}
const NAME_LABEL_KEY: &str = "manageServer.enterName";
const IP_LABEL_KEY: &str = "manageServer.enterIp";
const NARRATE_EDIT_BOX_KEY: &str = "gui.narrate.editBox";
const NARRATE_EDIT_BOX_EN_US: &str = "%s edit box: %s";
const DEFAULT_FONT: &str = "minecraft:default";
const INSERT_CURSOR_LINE_HEIGHT: i32 = 10;
fn font_width(font: &dyn FontWidthMeasurer, text: &str) -> i32 {
    font.measure_width(text, 1.0).ceil() as i32
}
fn draw_edit_box_plan(graphics: &mut GuiGraphicsExtractor, plan: &EditBoxRenderPlan) {
    for draw in &plan.draws {
        match draw {
            EditBoxDraw::Sprite {
                sprite,
                x,
                y,
                width,
                height,
            } => {
                graphics.blit(
                    ResourceLocation::from_string(sprite),
                    *x,
                    *y,
                    *width,
                    *height,
                );
            }
            EditBoxDraw::Text {
                text,
                x,
                y,
                color,
                shadow,
            } => {
                graphics.text_with_shadow(
                    ResourceLocation::from_string(DEFAULT_FONT),
                    text.clone(),
                    *x,
                    *y,
                    *color,
                    *shadow,
                );
            }
            EditBoxDraw::Highlight {
                x0,
                y0,
                x1,
                y1,
                invert,
            } => {
                let (left, right) = (*x0.min(x1), *x0.max(x1));
                let (top, bottom) = (*y0.min(y1), *y0.max(y1));
                graphics.text_highlight(left, top, right, bottom, *invert);
            }
            EditBoxDraw::InsertCursor { x, y, color } => {
                graphics.fill(*x, *y - 1, *x + 1, *y + INSERT_CURSOR_LINE_HEIGHT, *color);
            }
            EditBoxDraw::AppendCursor {
                x,
                y,
                color,
                shadow,
            } => {
                graphics.text_with_shadow(
                    ResourceLocation::from_string(DEFAULT_FONT),
                    EditBox::append_cursor_character(),
                    *x,
                    *y,
                    *color,
                    *shadow,
                );
            }
        }
    }
    if let Some(underline) = plan.preedit_underline {
        if underline.x1 > underline.x0 {
            graphics.fill(
                underline.x0,
                underline.y,
                underline.x1,
                underline.y + 1,
                underline.color,
            );
        }
    }
}
struct EditBoxHandle {
    edit_box: Rc<RefCell<EditBox>>,
    font: Arc<dyn FontWidthMeasurer>,
    clock_ms: Rc<Cell<i64>>,
    message_key: &'static str,
    clipboard: Rc<RefCell<Option<Rc<RefCell<dyn Clipboard>>>>>,
}
impl EditBoxHandle {
    fn new(
        edit_box: Rc<RefCell<EditBox>>,
        font: Arc<dyn FontWidthMeasurer>,
        clock_ms: Rc<Cell<i64>>,
        message_key: &'static str,
        clipboard: Rc<RefCell<Option<Rc<RefCell<dyn Clipboard>>>>>,
    ) -> Self {
        Self {
            edit_box,
            font,
            clock_ms,
            message_key,
            clipboard,
        }
    }
    fn now_ms(&self) -> u64 {
        let now = self.clock_ms.get();
        if now < 0 { 0 } else { now as u64 }
    }
}
impl Renderable for EditBoxHandle {
    fn extract_render_state(
        &mut self,
        _graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
    }
}
impl TabOrderedElement for EditBoxHandle {
    fn tab_order_group(&self) -> i32 {
        0
    }
}
impl GuiEventListener for EditBoxHandle {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        if event.button() != 0 {
            return false;
        }
        if !self.edit_box.borrow().contains(event.x, event.y) {
            return false;
        }
        let shift = event.button_info.modifiers & KeyEvent::MOD_SHIFT != 0;
        self.edit_box
            .borrow_mut()
            .on_click(self.font.as_ref(), event.x, shift, double_click);
        true
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, _dx: f64, _dy: f64) -> bool {
        if event.button() != 0 {
            return false;
        }
        self.edit_box
            .borrow_mut()
            .on_drag(self.font.as_ref(), event.x);
        true
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if !self.edit_box.borrow().is_focused() {
            return false;
        }
        let key = match event.key {
            KeyEvent::KEY_BACKSPACE => Some(EditBoxKey::Backspace),
            KeyEvent::KEY_DELETE => Some(EditBoxKey::Delete),
            KeyEvent::KEY_LEFT => Some(EditBoxKey::Left),
            KeyEvent::KEY_RIGHT => Some(EditBoxKey::Right),
            KeyEvent::KEY_HOME => Some(EditBoxKey::Home),
            KeyEvent::KEY_END => Some(EditBoxKey::End),
            _ => None,
        };
        if let Some(key) = key {
            return self.edit_box.borrow_mut().key_pressed(
                self.font.as_ref(),
                key,
                event.has_control_down(),
                event.has_shift_down(),
            );
        }
        if event.is_select_all() {
            return self.edit_box.borrow_mut().select_all(self.font.as_ref());
        }
        let Some(clipboard) = self.clipboard.borrow().clone() else {
            return false;
        };
        let mut clipboard = clipboard.borrow_mut();
        if event.is_copy() {
            return self.edit_box.borrow().copy(&mut *clipboard);
        }
        if event.is_paste() {
            return self
                .edit_box
                .borrow_mut()
                .paste(self.font.as_ref(), &*clipboard);
        }
        if event.is_cut() {
            return self
                .edit_box
                .borrow_mut()
                .cut(self.font.as_ref(), &mut *clipboard);
        }
        false
    }
    fn char_typed(&mut self, event: CharacterEvent) -> bool {
        self.edit_box
            .borrow_mut()
            .char_typed(self.font.as_ref(), event.codepoint)
    }
    fn preedit_updated(&mut self, event: Option<PreeditEvent>) -> bool {
        self.edit_box
            .borrow_mut()
            .preedit_updated(self.font.as_ref(), event)
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        None
    }
    fn next_focus_path(&self, _event: &FocusNavigationEvent) -> Option<ComponentPath> {
        (!self.edit_box.borrow().is_focused()).then(|| ComponentPath::leaf(0))
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.edit_box.borrow().contains(mouse_x, mouse_y)
    }
    fn rectangle(&self) -> ScreenRectangle {
        let edit_box = self.edit_box.borrow();
        ScreenRectangle::new(
            edit_box.x(),
            edit_box.y(),
            edit_box.width(),
            edit_box.height(),
        )
    }
    fn set_focused(&mut self, focused: bool) {
        self.edit_box
            .borrow_mut()
            .set_focused(focused, self.now_ms());
    }
    fn is_focused(&self) -> bool {
        self.edit_box.borrow().is_focused()
    }
}
impl NarratableEntry for EditBoxHandle {
    fn is_active(&self) -> bool {
        self.edit_box.borrow().is_visible()
    }
    fn narration_priority(&self) -> NarrationPriority {
        if self.edit_box.borrow().is_focused() {
            NarrationPriority::Focused
        } else {
            NarrationPriority::None
        }
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        let label = crate::translate(self.message_key);
        let value = self.edit_box.borrow().value();
        output.add(
            NarratedElementType::Title,
            translate_args(
                NARRATE_EDIT_BOX_KEY,
                NARRATE_EDIT_BOX_EN_US,
                &[&label, &value],
            ),
        );
    }
}
impl LayoutElement for EditBoxHandle {
    fn set_x(&mut self, x: i32) {
        let y = self.edit_box.borrow().y();
        self.edit_box
            .borrow_mut()
            .set_position(self.font.as_ref(), x, y);
    }
    fn set_y(&mut self, y: i32) {
        let x = self.edit_box.borrow().x();
        self.edit_box
            .borrow_mut()
            .set_position(self.font.as_ref(), x, y);
    }
    fn x(&self) -> i32 {
        self.edit_box.borrow().x()
    }
    fn y(&self) -> i32 {
        self.edit_box.borrow().y()
    }
    fn width(&self) -> i32 {
        self.edit_box.borrow().width()
    }
    fn height(&self) -> i32 {
        self.edit_box.borrow().height()
    }
}
struct CycleButtonHandle<T: Clone + PartialEq> {
    button: Rc<RefCell<CycleButton<T>>>,
}
impl<T: Clone + PartialEq> CycleButtonHandle<T> {
    fn new(button: Rc<RefCell<CycleButton<T>>>) -> Self {
        Self { button }
    }
}
impl<T: Clone + PartialEq> TabOrderedElement for CycleButtonHandle<T> {
    fn tab_order_group(&self) -> i32 {
        self.button.borrow().tab_order_group()
    }
}
impl<T: Clone + PartialEq> Renderable for CycleButtonHandle<T> {
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        self.button
            .borrow_mut()
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
    }
}
impl<T: Clone + PartialEq> GuiEventListener for CycleButtonHandle<T> {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.button.borrow_mut().mouse_clicked(event, double_click)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        self.button.borrow_mut().key_pressed(event)
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        self.button
            .borrow_mut()
            .mouse_scrolled(x, y, scroll_x, scroll_y)
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.button.borrow().play_down_sound()
    }
    fn next_focus_path(&self, event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.button.borrow().next_focus_path(event)
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.button.borrow().is_mouse_over(mouse_x, mouse_y)
    }
    fn rectangle(&self) -> ScreenRectangle {
        GuiEventListener::rectangle(&*self.button.borrow())
    }
    fn set_focused(&mut self, focused: bool) {
        self.button.borrow_mut().set_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.button.borrow().is_focused()
    }
}
impl<T: Clone + PartialEq> NarratableEntry for CycleButtonHandle<T> {
    fn is_active(&self) -> bool {
        self.button.borrow().base.is_active()
    }
    fn narration_priority(&self) -> NarrationPriority {
        self.button.borrow().narration_priority()
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        self.button.borrow().update_narration(output);
    }
}
impl<T: Clone + PartialEq> LayoutElement for CycleButtonHandle<T> {
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
struct ButtonHandle<A: Clone + std::fmt::Debug + PartialEq> {
    button: Rc<RefCell<Button<A>>>,
}
impl<A: Clone + std::fmt::Debug + PartialEq> ButtonHandle<A> {
    fn new(button: Rc<RefCell<Button<A>>>) -> Self {
        Self { button }
    }
}
impl<A: Clone + std::fmt::Debug + PartialEq> TabOrderedElement for ButtonHandle<A> {
    fn tab_order_group(&self) -> i32 {
        self.button.borrow().tab_order_group()
    }
}
impl<A: Clone + std::fmt::Debug + PartialEq> Renderable for ButtonHandle<A> {
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        self.button
            .borrow_mut()
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
    }
}
impl<A: Clone + std::fmt::Debug + PartialEq> GuiEventListener for ButtonHandle<A> {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.button.borrow_mut().mouse_clicked(event, double_click)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        self.button.borrow_mut().key_pressed(event)
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.button.borrow().play_down_sound()
    }
    fn next_focus_path(&self, event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.button.borrow().next_focus_path(event)
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.button.borrow().is_mouse_over(mouse_x, mouse_y)
    }
    fn rectangle(&self) -> ScreenRectangle {
        GuiEventListener::rectangle(&*self.button.borrow())
    }
    fn set_focused(&mut self, focused: bool) {
        self.button.borrow_mut().set_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.button.borrow().is_focused()
    }
}
impl<A: Clone + std::fmt::Debug + PartialEq> NarratableEntry for ButtonHandle<A> {
    fn narration_priority(&self) -> NarrationPriority {
        self.button.borrow().narration_priority()
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        self.button.borrow().update_narration(output);
    }
}
impl<A: Clone + std::fmt::Debug + PartialEq> LayoutElement for ButtonHandle<A> {
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
pub fn find_clicked_position(text: &str, click_x: i32, font: &dyn FontWidthMeasurer) -> usize {
    if click_x <= 0 {
        return 0;
    }
    let mut closest_idx = 0;
    let mut min_diff = click_x;
    let chars: Vec<char> = text.chars().collect();
    for idx in 0..=chars.len() {
        let prefix: String = chars[..idx].iter().collect();
        let w = font.measure_width(&prefix, 1.0).round() as i32;
        let diff = (w - click_x).abs();
        if diff < min_diff {
            min_diff = diff;
            closest_idx = idx;
        }
    }
    closest_idx
}
fn translate_args(key: &str, en_us: &str, args: &[&str]) -> String {
    let translated = crate::translate(key);
    let mut filled = if translated.contains('%') {
        translated
    } else {
        en_us.to_string()
    };
    for arg in args {
        if filled.contains("%s") {
            filled = filled.replacen("%s", arg, 1);
        }
    }
    filled
}
