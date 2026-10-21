use crate::components::{Renderable, WidgetBase};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{NarratableEntry, NarratedElementType};
use crate::framework::narration::{NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{ComponentPath, ScreenRectangle};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::Component;
const HANDLE_WIDTH: i32 = 8;
const HANDLE_HALF_WIDTH: i32 = 4;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SliderInputKind {
    KeyboardArrow,
    #[default]
    Mouse,
    KeyboardTab,
}
impl SliderInputKind {
    pub fn enables_value_change_on_focus(self) -> bool {
        matches!(self, Self::Mouse | Self::KeyboardTab)
    }
}
pub type LabelFormatter = Box<dyn Fn(f64) -> String>;
pub struct SliderButton {
    pub base: WidgetBase,
    caption: Component,
    value: f64,
    min_value: f64,
    max_value: f64,
    step: f64,
    can_change_value: bool,
    dragging: bool,
    label_formatter: Option<LabelFormatter>,
    value_changed: bool,
    last_input_kind: SliderInputKind,
}
impl SliderButton {
    pub fn new(x: i32, y: i32, width: i32, height: i32, message: Component) -> Self {
        Self::with_initial_value(x, y, width, height, message, 0.0)
    }
    pub fn with_initial_value(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        message: Component,
        initial_value: f64,
    ) -> Self {
        let caption = message.clone();
        let mut slider = Self {
            caption,
            base: WidgetBase::new(x, y, width, height, message),
            value: 0.0,
            min_value: 0.0,
            max_value: 1.0,
            step: 0.0,
            can_change_value: false,
            dragging: false,
            label_formatter: None,
            value_changed: false,
            last_input_kind: SliderInputKind::default(),
        };
        slider.set_value(initial_value);
        slider.value_changed = false;
        slider
    }
    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min_value = min.min(max);
        self.max_value = min.max(max);
        self.update_message();
        self
    }
    pub fn with_step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }
    pub fn set_label_formatter(&mut self, formatter: LabelFormatter) {
        self.label_formatter = Some(formatter);
        self.update_message();
    }
    pub fn clear_label_formatter(&mut self) {
        self.label_formatter = None;
        self.update_message();
    }
    pub fn value(&self) -> f64 {
        self.ratio_to_value(self.value)
    }
    pub fn set_value(&mut self, new_value: f64) {
        self.set_ratio(self.value_to_ratio(new_value));
    }
    pub fn min_value(&self) -> f64 {
        self.min_value
    }
    pub fn max_value(&self) -> f64 {
        self.max_value
    }
    pub fn value_ratio(&self) -> f64 {
        self.value
    }
    pub fn set_value_ratio(&mut self, ratio: f64) {
        self.set_ratio(ratio);
    }
    pub fn set_value_from_mouse(&mut self, mouse_x: f64) {
        let usable = (self.base.width() - HANDLE_WIDTH) as f64;
        let ratio = if usable > 0.0 {
            (mouse_x - f64::from(self.base.x() + HANDLE_HALF_WIDTH)) / usable
        } else {
            0.0
        };
        self.set_ratio(ratio);
    }
    pub fn take_value_changed(&mut self) -> bool {
        let changed = self.value_changed;
        self.value_changed = false;
        changed
    }
    pub fn can_change_value(&self) -> bool {
        self.can_change_value
    }
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }
    pub fn set_input_kind(&mut self, kind: SliderInputKind) {
        self.last_input_kind = kind;
    }
    pub fn set_focused_from_input(&mut self, focused: bool, kind: SliderInputKind) {
        self.last_input_kind = kind;
        self.base.set_focused(focused);
        self.apply_focus_side_effects(focused);
    }
    fn apply_focus_side_effects(&mut self, focused: bool) {
        if focused {
            self.can_change_value = self.last_input_kind.enables_value_change_on_focus();
        } else {
            self.can_change_value = false;
        }
    }
    fn set_ratio(&mut self, new_ratio: f64) {
        let old_value = self.value;
        self.value = self.clamp_and_snap(new_ratio);
        if old_value != self.value {
            self.apply_value();
        }
        self.update_message();
    }
    fn clamp_and_snap(&self, new_ratio: f64) -> f64 {
        let clamped = if new_ratio.is_finite() {
            new_ratio.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if self.step > 0.0 && self.step.is_finite() {
            let span = (self.max_value - self.min_value).abs();
            if span > 0.0 {
                let snapped = (clamped * span / self.step).round() * self.step / span;
                return snapped.clamp(0.0, 1.0);
            }
        }
        clamped
    }
    fn apply_value(&mut self) {
        self.value_changed = true;
    }
    fn update_message(&mut self) {
        let value = self.value();
        let text = match &self.label_formatter {
            Some(formatter) => formatter(value),
            None => default_percentage_label(&self.caption.get_string(), value),
        };
        self.base.set_message(Component::literal(&text));
    }
    pub const TEXT_MARGIN: i32 = 2;
    pub const DEFAULT_HEIGHT: i32 = 20;
    fn ratio_to_value(&self, ratio: f64) -> f64 {
        self.min_value + ratio * (self.max_value - self.min_value)
    }
    fn value_to_ratio(&self, value: f64) -> f64 {
        let span = self.max_value - self.min_value;
        if span.is_finite() && span.abs() > f64::EPSILON {
            (value - self.min_value) / span
        } else {
            0.0
        }
    }
    fn sprite_state(&self) -> (bool, bool, bool, bool) {
        (
            self.base.is_active(),
            self.base.focused(),
            self.can_change_value,
            self.dragging,
        )
    }
}
fn default_percentage_label(caption: &str, value: f64) -> String {
    let percent = (value * 100.0) as i64;
    format!("{caption}: {percent}%")
}
impl Renderable for SliderButton {
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        _partial_tick: f32,
    ) {
        let in_scissor = graphics.contains_point_in_scissor(mouse_x, mouse_y);
        self.base.set_hovered(
            in_scissor
                && self
                    .base
                    .are_coordinates_in_rectangle(f64::from(mouse_x), f64::from(mouse_y)),
        );
        let (active, focused, can_change_value, dragging) = self.sprite_state();
        graphics.draw_slider_with_interaction_state_and_alpha(
            &SixPixelFont,
            self.base.x(),
            self.base.y(),
            self.base.width(),
            self.base.height(),
            &self.base.message().get_string(),
            self.value as f32,
            active,
            focused,
            can_change_value,
            dragging,
            self.base.alpha,
        );
    }
}
struct SixPixelFont;
impl crate::FontWidthMeasurer for SixPixelFont {
    fn measure_width(&self, text: &str, _scale: f32) -> f32 {
        (text.chars().count() * 6) as f32
    }
}
impl LayoutElement for SliderButton {
    fn set_x(&mut self, x: i32) {
        self.base.set_x(x);
    }
    fn set_y(&mut self, y: i32) {
        self.base.set_y(y);
    }
    fn x(&self) -> i32 {
        self.base.x()
    }
    fn y(&self) -> i32 {
        self.base.y()
    }
    fn width(&self) -> i32 {
        self.base.width()
    }
    fn height(&self) -> i32 {
        self.base.height()
    }
}
impl crate::components::TabOrderedElement for SliderButton {
    fn tab_order_group(&self) -> i32 {
        self.base.tab_order_group()
    }
}
impl crate::components::GuiEventListener for SliderButton {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        if !self.base.is_active() || event.button() != 0 || !self.is_mouse_over(event.x, event.y) {
            return false;
        }
        self.dragging = self.base.active;
        self.set_value_from_mouse(event.x);
        true
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, _dx: f64, _dy: f64) -> bool {
        if event.button() != 0 {
            return false;
        }
        self.set_value_from_mouse(event.x);
        true
    }
    fn mouse_released(&mut self, _event: MouseButtonEvent) -> bool {
        if _event.button() != 0 {
            return false;
        }
        self.dragging = false;
        crate::components::click_sound_sink::record_button_click();
        true
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, _scroll_x: f64, scroll_y: f64) -> bool {
        if !self.base.is_active() || !self.is_mouse_over(x, y) || scroll_y == 0.0 {
            return false;
        }
        let direction = if scroll_y > 0.0 { 1.0 } else { -1.0 };
        let usable = (self.base.width() - HANDLE_WIDTH) as f64;
        let step = if usable > 0.0 {
            direction / usable
        } else {
            direction
        };
        self.set_ratio(self.value + step);
        true
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if event.is_selection() {
            self.can_change_value = !self.can_change_value;
            return true;
        }
        if !self.can_change_value {
            return false;
        }
        let left = event.key == KeyEvent::KEY_LEFT;
        let right = event.key == KeyEvent::KEY_RIGHT;
        if left || right {
            let direction = if left { -1.0 } else { 1.0 };
            let usable = (self.base.width() - HANDLE_WIDTH) as f64;
            let delta = if usable > 0.0 {
                direction / usable
            } else {
                direction
            };
            self.set_ratio(self.value + delta);
            return true;
        }
        false
    }
    fn play_down_sound(&self) -> Option<crate::components::WidgetClickSound> {
        None
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.base.is_active() && self.base.are_coordinates_in_rectangle(mouse_x, mouse_y)
    }
    fn set_focused(&mut self, focused: bool) {
        self.base.set_focused(focused);
        self.apply_focus_side_effects(focused);
    }
    fn is_focused(&self) -> bool {
        self.base.focused()
    }
    fn rectangle(&self) -> ScreenRectangle {
        LayoutElement::rectangle(self)
    }
    fn next_focus_path(
        &self,
        _navigation_event: &crate::framework::navigation::FocusNavigationEvent,
    ) -> Option<ComponentPath> {
        if self.base.is_active() && !self.is_focused() {
            Some(ComponentPath::leaf(0))
        } else {
            None
        }
    }
}
impl NarratableEntry for SliderButton {
    fn is_active(&self) -> bool {
        self.base.is_active()
    }
    fn narration_priority(&self) -> NarrationPriority {
        if self.base.focused() {
            NarrationPriority::Focused
        } else if self.base.hovered() {
            NarrationPriority::Hovered
        } else {
            NarrationPriority::None
        }
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        output.add(
            NarratedElementType::Title,
            format!("gui.narrate.slider {}", self.base.message().get_string()),
        );
        if self.base.active {
            if self.base.focused() {
                if self.can_change_value {
                    output.add(NarratedElementType::Usage, "narration.slider.usage.focused");
                } else {
                    output.add(
                        NarratedElementType::Usage,
                        "narration.slider.usage.focused.keyboard_cannot_change_value",
                    );
                }
            } else {
                output.add(NarratedElementType::Usage, "narration.slider.usage.hovered");
            }
        }
    }
}
