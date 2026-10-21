use crate::framework::navigation::ScreenRectangle;
use crate::framework::pose::PoseStack;
use crate::framework::render_state::{
    GuiElementRenderState, GuiItemRenderState, GuiRenderState, GuiTextRenderState,
    PanoramaRenderState, RenderPipeline, TextureSetup, rect_bounds,
};
use crate::input::events::CursorType;
use el_core::{ItemStack, ResourceLocation};
const SLIDER_SPRITE: &str = "minecraft:widget/slider";
const SLIDER_HIGHLIGHTED_SPRITE: &str = "minecraft:widget/slider_highlighted";
const SLIDER_HANDLE_SPRITE: &str = "minecraft:widget/slider_handle";
const SLIDER_HANDLE_HIGHLIGHTED_SPRITE: &str = "minecraft:widget/slider_handle_highlighted";
fn slider_sprites(
    active: bool,
    hovered: bool,
    focused: bool,
    can_change_value: bool,
) -> (&'static str, &'static str) {
    let track = if active && focused && !can_change_value {
        SLIDER_HIGHLIGHTED_SPRITE
    } else {
        SLIDER_SPRITE
    };
    let handle = if !active || (!hovered && !can_change_value) {
        SLIDER_HANDLE_SPRITE
    } else {
        SLIDER_HANDLE_HIGHLIGHTED_SPRITE
    };
    (track, handle)
}
pub fn argb_white(alpha: f32) -> i32 {
    let channel = (alpha.clamp(0.0, 1.0) * 255.0) as i32;
    channel << 24 | 0x00FF_FFFF
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetTooltipPosition {
    Menu,
    BelowOrAbove,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ScrollingTextLayout {
    x: i32,
    y: i32,
    scrolls: bool,
}
fn java_scrolling_text_layout(
    line_width: i32,
    center_x: i32,
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
    now_millis: u128,
) -> ScrollingTextLayout {
    const LINE_HEIGHT: i32 = 9;
    let y = (top + bottom - LINE_HEIGHT) / 2 + 1;
    let available_width = right - left;
    if line_width > available_width {
        let max_position = line_width - available_width;
        let time = now_millis as f64 / 1000.0;
        let period = (max_position as f64 * 0.5).max(3.0);
        let alpha = ((std::f64::consts::FRAC_PI_2 * (std::f64::consts::TAU * time / period).cos())
            .sin()
            / 2.0)
            + 0.5;
        let position = (alpha * max_position as f64) as i32;
        ScrollingTextLayout {
            x: left - position,
            y,
            scrolls: true,
        }
    } else {
        let anchor_x = center_x.clamp(left + line_width / 2, right - line_width / 2);
        ScrollingTextLayout {
            x: anchor_x - line_width / 2,
            y,
            scrolls: false,
        }
    }
}
#[derive(Clone, Debug)]
pub struct GuiGraphicsExtractor {
    width: i32,
    height: i32,
    mouse_x: i32,
    mouse_y: i32,
    pose: PoseStack,
    scissor_stack: Vec<ScreenRectangle>,
    render_state: GuiRenderState,
    pending_cursor: CursorType,
}
impl GuiGraphicsExtractor {
    pub fn new(width: i32, height: i32, mouse_x: i32, mouse_y: i32) -> Self {
        let root = ScreenRectangle::new(0, 0, width, height);
        Self {
            width,
            height,
            mouse_x,
            mouse_y,
            pose: PoseStack::new(),
            scissor_stack: vec![root],
            render_state: GuiRenderState::new(),
            pending_cursor: CursorType::Default,
        }
    }
    pub fn with_state(
        width: i32,
        height: i32,
        mouse_x: i32,
        mouse_y: i32,
        render_state: GuiRenderState,
    ) -> Self {
        let mut graphics = Self::new(width, height, mouse_x, mouse_y);
        graphics.render_state = render_state;
        graphics
    }
    pub fn gui_width(&self) -> i32 {
        self.width
    }
    pub fn gui_height(&self) -> i32 {
        self.height
    }
    pub fn mouse_x(&self) -> i32 {
        self.mouse_x
    }
    pub fn mouse_y(&self) -> i32 {
        self.mouse_y
    }
    pub fn pose(&self) -> &PoseStack {
        &self.pose
    }
    pub fn pose_mut(&mut self) -> &mut PoseStack {
        &mut self.pose
    }
    pub fn render_state(&self) -> &GuiRenderState {
        &self.render_state
    }
    pub fn render_state_mut(&mut self) -> &mut GuiRenderState {
        &mut self.render_state
    }
    pub fn finish(self) -> GuiRenderState {
        self.render_state
    }
    pub fn request_cursor(&mut self, cursor: CursorType) {
        self.pending_cursor = cursor;
    }
    pub fn pending_cursor(&self) -> CursorType {
        self.pending_cursor
    }
    pub fn next_stratum(&mut self) {
        self.render_state.next_stratum();
    }
    pub fn begin_tooltip_overlay(&mut self) {
        self.render_state.begin_tooltip_overlay();
    }
    pub fn end_tooltip_overlay(&mut self) {
        self.render_state.end_tooltip_overlay();
    }
    pub fn is_in_tooltip_overlay(&self) -> bool {
        self.render_state.is_in_tooltip_overlay()
    }
    pub fn blur_before_this_stratum(&mut self) {
        self.render_state.blur_before_this_stratum();
    }
    pub fn enable_scissor(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let rect =
            ScreenRectangle::new(x0, y0, x1 - x0, y1 - y0).transform_axis_aligned(self.pose.pose());
        let clipped = self
            .scissor_stack
            .last()
            .and_then(|top| top.intersection(rect));
        self.scissor_stack
            .push(clipped.unwrap_or_else(ScreenRectangle::empty));
    }
    pub fn disable_scissor(&mut self) {
        if self.scissor_stack.len() > 1 {
            self.scissor_stack.pop();
        }
    }
    pub fn contains_point_in_scissor(&self, x: i32, y: i32) -> bool {
        self.scissor_stack
            .last()
            .is_some_and(|rect| rect.contains_point(x, y))
    }
    pub fn current_scissor(&self) -> Option<ScreenRectangle> {
        self.scissor_stack.last().copied()
    }
    pub fn horizontal_line(&mut self, mut x0: i32, mut x1: i32, y: i32, color: i32) {
        if x1 < x0 {
            std::mem::swap(&mut x0, &mut x1);
        }
        self.fill(x0, y, x1 + 1, y + 1, color);
    }
    pub fn vertical_line(&mut self, x: i32, mut y0: i32, mut y1: i32, color: i32) {
        if y1 < y0 {
            std::mem::swap(&mut y0, &mut y1);
        }
        self.fill(x, y0 + 1, x + 1, y1, color);
    }
    pub fn fill(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: i32) {
        self.fill_with_pipeline(
            RenderPipeline::Gui,
            TextureSetup::no_texture(),
            x0,
            y0,
            x1,
            y1,
            color,
        );
    }
    pub fn fill_with_pipeline(
        &mut self,
        pipeline: RenderPipeline,
        texture_setup: TextureSetup,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        color: i32,
    ) {
        self.inner_fill(pipeline, texture_setup, x0, y0, x1, y1, color, color);
    }
    pub fn fill_gradient(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color1: i32, color2: i32) {
        self.inner_fill(
            RenderPipeline::Gui,
            TextureSetup::no_texture(),
            x0,
            y0,
            x1,
            y1,
            color1,
            color2,
        );
    }
    pub fn text_highlight(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, invert_text: bool) {
        if invert_text {
            self.fill_with_pipeline(
                RenderPipeline::GuiInvert,
                TextureSetup::no_texture(),
                x0,
                y0,
                x1,
                y1,
                -1,
            );
        }
        self.fill_with_pipeline(
            RenderPipeline::GuiTextHighlight,
            TextureSetup::no_texture(),
            x0,
            y0,
            x1,
            y1,
            -16776961,
        );
    }
    pub fn outline(&mut self, x: i32, y: i32, width: i32, height: i32, color: i32) {
        self.fill(x, y, x + width, y + 1, color);
        self.fill(x, y + height - 1, x + width, y + height, color);
        self.fill(x, y + 1, x + 1, y + height - 1, color);
        self.fill(x + width - 1, y + 1, x + width, y + height - 1, color);
    }
    fn inner_fill(
        &mut self,
        pipeline: RenderPipeline,
        texture_setup: TextureSetup,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        color1: i32,
        color2: i32,
    ) {
        let pose = self.pose.pose();
        let scissor = self.current_scissor();
        self.render_state
            .add_gui_element(GuiElementRenderState::ColoredRectangle {
                pipeline,
                texture_setup,
                pose,
                x0,
                y0,
                x1,
                y1,
                color1,
                color2,
                scissor,
                bounds: rect_bounds(x0, y0, x1, y1, pose, scissor),
            });
    }
    pub fn text(
        &mut self,
        font: ResourceLocation,
        text: impl Into<String>,
        x: i32,
        y: i32,
        color: i32,
    ) {
        self.text_with_shadow(font, text, x, y, color, true);
    }
    pub fn text_with_shadow(
        &mut self,
        font: ResourceLocation,
        text: impl Into<String>,
        x: i32,
        y: i32,
        color: i32,
        shadow: bool,
    ) {
        self.text_with_style(font, text, x, y, color, shadow, false);
    }
    pub fn text_obfuscated(
        &mut self,
        font: ResourceLocation,
        text: impl Into<String>,
        x: i32,
        y: i32,
        color: i32,
    ) {
        self.text_with_style(font, text, x, y, color, true, true);
    }
    fn text_with_style(
        &mut self,
        font: ResourceLocation,
        text: impl Into<String>,
        x: i32,
        y: i32,
        color: i32,
        shadow: bool,
        obfuscated: bool,
    ) {
        self.text_with_bounds(font, text, x, y, color, shadow, obfuscated, None);
    }
    fn text_with_bounds(
        &mut self,
        font: ResourceLocation,
        text: impl Into<String>,
        x: i32,
        y: i32,
        color: i32,
        shadow: bool,
        obfuscated: bool,
        bounds_width: Option<i32>,
    ) {
        let text = text.into();
        if ((color >> 24) & 0xFF) == 0 {
            return;
        }
        let pose = self.pose.pose();
        let scissor = self.current_scissor();
        let width = bounds_width
            .unwrap_or_else(|| (text.chars().count() as i32).saturating_mul(6))
            .max(0);
        let bounds = rect_bounds(x, y, x + width, y + 9, pose, scissor);
        self.render_state.add_text(GuiTextRenderState {
            font,
            text,
            pose,
            x,
            y,
            color,
            background_color: 0,
            shadow,
            obfuscated,
            see_through: false,
            scissor,
            bounds,
        });
    }
    pub fn blit(&mut self, texture: ResourceLocation, x: i32, y: i32, width: i32, height: i32) {
        self.blit_region(texture, x, y, width, height, 0.0, 0.0, 1.0, 1.0, -1);
    }
    pub fn blit_region(
        &mut self,
        texture: ResourceLocation,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
        color: i32,
    ) {
        let pose = self.pose.pose();
        let scissor = self.current_scissor();
        self.render_state
            .add_gui_element(GuiElementRenderState::Blit {
                pipeline: RenderPipeline::GuiTextured,
                texture_setup: TextureSetup::single_texture(texture),
                pose,
                x0: x,
                y0: y,
                x1: x + width,
                y1: y + height,
                u0,
                u1,
                v0,
                v1,
                color,
                scissor,
                bounds: rect_bounds(x, y, x + width, y + height, pose, scissor),
            });
    }
    pub fn blit_with_alpha(
        &mut self,
        texture: ResourceLocation,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        alpha: f32,
    ) {
        self.blit_region(
            texture,
            x,
            y,
            width,
            height,
            0.0,
            0.0,
            1.0,
            1.0,
            argb_white(alpha),
        );
    }
    pub fn blit_resource_region(
        &mut self,
        texture: ResourceLocation,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
        color: i32,
    ) {
        let pose = self.pose.pose();
        let scissor = self.current_scissor();
        self.render_state
            .add_gui_element(GuiElementRenderState::ResourceTextureBlit {
                pipeline: RenderPipeline::GuiTextured,
                texture_setup: TextureSetup::single_texture(texture),
                pose,
                x0: x,
                y0: y,
                x1: x + width,
                y1: y + height,
                u0,
                u1,
                v0,
                v1,
                color,
                scissor,
                bounds: rect_bounds(x, y, x + width, y + height, pose, scissor),
            });
    }
    pub fn render_item(&mut self, item_stack: ItemStack, x: i32, y: i32) {
        self.render_item_with_decorations(item_stack, x, y, false);
    }
    pub fn render_item_with_decorations(
        &mut self,
        item_stack: ItemStack,
        x: i32,
        y: i32,
        decorations: bool,
    ) {
        let pose = self.pose.pose();
        let scissor = self.current_scissor();
        self.render_state.add_item(GuiItemRenderState {
            item_stack,
            decorations,
            pose,
            x,
            y,
            scissor,
            bounds: rect_bounds(x, y, x + 16, y + 16, pose, scissor),
        });
    }
    pub fn resource_pipeline_blit(
        &mut self,
        pipeline: RenderPipeline,
        texture_setup: TextureSetup,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        let pose = self.pose.pose();
        let scissor = self.current_scissor();
        self.render_state
            .add_gui_element(GuiElementRenderState::ResourceTextureBlit {
                pipeline,
                texture_setup,
                pose,
                x0: x,
                y0: y,
                x1: x + width,
                y1: y + height,
                u0: 0.0,
                u1: 1.0,
                v0: 0.0,
                v1: 1.0,
                color: -1,
                scissor,
                bounds: rect_bounds(x, y, x + width, y + height, pose, scissor),
            });
    }
    pub fn panorama(&mut self) {
        self.render_state.panorama_render_state = Some(PanoramaRenderState {});
    }
    pub fn draw_menu_background(&mut self, in_world: bool, width: i32, height: i32) {
        let bg_tex = if in_world {
            "minecraft:gui/inworld_menu_background"
        } else {
            "minecraft:gui/menu_background"
        };
        let background = ResourceLocation::from_string(bg_tex);
        let mut y = 0;
        while y < height {
            let tile_h = 32.min(height - y);
            let mut x = 0;
            while x < width {
                let tile_w = 32.min(width - x);
                self.blit_region(
                    background.clone(),
                    x,
                    y,
                    tile_w,
                    tile_h,
                    0.0,
                    0.0,
                    tile_w as f32 / 32.0,
                    tile_h as f32 / 32.0,
                    -1,
                );
                x += 32;
            }
            y += tile_h;
        }
    }
    pub fn draw_button(
        &mut self,
        font: &dyn crate::FontWidthMeasurer,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        text: &str,
        active: bool,
    ) {
        self.draw_button_with_focus(font, x, y, w, h, text, active, false);
    }
    pub fn draw_button_with_focus(
        &mut self,
        font: &dyn crate::FontWidthMeasurer,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        text: &str,
        active: bool,
        focused: bool,
    ) {
        let mouse_x = self.mouse_x();
        let mouse_y = self.mouse_y();
        let mouse_over = mouse_x >= x && mouse_x < x + w && mouse_y >= y && mouse_y < y + h;
        let highlighted = active && (mouse_over || focused);
        let sprite = if !active {
            "minecraft:widget/button_disabled"
        } else if highlighted {
            "minecraft:widget/button_highlighted"
        } else {
            "minecraft:widget/button"
        };
        if mouse_over {
            self.request_cursor(if active {
                CursorType::PointingHand
            } else {
                CursorType::NotAllowed
            });
        }
        self.blit(ResourceLocation::from_string(sprite), x, y, w, h);
        let text_color = if active {
            0xFFFF_FFFFu32 as i32
        } else {
            0xFFA0_A0A0u32 as i32
        };
        self.draw_scrolling_text_with_default_center(
            font,
            text,
            x + 2,
            x + w - 2,
            y,
            y + h,
            text_color,
        );
    }
    pub fn draw_slider(
        &mut self,
        font: &dyn crate::FontWidthMeasurer,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        text: &str,
        pct: f32,
        dragging: bool,
    ) {
        self.draw_slider_with_interaction_state(
            font, x, y, w, h, text, pct, true, false, dragging, dragging,
        );
    }
    pub fn draw_slider_with_interaction_state(
        &mut self,
        font: &dyn crate::FontWidthMeasurer,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        text: &str,
        pct: f32,
        active: bool,
        focused: bool,
        can_change_value: bool,
        dragging: bool,
    ) {
        self.draw_slider_with_interaction_state_and_alpha(
            font,
            x,
            y,
            w,
            h,
            text,
            pct,
            active,
            focused,
            can_change_value,
            dragging,
            1.0,
        );
    }
    #[allow(clippy::too_many_arguments)]
    pub fn draw_slider_with_interaction_state_and_alpha(
        &mut self,
        font: &dyn crate::FontWidthMeasurer,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        text: &str,
        pct: f32,
        active: bool,
        focused: bool,
        can_change_value: bool,
        _dragging: bool,
        alpha: f32,
    ) {
        let mouse_x = self.mouse_x();
        let mouse_y = self.mouse_y();
        let mouse_over = mouse_x >= x
            && mouse_x < x + w
            && mouse_y >= y
            && mouse_y < y + h
            && self.contains_point_in_scissor(mouse_x, mouse_y);
        let (slider_sprite, handle_sprite) =
            slider_sprites(active, mouse_over, focused, can_change_value);
        if mouse_over {
            self.request_cursor(if active {
                CursorType::PointingHand
            } else {
                CursorType::NotAllowed
            });
        }
        self.blit_with_alpha(
            ResourceLocation::from_string(slider_sprite),
            x,
            y,
            w,
            h,
            alpha,
        );
        let handle_x = x + ((w - 8) as f32 * pct.clamp(0.0, 1.0)) as i32;
        self.blit_with_alpha(
            ResourceLocation::from_string(handle_sprite),
            handle_x,
            y,
            8,
            h,
            alpha,
        );
        let label_rgb = if active {
            0x00FF_FFFFu32
        } else {
            0x00A0_A0A0u32
        };
        let label_color = ((argb_white(alpha) as u32) & 0xFF00_0000) | label_rgb;
        self.draw_scrolling_text_with_default_center(
            font,
            text,
            x + 2,
            x + w - 2,
            y,
            y + h,
            label_color as i32,
        );
    }
    pub fn draw_scrolling_text_with_default_center(
        &mut self,
        font: &dyn crate::FontWidthMeasurer,
        text: &str,
        left: i32,
        right: i32,
        top: i32,
        bottom: i32,
        color: i32,
    ) {
        let now_millis = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        self.draw_scrolling_text_with_default_center_at_millis(
            font, text, left, right, top, bottom, color, now_millis,
        );
    }
    fn draw_scrolling_text_with_default_center_at_millis(
        &mut self,
        font: &dyn crate::FontWidthMeasurer,
        text: &str,
        left: i32,
        right: i32,
        top: i32,
        bottom: i32,
        color: i32,
        now_millis: u128,
    ) {
        let line_width = font.measure_width(text, 1.0).round() as i32;
        let layout = java_scrolling_text_layout(
            line_width,
            (left + right) / 2,
            left,
            right,
            top,
            bottom,
            now_millis,
        );
        if layout.scrolls {
            self.enable_scissor(left, top, right, bottom);
        }
        self.text_with_bounds(
            ResourceLocation::from_string("minecraft:default"),
            text.to_string(),
            layout.x,
            layout.y,
            color,
            true,
            false,
            Some(line_width),
        );
        if layout.scrolls {
            self.disable_scissor();
        }
    }
    pub fn draw_scrolling_text_with_default_center_estimated(
        &mut self,
        text: &str,
        left: i32,
        right: i32,
        top: i32,
        bottom: i32,
        color: i32,
    ) {
        let line_width_estimated = text.chars().count() as i32 * 6;
        let layout = java_scrolling_text_layout(
            line_width_estimated,
            (left + right) / 2,
            left,
            right,
            top,
            bottom,
            web_time::SystemTime::now()
                .duration_since(web_time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        );
        if layout.scrolls {
            self.enable_scissor(left, top, right, bottom);
        }
        self.text_with_bounds(
            ResourceLocation::from_string("minecraft:default"),
            text.to_string(),
            layout.x,
            layout.y,
            color,
            true,
            false,
            Some(line_width_estimated),
        );
        if layout.scrolls {
            self.disable_scissor();
        }
    }
    pub fn draw_checkbox(
        &mut self,
        _font: &dyn crate::FontWidthMeasurer,
        x: i32,
        y: i32,
        checked: bool,
        focused: bool,
        _hovered: bool,
        label: &str,
        label_color: i32,
        alpha: f32,
    ) {
        let sprite = match (checked, focused) {
            (true, true) => "minecraft:widget/checkbox_selected_highlighted",
            (true, false) => "minecraft:widget/checkbox_selected",
            (false, true) => "minecraft:widget/checkbox_highlighted",
            (false, false) => "minecraft:widget/checkbox",
        };
        self.blit_with_alpha(ResourceLocation::from_string(sprite), x, y, 17, 17, alpha);
        let label_width = _font.measure_width(label, 1.0).round() as i32;
        let channel = ((alpha.clamp(0.0, 1.0)) * 255.0) as i32;
        self.text_with_bounds(
            ResourceLocation::from_string("minecraft:default"),
            label.to_string(),
            x + 21,
            y + 4,
            (label_color & 0x00FF_FFFF) | (channel << 24),
            true,
            false,
            Some(label_width),
        );
    }
    pub fn draw_edit_box(
        &mut self,
        font: &dyn crate::FontWidthMeasurer,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        text: &str,
        hint: Option<&str>,
        focused: bool,
        cursor_visible: bool,
        cursor_pos: usize,
    ) {
        let mouse_x = self.mouse_x();
        let mouse_y = self.mouse_y();
        let hovered = mouse_x >= x && mouse_x < x + w && mouse_y >= y && mouse_y < y + h;
        if hovered {
            self.request_cursor(CursorType::Text);
        }
        let sprite = if focused {
            "minecraft:widget/text_field_highlighted"
        } else {
            "minecraft:widget/text_field"
        };
        self.blit(ResourceLocation::from_string(sprite), x, y, w, h);
        let inner_x = x + 4;
        let inner_y = y + (h - 8) / 2;
        self.enable_scissor(x + 1, y + 1, x + w - 1, y + h - 1);
        if text.is_empty() {
            if let Some(hint_text) = hint {
                let hint_width = font.measure_width(hint_text, 1.0).round() as i32;
                self.text_with_bounds(
                    ResourceLocation::from_string("minecraft:default"),
                    hint_text.to_string(),
                    inner_x,
                    inner_y,
                    0xFF55_5555u32 as i32,
                    true,
                    false,
                    Some(hint_width),
                );
            }
        } else {
            let text_width = font.measure_width(text, 1.0).round() as i32;
            self.text_with_bounds(
                ResourceLocation::from_string("minecraft:default"),
                text.to_string(),
                inner_x,
                inner_y,
                0xFFE0_E0E0u32 as i32,
                true,
                false,
                Some(text_width),
            );
        }
        self.disable_scissor();
        if focused && cursor_visible {
            let left_part: String = text.chars().take(cursor_pos).collect();
            let left_w = font.measure_width(&left_part, 1.0).round() as i32;
            let cx = inner_x + left_w;
            self.fill(cx, inner_y - 1, cx + 1, inner_y + 9, 0xFFE0_E0E0u32 as i32);
        }
    }
    fn tooltip_metrics(
        &mut self,
        font: &dyn crate::FontWidthMeasurer,
        lines: &[String],
    ) -> (i32, i32) {
        let mut text_width = 0;
        for line in lines {
            let w = font.measure_width(line, 1.0).round() as i32;
            if w > text_width {
                text_width = w;
            }
        }
        let text_height = if lines.len() == 1 {
            8
        } else {
            lines.len() as i32 * 10
        };
        (text_width, text_height)
    }
    fn draw_tooltip_background_and_lines(
        &mut self,
        font: &dyn crate::FontWidthMeasurer,
        lines: &[String],
        x: i32,
        y: i32,
        text_width: i32,
        text_height: i32,
    ) {
        let background_x = x - 12;
        let background_y = y - 12;
        let background_width = text_width + 24;
        let background_height = text_height + 24;
        self.blit(
            ResourceLocation::from_string("minecraft:tooltip/background"),
            background_x,
            background_y,
            background_width,
            background_height,
        );
        self.blit(
            ResourceLocation::from_string("minecraft:tooltip/frame"),
            background_x,
            background_y,
            background_width,
            background_height,
        );
        let mut text_y = y;
        for (i, line) in lines.iter().enumerate() {
            let line_width = font.measure_width(line, 1.0).round() as i32;
            self.text_with_bounds(
                ResourceLocation::from_string("minecraft:default"),
                line.clone(),
                x,
                text_y,
                0xFFFF_FFFFu32 as i32,
                true,
                false,
                Some(line_width),
            );
            text_y += 10 + if i == 0 { 2 } else { 0 };
        }
    }
    pub fn draw_tooltip(
        &mut self,
        font: &dyn crate::FontWidthMeasurer,
        lines: &[String],
        mouse_x: i32,
        mouse_y: i32,
    ) {
        if lines.is_empty() {
            return;
        }
        self.begin_tooltip_overlay();
        let (text_width, text_height) = self.tooltip_metrics(font, lines);
        let mut rx = mouse_x + 12;
        let mut ry = mouse_y - 12;
        if rx + text_width > self.gui_width() {
            rx = (mouse_x + 12 - 24 - text_width).max(4);
        }
        let padded_height = text_height + 3;
        if ry + padded_height > self.gui_height() {
            ry = self.gui_height() - padded_height;
        }
        self.draw_tooltip_background_and_lines(font, lines, rx, ry, text_width, text_height);
        self.end_tooltip_overlay();
    }
    pub fn draw_widget_tooltip(
        &mut self,
        font: &dyn crate::FontWidthMeasurer,
        lines: &[String],
        mouse_x: i32,
        mouse_y: i32,
        widget_rect: ScreenRectangle,
        position: WidgetTooltipPosition,
    ) {
        if lines.is_empty() {
            return;
        }
        self.begin_tooltip_overlay();
        let (text_width, text_height) = self.tooltip_metrics(font, lines);
        let (x, y) = match position {
            WidgetTooltipPosition::BelowOrAbove => {
                let mut rx = widget_rect.left() + 3;
                let mut ry = widget_rect.bottom() + 3 + 1;
                if ry + text_height + 3 > self.gui_height() {
                    ry = widget_rect.top() - text_height - 3 - 1;
                }
                if rx + text_width > self.gui_width() {
                    rx = (widget_rect.right() - text_width - 3).max(4);
                }
                (rx, ry)
            }
            WidgetTooltipPosition::Menu => {
                fn menu_offset(mouse_y: i32, widget_y: i32, widget_height: i32) -> i32 {
                    if widget_height <= 0 {
                        return 0;
                    }
                    let distance =
                        (mouse_y - widget_y).abs().min(widget_height) as f32 / widget_height as f32;
                    let lerped = widget_height as f32 - 3.0
                        + distance * (5.0 - (widget_height as f32 - 3.0));
                    lerped.round() as i32
                }
                let mut rx = mouse_x + 12;
                if rx + text_width > self.gui_width() - 5 {
                    rx = (mouse_x - 12 - text_width).max(9);
                }
                let mut ry = mouse_y;
                ry += 3;
                let padded_height = text_height + 3 + 3;
                let lowest_possible_y =
                    widget_rect.bottom() + 3 + menu_offset(0, 0, widget_rect.height);
                let max_y = self.gui_height() - 5;
                if lowest_possible_y + padded_height <= max_y {
                    ry += menu_offset(ry, widget_rect.top(), widget_rect.height);
                } else {
                    ry -= padded_height + menu_offset(ry, widget_rect.bottom(), widget_rect.height);
                }
                (rx, ry)
            }
        };
        self.draw_tooltip_background_and_lines(font, lines, x, y, text_width, text_height);
        self.end_tooltip_overlay();
    }
}
