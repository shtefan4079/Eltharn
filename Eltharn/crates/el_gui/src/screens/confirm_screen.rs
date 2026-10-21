use crate::FontWidthMeasurer;
use crate::components::text_widget::{LINE_HEIGHT, MultiLineTextWidget, StringWidget};
use crate::components::{
    Button, DEFAULT_HEIGHT, DEFAULT_WIDTH, GuiEventListener, Renderable, TabOrderedElement,
    WidgetClickSound,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{
    NarratableEntry, NarratedElementType, NarrationElementOutput, NarrationPriority,
};
use crate::framework::navigation::{
    ComponentPath, FocusNavigationEvent, ScreenDirection, ScreenRectangle,
};
use crate::framework::screen::{Screen, ScreenBase, screen_init};
use crate::input::events::{CharacterEvent, KeyEvent, MouseButtonEvent, PreeditEvent};
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
pub const MESSAGE_MAX_ROWS: i32 = 15;
pub const MESSAGE_WIDTH_MARGIN: i32 = 25;
pub const MESSAGE_ROW_HEIGHT: i32 = LINE_HEIGHT;
pub const BASE_HEIGHT: i32 = 61;
pub const WARNING_BASE_HEIGHT: i32 = 78;
pub const TITLE_OFFSET_Y: i32 = 0;
pub const MESSAGE_OFFSET_Y: i32 = 17;
pub const WARNING_OFFSET_Y: i32 = 25;
pub const BUTTON_OFFSET_Y: i32 = 41;
pub const WARNING_BUTTON_OFFSET_Y: i32 = 58;
pub const TWO_BUTTON_WIDTH: i32 = DEFAULT_WIDTH;
pub const TWO_BUTTON_X_OFFSETS: [i32; 2] = [-152, 2];
pub const LINK_BUTTON_WIDTH: i32 = 100;
pub const LINK_BUTTON_X_OFFSETS: [i32; 3] = [-155, -50, 55];
pub const BUTTON_HEIGHT: i32 = DEFAULT_HEIGHT;
pub const WARNING_COLOR: i32 = 0xFFFF_5555u32 as i32;
pub const TEXT_COLOR: i32 = 0xFFFF_FFFFu32 as i32;
pub const NARRATION_SEPARATOR: &str = ". ";
const YES_KEY: &str = "gui.yes";
const NO_KEY: &str = "gui.no";
const PROCEED_KEY: &str = "gui.proceed";
const DISCONNECT_KEY: &str = "menu.disconnect";
const CANCEL_KEY: &str = "gui.cancel";
const OPEN_KEY: &str = "chat.link.open";
const COPY_KEY: &str = "chat.copy";
const WARNING_KEY: &str = "chat.link.warning";
const CONFIRM_KEY: &str = "chat.link.confirm";
const CONFIRM_TRUSTED_KEY: &str = "chat.link.confirmTrusted";
const PACK_LINE1_OPTIONAL: &str = "multiplayer.texturePrompt.line1";
const PACK_LINE1_REQUIRED: &str = "multiplayer.requiredTexturePrompt.line1";
const PACK_LINE2_OPTIONAL: &str = "multiplayer.texturePrompt.line2";
const PACK_LINE2_REQUIRED: &str = "multiplayer.requiredTexturePrompt.line2";
fn pack_translation(key: &str, fallback: &str) -> String {
    let translated = crate::translate(key);
    if translated == key {
        fallback.to_string()
    } else {
        translated
    }
}
fn pack_message(required: bool, prompt: Option<&str>) -> String {
    let header = if required {
        let text = pack_translation(
            PACK_LINE2_REQUIRED,
            "Rejecting this custom resource pack will disconnect you from this server.",
        );
        format!("\u{1b}[38;2;255;255;85m\u{1b}[1m{text}\u{1b}[m")
    } else {
        pack_translation(
            PACK_LINE2_OPTIONAL,
            "Would you like to download and install it automagically?",
        )
    };
    match prompt {
        None => header,
        Some(prompt) => {
            let template = pack_translation(
                "multiplayer.texturePrompt.serverPrompt",
                "%s\n\nMessage from server:\n%s",
            );
            format_pack_prompt(&template, &header, prompt)
        }
    }
}
fn format_pack_prompt(template: &str, header: &str, prompt: &str) -> String {
    let arguments = [header, prompt];
    let mut next_argument = 0;
    let mut output = String::new();
    let mut rest = template;
    while let Some(percent) = rest.find('%') {
        output.push_str(&rest[..percent]);
        rest = &rest[percent..];
        if let Some(next) = rest.strip_prefix("%%") {
            output.push('%');
            rest = next;
        } else if let Some(next) = rest.strip_prefix("%1$s") {
            output.push_str(header);
            rest = next;
        } else if let Some(next) = rest.strip_prefix("%2$s") {
            output.push_str(prompt);
            rest = next;
        } else if let Some(next) = rest.strip_prefix("%s") {
            output.push_str(arguments.get(next_argument).copied().unwrap_or("%s"));
            next_argument += 1;
            rest = next;
        } else {
            output.push('%');
            rest = &rest[1..];
        }
    }
    output.push_str(rest);
    output
}
fn pack_title(required: bool) -> String {
    if required {
        pack_translation(
            PACK_LINE1_REQUIRED,
            "This server requires the use of a custom resource pack.",
        )
    } else {
        pack_translation(
            PACK_LINE1_OPTIONAL,
            "This server recommends the use of a custom resource pack.",
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfirmContext {
    DeleteServer(usize),
    ResourcePackIncompatible(String),
    OpenLink(String),
    ServerResourcePack {
        id: uuid::Uuid,
        url: String,
        hash: String,
        required: bool,
    },
    DeathScreen,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmAction {
    Confirm(bool),
    CopyToClipboard,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmLinkAction {
    Open,
    CopyToClipboard,
    Cancel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackConfirmAction {
    Proceed,
    Decline,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ConfirmRows {
    start_y: i32,
    message_y: i32,
    warning_y: i32,
    button_y: i32,
}
fn confirm_rows(height: i32, message_height: i32, show_warning: bool) -> ConfirmRows {
    let block = if show_warning {
        WARNING_BASE_HEIGHT
    } else {
        BASE_HEIGHT
    };
    let start_y = (height - (block + message_height)) / 2;
    let button_offset = if show_warning {
        WARNING_BUTTON_OFFSET_Y
    } else {
        BUTTON_OFFSET_Y
    };
    ConfirmRows {
        start_y,
        message_y: start_y + MESSAGE_OFFSET_Y,
        warning_y: start_y + WARNING_OFFSET_Y + message_height,
        button_y: start_y + button_offset + message_height,
    }
}
fn build_button<A: Clone + PartialEq + std::fmt::Debug>(
    label: String,
    action: A,
    font: &Arc<dyn FontWidthMeasurer>,
    width: i32,
) -> Button<A> {
    Button::<A>::builder(Component::literal(&label))
        .bounds(0, 0, width, BUTTON_HEIGHT)
        .with_action(action)
        .with_font(Arc::clone(font))
        .build()
}
fn build_title_row(label: String, font: &Arc<dyn FontWidthMeasurer>) -> StringWidget {
    let message = Component::literal(&label);
    let mut row =
        StringWidget::new(0, 0, 0, MESSAGE_ROW_HEIGHT, message.clone()).with_font(Arc::clone(font));
    row.set_message(message);
    row
}
fn build_warning_row(label: String, font: &Arc<dyn FontWidthMeasurer>) -> StringWidget {
    let mut row = build_title_row(label, font);
    row.color = WARNING_COLOR;
    row
}
fn build_message_block(
    message: String,
    font: &Arc<dyn FontWidthMeasurer>,
    width: i32,
) -> MultiLineTextWidget {
    MultiLineTextWidget::new(0, 0, Component::literal(&message))
        .with_font(Arc::clone(font))
        .tap_max_width((width - 2 * MESSAGE_WIDTH_MARGIN).max(1))
        .tap_max_rows(MESSAGE_MAX_ROWS)
        .tap_centered(true)
}
trait MessageBlockExt {
    fn tap_max_width(self, max_width: i32) -> Self;
    fn tap_max_rows(self, max_rows: i32) -> Self;
    fn tap_centered(self, centered: bool) -> Self;
}
impl MessageBlockExt for MultiLineTextWidget {
    fn tap_max_width(mut self, max_width: i32) -> Self {
        self.set_max_width(max_width);
        self
    }
    fn tap_max_rows(mut self, max_rows: i32) -> Self {
        self.set_max_rows(max_rows);
        self
    }
    fn tap_centered(mut self, centered: bool) -> Self {
        self.set_centered(centered);
        self
    }
}
pub struct ConfirmScreen {
    base: ScreenBase,
    pub in_world: bool,
    pub title: String,
    pub message: String,
    pub yes_button_text: String,
    pub no_button_text: String,
    pub copy_button_text: Option<String>,
    pub show_warning: bool,
    pub warning_text: String,
    pub delay_ticker: i32,
    pub context: ConfirmContext,
    death_screen_background: bool,
    font: Arc<dyn FontWidthMeasurer>,
    title_row: StringWidget,
    message_block: MultiLineTextWidget,
    warning_row: Option<StringWidget>,
    yes_button: Rc<RefCell<Button<ConfirmAction>>>,
    no_button: Rc<RefCell<Button<ConfirmAction>>>,
}
impl ConfirmScreen {
    pub fn new(
        in_world: bool,
        title: String,
        message: String,
        yes_button_text: String,
        no_button_text: String,
        context: ConfirmContext,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let yes_button = Rc::new(RefCell::new(build_button(
            crate::translate(&yes_button_text),
            ConfirmAction::Confirm(true),
            &font,
            TWO_BUTTON_WIDTH,
        )));
        let no_button = Rc::new(RefCell::new(build_button(
            crate::translate(&no_button_text),
            ConfirmAction::Confirm(false),
            &font,
            TWO_BUTTON_WIDTH,
        )));
        let mut base = ScreenBase::new(Component::literal(&crate::translate(&title)));
        base.set_close_on_esc(true);
        let mut screen = Self {
            base,
            in_world,
            title,
            message,
            yes_button_text,
            no_button_text,
            copy_button_text: None,
            show_warning: false,
            warning_text: WARNING_KEY.to_string(),
            delay_ticker: 0,
            context,
            death_screen_background: false,
            font,
            title_row: StringWidget::new(0, 0, 0, MESSAGE_ROW_HEIGHT, Component::literal("")),
            message_block: MultiLineTextWidget::new(0, 0, Component::literal("")),
            warning_row: None,
            yes_button,
            no_button,
        };
        screen.build_rows();
        screen
    }
    pub fn with_delay(mut self, delay: i32) -> Self {
        self.delay_ticker = delay;
        self.sync_delay_state();
        self
    }
    pub fn with_death_screen_background(mut self) -> Self {
        self.death_screen_background = true;
        self
    }
    pub const fn uses_death_screen_background(&self) -> bool {
        self.death_screen_background
    }
    fn build_rows(&mut self) {
        let font = Arc::clone(&self.font);
        self.title_row = build_title_row(crate::translate(&self.title), &font);
        self.message_block = build_message_block(
            crate::translate(&self.message),
            &font,
            self.base.width.max(1),
        );
        self.warning_row = self
            .show_warning
            .then(|| build_warning_row(crate::translate(&self.warning_text), &font));
    }
    fn rows(&self) -> ConfirmRows {
        confirm_rows(
            self.base.height,
            self.message_block.get_height(),
            self.show_warning,
        )
    }
    fn arrange(&mut self) {
        let width = self.base.width;
        let center_x = width / 2;
        let rows = self.rows();
        let title_width = self.title_row.get_width();
        self.title_row
            .set_position(center_x - title_width / 2, rows.start_y);
        let message_width = self.message_block.get_width();
        self.message_block
            .set_position(center_x - message_width / 2, rows.message_y);
        if let Some(warning) = self.warning_row.as_mut() {
            let warning_width = warning.get_width();
            warning.set_position(center_x - warning_width / 2, rows.warning_y);
        }
        let offsets = TWO_BUTTON_X_OFFSETS;
        {
            let mut button = self.yes_button.borrow_mut();
            button.set_position(center_x + offsets[0], rows.button_y);
        }
        {
            let mut button = self.no_button.borrow_mut();
            button.set_position(center_x + offsets[1], rows.button_y);
        }
    }
    fn sync_delay_state(&mut self) {
        let active = self.delay_ticker <= 0;
        for button in [&self.yes_button, &self.no_button] {
            if let Ok(mut button) = button.try_borrow_mut() {
                button.base.active = active;
            }
        }
        self.base.set_close_on_esc(active);
    }
    pub fn take_action(&mut self) -> Option<ConfirmAction> {
        for button in [&self.yes_button, &self.no_button] {
            if let Ok(mut button) = button.try_borrow_mut() {
                if let Some(action) = button.take_press_action() {
                    return Some(action);
                }
            }
        }
        None
    }
    pub fn tick(&mut self) {
        self.advance_delay();
    }
    pub fn should_close_on_esc(&self) -> bool {
        self.esc_allowed()
    }
    fn advance_delay(&mut self) {
        if self.delay_ticker > 0 {
            self.delay_ticker -= 1;
        }
        self.sync_delay_state();
    }
    fn esc_allowed(&self) -> bool {
        self.delay_ticker <= 0
    }
}
impl Screen for ConfirmScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        self.build_rows();
        self.arrange();
        self.base
            .add_widget(SharedButton::new(Rc::clone(&self.yes_button)));
        self.base
            .add_widget(SharedButton::new(Rc::clone(&self.no_button)));
    }
    fn tick(&mut self) {
        self.advance_delay();
    }
    fn should_close_on_esc(&self) -> bool {
        self.esc_allowed()
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        let width = self.base.width;
        let height = self.base.height;
        if self.death_screen_background {
            crate::screens::death_screen::DeathScreen::extract_death_background(
                graphics, width, height,
            );
        } else {
            self.base.extract_background_layers(graphics, self.in_world);
        }
        Renderable::extract_render_state(
            &mut self.title_row,
            graphics,
            mouse_x,
            mouse_y,
            partial_tick,
        );
        Renderable::extract_render_state(
            &mut self.message_block,
            graphics,
            mouse_x,
            mouse_y,
            partial_tick,
        );
        if let Some(warning) = self.warning_row.as_mut() {
            Renderable::extract_render_state(warning, graphics, mouse_x, mouse_y, partial_tick);
        }
        self.base
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
    }
    fn update_narration(&mut self, output: &mut NarrationElementOutput) {
        self.base.update_narration(output);
        let joined = format!(
            "{}{}{}",
            crate::translate(&self.title),
            NARRATION_SEPARATOR,
            crate::translate(&self.message)
        );
        output.add(NarratedElementType::Hint, joined);
    }
}
impl Clone for ConfirmScreen {
    fn clone(&self) -> Self {
        let mut cloned = Self::new(
            self.in_world,
            self.title.clone(),
            self.message.clone(),
            self.yes_button_text.clone(),
            self.no_button_text.clone(),
            self.context.clone(),
            Arc::clone(&self.font),
        );
        cloned.copy_button_text = self.copy_button_text.clone();
        cloned.show_warning = self.show_warning;
        cloned.warning_text = self.warning_text.clone();
        cloned.delay_ticker = self.delay_ticker;
        cloned.death_screen_background = self.death_screen_background;
        let (width, height) = (self.base.width, self.base.height);
        screen_init(&mut cloned, width, height);
        cloned
    }
}
impl std::fmt::Debug for ConfirmScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfirmScreen")
            .field("in_world", &self.in_world)
            .field("title", &self.title)
            .field("message", &self.message)
            .field("show_warning", &self.show_warning)
            .field("delay_ticker", &self.delay_ticker)
            .field("death_screen_background", &self.death_screen_background)
            .field("width", &self.base.width)
            .field("height", &self.base.height)
            .field("widgets", &self.base.widgets_len())
            .field("focused", &self.base.focused_index())
            .finish()
    }
}
pub(crate) struct SharedButton<A: Clone + PartialEq + std::fmt::Debug> {
    button: Rc<RefCell<Button<A>>>,
}
impl<A: Clone + PartialEq + std::fmt::Debug> SharedButton<A> {
    pub(crate) fn new(button: Rc<RefCell<Button<A>>>) -> Self {
        Self { button }
    }
}
impl<A: Clone + PartialEq + std::fmt::Debug> crate::components::Renderable for SharedButton<A> {
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        if let Ok(mut button) = self.button.try_borrow_mut() {
            button.extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
        }
    }
}
impl<A: Clone + PartialEq + std::fmt::Debug> TabOrderedElement for SharedButton<A> {
    fn tab_order_group(&self) -> i32 {
        match self.button.try_borrow() {
            Ok(button) => button.tab_order_group(),
            Err(_) => 0,
        }
    }
}
impl<A: Clone + PartialEq + std::fmt::Debug> GuiEventListener for SharedButton<A> {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        match self.button.try_borrow_mut() {
            Ok(mut button) => button.mouse_clicked(event, double_click),
            Err(_) => false,
        }
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        match self.button.try_borrow_mut() {
            Ok(mut button) => button.key_pressed(event),
            Err(_) => false,
        }
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        match self.button.try_borrow() {
            Ok(button) => button.play_down_sound(),
            Err(_) => None,
        }
    }
    fn mouse_moved(&mut self, x: f64, y: f64) {
        if let Ok(mut button) = self.button.try_borrow_mut() {
            button.mouse_moved(x, y);
        }
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        match self.button.try_borrow_mut() {
            Ok(mut button) => button.mouse_released(event),
            Err(_) => false,
        }
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        match self.button.try_borrow_mut() {
            Ok(mut button) => button.mouse_dragged(event, dx, dy),
            Err(_) => false,
        }
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        match self.button.try_borrow_mut() {
            Ok(mut button) => button.mouse_scrolled(x, y, scroll_x, scroll_y),
            Err(_) => false,
        }
    }
    fn key_released(&mut self, event: KeyEvent) -> bool {
        match self.button.try_borrow_mut() {
            Ok(mut button) => button.key_released(event),
            Err(_) => false,
        }
    }
    fn char_typed(&mut self, event: CharacterEvent) -> bool {
        match self.button.try_borrow_mut() {
            Ok(mut button) => button.char_typed(event),
            Err(_) => false,
        }
    }
    fn preedit_updated(&mut self, event: Option<PreeditEvent>) -> bool {
        match self.button.try_borrow_mut() {
            Ok(mut button) => button.preedit_updated(event),
            Err(_) => false,
        }
    }
    fn next_focus_path(&self, navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        match self.button.try_borrow() {
            Ok(button) => GuiEventListener::next_focus_path(&*button, navigation_event),
            Err(_) => None,
        }
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        match self.button.try_borrow() {
            Ok(button) => button.is_mouse_over(mouse_x, mouse_y),
            Err(_) => false,
        }
    }
    fn set_focused(&mut self, focused: bool) {
        if let Ok(mut button) = self.button.try_borrow_mut() {
            button.set_focused(focused);
        }
    }
    fn is_focused(&self) -> bool {
        match self.button.try_borrow() {
            Ok(button) => button.is_focused(),
            Err(_) => false,
        }
    }
    fn should_take_focus_after_interaction(&self) -> bool {
        match self.button.try_borrow() {
            Ok(button) => button.should_take_focus_after_interaction(),
            Err(_) => true,
        }
    }
    fn rectangle(&self) -> ScreenRectangle {
        match self.button.try_borrow() {
            Ok(button) => GuiEventListener::rectangle(&*button),
            Err(_) => ScreenRectangle::empty(),
        }
    }
    fn border_for_arrow_navigation(&self, opposite: ScreenDirection) -> ScreenRectangle {
        match self.button.try_borrow() {
            Ok(button) => button.border_for_arrow_navigation(opposite),
            Err(_) => ScreenRectangle::empty(),
        }
    }
}
impl<A: Clone + PartialEq + std::fmt::Debug> NarratableEntry for SharedButton<A> {
    fn narration_priority(&self) -> NarrationPriority {
        match self.button.try_borrow() {
            Ok(button) => button.narration_priority(),
            Err(_) => NarrationPriority::None,
        }
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        if let Ok(button) = self.button.try_borrow() {
            button.update_narration(output);
        }
    }
}
impl<A: Clone + PartialEq + std::fmt::Debug> LayoutElement for SharedButton<A> {
    fn set_x(&mut self, x: i32) {
        if let Ok(mut button) = self.button.try_borrow_mut() {
            button.set_x(x);
        }
    }
    fn set_y(&mut self, y: i32) {
        if let Ok(mut button) = self.button.try_borrow_mut() {
            button.set_y(y);
        }
    }
    fn x(&self) -> i32 {
        match self.button.try_borrow() {
            Ok(button) => button.x(),
            Err(_) => 0,
        }
    }
    fn y(&self) -> i32 {
        match self.button.try_borrow() {
            Ok(button) => button.y(),
            Err(_) => 0,
        }
    }
    fn width(&self) -> i32 {
        match self.button.try_borrow() {
            Ok(button) => button.width(),
            Err(_) => 0,
        }
    }
    fn height(&self) -> i32 {
        match self.button.try_borrow() {
            Ok(button) => button.height(),
            Err(_) => 0,
        }
    }
}
pub struct ConfirmLinkScreen {
    base: ScreenBase,
    pub in_world: bool,
    pub title: String,
    pub message: String,
    pub yes_button_text: String,
    pub no_button_text: String,
    pub copy_button_text: Option<String>,
    pub show_warning: bool,
    pub warning_text: String,
    pub delay_ticker: i32,
    pub url: String,
    pub trusted: bool,
    font: Arc<dyn FontWidthMeasurer>,
    title_row: StringWidget,
    message_block: MultiLineTextWidget,
    warning_row: Option<StringWidget>,
    yes_button: Rc<RefCell<Button<ConfirmLinkAction>>>,
    copy_button: Rc<RefCell<Button<ConfirmLinkAction>>>,
    no_button: Rc<RefCell<Button<ConfirmLinkAction>>>,
}
impl ConfirmLinkScreen {
    pub fn new(
        in_world: bool,
        url: String,
        trusted: bool,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let (title, no_button_text) = if trusted {
            (CONFIRM_TRUSTED_KEY.to_string(), CANCEL_KEY.to_string())
        } else {
            (CONFIRM_KEY.to_string(), NO_KEY.to_string())
        };
        let yes_button_text = OPEN_KEY.to_string();
        let copy_button_text = Some(COPY_KEY.to_string());
        let show_warning = !trusted;
        let warning_text = WARNING_KEY.to_string();
        let yes_button = Rc::new(RefCell::new(build_button(
            crate::translate(&yes_button_text),
            ConfirmLinkAction::Open,
            &font,
            LINK_BUTTON_WIDTH,
        )));
        let copy_button = Rc::new(RefCell::new(build_button(
            crate::translate(copy_button_text.as_deref().unwrap_or_default()),
            ConfirmLinkAction::CopyToClipboard,
            &font,
            LINK_BUTTON_WIDTH,
        )));
        let no_button = Rc::new(RefCell::new(build_button(
            crate::translate(&no_button_text),
            ConfirmLinkAction::Cancel,
            &font,
            LINK_BUTTON_WIDTH,
        )));
        let mut base = ScreenBase::new(Component::literal(&crate::translate(&title)));
        base.set_close_on_esc(true);
        let mut screen = Self {
            base,
            in_world,
            title,
            message: url.clone(),
            yes_button_text,
            no_button_text,
            copy_button_text,
            show_warning,
            warning_text,
            delay_ticker: 0,
            url,
            trusted,
            font,
            title_row: StringWidget::new(0, 0, 0, MESSAGE_ROW_HEIGHT, Component::literal("")),
            message_block: MultiLineTextWidget::new(0, 0, Component::literal("")),
            warning_row: None,
            yes_button,
            copy_button,
            no_button,
        };
        screen.build_rows();
        screen
    }
    pub fn with_delay(mut self, delay: i32) -> Self {
        self.delay_ticker = delay;
        self.sync_delay_state();
        self
    }
    fn build_rows(&mut self) {
        let font = Arc::clone(&self.font);
        self.title_row = build_title_row(crate::translate(&self.title), &font);
        self.message_block = build_message_block(
            crate::translate(&self.message),
            &font,
            self.base.width.max(1),
        );
        self.warning_row = self
            .show_warning
            .then(|| build_warning_row(crate::translate(&self.warning_text), &font));
    }
    fn rows(&self) -> ConfirmRows {
        confirm_rows(
            self.base.height,
            self.message_block.get_height(),
            self.show_warning,
        )
    }
    fn arrange(&mut self) {
        let width = self.base.width;
        let center_x = width / 2;
        let rows = self.rows();
        let title_width = self.title_row.get_width();
        self.title_row
            .set_position(center_x - title_width / 2, rows.start_y);
        let message_width = self.message_block.get_width();
        self.message_block
            .set_position(center_x - message_width / 2, rows.message_y);
        if let Some(warning) = self.warning_row.as_mut() {
            let warning_width = warning.get_width();
            warning.set_position(center_x - warning_width / 2, rows.warning_y);
        }
        let offsets = LINK_BUTTON_X_OFFSETS;
        let buttons = [&self.yes_button, &self.copy_button, &self.no_button];
        for (index, button) in buttons.iter().enumerate() {
            if let Ok(mut button) = button.try_borrow_mut() {
                button.set_position(center_x + offsets[index], rows.button_y);
            }
        }
    }
    fn sync_delay_state(&mut self) {
        let active = self.delay_ticker <= 0;
        for button in [&self.yes_button, &self.copy_button, &self.no_button] {
            if let Ok(mut button) = button.try_borrow_mut() {
                button.base.active = active;
            }
        }
        self.base.set_close_on_esc(active);
    }
    pub fn take_action(&mut self) -> Option<ConfirmLinkAction> {
        for button in [&self.yes_button, &self.copy_button, &self.no_button] {
            if let Ok(mut button) = button.try_borrow_mut() {
                if let Some(action) = button.take_press_action() {
                    return Some(action);
                }
            }
        }
        None
    }
    pub fn take_confirm_action(&mut self) -> Option<ConfirmAction> {
        match self.take_action()? {
            ConfirmLinkAction::Open => Some(ConfirmAction::Confirm(true)),
            ConfirmLinkAction::Cancel => Some(ConfirmAction::Confirm(false)),
            ConfirmLinkAction::CopyToClipboard => Some(ConfirmAction::CopyToClipboard),
        }
    }
    pub fn tick(&mut self) {
        self.advance_delay();
    }
    pub fn should_close_on_esc(&self) -> bool {
        self.esc_allowed()
    }
    fn advance_delay(&mut self) {
        if self.delay_ticker > 0 {
            self.delay_ticker -= 1;
        }
        self.sync_delay_state();
    }
    fn esc_allowed(&self) -> bool {
        self.delay_ticker <= 0
    }
}
impl Screen for ConfirmLinkScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        self.build_rows();
        self.arrange();
        self.base
            .add_widget(SharedButton::new(Rc::clone(&self.yes_button)));
        self.base
            .add_widget(SharedButton::new(Rc::clone(&self.copy_button)));
        self.base
            .add_widget(SharedButton::new(Rc::clone(&self.no_button)));
    }
    fn tick(&mut self) {
        self.advance_delay();
    }
    fn should_close_on_esc(&self) -> bool {
        self.esc_allowed()
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        self.base.extract_background_layers(graphics, self.in_world);
        Renderable::extract_render_state(
            &mut self.title_row,
            graphics,
            mouse_x,
            mouse_y,
            partial_tick,
        );
        Renderable::extract_render_state(
            &mut self.message_block,
            graphics,
            mouse_x,
            mouse_y,
            partial_tick,
        );
        if let Some(warning) = self.warning_row.as_mut() {
            Renderable::extract_render_state(warning, graphics, mouse_x, mouse_y, partial_tick);
        }
        self.base
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
    }
    fn update_narration(&mut self, output: &mut NarrationElementOutput) {
        self.base.update_narration(output);
        let joined = format!(
            "{}{}{}",
            crate::translate(&self.title),
            NARRATION_SEPARATOR,
            crate::translate(&self.message)
        );
        output.add(NarratedElementType::Hint, joined);
    }
}
impl Clone for ConfirmLinkScreen {
    fn clone(&self) -> Self {
        let mut cloned = Self::new(
            self.in_world,
            self.url.clone(),
            self.trusted,
            Arc::clone(&self.font),
        );
        cloned.delay_ticker = self.delay_ticker;
        let (width, height) = (self.base.width, self.base.height);
        screen_init(&mut cloned, width, height);
        cloned
    }
}
impl std::fmt::Debug for ConfirmLinkScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfirmLinkScreen")
            .field("in_world", &self.in_world)
            .field("trusted", &self.trusted)
            .field("url", &self.url)
            .field("show_warning", &self.show_warning)
            .field("delay_ticker", &self.delay_ticker)
            .field("width", &self.base.width)
            .field("height", &self.base.height)
            .field("widgets", &self.base.widgets_len())
            .field("focused", &self.base.focused_index())
            .finish()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingPackRequest {
    pub id: uuid::Uuid,
    pub url: String,
    pub hash: String,
}
pub struct PackConfirmScreen {
    base: ScreenBase,
    pub in_world: bool,
    pub title: String,
    pub message: String,
    pub yes_button_text: String,
    pub no_button_text: String,
    pub copy_button_text: Option<String>,
    pub show_warning: bool,
    pub warning_text: String,
    pub delay_ticker: i32,
    pub id: uuid::Uuid,
    pub url: String,
    pub hash: String,
    pub required: bool,
    pub prompt: Option<String>,
    pub requests: Vec<PendingPackRequest>,
    font: Arc<dyn FontWidthMeasurer>,
    title_row: StringWidget,
    message_block: MultiLineTextWidget,
    yes_button: Rc<RefCell<Button<PackConfirmAction>>>,
    no_button: Rc<RefCell<Button<PackConfirmAction>>>,
}
impl PackConfirmScreen {
    pub fn new(
        in_world: bool,
        id: uuid::Uuid,
        url: String,
        hash: String,
        required: bool,
        prompt: Option<String>,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let title = if required {
            PACK_LINE1_REQUIRED
        } else {
            PACK_LINE1_OPTIONAL
        };
        let message = pack_message(required, prompt.as_deref());
        let yes_button_text = if required { PROCEED_KEY } else { YES_KEY };
        let no_button_text = if required { DISCONNECT_KEY } else { NO_KEY };
        let yes_button = Rc::new(RefCell::new(build_button(
            crate::translate(yes_button_text),
            PackConfirmAction::Proceed,
            &font,
            TWO_BUTTON_WIDTH,
        )));
        let no_button = Rc::new(RefCell::new(build_button(
            crate::translate(no_button_text),
            PackConfirmAction::Decline,
            &font,
            TWO_BUTTON_WIDTH,
        )));
        let mut base = ScreenBase::new(Component::literal(&pack_title(required)));
        base.set_close_on_esc(true);
        let requests = vec![PendingPackRequest {
            id,
            url: url.clone(),
            hash: hash.clone(),
        }];
        let mut screen = Self {
            base,
            in_world,
            title: title.to_string(),
            message,
            yes_button_text: yes_button_text.to_string(),
            no_button_text: no_button_text.to_string(),
            copy_button_text: None,
            show_warning: false,
            warning_text: WARNING_KEY.to_string(),
            delay_ticker: 0,
            id,
            url,
            hash,
            required,
            prompt,
            requests,
            font,
            title_row: StringWidget::new(0, 0, 0, MESSAGE_ROW_HEIGHT, Component::literal("")),
            message_block: MultiLineTextWidget::new(0, 0, Component::literal("")),
            yes_button,
            no_button,
        };
        screen.build_rows();
        screen
    }
    pub fn with_delay(mut self, delay: i32) -> Self {
        self.delay_ticker = delay;
        self.sync_delay_state();
        self
    }
    fn build_rows(&mut self) {
        let font = Arc::clone(&self.font);
        self.title_row = build_title_row(pack_title(self.required), &font);
        self.message = pack_message(self.required, self.prompt.as_deref());
        self.message_block =
            build_message_block(self.message.clone(), &font, self.base.width.max(1));
    }
    fn rows(&self) -> ConfirmRows {
        confirm_rows(
            self.base.height,
            self.message_block.get_height(),
            self.show_warning,
        )
    }
    fn arrange(&mut self) {
        let width = self.base.width;
        let center_x = width / 2;
        let rows = self.rows();
        let title_width = self.title_row.get_width();
        self.title_row
            .set_position(center_x - title_width / 2, rows.start_y);
        let message_width = self.message_block.get_width();
        self.message_block
            .set_position(center_x - message_width / 2, rows.message_y);
        let offsets = TWO_BUTTON_X_OFFSETS;
        for (index, button) in [&self.yes_button, &self.no_button].iter().enumerate() {
            if let Ok(mut button) = button.try_borrow_mut() {
                button.set_position(center_x + offsets[index], rows.button_y);
            }
        }
    }
    fn sync_delay_state(&mut self) {
        let active = self.delay_ticker <= 0;
        for button in [&self.yes_button, &self.no_button] {
            if let Ok(mut button) = button.try_borrow_mut() {
                button.base.active = active;
            }
        }
        self.base.set_close_on_esc(active);
    }
    pub fn take_action(&mut self) -> Option<PackConfirmAction> {
        if self.base.take_close_request() {
            return Some(PackConfirmAction::Decline);
        }
        for button in [&self.yes_button, &self.no_button] {
            if let Ok(mut button) = button.try_borrow_mut() {
                if let Some(action) = button.take_press_action() {
                    return Some(action);
                }
            }
        }
        None
    }
    pub fn take_confirm_action(&mut self) -> Option<ConfirmAction> {
        match self.take_action()? {
            PackConfirmAction::Proceed => Some(ConfirmAction::Confirm(true)),
            PackConfirmAction::Decline => Some(ConfirmAction::Confirm(false)),
        }
    }
    pub fn tick(&mut self) {
        self.advance_delay();
    }
    pub fn should_close_on_esc(&self) -> bool {
        self.esc_allowed()
    }
    fn advance_delay(&mut self) {
        if self.delay_ticker > 0 {
            self.delay_ticker -= 1;
        }
        self.sync_delay_state();
    }
    fn esc_allowed(&self) -> bool {
        self.delay_ticker <= 0
    }
}
impl Screen for PackConfirmScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        self.build_rows();
        self.arrange();
        self.base
            .add_widget(SharedButton::new(Rc::clone(&self.yes_button)));
        self.base
            .add_widget(SharedButton::new(Rc::clone(&self.no_button)));
    }
    fn tick(&mut self) {
        self.advance_delay();
    }
    fn should_close_on_esc(&self) -> bool {
        self.esc_allowed()
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        self.base.extract_background_layers(graphics, self.in_world);
        Renderable::extract_render_state(
            &mut self.title_row,
            graphics,
            mouse_x,
            mouse_y,
            partial_tick,
        );
        Renderable::extract_render_state(
            &mut self.message_block,
            graphics,
            mouse_x,
            mouse_y,
            partial_tick,
        );
        self.base
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
    }
    fn update_narration(&mut self, output: &mut NarrationElementOutput) {
        self.base.update_narration(output);
        let joined = format!(
            "{}{}{}",
            pack_title(self.required),
            NARRATION_SEPARATOR,
            crate::formatted_text::plain(&self.message)
        );
        output.add(NarratedElementType::Hint, joined);
    }
}
impl Clone for PackConfirmScreen {
    fn clone(&self) -> Self {
        let mut cloned = Self::new(
            self.in_world,
            self.id,
            self.url.clone(),
            self.hash.clone(),
            self.required,
            self.prompt.clone(),
            Arc::clone(&self.font),
        );
        cloned.delay_ticker = self.delay_ticker;
        cloned.requests = self.requests.clone();
        let (width, height) = (self.base.width, self.base.height);
        screen_init(&mut cloned, width, height);
        cloned
    }
}
impl std::fmt::Debug for PackConfirmScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PackConfirmScreen")
            .field("in_world", &self.in_world)
            .field("required", &self.required)
            .field("id", &self.id)
            .field("delay_ticker", &self.delay_ticker)
            .field("width", &self.base.width)
            .field("height", &self.base.height)
            .field("widgets", &self.base.widgets_len())
            .field("focused", &self.base.focused_index())
            .finish()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FriendsListConfirmAction {
    Accept(bool),
}
pub const FRIENDS_TITLE_KEY: &str = "options.friendsList.confirm.title";
pub const FRIENDS_MESSAGE_KEY: &str = "options.friendsList.confirm.message";
pub const FRIENDS_MESSAGE_LINK_KEY: &str = "options.friendsList.confirm.message.link";
pub const FRIENDS_TURN_ON_KEY: &str = "options.friendsList.confirm.turnOn";
pub const FRIENDS_TURN_OFF_KEY: &str = "options.friendsList.confirm.turnOff";
pub const FRIENDS_USAGE_FOCUSED_KEY: &str = "narration.link.usage.focused";
pub const FRIENDS_USAGE_HOVERED_KEY: &str = "narration.link.usage.hovered";
pub const FRIENDS_PANEL_SPRITE: &str = "minecraft:friends/background_dark";
pub const FRIENDS_PANEL_PADDING: i32 = 10;
pub const FRIENDS_PANEL_BORDER: i32 = 8;
pub const FRIENDS_TEXT_WIDGET_PADDING: i32 = 4;
pub const FRIENDS_MESSAGE_WIDTH_MARGIN: i32 = 90;
fn friends_translation(key: &str, fallback: &str) -> String {
    let translated = crate::translate(key);
    if translated == key {
        fallback.to_string()
    } else {
        translated
    }
}
fn friends_title() -> String {
    friends_translation(FRIENDS_TITLE_KEY, "Friends List")
}
fn friends_message() -> String {
    friends_translation(
            FRIENDS_MESSAGE_KEY,
            "Exchange friend requests through the Friends List.\n\nSet online safety options using your %s.",
        )
        .replacen(
            "%s",
            &friends_translation(FRIENDS_MESSAGE_LINK_KEY, "Microsoft account"),
            1,
        )
}
pub struct FriendsListConfirmScreen {
    base: ScreenBase,
    pub in_world: bool,
    pub title: String,
    pub message: String,
    pub yes_button_text: String,
    pub no_button_text: String,
    pub copy_button_text: Option<String>,
    pub show_warning: bool,
    pub warning_text: String,
    pub delay_ticker: i32,
    font: Arc<dyn FontWidthMeasurer>,
    title_row: StringWidget,
    message_block: MultiLineTextWidget,
    yes_button: Rc<RefCell<Button<FriendsListConfirmAction>>>,
    no_button: Rc<RefCell<Button<FriendsListConfirmAction>>>,
}
impl FriendsListConfirmScreen {
    pub fn new(in_world: bool, font: Arc<dyn FontWidthMeasurer>) -> Self {
        let yes_button = Rc::new(RefCell::new(build_button(
            friends_translation(FRIENDS_TURN_ON_KEY, "Turn ON"),
            FriendsListConfirmAction::Accept(true),
            &font,
            TWO_BUTTON_WIDTH,
        )));
        let no_button = Rc::new(RefCell::new(build_button(
            friends_translation(FRIENDS_TURN_OFF_KEY, "Turn OFF"),
            FriendsListConfirmAction::Accept(false),
            &font,
            TWO_BUTTON_WIDTH,
        )));
        let mut base = ScreenBase::new(Component::literal(&friends_title()));
        base.set_close_on_esc(true);
        let mut screen = Self {
            base,
            in_world,
            title: FRIENDS_TITLE_KEY.to_string(),
            message: friends_message(),
            yes_button_text: FRIENDS_TURN_ON_KEY.to_string(),
            no_button_text: FRIENDS_TURN_OFF_KEY.to_string(),
            copy_button_text: None,
            show_warning: false,
            warning_text: WARNING_KEY.to_string(),
            delay_ticker: 0,
            font,
            title_row: StringWidget::new(0, 0, 0, MESSAGE_ROW_HEIGHT, Component::literal("")),
            message_block: MultiLineTextWidget::new(0, 0, Component::literal("")),
            yes_button,
            no_button,
        };
        screen.build_rows();
        screen
    }
    pub fn with_delay(mut self, delay: i32) -> Self {
        self.delay_ticker = delay;
        self.sync_delay_state();
        self
    }
    fn build_rows(&mut self) {
        let font = Arc::clone(&self.font);
        self.title_row = build_title_row(friends_title(), &font);
        self.message_block =
            friends_message_block(self.message.clone(), &font, self.base.width.max(1));
    }
    fn rows(&self) -> ConfirmRows {
        confirm_rows(
            self.base.height,
            self.message_block.get_height(),
            self.show_warning,
        )
    }
    fn arrange(&mut self) {
        let width = self.base.width;
        let center_x = width / 2;
        let rows = self.rows();
        let title_width = self.title_row.get_width();
        self.title_row
            .set_position(center_x - title_width / 2, rows.start_y);
        let message_width = self.message_block.get_width();
        self.message_block
            .set_position(center_x - message_width / 2, rows.message_y);
        let offsets = TWO_BUTTON_X_OFFSETS;
        for (index, button) in [&self.yes_button, &self.no_button].iter().enumerate() {
            if let Ok(mut button) = button.try_borrow_mut() {
                button.set_position(center_x + offsets[index], rows.button_y);
            }
        }
    }
    fn panel_rect(&self) -> (i32, i32, i32, i32) {
        let rows = self.rows();
        let center_x = self.base.width / 2;
        let wide = (TWO_BUTTON_X_OFFSETS[1] + TWO_BUTTON_WIDTH - TWO_BUTTON_X_OFFSETS[0])
            .max(self.title_row.get_width())
            .max(self.message_block.get_width() + 2 * FRIENDS_PANEL_PADDING);
        let left = center_x - wide / 2;
        let top = rows.start_y;
        let height = rows.button_y + BUTTON_HEIGHT - top;
        let x = left - FRIENDS_PANEL_PADDING - FRIENDS_PANEL_BORDER;
        let y = top - FRIENDS_PANEL_PADDING - FRIENDS_PANEL_BORDER;
        (
            x,
            y,
            wide + 2 * FRIENDS_PANEL_PADDING + 2 * FRIENDS_PANEL_BORDER + 1,
            height + 2 * FRIENDS_PANEL_PADDING + 2 * FRIENDS_PANEL_BORDER + 1,
        )
    }
    fn sync_delay_state(&mut self) {
        let active = self.delay_ticker <= 0;
        for button in [&self.yes_button, &self.no_button] {
            if let Ok(mut button) = button.try_borrow_mut() {
                button.base.active = active;
            }
        }
        self.base.set_close_on_esc(active);
    }
    pub fn take_action(&mut self) -> Option<FriendsListConfirmAction> {
        if self.base.take_close_request() {
            return Some(FriendsListConfirmAction::Accept(false));
        }
        for button in [&self.yes_button, &self.no_button] {
            if let Ok(mut button) = button.try_borrow_mut() {
                if let Some(action) = button.take_press_action() {
                    return Some(action);
                }
            }
        }
        None
    }
    pub fn tick(&mut self) {
        self.advance_delay();
    }
    pub fn should_close_on_esc(&self) -> bool {
        self.esc_allowed()
    }
    fn advance_delay(&mut self) {
        if self.delay_ticker > 0 {
            self.delay_ticker -= 1;
        }
        self.sync_delay_state();
    }
    fn esc_allowed(&self) -> bool {
        self.delay_ticker <= 0
    }
}
fn friends_message_block(
    message: String,
    font: &Arc<dyn FontWidthMeasurer>,
    width: i32,
) -> MultiLineTextWidget {
    let bound = (width - 2 * FRIENDS_MESSAGE_WIDTH_MARGIN - 2 * FRIENDS_TEXT_WIDGET_PADDING).max(1);
    MultiLineTextWidget::new(0, 0, Component::literal(&message))
        .with_font(Arc::clone(font))
        .tap_max_width(bound)
        .tap_max_rows(MESSAGE_MAX_ROWS)
        .tap_centered(true)
}
impl Screen for FriendsListConfirmScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        self.build_rows();
        self.arrange();
        self.base
            .add_widget(SharedButton::new(Rc::clone(&self.yes_button)));
        self.base
            .add_widget(SharedButton::new(Rc::clone(&self.no_button)));
    }
    fn tick(&mut self) {
        self.advance_delay();
    }
    fn should_close_on_esc(&self) -> bool {
        self.esc_allowed()
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        self.base.extract_background_layers(graphics, self.in_world);
        let (x, y, panel_width, panel_height) = self.panel_rect();
        graphics.blit(
            ResourceLocation::from_string(FRIENDS_PANEL_SPRITE),
            x,
            y,
            panel_width,
            panel_height,
        );
        Renderable::extract_render_state(
            &mut self.title_row,
            graphics,
            mouse_x,
            mouse_y,
            partial_tick,
        );
        Renderable::extract_render_state(
            &mut self.message_block,
            graphics,
            mouse_x,
            mouse_y,
            partial_tick,
        );
        self.base
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
    }
    fn update_narration(&mut self, output: &mut NarrationElementOutput) {
        self.base.update_narration(output);
        let joined = format!(
            "{}{}{}",
            friends_title(),
            NARRATION_SEPARATOR,
            crate::formatted_text::plain(&self.message)
        );
        output.add(NarratedElementType::Hint, joined);
    }
}
impl Clone for FriendsListConfirmScreen {
    fn clone(&self) -> Self {
        let mut cloned = Self::new(self.in_world, Arc::clone(&self.font));
        cloned.delay_ticker = self.delay_ticker;
        let (width, height) = (self.base.width, self.base.height);
        screen_init(&mut cloned, width, height);
        cloned
    }
}
impl std::fmt::Debug for FriendsListConfirmScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FriendsListConfirmScreen")
            .field("in_world", &self.in_world)
            .field("title", &self.title)
            .field("delay_ticker", &self.delay_ticker)
            .field("width", &self.base.width)
            .field("height", &self.base.height)
            .field("widgets", &self.base.widgets_len())
            .field("focused", &self.base.focused_index())
            .finish()
    }
}
