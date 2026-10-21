use crate::FontWidthMeasurer;
use crate::components::{
    Button, GuiEventListener, Renderable, TabOrderedElement, WidgetClickSound,
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
pub const CANCEL_X_OFFSET: i32 = -100;
pub const CANCEL_BASE_Y: i32 = 120;
pub const CANCEL_GAP_Y: i32 = 12;
pub const CANCEL_WIDTH: i32 = 200;
pub const CANCEL_HEIGHT: i32 = 20;
pub const STATUS_OFFSET_Y: i32 = -50;
pub const DETAIL_OFFSET_Y: i32 = -30;
pub const STATUS_COLOR: i32 = 0xFFFF_FFFFu32 as i32;
pub const DETAIL_COLOR: i32 = 0xFFA0_A0A0u32 as i32;
pub const NARRATION_DELAY_MS: i64 = 2000;
const DEFAULT_STATUS_KEY: &str = "connect.connecting";
const CANCEL_MESSAGE_KEY: &str = "gui.cancel";
const JOINING_NARRATION_KEY: &str = "narrator.joining";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectingAction {
    Cancel,
}
pub struct ConnectingScreen {
    base: ScreenBase,
    in_world: bool,
    pub server_name: String,
    pub server_ip: String,
    status: String,
    detail: Option<String>,
    font: Arc<dyn FontWidthMeasurer>,
    cancel: Rc<RefCell<Button<ConnectingAction>>>,
    last_narration_ms: i64,
}
impl ConnectingScreen {
    pub fn new(
        in_world: bool,
        server_name: String,
        server_ip: String,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let cancel = Rc::new(RefCell::new(
            Button::<ConnectingAction>::builder(Component::literal(&crate::translate(
                CANCEL_MESSAGE_KEY,
            )))
            .bounds(0, 0, CANCEL_WIDTH, CANCEL_HEIGHT)
            .with_action(ConnectingAction::Cancel)
            .with_font(Arc::clone(&font))
            .build(),
        ));
        let mut base = ScreenBase::new(Component::literal(""));
        base.set_close_on_esc(false);
        Self {
            base,
            in_world,
            server_name,
            server_ip,
            status: DEFAULT_STATUS_KEY.to_string(),
            detail: None,
            font,
            cancel,
            last_narration_ms: -1,
        }
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn set_status(&mut self, status_key: &str, param: Option<String>) {
        self.status = status_key.to_string();
        self.detail = param;
    }
    pub fn status_key(&self) -> &str {
        &self.status
    }
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }
    pub fn cancel_x(width: i32) -> i32 {
        width / 2 + CANCEL_X_OFFSET
    }
    pub fn cancel_y(height: i32) -> i32 {
        height / 4 + CANCEL_BASE_Y + CANCEL_GAP_Y
    }
    pub fn take_action(&mut self) -> Option<ConnectingAction> {
        self.cancel.borrow_mut().take_press_action()
    }
}
impl Screen for ConnectingScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn should_close_on_esc(&self) -> bool {
        false
    }
    fn init(&mut self) {
        let x = Self::cancel_x(self.base.width);
        let y = Self::cancel_y(self.base.height);
        {
            let mut button = self.cancel.borrow_mut();
            button.set_position(x, y);
        }
        self.base
            .add_widget(CancelButton::new(Rc::clone(&self.cancel)));
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        let center_x = width / 2;
        self.base.extract_background_layers(graphics, self.in_world);
        self.base
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
        let status = crate::translate(&self.status);
        let status_x = center_x - font_width(self.font.as_ref(), &status) / 2;
        graphics.text(
            ResourceLocation::from_string(DEFAULT_FONT),
            status,
            status_x,
            height / 2 + STATUS_OFFSET_Y,
            STATUS_COLOR,
        );
        if let Some(detail) = &self.detail {
            let detail_x = center_x - font_width(self.font.as_ref(), detail) / 2;
            graphics.text(
                ResourceLocation::from_string(DEFAULT_FONT),
                detail.clone(),
                detail_x,
                height / 2 + DETAIL_OFFSET_Y,
                DETAIL_COLOR,
            );
        }
    }
    fn update_narration(&mut self, output: &mut NarrationElementOutput) {
        self.base.update_narration(output);
        let now = self.base.clock_ms();
        if now - self.last_narration_ms > NARRATION_DELAY_MS {
            self.last_narration_ms = now;
            output.add(
                NarratedElementType::Hint,
                crate::translate(JOINING_NARRATION_KEY),
            );
        }
    }
}
impl Clone for ConnectingScreen {
    fn clone(&self) -> Self {
        let mut cloned = Self::new(
            self.in_world,
            self.server_name.clone(),
            self.server_ip.clone(),
            Arc::clone(&self.font),
        );
        cloned.status = self.status.clone();
        cloned.detail = self.detail.clone();
        cloned.last_narration_ms = self.last_narration_ms;
        let (width, height) = (self.base.width, self.base.height);
        screen_init(&mut cloned, width, height);
        cloned
    }
}
impl std::fmt::Debug for ConnectingScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectingScreen")
            .field("in_world", &self.in_world)
            .field("server_name", &self.server_name)
            .field("server_ip", &self.server_ip)
            .field("status", &self.status)
            .field("detail", &self.detail)
            .field("width", &self.base.width)
            .field("height", &self.base.height)
            .field("widgets", &self.base.widgets_len())
            .field("focused", &self.base.focused_index())
            .finish()
    }
}
fn font_width(font: &dyn FontWidthMeasurer, text: &str) -> i32 {
    font.measure_width(text, 1.0).ceil() as i32
}
const DEFAULT_FONT: &str = "minecraft:default";
struct CancelButton {
    button: Rc<RefCell<Button<ConnectingAction>>>,
}
impl CancelButton {
    fn new(button: Rc<RefCell<Button<ConnectingAction>>>) -> Self {
        Self { button }
    }
}
impl Renderable for CancelButton {
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
impl TabOrderedElement for CancelButton {
    fn tab_order_group(&self) -> i32 {
        self.button.borrow().tab_order_group()
    }
}
impl GuiEventListener for CancelButton {
    fn mouse_moved(&mut self, x: f64, y: f64) {
        self.button.borrow_mut().mouse_moved(x, y);
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.button.borrow_mut().mouse_clicked(event, double_click)
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.button.borrow().play_down_sound()
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        self.button.borrow_mut().mouse_released(event)
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        self.button.borrow_mut().mouse_dragged(event, dx, dy)
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        self.button
            .borrow_mut()
            .mouse_scrolled(x, y, scroll_x, scroll_y)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        self.button.borrow_mut().key_pressed(event)
    }
    fn key_released(&mut self, event: KeyEvent) -> bool {
        self.button.borrow_mut().key_released(event)
    }
    fn char_typed(&mut self, event: CharacterEvent) -> bool {
        self.button.borrow_mut().char_typed(event)
    }
    fn preedit_updated(&mut self, event: Option<PreeditEvent>) -> bool {
        self.button.borrow_mut().preedit_updated(event)
    }
    fn next_focus_path(&self, event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.button.borrow().next_focus_path(event)
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.button.borrow().is_mouse_over(mouse_x, mouse_y)
    }
    fn set_focused(&mut self, focused: bool) {
        self.button.borrow_mut().set_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.button.borrow().is_focused()
    }
    fn should_take_focus_after_interaction(&self) -> bool {
        self.button.borrow().should_take_focus_after_interaction()
    }
    fn rectangle(&self) -> ScreenRectangle {
        GuiEventListener::rectangle(&*self.button.borrow())
    }
    fn border_for_arrow_navigation(&self, opposite: ScreenDirection) -> ScreenRectangle {
        self.button.borrow().border_for_arrow_navigation(opposite)
    }
}
impl NarratableEntry for CancelButton {
    fn narration_priority(&self) -> NarrationPriority {
        self.button.borrow().narration_priority()
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        self.button.borrow().update_narration(output);
    }
}
impl LayoutElement for CancelButton {
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
