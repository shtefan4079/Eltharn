use super::extension_catalog::ExtensionCatalogKind;
use crate::FontWidthMeasurer;
use crate::components::sprite_icon_button::SpriteIconButton;
use crate::components::sprites::sprites as sprite_sets;
use crate::components::{
    GuiEventListener, Renderable, TabOrderedElement, WidgetClickSound, click_sound_sink,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{NarratableEntry, NarratedElementType, NarrationElementOutput};
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::framework::screen::{Screen, ScreenBase, screen_init};
use crate::input::events::{CharacterEvent, KeyEvent, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use web_time::Instant;
const BUTTON_SPACING: i32 = 24;
const BUTTON_HEIGHT: i32 = 20;
const FULL_BUTTON_WIDTH: i32 = 200;
const HALF_BUTTON_WIDTH: i32 = 98;
const ICON_BUTTON_WIDTH: i32 = 20;
const ICON_SIZE: i32 = 15;
const LOGO_WIDTH: i32 = 256;
const LOGO_HEIGHT: i32 = 44;
const LOGO_TEXTURE_WIDTH: f32 = 256.0;
const LOGO_TEXTURE_HEIGHT: f32 = 64.0;
const EDITION_WIDTH: i32 = 128;
const EDITION_HEIGHT: i32 = 14;
const EDITION_TEXTURE_WIDTH: f32 = 128.0;
const EDITION_TEXTURE_HEIGHT: f32 = 16.0;
const LOGO_HEIGHT_OFFSET: i32 = 30;
const EDITION_LOGO_OVERLAP: i32 = 7;
const SPLASH_WIDTH_OFFSET: f32 = 123.0;
const SPLASH_HEIGHT_OFFSET: f32 = 69.0;
const SPLASH_TEXT_ANGLE: f32 = -std::f32::consts::PI / 9.0;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleScreenAction {
    Multiplayer,
    Online,
    Friends,
    Language,
    Accessibility,
    Options,
    Quit,
    CreditsAndAttribution,
    PlayDemo,
    ResetDemo,
    OpenExtensionCatalog(ExtensionCatalogKind),
}
pub struct TitleScreen {
    base: ScreenBase,
    font: Arc<dyn FontWidthMeasurer>,
    fading: bool,
    created_at: Instant,
    fade_in_start: Option<Instant>,
    splash: Option<String>,
    hide_splash_texts: bool,
    version_name: String,
    modified: bool,
    demo: bool,
    demo_world_present: bool,
    multiplayer_allowed: bool,
    multiplayer_disabled_reason: Option<String>,
    show_easter_egg_logo: bool,
    pub buttons: Vec<TitleButton>,
    sprite_icon_buttons: Vec<(TitleButtonKind, SpriteIconButton<TitleScreenAction>)>,
    last_width: i32,
    last_height: i32,
    extensions: (bool, bool),
    extension_focus: Option<ExtensionCatalogKind>,
    action_sink: Rc<RefCell<Option<TitleScreenAction>>>,
}
impl TitleScreen {
    pub fn new(settings: TitleScreenSettings, font: Arc<dyn FontWidthMeasurer>) -> Self {
        let mut screen = Self {
            base: ScreenBase::new(Component::literal(&crate::translate(
                "narrator.screen.title",
            ))),
            font,
            fading: settings.fading,
            created_at: Instant::now(),
            fade_in_start: None,
            splash: settings.splash,
            hide_splash_texts: settings.hide_splash_texts,
            version_name: settings.version_name,
            modified: settings.modified,
            demo: settings.demo,
            demo_world_present: settings.demo_world_present,
            multiplayer_allowed: settings.multiplayer_allowed,
            multiplayer_disabled_reason: settings.multiplayer_disabled_reason,
            show_easter_egg_logo: settings.show_easter_egg_logo,
            buttons: Vec::new(),
            sprite_icon_buttons: Vec::new(),
            last_width: 0,
            last_height: 0,
            extensions: (false, false),
            extension_focus: None,
            action_sink: Rc::new(RefCell::new(None)),
        };
        screen.base.set_close_on_esc(false);
        screen
    }
    pub fn should_close_on_esc(&self) -> bool {
        false
    }
    pub fn take_action(&mut self) -> Option<TitleScreenAction> {
        self.action_sink.borrow_mut().take()
    }
    pub fn set_extensions(&mut self, mods: bool, _shaderpacks: bool) {
        if self.extensions != (mods, false) {
            self.extensions = (mods, false);
            self.extension_focus = None;
            self.buttons.clear();
            self.base.clear_widgets();
            let width = self.base.width;
            let height = self.base.height;
            if width > 0 && height > 0 {
                self.ensure_layout(width, height);
            }
        }
    }
    pub fn focus_extension_button(&mut self, reverse: bool) {
        self.extension_focus = super::extension_catalog::cycle_extension_focus(
            self.extension_focus,
            self.extensions.0,
            self.extensions.1,
            reverse,
        );
        if let Some(kind) = self.extension_focus {
            for (i, btn) in self.buttons.iter().enumerate() {
                if btn.action == Some(TitleScreenAction::OpenExtensionCatalog(kind)) {
                    self.base.set_focused(Some(i));
                    break;
                }
            }
        }
    }
    pub fn activate_extension_button(&self) -> Option<TitleScreenAction> {
        let kind = self.extension_focus?;
        click_sound_sink::record_button_click();
        let action = TitleScreenAction::OpenExtensionCatalog(kind);
        *self.action_sink.borrow_mut() = Some(action);
        Some(action)
    }
    pub fn mouse_clicked(&mut self, mouse_x: f32, mouse_y: f32) -> Option<TitleScreenAction> {
        let mx = mouse_x.floor() as i32;
        let my = mouse_y.floor() as i32;
        let accepted = self
            .buttons
            .iter()
            .find(|button| button.active && button.contains(mx, my) && button.action.is_some());
        if let Some(button) = accepted {
            click_sound_sink::record(button.click_sound());
        }
        let action = accepted.and_then(|button| button.action);
        if let Some(action) = action {
            *self.action_sink.borrow_mut() = Some(action);
        }
        action
    }
    pub fn render_screen(&mut self, graphics: &mut GuiGraphicsExtractor) {
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        self.ensure_layout(width, height);
        let widget_fade = self.update_fade();
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/title/background/panorama_overlay"),
            0,
            0,
            width,
            height,
        );
        let mouse_x = graphics.mouse_x();
        let mouse_y = graphics.mouse_y();
        let focused_idx = self.base.focused_index();
        let mut hovered_tooltip = None;
        for (i, button) in self.buttons.iter().enumerate() {
            let focused = focused_idx == Some(i)
                || (self.extension_focus.is_some()
                    && button.action
                        == self
                            .extension_focus
                            .map(TitleScreenAction::OpenExtensionCatalog));
            match button.kind {
                TitleButtonKind::Language | TitleButtonKind::Accessibility => {
                    if let Some((_, widget)) = self
                        .sprite_icon_buttons
                        .iter_mut()
                        .find(|(kind, _)| *kind == button.kind)
                    {
                        widget.base.set_focused(focused);
                        widget.base.alpha = widget_fade;
                        widget.extract_render_state(graphics, mouse_x, mouse_y, 0.0);
                    }
                }
                _ => {
                    button.extract_render_state(
                        graphics,
                        self.font.as_ref(),
                        mouse_x,
                        mouse_y,
                        widget_fade,
                        focused,
                    );
                    if button.active && button.contains(mouse_x, mouse_y) {
                        if let Some(tooltip) = button.tooltip.as_deref() {
                            hovered_tooltip = Some(tooltip.to_string());
                        }
                    }
                }
            }
        }
        self.extract_logo(graphics, width, widget_fade);
        if let Some(splash) = self.splash.as_deref() {
            if !self.hide_splash_texts {
                self.extract_splash(graphics, self.font.as_ref(), width, splash, widget_fade);
            }
        }
        let version_string = self.version_string();
        graphics.text(
            default_font(),
            version_string,
            2,
            height - 10,
            white(widget_fade),
        );
        if let Some(tooltip) = hovered_tooltip {
            graphics.draw_tooltip(self.font.as_ref(), &[tooltip], mouse_x, mouse_y);
        }
    }
    fn ensure_layout(&mut self, width: i32, height: i32) {
        if self.last_width == width && self.last_height == height && !self.buttons.is_empty() {
            return;
        }
        self.base.width = width;
        self.base.height = height;
        self.last_width = width;
        self.last_height = height;
        self.buttons.clear();
        self.sprite_icon_buttons.clear();
        self.base.clear_widgets();
        let mut top_pos = height / 4 + 48;
        if self.demo {
            top_pos = self.create_demo_menu_options(width, top_pos, BUTTON_SPACING);
        } else {
            top_pos = self.create_normal_menu_options(width, top_pos, BUTTON_SPACING);
        }
        top_pos += BUTTON_SPACING;
        let icon_count = if self.extensions.0 { 4 } else { 3 };
        self.buttons.push(TitleButton::icon(
            TitleButtonKind::Friends,
            horizontal_position(width, 1, icon_count, ICON_BUTTON_WIDTH),
            top_pos,
            "gui.friends.open",
            "minecraft:friends/friends",
            Some(TitleScreenAction::Friends),
            !self.demo,
            None,
        ));
        self.push_sprite_icon_button(
            TitleButtonKind::Language,
            horizontal_position(width, 2, icon_count, ICON_BUTTON_WIDTH),
            top_pos,
            "options.language",
            sprite_sets::ICON_LANGUAGE,
            Some(TitleScreenAction::Language),
            "options.language.narration",
            Some(translated("options.language.tooltip")),
        );
        self.push_sprite_icon_button(
            TitleButtonKind::Accessibility,
            horizontal_position(width, 3, icon_count, ICON_BUTTON_WIDTH),
            top_pos,
            "options.accessibility",
            sprite_sets::ICON_ACCESSIBILITY,
            Some(TitleScreenAction::Accessibility),
            "accessibility.onboarding.accessibility.button.narration",
            Some(translated("options.accessibility.tooltip")),
        );
        if self.extensions.0 {
            let mut button = TitleButton::icon(
                TitleButtonKind::Extension,
                horizontal_position(width, 4, icon_count, ICON_BUTTON_WIDTH),
                top_pos,
                "Mods",
                "eltharn:icon/mods",
                Some(TitleScreenAction::OpenExtensionCatalog(
                    ExtensionCatalogKind::Mods,
                )),
                true,
                None,
            );
            button.label = "Mods".into();
            button.tooltip = Some("Mods".into());
            self.buttons.push(button);
        }
        top_pos += BUTTON_SPACING;
        self.buttons.push(TitleButton::text(
            TitleButtonKind::Options,
            width / 2 - 100,
            top_pos,
            HALF_BUTTON_WIDTH,
            "menu.options",
            Some(TitleScreenAction::Options),
            true,
            None,
        ));
        self.buttons.push(TitleButton::text(
            TitleButtonKind::Quit,
            width / 2 + 2,
            top_pos,
            HALF_BUTTON_WIDTH,
            "menu.quit",
            Some(TitleScreenAction::Quit),
            true,
            None,
        ));
        let copyright = translated("title.credits");
        let copyright_width = self.font.measure_width(&copyright, 1.0).round() as i32;
        self.buttons.push(TitleButton::plain_text(
            TitleButtonKind::Copyright,
            width - copyright_width - 2,
            height - 10,
            copyright_width,
            10,
            "title.credits",
            Some(TitleScreenAction::CreditsAndAttribution),
        ));
        for button in &self.buttons {
            self.base.add_widget(TitleButtonWidget {
                x: button.x,
                y: button.y,
                width: button.width,
                height: button.height,
                label: button.label.clone(),
                action: button.action,
                plain_text: button.plain_text,
                active: button.active,
                focused: false,
                action_sink: Rc::clone(&self.action_sink),
            });
        }
    }
    fn push_sprite_icon_button(
        &mut self,
        kind: TitleButtonKind,
        x: i32,
        y: i32,
        message_key: &str,
        sprites: crate::components::sprites::WidgetSprites,
        action: Option<TitleScreenAction>,
        narration_key: &'static str,
        tooltip: Option<String>,
    ) {
        let mut builder = SpriteIconButton::<TitleScreenAction>::builder(Component::literal(
            &translated(message_key),
        ))
        .bounds(x, y, ICON_BUTTON_WIDTH, BUTTON_HEIGHT)
        .with_sprite(sprites, ICON_SIZE, ICON_SIZE)
        .icon_only(true)
        .narration(narration_key);
        if let Some(action) = action {
            builder = builder.with_action(action);
        }
        if let Some(tooltip) = tooltip {
            builder = builder.tooltip(vec![tooltip]);
        }
        self.sprite_icon_buttons.push((kind, builder.build()));
        self.buttons.push(TitleButton {
            kind,
            x,
            y,
            width: ICON_BUTTON_WIDTH,
            height: BUTTON_HEIGHT,
            label: translated(narration_key),
            tooltip: None,
            icon: None,
            active: true,
            action,
            plain_text: false,
        });
    }
    fn create_normal_menu_options(&mut self, width: i32, top_pos: i32, spacing: i32) -> i32 {
        let disabled_reason = self.multiplayer_disabled_reason.clone();
        self.buttons.push(TitleButton::text(
            TitleButtonKind::Singleplayer,
            width / 2 - 100,
            top_pos,
            FULL_BUTTON_WIDTH,
            "menu.singleplayer",
            None,
            true,
            None,
        ));
        self.buttons.push(TitleButton::text(
            TitleButtonKind::Multiplayer,
            width / 2 - 100,
            top_pos + spacing,
            FULL_BUTTON_WIDTH,
            "menu.multiplayer",
            Some(TitleScreenAction::Multiplayer),
            self.multiplayer_allowed,
            disabled_reason.clone(),
        ));
        self.buttons.push(TitleButton::text(
            TitleButtonKind::Online,
            width / 2 - 100,
            top_pos + spacing * 2,
            FULL_BUTTON_WIDTH,
            "menu.online",
            Some(TitleScreenAction::Online),
            self.multiplayer_allowed,
            disabled_reason,
        ));
        top_pos + spacing * 2
    }
    fn create_demo_menu_options(&mut self, width: i32, top_pos: i32, spacing: i32) -> i32 {
        self.buttons.push(TitleButton::text(
            TitleButtonKind::PlayDemo,
            width / 2 - 100,
            top_pos,
            FULL_BUTTON_WIDTH,
            "menu.playdemo",
            Some(TitleScreenAction::PlayDemo),
            true,
            None,
        ));
        self.buttons.push(TitleButton::text(
            TitleButtonKind::ResetDemo,
            width / 2 - 100,
            top_pos + spacing,
            FULL_BUTTON_WIDTH,
            "menu.resetdemo",
            Some(TitleScreenAction::ResetDemo),
            self.demo_world_present,
            None,
        ));
        top_pos + spacing
    }
    fn update_fade(&mut self) -> f32 {
        if !self.fading {
            return 1.0;
        }
        let start = *self.fade_in_start.get_or_insert_with(Instant::now);
        let fade =
            (Instant::now().duration_since(start).as_millis() as f32 / 2000.0).clamp(0.0, 1.0);
        if fade >= 1.0 {
            self.fading = false;
            return 1.0;
        }
        clamped_map(fade, 0.5, 1.0, 0.0, 1.0)
    }
    fn extract_logo(&self, graphics: &mut GuiGraphicsExtractor, width: i32, alpha: f32) {
        let logo_x = width / 2 - 128;
        let color = white(alpha);
        let logo = if self.show_easter_egg_logo {
            "minecraft:gui/title/minceraft"
        } else {
            "minecraft:gui/title/minecraft"
        };
        graphics.blit_region(
            ResourceLocation::from_string(logo),
            logo_x,
            LOGO_HEIGHT_OFFSET,
            LOGO_WIDTH,
            LOGO_HEIGHT,
            0.0,
            0.0,
            LOGO_WIDTH as f32 / LOGO_TEXTURE_WIDTH,
            LOGO_HEIGHT as f32 / LOGO_TEXTURE_HEIGHT,
            color,
        );
        graphics.blit_region(
            ResourceLocation::from_string("minecraft:gui/title/edition"),
            width / 2 - 64,
            LOGO_HEIGHT_OFFSET + LOGO_HEIGHT - EDITION_LOGO_OVERLAP,
            EDITION_WIDTH,
            EDITION_HEIGHT,
            0.0,
            0.0,
            EDITION_WIDTH as f32 / EDITION_TEXTURE_WIDTH,
            EDITION_HEIGHT as f32 / EDITION_TEXTURE_HEIGHT,
            color,
        );
    }
    fn extract_splash(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
        screen_width: i32,
        splash: &str,
        alpha: f32,
    ) {
        let text_width = font.measure_width(splash, 1.0).round() as i32;
        if text_width <= 0 {
            return;
        }
        let millis = self.created_at.elapsed().as_millis() as f32;
        let text_phase = 1.8
            - ((millis % 1000.0) / 1000.0 * std::f32::consts::TAU)
                .sin()
                .abs()
                * 0.1;
        let text_scale = text_phase * 100.0 / (text_width as f32 + 32.0);
        graphics.pose_mut().push_pose();
        graphics.pose_mut().translate(
            screen_width as f32 / 2.0 + SPLASH_WIDTH_OFFSET,
            SPLASH_HEIGHT_OFFSET,
        );
        graphics.pose_mut().rotate(SPLASH_TEXT_ANGLE);
        graphics.pose_mut().scale(text_scale, text_scale);
        graphics.text_with_shadow(
            default_font(),
            splash.to_string(),
            -text_width / 2,
            -8,
            argb(255, 255, 0, alpha),
            true,
        );
        graphics.pose_mut().pop_pose();
    }
    fn version_string(&self) -> String {
        let mut version = format!("Minecraft {}", self.version_name);
        if self.demo {
            version.push_str(" Demo");
        }
        if self.modified {
            version.push_str(&translated("menu.modded"));
        }
        version
    }
}
impl Screen for TitleScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn should_close_on_esc(&self) -> bool {
        false
    }
    fn init(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        self.last_width = 0;
        self.last_height = 0;
        self.ensure_layout(width, height);
    }
    fn reposition_elements(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        self.ensure_layout(width, height);
    }
    fn tick(&mut self) {
        self.base.tick();
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        crate::framework::screen::screen_mouse_clicked(self, event, double_click)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if event.is_escape() {
            return false;
        }
        let handled = crate::framework::screen::screen_key_pressed(self, event);
        if handled || (event.key == KeyEvent::KEY_TAB && self.base_ref().focused_index().is_some())
        {
            return true;
        }
        false
    }
    fn char_typed(&mut self, event: CharacterEvent) -> bool {
        crate::framework::screen::screen_char_typed(self, event)
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_ticks: f32,
    ) {
        self.render_screen(graphics);
    }
}
impl Clone for TitleScreen {
    fn clone(&self) -> Self {
        let mut cloned = Self::new(
            TitleScreenSettings {
                fading: self.fading,
                splash: self.splash.clone(),
                hide_splash_texts: self.hide_splash_texts,
                version_name: self.version_name.clone(),
                modified: self.modified,
                demo: self.demo,
                demo_world_present: self.demo_world_present,
                multiplayer_allowed: self.multiplayer_allowed,
                multiplayer_disabled_reason: self.multiplayer_disabled_reason.clone(),
                show_easter_egg_logo: self.show_easter_egg_logo,
            },
            Arc::clone(&self.font),
        );
        cloned.created_at = self.created_at;
        cloned.fade_in_start = self.fade_in_start;
        cloned.extensions = self.extensions;
        cloned.extension_focus = self.extension_focus;
        let (width, height) = (self.base.width, self.base.height);
        if width > 0 && height > 0 {
            screen_init(&mut cloned, width, height);
        }
        cloned
    }
}
impl std::fmt::Debug for TitleScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TitleScreen")
            .field("version_name", &self.version_name)
            .field("demo", &self.demo)
            .field("multiplayer_allowed", &self.multiplayer_allowed)
            .field("extensions", &self.extensions)
            .field("button_count", &self.buttons.len())
            .finish()
    }
}
#[derive(Clone, Debug)]
pub struct TitleScreenSettings {
    pub fading: bool,
    pub splash: Option<String>,
    pub hide_splash_texts: bool,
    pub version_name: String,
    pub modified: bool,
    pub demo: bool,
    pub demo_world_present: bool,
    pub multiplayer_allowed: bool,
    pub multiplayer_disabled_reason: Option<String>,
    pub show_easter_egg_logo: bool,
}
impl Default for TitleScreenSettings {
    fn default() -> Self {
        Self {
            fading: false,
            splash: Some("Minecraft!".to_string()),
            hide_splash_texts: false,
            version_name: "26.2".to_string(),
            modified: false,
            demo: false,
            demo_world_present: false,
            multiplayer_allowed: true,
            multiplayer_disabled_reason: None,
            show_easter_egg_logo: false,
        }
    }
}
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct TitleButton {
    pub kind: TitleButtonKind,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub label: String,
    pub tooltip: Option<String>,
    pub icon: Option<ResourceLocation>,
    pub active: bool,
    pub action: Option<TitleScreenAction>,
    pub plain_text: bool,
}
impl TitleButton {
    pub fn click_sound(&self) -> Option<WidgetClickSound> {
        if self.plain_text {
            None
        } else {
            Some(WidgetClickSound::UiButtonClick)
        }
    }
    fn text(
        kind: TitleButtonKind,
        x: i32,
        y: i32,
        width: i32,
        label_key: &str,
        action: Option<TitleScreenAction>,
        active: bool,
        tooltip: Option<String>,
    ) -> Self {
        Self {
            kind,
            x,
            y,
            width,
            height: BUTTON_HEIGHT,
            label: translated(label_key),
            tooltip,
            icon: None,
            active,
            action,
            plain_text: false,
        }
    }
    fn icon(
        kind: TitleButtonKind,
        x: i32,
        y: i32,
        label_key: &str,
        icon: &str,
        action: Option<TitleScreenAction>,
        active: bool,
        tooltip: Option<String>,
    ) -> Self {
        Self {
            kind,
            x,
            y,
            width: ICON_BUTTON_WIDTH,
            height: BUTTON_HEIGHT,
            label: translated(label_key),
            tooltip: tooltip.or_else(|| Some(translated(label_key))),
            icon: Some(ResourceLocation::from_string(icon)),
            active,
            action,
            plain_text: false,
        }
    }
    fn plain_text(
        kind: TitleButtonKind,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        label_key: &str,
        action: Option<TitleScreenAction>,
    ) -> Self {
        Self {
            kind,
            x,
            y,
            width,
            height,
            label: translated(label_key),
            tooltip: None,
            icon: None,
            active: true,
            action,
            plain_text: true,
        }
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
    fn extract_render_state(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
        mouse_x: i32,
        mouse_y: i32,
        alpha: f32,
        focused: bool,
    ) {
        let hovered = self.active && (self.contains(mouse_x, mouse_y) || focused);
        if self.plain_text {
            graphics.text(
                default_font(),
                self.label.clone(),
                self.x,
                self.y,
                white(alpha),
            );
            return;
        }
        let button_sprite = if !self.active {
            "minecraft:widget/button_disabled"
        } else if hovered {
            "minecraft:widget/button_highlighted"
        } else {
            "minecraft:widget/button"
        };
        graphics.blit_region(
            ResourceLocation::from_string(button_sprite),
            self.x,
            self.y,
            self.width,
            self.height,
            0.0,
            0.0,
            1.0,
            1.0,
            white(alpha),
        );
        if let Some(icon) = &self.icon {
            let icon_x = self.x + self.width / 2 - ICON_SIZE / 2;
            let icon_y = self.y + self.height / 2 - ICON_SIZE / 2;
            graphics.blit_region(
                icon.clone(),
                icon_x,
                icon_y,
                ICON_SIZE,
                ICON_SIZE,
                0.0,
                0.0,
                1.0,
                1.0,
                white(alpha),
            );
        } else {
            let color = if self.active {
                white(alpha)
            } else {
                argb(160, 160, 160, alpha)
            };
            graphics.draw_scrolling_text_with_default_center(
                font,
                &self.label,
                self.x + 2,
                self.x + self.width - 2,
                self.y,
                self.y + self.height,
                color,
            );
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleButtonKind {
    Singleplayer,
    Multiplayer,
    Online,
    Friends,
    Language,
    Accessibility,
    Options,
    Quit,
    Copyright,
    PlayDemo,
    ResetDemo,
    Extension,
}
struct TitleButtonWidget {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    label: String,
    action: Option<TitleScreenAction>,
    plain_text: bool,
    active: bool,
    focused: bool,
    action_sink: Rc<RefCell<Option<TitleScreenAction>>>,
}
impl crate::components::Renderable for TitleButtonWidget {
    fn extract_render_state(
        &mut self,
        _graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
    }
}
impl TabOrderedElement for TitleButtonWidget {
    fn tab_order_group(&self) -> i32 {
        0
    }
}
impl LayoutElement for TitleButtonWidget {
    fn set_x(&mut self, x: i32) {
        self.x = x;
    }
    fn set_y(&mut self, y: i32) {
        self.y = y;
    }
    fn x(&self) -> i32 {
        self.x
    }
    fn y(&self) -> i32 {
        self.y
    }
    fn width(&self) -> i32 {
        self.width
    }
    fn height(&self) -> i32 {
        self.height
    }
}
impl NarratableEntry for TitleButtonWidget {
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        output.add(NarratedElementType::Title, &self.label);
    }
}
impl GuiEventListener for TitleButtonWidget {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        if !self.active || event.button() != 0 {
            return false;
        }
        if event.x as i32 >= self.x
            && (event.x as i32) < self.x + self.width
            && event.y as i32 >= self.y
            && (event.y as i32) < self.y + self.height
        {
            click_sound_sink::record(self.play_down_sound());
            if let Some(action) = self.action {
                *self.action_sink.borrow_mut() = Some(action);
            }
            true
        } else {
            false
        }
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if !self.active {
            return false;
        }
        if event.is_selection() {
            if let Some(action) = self.action {
                *self.action_sink.borrow_mut() = Some(action);
            }
            true
        } else {
            false
        }
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        if self.plain_text {
            None
        } else {
            Some(WidgetClickSound::UiButtonClick)
        }
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        let mx = mouse_x as i32;
        let my = mouse_y as i32;
        mx >= self.x && mx < self.x + self.width && my >= self.y && my < self.y + self.height
    }
    fn rectangle(&self) -> ScreenRectangle {
        ScreenRectangle::new(self.x, self.y, self.width, self.height)
    }
    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
    fn is_focused(&self) -> bool {
        self.focused
    }
    fn next_focus_path(&self, _navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.active.then(|| ComponentPath::leaf(0))
    }
}
fn horizontal_position(
    screen_width: i32,
    current_button: i32,
    number_of_buttons: i32,
    button_width: i32,
) -> i32 {
    let total_width = number_of_buttons * button_width + (number_of_buttons - 1) * 4;
    screen_width / 2 - total_width / 2 + (current_button - 1) * (button_width + 4)
}
fn default_font() -> ResourceLocation {
    ResourceLocation::from_string("minecraft:default")
}
fn translated(key: &str) -> String {
    crate::translate(key)
}
fn white(alpha: f32) -> i32 {
    argb(255, 255, 255, alpha)
}
fn argb(r: u8, g: u8, b: u8, alpha: f32) -> i32 {
    let a = (alpha.clamp(0.0, 1.0) * 255.0).floor() as u32;
    ((a << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b as u32) as i32
}
fn clamped_map(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    let t = ((value - from_min) / (from_max - from_min)).clamp(0.0, 1.0);
    to_min + t * (to_max - to_min)
}
