use super::sprites::{self, WidgetSprites};
use super::{
    BUTTON_SPRITES, WidgetBase, WidgetClickSound, WidgetTooltipHolder, argb_white, click_sound_sink,
};
use crate::FontWidthMeasurer;
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{
    NarratableEntry, NarratedElementType, NarrationElementOutput, NarrationPriority,
};
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::input::events::{CursorType, KeyEvent, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
use std::sync::Arc;
pub const LOADING_SPRITE_WIDTH: i32 = 5;
pub const LOADING_SPRITE_HEIGHT: i32 = 2;
pub const LOADING_TOOLTIP_KEY: &str = "gui.friends.button.loading";
#[derive(Clone)]
pub struct SpriteIconButton<A: Clone + PartialEq + std::fmt::Debug> {
    pub base: WidgetBase,
    sprite: WidgetSprites,
    sprite_width: i32,
    sprite_height: i32,
    sprite_offset_x: i32,
    sprite_offset_y: i32,
    icon_only: bool,
    loading: bool,
    switch_to_loading_after_press: bool,
    default_tooltip_lines: Option<Vec<String>>,
    narration_key: Option<String>,
    action: Option<A>,
    pressed_this_frame: bool,
    font: Option<Arc<dyn FontWidthMeasurer>>,
    tooltip_holder: WidgetTooltipHolder,
}
impl<A: Clone + PartialEq + std::fmt::Debug> SpriteIconButton<A> {
    pub fn builder(message: Component) -> SpriteIconButtonBuilder<A> {
        SpriteIconButtonBuilder::new(message)
    }
    fn icon_is_active(&self) -> bool {
        self.base.is_active() && !self.loading
    }
    pub fn is_hovered_or_focused(&self) -> bool {
        self.base.hovered() || self.base.focused()
    }
    pub fn sprite(&self) -> &WidgetSprites {
        &self.sprite
    }
    #[doc(alias = "setLoading")]
    pub fn set_loading(&mut self, loading: bool) {
        self.set_loading_with_tooltip(loading, Some(vec![LOADING_TOOLTIP_KEY.to_string()]));
    }
    #[doc(alias = "setLoading")]
    pub fn set_loading_with_tooltip(&mut self, loading: bool, tooltip: Option<Vec<String>>) {
        self.loading = loading;
        if loading {
            self.tooltip_holder.set_lines(tooltip);
        } else {
            self.tooltip_holder
                .set_lines(self.default_tooltip_lines.clone());
        }
    }
    #[doc(alias = "isLoading")]
    pub fn is_loading(&self) -> bool {
        self.loading
    }
    pub fn set_tooltip(&mut self, lines: Option<Vec<String>>) {
        self.default_tooltip_lines = lines.clone();
        self.tooltip_holder.set_lines(lines);
    }
    pub fn set_tooltip_delay(&mut self, delay_millis: u64) {
        self.tooltip_holder.set_delay_millis(delay_millis);
    }
    pub fn action(&self) -> Option<&A> {
        self.action.as_ref()
    }
    #[doc(alias = "onPress")]
    pub fn on_press(&mut self) {
        if self.switch_to_loading_after_press {
            self.set_loading(true);
        }
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
    pub fn set_alpha(&mut self, alpha: f32) {
        self.base.alpha = alpha;
    }
    fn extract_default_sprite(&self, graphics: &mut GuiGraphicsExtractor) {
        graphics.blit_region(
            super::sprites::widget_sprite(
                BUTTON_SPRITES.get(self.base.active, self.is_hovered_or_focused()),
            ),
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
    fn extract_loading_state_if_loading(&mut self, graphics: &mut GuiGraphicsExtractor) -> bool {
        if !self.loading {
            return false;
        }
        graphics.blit_with_alpha(
            ResourceLocation::from_string(sprites::sprites::BUTTON_DISABLED),
            self.base.x(),
            self.base.y(),
            self.base.width(),
            self.base.height(),
            self.base.alpha,
        );
        graphics.blit_with_alpha(
            ResourceLocation::from_string(sprites::sprites::LOADING),
            self.base.x() + (self.base.width() - LOADING_SPRITE_WIDTH) / 2,
            self.base.y() + (self.base.height() - LOADING_SPRITE_HEIGHT) / 2,
            LOADING_SPRITE_WIDTH,
            LOADING_SPRITE_HEIGHT,
            self.base.alpha,
        );
        true
    }
    fn extract_sprite(&self, graphics: &mut GuiGraphicsExtractor) {
        let sprite = self
            .sprite
            .get(self.icon_is_active(), self.is_hovered_or_focused());
        let (x, y) = self.sprite_position();
        graphics.blit_with_alpha(
            sprites::widget_sprite(sprite),
            x,
            y,
            self.sprite_width,
            self.sprite_height,
            self.base.alpha,
        );
    }
    fn sprite_position(&self) -> (i32, i32) {
        let x = self.base.x();
        let y = self.base.y();
        let (width, height) = (self.base.width(), self.base.height());
        let (sprite_width, sprite_height) = (self.sprite_width, self.sprite_height);
        if self.icon_only {
            (
                self.sprite_offset_x + x + width / 2 - sprite_width / 2,
                self.sprite_offset_y + y + height / 2 - sprite_height / 2,
            )
        } else {
            (
                self.sprite_offset_x + x + width - sprite_width - 2,
                self.sprite_offset_y + y + height / 2 - sprite_height / 2,
            )
        }
    }
    fn extract_contents(&mut self, graphics: &mut GuiGraphicsExtractor) {
        if self.extract_loading_state_if_loading(graphics) {
            return;
        }
        self.extract_default_sprite(graphics);
        if !self.icon_only {
            let left = self.base.x() + super::TEXT_MARGIN;
            let right = self.base.x() + self.base.width() - self.sprite_width - 4;
            let top = self.base.y();
            let bottom = self.base.y() + self.base.height();
            let text = self.base.message().get_string();
            let color = if self.base.active {
                super::ACTIVE_TEXT_COLOR
            } else {
                super::INACTIVE_TEXT_COLOR
            };
            let font: Option<Arc<dyn FontWidthMeasurer>> = self.font.as_ref().map(Arc::clone);
            match font {
                Some(font) => {
                    graphics.draw_scrolling_text_with_default_center(
                        font.as_ref(),
                        &text,
                        left,
                        right,
                        top,
                        bottom,
                        color,
                    );
                }
                None => {
                    graphics.draw_scrolling_text_with_default_center_estimated(
                        &text, left, right, top, bottom, color,
                    );
                }
            }
        }
        self.extract_sprite(graphics);
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
}
struct SixPixelFontStandIn;
impl crate::FontWidthMeasurer for SixPixelFontStandIn {
    fn measure_width(&self, text: &str, _scale: f32) -> f32 {
        6.0 * text.chars().count() as f32
    }
}
impl<A: Clone + PartialEq + std::fmt::Debug> super::Renderable for SpriteIconButton<A> {
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
            graphics.request_cursor(if self.icon_is_active() {
                CursorType::PointingHand
            } else {
                CursorType::NotAllowed
            });
        }
        self.extract_tooltip(graphics, mouse_x, mouse_y);
    }
}
impl<A: Clone + PartialEq + std::fmt::Debug> super::TabOrderedElement for SpriteIconButton<A> {
    fn tab_order_group(&self) -> i32 {
        self.base.tab_order_group()
    }
}
impl<A: Clone + PartialEq + std::fmt::Debug> super::GuiEventListener for SpriteIconButton<A> {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        if !self.icon_is_active() || event.button() != 0 {
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
        if !self.icon_is_active() || !event.is_selection() {
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
        self.icon_is_active() && self.base.are_coordinates_in_rectangle(mouse_x, mouse_y)
    }
    fn next_focus_path(&self, _navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        if self.icon_is_active() && !self.base.focused() {
            Some(ComponentPath::leaf(0))
        } else {
            None
        }
    }
    fn rectangle(&self) -> ScreenRectangle {
        self.base.rectangle()
    }
}
impl<A: Clone + PartialEq + std::fmt::Debug> NarratableEntry for SpriteIconButton<A> {
    fn is_active(&self) -> bool {
        self.icon_is_active()
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
        let title = match &self.narration_key {
            Some(key) => key.clone(),
            None => format!("gui.narrate.button {}", self.base.message().get_string()),
        };
        output.add(NarratedElementType::Title, title);
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
impl<A: Clone + PartialEq + std::fmt::Debug> LayoutElement for SpriteIconButton<A> {
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
impl<A: Clone + PartialEq + std::fmt::Debug> std::fmt::Debug for SpriteIconButton<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpriteIconButton")
            .field("action", &self.action)
            .field("pressed_this_frame", &self.pressed_this_frame)
            .field("icon_only", &self.icon_only)
            .field("loading", &self.loading)
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
pub struct SpriteIconButtonBuilder<A: Clone + PartialEq + std::fmt::Debug> {
    message: Component,
    action: Option<A>,
    font: Option<Arc<dyn FontWidthMeasurer>>,
    tooltip_holder: WidgetTooltipHolder,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    sprite: Option<WidgetSprites>,
    sprite_width: i32,
    sprite_height: i32,
    sprite_offset_x: i32,
    sprite_offset_y: i32,
    icon_only: bool,
    narration_key: Option<String>,
    switch_to_loading_after_press: bool,
}
impl<A: Clone + PartialEq + std::fmt::Debug> SpriteIconButtonBuilder<A> {
    fn new(message: Component) -> Self {
        Self {
            message,
            action: None,
            font: None,
            tooltip_holder: WidgetTooltipHolder::default(),
            x: 0,
            y: 0,
            width: super::DEFAULT_WIDTH,
            height: super::DEFAULT_HEIGHT,
            sprite: None,
            sprite_width: 0,
            sprite_height: 0,
            sprite_offset_x: 0,
            sprite_offset_y: 0,
            icon_only: false,
            narration_key: None,
            switch_to_loading_after_press: false,
        }
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
    pub fn bounds(mut self, x: i32, y: i32, width: i32, height: i32) -> Self {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
        self
    }
    pub fn with_action(mut self, action: impl Into<A>) -> Self {
        self.action = Some(action.into());
        self
    }
    pub fn with_sprite(
        mut self,
        config: WidgetSprites,
        sprite_width: i32,
        sprite_height: i32,
    ) -> Self {
        self.sprite = Some(config);
        self.sprite_width = sprite_width;
        self.sprite_height = sprite_height;
        self
    }
    pub fn with_sprite_uniform(
        self,
        sprite: &'static str,
        sprite_width: i32,
        sprite_height: i32,
    ) -> Self {
        self.with_sprite(WidgetSprites::uniform(sprite), sprite_width, sprite_height)
    }
    pub fn sprite_offset(mut self, sprite_offset_x: i32, sprite_offset_y: i32) -> Self {
        self.sprite_offset_x = sprite_offset_x;
        self.sprite_offset_y = sprite_offset_y;
        self
    }
    pub fn tooltip(mut self, lines: Vec<String>) -> Self {
        self.tooltip_holder.set_lines(Some(lines));
        self
    }
    pub fn with_message_tooltip(mut self) -> Self {
        let lines = vec![self.message.get_string()];
        self.tooltip_holder.set_lines(Some(lines));
        self
    }
    pub fn narration(mut self, narration_key: impl Into<String>) -> Self {
        self.narration_key = Some(narration_key.into());
        self
    }
    pub fn switch_to_loading_after_press(mut self) -> Self {
        self.switch_to_loading_after_press = true;
        self
    }
    pub fn with_font(mut self, font: Arc<dyn FontWidthMeasurer>) -> Self {
        self.font = Some(font);
        self
    }
    pub fn icon_only(mut self, icon_only: bool) -> Self {
        self.icon_only = icon_only;
        self
    }
    pub fn build(self) -> SpriteIconButton<A> {
        let sprite = match self.sprite {
            Some(sprite) => sprite,
            None => panic!("Sprite not set"),
        };
        SpriteIconButton {
            base: WidgetBase::new(self.x, self.y, self.width, self.height, self.message),
            sprite,
            sprite_width: self.sprite_width,
            sprite_height: self.sprite_height,
            sprite_offset_x: self.sprite_offset_x,
            sprite_offset_y: self.sprite_offset_y,
            icon_only: self.icon_only,
            loading: false,
            switch_to_loading_after_press: self.switch_to_loading_after_press,
            default_tooltip_lines: self.tooltip_holder.lines.clone(),
            narration_key: self.narration_key,
            action: self.action,
            pressed_this_frame: false,
            font: self.font,
            tooltip_holder: self.tooltip_holder,
        }
    }
}
