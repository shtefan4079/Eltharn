use crate::components::{
    GuiEventListener, LayoutElement, NarratableEntry, Renderable, TabOrderedElement, WidgetBase,
    WidgetClickSound, click_sound_sink,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::narration::{NarratedElementType, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::input::events::{CursorType, KeyEvent, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
pub const DEFAULT_WIDTH: i32 = 150;
pub const DEFAULT_HEIGHT: i32 = 20;
pub const TEXT_MARGIN: i32 = 2;
const BUTTON_SPRITE: &str = "minecraft:widget/button";
const BUTTON_DISABLED_SPRITE: &str = "minecraft:widget/button_disabled";
const BUTTON_HIGHLIGHTED_SPRITE: &str = "minecraft:widget/button_highlighted";
const BOOLEAN_OPTIONS: [bool; 2] = [true, false];
const OPTION_ON: &str = "options.on";
const OPTION_OFF: &str = "options.off";
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
fn option_name_value(name: &Component, value: &Component) -> Component {
    Component::literal(&translate_args(
        "options.generic_value",
        "%s: %s",
        &[&name.get_string(), &value.get_string()],
    ))
}
fn positive_modulo(input: i32, modulus: i32) -> i32 {
    input.rem_euclid(modulus)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayState {
    NameAndValue,
    Value,
    Hide,
}
pub trait ValueListSupplier<T> {
    fn values(&self) -> Vec<T>;
    fn default_values(&self) -> Vec<T> {
        self.values()
    }
}
impl<T: Clone> ValueListSupplier<T> for Vec<T> {
    fn values(&self) -> Vec<T> {
        self.clone()
    }
    fn default_values(&self) -> Vec<T> {
        self.clone()
    }
}
pub struct AltValueListSupplier<T> {
    use_alt: Box<dyn Fn() -> bool>,
    default_list: Vec<T>,
    alt_list: Vec<T>,
}
impl<T> AltValueListSupplier<T> {
    pub fn new(
        use_alt: impl Fn() -> bool + 'static,
        default_list: Vec<T>,
        alt_list: Vec<T>,
    ) -> Self {
        Self {
            use_alt: Box::new(use_alt),
            default_list,
            alt_list,
        }
    }
}
impl<T: Clone> ValueListSupplier<T> for AltValueListSupplier<T> {
    fn values(&self) -> Vec<T> {
        if (self.use_alt)() {
            self.alt_list.clone()
        } else {
            self.default_list.clone()
        }
    }
    fn default_values(&self) -> Vec<T> {
        self.default_list.clone()
    }
}
pub struct CycleButton<T> {
    pub base: WidgetBase,
    name: Component,
    index: i32,
    value: T,
    default_value_supplier: Box<dyn Fn() -> T>,
    values: Box<dyn ValueListSupplier<T>>,
    value_stringifier: Box<dyn Fn(&T) -> Component>,
    narration_provider: Box<dyn Fn(&CycleButton<T>) -> Component>,
    on_value_change: Box<dyn Fn(&CycleButton<T>, &T)>,
    display_state: DisplayState,
    tooltip_supplier: Box<dyn Fn(&T) -> Option<Vec<String>>>,
    tooltip: Option<Vec<String>>,
    tooltip_delay_millis: u64,
    tooltip_display_start_millis: Option<u128>,
    tooltip_was_displayed: bool,
    sprite_supplier: Box<dyn Fn(&CycleButton<T>, &T) -> Option<ResourceLocation>>,
    value_changed: bool,
}
impl<T: Clone + PartialEq> CycleButton<T> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        message: Component,
        name: Component,
        index: i32,
        value: T,
        default_value_supplier: Box<dyn Fn() -> T>,
        values: Box<dyn ValueListSupplier<T>>,
        value_stringifier: Box<dyn Fn(&T) -> Component>,
        narration_provider: Box<dyn Fn(&CycleButton<T>) -> Component>,
        on_value_change: Box<dyn Fn(&CycleButton<T>, &T)>,
        display_state: DisplayState,
        tooltip_supplier: Box<dyn Fn(&T) -> Option<Vec<String>>>,
        sprite_supplier: Box<dyn Fn(&CycleButton<T>, &T) -> Option<ResourceLocation>>,
    ) -> Self {
        let mut button = Self {
            base: WidgetBase::new(x, y, width, height, message),
            name,
            index,
            value,
            default_value_supplier,
            values,
            value_stringifier,
            narration_provider,
            on_value_change,
            display_state,
            tooltip_supplier,
            tooltip: None,
            sprite_supplier,
            value_changed: false,
            tooltip_delay_millis: 0,
            tooltip_display_start_millis: None,
            tooltip_was_displayed: false,
        };
        button.update_tooltip();
        button
    }
    pub fn value(&self) -> &T {
        &self.value
    }
    pub fn index(&self) -> i32 {
        self.index
    }
    pub fn set_value(&mut self, new_value: T) {
        let list = self.values.values();
        if let Some(new_index) = list.iter().position(|candidate| *candidate == new_value) {
            self.index = new_index as i32;
        }
        self.update_value(new_value);
    }
    pub fn reset_value(&mut self) {
        let default_value = (self.default_value_supplier)();
        self.set_value(default_value);
    }
    pub fn cycle_value(&mut self, delta: i32) {
        let list = self.values.values();
        if list.is_empty() {
            return;
        }
        self.index = positive_modulo(self.index.wrapping_add(delta), list.len() as i32);
        if let Some(new_value) = list.into_iter().nth(self.index as usize) {
            self.update_value(new_value.clone());
            (self.on_value_change)(self, &new_value);
        }
    }
    pub fn cycled_value(&self, delta: i32) -> T {
        let list = self.values.values();
        let index = positive_modulo(self.index.wrapping_add(delta), list.len().max(1) as i32);
        match list.into_iter().nth(index as usize) {
            Some(value) => value,
            None => self.value.clone(),
        }
    }
    fn update_value(&mut self, new_value: T) {
        let new_message = self.create_label_for_value(&new_value);
        if new_value != self.value {
            self.value_changed = true;
        }
        self.base.set_message(new_message);
        self.value = new_value;
        self.update_tooltip();
    }
    fn update_tooltip(&mut self) {
        let tooltip = (self.tooltip_supplier)(&self.value);
        self.tooltip = tooltip;
    }
    fn create_label_for_value(&self, value: &T) -> Component {
        if self.display_state == DisplayState::Value {
            Component::literal(&crate::translate(
                &(self.value_stringifier)(value).get_string(),
            ))
        } else {
            self.create_full_name(value)
        }
    }
    fn create_full_name(&self, value: &T) -> Component {
        option_name_value(&self.name, &(self.value_stringifier)(value))
    }
    pub fn create_default_narration_message(&self) -> Component {
        let inner = if self.display_state == DisplayState::Value {
            self.create_full_name(&self.value)
        } else {
            self.base.message().clone()
        };
        Component::literal(&translate_args(
            "gui.narrate.button",
            "%s button",
            &[&inner.get_string()],
        ))
    }
    fn create_narration_message(&self) -> Component {
        (self.narration_provider)(self)
    }
    pub fn values(&self) -> Vec<T> {
        self.values.values()
    }
    pub fn default_values(&self) -> Vec<T> {
        self.values.default_values()
    }
    pub fn display_state(&self) -> DisplayState {
        self.display_state
    }
    pub fn tooltip_lines(&self) -> Option<&[String]> {
        self.tooltip.as_deref()
    }
    pub fn take_value_changed(&mut self) -> bool {
        let changed = self.value_changed;
        self.value_changed = false;
        changed
    }
    pub fn update_values(&mut self) {
        let list = self.values.values();
        if let Some(index) = list.iter().position(|candidate| *candidate == self.value) {
            self.index = index as i32;
        }
        let value = self.value.clone();
        self.update_value(value);
    }
    pub fn update_size_for_values(&mut self) {
        let mut widest = 0;
        for value in self.values.values() {
            let label = self.create_label_for_value(&value);
            widest = widest.max(label.get_string().chars().count() as i32 * 6);
        }
        self.base.set_width(widest + 2 * TEXT_MARGIN);
        self.base.set_height(DEFAULT_HEIGHT);
    }
    pub fn extract_tooltip(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn crate::FontWidthMeasurer,
        mouse_x: i32,
        mouse_y: i32,
    ) {
        let now_millis = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        self.refresh_tooltip_for_next_render_pass_at_millis(
            graphics, font, mouse_x, mouse_y, false, now_millis,
        );
    }
    pub fn refresh_tooltip_for_next_render_pass_at_millis(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn crate::FontWidthMeasurer,
        mouse_x: i32,
        mouse_y: i32,
        last_input_was_keyboard: bool,
        now_millis: u128,
    ) {
        if !self.base.visible {
            return;
        }
        let is_hovered = self.base.hovered();
        let is_focused = self.base.focused();
        let should_display = is_hovered || (is_focused && last_input_was_keyboard);
        if self.tooltip.is_none() {
            self.tooltip_was_displayed = false;
            return;
        }
        if should_display != self.tooltip_was_displayed {
            if should_display {
                self.tooltip_display_start_millis = Some(now_millis);
            }
            self.tooltip_was_displayed = should_display;
        }
        let displayed_long_enough = self.tooltip_display_start_millis.is_some_and(|start| {
            now_millis.saturating_sub(start) > u128::from(self.tooltip_delay_millis)
        });
        if !should_display || !displayed_long_enough {
            return;
        }
        if let Some(lines) = self.tooltip.as_ref() {
            if lines.is_empty() {
                return;
            }
            let rect = crate::framework::layouts::LayoutElement::rectangle(self);
            let position = if !is_hovered && is_focused && last_input_was_keyboard {
                crate::framework::graphics::WidgetTooltipPosition::BelowOrAbove
            } else {
                crate::framework::graphics::WidgetTooltipPosition::Menu
            };
            graphics.draw_widget_tooltip(font, lines, mouse_x, mouse_y, rect, position);
        }
    }
    pub fn set_tooltip_delay(&mut self, delay_millis: u64) {
        self.tooltip_delay_millis = delay_millis;
    }
    pub fn tooltip_delay_millis(&self) -> u64 {
        self.tooltip_delay_millis
    }
}
impl<T: Clone + PartialEq> Renderable for CycleButton<T> {
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        _partial_tick: f32,
    ) {
        if !self.base.visible {
            return;
        }
        self.base.set_hovered(
            graphics.contains_point_in_scissor(mouse_x, mouse_y)
                && self
                    .base
                    .are_coordinates_in_rectangle(mouse_x as f64, mouse_y as f64),
        );
        let custom_sprite = (self.sprite_supplier)(self, &self.value);
        let x = self.base.x();
        let y = self.base.y();
        let width = self.base.width();
        let height = self.base.height();
        if let Some(sprite) = custom_sprite {
            graphics.blit(sprite, x, y, width, height);
        } else {
            let highlighted = self.base.hovered() || self.base.focused();
            let sprite = if !self.base.active {
                BUTTON_DISABLED_SPRITE
            } else if highlighted {
                BUTTON_HIGHLIGHTED_SPRITE
            } else {
                BUTTON_SPRITE
            };
            graphics.blit_with_alpha(
                ResourceLocation::from_string(sprite),
                x,
                y,
                width,
                height,
                self.base.alpha,
            );
        }
        if self.display_state != DisplayState::Hide {
            let label = self.base.message().get_string();
            let rgb = if self.base.active {
                0x00FF_FFFFu32
            } else {
                0x00A0_A0A0u32
            };
            let color = ((crate::framework::graphics::argb_white(self.base.alpha) as u32)
                & 0xFF00_0000)
                | rgb;
            graphics.draw_scrolling_text_with_default_center_estimated(
                &label,
                x + TEXT_MARGIN,
                x + width - TEXT_MARGIN,
                y,
                y + height,
                color as i32,
            );
        }
        if self.base.hovered() {
            graphics.request_cursor(if self.base.is_active() {
                CursorType::PointingHand
            } else {
                CursorType::NotAllowed
            });
        }
    }
}
impl<T> TabOrderedElement for CycleButton<T> {
    fn tab_order_group(&self) -> i32 {
        self.base.tab_order_group()
    }
}
impl<T: Clone + PartialEq> GuiEventListener for CycleButton<T> {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        if !self.base.is_active() {
            return false;
        }
        if event.button() != 0 || !self.is_mouse_over(event.x, event.y) {
            return false;
        }
        click_sound_sink::record(Some(WidgetClickSound::UiButtonClick));
        let backwards = event.button_info.modifiers & KeyEvent::MOD_SHIFT != 0;
        self.cycle_value(if backwards { -1 } else { 1 });
        true
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if !self.base.is_active() || !event.is_selection() {
            return false;
        }
        click_sound_sink::record(Some(WidgetClickSound::UiButtonClick));
        self.cycle_value(if event.has_shift_down() { -1 } else { 1 });
        true
    }
    fn mouse_scrolled(&mut self, _x: f64, _y: f64, _scroll_x: f64, scroll_y: f64) -> bool {
        if scroll_y > 0.0 {
            self.cycle_value(-1);
        } else if scroll_y < 0.0 {
            self.cycle_value(1);
        }
        true
    }
    fn next_focus_path(&self, _navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        (self.base.is_active() && !self.base.focused()).then(|| ComponentPath::leaf(0))
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.base.is_active() && self.base.are_coordinates_in_rectangle(mouse_x, mouse_y)
    }
    fn set_focused(&mut self, focused: bool) {
        self.base.set_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.base.focused()
    }
    fn rectangle(&self) -> ScreenRectangle {
        self.base.rectangle()
    }
}
impl<T: Clone + PartialEq> NarratableEntry for CycleButton<T> {
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
            self.create_narration_message().get_string(),
        );
        if self.base.active {
            let next_value_text = self
                .create_label_for_value(&self.cycled_value(1))
                .get_string();
            let (key, en_us) = if self.base.focused() {
                (
                    "narration.cycle_button.usage.focused",
                    "Press Enter to switch to %s",
                )
            } else {
                (
                    "narration.cycle_button.usage.hovered",
                    "Left click to switch to %s",
                )
            };
            output.add(
                NarratedElementType::Usage,
                translate_args(key, en_us, &[&next_value_text]),
            );
        }
    }
}
impl<T> LayoutElement for CycleButton<T> {
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
pub struct CycleButtonBuilder<T> {
    default_value_supplier: Box<dyn Fn() -> T>,
    value_stringifier: Box<dyn Fn(&T) -> Component>,
    tooltip_supplier: Box<dyn Fn(&T) -> Option<Vec<String>>>,
    sprite_supplier: Box<dyn Fn(&CycleButton<T>, &T) -> Option<ResourceLocation>>,
    narration_provider: Box<dyn Fn(&CycleButton<T>) -> Component>,
    values: Box<dyn ValueListSupplier<T>>,
    display_state: DisplayState,
}
impl<T: Clone + PartialEq + 'static> CycleButtonBuilder<T> {
    pub fn new(
        value_stringifier: impl Fn(&T) -> Component + 'static,
        default_value_supplier: impl Fn() -> T + 'static,
    ) -> Self {
        Self {
            default_value_supplier: Box::new(default_value_supplier),
            value_stringifier: Box::new(value_stringifier),
            tooltip_supplier: Box::new(|_value| None),
            sprite_supplier: Box::new(|_button, _value| None),
            narration_provider: Box::new(CycleButton::create_default_narration_message),
            values: Box::new(Vec::new()),
            display_state: DisplayState::NameAndValue,
        }
    }
    pub fn with_values(mut self, values: Vec<T>) -> Self {
        self.values = Box::new(values);
        self
    }
    pub fn with_value_supplier(mut self, supplier: impl ValueListSupplier<T> + 'static) -> Self {
        self.values = Box::new(supplier);
        self
    }
    pub fn with_alt_values(
        mut self,
        use_alt: impl Fn() -> bool + 'static,
        values: Vec<T>,
        alt_values: Vec<T>,
    ) -> Self {
        self.values = Box::new(AltValueListSupplier::new(use_alt, values, alt_values));
        self
    }
    pub fn with_tooltip(
        mut self,
        tooltip_supplier: impl Fn(&T) -> Option<Vec<String>> + 'static,
    ) -> Self {
        self.tooltip_supplier = Box::new(tooltip_supplier);
        self
    }
    pub fn with_custom_narration(
        mut self,
        narration_provider: impl Fn(&CycleButton<T>) -> Component + 'static,
    ) -> Self {
        self.narration_provider = Box::new(narration_provider);
        self
    }
    pub fn with_sprite(
        mut self,
        sprite_supplier: impl Fn(&CycleButton<T>, &T) -> Option<ResourceLocation> + 'static,
    ) -> Self {
        self.sprite_supplier = Box::new(sprite_supplier);
        self
    }
    pub fn display_state(mut self, state: DisplayState) -> Self {
        self.display_state = state;
        self
    }
    pub fn display_only_value(mut self) -> Self {
        self.display_state = DisplayState::Value;
        self
    }
    pub fn with_initial_value(mut self, initial_value: T) -> Self {
        self.default_value_supplier = Box::new(move || initial_value.clone());
        self
    }
    pub fn create(
        self,
        name: Component,
        on_value_change: impl Fn(&CycleButton<T>, &T) + 'static,
    ) -> Option<CycleButton<T>> {
        self.create_at_with_listener(0, 0, DEFAULT_WIDTH, DEFAULT_HEIGHT, name, on_value_change)
    }
    pub fn create_at(
        self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        name: Component,
    ) -> Option<CycleButton<T>> {
        self.create_at_with_listener(x, y, width, height, name, |_button, _value| {})
    }
    pub fn create_at_with_listener(
        self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        name: Component,
        on_value_change: impl Fn(&CycleButton<T>, &T) + 'static,
    ) -> Option<CycleButton<T>> {
        let values = self.values.default_values();
        if values.is_empty() {
            return None;
        }
        let initial_value = (self.default_value_supplier)();
        let initial_index = values
            .iter()
            .position(|candidate| *candidate == initial_value)
            .map_or(-1, |index| index as i32);
        let value_text = (self.value_stringifier)(&initial_value);
        let initial_message = if self.display_state == DisplayState::Value {
            Component::literal(&crate::translate(&value_text.get_string()))
        } else {
            option_name_value(&name, &value_text)
        };
        Some(CycleButton::new(
            x,
            y,
            width,
            height,
            initial_message,
            name,
            initial_index,
            initial_value,
            self.default_value_supplier,
            self.values,
            self.value_stringifier,
            self.narration_provider,
            Box::new(on_value_change),
            self.display_state,
            self.tooltip_supplier,
            self.sprite_supplier,
        ))
    }
}
impl<T: Clone + PartialEq + 'static> CycleButton<T> {
    pub fn builder(
        value_stringifier: impl Fn(&T) -> Component + 'static,
        default_value_supplier: impl Fn() -> T + 'static,
    ) -> CycleButtonBuilder<T> {
        CycleButtonBuilder::new(value_stringifier, default_value_supplier)
    }
    pub fn builder_with_default(
        value_stringifier: impl Fn(&T) -> Component + 'static,
        default_value: T,
    ) -> CycleButtonBuilder<T> {
        let initial = default_value.clone();
        CycleButtonBuilder::new(value_stringifier, move || initial.clone())
    }
    pub fn on_off_builder(initial_value: bool) -> CycleButtonBuilder<bool> {
        CycleButtonBuilder::new(
            |value: &bool| {
                Component::literal(&crate::translate(if *value {
                    OPTION_ON
                } else {
                    OPTION_OFF
                }))
            },
            move || initial_value,
        )
        .with_values(BOOLEAN_OPTIONS.to_vec())
    }
    pub fn boolean_builder(
        true_text: Component,
        false_text: Component,
        initial_value: bool,
    ) -> CycleButtonBuilder<bool> {
        CycleButtonBuilder::new(
            move |value: &bool| {
                if *value {
                    true_text.clone()
                } else {
                    false_text.clone()
                }
            },
            move || initial_value,
        )
        .with_values(BOOLEAN_OPTIONS.to_vec())
    }
}
