pub mod checkbox;
pub mod cycle_button;
pub mod edit_box;
pub mod multiline_edit_box;
pub mod scroll_area;
pub mod selection_list;
pub mod slider_button;
pub mod sprite_icon_button;
pub mod sprites;
pub mod text_widget;
use crate::FontWidthMeasurer;
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{
    NarratableEntry, NarratedElementType, NarrationElementOutput, NarrationPriority,
};
use crate::framework::navigation::{
    ComponentPath, FocusNavigationEvent, ScreenDirection, ScreenRectangle,
};
use crate::input::events::{CharacterEvent, CursorType, KeyEvent, MouseButtonEvent, PreeditEvent};
use el_core::{Component, ResourceLocation};
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetClickSound {
    UiButtonClick,
    BookPageTurn,
}
impl WidgetClickSound {
    pub fn event(self) -> &'static str {
        match self {
            Self::UiButtonClick => "minecraft:ui.button.click",
            Self::BookPageTurn => "minecraft:item.book.page_turn",
        }
    }
    pub fn volume(self) -> f32 {
        0.25
    }
    pub fn pitch(self) -> f32 {
        1.0
    }
}
pub mod click_sound_sink {
    use super::WidgetClickSound;
    use std::cell::RefCell;
    thread_local! {
        static PENDING : RefCell < Vec < WidgetClickSound >> = const {
        RefCell::new(Vec::new()) };
    }
    pub fn record(sound: Option<WidgetClickSound>) {
        if let Some(sound) = sound {
            PENDING.with(|pending| pending.borrow_mut().push(sound));
        }
    }
    pub fn record_button_click() {
        record(Some(WidgetClickSound::UiButtonClick));
    }
    pub fn take_all() -> Vec<WidgetClickSound> {
        PENDING.with(|pending| std::mem::take(&mut *pending.borrow_mut()))
    }
    pub fn clear() {
        PENDING.with(|pending| pending.borrow_mut().clear());
    }
}
pub trait Renderable {
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    );
}
pub trait TabOrderedElement {
    fn tab_order_group(&self) -> i32 {
        0
    }
}
pub trait GuiEventListener: TabOrderedElement {
    fn mouse_moved(&mut self, _x: f64, _y: f64) {}
    fn mouse_clicked(&mut self, _event: MouseButtonEvent, _double_click: bool) -> bool {
        false
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        Some(WidgetClickSound::UiButtonClick)
    }
    fn mouse_released(&mut self, _event: MouseButtonEvent) -> bool {
        false
    }
    fn mouse_dragged(&mut self, _event: MouseButtonEvent, _dx: f64, _dy: f64) -> bool {
        false
    }
    fn mouse_scrolled(&mut self, _x: f64, _y: f64, _scroll_x: f64, _scroll_y: f64) -> bool {
        false
    }
    fn key_pressed(&mut self, _event: KeyEvent) -> bool {
        false
    }
    fn key_released(&mut self, _event: KeyEvent) -> bool {
        false
    }
    fn char_typed(&mut self, _event: CharacterEvent) -> bool {
        false
    }
    fn preedit_updated(&mut self, _event: Option<PreeditEvent>) -> bool {
        false
    }
    fn next_focus_path(&self, _navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        None
    }
    fn is_mouse_over(&self, _mouse_x: f64, _mouse_y: f64) -> bool {
        false
    }
    fn set_focused(&mut self, focused: bool);
    fn is_focused(&self) -> bool;
    fn should_take_focus_after_interaction(&self) -> bool {
        true
    }
    fn rectangle(&self) -> ScreenRectangle {
        ScreenRectangle::empty()
    }
    fn border_for_arrow_navigation(&self, opposite: ScreenDirection) -> ScreenRectangle {
        self.rectangle().get_border(opposite)
    }
}
pub trait Widget: Renderable + GuiEventListener + NarratableEntry + LayoutElement {}
impl<T> Widget for T where T: Renderable + GuiEventListener + NarratableEntry + LayoutElement {}
#[derive(Clone, Debug)]
pub struct WidgetBase {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    message: Component,
    hovered: bool,
    focused: bool,
    tab_order_group: i32,
    pub active: bool,
    pub visible: bool,
    pub alpha: f32,
}
impl WidgetBase {
    pub fn new(x: i32, y: i32, width: i32, height: i32, message: Component) -> Self {
        Self {
            x,
            y,
            width,
            height,
            message,
            hovered: false,
            focused: false,
            tab_order_group: 0,
            active: true,
            visible: true,
            alpha: 1.0,
        }
    }
    pub fn x(&self) -> i32 {
        self.x
    }
    pub fn y(&self) -> i32 {
        self.y
    }
    pub fn width(&self) -> i32 {
        self.width
    }
    pub fn height(&self) -> i32 {
        self.height
    }
    pub fn set_x(&mut self, x: i32) {
        self.x = x;
    }
    pub fn set_y(&mut self, y: i32) {
        self.y = y;
    }
    pub fn set_width(&mut self, width: i32) {
        self.width = width;
    }
    pub fn set_height(&mut self, height: i32) {
        self.height = height;
    }
    pub fn message(&self) -> &Component {
        &self.message
    }
    pub fn set_message(&mut self, message: Component) {
        self.message = message;
    }
    pub fn hovered(&self) -> bool {
        self.hovered
    }
    pub fn focused(&self) -> bool {
        self.focused
    }
    pub fn set_hovered(&mut self, hovered: bool) {
        self.hovered = hovered;
    }
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
    pub fn is_active(&self) -> bool {
        self.visible && self.active
    }
    pub fn set_tab_order_group(&mut self, group: i32) {
        self.tab_order_group = group;
    }
    pub fn tab_order_group(&self) -> i32 {
        self.tab_order_group
    }
    pub fn rectangle(&self) -> ScreenRectangle {
        ScreenRectangle::new(self.x, self.y, self.width, self.height)
    }
    pub fn are_coordinates_in_rectangle(&self, mouse_x: f64, mouse_y: f64) -> bool {
        mouse_x >= self.x as f64
            && mouse_x < (self.x + self.width) as f64
            && mouse_y >= self.y as f64
            && mouse_y < (self.y + self.height) as f64
    }
}
pub const SMALL_WIDTH: i32 = 120;
pub const DEFAULT_WIDTH: i32 = 150;
pub const BIG_WIDTH: i32 = 200;
pub const DEFAULT_HEIGHT: i32 = 20;
pub const DEFAULT_SPACING: i32 = 8;
pub const TEXT_MARGIN: i32 = 2;
pub const INACTIVE_TEXT_COLOR: i32 = 0xFFA0_A0A0u32 as i32;
pub const ACTIVE_TEXT_COLOR: i32 = 0xFFFF_FFFFu32 as i32;
const BUTTON_SPRITES: sprites::WidgetSprites = sprites::sprites::BUTTON;
#[derive(Clone)]
pub struct Button<A: Clone + PartialEq + std::fmt::Debug = String> {
    pub base: WidgetBase,
    action: Option<A>,
    pressed_this_frame: bool,
    font: Option<Arc<dyn FontWidthMeasurer>>,
    tooltip_holder: WidgetTooltipHolder,
}
impl<A: Clone + PartialEq + std::fmt::Debug> Button<A> {
    pub fn builder(message: Component) -> ButtonBuilder<A> {
        ButtonBuilder::new(message)
    }
    pub fn action(&self) -> Option<&A> {
        self.action.as_ref()
    }
    pub fn with_action(mut self, action: impl Into<A>) -> Self {
        self.action = Some(action.into());
        self
    }
    pub fn with_font(mut self, font: Arc<dyn FontWidthMeasurer>) -> Self {
        self.set_font(font);
        self
    }
    pub fn set_font(&mut self, font: Arc<dyn FontWidthMeasurer>) {
        self.font = Some(font);
    }
    pub fn has_font(&self) -> bool {
        self.font.is_some()
    }
    pub fn on_press(&mut self) {
        self.pressed_this_frame = true;
    }
    pub fn take_press_action(&mut self) -> Option<A> {
        if self.pressed_this_frame {
            self.pressed_this_frame = false;
            self.action.clone()
        } else {
            None
        }
    }
    pub fn is_hovered_or_focused(&self) -> bool {
        self.base.hovered() || self.base.focused()
    }
    fn extract_default_sprite(&self, graphics: &mut GuiGraphicsExtractor) {
        let sprite = BUTTON_SPRITES.get(self.base.active, self.is_hovered_or_focused());
        graphics.blit_region(
            sprites::widget_sprite(sprite),
            self.base.x(),
            self.base.y(),
            self.base.width(),
            self.base.height(),
            0.0,
            0.0,
            1.0,
            1.0,
            argb_white(self.base.alpha),
        );
    }
    pub fn set_tooltip(&mut self, lines: Option<Vec<String>>) {
        self.tooltip_holder.set_lines(lines);
    }
    pub fn set_tooltip_delay(&mut self, delay_millis: u64) {
        self.tooltip_holder.set_delay_millis(delay_millis);
    }
    fn extract_tooltip(&mut self, graphics: &mut GuiGraphicsExtractor, mouse_x: i32, mouse_y: i32) {
        let now_millis = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let font: Option<Arc<dyn FontWidthMeasurer>> = self.font.as_ref().map(Arc::clone);
        match font {
            Some(font) => {
                self.refresh_tooltip_holder(graphics, font.as_ref(), mouse_x, mouse_y, now_millis)
            }
            None => self.refresh_tooltip_holder(
                graphics,
                &SixPixelFontStandIn,
                mouse_x,
                mouse_y,
                now_millis,
            ),
        }
    }
    fn refresh_tooltip_holder(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
        mouse_x: i32,
        mouse_y: i32,
        now_millis: u128,
    ) {
        self.tooltip_holder
            .refresh_tooltip_for_next_render_pass_at_millis(
                graphics,
                font,
                mouse_x,
                mouse_y,
                self.base.hovered(),
                self.base.focused(),
                false,
                self.base.rectangle(),
                now_millis,
            );
    }
    fn extract_default_label(&self, graphics: &mut GuiGraphicsExtractor) {
        let text = self.base.message().get_string();
        let left = self.base.x() + TEXT_MARGIN;
        let right = self.base.x() + self.base.width() - TEXT_MARGIN;
        let top = self.base.y();
        let bottom = self.base.y() + self.base.height();
        let color = if self.base.active {
            ACTIVE_TEXT_COLOR
        } else {
            INACTIVE_TEXT_COLOR
        };
        match &self.font {
            Some(font) => graphics.draw_scrolling_text_with_default_center(
                font.as_ref(),
                &text,
                left,
                right,
                top,
                bottom,
                color,
            ),
            None => graphics.draw_scrolling_text_with_default_center_estimated(
                &text, left, right, top, bottom, color,
            ),
        }
    }
}
impl Button<String> {
    pub fn new(x: i32, y: i32, width: i32, height: i32, message: Component) -> Self {
        Self {
            base: WidgetBase::new(x, y, width, height, message),
            action: None,
            pressed_this_frame: false,
            font: None,
            tooltip_holder: WidgetTooltipHolder::default(),
        }
    }
}
pub struct ButtonBuilder<A: Clone + PartialEq + std::fmt::Debug> {
    message: Component,
    action: Option<A>,
    font: Option<Arc<dyn FontWidthMeasurer>>,
    tooltip_holder: WidgetTooltipHolder,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}
