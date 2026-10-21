use crate::FontWidthMeasurer;
use crate::components::scroll_area::{AbstractScrollArea, SCROLLBAR_WIDTH, ScrollbarSettings};
use crate::components::{
    Button, GuiEventListener, Renderable, TabOrderedElement, WidgetClickSound,
};
use crate::framework::graphics::{GuiGraphicsExtractor, WidgetTooltipPosition};
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{NarratableEntry, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
pub const HEADER_HEIGHT: i32 = 33;
pub const FOOTER_HEIGHT: i32 = 33;
pub const ROW_HEIGHT: i32 = 20;
pub const ENTRY_HEIGHT: i32 = 25;
pub const BIG_BUTTON_WIDTH: i32 = 310;
pub const SMALL_BUTTON_WIDTH: i32 = 150;
pub const SMALL_BUTTON_STEP: i32 = 160;
pub const LIST_TOP_INSET: i32 = 2;
pub const FOOTER_BUTTON_WIDTH: i32 = 200;
pub const FOOTER_BUTTON_HEIGHT: i32 = 20;
pub const FOOTER_BUTTON_Y_OFFSET: i32 = 26;
pub const SCROLL_RATE: i32 = ENTRY_HEIGHT / 2;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OnlineOption {
    FriendsList,
    AllowFriendRequests,
    InGameNotifications,
    Presence,
    PrivacySettings,
    AllowServerListing,
    RealmsNotifications,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ButtonRequest {
    ToggleServerListing,
    PrivacySettingsLink,
    Done,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OnlineOptionsAction {
    Done,
    AllowServerListingChanged(bool),
    PrivacySettingsLinkClicked,
}
#[derive(Clone, Copy, Debug)]
struct HeaderPlacement {
    key: &'static str,
    fallback: &'static str,
    y: i32,
}
#[derive(Clone, Copy, Debug)]
struct ButtonPlacement {
    option: OnlineOption,
    content_y: i32,
    column: i32,
    width: i32,
    enabled: bool,
    key: &'static str,
    fallback: &'static str,
    request: Option<ButtonRequest>,
}
struct ButtonHandle {
    button: Rc<RefCell<Button<ButtonRequest>>>,
}
impl ButtonHandle {
    fn new(button: Rc<RefCell<Button<ButtonRequest>>>) -> Self {
        Self { button }
    }
}
impl Renderable for ButtonHandle {
    fn extract_render_state(
        &mut self,
        _graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
    }
}
impl TabOrderedElement for ButtonHandle {
    fn tab_order_group(&self) -> i32 {
        self.button.borrow().tab_order_group()
    }
}
impl GuiEventListener for ButtonHandle {
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.button.borrow().is_mouse_over(mouse_x, mouse_y)
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.button.borrow_mut().mouse_clicked(event, double_click)
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        self.button.borrow_mut().mouse_released(event)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        self.button.borrow_mut().key_pressed(event)
    }
    fn set_focused(&mut self, focused: bool) {
        self.button.borrow_mut().set_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.button.borrow().is_focused()
    }
    fn rectangle(&self) -> ScreenRectangle {
        GuiEventListener::rectangle(&*self.button.borrow())
    }
    fn next_focus_path(&self, event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.button.borrow().next_focus_path(event)
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.button.borrow().play_down_sound()
    }
}
impl LayoutElement for ButtonHandle {
    fn set_x(&mut self, x: i32) {
        self.button.borrow_mut().set_x(x);
    }
    fn set_y(&mut self, y: i32) {
        self.button.borrow_mut().set_y(y);
    }
    fn x(&self) -> i32 {
        self.button.borrow().x()
    }
    fn y(&self) -> i32 {
        self.button.borrow().y()
    }
    fn width(&self) -> i32 {
        self.button.borrow().width()
    }
    fn height(&self) -> i32 {
        self.button.borrow().height()
    }
}
impl NarratableEntry for ButtonHandle {
    fn narration_priority(&self) -> NarrationPriority {
        self.button.borrow().narration_priority()
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        self.button.borrow().update_narration(output);
    }
}
pub struct OnlineOptionsScreen {
    base: ScreenBase,
    buttons: Vec<(OnlineOption, Rc<RefCell<Button<ButtonRequest>>>)>,
    done_button: Rc<RefCell<Button<ButtonRequest>>>,
    placements: Vec<ButtonPlacement>,
    headers: Vec<HeaderPlacement>,
    area: AbstractScrollArea,
    font: Arc<dyn FontWidthMeasurer>,
    in_world: bool,
    allow_server_listing: bool,
    pending_action: Option<OnlineOptionsAction>,
    last_width: i32,
    last_height: i32,
}
impl OnlineOptionsScreen {
    pub fn new(
        in_world: bool,
        allow_server_listing: bool,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let placements = Self::button_placements(allow_server_listing);
        let buttons = placements
            .iter()
            .map(|placement| {
                let label = Self::button_label(placement, allow_server_listing);
                let mut button = Button::<ButtonRequest>::builder(Component::literal(&label))
                    .bounds(0, 0, placement.width, ROW_HEIGHT)
                    .with_font(Arc::clone(&font));
                if let Some(request) = placement.request {
                    button = button.with_action(request);
                }
                let mut button = button.build();
                button.base.active = placement.enabled;
                (placement.option, Rc::new(RefCell::new(button)))
            })
            .collect();
        let done_button = Rc::new(RefCell::new(
            Button::<ButtonRequest>::builder(Component::literal(&translated_or(
                "gui.done", "Done",
            )))
            .bounds(0, 0, FOOTER_BUTTON_WIDTH, FOOTER_BUTTON_HEIGHT)
            .with_action(ButtonRequest::Done)
            .with_font(Arc::clone(&font))
            .build(),
        ));
        let mut area = AbstractScrollArea::new(
            0,
            HEADER_HEIGHT,
            0,
            0,
            Component::literal(""),
            ScrollbarSettings::default_settings(SCROLL_RATE),
        );
        area.content_height = 200;
        Self {
            base: ScreenBase::new(Component::literal(&translated_or(
                "options.online.title",
                "Online Options",
            ))),
            buttons,
            done_button,
            placements,
            headers: Self::header_placements(),
            area,
            font,
            in_world,
            allow_server_listing,
            pending_action: None,
            last_width: 0,
            last_height: 0,
        }
    }
    fn header_placements() -> Vec<HeaderPlacement> {
        let mut y = LIST_TOP_INSET;
        let mut headers = Vec::with_capacity(3);
        headers.push(HeaderPlacement {
            key: "options.online.friends.header",
            fallback: "Friends List",
            y,
        });
        y += 13 + ENTRY_HEIGHT * 2 + ENTRY_HEIGHT;
        headers.push(HeaderPlacement {
            key: "options.online.servers.header",
            fallback: "Servers",
            y: y + 18,
        });
        y += 31 + ENTRY_HEIGHT;
        headers.push(HeaderPlacement {
            key: "options.online.realms.header",
            fallback: "Realms",
            y: y + 18,
        });
        headers
    }
    fn button_placements(_allow_server_listing: bool) -> Vec<ButtonPlacement> {
        let mut y = LIST_TOP_INSET + 13;
        let mut placements = Vec::with_capacity(7);
        macro_rules! push {
            (
                $option:expr, $key:expr, $fallback:expr, $column:expr, $width:expr,
                $enabled:expr, $request:expr $(,)?
            ) => {
                placements.push(ButtonPlacement {
                    option: $option,
                    content_y: y,
                    column: $column,
                    width: $width,
                    enabled: $enabled,
                    key: $key,
                    fallback: $fallback,
                    request: $request,
                });
            };
        }
        push!(
            OnlineOption::FriendsList,
            "options.friendsList",
            "Friends",
            0,
            SMALL_BUTTON_WIDTH,
            false,
            None,
        );
        push!(
            OnlineOption::AllowFriendRequests,
            "options.allowFriendRequests",
            "Requests",
            SMALL_BUTTON_STEP,
            SMALL_BUTTON_WIDTH,
            false,
            None,
        );
        y += ENTRY_HEIGHT;
        push!(
            OnlineOption::InGameNotifications,
            "options.inGameNotification",
            "In-game",
            0,
            SMALL_BUTTON_WIDTH,
            false,
            None,
        );
        push!(
            OnlineOption::Presence,
            "options.sharePresence",
            "Presence",
            SMALL_BUTTON_STEP,
            SMALL_BUTTON_WIDTH,
            false,
            None,
        );
        y += ENTRY_HEIGHT;
        push!(
            OnlineOption::PrivacySettings,
            "options.online.xboxSettings",
            "Xbox Settings...",
            0,
            BIG_BUTTON_WIDTH,
            true,
            Some(ButtonRequest::PrivacySettingsLink),
        );
        y += ENTRY_HEIGHT + 31;
        push!(
            OnlineOption::AllowServerListing,
            "options.allowServerListing",
            "Allow Server Listings",
            0,
            BIG_BUTTON_WIDTH,
            true,
            Some(ButtonRequest::ToggleServerListing),
        );
        y += ENTRY_HEIGHT + 31;
        push!(
            OnlineOption::RealmsNotifications,
            "options.realmsNotifications.button",
            "News & Invites",
            0,
            BIG_BUTTON_WIDTH,
            false,
            None,
        );
        placements
    }
    fn button_label(placement: &ButtonPlacement, allow_server_listing: bool) -> String {
        let name = translated_or(placement.key, placement.fallback);
        match placement.option {
            OnlineOption::FriendsList => "Friends: Unavailable".to_string(),
            OnlineOption::AllowFriendRequests => "Requests: Unavailable".to_string(),
            OnlineOption::InGameNotifications => "In-game: Unavailable".to_string(),
            OnlineOption::Presence => "Presence: Unavailable".to_string(),
            OnlineOption::RealmsNotifications => {
                format!(
                    "{}: {}",
                    name,
                    translated_or("eltharn.settings.option_unavailable", "Unavailable")
                )
            }
            OnlineOption::AllowServerListing => {
                let state_key = if allow_server_listing {
                    "options.on.composed"
                } else {
                    "options.off.composed"
                };
                let translated = crate::translate(state_key);
                if translated == state_key {
                    format!(
                        "{}: {}",
                        name,
                        translated_or(
                            if allow_server_listing {
                                "options.on"
                            } else {
                                "options.off"
                            },
                            if allow_server_listing { "ON" } else { "OFF" },
                        )
                    )
                } else {
                    translated.replace("%s", &name)
                }
            }
            OnlineOption::PrivacySettings => name,
        }
    }
    fn ensure_layout(&mut self, width: i32, height: i32) {
        if self.last_width == width && self.last_height == height && !self.buttons.is_empty() {
            return;
        }
        self.last_width = width;
        self.last_height = height;
        self.base.clear_widgets();
        let viewport_height = (height - HEADER_HEIGHT - FOOTER_HEIGHT).max(0);
        let list_left = width / 2 - BIG_BUTTON_WIDTH / 2;
        let scroll_bar_x = list_left + BIG_BUTTON_WIDTH + 8;
        self.area.base.set_x(scroll_bar_x);
        self.area.base.set_y(HEADER_HEIGHT);
        self.area.base.set_width(SCROLLBAR_WIDTH);
        self.area.base.set_height(viewport_height);
        self.area.content_height = 200;
        self.area.clamp_scroll(viewport_height);
        self.apply_scroll_positions();
        for (_, button) in &self.buttons {
            self.base.add_widget(ButtonHandle::new(Rc::clone(button)));
        }
        {
            let mut done = self.done_button.borrow_mut();
            done.set_position(
                width / 2 - FOOTER_BUTTON_WIDTH / 2,
                height - FOOTER_BUTTON_Y_OFFSET,
            );
        }
        self.base
            .add_widget(ButtonHandle::new(Rc::clone(&self.done_button)));
    }
    fn apply_scroll_positions(&mut self) {
        let width = self.base.width;
        let top = HEADER_HEIGHT;
        let bottom = self.base.height - FOOTER_HEIGHT;
        let offset = self.area.scroll_amount as i32;
        let list_left = width / 2 - BIG_BUTTON_WIDTH / 2;
        for (index, placement) in self.placements.iter().enumerate() {
            let Some((option, button)) = self.buttons.get(index) else {
                continue;
            };
            debug_assert_eq!(*option, placement.option);
            let x = list_left + placement.column;
            let y = top + placement.content_y - offset;
            let rect_bottom = y + ROW_HEIGHT;
            let fully_visible = y >= top && rect_bottom <= bottom;
            let mut button = button.borrow_mut();
            button.set_position(x, y);
            button.base.set_width(placement.width);
            button.base.active = placement.enabled && fully_visible;
            if placement.option == OnlineOption::AllowServerListing {
                button
                    .base
                    .set_message(Component::literal(&Self::button_label(
                        placement,
                        self.allow_server_listing,
                    )));
            }
        }
    }
    fn handle_request(&mut self, request: ButtonRequest) {
        match request {
            ButtonRequest::ToggleServerListing => {
                self.allow_server_listing = !self.allow_server_listing;
                self.pending_action = Some(OnlineOptionsAction::AllowServerListingChanged(
                    self.allow_server_listing,
                ));
                self.apply_scroll_positions();
            }
            ButtonRequest::PrivacySettingsLink => {
                self.pending_action = Some(OnlineOptionsAction::PrivacySettingsLinkClicked);
            }
            ButtonRequest::Done => {
                self.pending_action = Some(OnlineOptionsAction::Done);
            }
        }
    }
    fn drain_button_actions(&mut self) {
        let requests: Vec<_> = self
            .buttons
            .iter()
            .filter_map(|(_, button)| button.borrow_mut().take_press_action())
            .collect();
        for request in requests {
            self.handle_request(request);
        }
        let done_request = self.done_button.borrow_mut().take_press_action();
        if let Some(request) = done_request {
            self.handle_request(request);
        }
    }
    fn viewport_height(&self) -> i32 {
        (self.base_ref().height - HEADER_HEIGHT - FOOTER_HEIGHT).max(0)
    }
    pub fn allow_server_listing(&self) -> bool {
        self.allow_server_listing
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn set_allow_server_listing(&mut self, enabled: bool) {
        self.allow_server_listing = enabled;
        self.apply_scroll_positions();
    }
    pub fn option_is_available(&self, option: OnlineOption) -> bool {
        matches!(
            option,
            OnlineOption::PrivacySettings | OnlineOption::AllowServerListing
        )
    }
    pub fn option_label(&self, option: OnlineOption) -> Option<String> {
        self.placements
            .iter()
            .find(|placement| placement.option == option)
            .map(|placement| Self::button_label(placement, self.allow_server_listing))
    }
    pub fn option_rectangle(&self, option: OnlineOption) -> Option<ScreenRectangle> {
        self.buttons
            .iter()
            .find(|(candidate, _)| *candidate == option)
            .map(|(_, button)| LayoutElement::rectangle(&*button.borrow()))
    }
    pub fn take_action(&mut self) -> Option<OnlineOptionsAction> {
        self.pending_action.take()
    }
}
fn translated_or(key: &str, fallback: &str) -> String {
    let translated = crate::translate(key);
    if translated == key {
        fallback.to_string()
    } else {
        translated
    }
}
impl Screen for OnlineOptionsScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn should_close_on_esc(&self) -> bool {
        true
    }
    fn init(&mut self) {
        self.last_width = 0;
        self.last_height = 0;
        self.ensure_layout(self.base.width, self.base.height);
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if event.is_escape() {
            self.pending_action = Some(OnlineOptionsAction::Done);
            return true;
        }
        let handled = crate::framework::screen::screen_key_pressed(self, event);
        if handled {
            self.drain_button_actions();
        }
        handled
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        let height = self.base_ref().height;
        let viewport_bottom = height - FOOTER_HEIGHT;
        let inside_list = event.y >= HEADER_HEIGHT as f64 && event.y < viewport_bottom as f64;
        let done_rect = LayoutElement::rectangle(&*self.done_button.borrow());
        let on_done = done_rect.contains_point(event.x as i32, event.y as i32);
        let handled = if inside_list || on_done {
            crate::framework::screen::screen_mouse_clicked(self, event, double_click)
        } else {
            false
        };
        if handled {
            self.drain_button_actions();
            return true;
        }
        let started_scroll = self.area.scrollbar_clicked_with_button(
            event.x,
            event.y,
            event.button(),
            self.viewport_height(),
        );
        if started_scroll {
            self.area.base.set_focused(false);
        }
        started_scroll
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        let was_scrolling = self.area.is_scrolling();
        self.area.scrollbar_released();
        crate::framework::screen::screen_mouse_released(self, event) || was_scrolling
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        if self.area.is_scrolling() {
            self.area
                .scrollbar_dragged(event.y, dy, self.viewport_height());
            self.apply_scroll_positions();
            return true;
        }
        crate::framework::screen::screen_mouse_dragged(self, event, dx, dy)
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, _scroll_x: f64, scroll_y: f64) -> bool {
        let width = self.base_ref().width;
        let left = width / 2 - BIG_BUTTON_WIDTH / 2;
        let viewport = ScreenRectangle::new(
            left,
            HEADER_HEIGHT,
            BIG_BUTTON_WIDTH,
            self.viewport_height(),
        );
        if !viewport.contains_point(x as i32, y as i32) {
            return false;
        }
        self.area.scroll(scroll_y as f32, self.viewport_height());
        self.apply_scroll_positions();
        true
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        self.ensure_layout(width, height);
        self.apply_scroll_positions();
        self.base
            .extract_background_backdrop(graphics, self.in_world);
        let list_top = HEADER_HEIGHT;
        let list_bottom = (height - FOOTER_HEIGHT).max(list_top);
        let background = ResourceLocation::from_string(if self.in_world {
            "minecraft:gui/inworld_menu_list_background"
        } else {
            "minecraft:gui/menu_list_background"
        });
        let mut y = list_top;
        while y < list_bottom {
            let tile_height = 32.min(list_bottom - y);
            let mut x = 0;
            while x < width {
                let tile_width = 32.min(width - x);
                graphics.blit_region(
                    background.clone(),
                    x,
                    y,
                    tile_width,
                    tile_height,
                    0.0,
                    (y as f32 + self.area.scroll_amount) / 32.0,
                    tile_width as f32 / 32.0,
                    tile_height as f32 / 32.0,
                    0xFFFF_FFFFu32 as i32,
                );
                x += 32;
            }
            y += tile_height;
        }
        graphics.enable_scissor(0, list_top, width, list_bottom);
        for header in &self.headers {
            let text = translated_or(header.key, header.fallback);
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                text,
                width / 2 - BIG_BUTTON_WIDTH / 2,
                list_top + header.y - self.area.scroll_amount as i32,
                0xFFFF_FFFFu32 as i32,
            );
        }
        for (_, button) in &self.buttons {
            let mut button = button.borrow_mut();
            let rect = LayoutElement::rectangle(&*button);
            if rect.bottom() > list_top && rect.top() < list_bottom {
                button.extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
            }
        }
        graphics.disable_scissor();
        graphics.draw_menu_background(self.in_world, width, HEADER_HEIGHT);
        graphics.pose_mut().push_pose();
        graphics
            .pose_mut()
            .translate(0.0, (height - FOOTER_HEIGHT) as f32);
        graphics.draw_menu_background(self.in_world, width, FOOTER_HEIGHT);
        graphics.pose_mut().pop_pose();
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/header_separator"),
            0,
            HEADER_HEIGHT - 2,
            width,
            2,
        );
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/footer_separator"),
            0,
            height - FOOTER_HEIGHT,
            width,
            2,
        );
        let title = translated_or("options.online.title", "Online Options");
        let title_width = self.font.measure_width(&title, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            title,
            (width - title_width) / 2,
            12,
            0xFFFF_FFFFu32 as i32,
        );
        self.area.extract_scrollbar(graphics);
        self.done_button.borrow_mut().extract_render_state(
            graphics,
            mouse_x,
            mouse_y,
            partial_tick,
        );
        if mouse_y >= list_top && mouse_y < list_bottom {
            for (index, placement) in self.placements.iter().enumerate() {
                let Some((_, button)) = self.buttons.get(index) else {
                    continue;
                };
                let rect = LayoutElement::rectangle(&*button.borrow());
                if rect.contains_point(mouse_x, mouse_y)
                    && let Some(lines) =
                        crate::settings::option_tooltips::online_tooltip(&placement.key)
                {
                    graphics.draw_widget_tooltip(
                        self.font.as_ref(),
                        &lines,
                        mouse_x,
                        mouse_y,
                        rect,
                        WidgetTooltipPosition::Menu,
                    );
                    break;
                }
            }
        }
    }
}
