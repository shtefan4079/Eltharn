use crate::FontWidthMeasurer;
use crate::components::{
    Button, GuiEventListener, Renderable, TabOrderedElement, WidgetClickSound,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{NarratableEntry, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::framework::screen::{Screen, ScreenBase, screen_key_pressed};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
const HEADER_HEIGHT: i32 = 33;
const FOOTER_HEIGHT: i32 = 60;
const SERVER_ROW_HEIGHT: i32 = 36;
const ENTRY_LIST_INSET: i32 = 2;
const ENTRY_CONTENT_INSET: i32 = 2;
const SERVER_ROW_WIDTH: i32 = 305;
const SERVER_ICON_SIZE: i32 = 32;
const STATUS_ICON_WIDTH: i32 = 10;
const STATUS_ICON_HEIGHT: i32 = 8;
const STATUS_ICON_SPACING: i32 = 5;
const DOUBLE_CLICK_THRESHOLD_MS: u128 = 250;
fn now_millis() -> u128 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
fn first_entry_y(scroll_y: f32) -> i32 {
    HEADER_HEIGHT + ENTRY_LIST_INSET - scroll_y as i32
}
fn wrap_motd_runs(
    runs: &[MotdRun],
    max_width: f32,
    font: &dyn FontWidthMeasurer,
) -> Vec<Vec<MotdRun>> {
    let max_width = max_width.max(1.0);
    let chars: Vec<(char, Option<u32>)> = runs
        .iter()
        .flat_map(|run| run.text.chars().map(move |ch| (ch, run.color)))
        .collect();
    let mut lines = Vec::new();
    let mut paragraph = 0usize;
    for paragraph_end in paragraph_ends(&chars) {
        let para = &chars[paragraph..paragraph_end];
        paragraph = paragraph_end;
        let mut start = 0usize;
        loop {
            if start >= para.len() {
                lines.push(Vec::new());
                break;
            }
            let mut width = 0.0f32;
            let mut had_non_zero_width_char = false;
            let mut last_space: Option<usize> = None;
            let mut next_char = start;
            let mut line_break: Option<usize> = None;
            for i in start..para.len() {
                let (codepoint, _) = para[i];
                if codepoint == ' ' {
                    last_space = Some(i);
                }
                let char_width = font.measure_width(codepoint.to_string().as_str(), 1.0);
                width += char_width;
                if !had_non_zero_width_char || !(width > max_width) {
                    if char_width != 0.0 {
                        had_non_zero_width_char = true;
                    }
                    next_char = i + 1;
                } else if let Some(space) = last_space {
                    line_break = Some(space);
                } else {
                    line_break = Some(i);
                }
                if let Some(_) = line_break {
                    break;
                }
            }
            let end = line_break.unwrap_or(next_char);
            let slice = &para[start..end];
            if slice.is_empty() {
                lines.push(Vec::new());
                start = end;
            } else {
                lines.push(runs_from_chars(slice));
                let break_char = para.get(end).map(|(ch, _)| *ch);
                let adjust = match break_char {
                    Some(' ') | Some('\n') => 1,
                    _ => 0,
                };
                start = end + adjust;
                if line_break.is_none() && (end == next_char || end == para.len()) {
                    break;
                }
            }
        }
    }
    if lines.is_empty() {
        lines.push(Vec::new());
    }
    lines
}
fn paragraph_ends(chars: &[(char, Option<u32>)]) -> Vec<usize> {
    let mut ends = Vec::new();
    for (idx, (ch, _)) in chars.iter().enumerate() {
        if *ch == '\n' {
            ends.push(idx);
        }
    }
    ends.push(chars.len());
    ends
}
fn runs_from_chars(chars: &[(char, Option<u32>)]) -> Vec<MotdRun> {
    let mut runs: Vec<MotdRun> = Vec::new();
    for &(ch, color) in chars {
        if let Some(run) = runs.last_mut().filter(|run| run.color == color) {
            run.text.push(ch);
        } else {
            runs.push(MotdRun {
                text: ch.to_string(),
                color,
            });
        }
    }
    runs
}
fn server_row_left(width: i32) -> i32 {
    width / 2 - SERVER_ROW_WIDTH / 2
}
fn status_icon_x(row_left: i32) -> i32 {
    row_left + SERVER_ROW_WIDTH - ENTRY_CONTENT_INSET - STATUS_ICON_WIDTH - STATUS_ICON_SPACING
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerPingState {
    Initial,
    Pinging,
    Unreachable,
    Incompatible,
    Successful,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MotdRun {
    pub text: String,
    pub color: Option<u32>,
}
#[derive(Clone, Debug)]
pub struct ServerEntry {
    pub name: String,
    pub ip: String,
    pub motd: String,
    pub motd_red: bool,
    pub motd_runs: Vec<MotdRun>,
    pub players_online: i32,
    pub players_max: i32,
    pub players_present: bool,
    pub status_text_ready: bool,
    pub ping: i64,
    pub ping_state: ServerPingState,
    pub status_display_state: ServerPingState,
    pub cached_pinging_frame: Option<u8>,
    pub version: String,
    pub player_list: Vec<String>,
    pub icon_bytes: Option<Vec<u8>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanServerEntry {
    pub motd: String,
    pub address: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultiplayerAction {
    Back,
    Join(usize),
    DirectConnect,
    AddServer,
    Edit(usize),
    Delete(usize),
    Refresh,
    Swap(usize, usize),
    JoinLan(usize),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultiplayerFooterButton {
    Select,
    DirectConnect,
    AddServer,
    Edit,
    Delete,
    Refresh,
    Back,
}
struct FooterButtonHandle {
    button: Rc<RefCell<Button<MultiplayerFooterButton>>>,
}
impl FooterButtonHandle {
    fn new(button: Rc<RefCell<Button<MultiplayerFooterButton>>>) -> Self {
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
pub struct MultiplayerScreen {
    base: ScreenBase,
    font: Arc<dyn FontWidthMeasurer>,
    action_sink: Rc<RefCell<Option<MultiplayerAction>>>,
    select_button: Rc<RefCell<Button<MultiplayerFooterButton>>>,
    direct_button: Rc<RefCell<Button<MultiplayerFooterButton>>>,
    add_button: Rc<RefCell<Button<MultiplayerFooterButton>>>,
    edit_button: Rc<RefCell<Button<MultiplayerFooterButton>>>,
    delete_button: Rc<RefCell<Button<MultiplayerFooterButton>>>,
    refresh_button: Rc<RefCell<Button<MultiplayerFooterButton>>>,
    back_button: Rc<RefCell<Button<MultiplayerFooterButton>>>,
    pub in_world: bool,
    pub scroll_y: f32,
    pub dragging_scrollbar: bool,
    pub selected_index: Option<usize>,
    pub selected_lan_index: Option<usize>,
    pub lan_header_selected: bool,
    pub servers: Vec<ServerEntry>,
    pub lan_servers: Vec<LanServerEntry>,
    pub width: i32,
    pub height: i32,
    pub hide_server_address: bool,
    last_click_ms: Option<u128>,
    pub list_focused: bool,
    pub focused_footer_button: Option<MultiplayerFooterButton>,
    server_icons: HashMap<String, ResourceLocation>,
    pending_server_icons: HashMap<String, ResourceLocation>,
    pending_ping_entries: Vec<ServerEntry>,
    refresh_key_requested: bool,
}
impl MultiplayerScreen {
    pub fn new(
        servers: Vec<ServerEntry>,
        in_world: bool,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let select_btn =
            Button::builder(Component::literal(&crate::translate("selectServer.select")))
                .bounds(0, 0, 100, 20)
                .with_action(MultiplayerFooterButton::Select)
                .with_font(Arc::clone(&font))
                .build();
        let direct_btn =
            Button::builder(Component::literal(&crate::translate("selectServer.direct")))
                .bounds(0, 0, 100, 20)
                .with_action(MultiplayerFooterButton::DirectConnect)
                .with_font(Arc::clone(&font))
                .build();
        let add_btn = Button::builder(Component::literal(&crate::translate("selectServer.add")))
            .bounds(0, 0, 100, 20)
            .with_action(MultiplayerFooterButton::AddServer)
            .with_font(Arc::clone(&font))
            .build();
        let edit_btn = Button::builder(Component::literal(&crate::translate("selectServer.edit")))
            .bounds(0, 0, 74, 20)
            .with_action(MultiplayerFooterButton::Edit)
            .with_font(Arc::clone(&font))
            .build();
        let delete_btn =
            Button::builder(Component::literal(&crate::translate("selectServer.delete")))
                .bounds(0, 0, 74, 20)
                .with_action(MultiplayerFooterButton::Delete)
                .with_font(Arc::clone(&font))
                .build();
        let refresh_btn = Button::builder(Component::literal(&crate::translate(
            "selectServer.refresh",
        )))
        .bounds(0, 0, 74, 20)
        .with_action(MultiplayerFooterButton::Refresh)
        .with_font(Arc::clone(&font))
        .build();
        let back_btn = Button::builder(Component::literal(&crate::translate("gui.back")))
            .bounds(0, 0, 74, 20)
            .with_action(MultiplayerFooterButton::Back)
            .with_font(Arc::clone(&font))
            .build();
        let mut base = ScreenBase::new(Component::literal(&crate::translate("multiplayer.title")));
        base.set_close_on_esc(true);
        let mut s = Self {
            base,
            font,
            action_sink: Rc::new(RefCell::new(None)),
            select_button: Rc::new(RefCell::new(select_btn)),
            direct_button: Rc::new(RefCell::new(direct_btn)),
            add_button: Rc::new(RefCell::new(add_btn)),
            edit_button: Rc::new(RefCell::new(edit_btn)),
            delete_button: Rc::new(RefCell::new(delete_btn)),
            refresh_button: Rc::new(RefCell::new(refresh_btn)),
            back_button: Rc::new(RefCell::new(back_btn)),
            in_world,
            scroll_y: 0.0,
            dragging_scrollbar: false,
            selected_index: None,
            selected_lan_index: None,
            lan_header_selected: false,
            servers,
            lan_servers: Vec::new(),
            width: 0,
            height: 0,
            hide_server_address: false,
            last_click_ms: None,
            list_focused: false,
            focused_footer_button: None,
            server_icons: HashMap::new(),
            pending_server_icons: HashMap::new(),
            pending_ping_entries: Vec::new(),
            refresh_key_requested: false,
        };
        s.update_button_states();
        s
    }
    pub fn take_action(&mut self) -> Option<MultiplayerAction> {
        if let Some(action) = self.action_sink.borrow_mut().take() {
            return Some(action);
        }
        if self.refresh_key_requested {
            self.refresh_key_requested = false;
            return Some(MultiplayerAction::Refresh);
        }
        if self
            .select_button
            .borrow_mut()
            .take_press_action()
            .is_some()
        {
            if let Some(idx) = self.selected_index.filter(|&i| i < self.servers.len()) {
                return Some(MultiplayerAction::Join(idx));
            }
            if let Some(idx) = self
                .selected_lan_index
                .filter(|&i| i < self.lan_servers.len())
            {
                return Some(MultiplayerAction::JoinLan(idx));
            }
        }
        if self
            .direct_button
            .borrow_mut()
            .take_press_action()
            .is_some()
        {
            return Some(MultiplayerAction::DirectConnect);
        }
        if self.add_button.borrow_mut().take_press_action().is_some() {
            return Some(MultiplayerAction::AddServer);
        }
        if self.edit_button.borrow_mut().take_press_action().is_some() {
            if let Some(idx) = self.selected_index.filter(|&i| i < self.servers.len()) {
                return Some(MultiplayerAction::Edit(idx));
            }
        }
        if self
            .delete_button
            .borrow_mut()
            .take_press_action()
            .is_some()
        {
            if let Some(idx) = self.selected_index.filter(|&i| i < self.servers.len()) {
                return Some(MultiplayerAction::Delete(idx));
            }
        }
        if self
            .refresh_button
            .borrow_mut()
            .take_press_action()
            .is_some()
        {
            return Some(MultiplayerAction::Refresh);
        }
        if self.back_button.borrow_mut().take_press_action().is_some() {
            return Some(MultiplayerAction::Back);
        }
        if self.base.take_close_request() {
            return Some(MultiplayerAction::Back);
        }
        None
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn font(&self) -> &Arc<dyn FontWidthMeasurer> {
        &self.font
    }
    pub fn update_button_states(&mut self) {
        let has_joinable = self.has_joinable_selection();
        let has_online = self.has_online_selection();
        self.select_button.borrow_mut().base.active = has_joinable;
        self.edit_button.borrow_mut().base.active = has_online;
        self.delete_button.borrow_mut().base.active = has_online;
        self.direct_button.borrow_mut().base.active = true;
        self.add_button.borrow_mut().base.active = true;
        self.refresh_button.borrow_mut().base.active = true;
        self.back_button.borrow_mut().base.active = true;
    }
    pub fn ensure_layout(&mut self, width: i32, height: i32) {
        self.base.width = width;
        self.base.height = height;
        self.width = width;
        self.height = height;
        let btn_y_top = height - 52;
        let btn_y_bottom = height - 28;
        {
            let mut s = self.select_button.borrow_mut();
            s.base.set_x(width / 2 - 154);
            s.base.set_y(btn_y_top);
            s.base.set_width(100);
            s.base.set_height(20);
        }
        {
            let mut d = self.direct_button.borrow_mut();
            d.base.set_x(width / 2 - 50);
            d.base.set_y(btn_y_top);
            d.base.set_width(100);
            d.base.set_height(20);
        }
        {
            let mut a = self.add_button.borrow_mut();
            a.base.set_x(width / 2 + 54);
            a.base.set_y(btn_y_top);
            a.base.set_width(100);
            a.base.set_height(20);
        }
        {
            let mut e = self.edit_button.borrow_mut();
            e.base.set_x(width / 2 - 154);
            e.base.set_y(btn_y_bottom);
            e.base.set_width(74);
            e.base.set_height(20);
        }
        {
            let mut del = self.delete_button.borrow_mut();
            del.base.set_x(width / 2 - 76);
            del.base.set_y(btn_y_bottom);
            del.base.set_width(74);
            del.base.set_height(20);
        }
        {
            let mut r = self.refresh_button.borrow_mut();
            r.base.set_x(width / 2 + 2);
            r.base.set_y(btn_y_bottom);
            r.base.set_width(74);
            r.base.set_height(20);
        }
        {
            let mut b = self.back_button.borrow_mut();
            b.base.set_x(width / 2 + 80);
            b.base.set_y(btn_y_bottom);
            b.base.set_width(74);
            b.base.set_height(20);
        }
        self.update_button_states();
        self.base.clear_widgets();
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.select_button)));
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.direct_button)));
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.add_button)));
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.edit_button)));
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.delete_button)));
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.refresh_button)));
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.back_button)));
    }
    pub fn scroll_to_entry(&mut self, idx: usize) {
        let top = (idx as i32 * SERVER_ROW_HEIGHT) as f32;
        if top < self.scroll_y {
            self.scroll_y = top;
        }
        let height = if self.base.height > 0 {
            self.base.height
        } else {
            self.height.max(240)
        };
        let visible_h = (height - HEADER_HEIGHT - FOOTER_HEIGHT).max(1) as f32;
        let min_scroll_for_bottom =
            top + (SERVER_ROW_HEIGHT + ENTRY_LIST_INSET * 2) as f32 - visible_h;
        if self.scroll_y < min_scroll_for_bottom {
            self.scroll_y = min_scroll_for_bottom
                .max(0.0)
                .min(self.max_scroll_amount(height));
        }
    }
    pub fn tick(&mut self) {
        self.base.tick();
    }
    pub fn apply_initial_keyboard_focus(&mut self) {
        self.select_entry_ordinal(Some(0));
    }
    pub fn entry_count(&self) -> usize {
        self.servers.len() + 1 + self.lan_servers.len()
    }
    pub fn selected_entry_ordinal(&self) -> Option<usize> {
        if let Some(index) = self.selected_index.filter(|&i| i < self.servers.len()) {
            Some(index)
        } else if self.lan_header_selected {
            Some(self.servers.len())
        } else {
            self.selected_lan_index
                .filter(|&i| i < self.lan_servers.len())
                .map(|i| self.servers.len() + 1 + i)
        }
    }
    pub fn select_entry_ordinal(&mut self, ordinal: Option<usize>) {
        self.selected_index = None;
        self.selected_lan_index = None;
        self.lan_header_selected = false;
        match ordinal.filter(|&ordinal| ordinal < self.entry_count()) {
            Some(ordinal) if ordinal < self.servers.len() => {
                self.selected_index = Some(ordinal);
            }
            Some(ordinal) if ordinal == self.servers.len() => {
                self.lan_header_selected = true;
            }
            Some(ordinal) => {
                self.selected_lan_index = Some(ordinal - self.servers.len() - 1);
            }
            None => {}
        }
        self.focus_list();
    }
    pub fn take_pending_ping_entries(&mut self) -> Vec<ServerEntry> {
        std::mem::take(&mut self.pending_ping_entries)
    }
    fn focus_list(&mut self) {
        self.list_focused = true;
        self.focused_footer_button = None;
        self.base.set_focused(None);
        self.update_button_states();
    }
    fn focus_footer(&mut self, button: MultiplayerFooterButton, now: u128) {
        self.list_focused = false;
        self.focused_footer_button = Some(button);
        self.last_click_ms = Some(now);
        let widget_idx = match button {
            MultiplayerFooterButton::Select => Some(0),
            MultiplayerFooterButton::DirectConnect => Some(1),
            MultiplayerFooterButton::AddServer => Some(2),
            MultiplayerFooterButton::Edit => Some(3),
            MultiplayerFooterButton::Delete => Some(4),
            MultiplayerFooterButton::Refresh => Some(5),
            MultiplayerFooterButton::Back => Some(6),
        };
        if let Some(idx) = widget_idx {
            if idx < self.base.widgets_len() {
                self.base.set_focused(Some(idx));
            }
        } else {
            self.base.set_focused(None);
        }
    }
    pub fn set_server_icon(&mut self, server_ip: impl Into<String>, texture: ResourceLocation) {
        self.pending_server_icons.insert(server_ip.into(), texture);
    }
    pub fn remove_server_icon(&mut self, server_ip: &str) -> Option<ResourceLocation> {
        let pending = self.pending_server_icons.remove(server_ip);
        let active = self.server_icons.remove(server_ip);
        pending.or(active)
    }
    pub fn set_lan_servers(&mut self, servers: Vec<LanServerEntry>) {
        let selected_address = self
            .selected_lan_index
            .and_then(|index| self.lan_servers.get(index))
            .map(|server| server.address.clone());
        self.lan_servers = servers;
        self.selected_lan_index = selected_address.and_then(|address| {
            self.lan_servers
                .iter()
                .position(|server| server.address == address)
        });
        if self.selected_lan_index.is_some() {
            self.selected_index = None;
            self.lan_header_selected = false;
        }
    }
    pub fn should_close_on_esc(&self) -> bool {
        true
    }
    fn has_online_selection(&self) -> bool {
        self.selected_index
            .is_some_and(|index| index < self.servers.len())
    }
    fn has_lan_selection(&self) -> bool {
        self.selected_lan_index
            .is_some_and(|index| index < self.lan_servers.len())
    }
    fn has_joinable_selection(&self) -> bool {
        self.has_online_selection() || self.has_lan_selection()
    }
    pub fn max_scroll_amount(&self, height: i32) -> f32 {
        let total_height =
            (self.servers.len() as i32 + 1 + self.lan_servers.len() as i32) * SERVER_ROW_HEIGHT + 4;
        let visible_height = height - HEADER_HEIGHT - FOOTER_HEIGHT;
        (total_height - visible_height).max(0) as f32
    }
    pub fn scrollbar_height(&self, height: i32) -> i32 {
        let scroll_area_h = height - HEADER_HEIGHT - FOOTER_HEIGHT;
        if scroll_area_h <= 0 {
            return 0;
        }
        let total_content_height =
            ((self.servers.len() as i32 + 1 + self.lan_servers.len() as i32) * SERVER_ROW_HEIGHT
                + 4)
            .max(1);
        let bar_h = (scroll_area_h * scroll_area_h) / total_content_height;
        let upper = (scroll_area_h - 8).max(1);
        let lower = 32.min(upper);
        bar_h.clamp(lower, upper).min(scroll_area_h)
    }
    pub fn mouse_clicked(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        width: i32,
        height: i32,
    ) -> Option<MultiplayerAction> {
        let mx = mouse_x.floor() as i32;
        let my = mouse_y.floor() as i32;
        let now = now_millis();
        let double_click = self
            .last_click_ms
            .is_some_and(|t| now.saturating_sub(t) < DOUBLE_CLICK_THRESHOLD_MS);
        let consume = |s: &mut Self| s.last_click_ms = Some(now);
        let btn_y_top = height - 52;
        if mx >= width / 2 - 154 && mx < width / 2 - 54 && my >= btn_y_top && my < btn_y_top + 20 {
            if let Some(idx) = self.selected_index.filter(|&idx| idx < self.servers.len()) {
                crate::components::click_sound_sink::record_button_click();
                self.focus_footer(MultiplayerFooterButton::Select, now);
                return Some(MultiplayerAction::Join(idx));
            }
            if let Some(idx) = self
                .selected_lan_index
                .filter(|&idx| idx < self.lan_servers.len())
            {
                crate::components::click_sound_sink::record_button_click();
                self.focus_footer(MultiplayerFooterButton::Select, now);
                return Some(MultiplayerAction::JoinLan(idx));
            }
            return None;
        }
        if mx >= width / 2 - 50 && mx < width / 2 + 50 && my >= btn_y_top && my < btn_y_top + 20 {
            crate::components::click_sound_sink::record_button_click();
            self.focus_footer(MultiplayerFooterButton::DirectConnect, now);
            return Some(MultiplayerAction::DirectConnect);
        }
        if mx >= width / 2 + 54 && mx < width / 2 + 154 && my >= btn_y_top && my < btn_y_top + 20 {
            crate::components::click_sound_sink::record_button_click();
            self.focus_footer(MultiplayerFooterButton::AddServer, now);
            return Some(MultiplayerAction::AddServer);
        }
        let btn_y_bottom = height - 28;
        if mx >= width / 2 - 154
            && mx < width / 2 - 80
            && my >= btn_y_bottom
            && my < btn_y_bottom + 20
        {
            if let Some(idx) = self.selected_index.filter(|&idx| idx < self.servers.len()) {
                crate::components::click_sound_sink::record_button_click();
                self.focus_footer(MultiplayerFooterButton::Edit, now);
                return Some(MultiplayerAction::Edit(idx));
            }
            return None;
        }
        if mx >= width / 2 - 76
            && mx < width / 2 - 2
            && my >= btn_y_bottom
            && my < btn_y_bottom + 20
        {
            if let Some(idx) = self.selected_index.filter(|&idx| idx < self.servers.len()) {
                crate::components::click_sound_sink::record_button_click();
                self.focus_footer(MultiplayerFooterButton::Delete, now);
                return Some(MultiplayerAction::Delete(idx));
            }
            return None;
        }
        if mx >= width / 2 + 2
            && mx < width / 2 + 76
            && my >= btn_y_bottom
            && my < btn_y_bottom + 20
        {
            crate::components::click_sound_sink::record_button_click();
            self.focus_footer(MultiplayerFooterButton::Refresh, now);
            return Some(MultiplayerAction::Refresh);
        }
        if mx >= width / 2 + 80
            && mx < width / 2 + 154
            && my >= btn_y_bottom
            && my < btn_y_bottom + 20
        {
            crate::components::click_sound_sink::record_button_click();
            self.focus_footer(MultiplayerFooterButton::Back, now);
            return Some(MultiplayerAction::Back);
        }
        let max_scroll = self.max_scroll_amount(height);
        if max_scroll > 0.0 {
            let scrollbar_x = width / 2 + 161;
            if mx >= scrollbar_x
                && mx < scrollbar_x + 6
                && my >= HEADER_HEIGHT
                && my < height - FOOTER_HEIGHT
            {
                self.dragging_scrollbar = true;
                self.focus_list();
                consume(self);
                return None;
            }
        }
        if my >= HEADER_HEIGHT && my < height - FOOTER_HEIGHT {
            let first_row_y = first_entry_y(self.scroll_y);
            let item_x = server_row_left(width);
            if mx >= item_x && mx < item_x + SERVER_ROW_WIDTH && my >= first_row_y {
                let clicked_idx = (my - first_row_y) / SERVER_ROW_HEIGHT;
                if clicked_idx >= 0 && clicked_idx < self.servers.len() as i32 {
                    let idx = clicked_idx as usize;
                    let item_y = first_row_y + clicked_idx * SERVER_ROW_HEIGHT;
                    let icon_x = item_x + ENTRY_CONTENT_INSET;
                    let icon_y = item_y + ENTRY_CONTENT_INSET;
                    self.selected_index = Some(idx);
                    self.selected_lan_index = None;
                    self.lan_header_selected = false;
                    self.focus_list();
                    if mx >= icon_x
                        && mx < icon_x + SERVER_ICON_SIZE
                        && my >= icon_y
                        && my < icon_y + SERVER_ICON_SIZE
                    {
                        let rel_x = mx - icon_x;
                        let rel_y = my - icon_y;
                        if rel_x >= 16 {
                            crate::components::click_sound_sink::record_button_click();
                            consume(self);
                            return Some(MultiplayerAction::Join(idx));
                        } else if rel_y < 16 && idx > 0 {
                            crate::components::click_sound_sink::record_button_click();
                            consume(self);
                            return Some(MultiplayerAction::Swap(idx, idx - 1));
                        } else if rel_y >= 16 && idx + 1 < self.servers.len() {
                            crate::components::click_sound_sink::record_button_click();
                            consume(self);
                            return Some(MultiplayerAction::Swap(idx, idx + 1));
                        }
                    }
                    if double_click {
                        crate::components::click_sound_sink::record_button_click();
                        consume(self);
                        return Some(MultiplayerAction::Join(idx));
                    }
                } else if clicked_idx == self.servers.len() as i32 {
                    self.selected_index = None;
                    self.selected_lan_index = None;
                    self.lan_header_selected = true;
                    self.focus_list();
                } else if clicked_idx > self.servers.len() as i32 {
                    let lan_idx = clicked_idx - self.servers.len() as i32 - 1;
                    if lan_idx >= 0 && (lan_idx as usize) < self.lan_servers.len() {
                        let lan_idx = lan_idx as usize;
                        self.selected_lan_index = Some(lan_idx);
                        self.selected_index = None;
                        self.lan_header_selected = false;
                        self.focus_list();
                        if double_click {
                            crate::components::click_sound_sink::record_button_click();
                            consume(self);
                            return Some(MultiplayerAction::JoinLan(lan_idx));
                        }
                    }
                }
            }
            consume(self);
        }
        None
    }
    pub fn mouse_released(&mut self) {
        self.dragging_scrollbar = false;
    }
    pub fn reset_after_refresh(&mut self) {
        self.selected_index = None;
        self.selected_lan_index = None;
        self.lan_header_selected = false;
        self.scroll_y = 0.0;
        self.last_click_ms = None;
        self.list_focused = false;
        self.focused_footer_button = None;
        self.pending_ping_entries.clear();
    }
    pub fn mouse_dragged(
        &mut self,
        _mouse_x: f32,
        mouse_y: f32,
        dy: f32,
        _width: i32,
        height: i32,
    ) {
        if self.dragging_scrollbar {
            let max_scroll = self.max_scroll_amount(height);
            if mouse_y < HEADER_HEIGHT as f32 {
                self.scroll_y = 0.0;
            } else if mouse_y > (height - FOOTER_HEIGHT) as f32 {
                self.scroll_y = max_scroll;
            } else {
                let scroll_area_h = height - HEADER_HEIGHT - FOOTER_HEIGHT;
                let bar_h = self.scrollbar_height(height);
                if scroll_area_h > bar_h {
                    let max = max_scroll.max(1.0);
                    let y_drag_scale = (max / (scroll_area_h - bar_h) as f32).max(1.0);
                    self.scroll_y = (self.scroll_y + dy * y_drag_scale).clamp(0.0, max_scroll);
                }
            }
        }
    }
    pub fn mouse_scrolled(&mut self, scroll_y: f32, height: i32) {
        let max_scroll = self.max_scroll_amount(height);
        self.scroll_y = (self.scroll_y - scroll_y * 18.0).clamp(0.0, max_scroll);
    }
    pub fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
    ) {
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        if self.width != width || self.height != height {
            if let Some(ordinal) = self.selected_entry_ordinal() {
                let row_offset = (ordinal as i32 * SERVER_ROW_HEIGHT) - self.scroll_y as i32;
                if row_offset < 0 {
                    self.scroll_y += row_offset as f32;
                }
                let entry_y = first_entry_y(self.scroll_y) + ordinal as i32 * SERVER_ROW_HEIGHT;
                let bottom_delta =
                    (height - FOOTER_HEIGHT) - entry_y - SERVER_ROW_HEIGHT - ENTRY_LIST_INSET;
                if bottom_delta < 0 {
                    self.scroll_y -= bottom_delta as f32;
                }
            }
            self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_amount(height));
        }
        self.width = width;
        self.height = height;
        let mouse_x = graphics.mouse_x();
        let mouse_y = graphics.mouse_y();
        self.base
            .extract_background_backdrop(graphics, self.in_world);
        graphics.draw_menu_background(self.in_world, width, 33);
        let footer_y = height - 60;
        graphics.pose_mut().push_pose();
        graphics.pose_mut().translate(0.0, footer_y as f32);
        graphics.draw_menu_background(self.in_world, width, 60);
        graphics.pose_mut().pop_pose();
        let list_bg = ResourceLocation::from_string(if self.in_world {
            "minecraft:gui/inworld_menu_list_background"
        } else {
            "minecraft:gui/menu_list_background"
        });
        {
            let mut y = 33;
            while y < height - 60 {
                let tile_h = 32.min(height - 60 - y);
                let mut x = 0;
                while x < width {
                    let tile_w = 32.min(width - x);
                    graphics.blit_region(
                        list_bg.clone(),
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
        }
        let title_text = crate::translate("multiplayer.title");
        let title_w = font.measure_width(&title_text, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            title_text,
            (width - title_w) / 2,
            12,
            0xFFFF_FFFFu32 as i32,
        );
        let list_top = HEADER_HEIGHT;
        let list_bottom = height - FOOTER_HEIGHT;
        graphics.enable_scissor(0, list_top, width, list_bottom);
        let item_w = SERVER_ROW_WIDTH;
        let item_x = server_row_left(width);
        let mut hovered_tooltip = None;
        let start_y = first_entry_y(self.scroll_y);
        let outline_color = if self.list_focused {
            0xFFFF_FFFFu32 as i32
        } else {
            0xFF80_8080u32 as i32
        };
        let server_count = self.servers.len();
        for (i, server) in self.servers.iter_mut().enumerate() {
            let item_y = start_y + i as i32 * SERVER_ROW_HEIGHT;
            if item_y + SERVER_ROW_HEIGHT < list_top || item_y > list_bottom {
                continue;
            }
            if server.ping_state == ServerPingState::Initial {
                server.ping_state = ServerPingState::Pinging;
                server.motd.clear();
                server.motd_red = false;
                server.player_list.clear();
                self.pending_ping_entries.push(server.clone());
            }
            let is_selected = self.selected_index == Some(i);
            let hovered = mouse_x >= item_x
                && mouse_x < item_x + item_w
                && mouse_y >= item_y
                && mouse_y < item_y + SERVER_ROW_HEIGHT;
            if is_selected {
                graphics.fill(
                    item_x,
                    item_y,
                    item_x + item_w,
                    item_y + SERVER_ROW_HEIGHT,
                    outline_color,
                );
                graphics.fill(
                    item_x + 1,
                    item_y + 1,
                    item_x + item_w - 1,
                    item_y + SERVER_ROW_HEIGHT - 1,
                    0xFF00_0000u32 as i32,
                );
            }
            let icon_x = item_x + ENTRY_CONTENT_INSET;
            let icon_y = item_y + ENTRY_CONTENT_INSET;
            let icon = self
                .server_icons
                .get(&server.ip)
                .cloned()
                .unwrap_or_else(|| ResourceLocation::from_string("minecraft:misc/unknown_server"));
            graphics.blit(icon, icon_x, icon_y, SERVER_ICON_SIZE, SERVER_ICON_SIZE);
            if let Some(texture) = self.pending_server_icons.remove(&server.ip) {
                self.server_icons.insert(server.ip.clone(), texture);
            }
            let mouse_in_icon = mouse_x >= icon_x
                && mouse_x < icon_x + SERVER_ICON_SIZE
                && mouse_y >= icon_y
                && mouse_y < icon_y + SERVER_ICON_SIZE;
            if hovered {
                graphics.fill(
                    icon_x,
                    icon_y,
                    icon_x + SERVER_ICON_SIZE,
                    icon_y + SERVER_ICON_SIZE,
                    0xA090_9090u32 as i32,
                );
                let rel_x = mouse_x - icon_x;
                let rel_y = mouse_y - icon_y;
                if mouse_in_icon && rel_x >= 16 {
                    graphics.request_cursor(crate::input::events::CursorType::PointingHand);
                } else if i > 0 && mouse_in_icon && rel_x < 16 && rel_y < 16 {
                    graphics.request_cursor(crate::input::events::CursorType::PointingHand);
                } else if i + 1 < server_count && mouse_in_icon && rel_x < 16 && rel_y >= 16 {
                    graphics.request_cursor(crate::input::events::CursorType::PointingHand);
                }
                if mouse_in_icon && rel_x >= 16 {
                    graphics.blit(
                        ResourceLocation::from_string("minecraft:server_list/join_highlighted"),
                        icon_x,
                        icon_y,
                        32,
                        32,
                    );
                } else {
                    graphics.blit(
                        ResourceLocation::from_string("minecraft:server_list/join"),
                        icon_x,
                        icon_y,
                        32,
                        32,
                    );
                }
                if i > 0 {
                    if mouse_in_icon && rel_x < 16 && rel_y < 16 {
                        graphics.blit(
                            ResourceLocation::from_string(
                                "minecraft:server_list/move_up_highlighted",
                            ),
                            icon_x,
                            icon_y,
                            32,
                            32,
                        );
                    } else {
                        graphics.blit(
                            ResourceLocation::from_string("minecraft:server_list/move_up"),
                            icon_x,
                            icon_y,
                            32,
                            32,
                        );
                    }
                }
                if i + 1 < server_count {
                    if mouse_in_icon && rel_x < 16 && rel_y >= 16 {
                        graphics.blit(
                            ResourceLocation::from_string(
                                "minecraft:server_list/move_down_highlighted",
                            ),
                            icon_x,
                            icon_y,
                            32,
                            32,
                        );
                    } else {
                        graphics.blit(
                            ResourceLocation::from_string("minecraft:server_list/move_down"),
                            icon_x,
                            icon_y,
                            32,
                            32,
                        );
                    }
                }
            }
            let text_x = item_x + ENTRY_CONTENT_INSET + SERVER_ICON_SIZE + 3;
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                server.name.clone(),
                text_x,
                item_y + 3,
                0xFFFF_FFFFu32 as i32,
            );
            let motd_color = if server.motd_red {
                0xFFFF_0000u32 as i32
            } else {
                0xFF80_8080u32 as i32
            };
            if server.motd_runs.is_empty() {
                let desc_lines = crate::wrap_text(&server.motd, 267.0, font);
                for (line_idx, line) in desc_lines.iter().take(2).enumerate() {
                    graphics.text(
                        ResourceLocation::from_string("minecraft:default"),
                        line.clone(),
                        text_x,
                        item_y + 14 + line_idx as i32 * 9,
                        motd_color,
                    );
                }
            } else {
                let desc_lines = wrap_motd_runs(&server.motd_runs, 267.0, font);
                for (line_idx, line) in desc_lines.iter().take(2).enumerate() {
                    let mut x = text_x as f32;
                    for run in line {
                        let color = match run.color {
                            Some(rgb) => (0xFF00_0000u32 | rgb) as i32,
                            None => motd_color,
                        };
                        graphics.text(
                            ResourceLocation::from_string("minecraft:default"),
                            run.text.clone(),
                            x.round() as i32,
                            item_y + 14 + line_idx as i32 * 9,
                            color,
                        );
                        x += font.measure_width(&run.text, 1.0);
                    }
                }
            }
            let status_icon_x = status_icon_x(item_x);
            let status_icon_y = item_y + ENTRY_CONTENT_INSET;
            let status_sprite = if server.ping_state == ServerPingState::Pinging {
                let elapsed_ms = now_millis();
                let icon_index = (elapsed_ms / 100 + (i as u128) * 2) & 7;
                let icon_index = if icon_index > 4 {
                    8 - icon_index
                } else {
                    icon_index
                };
                server.cached_pinging_frame = Some(icon_index as u8 + 1);
                match icon_index {
                    1 => "minecraft:server_list/pinging_2",
                    2 => "minecraft:server_list/pinging_3",
                    3 => "minecraft:server_list/pinging_4",
                    4 => "minecraft:server_list/pinging_5",
                    _ => "minecraft:server_list/pinging_1",
                }
            } else {
                match server.status_display_state {
                    ServerPingState::Initial | ServerPingState::Pinging => {
                        match server.cached_pinging_frame {
                            Some(2) => "minecraft:server_list/pinging_2",
                            Some(3) => "minecraft:server_list/pinging_3",
                            Some(4) => "minecraft:server_list/pinging_4",
                            Some(5) => "minecraft:server_list/pinging_5",
                            Some(_) => "minecraft:server_list/pinging_1",
                            None => "minecraft:server_list/ping_1",
                        }
                    }
                    ServerPingState::Unreachable => "minecraft:server_list/unreachable",
                    ServerPingState::Incompatible => "minecraft:server_list/incompatible",
                    ServerPingState::Successful => {
                        if server.ping < 150 {
                            "minecraft:server_list/ping_5"
                        } else if server.ping < 300 {
                            "minecraft:server_list/ping_4"
                        } else if server.ping < 600 {
                            "minecraft:server_list/ping_3"
                        } else if server.ping < 1000 {
                            "minecraft:server_list/ping_2"
                        } else {
                            "minecraft:server_list/ping_1"
                        }
                    }
                }
            };
            graphics.blit(
                ResourceLocation::from_string(status_sprite),
                status_icon_x,
                status_icon_y,
                STATUS_ICON_WIDTH,
                STATUS_ICON_HEIGHT,
            );
            let mouse_in_list_viewport = mouse_y >= list_top && mouse_y < list_bottom;
            let status_text_bounds = if server.ping_state == ServerPingState::Incompatible {
                let status_w = font.measure_width(&server.version, 1.0).round() as i32;
                let status_x = status_icon_x - status_w - STATUS_ICON_SPACING;
                graphics.text(
                    ResourceLocation::from_string("minecraft:default"),
                    server.version.clone(),
                    status_x,
                    item_y + 3,
                    0xFFFF_5555u32 as i32,
                );
                Some((status_x, status_w))
            } else if server.status_text_ready {
                let status_w;
                let status_x;
                if server.players_present {
                    let online = server.players_online.to_string();
                    let slash = "/".to_string();
                    let max = server.players_max.to_string();
                    let w_online = font.measure_width(&online, 1.0).round() as i32;
                    let w_slash = font.measure_width(&slash, 1.0).round() as i32;
                    let w_max = font.measure_width(&max, 1.0).round() as i32;
                    status_w = w_online + w_slash + w_max;
                    status_x = status_icon_x - status_w - STATUS_ICON_SPACING;
                    graphics.text(
                        ResourceLocation::from_string("minecraft:default"),
                        online,
                        status_x,
                        item_y + 3,
                        0xFFAA_AAAAu32 as i32,
                    );
                    graphics.text(
                        ResourceLocation::from_string("minecraft:default"),
                        slash,
                        status_x + w_online,
                        item_y + 3,
                        0xFF55_5555u32 as i32,
                    );
                    graphics.text(
                        ResourceLocation::from_string("minecraft:default"),
                        max,
                        status_x + w_online + w_slash,
                        item_y + 3,
                        0xFFAA_AAAAu32 as i32,
                    );
                } else {
                    let unknown = crate::translate("multiplayer.status.unknown");
                    status_w = font.measure_width(&unknown, 1.0).round() as i32;
                    status_x = status_icon_x - status_w - STATUS_ICON_SPACING;
                    graphics.text(
                        ResourceLocation::from_string("minecraft:default"),
                        unknown,
                        status_x,
                        item_y + 3,
                        0xFF55_5555u32 as i32,
                    );
                }
                Some((status_x, status_w))
            } else {
                None
            };
            if mouse_in_list_viewport
                && matches!(
                    server.status_display_state,
                    ServerPingState::Incompatible | ServerPingState::Successful
                )
                && let Some((status_x, status_w)) = status_text_bounds
            {
                let hover_players = mouse_x >= status_x
                    && mouse_x <= status_x + status_w
                    && mouse_y >= item_y + 2
                    && mouse_y <= item_y + 10;
                if hover_players && !server.player_list.is_empty() {
                    hovered_tooltip = Some(server.player_list.clone());
                }
            }
            if mouse_in_list_viewport {
                let hover_ping = mouse_x >= status_icon_x
                    && mouse_x <= status_icon_x + STATUS_ICON_WIDTH
                    && mouse_y >= status_icon_y
                    && mouse_y <= status_icon_y + STATUS_ICON_HEIGHT;
                if hover_ping {
                    let tooltip_text = match server.status_display_state {
                        ServerPingState::Initial | ServerPingState::Pinging => {
                            crate::translate("multiplayer.status.pinging")
                        }
                        ServerPingState::Unreachable => {
                            crate::translate("multiplayer.status.no_connection")
                        }
                        ServerPingState::Incompatible => {
                            crate::translate("multiplayer.status.incompatible")
                        }
                        ServerPingState::Successful => {
                            let raw = crate::translate("multiplayer.status.ping");
                            raw.replace("%s", &server.ping.to_string())
                        }
                    };
                    hovered_tooltip = Some(vec![tooltip_text]);
                }
            }
        }
        let lan_y = start_y + self.servers.len() as i32 * SERVER_ROW_HEIGHT;
        if lan_y + SERVER_ROW_HEIGHT >= list_top && lan_y < list_bottom {
            if self.lan_header_selected {
                graphics.fill(
                    item_x,
                    lan_y,
                    item_x + item_w,
                    lan_y + SERVER_ROW_HEIGHT,
                    outline_color,
                );
                graphics.fill(
                    item_x + 1,
                    lan_y + 1,
                    item_x + item_w - 1,
                    lan_y + SERVER_ROW_HEIGHT - 1,
                    0xFF00_0000u32 as i32,
                );
            }
            let elapsed_ms = now_millis();
            let frames = ["O o o", "o O o", "o o O", "o O o"];
            let dots = frames[(elapsed_ms / 300) as usize % 4];
            let lan_text = crate::translate("lanServer.scanning");
            let lan_w = font.measure_width(&lan_text, 1.0).round() as i32;
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                lan_text,
                width / 2 - lan_w / 2,
                lan_y + ENTRY_CONTENT_INSET + 2,
                0xFFFF_FFFFu32 as i32,
            );
            let dots_w = font.measure_width(dots, 1.0).round() as i32;
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                dots.to_string(),
                width / 2 - dots_w / 2,
                lan_y + ENTRY_CONTENT_INSET + 17,
                0xFF80_8080u32 as i32,
            );
        }
        for (lan_idx, server) in self.lan_servers.iter().enumerate() {
            let item_y = lan_y + SERVER_ROW_HEIGHT + lan_idx as i32 * SERVER_ROW_HEIGHT;
            if item_y + SERVER_ROW_HEIGHT < list_top || item_y > list_bottom {
                continue;
            }
            let is_selected = self.selected_lan_index == Some(lan_idx);
            if is_selected {
                graphics.fill(
                    item_x,
                    item_y,
                    item_x + item_w,
                    item_y + SERVER_ROW_HEIGHT,
                    outline_color,
                );
                graphics.fill(
                    item_x + 1,
                    item_y + 1,
                    item_x + item_w - 1,
                    item_y + SERVER_ROW_HEIGHT - 1,
                    0xFF00_0000u32 as i32,
                );
            }
            let text_x = item_x + ENTRY_CONTENT_INSET + SERVER_ICON_SIZE + 3;
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                crate::translate("lanServer.title"),
                text_x,
                item_y + 3,
                0xFFFF_FFFFu32 as i32,
            );
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                server.motd.clone(),
                text_x,
                item_y + 14,
                0xFF80_8080u32 as i32,
            );
            let address_text = if self.hide_server_address {
                crate::translate("selectServer.hiddenAddress")
            } else {
                server.address.clone()
            };
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                address_text,
                text_x,
                item_y + 25,
                0xFF80_8080u32 as i32,
            );
        }
        graphics.disable_scissor();
        let header_sep = if self.in_world {
            "minecraft:gui/inworld_header_separator"
        } else {
            "minecraft:gui/header_separator"
        };
        let footer_sep = if self.in_world {
            "minecraft:gui/inworld_footer_separator"
        } else {
            "minecraft:gui/footer_separator"
        };
        graphics.blit(
            ResourceLocation::from_string(header_sep),
            0,
            list_top - 2,
            width,
            2,
        );
        graphics.blit(
            ResourceLocation::from_string(footer_sep),
            0,
            list_bottom,
            width,
            2,
        );
        let max_scroll = self.max_scroll_amount(height);
        if max_scroll > 0.0 {
            let scrollbar_x = width / 2 + 161;
            let scroll_area_h = list_bottom - list_top;
            let bar_h = self.scrollbar_height(height);
            let bar_y = list_top
                + (self.scroll_y * (scroll_area_h - bar_h) as f32 / max_scroll).round() as i32;
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller_background"),
                scrollbar_x,
                list_top,
                6,
                scroll_area_h,
            );
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller"),
                scrollbar_x,
                bar_y,
                6,
                bar_h,
            );
            if self.dragging_scrollbar {
                graphics.request_cursor(crate::input::events::CursorType::ResizeNs);
            } else if mouse_x >= scrollbar_x
                && mouse_x < scrollbar_x + 6
                && mouse_y >= list_top
                && mouse_y < list_bottom
            {
                graphics.request_cursor(crate::input::events::CursorType::PointingHand);
            }
        }
        let btn_y_top = height - 52;
        let btn_y_bottom = height - 28;
        let select_active = self.has_joinable_selection();
        let online_selected = self.has_online_selection();
        graphics.draw_button_with_focus(
            font,
            width / 2 - 154,
            btn_y_top,
            100,
            20,
            &crate::translate("selectServer.select"),
            select_active,
            self.focused_footer_button == Some(MultiplayerFooterButton::Select),
        );
        graphics.draw_button_with_focus(
            font,
            width / 2 - 50,
            btn_y_top,
            100,
            20,
            &crate::translate("selectServer.direct"),
            true,
            self.focused_footer_button == Some(MultiplayerFooterButton::DirectConnect),
        );
        graphics.draw_button_with_focus(
            font,
            width / 2 + 54,
            btn_y_top,
            100,
            20,
            &crate::translate("selectServer.add"),
            true,
            self.focused_footer_button == Some(MultiplayerFooterButton::AddServer),
        );
        graphics.draw_button_with_focus(
            font,
            width / 2 - 154,
            btn_y_bottom,
            74,
            20,
            &crate::translate("selectServer.edit"),
            online_selected,
            self.focused_footer_button == Some(MultiplayerFooterButton::Edit),
        );
        graphics.draw_button_with_focus(
            font,
            width / 2 - 76,
            btn_y_bottom,
            74,
            20,
            &crate::translate("selectServer.delete"),
            online_selected,
            self.focused_footer_button == Some(MultiplayerFooterButton::Delete),
        );
        graphics.draw_button_with_focus(
            font,
            width / 2 + 2,
            btn_y_bottom,
            74,
            20,
            &crate::translate("selectServer.refresh"),
            true,
            self.focused_footer_button == Some(MultiplayerFooterButton::Refresh),
        );
        graphics.draw_button_with_focus(
            font,
            width / 2 + 80,
            btn_y_bottom,
            74,
            20,
            &crate::translate("gui.back"),
            true,
            self.focused_footer_button == Some(MultiplayerFooterButton::Back),
        );
        if let Some(lines) = hovered_tooltip {
            graphics.draw_tooltip(font, &lines, mouse_x, mouse_y);
        }
    }
}
impl Clone for MultiplayerScreen {
    fn clone(&self) -> Self {
        let mut cloned = Self::new(self.servers.clone(), self.in_world, Arc::clone(&self.font));
        cloned.lan_servers = self.lan_servers.clone();
        cloned.scroll_y = self.scroll_y;
        cloned.dragging_scrollbar = self.dragging_scrollbar;
        cloned.selected_index = self.selected_index;
        cloned.selected_lan_index = self.selected_lan_index;
        cloned.lan_header_selected = self.lan_header_selected;
        cloned.width = self.width;
        cloned.height = self.height;
        cloned.hide_server_address = self.hide_server_address;
        cloned.last_click_ms = self.last_click_ms;
        cloned.list_focused = self.list_focused;
        cloned.focused_footer_button = self.focused_footer_button;
        cloned.server_icons = self.server_icons.clone();
        cloned.pending_server_icons = self.pending_server_icons.clone();
        cloned.pending_ping_entries = self.pending_ping_entries.clone();
        if self.width > 0 && self.height > 0 {
            cloned.ensure_layout(self.width, self.height);
        }
        cloned
    }
}
impl std::fmt::Debug for MultiplayerScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiplayerScreen")
            .field("in_world", &self.in_world)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("servers_count", &self.servers.len())
            .field("lan_servers_count", &self.lan_servers.len())
            .field("selected_index", &self.selected_index)
            .field("selected_lan_index", &self.selected_lan_index)
            .field("scroll_y", &self.scroll_y)
            .finish()
    }
}
impl Screen for MultiplayerScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        self.ensure_layout(width, height);
    }
    fn reposition_elements(&mut self) {
        self.init();
    }
    fn tick(&mut self) {
        self.base.tick();
    }
    fn on_close(&mut self) {
        self.base.request_close();
    }
    fn should_close_on_esc(&self) -> bool {
        true
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if screen_key_pressed(self, event) {
            return true;
        }
        if event.key == 294 {
            self.refresh_key_requested = true;
            return true;
        }
        false
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        if self.base.mouse_clicked(event, double_click) {
            return true;
        }
        let action = MultiplayerScreen::mouse_clicked(
            self,
            event.x as f32,
            event.y as f32,
            self.base.width,
            self.base.height,
        );
        if let Some(action) = action {
            *self.action_sink.borrow_mut() = Some(action);
            true
        } else {
            event.x >= 0.0
                && event.x < self.base.width as f64
                && event.y >= 0.0
                && event.y < self.base.height as f64
        }
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        MultiplayerScreen::mouse_released(self);
        self.base.mouse_released(event)
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        if self.dragging_scrollbar {
            MultiplayerScreen::mouse_dragged(
                self,
                event.x as f32,
                event.y as f32,
                dy as f32,
                self.base.width,
                self.base.height,
            );
            return true;
        }
        self.base.mouse_dragged(event, dx, dy)
    }
    fn mouse_scrolled(&mut self, _x: f64, y: f64, _scroll_x: f64, scroll_y: f64) -> bool {
        if y >= HEADER_HEIGHT as f64 && y < (self.base.height - FOOTER_HEIGHT) as f64 {
            MultiplayerScreen::mouse_scrolled(self, scroll_y as f32, self.base.height);
            return true;
        }
        self.base.mouse_scrolled(_x, y, _scroll_x, scroll_y)
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
        let font = Arc::clone(&self.font);
        MultiplayerScreen::extract_render_state(self, graphics, font.as_ref());
    }
}
