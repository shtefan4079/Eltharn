use crate::components::{
    GuiEventListener, NarratableEntry, Renderable, TabOrderedElement, WidgetBase,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::{Layout, LayoutElement};
use crate::framework::narration::NarrationPriority;
use crate::framework::navigation::ScreenRectangle;
use crate::input::events::{CursorType, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
pub const SCROLLBAR_WIDTH: i32 = 6;
pub const SCROLLBAR_MIN_HEIGHT: i32 = 32;
pub const SCROLLER_SPRITE: &str = "minecraft:widget/scroller";
pub const SCROLLER_BACKGROUND_SPRITE: &str = "minecraft:widget/scroller_background";
pub const DEFAULT_SCROLLBAR_SPACING: i32 = 4;
pub const SCROLLABLE_LAYOUT_SCROLL_RATE: i32 = 10;
pub const MENU_LIST_BACKGROUND: &str = "minecraft:gui/menu_list_background";
pub const INWORLD_MENU_LIST_BACKGROUND: &str = "minecraft:gui/inworld_menu_list_background";
fn mth_clamp(value: i32, min: i32, max: i32) -> i32 {
    value.max(min).min(max)
}
fn mx_my_over_scrollbar(area: &AbstractScrollArea, mx: i32, my: i32, viewport: i32) -> bool {
    let x = area.scrollbar_x();
    mx >= x
        && mx <= x + area.scrollbar_width()
        && my >= area.base.y()
        && my < area.base.y() + viewport
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScrollbarSettings {
    pub scroller_sprite: String,
    pub disabled_scroller_sprite: String,
    pub background_sprite: String,
    pub scrollbar_width: i32,
    pub scrollbar_min_height: i32,
    pub scroll_rate: i32,
    pub resizing_scrollbar: bool,
}
impl ScrollbarSettings {
    pub fn default_settings(scroll_rate: i32) -> Self {
        Self {
            scroller_sprite: SCROLLER_SPRITE.to_string(),
            disabled_scroller_sprite: String::new(),
            background_sprite: SCROLLER_BACKGROUND_SPRITE.to_string(),
            scrollbar_width: SCROLLBAR_WIDTH,
            scrollbar_min_height: SCROLLBAR_MIN_HEIGHT,
            scroll_rate,
            resizing_scrollbar: true,
        }
    }
    pub fn no_scroll() -> Self {
        Self::default_settings(0)
    }
    pub fn has_disabled_scroller(&self) -> bool {
        !self.disabled_scroller_sprite.is_empty()
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollMargins {
    pub top: i32,
    pub bottom: i32,
}
impl ScrollMargins {
    pub fn new(top: i32, bottom: i32) -> Self {
        Self { top, bottom }
    }
    pub fn uniform(margin: i32) -> Self {
        Self::new(margin, margin)
    }
    pub fn vertical(self) -> i32 {
        self.top + self.bottom
    }
}
#[derive(Clone, Debug)]
pub struct AbstractScrollArea {
    pub base: WidgetBase,
    pub content_height: i32,
    pub scroll_amount: f32,
    pub scrollbar_settings: ScrollbarSettings,
    pub scrolling: bool,
    pub margins: ScrollMargins,
    pub visible: bool,
    pub draw_list_background: bool,
}
impl AbstractScrollArea {
    pub fn new(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        message: Component,
        scrollbar_settings: ScrollbarSettings,
    ) -> Self {
        Self {
            base: WidgetBase::new(x, y, width, height, message),
            content_height: height,
            scroll_amount: 0.0,
            scrollbar_settings,
            scrolling: false,
            margins: ScrollMargins::default(),
            visible: true,
            draw_list_background: true,
        }
    }
    pub fn scroll_rate(&self) -> i32 {
        self.scrollbar_settings.scroll_rate
    }
    pub fn scrollbar_width(&self) -> i32 {
        self.scrollbar_settings.scrollbar_width
    }
    pub fn viewport_height(&self, height: i32) -> i32 {
        (height - self.margins.vertical()).max(0)
    }
    pub fn viewport(&self) -> i32 {
        self.base.height()
    }
    pub fn height_for_viewport(&self) -> i32 {
        self.base.height() + self.margins.vertical()
    }
    pub fn viewport_top(&self, height: i32) -> i32 {
        let _ = height;
        self.base.y()
    }
    pub fn viewport_bottom(&self, height: i32) -> i32 {
        self.viewport_top(height) + self.viewport_height(height)
    }
    fn rect_bottom(&self) -> i32 {
        self.base.y() + self.viewport()
    }
    pub fn max_scroll_amount(&self, height: i32) -> f32 {
        self.max_scroll_for_viewport(self.viewport_height(height)) as f32
    }
    fn max_scroll_for_viewport(&self, viewport: i32) -> i32 {
        if viewport <= 0 {
            return 0;
        }
        (self.content_height - viewport).max(0)
    }
    pub fn scrollable(&self, height: i32) -> bool {
        self.max_scroll_amount(height) > 0.0
    }
    pub fn scrollbar_height(&self, height: i32) -> i32 {
        self.scrollbar_height_for_viewport(self.viewport_height(height))
    }
    fn scrollbar_height_for_viewport(&self, viewport: i32) -> i32 {
        if viewport <= 0 {
            return 0;
        }
        let content = self.content_height.max(1);
        let ratio = viewport as f32 * viewport as f32 / content as f32;
        mth_clamp(ratio as i32, SCROLLBAR_MIN_HEIGHT, viewport - 8).max(0)
    }
    pub fn scrollbar_x(&self) -> i32 {
        self.base.x() + self.base.width() - self.scrollbar_width()
    }
    pub fn scrollbar_y(&self, height: i32) -> i32 {
        self.scrollbar_y_for_viewport(self.viewport_height(height))
    }
    fn scrollbar_y_for_viewport(&self, viewport: i32) -> i32 {
        let top = self.base.y();
        let max = self.max_scroll_for_viewport(viewport);
        if max == 0 {
            return top;
        }
        let travel = viewport - self.scrollbar_height_for_viewport(viewport);
        let raw = self.scroll_amount as i32 * travel / max + top;
        raw.max(top)
    }
    pub fn set_scroll_amount(&mut self, scroll_amount: f32, height: i32) {
        let max = self.max_scroll_amount(height);
        self.scroll_amount = scroll_amount.clamp(0.0, max);
    }
    pub fn clamp_scroll(&mut self, height: i32) {
        self.set_scroll_amount(self.scroll_amount, height);
    }
    pub fn scroll(&mut self, delta: f32, height: i32) -> bool {
        if !self.visible {
            return false;
        }
        let target = self.scroll_amount - delta * self.scroll_rate() as f32;
        self.set_scroll_amount(target, height);
        true
    }
    pub fn scroll_by(&mut self, pixels: f32, height: i32) -> bool {
        if !self.visible {
            return false;
        }
        let target = self.scroll_amount + pixels;
        self.set_scroll_amount(target, height);
        true
    }
    pub fn is_over_scrollbar(&self, mx: f64, my: f64, height: i32) -> bool {
        let x = self.scrollbar_x() as f64;
        mx >= x
            && mx <= x + self.scrollbar_width() as f64
            && my >= self.base.y() as f64
            && my < self.viewport_bottom(height) as f64
    }
    pub fn scrollbar_clicked(&mut self, mx: f64, my: f64, height: i32) -> bool {
        self.scrolling = self.scrollable(height) && self.is_over_scrollbar(mx, my, height);
        self.scrolling
    }
    pub fn scrollbar_clicked_with_button(
        &mut self,
        mx: f64,
        my: f64,
        button: i32,
        height: i32,
    ) -> bool {
        if button != 0 {
            self.scrolling = false;
            return false;
        }
        self.scrollbar_clicked(mx, my, height)
    }
    pub fn scrollbar_dragged(&mut self, event_y: f64, dy: f64, height: i32) -> bool {
        if !self.scrolling {
            return false;
        }
        let top = self.base.y() as f64;
        let bottom = self.rect_bottom() as f64;
        if event_y < top {
            self.set_scroll_amount(0.0, height);
        } else if event_y > bottom {
            self.set_scroll_amount(self.max_scroll_amount(height), height);
        } else {
            let travel = self.viewport_height(height) - self.scrollbar_height(height);
            let max = (self
                .max_scroll_for_viewport(self.viewport_height(height))
                .max(1)) as f32;
            let y_drag_scale = (max / travel as f32).max(1.0);
            let target = self.scroll_amount + (dy as f32) * y_drag_scale;
            self.set_scroll_amount(target, height);
        }
        true
    }
    pub fn scrollbar_released(&mut self) {
        self.scrolling = false;
    }
    pub fn is_scrolling(&self) -> bool {
        self.scrolling
    }
    pub fn scrollbar_rect(&self, height: i32) -> ScreenRectangle {
        ScreenRectangle::new(
            self.scrollbar_x(),
            self.base.y(),
            self.scrollbar_width(),
            self.viewport_height(height),
        )
    }
    pub fn scroller_rect(&self, height: i32) -> ScreenRectangle {
        ScreenRectangle::new(
            self.scrollbar_x(),
            self.scrollbar_y(height),
            self.scrollbar_width(),
            self.scrollbar_height(height),
        )
    }
    pub fn scissor_rect(&self) -> ScreenRectangle {
        self.base.rectangle()
    }
    pub fn content_scissor(&self) -> Option<ScreenRectangle> {
        self.visible.then(|| self.scissor_rect())
    }
    pub fn content_y_offset(&self) -> i32 {
        -(self.scroll_amount as i32)
    }
    pub fn content_offset_y(&self) -> i32 {
        self.content_y_offset()
    }
    pub fn scrollbar_visible(&self, height: i32) -> bool {
        self.base.visible
            && self.visible
            && self.max_scroll_for_viewport(self.viewport_height(height)) > 0
    }
    fn scrollbar_shown(&self, viewport: i32) -> bool {
        self.max_scroll_for_viewport(viewport) > 0
    }
    pub fn extract_scrollbar(&mut self, graphics: &mut GuiGraphicsExtractor) {
        let viewport = self.viewport();
        if !self.base.visible || !self.visible {
            return;
        }
        let mouse_x = graphics.mouse_x();
        let mouse_y = graphics.mouse_y();
        let bar_x = self.scrollbar_x();
        let top = self.base.y();
        let track_height = viewport;
        let bar_height = self.scrollbar_height_for_viewport(viewport);
        let bar_y = self.scrollbar_y_for_viewport(viewport);
        let width = self.scrollbar_width();
        let over = mx_my_over_scrollbar(self, mouse_x, mouse_y, viewport);
        let scrolling = self.scrolling;
        let background = self.scrollbar_settings.background_sprite.clone();
        if !self.scrollbar_shown(viewport) {
            if self.scrollbar_settings.has_disabled_scroller() {
                let disabled = self.scrollbar_settings.disabled_scroller_sprite.clone();
                graphics.blit(
                    ResourceLocation::from_string(&background),
                    bar_x,
                    top,
                    width,
                    track_height,
                );
                graphics.blit(
                    ResourceLocation::from_string(&disabled),
                    bar_x,
                    top,
                    width,
                    bar_height,
                );
                if over {
                    graphics.request_cursor(CursorType::NotAllowed);
                }
            }
            return;
        }
        graphics.blit(
            ResourceLocation::from_string(&background),
            bar_x,
            top,
            width,
            track_height,
        );
        let scroller = self.scrollbar_settings.scroller_sprite.clone();
        graphics.blit(
            ResourceLocation::from_string(&scroller),
            bar_x,
            bar_y,
            width,
            bar_height,
        );
        if over {
            graphics.request_cursor(if scrolling {
                CursorType::ResizeNs
            } else {
                CursorType::PointingHand
            });
        }
    }
    pub fn arrange(&mut self, x: i32, y: i32, height: i32) -> bool {
        self.base.set_x(x);
        self.base.set_y(y);
        let viewport = self.viewport_height(height);
        let changed = self.base.height() != viewport;
        self.base.set_height(viewport);
        self.clamp_scroll(height);
        changed
    }
    pub fn content_y(&self, height: i32) -> i32 {
        self.viewport_top(height) + self.content_y_offset()
    }
    pub fn extract_list_background(&self, graphics: &mut GuiGraphicsExtractor, in_world: bool) {
        self.extract_list_background_at(
            graphics,
            in_world,
            self.base.x(),
            self.base.y(),
            self.base.width(),
            self.base.height(),
            self.scroll_amount,
        );
    }
    pub fn extract_list_background_at(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        in_world: bool,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        scroll_amount: f32,
    ) {
        if width <= 0 || height <= 0 {
            return;
        }
        let texture = if in_world {
            INWORLD_MENU_LIST_BACKGROUND
        } else {
            MENU_LIST_BACKGROUND
        };
        let bottom = y + height + scroll_amount as i32;
        let right = x + width;
        let texture = ResourceLocation::from_string(texture);
        let size = 32.0f32;
        graphics.blit_region(
            texture,
            x,
            y,
            width,
            height,
            right as f32 / size,
            bottom as f32 / size,
            (right + width) as f32 / size,
            (bottom + height) as f32 / size,
            -1,
        );
    }
    pub fn set_draw_list_background(&mut self, draw_list_background: bool) {
        self.draw_list_background = draw_list_background;
    }
    pub fn draws_list_background(&self) -> bool {
        self.draw_list_background
    }
}
impl TabOrderedElement for AbstractScrollArea {
    fn tab_order_group(&self) -> i32 {
        self.base.tab_order_group()
    }
}
impl LayoutElement for AbstractScrollArea {
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
impl GuiEventListener for AbstractScrollArea {
    fn mouse_scrolled(&mut self, _x: f64, _y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        let _ = scroll_x;
        self.scroll(scroll_y as f32, self.height_for_viewport())
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        self.scrollbar_clicked_with_button(
            event.x,
            event.y,
            event.button(),
            self.height_for_viewport(),
        )
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, _dx: f64, dy: f64) -> bool {
        if event.button() != 0 {
            return false;
        }
        self.scrollbar_dragged(event.y, dy, self.height_for_viewport());
        true
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        if event.button() != 0 {
            return false;
        }
        self.scrollbar_released();
        true
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.base.active && self.base.are_coordinates_in_rectangle(mouse_x, mouse_y)
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
    fn border_for_arrow_navigation(
        &self,
        opposite: crate::framework::navigation::ScreenDirection,
    ) -> ScreenRectangle {
        let mut rect = self.base.rectangle();
        rect.height = self.content_height;
        rect.get_border(opposite)
    }
    fn play_down_sound(&self) -> Option<crate::components::WidgetClickSound> {
        None
    }
}
impl NarratableEntry for AbstractScrollArea {
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
    fn update_narration(&self, _output: &mut crate::framework::narration::NarrationElementOutput) {}
}
impl Renderable for AbstractScrollArea {
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
        if self.draw_list_background {
            self.extract_list_background(graphics, false);
        }
        self.extract_scrollbar(graphics);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReserveStrategy {
    Right,
    Both,
}
impl Default for ReserveStrategy {
    fn default() -> Self {
        Self::Both
    }
}
pub struct ScrollableLayout {
    content: Box<dyn Layout>,
    container: AbstractScrollArea,
    reserve_strategy: ReserveStrategy,
    scrollbar_spacing: i32,
    min_width: i32,
    min_height: i32,
    max_height: i32,
}
impl ScrollableLayout {
    pub fn new(content: impl Layout + 'static, max_height: i32) -> Self {
        Self::with_reserve_strategy(content, max_height, ReserveStrategy::Both)
    }
    pub fn with_reserve_strategy(
        content: impl Layout + 'static,
        max_height: i32,
        reserve_strategy: ReserveStrategy,
    ) -> Self {
        let mut container = AbstractScrollArea::new(
            0,
            0,
            0,
            max_height,
            Component::literal(""),
            ScrollbarSettings::default_settings(SCROLLABLE_LAYOUT_SCROLL_RATE),
        );
        container.content_height = 0;
        Self {
            content: Box::new(content),
            container,
            reserve_strategy,
            scrollbar_spacing: DEFAULT_SCROLLBAR_SPACING,
            min_width: 0,
            min_height: 0,
            max_height,
        }
    }
    pub fn set_scrollbar_spacing(&mut self, scrollbar_spacing: i32) {
        self.scrollbar_spacing = scrollbar_spacing;
    }
    pub fn set_min_width(&mut self, min_width: i32) {
        self.min_width = min_width;
        let content_width = self.content.width();
        self.container.base.set_width(content_width.max(min_width));
    }
    pub fn set_min_height(&mut self, min_height: i32) {
        self.min_height = min_height;
        let content_height = self.content.height();
        self.container
            .base
            .set_height(content_height.max(min_height));
    }
    pub fn set_max_height(&mut self, max_height: i32) {
        self.max_height = max_height;
        let content_height = self.content.height();
        self.container
            .base
            .set_height(content_height.min(max_height));
        let viewport = self.container.base.height();
        self.container.clamp_scroll(viewport);
    }
    pub fn arrange_elements(&mut self) {
        self.content.arrange_elements();
        let content_width = self.content.width();
        let scrollbar_reserve = match self.reserve_strategy {
            ReserveStrategy::Right => self.scrollbar_reserve(),
            ReserveStrategy::Both => 2 * self.scrollbar_reserve(),
        };
        self.container
            .base
            .set_width(content_width.max(self.min_width) + scrollbar_reserve);
        let height = mth_clamp(
            self.container.base.height(),
            self.min_height,
            self.max_height,
        );
        self.container.base.set_height(height);
        self.container.content_height = self.content.height();
        self.sync_content_position();
        self.container.clamp_scroll(height);
    }
    pub fn scrollbar_reserve(&self) -> i32 {
        self.scrollbar_spacing + self.container.scrollbar_width()
    }
    pub fn content_x(&self) -> i32 {
        let extra = if self.reserve_strategy == ReserveStrategy::Both {
            self.scrollbar_reserve()
        } else {
            0
        };
        self.container.base.x() + extra
    }
    pub fn content_y(&self) -> i32 {
        self.container.base.y() + self.container.content_y_offset()
    }
    fn sync_content_position(&mut self) {
        let x = self.content_x();
        let y = self.content_y();
        self.content.set_x(x);
        self.content.set_y(y);
    }
    pub fn scroll_focused_into_view(&mut self, focused: ScreenRectangle, keyboard_driven: bool) {
        if !keyboard_driven {
            return;
        }
        let area = self.container.base.rectangle();
        let top_delta = focused.top() - area.top();
        let bottom_delta = focused.bottom() - area.bottom();
        if top_delta < 0 {
            let target =
                self.container.scroll_amount + (top_delta - self.container.scroll_rate()) as f32;
            let viewport = self.container.base.height();
            self.container.set_scroll_amount(target, viewport);
        } else if bottom_delta > 0 {
            let target =
                self.container.scroll_amount + (bottom_delta + self.container.scroll_rate()) as f32;
            let viewport = self.container.base.height();
            self.container.set_scroll_amount(target, viewport);
        }
    }
    pub fn container(&self) -> &AbstractScrollArea {
        &self.container
    }
    pub fn container_mut(&mut self) -> &mut AbstractScrollArea {
        &mut self.container
    }
    pub fn content(&self) -> &dyn Layout {
        self.content.as_ref()
    }
    pub fn content_mut(&mut self) -> &mut dyn Layout {
        self.content.as_mut()
    }
    pub fn content_height(&self) -> i32 {
        self.content.height()
    }
    pub fn scroll_amount(&self) -> f32 {
        self.container.scroll_amount
    }
    pub fn max_scroll_amount(&self) -> f32 {
        let viewport = self.container.base.height();
        self.container.max_scroll_amount(viewport)
    }
    pub fn scrollbar_height(&self) -> i32 {
        let viewport = self.container.base.height();
        self.container.scrollbar_height(viewport)
    }
    pub fn content_y_offset(&self) -> i32 {
        self.container.content_y_offset()
    }
    pub fn set_scroll_amount(&mut self, scroll_amount: f32) {
        let viewport = self.container.base.height();
        self.container.set_scroll_amount(scroll_amount, viewport);
        self.sync_content_position();
    }
    pub fn scrollbar_settings(&self) -> &ScrollbarSettings {
        &self.container.scrollbar_settings
    }
}
impl LayoutElement for ScrollableLayout {
    fn set_x(&mut self, x: i32) {
        self.container.base.set_x(x);
        let content_x = self.content_x();
        self.content.set_x(content_x);
    }
    fn set_y(&mut self, y: i32) {
        self.container.base.set_y(y);
        let content_y = self.content_y();
        self.content.set_y(content_y);
    }
    fn x(&self) -> i32 {
        self.container.base.x()
    }
    fn y(&self) -> i32 {
        self.container.base.y()
    }
    fn width(&self) -> i32 {
        self.container.base.width()
    }
    fn height(&self) -> i32 {
        self.container.base.height()
    }
    fn arrange(&mut self) {
        self.arrange_elements();
    }
}
impl Layout for ScrollableLayout {
    fn arrange_elements(&mut self) {
        ScrollableLayout::arrange_elements(self);
    }
    fn remove_children(&mut self) {
        self.content.remove_children();
    }
}
