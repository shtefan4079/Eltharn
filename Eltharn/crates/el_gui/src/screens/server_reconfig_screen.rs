use crate::FontWidthMeasurer;
use crate::components::text_widget::{LINE_HEIGHT, StringWidget};
use crate::components::{
    Button, DEFAULT_HEIGHT, DEFAULT_WIDTH, GuiEventListener, TabOrderedElement, WidgetClickSound,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::{FrameLayout, Layout, LayoutElement, LinearLayout};
use crate::framework::narration::{NarratableEntry, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{
    ComponentPath, FocusNavigationEvent, ScreenDirection, ScreenRectangle,
};
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::Component;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
pub const DISCONNECT_TIME: i32 = 600;
const TITLE_KEY: &str = "connect.reconfiguring";
const DISCONNECT_KEY: &str = "menu.disconnect";
const CELL_PADDING: i32 = 10;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerReconfigAction {
    Disconnect,
}
pub struct ServerReconfigScreen {
    base: ScreenBase,
    in_world: bool,
    delay_ticker: i32,
    font: Arc<dyn FontWidthMeasurer>,
    title_handle: usize,
    button_handle: usize,
    disconnect_button: Rc<RefCell<Button<ServerReconfigAction>>>,
}
impl ServerReconfigScreen {
    pub fn new(in_world: bool, font: Arc<dyn FontWidthMeasurer>) -> Self {
        let disconnect_button = Rc::new(RefCell::new(Self::build_button(&font)));
        let mut screen = Self {
            base: ScreenBase::new(Component::literal(&crate::translate(TITLE_KEY))),
            in_world,
            delay_ticker: 0,
            font,
            title_handle: 0,
            button_handle: 0,
            disconnect_button,
        };
        screen.base.set_close_on_esc(false);
        screen
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn tick(&mut self) {
        if self.delay_ticker < DISCONNECT_TIME {
            self.delay_ticker += 1;
        }
        self.sync_disconnect_gate();
    }
    pub fn should_close_on_esc(&self) -> bool {
        self.base_ref().should_close_on_esc()
    }
    pub fn take_action(&mut self) -> Option<ServerReconfigAction> {
        self.with_button(Button::take_press_action).flatten()
    }
    fn build_button(font: &Arc<dyn FontWidthMeasurer>) -> Button<ServerReconfigAction> {
        Button::<ServerReconfigAction>::builder(Component::literal(&crate::translate(
            DISCONNECT_KEY,
        )))
        .with_action(ServerReconfigAction::Disconnect)
        .with_font(font.clone())
        .build()
    }
    fn build_title(&self) -> StringWidget {
        let message = Component::literal(&crate::translate(TITLE_KEY));
        let mut title =
            StringWidget::new(0, 0, 0, LINE_HEIGHT, message.clone()).with_font(self.font.clone());
        title.set_message(message);
        title
    }
    fn with_button<T>(
        &self,
        apply: impl FnOnce(&mut Button<ServerReconfigAction>) -> T,
    ) -> Option<T> {
        let mut button = self.disconnect_button.try_borrow_mut().ok()?;
        Some(apply(&mut button))
    }
    fn sync_disconnect_gate(&self) {
        let active = self.delay_ticker >= DISCONNECT_TIME;
        self.with_button(|button| button.base.active = active);
    }
    fn widget_size(&self, handle: usize) -> (i32, i32) {
        match self.base.widgets().get(handle) {
            Some(widget) => (
                LayoutElement::width(&**widget),
                LayoutElement::height(&**widget),
            ),
            None => (0, 0),
        }
    }
    fn arrange(&mut self) {
        let mut layout = LinearLayout::vertical().spacing(0);
        {
            let cell = layout.default_cell_settings();
            cell.align_horizontally_center();
            cell.padding(CELL_PADDING);
        }
        let (title_width, title_height) = self.widget_size(self.title_handle);
        layout.add_handle(self.title_handle, title_width, title_height);
        layout.add_handle(self.button_handle, DEFAULT_WIDTH, DEFAULT_HEIGHT);
        layout.arrange_elements();
        let width = self.base.width;
        let height = self.base.height;
        layout.set_position(
            FrameLayout::align_in_dimension(0, width, layout.width(), 0.5),
            FrameLayout::align_in_dimension(0, height, layout.height(), 0.5),
        );
        let mut widgets = self
            .base
            .widgets_mut()
            .iter_mut()
            .enumerate()
            .map(|(index, widget)| (index, widget.as_mut() as &mut dyn LayoutElement));
        layout.arrange_widgets(&mut widgets);
    }
}
impl std::fmt::Debug for ServerReconfigScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let button = self.disconnect_button.try_borrow().ok();
        f.debug_struct("ServerReconfigScreen")
            .field("in_world", &self.in_world)
            .field("delay_ticker", &self.delay_ticker)
            .field("widgets", &self.base.widgets_len())
            .field("title_handle", &self.title_handle)
            .field("button_handle", &self.button_handle)
            .field("close_on_esc", &self.base.should_close_on_esc())
            .field(
                "disconnect_button",
                &button
                    .as_ref()
                    .map(|button| GuiEventListener::rectangle(&**button)),
            )
            .field(
                "button_active",
                &button.as_ref().map(|button| button.base.active),
            )
            .finish()
    }
}
impl Screen for ServerReconfigScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        let title = self.build_title();
        let disconnect = Self::build_button(&self.font);
        self.with_button(|slot| *slot = disconnect);
        self.title_handle = self.base.add_widget(title);
        self.button_handle = self
            .base
            .add_widget(SharedButton::new(self.disconnect_button.clone()));
        self.sync_disconnect_gate();
        self.arrange();
    }
    fn tick(&mut self) {
        self.base().tick();
        ServerReconfigScreen::tick(self);
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        let in_world = self.in_world;
        self.base().extract_background_layers(graphics, in_world);
        self.base()
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
    }
}
struct SharedButton {
    button: Rc<RefCell<Button<ServerReconfigAction>>>,
}
impl SharedButton {
    fn new(button: Rc<RefCell<Button<ServerReconfigAction>>>) -> Self {
        Self { button }
    }
}
impl crate::components::Renderable for SharedButton {
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
impl TabOrderedElement for SharedButton {
    fn tab_order_group(&self) -> i32 {
        match self.button.try_borrow() {
            Ok(button) => button.tab_order_group(),
            Err(_) => 0,
        }
    }
}
impl GuiEventListener for SharedButton {
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
    fn rectangle(&self) -> ScreenRectangle {
        match self.button.try_borrow() {
            Ok(button) => GuiEventListener::rectangle(&*button),
            Err(_) => ScreenRectangle::empty(),
        }
    }
    fn border_for_arrow_navigation(&self, opposite: ScreenDirection) -> ScreenRectangle {
        match self.button.try_borrow() {
            Ok(button) => button.border_for_arrow_navigation(opposite),
            Err(_) => GuiEventListener::rectangle(self),
        }
    }
}
impl NarratableEntry for SharedButton {
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
impl LayoutElement for SharedButton {
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
