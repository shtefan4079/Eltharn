use crate::FontWidthMeasurer;
use crate::components::click_sound_sink;
use crate::components::{
    GuiEventListener, Renderable, TabOrderedElement, WidgetBase, WidgetClickSound,
    WidgetTooltipHolder,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{
    NarratableEntry, NarratedElementType, NarrationElementOutput, NarrationPriority,
};
use crate::framework::navigation::{
    ComponentPath, FocusNavigationEvent, ScreenAxis, ScreenDirection, ScreenRectangle,
};
use crate::input::events::{CursorType, KeyEvent, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
use std::sync::Arc;
pub const SPACING: i32 = 4;
pub const ROWS: i32 = 2;
pub const BOX_PADDING: i32 = 8;
pub const BOX_SIZE: i32 = 9 + BOX_PADDING;
pub const LABEL_OFFSET_X: i32 = BOX_SIZE + SPACING;
const LINE_HEIGHT: i32 = 9;
pub const DISPLAY_HEIGHT: i32 = ROWS * LINE_HEIGHT;
const ELLIPSIS: &str = "...";
const LABEL_COLOR: i32 = 0xFFFF_FFFFu32 as i32;
#[derive(Clone)]
pub struct Checkbox {
    pub base: WidgetBase,
    checked: bool,
    pending_value_changed: bool,
    font: CheckboxFont,
    max_width: i32,
    checkbox_size: i32,
    display_height: i32,
    label_color: i32,
    label_lines: Vec<String>,
    tooltip_holder: WidgetTooltipHolder,
}
impl Checkbox {
    pub fn new(x: i32, y: i32, message: Component, checked: bool) -> Self {
        Self::build(CheckboxBuilder::new(message).pos(x, y).selected(checked))
    }
    pub fn builder(message: Component) -> CheckboxBuilder {
        CheckboxBuilder::new(message)
    }
    fn build(builder: CheckboxBuilder) -> Self {
        let mut checkbox = Self {
            base: WidgetBase::new(builder.x, builder.y, 0, 0, builder.message),
            checked: builder.checked,
            pending_value_changed: false,
            font: builder.font,
            max_width: builder.max_width.unwrap_or(i32::MAX),
            checkbox_size: builder.checkbox_size,
            display_height: builder.display_height,
            label_color: builder.label_color,
            label_lines: Vec::new(),
            tooltip_holder: WidgetTooltipHolder::default(),
        };
        checkbox.recalculate();
        if checkbox.overflows_row_limit() {
            checkbox.tooltip_holder.set_lines(builder.tooltip);
        }
        checkbox
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
        let font: Option<Arc<dyn FontWidthMeasurer>> = match &self.font {
            CheckboxFont::Client(font) => Some(Arc::clone(font)),
            CheckboxFont::DefaultSixPixel => None,
        };
        match font {
            Some(font) => {
                self.refresh_tooltip_holder(graphics, font.as_ref(), mouse_x, mouse_y, now_millis)
            }
            None => self.refresh_tooltip_holder(
                graphics,
                &SixPixelStandIn,
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
    pub fn with_font(mut self, font: Arc<dyn FontWidthMeasurer>) -> Self {
        self.set_font(font);
        self
    }
    pub fn set_font(&mut self, font: Arc<dyn FontWidthMeasurer>) {
        self.font = CheckboxFont::Client(font);
        self.recalculate();
    }
    pub fn with_width(mut self, max_width: i32) -> Self {
        self.set_max_width(max_width);
        self
    }
    pub fn set_max_width(&mut self, max_width: i32) {
        self.max_width = max_width;
        self.recalculate();
    }
    pub fn max_width(&self) -> i32 {
        self.max_width
    }
    pub fn selected(&self) -> bool {
        self.checked
    }
    pub fn is_checked(&self) -> bool {
        self.checked
    }
    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }
    pub fn take_value_changed(&mut self) -> bool {
        if self.pending_value_changed {
            self.pending_value_changed = false;
            true
        } else {
            false
        }
    }
    pub fn on_press(&mut self) {
        self.checked = !self.checked;
        self.pending_value_changed = true;
    }
    pub fn set_message(&mut self, message: Component) {
        self.base.set_message(message);
        self.recalculate();
    }
    pub fn with_message(mut self, message: Component) -> Self {
        self.set_message(message);
        self
    }
    pub fn box_size(&self) -> i32 {
        self.checkbox_size
    }
    pub fn label_offset_x(&self) -> i32 {
        self.box_size() + SPACING
    }
    pub fn label_height(&self) -> i32 {
        self.label_lines.len() as i32 * LINE_HEIGHT
    }
    pub fn label_lines(&self) -> &[String] {
        &self.label_lines
    }
    pub fn default_width(&self) -> i32 {
        self.box_size() + SPACING + font_width(self.font(), &self.base.message().get_string())
    }
    pub fn overflows_row_limit(&self) -> bool {
        self.split_label_lines(self.base.width()).len() > ROWS as usize
    }
    fn recalculate(&mut self) {
        let default = self.default_width();
        let width = default.min(self.max_width);
        self.base.set_width(width);
        let mut lines = self.split_label_lines(width);
        let clipped = lines.len() > ROWS as usize;
        lines.truncate(ROWS as usize);
        if clipped {
            if let Some(last) = lines.last_mut() {
                *last = ellipsize(self.font(), last);
            }
        }
        self.label_lines = lines;
        let height = self
            .box_size()
            .max(self.label_height())
            .max(self.display_height);
        self.base.set_height(height);
    }
    fn split_label_lines(&self, max_width: i32) -> Vec<String> {
        crate::wrap_text(
            &self.base.message().get_string(),
            (max_width.max(1)) as f32,
            self.font(),
        )
    }
    fn font(&self) -> &dyn FontWidthMeasurer {
        self.font.as_measurer()
    }
    fn extract_contents(&self, graphics: &mut GuiGraphicsExtractor) {
        let x = self.base.x();
        let y = self.base.y();
        let size = self.box_size();
        let checked = self.checked;
        let focused = self.base.focused();
        let hovered = self.base.hovered();
        if self.label_lines.len() <= 1 {
            let label = self.label_lines.first().map(String::as_str).unwrap_or("");
            graphics.draw_checkbox(
                self.font(),
                x,
                y,
                checked,
                focused,
                hovered,
                label,
                self.label_color,
                self.base.alpha,
            );
            return;
        }
        let channel = ((self.base.alpha.clamp(0.0, 1.0)) * 255.0) as i32;
        graphics.blit_with_alpha(
            box_sprite(checked, focused),
            x,
            y,
            size,
            size,
            self.base.alpha,
        );
        let label_color = (self.label_color & 0x00FF_FFFF) | (channel << 24);
        let text_x = x + self.label_offset_x();
        let text_y = y + size / 2 - self.label_height() / 2;
        for (index, line) in self.label_lines.iter().enumerate() {
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                line.clone(),
                text_x,
                text_y + index as i32 * LINE_HEIGHT,
                label_color,
            );
        }
    }
}
fn box_sprite(checked: bool, focused: bool) -> ResourceLocation {
    let sprite = match (checked, focused) {
        (true, true) => "minecraft:widget/checkbox_selected_highlighted",
        (true, false) => "minecraft:widget/checkbox_selected",
        (false, true) => "minecraft:widget/checkbox_highlighted",
        (false, false) => "minecraft:widget/checkbox",
    };
    ResourceLocation::from_string(sprite)
}
fn font_width(font: &dyn FontWidthMeasurer, text: &str) -> i32 {
    font.measure_width(text, 1.0).ceil() as i32
}
fn plain_substr_by_width(font: &dyn FontWidthMeasurer, text: &str, max_width: i32) -> String {
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
fn ellipsize(font: &dyn FontWidthMeasurer, line: &str) -> String {
    let budget = font_width(font, line) - font_width(font, ELLIPSIS);
    format!("{}{}", plain_substr_by_width(font, line, budget), ELLIPSIS)
}
fn java_border_for_arrow_navigation(
    rect: ScreenRectangle,
    direction: ScreenDirection,
) -> ScreenRectangle {
    let axis = direction.axis();
    let orthogonal = axis.orthogonal();
    let start_first = bound_in_direction(rect, direction);
    let start_second = bound_in_direction(rect, orthogonal.negative());
    let length = match orthogonal {
        ScreenAxis::Horizontal => rect.width,
        ScreenAxis::Vertical => rect.height,
    };
    let (x, y, width, height) = match axis {
        ScreenAxis::Horizontal => (start_first, start_second, 1, length),
        ScreenAxis::Vertical => (start_second, start_first, length, 1),
    };
    let (x, y) = match direction {
        ScreenDirection::Left => (x - 1, y),
        ScreenDirection::Right => (x + 1, y),
        ScreenDirection::Up => (x, y - 1),
        ScreenDirection::Down => (x, y + 1),
    };
    ScreenRectangle::new(x, y, width, height)
}
fn bound_in_direction(rect: ScreenRectangle, direction: ScreenDirection) -> i32 {
    let (start, length) = match direction.axis() {
        ScreenAxis::Horizontal => (rect.left(), rect.width),
        ScreenAxis::Vertical => (rect.top(), rect.height),
    };
    if direction.is_positive() {
        start + length - 1
    } else {
        start
    }
}
#[derive(Clone)]
enum CheckboxFont {
    DefaultSixPixel,
    Client(Arc<dyn FontWidthMeasurer>),
}
impl CheckboxFont {
    fn as_measurer(&self) -> &dyn FontWidthMeasurer {
        match self {
            Self::DefaultSixPixel => &SixPixelStandIn,
            Self::Client(font) => font.as_ref(),
        }
    }
}
impl std::fmt::Debug for CheckboxFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DefaultSixPixel => f.write_str("6px stand-in"),
            Self::Client(_) => f.write_str("client FontWidthMeasurer"),
        }
    }
}
struct SixPixelStandIn;
impl FontWidthMeasurer for SixPixelStandIn {
    fn measure_width(&self, text: &str, scale: f32) -> f32 {
        text.chars().count() as f32 * 6.0 * scale
    }
}
impl std::fmt::Debug for Checkbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checkbox")
            .field("checked", &self.checked)
            .field("pending_value_changed", &self.pending_value_changed)
            .field("x", &self.base.x())
            .field("y", &self.base.y())
            .field("width", &self.base.width())
            .field("height", &self.base.height())
            .field("message", &self.base.message().get_string())
            .field("label_lines", &self.label_lines)
            .field("font", &self.font)
            .finish()
    }
}
pub struct CheckboxBuilder {
    message: Component,
    font: CheckboxFont,
    x: i32,
    y: i32,
    checked: bool,
    max_width: Option<i32>,
    checkbox_size: i32,
    display_height: i32,
    label_color: i32,
    tooltip: Option<Vec<String>>,
}
impl CheckboxBuilder {
    pub fn new(message: Component) -> Self {
        Self {
            message,
            font: CheckboxFont::DefaultSixPixel,
            x: 0,
            y: 0,
            checked: false,
            max_width: None,
            checkbox_size: BOX_SIZE,
            display_height: 0,
            label_color: LABEL_COLOR,
            tooltip: None,
        }
    }
    pub fn with_font(mut self, font: Arc<dyn FontWidthMeasurer>) -> Self {
        self.font = CheckboxFont::Client(font);
        self
    }
    pub fn pos(mut self, x: i32, y: i32) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    pub fn selected(mut self, selected: bool) -> Self {
        self.checked = selected;
        self
    }
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
    pub fn max_width(mut self, max_width: i32) -> Self {
        self.max_width = Some(max_width);
        self
    }
    pub fn checkbox_size(mut self, size: i32) -> Self {
        self.checkbox_size = size;
        self
    }
    pub fn display_height(mut self, height: i32) -> Self {
        self.display_height = height;
        self
    }
    pub fn with_label_color(mut self, color: i32) -> Self {
        self.label_color = color;
        self
    }
    pub fn tooltip(mut self, lines: Vec<String>) -> Self {
        self.tooltip = Some(lines);
        self
    }
    pub fn build(self) -> Checkbox {
        Checkbox::build(self)
    }
}
impl Renderable for Checkbox {
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
        self.extract_contents(graphics);
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
impl TabOrderedElement for Checkbox {
    fn tab_order_group(&self) -> i32 {
        self.base.tab_order_group()
    }
}
impl GuiEventListener for Checkbox {
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
        self.base.is_active().then(|| ComponentPath::leaf(0))
    }
    fn rectangle(&self) -> ScreenRectangle {
        self.base.rectangle()
    }
    fn border_for_arrow_navigation(&self, opposite: ScreenDirection) -> ScreenRectangle {
        java_border_for_arrow_navigation(GuiEventListener::rectangle(self), opposite)
    }
}
impl NarratableEntry for Checkbox {
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
                if self.checked {
                    "narration.checkbox.usage.focused.uncheck"
                } else {
                    "narration.checkbox.usage.focused.check"
                }
            } else if self.checked {
                "narration.checkbox.usage.hovered.uncheck"
            } else {
                "narration.checkbox.usage.hovered.check"
            };
            output.add(NarratedElementType::Usage, crate::translate(usage));
        }
    }
}
impl LayoutElement for Checkbox {
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
