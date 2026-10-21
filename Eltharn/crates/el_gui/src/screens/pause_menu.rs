use super::confirm_screen::FriendsListConfirmScreen;
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
use crate::framework::screen::{self, Screen, ScreenBase};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
const COLUMNS: i32 = 2;
const MENU_PADDING_TOP: i32 = 50;
const BUTTON_PADDING: i32 = 4;
const BUTTON_WIDTH_FULL: i32 = 204;
const BUTTON_WIDTH_HALF: i32 = 98;
const BUTTON_HEIGHT: i32 = 20;
const ICON_SIZE: i32 = 15;
const ICON_BUTTON_WIDTH: i32 = 20;
const MENU_BACKGROUND_TILE: i32 = 32;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PauseMenuAction {
    ReturnToGame,
    OpenSubmenu(PauseSubmenu),
    Disconnect,
    OpenExtensionCatalog(ExtensionCatalogKind),
    ConfirmFriendsList,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseSubmenu {
    Singleplayer,
    Advancements,
    Stats,
    ReportBugs,
    Feedback,
    Friends,
    Realms,
    PlayerReporting,
    Options,
    MultiplayerOptions,
    OnlineOptions,
    Credits,
    DemoReset,
    SkinCustomization,
    MusicAndSounds,
    VideoSettings,
    Controls,
    Language,
    ChatSettings,
    ResourcePacks,
    Accessibility,
    Telemetry,
    CreditsAndAttribution,
    WorldOptions,
    GameRulesUnavailable,
    ServerLinks,
}
impl PauseSubmenu {
    pub fn title_key(self) -> &'static str {
        match self {
            Self::Singleplayer => "menu.singleplayer",
            Self::Advancements => "gui.advancements",
            Self::Stats => "gui.stats",
            Self::ReportBugs => "menu.reportBugs",
            Self::Feedback => "menu.sendFeedback",
            Self::Friends => "gui.friends.open",
            Self::Realms => "menu.online",
            Self::PlayerReporting => "menu.playerReporting",
            Self::Options => "options.title",
            Self::MultiplayerOptions => "menu.multiplayerOptions.button",
            Self::OnlineOptions => "options.online.title",
            Self::Credits => "credits_and_attribution.button.credits",
            Self::DemoReset => "menu.resetdemo",
            Self::SkinCustomization => "options.skinCustomisation",
            Self::MusicAndSounds => "options.sounds",
            Self::VideoSettings => "options.video",
            Self::Controls => "options.controls",
            Self::Language => "options.language",
            Self::ChatSettings => "options.chat",
            Self::ResourcePacks => "options.resourcepack",
            Self::Accessibility => "options.accessibility.title",
            Self::Telemetry => "options.telemetry",
            Self::CreditsAndAttribution => "options.credits_and_attribution",
            Self::WorldOptions => "options.worldOptions",
            Self::GameRulesUnavailable => "editGamerule.inGame.button",
            Self::ServerLinks => "menu.server_links.title",
        }
    }
}
pub struct PauseMenuScreen {
    base: ScreenBase,
    show_pause_menu: bool,
    singleplayer_server: bool,
    local_server: bool,
    pub has_reportable_players: bool,
    pub has_draft_report: bool,
    pub buttons: Vec<PauseButton>,
    sprite_icon_buttons: Vec<(PauseButtonKind, SpriteIconButton<PauseMenuAction>)>,
    friend_list_enabled: bool,
    title: String,
    last_width: i32,
    last_height: i32,
    extensions: (bool, bool),
    server_links_available: bool,
    custom_addition_title: Option<String>,
    extension_focus: Option<ExtensionCatalogKind>,
    font: Arc<dyn FontWidthMeasurer>,
    action_sink: Rc<RefCell<Option<PauseMenuAction>>>,
}
impl PauseMenuScreen {
    pub fn new(
        show_pause_menu: bool,
        has_singleplayer_server: bool,
        is_local_server: bool,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let title_key = if show_pause_menu {
            "menu.game"
        } else {
            "menu.paused"
        };
        let title = translated(title_key);
        let mut base = ScreenBase::new(Component::translatable(title_key));
        base.set_close_on_esc(true);
        base.set_pause_screen(true);
        Self {
            base,
            show_pause_menu,
            singleplayer_server: has_singleplayer_server,
            local_server: is_local_server,
            has_reportable_players: false,
            has_draft_report: false,
            buttons: Vec::new(),
            sprite_icon_buttons: Vec::new(),
            friend_list_enabled: true,
            title,
            last_width: 0,
            last_height: 0,
            extensions: (false, false),
            server_links_available: false,
            custom_addition_title: None,
            extension_focus: None,
            font,
            action_sink: Rc::new(RefCell::new(None)),
        }
    }
    pub fn take_action(&mut self) -> Option<PauseMenuAction> {
        if let Some(action) = self.action_sink.borrow_mut().take() {
            return Some(action);
        }
        if self.base.take_close_request() {
            return Some(PauseMenuAction::ReturnToGame);
        }
        None
    }
    pub fn show_pause_menu(&self) -> bool {
        self.show_pause_menu
    }
    pub fn layout_for_size(&self, width: i32, height: i32) -> PauseMenuLayout {
        let mut layout = PauseMenuLayout::calculate_with_server_links(
            width,
            height,
            self.singleplayer_server,
            self.local_server,
            self.server_links_available,
        );
        if let Some(title) = self.custom_addition_title.as_ref()
            && let Some(button) = layout
                .buttons
                .iter_mut()
                .find(|button| button.kind == PauseButtonKind::ServerLinks)
        {
            button.label = title.clone();
        }
        if !self.extensions.0 {
            return layout;
        }
        let icon_kinds = [
            PauseButtonKind::ReportBugs,
            PauseButtonKind::Feedback,
            PauseButtonKind::Friends,
            PauseButtonKind::PlayerReporting,
        ];
        let row_width = ICON_BUTTON_WIDTH * 5 + BUTTON_PADDING * 4;
        let first_x = layout.grid_x + (layout.grid_width - row_width) / 2;
        let mut row_y = 0;
        for (index, kind) in icon_kinds.iter().enumerate() {
            let button = layout
                .buttons
                .iter_mut()
                .find(|button| button.kind == *kind)
                .unwrap();
            button.x = first_x + index as i32 * (ICON_BUTTON_WIDTH + BUTTON_PADDING);
            row_y = button.y;
        }
        let mut mods = PauseButton::icon(
            PauseButtonKind::Extension,
            first_x + 4 * (ICON_BUTTON_WIDTH + BUTTON_PADDING),
            row_y,
            "Mods",
            "eltharn:icon/mods",
            Some(PauseMenuAction::OpenExtensionCatalog(
                ExtensionCatalogKind::Mods,
            )),
        );
        mods.label = "Mods".into();
        mods.tooltip = Some("Mods".into());
        layout.buttons.push(mods);
        layout
    }
    pub fn set_extensions(&mut self, mods: bool, _shaderpacks: bool) {
        if self.extensions != (mods, false) {
            self.extensions = (mods, false);
            self.extension_focus = None;
            self.buttons.clear();
            self.base.clear_widgets();
            if self.base.width > 0 && self.base.height > 0 {
                self.init();
            }
        }
    }
    pub fn set_server_links_available(&mut self, available: bool) {
        if self.server_links_available == available {
            return;
        }
        self.server_links_available = available;
        self.buttons.clear();
        self.base.clear_widgets();
        if self.base.width > 0 && self.base.height > 0 {
            self.init();
        }
    }
    pub fn set_friend_list_enabled(&mut self, enabled: bool) {
        if self.friend_list_enabled == enabled {
            return;
        }
        self.friend_list_enabled = enabled;
        self.buttons.clear();
        self.base.clear_widgets();
        if self.base.width > 0 && self.base.height > 0 {
            self.init();
        }
    }
    pub fn friends_confirm_screen(
        &self,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Option<FriendsListConfirmScreen> {
        (!self.friend_list_enabled).then(|| FriendsListConfirmScreen::new(true, font))
    }
    fn build_sprite_icon_buttons(&mut self) {
        self.sprite_icon_buttons = self
            .buttons
            .iter()
            .filter_map(|button| {
                let (sprites, message_key, narration_key, tooltip) = match button.kind {
                    PauseButtonKind::ReportBugs => (
                        sprite_sets::PAUSE_REPORT_BUGS,
                        "menu.reportBugs",
                        None,
                        Some(vec![translated("menu.reportBugs")]),
                    ),
                    PauseButtonKind::Feedback => (
                        sprite_sets::PAUSE_FEEDBACK,
                        "menu.sendFeedback",
                        None,
                        Some(vec![translated("menu.sendFeedback")]),
                    ),
                    PauseButtonKind::Friends => (
                        sprite_sets::FRIENDS_BUTTON,
                        "gui.friends.open",
                        Some("gui.friends.open.narration"),
                        Some(vec![translated("gui.friends.open.tooltip")]),
                    ),
                    PauseButtonKind::PlayerReporting => (
                        sprite_sets::PAUSE_PLAYER_REPORTING,
                        "menu.playerReporting",
                        None,
                        button.tooltip.as_ref().map(|line| vec![line.clone()]),
                    ),
                    _ => return None,
                };
                let mut builder = SpriteIconButton::<PauseMenuAction>::builder(Component::literal(
                    &translated(message_key),
                ))
                .bounds(button.x, button.y, button.width, button.height)
                .with_sprite(sprites, ICON_SIZE, ICON_SIZE)
                .icon_only(true);
                if let Some(lines) = tooltip {
                    builder = builder.tooltip(lines);
                }
                if let Some(narration_key) = narration_key {
                    builder = builder.narration(narration_key);
                }
                if let Some(action) = button.action.clone() {
                    builder = builder.with_action(action);
                }
                let mut component = builder.build();
                component.base.active = button.active;
                Some((button.kind, component))
            })
            .collect();
    }
    pub fn set_custom_addition(&mut self, available: bool, title: Option<String>) {
        if self.server_links_available == available && self.custom_addition_title == title {
            return;
        }
        self.server_links_available = available;
        self.custom_addition_title = title;
        self.buttons.clear();
        self.base.clear_widgets();
        if self.base.width > 0 && self.base.height > 0 {
            self.init();
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
                if btn.action == Some(PauseMenuAction::OpenExtensionCatalog(kind)) {
                    self.base.set_focused(Some(i));
                    break;
                }
            }
        }
    }
    pub fn activate_extension_button(&self) -> Option<PauseMenuAction> {
        let kind = self.extension_focus?;
        crate::components::click_sound_sink::record_button_click();
        let action = PauseMenuAction::OpenExtensionCatalog(kind);
        *self.action_sink.borrow_mut() = Some(action.clone());
        Some(action)
    }
    pub fn mouse_clicked(&mut self, mouse_x: f32, mouse_y: f32) -> Option<PauseMenuAction> {
        let hit = self.buttons.iter().find(|button| {
            button.active && button.contains(mouse_x.floor() as i32, mouse_y.floor() as i32)
        })?;
        crate::components::click_sound_sink::record_button_click();
        let action = hit.action.clone();
        if let Some(ref action) = action {
            *self.action_sink.borrow_mut() = Some(action.clone());
        }
        action
    }
    pub fn ensure_layout(&mut self, width: i32, height: i32) {
        if self.last_width == width && self.last_height == height && !self.buttons.is_empty() {
            return;
        }
        self.last_width = width;
        self.last_height = height;
        self.base.width = width;
        self.base.height = height;
        self.init();
    }
    fn extract_tiled_background(&self, graphics: &mut GuiGraphicsExtractor) {
        if !self.show_pause_menu {
            return;
        }
        self.base.extract_blurred_background(graphics);
        let texture = ResourceLocation::from_string("minecraft:gui/inworld_menu_background");
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        let mut y = 0;
        while y < height {
            let tile_h = MENU_BACKGROUND_TILE.min(height - y);
            let mut x = 0;
            while x < width {
                let tile_w = MENU_BACKGROUND_TILE.min(width - x);
                graphics.blit_region(
                    texture.clone(),
                    x,
                    y,
                    tile_w,
                    tile_h,
                    0.0,
                    0.0,
                    tile_w as f32 / MENU_BACKGROUND_TILE as f32,
                    tile_h as f32 / MENU_BACKGROUND_TILE as f32,
                    0xFFFF_FFFFu32 as i32,
                );
                x += MENU_BACKGROUND_TILE;
            }
            y += MENU_BACKGROUND_TILE;
        }
    }
}
impl Clone for PauseMenuScreen {
    fn clone(&self) -> Self {
        let mut screen = Self::new(
            self.show_pause_menu,
            self.singleplayer_server,
            self.local_server,
            Arc::clone(&self.font),
        );
        screen.has_reportable_players = self.has_reportable_players;
        screen.has_draft_report = self.has_draft_report;
        screen.friend_list_enabled = self.friend_list_enabled;
        screen.set_server_links_available(self.server_links_available);
        screen.custom_addition_title = self.custom_addition_title.clone();
        screen.set_extensions(self.extensions.0, self.extensions.1);
        screen
    }
}
impl std::fmt::Debug for PauseMenuScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PauseMenuScreen")
            .field("show_pause_menu", &self.show_pause_menu)
            .field("singleplayer_server", &self.singleplayer_server)
            .field("local_server", &self.local_server)
            .field("has_reportable_players", &self.has_reportable_players)
            .field("has_draft_report", &self.has_draft_report)
            .field("friend_list_enabled", &self.friend_list_enabled)
            .field("server_links_available", &self.server_links_available)
            .field("extensions", &self.extensions)
            .finish()
    }
}
impl Screen for PauseMenuScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        self.base.clear_widgets();
        let width = self.base.width;
        let height = self.base.height;
        if width == 0 || height == 0 {
            return;
        }
        self.buttons = self
            .layout_for_size(width, height)
            .buttons
            .into_iter()
            .map(|mut button| {
                if button.kind == PauseButtonKind::PlayerReporting
                    && self.singleplayer_server
                    && !self.has_reportable_players
                {
                    button.active = false;
                    button.tooltip = Some(translated("menu.playerReporting.no_players"));
                }
                if button.kind == PauseButtonKind::Friends && !self.friend_list_enabled {
                    button.action = Some(PauseMenuAction::ConfirmFriendsList);
                }
                button
            })
            .collect();
        self.build_sprite_icon_buttons();
        for button in &self.buttons {
            let widget = PauseButtonWidget {
                x: button.x,
                y: button.y,
                width: button.width,
                height: button.height,
                label: button.label.clone(),
                action: button.action.clone(),
                active: button.active,
                focused: false,
                action_sink: Rc::clone(&self.action_sink),
            };
            self.base.add_widget(widget);
        }
    }
    fn reposition_elements(&mut self) {
        self.init();
    }
    fn on_close(&mut self) {
        *self.action_sink.borrow_mut() = Some(PauseMenuAction::ReturnToGame);
        self.base.request_close();
    }
    fn should_close_on_esc(&self) -> bool {
        true
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.base.mouse_clicked(event, double_click)
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        self.base.mouse_released(event)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        screen::screen_key_pressed(self, event)
    }
    fn char_typed(&mut self, event: crate::input::events::CharacterEvent) -> bool {
        self.base.dispatch_focused_char_typed(event)
    }
    fn tick(&mut self) {
        self.base.tick();
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        if self.base.width != width || self.base.height != height || self.buttons.is_empty() {
            self.base.width = width;
            self.base.height = height;
            self.init();
        }
        self.extract_tiled_background(graphics);
        if self.show_pause_menu {
            let mouse_x = graphics.mouse_x();
            let mouse_y = graphics.mouse_y();
            let focused_idx = self.base.focused_index();
            for (idx, button) in self.buttons.iter().enumerate() {
                let focused = focused_idx == Some(idx)
                    || (self.extension_focus.is_some()
                        && button.action
                            == self
                                .extension_focus
                                .map(PauseMenuAction::OpenExtensionCatalog));
                match button.kind {
                    PauseButtonKind::ReportBugs
                    | PauseButtonKind::Feedback
                    | PauseButtonKind::Friends
                    | PauseButtonKind::PlayerReporting => {
                        if let Some((_, widget)) = self
                            .sprite_icon_buttons
                            .iter_mut()
                            .find(|(kind, _)| *kind == button.kind)
                        {
                            widget.base.set_focused(focused);
                            widget.extract_render_state(graphics, mouse_x, mouse_y, 0.0);
                        }
                    }
                    _ => {
                        button.extract_render_state(
                            graphics,
                            self.font.as_ref(),
                            mouse_x,
                            mouse_y,
                            focused,
                        );
                    }
                }
            }
            if self.has_draft_report {
                if let Some(disconnect) = self
                    .buttons
                    .iter()
                    .find(|button| button.kind == PauseButtonKind::Disconnect)
                {
                    graphics.blit(
                        ResourceLocation::from_string("minecraft:icon/draft_report"),
                        disconnect.x + disconnect.width - 17,
                        disconnect.y + 3,
                        ICON_SIZE,
                        ICON_SIZE,
                    );
                }
            }
        }
        let title_width = self.font.measure_width(&self.title, 1.0).round() as i32;
        graphics.text(
            default_font(),
            self.title.clone(),
            (width - title_width) / 2,
            if self.show_pause_menu { 40 } else { 10 },
            0xFFFF_FFFFu32 as i32,
        );
        let mouse_x = graphics.mouse_x();
        let mouse_y = graphics.mouse_y();
        if let Some(button) = self
            .buttons
            .iter()
            .find(|button| button.contains(mouse_x, mouse_y))
        {
            let component_owned = matches!(
                button.kind,
                PauseButtonKind::ReportBugs
                    | PauseButtonKind::Feedback
                    | PauseButtonKind::Friends
                    | PauseButtonKind::PlayerReporting
            );
            if !component_owned {
                if let Some(tooltip) = button.tooltip_text() {
                    graphics.draw_tooltip(
                        self.font.as_ref(),
                        &[tooltip.to_string()],
                        mouse_x,
                        mouse_y,
                    );
                }
            }
        }
    }
}
struct PauseButtonWidget {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    label: String,
    action: Option<PauseMenuAction>,
    active: bool,
    focused: bool,
    action_sink: Rc<RefCell<Option<PauseMenuAction>>>,
}
impl Renderable for PauseButtonWidget {
    fn extract_render_state(
        &mut self,
        _graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
    }
}
impl TabOrderedElement for PauseButtonWidget {
    fn tab_order_group(&self) -> i32 {
        0
    }
}
impl LayoutElement for PauseButtonWidget {
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
impl NarratableEntry for PauseButtonWidget {
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        output.add(NarratedElementType::Title, &self.label);
    }
}
impl GuiEventListener for PauseButtonWidget {
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
            if let Some(action) = self.action.clone() {
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
            click_sound_sink::record(self.play_down_sound());
            if let Some(action) = self.action.clone() {
                *self.action_sink.borrow_mut() = Some(action);
            }
            true
        } else {
            false
        }
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        Some(WidgetClickSound::UiButtonClick)
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PauseMenuLayout {
    pub buttons: Vec<PauseButton>,
    pub grid_x: i32,
    pub grid_y: i32,
    pub grid_width: i32,
    pub grid_height: i32,
}
impl PauseMenuLayout {
    pub fn calculate(
        width: i32,
        height: i32,
        singleplayer_server: bool,
        local_server: bool,
    ) -> Self {
        Self::calculate_with_server_links(width, height, singleplayer_server, local_server, false)
    }
    pub fn calculate_with_server_links(
        width: i32,
        height: i32,
        singleplayer_server: bool,
        local_server: bool,
        server_links_available: bool,
    ) -> Self {
        let grid_width = BUTTON_WIDTH_FULL + BUTTON_PADDING * COLUMNS;
        let grid_height =
            MENU_PADDING_TOP + BUTTON_HEIGHT + 24 * 4 + if server_links_available { 24 } else { 0 };
        let grid_x = ((width - grid_width) as f32 * 0.5) as i32;
        let grid_y = ((height - grid_height) as f32 * 0.25) as i32;
        let full_x = grid_x + BUTTON_PADDING;
        let left_x = grid_x + BUTTON_PADDING;
        let right_x = grid_x + BUTTON_PADDING + (BUTTON_WIDTH_HALF + BUTTON_PADDING * 2);
        let mut y = grid_y;
        let mut buttons = Vec::new();
        buttons.push(PauseButton::text(
            PauseButtonKind::ReturnToGame,
            full_x,
            y + MENU_PADDING_TOP,
            BUTTON_WIDTH_FULL,
            "menu.returnToGame",
            Some(PauseMenuAction::ReturnToGame),
        ));
        y += MENU_PADDING_TOP + BUTTON_HEIGHT;
        buttons.push(PauseButton::text(
            PauseButtonKind::Advancements,
            left_x,
            y + BUTTON_PADDING,
            BUTTON_WIDTH_HALF,
            "gui.advancements",
            Some(PauseMenuAction::OpenSubmenu(PauseSubmenu::Advancements)),
        ));
        buttons.push(PauseButton::text(
            PauseButtonKind::Stats,
            right_x,
            y + BUTTON_PADDING,
            BUTTON_WIDTH_HALF,
            "gui.stats",
            Some(PauseMenuAction::OpenSubmenu(PauseSubmenu::Stats)),
        ));
        y += BUTTON_PADDING + BUTTON_HEIGHT;
        let icon_row_width = ICON_BUTTON_WIDTH * 4 + BUTTON_PADDING * 3;
        let icon_y = y + BUTTON_PADDING;
        let mut icon_x = grid_x + (grid_width - icon_row_width) / 2;
        buttons.push(PauseButton::icon(
            PauseButtonKind::ReportBugs,
            icon_x,
            icon_y,
            "menu.reportBugs",
            "minecraft:pause_menu/bug",
            Some(PauseMenuAction::OpenSubmenu(PauseSubmenu::ReportBugs)),
        ));
        icon_x += ICON_BUTTON_WIDTH + BUTTON_PADDING;
        buttons.push(PauseButton::icon(
            PauseButtonKind::Feedback,
            icon_x,
            icon_y,
            "menu.sendFeedback",
            "minecraft:pause_menu/social_interactions",
            Some(PauseMenuAction::OpenSubmenu(PauseSubmenu::Feedback)),
        ));
        icon_x += ICON_BUTTON_WIDTH + BUTTON_PADDING;
        buttons.push(PauseButton::icon(
            PauseButtonKind::Friends,
            icon_x,
            icon_y,
            "gui.friends.open",
            "minecraft:friends/friends",
            Some(PauseMenuAction::OpenSubmenu(PauseSubmenu::Friends)),
        ));
        icon_x += ICON_BUTTON_WIDTH + BUTTON_PADDING;
        buttons.push(PauseButton::icon(
            PauseButtonKind::PlayerReporting,
            icon_x,
            icon_y,
            "menu.playerReporting",
            "minecraft:pause_menu/player_reporting",
            Some(PauseMenuAction::OpenSubmenu(PauseSubmenu::PlayerReporting)),
        ));
        y += BUTTON_PADDING + BUTTON_HEIGHT;
        if server_links_available {
            buttons.push(PauseButton::text(
                PauseButtonKind::ServerLinks,
                full_x,
                y + BUTTON_PADDING,
                BUTTON_WIDTH_FULL,
                "menu.server_links",
                Some(PauseMenuAction::OpenSubmenu(PauseSubmenu::ServerLinks)),
            ));
            y += BUTTON_PADDING + BUTTON_HEIGHT;
        }
        if singleplayer_server {
            buttons.push(PauseButton::text(
                PauseButtonKind::Options,
                left_x,
                y + BUTTON_PADDING,
                BUTTON_WIDTH_HALF,
                "menu.options",
                Some(PauseMenuAction::OpenSubmenu(PauseSubmenu::Options)),
            ));
            buttons.push(PauseButton::text(
                PauseButtonKind::MultiplayerOptions,
                right_x,
                y + BUTTON_PADDING,
                BUTTON_WIDTH_HALF,
                "menu.multiplayerOptions.button",
                Some(PauseMenuAction::OpenSubmenu(
                    PauseSubmenu::MultiplayerOptions,
                )),
            ));
            y += BUTTON_PADDING + BUTTON_HEIGHT;
        } else {
            buttons.push(PauseButton::text(
                PauseButtonKind::Options,
                full_x,
                y + BUTTON_PADDING,
                BUTTON_WIDTH_FULL,
                "menu.options",
                Some(PauseMenuAction::OpenSubmenu(PauseSubmenu::Options)),
            ));
            y += BUTTON_PADDING + BUTTON_HEIGHT;
        }
        buttons.push(PauseButton::text(
            PauseButtonKind::Disconnect,
            full_x,
            y + BUTTON_PADDING,
            BUTTON_WIDTH_FULL,
            if local_server {
                "menu.returnToMenu"
            } else {
                "menu.disconnect"
            },
            Some(PauseMenuAction::Disconnect),
        ));
        Self {
            buttons,
            grid_x,
            grid_y,
            grid_width,
            grid_height,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PauseButton {
    pub kind: PauseButtonKind,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub label: String,
    pub tooltip: Option<String>,
    pub icon: Option<ResourceLocation>,
    pub active: bool,
    pub action: Option<PauseMenuAction>,
}
impl PauseButton {
    fn text(
        kind: PauseButtonKind,
        x: i32,
        y: i32,
        width: i32,
        label_key: &str,
        action: Option<PauseMenuAction>,
    ) -> Self {
        Self {
            kind,
            x,
            y,
            width,
            height: BUTTON_HEIGHT,
            label: translated(label_key),
            tooltip: None,
            icon: None,
            active: true,
            action,
        }
    }
    fn icon(
        kind: PauseButtonKind,
        x: i32,
        y: i32,
        label_key: &str,
        icon: &str,
        action: Option<PauseMenuAction>,
    ) -> Self {
        Self {
            kind,
            x,
            y,
            width: ICON_BUTTON_WIDTH,
            height: BUTTON_HEIGHT,
            label: translated(label_key),
            tooltip: Some(translated(label_key)),
            icon: Some(ResourceLocation::from_string(icon)),
            active: true,
            action,
        }
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
    pub fn click_sound(&self) -> Option<WidgetClickSound> {
        Some(WidgetClickSound::UiButtonClick)
    }
    fn tooltip_text(&self) -> Option<&str> {
        self.tooltip.as_deref()
    }
    fn extract_render_state(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
        mouse_x: i32,
        mouse_y: i32,
        focused: bool,
    ) {
        let hovered = self.active && (self.contains(mouse_x, mouse_y) || focused);
        let button_sprite = if !self.active {
            "minecraft:widget/button_disabled"
        } else if hovered {
            "minecraft:widget/button_highlighted"
        } else {
            "minecraft:widget/button"
        };
        graphics.blit(
            ResourceLocation::from_string(button_sprite),
            self.x,
            self.y,
            self.width,
            self.height,
        );
        if let Some(icon) = &self.icon {
            let icon_x = self.x + self.width / 2 - ICON_SIZE / 2;
            let icon_y = self.y + self.height / 2 - ICON_SIZE / 2;
            graphics.blit(icon.clone(), icon_x, icon_y, ICON_SIZE, ICON_SIZE);
            return;
        }
        let color = if self.active {
            0xFFFF_FFFFu32 as i32
        } else {
            0xFFA0_A0A0u32 as i32
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseButtonKind {
    ReturnToGame,
    Advancements,
    Stats,
    ReportBugs,
    Feedback,
    Friends,
    PlayerReporting,
    Options,
    MultiplayerOptions,
    Disconnect,
    Extension,
    ServerLinks,
}
fn translated(key: &str) -> String {
    crate::translate(key)
}
fn default_font() -> ResourceLocation {
    ResourceLocation::from_string("minecraft:default")
}
