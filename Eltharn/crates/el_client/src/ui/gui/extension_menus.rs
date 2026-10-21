use super::*;
use el_gui::screens::extension_catalog::ExtensionCatalogKey;
impl GuiManager {
    pub fn set_extension_catalog(&mut self, catalog: ExtensionCatalog) {
        if self.extension_catalog == catalog {
            return;
        }
        self.extension_catalog = catalog;
        if let Some(screen) = self.active_screen.as_mut() {
            refresh_catalog_screen(screen, &self.extension_catalog);
        }
        self.mark_screen_dirty();
    }
    pub fn mod_settings_owner(&self) -> Option<&str> {
        match self.active_screen.as_ref() {
            Some(ActiveScreen::ExtensionSettings { owner, .. }) => Some(owner),
            _ => None,
        }
    }
    pub fn take_extension_settings_request(&mut self) -> Option<String> {
        self.pending_extension_settings_request.take()
    }
    pub(super) fn force_extension_screen_for_capture(&mut self, id: &str, in_world: bool) -> bool {
        let kind = if id == "shaderpacks" {
            ExtensionCatalogKind::ShaderPacks
        } else {
            ExtensionCatalogKind::Mods
        };
        let enabled = match kind {
            ExtensionCatalogKind::Mods => self.extension_catalog.mods_enabled,
            ExtensionCatalogKind::ShaderPacks => self.extension_catalog.shaderpacks_enabled,
        };
        if !enabled {
            return false;
        }
        let owner = if id == "mod-settings" {
            let Some(owner) = self
                .extension_catalog
                .mods
                .iter()
                .filter(|entry| entry.selected)
                .find_map(|entry| entry.settings_owner.clone())
            else {
                return false;
            };
            Some(owner)
        } else {
            None
        };
        if kind == ExtensionCatalogKind::ShaderPacks {
            let mut video = VideoSettingsScreen::new(
                in_world,
                &self.extra_options,
                self.gui_scale_setting,
                std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
            );
            video.set_shaderpacks_enabled(self.extension_catalog.shaderpacks_enabled);
            self.active_screen = Some(ActiveScreen::Video(video));
        } else if in_world {
            self.open_pause_screen();
        } else {
            self.open_title_screen();
        }
        let Some(parent) = self.active_screen.take() else {
            return false;
        };
        self.open_extension_catalog(kind, parent);
        if let Some(owner) = owner {
            let Some(mut screen) = self.active_screen.take() else {
                return false;
            };
            self.pending_extension_settings_request = Some(owner.clone());
            wrap_in_extension_settings(&mut screen, owner);
            self.active_screen = Some(screen);
            self.mark_screen_dirty();
        }
        true
    }
    pub(super) fn open_extension_catalog(
        &mut self,
        kind: ExtensionCatalogKind,
        parent: ActiveScreen,
    ) {
        let (enabled, entries) = match kind {
            ExtensionCatalogKind::Mods => (
                self.extension_catalog.mods_enabled,
                &self.extension_catalog.mods,
            ),
            ExtensionCatalogKind::ShaderPacks => (
                self.extension_catalog.shaderpacks_enabled,
                &self.extension_catalog.shaderpacks,
            ),
        };
        if !enabled {
            self.active_screen = Some(parent);
            return;
        }
        let screen = ExtensionCatalogScreen::new(kind, parent.is_in_world(), entries.clone());
        self.active_screen = Some(ActiveScreen::ExtensionCatalog {
            screen,
            parent: Some(Box::new(parent)),
        });
        self.mark_screen_dirty();
    }
    pub(super) fn extension_catalog_event(
        &mut self,
        screen: &mut ExtensionCatalogScreen,
        event: &WindowEvent,
        width: f32,
        height: f32,
    ) -> Option<ExtensionCatalogAction> {
        screen.set_size(width.round() as i32, height.round() as i32);
        if matches!(event, WindowEvent::CursorMoved { .. }) {
            screen.mouse_moved(self.mouse_x, self.mouse_y);
        }
        if matches!(
            event,
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            }
        ) {
            screen.mouse_released();
        }
        let action = if pressed_key(event, KeyCode::Escape) {
            Some(ExtensionCatalogAction::Back)
        } else if let WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        } = event
        {
            screen.mouse_clicked(self.mouse_x, self.mouse_y)
        } else if let WindowEvent::MouseWheel { delta, .. } = event {
            let y = match delta {
                winit::event::MouseScrollDelta::LineDelta(_, y) => *y as f64,
                winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y,
            };
            screen.mouse_scrolled(y);
            None
        } else {
            let key = [
                (KeyCode::ArrowUp, ExtensionCatalogKey::Previous),
                (KeyCode::PageUp, ExtensionCatalogKey::Previous),
                (KeyCode::ArrowDown, ExtensionCatalogKey::Next),
                (KeyCode::PageDown, ExtensionCatalogKey::Next),
                (KeyCode::Home, ExtensionCatalogKey::Home),
                (KeyCode::End, ExtensionCatalogKey::End),
                (KeyCode::Tab, ExtensionCatalogKey::Tab),
                (KeyCode::Enter, ExtensionCatalogKey::Activate),
                (KeyCode::Space, ExtensionCatalogKey::Activate),
            ]
            .into_iter()
            .find_map(|(code, key)| pressed_key(event, code).then_some(key));
            key.and_then(|key| screen.key_pressed(key, self.shift_held))
        };
        self.mark_screen_dirty();
        action
    }
    pub(super) fn extension_settings_back(
        &self,
        event: &WindowEvent,
        width: f32,
        height: f32,
    ) -> bool {
        if pressed_key(event, KeyCode::Escape) {
            #[cfg(feature = "extensions")]
            if crate::modding::app_ui::request_settings_close() {
                return false;
            }
            return true;
        }
        if settings_source_footer() {
            return false;
        }
        if let WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        } = event
        {
            let button_width = 200.0f32.min((width - 16.0).max(24.0));
            let x = (width - button_width) / 2.0;
            if self.mouse_x >= x
                && self.mouse_x < x + button_width
                && self.mouse_y >= height - 26.0
                && self.mouse_y < height - 6.0
            {
                el_gui::components::click_sound_sink::record_button_click();
                #[cfg(feature = "extensions")]
                if crate::modding::app_ui::request_settings_close() {
                    return false;
                }
                return true;
            }
        }
        false
    }
    pub fn close_extension_settings(&mut self) -> bool {
        let (screen, closed) = settings_parent(self.active_screen.take());
        self.active_screen = screen;
        if closed {
            self.mark_screen_dirty();
        }
        closed
    }
}
fn settings_parent(screen: Option<ActiveScreen>) -> (Option<ActiveScreen>, bool) {
    match screen {
        Some(ActiveScreen::ExtensionSettings { parent, .. }) => {
            (parent.map(|parent| *parent), true)
        }
        other => (other, false),
    }
}
pub(super) fn wrap_in_extension_settings(screen: &mut ActiveScreen, owner: String) {
    let mut parent = ActiveScreen::ExtensionSettings {
        owner,
        parent: None,
    };
    std::mem::swap(screen, &mut parent);
    if let ActiveScreen::ExtensionSettings { parent: slot, .. } = screen {
        *slot = Some(Box::new(parent));
    }
}
fn refresh_catalog_screen(screen: &mut ActiveScreen, catalog: &ExtensionCatalog) {
    match screen {
        ActiveScreen::Title(screen) => {
            screen.set_extensions(catalog.mods_enabled, catalog.shaderpacks_enabled)
        }
        ActiveScreen::Pause(screen) => {
            screen.set_extensions(catalog.mods_enabled, catalog.shaderpacks_enabled)
        }
        ActiveScreen::Video(screen) => screen.set_shaderpacks_enabled(catalog.shaderpacks_enabled),
        ActiveScreen::ExtensionCatalog { screen, parent } => {
            let entries = match screen.kind {
                ExtensionCatalogKind::Mods => &catalog.mods,
                ExtensionCatalogKind::ShaderPacks => &catalog.shaderpacks,
            };
            screen.replace_entries(entries.clone());
            if let Some(parent) = parent.as_mut() {
                refresh_catalog_screen(parent, catalog);
            }
        }
        ActiveScreen::ExtensionSettings { parent, .. } => {
            if let Some(parent) = parent.as_mut() {
                refresh_catalog_screen(parent, catalog);
            }
        }
        _ => {}
    }
}
pub(super) fn settings_title() -> String {
    #[cfg(feature = "extensions")]
    if let Some(title) = crate::modding::app_ui::settings_title() {
        return title;
    }
    "Mod Settings".into()
}
pub(super) fn settings_source_footer() -> bool {
    #[cfg(feature = "extensions")]
    {
        crate::modding::app_ui::settings_source_footer()
    }
    #[cfg(not(feature = "extensions"))]
    {
        false
    }
}
pub(super) fn render_settings_shell(
    graphics: &mut GuiGraphicsExtractor,
    font: &dyn el_gui::FontWidthMeasurer,
    in_world: bool,
) {
    let (width, height) = (graphics.gui_width(), graphics.gui_height());
    if in_world {
        graphics.blur_before_this_stratum();
        graphics.next_stratum();
    }
    graphics.draw_menu_background(in_world, width, height);
    let label = settings_title();
    graphics.text(
        ResourceLocation::from_string("minecraft:default"),
        label.clone(),
        (width - font.measure_width(&label, 1.0) as i32) / 2,
        12,
        0xFFFFFFFFu32 as i32,
    );
    if !settings_source_footer() {
        let button_width = 200.min((width - 16).max(24));
        graphics.draw_button(
            font,
            (width - button_width) / 2,
            height - 26,
            button_width,
            20,
            "Done",
            true,
        );
    }
}
