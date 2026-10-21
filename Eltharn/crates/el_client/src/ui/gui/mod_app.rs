use super::*;
use el_mod_runtime::app::{ButtonStyle, Widget};
pub(crate) fn native_menu_tooltip_position(
    screen: [i32; 2],
    rect: [i32; 4],
    mouse: [i32; 2],
    size: [i32; 2],
    keyboard: bool,
) -> [i32; 2] {
    let [left, top, width, height] = rect;
    let [tooltip_width, tooltip_height] = size;
    let right = left + width;
    let bottom = top + height;
    if keyboard {
        let mut x = left + 3;
        let mut y = bottom + 4;
        if y + tooltip_height + 3 > screen[1] {
            y = top - tooltip_height - 4;
        }
        if x + tooltip_width > screen[0] {
            x = (right - tooltip_width - 3).max(4);
        }
        return [x, y];
    }
    let offset = |mouse_y: i32, widget_y: i32| {
        let distance = (mouse_y - widget_y).abs().min(height);
        let from = (height - 3) as f32;
        let value = from + (distance as f32 / height as f32) * (5.0 - from);
        (value as f64 + 0.5).floor() as i32
    };
    let mut x = mouse[0] + 12;
    if x + tooltip_width > screen[0] - 5 {
        x = (mouse[0] - 12 - tooltip_width).max(9);
    }
    let mut y = mouse[1] + 3;
    let padded_height = tooltip_height + 6;
    if bottom + 3 + offset(0, 0) + padded_height <= screen[1] - 5 {
        y += offset(y, top);
    } else {
        y -= padded_height + offset(y, bottom);
    }
    [x, y]
}
fn native_widget_cursor(
    widget: &Widget,
    hovered: bool,
) -> Option<el_gui::input::events::CursorType> {
    use el_gui::input::events::CursorType;
    if !hovered {
        return None;
    }
    match widget {
        Widget::Action { .. }
        | Widget::Toggle { .. }
        | Widget::Slider { .. }
        | Widget::KeyBinding { .. }
        | Widget::TextField { .. }
            if !widget.interactive() =>
        {
            Some(CursorType::NotAllowed)
        }
        Widget::TextField { .. } => Some(CursorType::Text),
        _ if widget.interactive() => Some(CursorType::PointingHand),
        _ => None,
    }
}
impl GuiManager {
    pub(crate) fn mod_app_screen(&self) -> Option<std::mem::Discriminant<ActiveScreen>> {
        self.active_screen.as_ref().map(std::mem::discriminant)
    }
    pub(crate) fn mod_app_menu(&self) -> &'static str {
        crate::modding::app_ui::services(
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
            self.clipboard.clone(),
        );
        match self.active_screen.as_ref() {
            Some(ActiveScreen::Title(_)) => "title",
            Some(ActiveScreen::Multiplayer(_)) => "multiplayer",
            Some(ActiveScreen::Options(_)) => "options",
            Some(ActiveScreen::Connecting(_)) => "connecting",
            Some(ActiveScreen::ExtensionSettings { .. }) => "mod_settings",
            _ => "other",
        }
    }
    pub(super) fn render_mod_app(
        &mut self,
        device: &Device,
        queue: &Queue,
        layout: &wgpu::BindGroupLayout,
        width: i32,
        height: i32,
    ) {
        let settings = self.mod_settings_owner().is_some();
        if (self.rendering_in_world && !settings)
            || (!settings && !crate::modding::hooks::menu_ui_visible())
        {
            return;
        }
        let scene = crate::modding::app_ui::scene();
        let mouse = crate::modding::app_ui::pointer();
        let font = ResourceLocation::new("minecraft", "default");
        let mut tooltip = None;
        for (owner, ui) in &scene {
            for widget in &ui.widgets {
                let [x, y, w, h] = widget.rect();
                let in_clip = !settings
                    || (mouse[0] >= 0
                        && mouse[0] < width
                        && if ui.is_footer(widget.id()) {
                            mouse[1] >= height - 32 && mouse[1] < height
                        } else {
                            mouse[1] >= if ui.layout.is_some() { 33 } else { 0 }
                                && mouse[1] < height - 32
                        });
                let hovered = widget.contains(mouse) && in_clip;
                let focused = crate::modding::app_ui::focused(owner, widget.id());
                let keyboard_focus = focused && crate::modding::app_ui::keyboard_navigation();
                let draw_mouse = if in_clip { mouse } else { [-32768, -32768] };
                let mut graphics =
                    GuiGraphicsExtractor::new(width, height, draw_mouse[0], draw_mouse[1]);
                if settings {
                    let footer = ui.is_footer(widget.id());
                    graphics.enable_scissor(
                        0,
                        if footer {
                            (height - 32).max(0)
                        } else if ui.layout.is_some() {
                            33
                        } else {
                            0
                        },
                        width,
                        if footer { height } else { (height - 32).max(1) },
                    );
                }
                match widget {
                    Widget::Image {
                        texture, uv, color, ..
                    } => {
                        let Ok(id) = texture.parse::<el_mod_content::ContentId>() else {
                            continue;
                        };
                        if crate::modding::client_content::require_owned_image(owner, &id).is_err()
                        {
                            continue;
                        }
                        graphics.blit_resource_region(
                            ResourceLocation::new(id.namespace(), id.path()),
                            x,
                            y,
                            w,
                            h,
                            uv[0] as f32 / 10_000.0,
                            uv[1] as f32 / 10_000.0,
                            uv[2] as f32 / 10_000.0,
                            uv[3] as f32 / 10_000.0,
                            *color as i32,
                        );
                    }
                    Widget::Panel { color, .. } => graphics.fill(x, y, x + w, y + h, *color as i32),
                    Widget::Label { text, color, .. } => {
                        graphics.text(font.clone(), text.clone(), x, y, *color as i32)
                    }
                    Widget::Button {
                        text, color, style, ..
                    } => match style {
                        ButtonStyle::Flat => {
                            graphics.fill(x, y, x + w, y + h, *color as i32);
                            if hovered {
                                graphics.outline(x, y, w, h, -1);
                            }
                            graphics.text(font.clone(), text.clone(), x + 4, y + 4, -1);
                        }
                        ButtonStyle::Vanilla => {
                            graphics.draw_button(self.font.as_ref(), x, y, w, h, text, true)
                        }
                    },
                    Widget::Action { text, enabled, .. } => graphics.draw_button_with_focus(
                        self.font.as_ref(),
                        x,
                        y,
                        w,
                        h,
                        text,
                        *enabled,
                        crate::modding::app_ui::focused(owner, widget.id()),
                    ),
                    Widget::Toggle {
                        text,
                        value,
                        enabled,
                        ..
                    } => graphics.draw_button_with_focus(
                        self.font.as_ref(),
                        x,
                        y,
                        w,
                        h,
                        &format!("{text}: {}", if *value { "ON" } else { "OFF" }),
                        *enabled,
                        crate::modding::app_ui::focused(owner, widget.id()),
                    ),
                    Widget::KeyBinding {
                        text,
                        value,
                        enabled,
                        ..
                    } => {
                        let label = if crate::modding::app_ui::capturing(owner, widget.id()) {
                            format!("{text}: > Press a key <")
                        } else {
                            format!("{text}: {value}")
                        };
                        graphics.draw_button_with_focus(
                            self.font.as_ref(),
                            x,
                            y,
                            w,
                            h,
                            &label,
                            *enabled,
                            crate::modding::app_ui::focused(owner, widget.id()),
                        );
                    }
                    Widget::Slider {
                        text,
                        value,
                        min,
                        max,
                        enabled,
                        ..
                    } => {
                        let focused = crate::modding::app_ui::focused(owner, widget.id());
                        let dragging = crate::modding::app_ui::dragging(owner, widget.id());
                        graphics.draw_slider_with_interaction_state(
                            self.font.as_ref(),
                            x,
                            y,
                            w,
                            h,
                            &format!("{text}: {value}"),
                            (*value - *min) as f32 / (*max - *min) as f32,
                            *enabled,
                            focused,
                            focused,
                            dragging,
                        );
                    }
                    Widget::TextField { text, .. } => {
                        graphics.text(font.clone(), text.clone(), x, y - 10, 0xffaaaaaau32 as i32);
                        if let Some(plan) = crate::modding::app_ui::text_plan(owner, widget.id()) {
                            use el_gui::components::edit_box::EditBoxDraw;
                            for draw in &plan.draws {
                                if let EditBoxDraw::Sprite {
                                    sprite,
                                    x,
                                    y,
                                    width,
                                    height,
                                } = draw
                                {
                                    graphics.blit(
                                        ResourceLocation::from_string(sprite),
                                        *x,
                                        *y,
                                        *width,
                                        *height,
                                    );
                                }
                            }
                            for draw in plan.draws {
                                match draw {
                                    EditBoxDraw::Sprite { .. } => {}
                                    EditBoxDraw::Text {
                                        text,
                                        x,
                                        y,
                                        color,
                                        shadow,
                                    } => graphics.text_with_shadow(
                                        font.clone(),
                                        text,
                                        x,
                                        y,
                                        color,
                                        shadow,
                                    ),
                                    EditBoxDraw::Highlight {
                                        x0,
                                        y0,
                                        x1,
                                        y1,
                                        invert,
                                    } => {
                                        let (left, right) = (x0.min(x1), x0.max(x1));
                                        let (top, bottom) = (y0.min(y1), y0.max(y1));
                                        graphics.text_highlight(left, top, right, bottom, invert);
                                    }
                                    EditBoxDraw::InsertCursor { x, y, color } => {
                                        graphics.fill(x, y - 1, x + 1, y + 9, color)
                                    }
                                    EditBoxDraw::AppendCursor {
                                        x,
                                        y,
                                        color,
                                        shadow,
                                    } => graphics.text_with_shadow(
                                        font.clone(),
                                        "_",
                                        x,
                                        y,
                                        color,
                                        shadow,
                                    ),
                                }
                            }
                        }
                    }
                }
                if let Some(cursor) = native_widget_cursor(widget, hovered) {
                    self.pending_cursor = cursor;
                }
                if crate::modding::app_ui::tooltip_visible(owner, widget.id(), hovered) {
                    if let Some(p) = ui
                        .presentation(widget.id())
                        .filter(|p| !p.tooltip.is_empty())
                    {
                        if tooltip.is_none() || focused {
                            tooltip = Some((
                                p.tooltip.clone(),
                                widget.rect(),
                                keyboard_focus && !hovered,
                            ));
                        }
                    }
                }
                if settings {
                    graphics.disable_scissor();
                }
                self.render_eltharn_gui_state(device, queue, layout, &graphics.finish());
            }
            if let Some(mut scrollbar) = crate::modding::app_ui::menu_scrollbar(owner) {
                let mut graphics = GuiGraphicsExtractor::new(width, height, mouse[0], mouse[1]);
                if scrollbar.is_scrolling()
                    || scrollbar.is_over_scrollbar(
                        mouse[0] as f64,
                        mouse[1] as f64,
                        scrollbar.viewport(),
                    )
                {
                    self.pending_cursor = el_gui::input::events::CursorType::PointingHand;
                }
                scrollbar.extract_scrollbar(&mut graphics);
                self.render_eltharn_gui_state(device, queue, layout, &graphics.finish());
            }
        }
        if let Some((tooltip, rect, keyboard)) = tooltip {
            let lines = el_gui::wrap_text(&tooltip, 170.0, self.font.as_ref());
            let text = lines.join("\n");
            let size = self.text_tooltip_size(&text).map(|v| v as i32);
            let [x, y] = native_menu_tooltip_position([width, height], rect, mouse, size, keyboard);
            self.draw_text_tooltip_at(&text, x as f32, y as f32);
        }
    }
}
