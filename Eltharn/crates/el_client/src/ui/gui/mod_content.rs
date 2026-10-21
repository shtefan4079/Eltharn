use super::*;
use el_mod_content::scene_types::Widget;

fn label(text: &str) -> String {
    text.strip_prefix("translate:")
        .map(|key| crate::ui::i18n::translate(key).to_string())
        .unwrap_or_else(|| text.to_owned())
}

impl GuiManager {
    pub(super) fn render_mod_content(
        &mut self,
        device: &Device,
        queue: &Queue,
        layout: &wgpu::BindGroupLayout,
        width: i32,
        height: i32,
        focus: Option<(&str, u32)>,
    ) -> Result<(), String> {
        let scene = crate::modding::client_content::scene();
        self.renderer
            .sync_mod_canvases(device, queue, layout, &scene)?;
        if !self.rendering_in_world
            || self.active_screen.is_some()
            || !crate::modding::hooks::world_ui_visible()
        {
            return Ok(());
        }
        let mouse = [self.mouse_x as i32, self.mouse_y as i32];
        let font = ResourceLocation::new("minecraft", "default");
        if scene.actors.iter().any(|actor| actor.ui.screen) {
            let mut graphics = GuiGraphicsExtractor::new(width, height, mouse[0], mouse[1]);
            graphics.fill(0, 0, width, height, 0x88000000u32 as i32);
            self.render_eltharn_gui_state(device, queue, layout, &graphics.finish());
        }
        for actor in &scene.actors {
            for widget in &actor.ui.widgets {
                let [x, y, w, h] = widget.rect();
                let mut graphics = GuiGraphicsExtractor::new(width, height, mouse[0], mouse[1]);
                match widget {
                    Widget::Canvas { canvas, rect, .. } => {
                        self.renderer
                            .draw_mod_canvas(&actor.owner, *canvas, *rect)?;
                        continue;
                    }
                    Widget::Label { text, color, .. } => {
                        graphics.text(font.clone(), label(text), x, y, *color as i32)
                    }
                    Widget::Button {
                        id, text, color, ..
                    } => {
                        graphics.fill(x, y, x + w, y + h, *color as i32);
                        if focus == Some((actor.owner.as_str(), *id)) {
                            graphics.outline(x, y, w, h, -1);
                        }
                        graphics.text(font.clone(), label(text), x + 4, y + 4, -1);
                    }
                    Widget::Image { texture, .. } => {
                        crate::modding::client_content::require_image(texture)?;
                        let texture = ResourceLocation::new(
                            texture.namespace(),
                            &format!("textures/{}.png", texture.path()),
                        );
                        if !self.renderer.custom_texture_registry.contains_key(&texture) {
                            let bytes = self
                                .resource_manager
                                .read_resource_bytes(&texture)
                                .map_err(|e| format!("Mod UI image {texture}: {e}"))?;
                            crate::modding::resources::image_dimensions(&bytes)?;
                        }
                        graphics.blit(texture, x, y, w, h);
                    }
                    Widget::Slot {
                        id,
                        inventory,
                        slot,
                        ..
                    } => {
                        graphics.fill(x, y, x + w, y + h, 0xCC222222u32 as i32);
                        if focus == Some((actor.owner.as_str(), *id)) {
                            graphics.outline(x, y, w, h, -1);
                        }
                        self.render_eltharn_gui_state(device, queue, layout, &graphics.finish());
                        let stack = actor
                            .inventories
                            .iter()
                            .find(|i| i.id == *inventory)
                            .and_then(|i| i.slots.get(*slot as usize))
                            .ok_or("Invalid mod UI inventory slot")?;
                        if let Some(stack) = stack {
                            let previous = self.renderer.active_scissor;
                            self.renderer.active_scissor =
                                Some([x as f32, y as f32, (x + w) as f32, (y + h) as f32]);
                            let item = ResourceLocation::from_string(stack.item.as_str());
                            let drawn =
                                self.renderer
                                    .draw_mod_item(x as f32 + 1.0, y as f32 + 1.0, &item);
                            self.renderer.active_scissor = previous;
                            if !drawn {
                                return Err(format!("Missing mod item model {item}"));
                            }
                            if stack.count > 1 {
                                let mut count =
                                    GuiGraphicsExtractor::new(width, height, mouse[0], mouse[1]);
                                count.text(
                                    font.clone(),
                                    stack.count.to_string(),
                                    x + 1,
                                    y + h - 9,
                                    -1,
                                );
                                self.render_eltharn_gui_state(
                                    device,
                                    queue,
                                    layout,
                                    &count.finish(),
                                );
                            }
                        }
                        continue;
                    }
                }
                self.render_eltharn_gui_state(device, queue, layout, &graphics.finish());
            }
        }
        Ok(())
    }
}
