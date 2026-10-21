use crate::FontWidthMeasurer;
use crate::components::{
    Button, GuiEventListener, Renderable, TabOrderedElement, WidgetClickSound,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{NarratableEntry, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::Component;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InBedChatAction {
    StopSleeping,
}
struct LeaveBedButtonHandle {
    button: Rc<RefCell<Button<InBedChatAction>>>,
}
impl LeaveBedButtonHandle {
    fn new(button: Rc<RefCell<Button<InBedChatAction>>>) -> Self {
        Self { button }
    }
}
impl Renderable for LeaveBedButtonHandle {
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
impl TabOrderedElement for LeaveBedButtonHandle {
    fn tab_order_group(&self) -> i32 {
        self.button.borrow().tab_order_group()
    }
}
impl GuiEventListener for LeaveBedButtonHandle {
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
impl LayoutElement for LeaveBedButtonHandle {
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
impl NarratableEntry for LeaveBedButtonHandle {
    fn is_active(&self) -> bool {
        self.button.borrow().is_active()
    }
    fn narration_priority(&self) -> NarrationPriority {
        self.button.borrow().narration_priority()
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        self.button.borrow().update_narration(output);
    }
}
pub struct InBedChatScreen {
    base: ScreenBase,
    font: Arc<dyn FontWidthMeasurer>,
    leave_bed_button: Rc<RefCell<Button<InBedChatAction>>>,
    pub input: String,
    pub is_draft: bool,
}
impl InBedChatScreen {
    pub fn new(
        initial: impl Into<String>,
        is_draft: bool,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let leave_bed_btn = Button::builder(Component::literal(&crate::translate(
            "multiplayer.stopSleeping",
        )))
        .bounds(0, 0, 200, 20)
        .with_action(InBedChatAction::StopSleeping)
        .with_font(Arc::clone(&font))
        .build();
        let mut base = ScreenBase::new(Component::literal(&crate::translate(
            "multiplayer.stopSleeping",
        )));
        base.set_close_on_esc(true);
        Self {
            base,
            font,
            leave_bed_button: Rc::new(RefCell::new(leave_bed_btn)),
            input: initial.into(),
            is_draft,
        }
    }
    pub const fn leave_bed_button_bounds(width: f32, height: f32) -> (f32, f32, f32, f32) {
        (width / 2.0 - 100.0, height - 40.0, 200.0, 20.0)
    }
    pub fn on_player_woke_up(&self) -> Option<String> {
        if !self.is_draft && !self.input.is_empty() {
            Some(self.input.clone())
        } else {
            None
        }
    }
    pub fn take_action(&mut self) -> Option<InBedChatAction> {
        if self
            .leave_bed_button
            .borrow_mut()
            .take_press_action()
            .is_some()
        {
            return Some(InBedChatAction::StopSleeping);
        }
        if self.base.take_close_request() {
            return Some(InBedChatAction::StopSleeping);
        }
        None
    }
}
impl Clone for InBedChatScreen {
    fn clone(&self) -> Self {
        let mut cloned = Self::new(self.input.clone(), self.is_draft, Arc::clone(&self.font));
        if self.base.width > 0 && self.base.height > 0 {
            cloned.base.width = self.base.width;
            cloned.base.height = self.base.height;
            cloned.init();
        }
        cloned
    }
}
impl std::fmt::Debug for InBedChatScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InBedChatScreen")
            .field("input", &self.input)
            .field("is_draft", &self.is_draft)
            .finish()
    }
}
impl Screen for InBedChatScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        let (bx, by, bw, bh) = Self::leave_bed_button_bounds(width as f32, height as f32);
        {
            let mut btn = self.leave_bed_button.borrow_mut();
            btn.base.set_x(bx as i32);
            btn.base.set_y(by as i32);
            btn.base.set_width(bw as i32);
            btn.base.set_height(bh as i32);
        }
        self.base.clear_widgets();
        self.base
            .add_widget(LeaveBedButtonHandle::new(Rc::clone(&self.leave_bed_button)));
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
    fn is_pause_screen(&self) -> bool {
        false
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.base.mouse_clicked(event, double_click)
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        self.base.mouse_released(event)
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        self.base
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
    }
}
