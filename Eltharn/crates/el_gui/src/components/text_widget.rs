use crate::FontWidthMeasurer;
use crate::components::{
    GuiEventListener, LayoutElement, NarratableEntry, Renderable, TabOrderedElement, WidgetBase,
    WidgetClickSound, click_sound_sink,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::narration::{NarratedElementType, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::input::events::{CursorType, KeyEvent, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
use std::cell::{Cell, RefCell};
use std::sync::Arc;
pub const TEXT_MARGIN: i32 = 2;
pub const LINE_HEIGHT: i32 = 9;
pub const ELLIPSIS: &str = "...";
pub const DEFAULT_TOTAL_PADDING: i32 = 8;
pub const INNER_PADDING: i32 = 4;
pub const SCROLLBAR_WIDTH: i32 = 6;
pub const SCROLLBAR_MIN_HEIGHT: i32 = 32;
const DEFAULT_FONT: &str = "minecraft:default";
const WHITE: i32 = 0xFFFF_FFFFu32 as i32;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextAlignment {
    #[default]
    Left,
    Center,
    Right,
}
impl TextAlignment {
    pub fn calculate_left(self, anchor: i32, width: i32) -> i32 {
        match self {
            Self::Left => anchor,
            Self::Center => anchor - width / 2,
            Self::Right => anchor - width,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextOverflow {
    #[default]
    Clamped,
    Scrolling,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HoveredTextEffects {
    #[default]
    None,
    TooltipOnly,
    TooltipAndCursor,
}
impl HoveredTextEffects {
    pub const fn allow_tooltip(self) -> bool {
        matches!(self, Self::TooltipOnly | Self::TooltipAndCursor)
    }
    pub const fn allow_cursor_changes(self) -> bool {
        matches!(self, Self::TooltipAndCursor)
    }
    pub const fn not_clickable(can_tooltip: bool) -> Self {
        if can_tooltip {
            Self::TooltipOnly
        } else {
            Self::None
        }
    }
}
fn font_width(font: &dyn FontWidthMeasurer, text: &str) -> i32 {
    crate::formatted_text::width(text, font).ceil() as i32
}
fn plain_substr_by_width(font: &dyn FontWidthMeasurer, text: &str, max_width: i32) -> String {
    if crate::formatted_text::has_formatting(text) {
        return crate::formatted_text::truncate(text, max_width as f32, font);
    }
    let mut remaining = max_width as f32;
    let mut end = 0usize;
    for (offset, ch) in text.char_indices() {
        remaining -= font.measure_char(ch, 1.0);
        if remaining >= 0.0 {
            end = offset + ch.len_utf8();
        } else {
            break;
        }
    }
    text[..end].to_string()
}
fn clip_text(font: &dyn FontWidthMeasurer, text: &str, width: i32) -> String {
    let clipped = plain_substr_by_width(font, text, width - font_width(font, ELLIPSIS));
    format!("{clipped}{ELLIPSIS}")
}
fn strip_color(contents: &str) -> &str {
    contents
}
type LineVisitor<'a> = &'a mut dyn FnMut(&str, i32, i32);
pub trait MultiLineLabel {
    fn visit_lines(
        &self,
        align: TextAlignment,
        anchor_x: i32,
        top_y: i32,
        visitor: LineVisitor<'_>,
    ) -> i32;
    fn get_line_count(&self) -> i32;
    fn get_width(&self) -> i32;
    fn get_max_width(&self) -> i32;
    fn set_max_width(&mut self, max_width: i32);
    fn set_centered(&mut self, centered: bool);
    fn is_centered(&self) -> bool;
    fn get_line_height(&self) -> i32;
    fn set_line_height(&mut self, line_height: i32);
    fn text_color(&self) -> i32;
    fn set_color(&mut self, color: i32);
    fn has_shadow(&self) -> bool;
    fn set_shadow(&mut self, shadow: bool);
    fn narration_message(&self) -> String;
}
#[derive(Clone)]
pub struct PlainMultiLineLabel {
    lines: Vec<String>,
    line_widths: Vec<i32>,
    max_width: i32,
    width: i32,
    max_rows: Option<i32>,
    centered: bool,
    line_height: i32,
    color: i32,
    shadow: bool,
    font: Arc<dyn FontWidthMeasurer>,
    message: Component,
}
impl std::fmt::Debug for PlainMultiLineLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlainMultiLineLabel")
            .field("lines", &self.lines)
            .field("line_widths", &self.line_widths)
            .field("max_width", &self.max_width)
            .field("width", &self.width)
            .field("max_rows", &self.max_rows)
            .field("centered", &self.centered)
            .field("line_height", &self.line_height)
            .field("color", &self.color)
            .field("shadow", &self.shadow)
            .field("message", &self.message.get_string())
            .finish()
    }
}
impl PlainMultiLineLabel {
    pub fn empty() -> Self {
        Self {
            lines: Vec::new(),
            line_widths: Vec::new(),
            max_width: 0,
            width: 0,
            max_rows: None,
            centered: false,
            line_height: LINE_HEIGHT,
            color: WHITE,
            shadow: true,
            font: default_font(),
            message: Component::literal(""),
        }
    }
    pub fn create(font: Arc<dyn FontWidthMeasurer>, message: &Component, max_width: i32) -> Self {
        Self::create_with_rows(font, message, max_width, None)
    }
    pub fn create_with_rows(
        font: Arc<dyn FontWidthMeasurer>,
        message: &Component,
        max_width: i32,
        max_rows: Option<i32>,
    ) -> Self {
        Self::assemble(font, message, max_width, max_rows, false)
    }
    fn assemble(
        font: Arc<dyn FontWidthMeasurer>,
        message: &Component,
        max_width: i32,
        max_rows: Option<i32>,
        centered: bool,
    ) -> Self {
        let font_ref = font.as_ref();
        let owned = message.get_string();
        let text = strip_color(&owned);
        let split = crate::wrap_text(text, max_width.max(1) as f32, font_ref);
        let limit = max_rows.unwrap_or(i32::MAX).max(0) as usize;
        let clipped = split.len() > limit;
        let mut lines: Vec<String> = split.into_iter().take(limit).collect();
        if clipped {
            if let Some(last) = lines.last_mut() {
                *last = clip_text(font_ref, last, font_width(font_ref, last));
            }
        }
        let line_widths = lines
            .iter()
            .map(|line| font_width(font_ref, line))
            .collect();
        let mut label = Self {
            lines,
            line_widths,
            max_width,
            width: 0,
            max_rows,
            centered,
            line_height: LINE_HEIGHT,
            color: WHITE,
            shadow: true,
            font,
            message: message.clone(),
        };
        label.recompute_width();
        label
    }
    fn recompute_width(&mut self) {
        let widest = self.line_widths.iter().copied().max().unwrap_or(0);
        self.width = widest.min(self.max_width);
    }
    pub fn lines(&self) -> &[String] {
        &self.lines
    }
    pub fn line_widths(&self) -> &[i32] {
        &self.line_widths
    }
    pub fn message(&self) -> &Component {
        &self.message
    }
}
impl MultiLineLabel for PlainMultiLineLabel {
    fn visit_lines(
        &self,
        align: TextAlignment,
        anchor_x: i32,
        top_y: i32,
        visitor: LineVisitor<'_>,
    ) -> i32 {
        let mut y = top_y;
        for (line, width) in self.lines.iter().zip(self.line_widths.iter()) {
            let left_x = align.calculate_left(anchor_x, *width);
            visitor(line, left_x, y);
            y += self.line_height;
        }
        y
    }
    fn get_line_count(&self) -> i32 {
        self.lines.len() as i32
    }
    fn get_width(&self) -> i32 {
        self.width
    }
    fn get_max_width(&self) -> i32 {
        self.max_width
    }
    fn set_max_width(&mut self, max_width: i32) {
        let font = self.font.clone();
        let mut rebuilt =
            Self::assemble(font, &self.message, max_width, self.max_rows, self.centered);
        rebuilt.line_height = self.line_height;
        rebuilt.color = self.color;
        rebuilt.shadow = self.shadow;
        *self = rebuilt;
    }
    fn set_centered(&mut self, centered: bool) {
        self.centered = centered;
    }
    fn is_centered(&self) -> bool {
        self.centered
    }
    fn get_line_height(&self) -> i32 {
        self.line_height
    }
    fn set_line_height(&mut self, line_height: i32) {
        self.line_height = line_height;
    }
    fn text_color(&self) -> i32 {
        self.color
    }
    fn set_color(&mut self, color: i32) {
        self.color = color;
    }
    fn has_shadow(&self) -> bool {
        self.shadow
    }
    fn set_shadow(&mut self, shadow: bool) {
        self.shadow = shadow;
    }
    fn narration_message(&self) -> String {
        wrap_default_narration_message(&self.message)
    }
}
fn wrap_default_narration_message(message: &Component) -> String {
    format!("gui.narrate.button {}", message.get_string())
}
struct SixPixelStandIn;
impl FontWidthMeasurer for SixPixelStandIn {
    fn measure_width(&self, text: &str, scale: f32) -> f32 {
        text.chars().count() as f32 * 6.0 * scale
    }
}
fn default_font() -> Arc<dyn FontWidthMeasurer> {
    Arc::new(SixPixelStandIn)
}
#[derive(Clone)]
pub struct StringWidget {
    pub base: WidgetBase,
    pub shadow: bool,
    pub color: i32,
    max_width: i32,
    text_overflow: TextOverflow,
    cached_width: Cell<i32>,
    cached_width_dirty: Cell<bool>,
    font: Arc<dyn FontWidthMeasurer>,
    component_click_handler: Option<Arc<dyn Fn()>>,
}
impl StringWidget {
    pub fn new(x: i32, y: i32, width: i32, height: i32, message: Component) -> Self {
        Self {
            base: WidgetBase::new(x, y, width, height, message),
            shadow: true,
            color: WHITE,
            max_width: 0,
            text_overflow: TextOverflow::Clamped,
            cached_width: Cell::new(0),
            cached_width_dirty: Cell::new(true),
            font: default_font(),
            component_click_handler: None,
        }
        .deactivated()
    }
    fn deactivated(mut self) -> Self {
        self.base.active = false;
        self
    }
    pub fn from_message(message: Component) -> Self {
        let font = default_font();
        let width = font_width(font.as_ref(), &message.get_string());
        let mut widget = Self::new(0, 0, width, LINE_HEIGHT, message);
        widget.font = font;
        widget
    }
    pub fn with_size(width: i32, height: i32, message: Component) -> Self {
        Self::new(0, 0, width, height, message)
    }
    pub fn with_font(mut self, font: Arc<dyn FontWidthMeasurer>) -> Self {
        self.set_font(font);
        self
    }
    pub fn set_font(&mut self, font: Arc<dyn FontWidthMeasurer>) {
        self.font = font;
        self.cached_width_dirty.set(true);
    }
    pub fn set_max_width(&mut self, max_width: i32) {
        self.max_width = max_width;
        self.cached_width_dirty.set(true);
    }
    pub fn set_max_width_with_overflow(&mut self, max_width: i32, text_overflow: TextOverflow) {
        self.set_max_width(max_width);
        self.text_overflow = text_overflow;
    }
    pub fn max_width(&self) -> i32 {
        self.max_width
    }
    pub fn text_overflow(&self) -> TextOverflow {
        self.text_overflow
    }
    pub fn set_text_overflow(&mut self, text_overflow: TextOverflow) {
        self.text_overflow = text_overflow;
    }
    pub fn set_component_click_handler(&mut self, handler: Option<Arc<dyn Fn()>>) {
        self.component_click_handler = handler;
    }
    pub fn with_component_click_handler(mut self, handler: Arc<dyn Fn()>) -> Self {
        self.set_component_click_handler(Some(handler));
        self
    }
    pub fn component_click_handler(&self) -> Option<&Arc<dyn Fn()>> {
        self.component_click_handler.as_ref()
    }
    pub fn hovered_text_effects(&self) -> HoveredTextEffects {
        if self.base.hovered() {
            if self.component_click_handler.is_some() {
                HoveredTextEffects::TooltipAndCursor
            } else {
                HoveredTextEffects::TooltipOnly
            }
        } else {
            HoveredTextEffects::None
        }
    }
    fn on_click(&mut self, _event: MouseButtonEvent, _double_click: bool) {}
    pub fn set_message(&mut self, message: Component) {
        self.base.set_message(message.clone());
        let width = font_width(self.font.as_ref(), &message.get_string());
        self.base.set_width(width);
        self.cached_width_dirty.set(true);
    }
    pub fn get_width(&self) -> i32 {
        if self.max_width <= 0 {
            return self.base.width();
        }
        if self.cached_width_dirty.get() {
            let measured = font_width(self.font.as_ref(), &self.base.message().get_string());
            self.cached_width.set(self.max_width.min(measured));
            self.cached_width_dirty.set(false);
        }
        self.cached_width.get()
    }
    pub fn get_height(&self) -> i32 {
        self.base.height()
    }
    pub fn overflows(&self) -> bool {
        font_width(self.font.as_ref(), &self.base.message().get_string())
            > self.effective_max_width()
    }
    fn effective_max_width(&self) -> i32 {
        if self.max_width > 0 {
            self.max_width
        } else {
            self.get_width()
        }
    }
    fn text_y(&self) -> i32 {
        self.base.y() + (self.base.height() - LINE_HEIGHT) / 2
    }
    fn rows(&self) -> Vec<String> {
        let message = self.base.message().clone();
        let text = message.get_string();
        let max_width = self.effective_max_width();
        if font_width(self.font.as_ref(), &text) > max_width
            && self.text_overflow == TextOverflow::Clamped
        {
            vec![clip_text(self.font.as_ref(), &text, max_width)]
        } else {
            vec![text]
        }
    }
    fn contains_point(&self, mouse_x: f64, mouse_y: f64) -> bool {
        let left = self.base.x() as f64;
        let top = self.base.y() as f64;
        let right = (self.base.x() + self.get_width()) as f64;
        let bottom = (self.base.y() + self.get_height()) as f64;
        mouse_x >= left && mouse_y >= top && mouse_x < right && mouse_y < bottom
    }
}
impl std::fmt::Debug for StringWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StringWidget")
            .field("x", &self.base.x())
            .field("y", &self.base.y())
            .field("width", &self.get_width())
            .field("height", &self.get_height())
            .field("message", &self.base.message().get_string())
            .field("color", &self.color)
            .field("shadow", &self.shadow)
            .field("max_width", &self.max_width)
            .field("text_overflow", &self.text_overflow)
            .field("click_handler", &self.component_click_handler.is_some())
            .finish()
    }
}
impl Renderable for StringWidget {
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
                && self.contains_point(mouse_x as f64, mouse_y as f64),
        );
        let font = self.font.clone();
        let color = ((crate::framework::graphics::argb_white(self.base.alpha) as u32)
            & 0xFF00_0000)
            | (self.color as u32 & 0x00FF_FFFF);
        let color = color as i32;
        let shadow = self.shadow;
        let x = self.base.x();
        let y = self.text_y();
        if self.overflows() && self.text_overflow == TextOverflow::Scrolling {
            let right = x + self.get_width();
            graphics.draw_scrolling_text_with_default_center(
                font.as_ref(),
                &self.base.message().get_string(),
                x + TEXT_MARGIN,
                right - TEXT_MARGIN,
                self.base.y(),
                self.base.y() + self.base.height(),
                color,
            );
            return;
        }
        for row in self.rows() {
            graphics.text_with_shadow(
                ResourceLocation::from_string(DEFAULT_FONT),
                row,
                x,
                y,
                color,
                shadow,
            );
        }
    }
}
impl TabOrderedElement for StringWidget {}
impl GuiEventListener for StringWidget {
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        Some(WidgetClickSound::UiButtonClick)
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        if !self.base.is_active() {
            return false;
        }
        if event.button() != 0 {
            return false;
        }
        if !self.is_mouse_over(event.x, event.y) {
            return false;
        }
        self.on_click(event, double_click);
        true
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.base.is_active() && self.contains_point(mouse_x, mouse_y)
    }
    fn set_focused(&mut self, focused: bool) {
        self.base.set_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.base.focused()
    }
    fn next_focus_path(&self, _navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.base.is_active().then(|| ComponentPath::leaf(0))
    }
    fn rectangle(&self) -> ScreenRectangle {
        ScreenRectangle::new(
            self.base.x(),
            self.base.y(),
            self.get_width(),
            self.get_height(),
        )
    }
}
impl NarratableEntry for StringWidget {
    fn narration_priority(&self) -> NarrationPriority {
        NarrationPriority::None
    }
}
impl LayoutElement for StringWidget {
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
        self.get_width()
    }
    fn height(&self) -> i32 {
        self.get_height()
    }
}
pub struct StringWidgetBuilder {
    message: Component,
    x: i32,
    y: i32,
    width: Option<i32>,
    height: i32,
    max_width: Option<i32>,
    text_overflow: TextOverflow,
    color: i32,
    shadow: bool,
    font: Arc<dyn FontWidthMeasurer>,
}
impl StringWidgetBuilder {
    pub fn new(message: Component) -> Self {
        Self {
            message,
            x: 0,
            y: 0,
            width: None,
            height: LINE_HEIGHT,
            max_width: None,
            text_overflow: TextOverflow::Clamped,
            color: WHITE,
            shadow: true,
            font: default_font(),
        }
    }
    pub fn pos(mut self, x: i32, y: i32) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    pub fn width(mut self, width: i32) -> Self {
        self.width = Some(width);
        self
    }
    pub fn height(mut self, height: i32) -> Self {
        self.height = height;
        self
    }
    pub fn max_width(mut self, max_width: i32) -> Self {
        self.max_width = Some(max_width);
        self
    }
    pub fn text_overflow(mut self, text_overflow: TextOverflow) -> Self {
        self.text_overflow = text_overflow;
        self
    }
    pub fn with_font(mut self, font: Arc<dyn FontWidthMeasurer>) -> Self {
        self.font = font;
        self
    }
    pub fn color(mut self, color: i32) -> Self {
        self.color = color;
        self
    }
    pub fn shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }
    pub fn build(self) -> StringWidget {
        let font = self.font.clone();
        let width = match self.width {
            Some(width) => width,
            None => font_width(font.as_ref(), &self.message.get_string()),
        };
        let mut widget = StringWidget::new(self.x, self.y, width, self.height, self.message);
        widget.font = font;
        widget.color = self.color;
        widget.shadow = self.shadow;
        if let Some(max_width) = self.max_width {
            widget.set_max_width_with_overflow(max_width, self.text_overflow);
        }
        widget
    }
}
#[derive(Clone)]
pub struct MultiLineTextWidget {
    pub base: WidgetBase,
    pub shadow: bool,
    pub color: i32,
    max_width: Option<i32>,
    max_rows: Option<i32>,
    centered: bool,
    line_height: i32,
    label: RefCell<PlainMultiLineLabel>,
    cache_key: RefCell<Option<LabelCacheKey>>,
    cache_dirty: Cell<bool>,
    font: Arc<dyn FontWidthMeasurer>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct LabelCacheKey {
    message: String,
    max_width: Option<i32>,
    max_rows: Option<i32>,
}
impl MultiLineTextWidget {
    pub fn new(x: i32, y: i32, message: Component) -> Self {
        let font = default_font();
        let label = PlainMultiLineLabel::create_with_rows(font.clone(), &message, i32::MAX, None);
        let key = LabelCacheKey {
            message: message.get_string(),
            max_width: None,
            max_rows: None,
        };
        Self {
            base: WidgetBase::new(x, y, 0, 0, message),
            shadow: true,
            color: WHITE,
            max_width: None,
            max_rows: None,
            centered: false,
            line_height: LINE_HEIGHT,
            label: RefCell::new(label),
            cache_key: RefCell::new(Some(key)),
            cache_dirty: Cell::new(false),
            font,
        }
        .deactivated()
    }
    fn deactivated(mut self) -> Self {
        self.base.active = false;
        self
    }
    pub fn from_message(message: Component) -> Self {
        Self::new(0, 0, message)
    }
    pub fn with_font(mut self, font: Arc<dyn FontWidthMeasurer>) -> Self {
        self.set_font(font);
        self
    }
    pub fn set_font(&mut self, font: Arc<dyn FontWidthMeasurer>) {
        self.font = font;
        self.cache_dirty.set(true);
    }
    pub fn set_max_width(&mut self, max_width: i32) {
        self.max_width = Some(max_width);
        self.cache_dirty.set(true);
    }
    pub fn max_width(&self) -> Option<i32> {
        self.max_width
    }
    pub fn set_max_rows(&mut self, max_rows: i32) {
        self.max_rows = Some(max_rows);
        self.cache_dirty.set(true);
    }
    pub fn max_rows(&self) -> Option<i32> {
        self.max_rows
    }
    pub fn set_centered(&mut self, centered: bool) {
        self.centered = centered;
    }
    pub fn is_centered(&self) -> bool {
        self.centered
    }
    pub fn set_message(&mut self, message: Component) {
        self.base.set_message(message);
        self.cache_dirty.set(true);
    }
    pub fn get_width(&self) -> i32 {
        self.label_ref().get_width()
    }
    pub fn get_height(&self) -> i32 {
        self.label_ref().get_line_count() * self.line_height
    }
    pub fn text_x(&self) -> i32 {
        self.base.x()
    }
    pub fn text_y(&self) -> i32 {
        self.base.y()
    }
    pub fn visit_alignment(&self) -> TextAlignment {
        if self.centered {
            TextAlignment::Center
        } else {
            TextAlignment::Left
        }
    }
    pub fn lines(&self) -> Vec<String> {
        self.label_ref().lines().to_vec()
    }
    fn label_ref(&self) -> std::cell::Ref<'_, PlainMultiLineLabel> {
        let key = LabelCacheKey {
            message: self.base.message().get_string(),
            max_width: self.max_width,
            max_rows: self.max_rows,
        };
        let stale = if self.cache_dirty.get() {
            true
        } else {
            self.cache_key.borrow().as_ref() != Some(&key)
        };
        if stale {
            let label = PlainMultiLineLabel::create_with_rows(
                self.font.clone(),
                self.base.message(),
                self.max_width.unwrap_or(i32::MAX),
                self.max_rows,
            );
            *self.label.borrow_mut() = label;
            *self.cache_key.borrow_mut() = Some(key);
            self.cache_dirty.set(false);
        }
        self.label.borrow()
    }
    pub fn cache_is_dirty(&self) -> bool {
        self.cache_dirty.get() || {
            let key = LabelCacheKey {
                message: self.base.message().get_string(),
                max_width: self.max_width,
                max_rows: self.max_rows,
            };
            self.cache_key.borrow().as_ref() != Some(&key)
        }
    }
    fn contains_point(&self, mouse_x: f64, mouse_y: f64) -> bool {
        let left = self.base.x() as f64;
        let top = self.base.y() as f64;
        let right = (self.base.x() + self.get_width()) as f64;
        let bottom = (self.base.y() + self.get_height()) as f64;
        mouse_x >= left && mouse_y >= top && mouse_x < right && mouse_y < bottom
    }
}
impl std::fmt::Debug for MultiLineTextWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiLineTextWidget")
            .field("x", &self.base.x())
            .field("y", &self.base.y())
            .field("width", &self.get_width())
            .field("height", &self.get_height())
            .field("message", &self.base.message().get_string())
            .field("max_width", &self.max_width)
            .field("max_rows", &self.max_rows)
            .field("centered", &self.centered)
            .field("color", &self.color)
            .field("shadow", &self.shadow)
            .finish()
    }
}
impl MultiLineLabel for MultiLineTextWidget {
    fn visit_lines(
        &self,
        align: TextAlignment,
        anchor_x: i32,
        top_y: i32,
        visitor: LineVisitor<'_>,
    ) -> i32 {
        self.label_ref()
            .visit_lines(align, anchor_x, top_y, visitor)
    }
    fn get_line_count(&self) -> i32 {
        self.label_ref().get_line_count()
    }
    fn get_width(&self) -> i32 {
        self.label_ref().get_width()
    }
    fn get_max_width(&self) -> i32 {
        self.max_width.unwrap_or(i32::MAX)
    }
    fn set_max_width(&mut self, max_width: i32) {
        self.set_max_width(max_width);
    }
    fn set_centered(&mut self, centered: bool) {
        self.set_centered(centered);
    }
    fn is_centered(&self) -> bool {
        self.is_centered()
    }
    fn get_line_height(&self) -> i32 {
        self.line_height
    }
    fn set_line_height(&mut self, line_height: i32) {
        self.line_height = line_height;
    }
    fn text_color(&self) -> i32 {
        self.color
    }
    fn set_color(&mut self, color: i32) {
        self.color = color;
    }
    fn has_shadow(&self) -> bool {
        self.shadow
    }
    fn set_shadow(&mut self, shadow: bool) {
        self.shadow = shadow;
    }
    fn narration_message(&self) -> String {
        wrap_default_narration_message(self.base.message())
    }
}
impl Renderable for MultiLineTextWidget {
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
                && self.contains_point(mouse_x as f64, mouse_y as f64),
        );
        let font = ResourceLocation::from_string(DEFAULT_FONT);
        let color = ((crate::framework::graphics::argb_white(self.base.alpha) as u32)
            & 0xFF00_0000)
            | (self.color as u32 & 0x00FF_FFFF);
        let color = color as i32;
        let shadow = self.shadow;
        let align = self.visit_alignment();
        let anchor_x = if self.centered {
            self.base.x() + self.get_width() / 2
        } else {
            self.text_x()
        };
        let top_y = self.text_y();
        self.visit_lines(align, anchor_x, top_y, &mut |row, left_x, row_y| {
            graphics.text_with_shadow(font.clone(), row.to_string(), left_x, row_y, color, shadow);
        });
    }
}
impl TabOrderedElement for MultiLineTextWidget {}
impl GuiEventListener for MultiLineTextWidget {
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        Some(WidgetClickSound::UiButtonClick)
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.base.is_active() && self.contains_point(mouse_x, mouse_y)
    }
    fn set_focused(&mut self, focused: bool) {
        self.base.set_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.base.focused()
    }
    fn next_focus_path(&self, _navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.base.is_active().then(|| ComponentPath::leaf(0))
    }
    fn rectangle(&self) -> ScreenRectangle {
        ScreenRectangle::new(
            self.base.x(),
            self.base.y(),
            self.get_width(),
            self.get_height(),
        )
    }
}
impl NarratableEntry for MultiLineTextWidget {
    fn narration_priority(&self) -> NarrationPriority {
        NarrationPriority::None
    }
}
impl LayoutElement for MultiLineTextWidget {
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
        self.get_width()
    }
    fn height(&self) -> i32 {
        self.get_height()
    }
}
#[derive(Clone)]
pub struct FittingMultiLineTextWidget {
    pub base: WidgetBase,
    multiline: MultiLineTextWidget,
    scroll_amount: f64,
    scrolling: bool,
    show_background: bool,
    show_decorations: bool,
    inner_padding: i32,
}
impl FittingMultiLineTextWidget {
    pub fn new(x: i32, y: i32, width: i32, height: i32, message: Component) -> Self {
        Self::with_options(x, y, width, height, message, true, true)
    }
    pub fn with_options(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        message: Component,
        show_background: bool,
        show_decorations: bool,
    ) -> Self {
        let mut multiline =
            MultiLineTextWidget::new(x + INNER_PADDING, y + INNER_PADDING, message.clone());
        multiline.set_max_width(width - INNER_PADDING * 2);
        Self {
            base: WidgetBase::new(x, y, width, height, message),
            multiline,
            scroll_amount: 0.0,
            scrolling: false,
            show_background,
            show_decorations,
            inner_padding: INNER_PADDING,
        }
    }
    pub fn with_font(mut self, font: Arc<dyn FontWidthMeasurer>) -> Self {
        self.multiline.set_font(font);
        self
    }
    pub fn set_font(&mut self, font: Arc<dyn FontWidthMeasurer>) {
        self.multiline.set_font(font);
    }
    pub fn inner_padding(&self) -> i32 {
        self.inner_padding
    }
    pub fn total_inner_padding(&self) -> i32 {
        self.inner_padding * 2
    }
    pub fn inner_left(&self) -> i32 {
        self.base.x() + self.inner_padding
    }
    pub fn inner_top(&self) -> i32 {
        self.base.y() + self.inner_padding
    }
    pub fn multiline(&self) -> &MultiLineTextWidget {
        &self.multiline
    }
    pub fn set_message(&mut self, message: Component) {
        self.base.set_message(message.clone());
        self.multiline.set_message(message);
    }
    pub fn show_decorations(&self) -> bool {
        self.show_decorations
    }
    pub fn set_scroll_amount(&mut self, scroll_amount: f64) {
        let max = self.max_scroll_amount() as f64;
        self.scroll_amount = scroll_amount.clamp(0.0, max);
    }
    pub fn set_widget_width(&mut self, width: i32) {
        self.base.set_width(width);
    }
    pub fn set_widget_height(&mut self, height: i32) {
        self.base.set_height(height);
    }
    pub fn set_width(&mut self, width: i32) {
        self.set_widget_width(width);
        self.multiline
            .set_max_width(width - self.total_inner_padding());
    }
    fn content_top(&self) -> i32 {
        self.inner_top() + (-(self.scroll_amount)) as i32
    }
    fn position_inner(&mut self) {
        let x = self.inner_left();
        let y = self.content_top();
        self.multiline.base.set_x(x);
        self.multiline.base.set_y(y);
    }
    pub fn inner_height(&self) -> i32 {
        self.multiline.get_height()
    }
    pub fn content_height(&self) -> i32 {
        self.inner_height() + self.total_inner_padding()
    }
    pub fn minimize_height(&mut self) {
        if !self.showing_scroll_bar() {
            self.base
                .set_height(self.inner_height() + self.total_inner_padding());
        }
    }
    pub fn scrollable(&self) -> bool {
        self.max_scroll_amount() > 0
    }
    pub fn showing_scroll_bar(&self) -> bool {
        self.scrollable()
    }
    pub fn max_scroll_amount(&self) -> i32 {
        (self.content_height() - self.base.height()).max(0)
    }
    pub fn scroll_amount(&self) -> f64 {
        self.scroll_amount
    }
    pub fn refresh_scroll_amount(&mut self) {
        self.set_scroll_amount(self.scroll_amount);
    }
    pub fn scrollbar_width(&self) -> i32 {
        SCROLLBAR_WIDTH
    }
    pub fn scroll_bar_x(&self) -> i32 {
        self.base.x() + self.base.width()
    }
    pub fn scroller_height(&self) -> i32 {
        let ratio = (self.base.height() * self.base.height()) / self.content_height().max(1);
        let min = SCROLLBAR_MIN_HEIGHT;
        let max = (self.base.height() - 2 * INNER_PADDING).max(0);
        max.min(ratio.max(min))
    }
    pub fn scroll_bar_y(&self) -> i32 {
        let max = self.max_scroll_amount();
        if max == 0 {
            return self.base.y();
        }
        let offset = (self.scroll_amount * (self.base.height() - self.scroller_height()) as f64
            / max as f64) as i32;
        self.base.y().max(self.base.y() + offset)
    }
    pub fn is_over_scrollbar(&self, x: f64, y: f64) -> bool {
        let bar_left = self.scroll_bar_x() as f64;
        let bar_right = (self.scroll_bar_x() + self.scrollbar_width()) as f64;
        let top = self.base.y() as f64;
        let bottom = (self.base.y() + self.base.height()) as f64;
        x >= bar_left && x <= bar_right && y >= top && y < bottom
    }
    fn contains_point(&self, mouse_x: f64, mouse_y: f64) -> bool {
        let left = self.base.x() as f64;
        let top = self.base.y() as f64;
        let right = (self.base.x() + self.base.width() + self.scrollbar_width()) as f64;
        let bottom = (self.base.y() + self.base.height()) as f64;
        mouse_x >= left && mouse_y >= top && mouse_x < right && mouse_y < bottom
    }
    pub fn mouse_scrolled(&mut self, scroll_y: f64) -> bool {
        if !self.base.visible {
            return false;
        }
        self.set_scroll_amount(self.scroll_amount - scroll_y * self.scroll_rate());
        true
    }
    pub fn scroll_rate(&self) -> f64 {
        9.0
    }
    pub fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        let scrolling = self.update_scrolling(event);
        let clicked = self.is_mouse_over(event.x, event.y) && event.button() == 0;
        if clicked {
            click_sound_sink::record(self.play_down_sound());
        }
        clicked || scrolling
    }
    pub fn update_scrolling(&mut self, event: MouseButtonEvent) -> bool {
        self.scrolling =
            self.scrollable() && event.button() == 0 && self.is_over_scrollbar(event.x, event.y);
        self.scrolling
    }
    pub fn mouse_dragged(&mut self, event: MouseButtonEvent, _dy: f64) -> bool {
        if self.scrolling {
            if event.y < self.base.y() as f64 {
                self.set_scroll_amount(0.0);
            } else if event.y > (self.base.y() + self.base.height()) as f64 {
                self.set_scroll_amount(self.max_scroll_amount() as f64);
            } else {
                let max = self.max_scroll_amount().max(1) as f64;
                let drag_scale =
                    (max / (self.base.height() - self.scroller_height()).max(1) as f64).max(1.0);
                self.set_scroll_amount(self.scroll_amount + _dy * drag_scale);
            }
            true
        } else {
            false
        }
    }
    pub fn release(&mut self) {
        self.scrolling = false;
    }
}
impl std::fmt::Debug for FittingMultiLineTextWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FittingMultiLineTextWidget")
            .field("x", &self.base.x())
            .field("y", &self.base.y())
            .field("width", &self.base.width())
            .field("height", &self.base.height())
            .field("inner_height", &self.inner_height())
            .field("content_height", &self.content_height())
            .field("scroll_amount", &self.scroll_amount)
            .field("scrollable", &self.scrollable())
            .field("message", &self.base.message().get_string())
            .field("multiline", &self.multiline)
            .finish()
    }
}
impl Renderable for FittingMultiLineTextWidget {
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
                && self.contains_point(mouse_x as f64, mouse_y as f64),
        );
        if self.show_background {
            self.extract_background(graphics);
        }
        let x = self.base.x();
        let y = self.base.y();
        let width = self.base.width();
        let height = self.base.height();
        graphics.enable_scissor(x + 1, y + 1, x + width - 1, y + height - 1);
        self.position_inner();
        self.multiline
            .extract_render_state(graphics, mouse_x, mouse_y, _partial_tick);
        graphics.disable_scissor();
        self.extract_scrollbar(graphics, mouse_x, mouse_y);
    }
}
impl FittingMultiLineTextWidget {
    fn extract_background(&self, graphics: &mut GuiGraphicsExtractor) {
        let sprite = if self.base.is_active() && self.base.focused() {
            "minecraft:widget/text_field_highlighted"
        } else {
            "minecraft:widget/text_field"
        };
        graphics.blit(
            ResourceLocation::from_string(sprite),
            self.base.x(),
            self.base.y(),
            self.base.width(),
            self.base.height(),
        );
    }
    fn extract_scrollbar(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
    ) {
        let x = self.scroll_bar_x();
        let y = self.base.y();
        let width = self.scrollbar_width();
        let height = self.base.height();
        let scroller_height = self.scroller_height();
        let over = self.is_over_scrollbar(mouse_x as f64, mouse_y as f64);
        if self.scrollable() {
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller_background"),
                x,
                y,
                width,
                height,
            );
            let scroller_y = self.scroll_bar_y();
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller"),
                x,
                scroller_y,
                width,
                scroller_height,
            );
            if over {
                graphics.request_cursor(if self.scrolling {
                    CursorType::ResizeNs
                } else {
                    CursorType::PointingHand
                });
            }
        } else if over {
            graphics.request_cursor(CursorType::NotAllowed);
        }
    }
}
impl TabOrderedElement for FittingMultiLineTextWidget {}
impl GuiEventListener for FittingMultiLineTextWidget {
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        None
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.base.is_active() && self.contains_point(mouse_x, mouse_y)
    }
    fn set_focused(&mut self, focused: bool) {
        self.base.set_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.base.focused()
    }
    fn mouse_scrolled(&mut self, _x: f64, _y: f64, _scroll_x: f64, scroll_y: f64) -> bool {
        self.mouse_scrolled(scroll_y)
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.mouse_clicked(event, double_click)
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, _dx: f64, dy: f64) -> bool {
        self.mouse_dragged(event, dy)
    }
    fn mouse_released(&mut self, _event: MouseButtonEvent) -> bool {
        self.release();
        true
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        let is_up = event.key == KeyEvent::KEY_UP;
        let is_down = event.key == KeyEvent::KEY_DOWN;
        if is_up || is_down {
            let previous = self.scroll_amount;
            self.set_scroll_amount(
                self.scroll_amount + if is_up { -1.0 } else { 1.0 } * self.scroll_rate(),
            );
            if previous != self.scroll_amount {
                return true;
            }
        }
        false
    }
    fn next_focus_path(&self, _navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.base.is_active().then(|| ComponentPath::leaf(0))
    }
    fn rectangle(&self) -> ScreenRectangle {
        self.base.rectangle()
    }
}
impl NarratableEntry for FittingMultiLineTextWidget {
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
        output.add(NarratedElementType::Title, self.base.message().get_string());
    }
}
impl LayoutElement for FittingMultiLineTextWidget {
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
