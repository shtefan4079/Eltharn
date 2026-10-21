use crate::FontWidthMeasurer;
use crate::components::text_widget::{
    DEFAULT_TOTAL_PADDING, FittingMultiLineTextWidget, LINE_HEIGHT, StringWidget,
};
use crate::components::{
    Button, DEFAULT_HEIGHT, DEFAULT_WIDTH, GuiEventListener, Renderable, TabOrderedElement, Widget,
    WidgetClickSound,
    checkbox::{BOX_SIZE, Checkbox},
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{NarratableEntry, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{
    ComponentPath, FocusNavigationEvent, ScreenDirection, ScreenRectangle,
};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::Component;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
pub const MESSAGE_PADDING: i32 = 100;
pub const MESSAGE_CELL_PADDING: i32 = 12;
pub const LAYOUT_SPACING: i32 = 8;
pub const TITLE_HEIGHT: i32 = LINE_HEIGHT;
pub const CHECKBOX_HEIGHT: i32 = BOX_SIZE;
pub const FOOTER_BUTTON_WIDTH: i32 = DEFAULT_WIDTH;
pub const FOOTER_ROW_HEIGHT: i32 = DEFAULT_HEIGHT;
pub const WRAP_WIDTH_INSET: i32 = MESSAGE_PADDING + DEFAULT_TOTAL_PADDING;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WarningLayout {
    pub title_x: i32,
    pub title_y: i32,
    pub message_x: i32,
    pub message_y: i32,
    pub message_width: i32,
    pub message_height: i32,
    pub checkbox_x: i32,
    pub checkbox_y: i32,
    pub checkbox_width: i32,
    pub checkbox_height: i32,
    pub buttons_y: i32,
    pub first_button_x: i32,
    pub second_button_x: i32,
}
fn centered(pos: i32, length: i32, child_length: i32) -> i32 {
    pos + (((length - child_length) as f32) * 0.5) as i32
}
pub struct WarningScreen {
    message: String,
    title: String,
    narration: String,
    checkbox: Option<Rc<RefCell<Checkbox>>>,
    checkbox_width: i32,
    checkbox_height: i32,
    font: Arc<dyn FontWidthMeasurer>,
    title_handle: usize,
    message_handle: usize,
    checkbox_handle: usize,
}
impl WarningScreen {
    pub fn new(
        title_key: &str,
        message: String,
        check_key: Option<&str>,
        check_label_color: Option<i32>,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let title = crate::translate(title_key);
        let narration = format!("{title}\n{message}");
        let checkbox = check_key.map(|key| {
            let mut builder = Checkbox::builder(Component::literal(&crate::translate(key)))
                .with_font(font.clone());
            if let Some(color) = check_label_color {
                builder = builder.with_label_color(color);
            }
            Rc::new(RefCell::new(builder.build()))
        });
        let (checkbox_width, checkbox_height) = match checkbox.as_ref() {
            Some(checkbox) => match checkbox.try_borrow() {
                Ok(checkbox) => (checkbox.width(), checkbox.height()),
                Err(_) => (0, 0),
            },
            None => (0, 0),
        };
        Self {
            message,
            title,
            narration,
            checkbox,
            checkbox_width,
            checkbox_height,
            font,
            title_handle: 0,
            message_handle: 0,
            checkbox_handle: 0,
        }
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn narration_message(&self) -> String {
        self.narration.clone()
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn has_checkbox(&self) -> bool {
        self.checkbox.is_some()
    }
    pub fn checkbox(&self) -> Option<Rc<RefCell<Checkbox>>> {
        self.checkbox.clone()
    }
    pub fn set_handles(&mut self, title: usize, message: usize, checkbox: usize) {
        self.title_handle = title;
        self.message_handle = message;
        self.checkbox_handle = checkbox;
    }
    pub fn title_handle(&self) -> usize {
        self.title_handle
    }
    pub fn message_handle(&self) -> usize {
        self.message_handle
    }
    pub fn checkbox_handle(&self) -> usize {
        self.checkbox_handle
    }
    pub fn drain_value_changed(&mut self) -> bool {
        match self.checkbox.as_ref() {
            Some(checkbox) => match checkbox.try_borrow_mut() {
                Ok(mut checkbox) => checkbox.take_value_changed(),
                Err(_) => false,
            },
            None => false,
        }
    }
    pub fn is_checked(&self) -> bool {
        self.read_checkbox(Checkbox::is_checked, false)
    }
    pub fn set_checked(&mut self, checked: bool) {
        if let Some(checkbox) = self.checkbox.as_ref()
            && let Ok(mut checkbox) = checkbox.try_borrow_mut()
        {
            checkbox.set_checked(checked);
        }
    }
    fn read_checkbox<T>(&self, read: impl FnOnce(&Checkbox) -> T, default: T) -> T {
        match self.checkbox.as_ref() {
            Some(checkbox) => checkbox
                .try_borrow()
                .ok()
                .map(|c| read(&c))
                .unwrap_or(default),
            None => default,
        }
    }
    fn title_width(&self) -> i32 {
        self.font.measure_width(&self.title, 1.0).ceil() as i32
    }
    pub fn message_lines(&self, width: i32) -> Vec<String> {
        crate::wrap_text(
            &self.message,
            (width - WRAP_WIDTH_INSET).max(1) as f32,
            self.font.as_ref(),
        )
    }
    pub fn message_height(&self, width: i32, height: i32) -> i32 {
        let available = (height - MESSAGE_PADDING).max(1);
        let content = self.message_lines(width).len() as i32 * LINE_HEIGHT + DEFAULT_TOTAL_PADDING;
        content.min(available)
    }
    pub fn layout(&self, width: i32, height: i32) -> WarningLayout {
        let message_width = (width - MESSAGE_PADDING).max(1);
        let message_height = self.message_height(width, height);
        let message_cell_width = message_width + 2 * MESSAGE_CELL_PADDING;
        let checkbox_row = if self.checkbox_height > 0 {
            self.checkbox_height + LAYOUT_SPACING
        } else {
            0
        };
        let footer_width = self.checkbox_width.max(Self::button_row_width());
        let footer_height = checkbox_row + FOOTER_ROW_HEIGHT;
        let stack_height = TITLE_HEIGHT
            + LAYOUT_SPACING
            + (MESSAGE_CELL_PADDING + message_height + MESSAGE_CELL_PADDING)
            + LAYOUT_SPACING
            + footer_height;
        let stack_width = self.title_width().max(message_cell_width).max(footer_width);
        let stack_x = centered(0, width, stack_width);
        let title_y = centered(0, height, stack_height);
        let message_y = title_y + TITLE_HEIGHT + LAYOUT_SPACING + MESSAGE_CELL_PADDING;
        let footer_y = message_y + message_height + MESSAGE_CELL_PADDING + LAYOUT_SPACING;
        let buttons_y = if self.checkbox_height > 0 {
            footer_y + self.checkbox_height + LAYOUT_SPACING
        } else {
            footer_y
        };
        let row_x = centered(stack_x, stack_width, Self::button_row_width());
        WarningLayout {
            title_x: centered(stack_x, stack_width, self.title_width()),
            title_y,
            message_x: centered(stack_x, stack_width, message_cell_width) + MESSAGE_CELL_PADDING,
            message_y,
            message_width,
            message_height,
            checkbox_x: if self.checkbox_height > 0 {
                centered(stack_x, stack_width, self.checkbox_width)
            } else {
                0
            },
            checkbox_y: footer_y,
            checkbox_width: self.checkbox_width,
            checkbox_height: self.checkbox_height,
            buttons_y,
            first_button_x: row_x,
            second_button_x: row_x + FOOTER_BUTTON_WIDTH + LAYOUT_SPACING,
        }
    }
    pub const fn button_row_width() -> i32 {
        2 * FOOTER_BUTTON_WIDTH + LAYOUT_SPACING
    }
    pub fn build_title(&self) -> StringWidget {
        let message = Component::literal(&self.title);
        let mut title =
            StringWidget::new(0, 0, 0, TITLE_HEIGHT, message.clone()).with_font(self.font.clone());
        title.set_message(message);
        title
    }
    pub fn build_message(&self, width: i32, height: i32) -> FittingMultiLineTextWidget {
        let mut message = FittingMultiLineTextWidget::new(
            0,
            0,
            (width - MESSAGE_PADDING).max(1),
            (height - MESSAGE_PADDING).max(1),
            Component::literal(&self.message),
        )
        .with_font(self.font.clone());
        message.minimize_height();
        message
    }
    pub fn arrange(
        &self,
        widgets: &mut [Box<dyn Widget>],
        width: i32,
        height: i32,
    ) -> WarningLayout {
        let layout = self.layout(width, height);
        if let Some(title) = widgets.get_mut(self.title_handle) {
            LayoutElement::set_position(&mut **title, layout.title_x, layout.title_y);
        }
        if let Some(slot) = widgets.get_mut(self.message_handle) {
            let mut message = self.build_message(width, height);
            message.set_position(layout.message_x, layout.message_y);
            *slot = Box::new(message);
        }
        if self.has_checkbox()
            && let Some(checkbox) = widgets.get_mut(self.checkbox_handle)
        {
            LayoutElement::set_position(&mut **checkbox, layout.checkbox_x, layout.checkbox_y);
        }
        layout
    }
}
pub fn position_footer_button<A>(button: &Rc<RefCell<Button<A>>>, x: i32, y: i32)
where
    A: Clone + PartialEq + std::fmt::Debug,
{
    if let Ok(mut button) = button.try_borrow_mut() {
        button.set_position(x, y);
    }
}
pub struct SharedWidget<W> {
    inner: Rc<RefCell<W>>,
}
impl<W> SharedWidget<W> {
    pub fn new(inner: Rc<RefCell<W>>) -> Self {
        Self { inner }
    }
}
impl<W> Renderable for SharedWidget<W>
where
    W: Renderable,
{
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        if let Ok(mut inner) = self.inner.try_borrow_mut() {
            inner.extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
        }
    }
}
impl<W> TabOrderedElement for SharedWidget<W>
where
    W: TabOrderedElement,
{
    fn tab_order_group(&self) -> i32 {
        self.with_inner(TabOrderedElement::tab_order_group, 0)
    }
}
impl<W> GuiEventListener for SharedWidget<W>
where
    W: GuiEventListener,
{
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.with_inner_mut(|inner| inner.mouse_clicked(event, double_click), false)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        self.with_inner_mut(|inner| inner.key_pressed(event), false)
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.with_inner(GuiEventListener::play_down_sound, None)
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        self.with_inner_mut(|inner| inner.mouse_released(event), false)
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        self.with_inner_mut(|inner| inner.mouse_dragged(event, dx, dy), false)
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        self.with_inner_mut(
            |inner| inner.mouse_scrolled(x, y, scroll_x, scroll_y),
            false,
        )
    }
    fn key_released(&mut self, event: KeyEvent) -> bool {
        self.with_inner_mut(|inner| inner.key_released(event), false)
    }
    fn next_focus_path(&self, navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.with_inner(
            |inner| GuiEventListener::next_focus_path(inner, navigation_event),
            None,
        )
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.with_inner(|inner| inner.is_mouse_over(mouse_x, mouse_y), false)
    }
    fn set_focused(&mut self, focused: bool) {
        if let Ok(mut inner) = self.inner.try_borrow_mut() {
            inner.set_focused(focused);
        }
    }
    fn is_focused(&self) -> bool {
        self.with_inner(GuiEventListener::is_focused, false)
    }
    fn should_take_focus_after_interaction(&self) -> bool {
        self.with_inner(GuiEventListener::should_take_focus_after_interaction, false)
    }
    fn rectangle(&self) -> ScreenRectangle {
        self.with_inner(GuiEventListener::rectangle, ScreenRectangle::empty())
    }
    fn border_for_arrow_navigation(&self, opposite: ScreenDirection) -> ScreenRectangle {
        self.with_inner(
            |inner| inner.border_for_arrow_navigation(opposite),
            ScreenRectangle::empty(),
        )
    }
}
impl<W> NarratableEntry for SharedWidget<W>
where
    W: NarratableEntry,
{
    fn narration_priority(&self) -> crate::framework::narration::NarrationPriority {
        self.with_inner(NarratableEntry::narration_priority, NarrationPriority::None)
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        if let Ok(inner) = self.inner.try_borrow() {
            inner.update_narration(output);
        }
    }
}
impl<W> LayoutElement for SharedWidget<W>
where
    W: LayoutElement,
{
    fn set_x(&mut self, x: i32) {
        self.with_inner_mut(|inner| LayoutElement::set_x(inner, x), ());
    }
    fn set_y(&mut self, y: i32) {
        self.with_inner_mut(|inner| LayoutElement::set_y(inner, y), ());
    }
    fn x(&self) -> i32 {
        self.with_inner(LayoutElement::x, 0)
    }
    fn y(&self) -> i32 {
        self.with_inner(LayoutElement::y, 0)
    }
    fn width(&self) -> i32 {
        self.with_inner(LayoutElement::width, 0)
    }
    fn height(&self) -> i32 {
        self.with_inner(LayoutElement::height, 0)
    }
}
impl<W> SharedWidget<W> {
    fn with_inner<T>(&self, read: impl FnOnce(&W) -> T, default: T) -> T {
        self.inner
            .try_borrow()
            .ok()
            .map(|inner| read(&inner))
            .unwrap_or(default)
    }
    fn with_inner_mut<T>(&mut self, write: impl FnOnce(&mut W) -> T, default: T) -> T {
        self.inner
            .try_borrow_mut()
            .ok()
            .map(|mut inner| write(&mut inner))
            .unwrap_or(default)
    }
}