impl<A: Clone + PartialEq + std::fmt::Debug> ButtonBuilder<A> {
    pub fn new(message: Component) -> Self {
        Self {
            message,
            action: None,
            font: None,
            tooltip_holder: WidgetTooltipHolder::default(),
            x: 0,
            y: 0,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
        }
    }
    pub fn pos(mut self, x: i32, y: i32) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    pub fn width(mut self, width: i32) -> Self {
        self.width = width;
        self
    }
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
    pub fn bounds(self, x: i32, y: i32, width: i32, height: i32) -> Self {
        self.pos(x, y).size(width, height)
    }
    pub fn with_action(mut self, action: impl Into<A>) -> Self {
        self.action = Some(action.into());
        self
    }
    pub fn with_font(mut self, font: Arc<dyn FontWidthMeasurer>) -> Self {
        self.font = Some(font);
        self
    }
    pub fn tooltip(mut self, lines: Vec<String>) -> Self {
        self.tooltip_holder.set_lines(Some(lines));
        self
    }
    pub fn build(self) -> Button<A> {
        Button {
            base: WidgetBase::new(self.x, self.y, self.width, self.height, self.message),
            action: self.action,
            pressed_this_frame: false,
            font: self.font,
            tooltip_holder: self.tooltip_holder,
        }
    }
}
fn argb_white(alpha: f32) -> i32 {
    let channel = (alpha.clamp(0.0, 1.0) * 255.0) as i32;
    (channel << 24) | 0x00FF_FFFF
}
#[derive(Clone)]
pub struct WidgetTooltipHolder {
    lines: Option<Vec<String>>,
    delay_millis: u64,
    display_start_millis: Option<u128>,
    was_displayed: bool,
}
impl Default for WidgetTooltipHolder {
    fn default() -> Self {
        Self {
            lines: None,
            delay_millis: 0,
            display_start_millis: None,
            was_displayed: false,
        }
    }
}
impl WidgetTooltipHolder {
    #[allow(clippy::too_many_arguments)]
    pub fn refresh_tooltip_for_next_render_pass_at_millis(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
        mouse_x: i32,
        mouse_y: i32,
        hovered: bool,
        focused: bool,
        last_input_was_keyboard: bool,
        widget_rect: ScreenRectangle,
        now_millis: u128,
    ) {
        let should_display = hovered || (focused && last_input_was_keyboard);
        if self.lines.is_none() {
            self.was_displayed = false;
            return;
        }
        if should_display != self.was_displayed {
            if should_display {
                self.display_start_millis = Some(now_millis);
            }
            self.was_displayed = should_display;
        }
        let displayed_long_enough = self
            .display_start_millis
            .is_some_and(|start| now_millis.saturating_sub(start) > u128::from(self.delay_millis));
        if !should_display || !displayed_long_enough {
            return;
        }
        if let Some(lines) = self.lines.as_ref() {
            if lines.is_empty() {
                return;
            }
            let position = if !hovered && focused && last_input_was_keyboard {
                crate::framework::graphics::WidgetTooltipPosition::BelowOrAbove
            } else {
                crate::framework::graphics::WidgetTooltipPosition::Menu
            };
            graphics.draw_widget_tooltip(font, lines, mouse_x, mouse_y, widget_rect, position);
        }
    }
    pub fn set_lines(&mut self, lines: Option<Vec<String>>) {
        self.lines = lines;
    }
    pub fn set_delay_millis(&mut self, delay_millis: u64) {
        self.delay_millis = delay_millis;
    }
}
struct SixPixelFontStandIn;
impl crate::FontWidthMeasurer for SixPixelFontStandIn {
    fn measure_width(&self, text: &str, _scale: f32) -> f32 {
        6.0 * text.chars().count() as f32
    }
}
impl<A: Clone + PartialEq + std::fmt::Debug> Renderable for Button<A> {
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
        self.extract_default_sprite(graphics);
        self.extract_default_label(graphics);
        if self.base.hovered() {
            graphics.request_cursor(if self.base.is_active() {
                CursorType::PointingHand
            } else {
                CursorType::NotAllowed
            });
        }
        self.extract_tooltip(graphics, mouse_x, mouse_y);
    }
}
impl<A: Clone + PartialEq + std::fmt::Debug> TabOrderedElement for Button<A> {
    fn tab_order_group(&self) -> i32 {
        self.base.tab_order_group()
    }
}
impl<A: Clone + PartialEq + std::fmt::Debug> GuiEventListener for Button<A> {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        if !self.base.is_active() || event.button() != 0 {
            return false;
        }
        if !self.is_mouse_over(event.x, event.y) {
            return false;
        }
        click_sound_sink::record(self.play_down_sound());
        self.on_press();
        true
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if !self.base.is_active() || !event.is_selection() {
            return false;
        }
        click_sound_sink::record(self.play_down_sound());
        self.on_press();
        true
    }
    fn mouse_scrolled(&mut self, _x: f64, _y: f64, _scroll_x: f64, _scroll_y: f64) -> bool {
        false
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        Some(WidgetClickSound::UiButtonClick)
    }
    fn set_focused(&mut self, focused: bool) {
        self.base.set_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.base.focused()
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.base.is_active() && self.base.are_coordinates_in_rectangle(mouse_x, mouse_y)
    }
    fn next_focus_path(&self, _navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        if self.base.is_active() && !self.base.focused() {
            Some(ComponentPath::leaf(0))
        } else {
            None
        }
    }
    fn rectangle(&self) -> ScreenRectangle {
        self.base.rectangle()
    }
}
impl<A: Clone + PartialEq + std::fmt::Debug> NarratableEntry for Button<A> {
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
            format!("gui.narrate.button {}", self.base.message().get_string()),
        );
        if self.base.active {
            let usage = if self.base.focused() {
                "narration.button.usage.focused"
            } else if self.base.hovered() {
                "narration.button.usage.hovered"
            } else {
                return;
            };
            output.add(NarratedElementType::Usage, usage);
        }
    }
}
impl<A: Clone + PartialEq + std::fmt::Debug> LayoutElement for Button<A> {
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
impl<A: Clone + PartialEq + std::fmt::Debug> std::fmt::Debug for Button<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("action", &self.action)
            .field("pressed_this_frame", &self.pressed_this_frame)
            .field("x", &self.base.x())
            .field("y", &self.base.y())
            .field("width", &self.base.width())
            .field("height", &self.base.height())
            .field("message", &self.base.message().get_string())
            .field(
                "font",
                &self.font.as_ref().map(|_| "client FontWidthMeasurer"),
            )
            .finish()
    }
}
pub struct Label {
    pub base: WidgetBase,
    pub color: i32,
    pub shadow: bool,
}
impl Label {
    pub fn new(x: i32, y: i32, message: Component) -> Self {
        let width = message.get_string().chars().count() as i32 * 6;
        Self {
            base: WidgetBase::new(x, y, width, 9, message),
            color: 0xFFFFFFFFu32 as i32,
            shadow: true,
        }
    }
}
impl Renderable for Label {
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
        if self.base.visible {
            graphics.text_with_shadow(
                ResourceLocation::from_string("minecraft:default"),
                self.base.message().get_string(),
                self.base.x(),
                self.base.y() + (self.base.height() - 9) / 2,
                self.color,
                self.shadow,
            );
        }
    }
}
impl TabOrderedElement for Label {}
impl GuiEventListener for Label {
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        None
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
impl NarratableEntry for Label {
    fn is_active(&self) -> bool {
        self.base.is_active()
    }
}
impl LayoutElement for Label {
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
