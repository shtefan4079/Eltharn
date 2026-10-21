use crate::FontWidthMeasurer;
use crate::components::scroll_area::{
    INWORLD_MENU_LIST_BACKGROUND, MENU_LIST_BACKGROUND, ScrollableLayout,
};
use crate::components::text_widget::{LINE_HEIGHT, MultiLineTextWidget, StringWidget};
use crate::components::{
    Button, DEFAULT_HEIGHT, GuiEventListener, Renderable, TabOrderedElement, WidgetClickSound,
};
use crate::framework::focus::resolve_tab;
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::{
    FrameLayout, HeaderAndFooterLayout, Layout, LayoutElement, LinearLayout,
};
use crate::framework::narration::{
    NarratableEntry, NarratedElementType, NarrationElementOutput, NarrationPriority,
};
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::framework::screen::{
    Screen, ScreenBase, screen_init, screen_key_pressed, screen_mouse_clicked,
    screen_mouse_dragged, screen_mouse_released,
};
use crate::input::events::{KeyEvent, MouseButtonEvent, MouseButtonInfo};
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
pub const TITLE_KEY: &str = "restrictions_screen.title";
pub const DONE_KEY: &str = "gui.done";
pub const TEXT_BOX_WIDTH: i32 = 250;
pub const BODY_SPACING: i32 = 0;
pub const FIRST_ROW_TOP: i32 = 4;
pub const ACTION_BUTTON_WIDTH: i32 = 200;
pub const DONE_BUTTON_WIDTH: i32 = 200;
pub const SPACER_HEIGHT: i32 = 10;
pub const PERMISSION_SPACING: i32 = 4;
pub const RESTRICTION_COLOR: i32 = 0xFFFF_5555u32 as i32;
pub const DENIED_COLOR: i32 = 0xFFFF_5555u32 as i32;
pub const ALLOWED_COLOR: i32 = 0xFF55_FF55u32 as i32;
pub const BAND_HEIGHT: i32 = 33;
pub const SEPARATOR_HEIGHT: i32 = 2;
pub const PERMISSION_KEYS: [&str; 4] = [
    "send_messages",
    "send_commands",
    "receive_system_messages",
    "receive_player_messages",
];
pub const PERMISSION_TEXT_SEPARATOR: &str = "\n";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuiChatRestrictionAction {
    GoToAccount,
    GoToChatSettings,
}
impl GuiChatRestrictionAction {
    pub fn display_string(self) -> String {
        match self {
            Self::GoToAccount => crate::translate("restrictions_screen.action.open_account"),
            Self::GoToChatSettings => crate::translate("restrictions_screen.action.chat_settings"),
        }
    }
}
#[derive(Clone, Debug)]
pub struct GuiChatRestriction {
    pub translation_key: String,
    pub action: Option<GuiChatRestrictionAction>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestrictionsAction {
    Done,
    Resolve(GuiChatRestrictionAction),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowKind {
    RestrictionText { restriction: usize, color: i32 },
    ActionButton { restriction: usize },
    PermissionText { permission: usize },
}
pub struct RestrictionsScreen {
    base: ScreenBase,
    pub in_world: bool,
    pub restrictions: Vec<GuiChatRestriction>,
    pub can_send_messages: bool,
    pub can_send_commands: bool,
    pub can_receive_system: bool,
    pub can_receive_player: bool,
    font: Arc<dyn FontWidthMeasurer>,
    body_scroll: ScrollableLayout,
    layout: HeaderAndFooterLayout,
    rows: Vec<Row>,
    action_buttons: Vec<Option<Rc<RefCell<Button<RestrictionsAction>>>>>,
    done: Rc<RefCell<Button<RestrictionsAction>>>,
    title_handle: usize,
    done_handle: usize,
    last_size: (i32, i32),
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Row {
    handle: usize,
    kind: RowKind,
    width: i32,
    height: i32,
    top: i32,
}
impl RestrictionsScreen {
    pub fn new(
        in_world: bool,
        restrictions: Vec<GuiChatRestriction>,
        can_send_messages: bool,
        can_send_commands: bool,
        can_receive_system: bool,
        can_receive_player: bool,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let action_buttons = restrictions
            .iter()
            .map(|restriction| {
                restriction.action.map(|action| {
                    Rc::new(RefCell::new(
                        Button::<RestrictionsAction>::builder(Component::literal(
                            &action.display_string(),
                        ))
                        .bounds(0, 0, ACTION_BUTTON_WIDTH, DEFAULT_HEIGHT)
                        .with_action(RestrictionsAction::Resolve(action))
                        .with_font(font.clone())
                        .build(),
                    ))
                })
            })
            .collect();
        let done = Rc::new(RefCell::new(
            Button::<RestrictionsAction>::builder(Component::literal(&crate::translate(DONE_KEY)))
                .bounds(0, 0, DONE_BUTTON_WIDTH, DEFAULT_HEIGHT)
                .with_action(RestrictionsAction::Done)
                .with_font(font.clone())
                .build(),
        ));
        let mut screen = Self {
            base: ScreenBase::new(Component::literal(&crate::translate(TITLE_KEY))),
            in_world,
            restrictions,
            can_send_messages,
            can_send_commands,
            can_receive_system,
            can_receive_player,
            font,
            body_scroll: ScrollableLayout::new(LinearLayout::vertical().spacing(BODY_SPACING), 0),
            layout: HeaderAndFooterLayout::new(0, 0),
            rows: Vec::new(),
            action_buttons,
            done,
            title_handle: 0,
            done_handle: 0,
            last_size: (0, 0),
        };
        screen
            .body_scroll
            .container_mut()
            .set_draw_list_background(false);
        screen
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn has_any_restrictions(&self) -> bool {
        !self.restrictions.is_empty()
    }
    pub fn take_action(&mut self) -> Option<RestrictionsAction> {
        for button in &self.action_buttons {
            let action = button.as_ref().and_then(Self::drain_action);
            if action.is_some() {
                return action;
            }
        }
        Self::drain_action(&self.done)
    }
    fn drain_action(
        button: &Rc<RefCell<Button<RestrictionsAction>>>,
    ) -> Option<RestrictionsAction> {
        let mut button = button.try_borrow_mut().ok()?;
        button.take_press_action()
    }
    fn init_widgets(&mut self) {
        if self.base.initialized() {
            self.arrange();
            return;
        }
        let message = Component::literal(&crate::translate(TITLE_KEY));
        let mut title =
            StringWidget::new(0, 0, 0, LINE_HEIGHT, message.clone()).with_font(self.font.clone());
        title.set_message(message);
        self.title_handle = self.base.add_widget(title);
        self.rows.clear();
        for (restriction, entry) in self.restrictions.iter().enumerate() {
            let key = entry.translation_key.clone();
            let text = RestrictionText::new(
                Component::literal(&crate::translate(&key)),
                self.font.clone(),
                RESTRICTION_COLOR,
            );
            let (width, height) = (LayoutElement::width(&text), LayoutElement::height(&text));
            let handle = self.base.add_widget(text);
            self.rows.push(Row {
                handle,
                kind: RowKind::RestrictionText {
                    restriction,
                    color: RESTRICTION_COLOR,
                },
                width,
                height,
                top: 0,
            });
            if let (Some(action), Some(shared)) =
                (entry.action, self.action_buttons[restriction].clone())
            {
                let _ = action;
                let handle = self.base.add_widget(SharedButton::new(shared));
                self.rows.push(Row {
                    handle,
                    kind: RowKind::ActionButton { restriction },
                    width: ACTION_BUTTON_WIDTH,
                    height: DEFAULT_HEIGHT,
                    top: 0,
                });
            }
        }
        for permission in 0..PERMISSION_KEYS.len() {
            let key = self.permission_key(permission);
            let color = self.permission_color(permission);
            let text = RestrictionText::new(
                Component::literal(&crate::translate(&key)),
                self.font.clone(),
                color,
            );
            let (width, height) = (LayoutElement::width(&text), LayoutElement::height(&text));
            let handle = self.base.add_widget(text);
            self.rows.push(Row {
                handle,
                kind: RowKind::PermissionText { permission },
                width,
                height,
                top: 0,
            });
        }
        let done = self.done.clone();
        self.done_handle = self.base.add_widget(SharedButton::new(done));
        self.measure_rows();
        self.arrange();
    }
    fn permission_key(&self, permission: usize) -> String {
        let allowed = self.permission_state(permission);
        let suffix = if allowed { "allowed" } else { "denied" };
        format!(
            "restrictions_screen.permission.{}.{}",
            PERMISSION_KEYS[permission], suffix
        )
    }
    fn permission_state(&self, permission: usize) -> bool {
        [
            self.can_send_messages,
            self.can_send_commands,
            self.can_receive_system,
            self.can_receive_player,
        ][permission]
    }
    fn permission_color(&self, permission: usize) -> i32 {
        if self.permission_state(permission) {
            ALLOWED_COLOR
        } else {
            DENIED_COLOR
        }
    }
    fn measure_rows(&mut self) {
        let mut top = FIRST_ROW_TOP;
        let mut permission_seen = false;
        for index in 0..self.rows.len() {
            let kind = self.rows[index].kind.clone();
            if matches!(kind, RowKind::PermissionText { .. }) {
                if permission_seen {
                    top += PERMISSION_SPACING;
                } else if self.has_any_restrictions() {
                    top += SPACER_HEIGHT;
                }
                permission_seen = true;
            }
            let height = self.rows[index].height;
            self.rows[index].top = top;
            top += height;
        }
    }
    fn ensure_initialized(&mut self, width: i32, height: i32) {
        if self.base.initialized() && self.last_size == (width, height) {
            return;
        }
        screen_init(self, width, height);
        self.last_size = (self.base.width, self.base.height);
    }
    fn arrange(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        let mut layout = HeaderAndFooterLayout::new(width, height);
        layout.add_to_header_handle(self.title_handle, self.title_width(), LINE_HEIGHT);
        layout.add_to_footer_handle(self.done_handle, DONE_BUTTON_WIDTH, DEFAULT_HEIGHT);
        {
            let band_handles = [self.title_handle, self.done_handle];
            let mut bands = self
                .base
                .widgets_mut()
                .iter_mut()
                .enumerate()
                .filter(|(index, _)| band_handles.contains(index))
                .map(|(index, widget)| (index, widget.as_mut() as &mut dyn LayoutElement));
            layout.arrange_widgets(&mut bands);
        }
        self.layout = layout;
        self.arrange_body();
    }
    fn title_width(&self) -> i32 {
        match self.base.widgets().get(self.title_handle) {
            Some(widget) => LayoutElement::width(&**widget),
            None => 0,
        }
    }
    fn arrange_body(&mut self) {
        let width = self.base.width;
        let viewport = self.layout.content_height().max(0);
        let scroll = self.body_scroll.scroll_amount();
        let preserve_list_background = self.body_scroll.container().draws_list_background();
        let mut body = LinearLayout::vertical().spacing(BODY_SPACING);
        {
            let cell = body.default_cell_settings();
            cell.align_horizontally_center();
        }
        {
            let specs: Vec<(usize, i32, i32)> = self
                .rows
                .iter()
                .map(|row| (row.handle, row.width, row.height))
                .collect();
            for (handle, row_width, row_height) in &specs {
                body.add_handle(*handle, *row_width, *row_height);
            }
            body.arrange_elements();
        }
        let mut body_scroll = ScrollableLayout::new(body, viewport);
        body_scroll
            .container_mut()
            .set_draw_list_background(preserve_list_background);
        body_scroll.set_max_height(viewport);
        body_scroll.arrange_elements();
        body_scroll.set_scroll_amount(scroll);
        let container_width = LayoutElement::width(&body_scroll);
        let container_x = FrameLayout::align_in_dimension(0, width, container_width, 0.5);
        body_scroll.set_position(container_x, BAND_HEIGHT);
        let handles: Vec<usize> = self.rows.iter().map(|row| row.handle).collect();
        let _ = &handles;
        self.place_rows(&body_scroll);
        self.body_scroll = body_scroll;
    }
    fn place_rows(&mut self, body_scroll: &ScrollableLayout) {
        let content_x = body_scroll.content_x();
        let content_y = body_scroll.content_y();
        let body_width = LayoutElement::width(body_scroll.content());
        let specs: Vec<(usize, i32, i32)> = self
            .rows
            .iter()
            .map(|row| (row.handle, row.width, row.top))
            .collect();
        for (handle, row_width, top) in specs {
            let x = content_x + FrameLayout::align_in_dimension(0, body_width, row_width, 0.5);
            if let Some(widget) = self.base.widgets_mut().get_mut(handle) {
                LayoutElement::set_position(&mut **widget, x, content_y + top);
            }
        }
    }
    pub fn mouse_clicked(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        width: i32,
        height: i32,
        _font: &dyn FontWidthMeasurer,
    ) -> Option<RestrictionsAction> {
        self.ensure_initialized(width, height);
        let event =
            MouseButtonEvent::new(mouse_x as f64, mouse_y as f64, MouseButtonInfo::new(0, 0));
        let _ = Screen::mouse_clicked(self, event, false);
        self.take_action()
    }
    pub fn mouse_released(&mut self) {
        let event = MouseButtonEvent::new(0.0, 0.0, MouseButtonInfo::new(0, 0));
        let _ = Screen::mouse_released(self, event);
    }
    pub fn mouse_dragged(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        dy: f32,
        width: i32,
        height: i32,
        _font: &dyn FontWidthMeasurer,
    ) {
        self.ensure_initialized(width, height);
        let event =
            MouseButtonEvent::new(mouse_x as f64, mouse_y as f64, MouseButtonInfo::new(0, 0));
        let _ = Screen::mouse_dragged(self, event, 0.0, dy as f64);
    }
    pub fn mouse_scrolled(&mut self, scroll_y: f32, _height: i32, _font: &dyn FontWidthMeasurer) {
        let event_scroll = scroll_y as f64;
        let _ = Screen::mouse_scrolled(self, 0.0, 0.0, 0.0, event_scroll);
    }
    pub fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        _font: &dyn FontWidthMeasurer,
    ) {
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        self.ensure_initialized(width, height);
        let mouse_x = graphics.mouse_x();
        let mouse_y = graphics.mouse_y();
        Screen::extract_render_state(self, graphics, mouse_x, mouse_y, 0.0);
    }
    pub fn should_close_on_esc(&self) -> bool {
        self.base_ref().should_close_on_esc()
    }
    pub fn options(&self) -> RestrictionsOptions<'_> {
        RestrictionsOptions { screen: self }
    }
}
pub struct RestrictionsOptions<'a> {
    screen: &'a RestrictionsScreen,
}
impl<'a> RestrictionsOptions<'a> {
    pub fn rows(&self) -> Vec<RowKind> {
        self.screen
            .rows
            .iter()
            .map(|row| row.kind.clone())
            .collect()
    }
    pub fn row_rect(&self, index: usize) -> Option<ScreenRectangle> {
        let handle = self.screen.rows.get(index)?.handle;
        Some(GuiEventListener::rectangle(
            &**self.screen.base.widgets().get(handle)?,
        ))
    }
    pub fn viewport(&self) -> ScreenRectangle {
        self.screen
            .body_scroll
            .container()
            .content_scissor()
            .unwrap_or_default()
    }
    pub fn scrollbar_rect(&self) -> ScreenRectangle {
        let container = self.screen.body_scroll.container();
        ScreenRectangle::new(
            container.scrollbar_x(),
            container.base.y(),
            container.scrollbar_width(),
            container.viewport(),
        )
    }
    pub fn scroller_rect(&self) -> Option<ScreenRectangle> {
        let container = self.screen.body_scroll.container();
        let viewport = container.viewport();
        container
            .scrollbar_visible(viewport)
            .then(|| container.scroller_rect(viewport))
    }
    pub fn max_scroll_amount(&self) -> f32 {
        let container = self.screen.body_scroll.container();
        container.max_scroll_amount(container.viewport())
    }
    pub fn scrollbar_height(&self) -> i32 {
        let container = self.screen.body_scroll.container();
        container.scrollbar_height(container.viewport())
    }
    pub fn scroll_amount(&self) -> f32 {
        self.screen.body_scroll.scroll_amount()
    }
}
impl std::fmt::Debug for RestrictionsScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RestrictionsScreen")
            .field("in_world", &self.in_world)
            .field("restrictions", &self.restrictions)
            .field("can_send_messages", &self.can_send_messages)
            .field("can_send_commands", &self.can_send_commands)
            .field("can_receive_system", &self.can_receive_system)
            .field("can_receive_player", &self.can_receive_player)
            .field("rows", &self.rows.len())
            .field("widgets", &self.base.widgets_len())
            .field("scroll_amount", &self.body_scroll.scroll_amount())
            .field("content_height", &self.body_scroll.content_height())
            .field("close_on_esc", &self.base.should_close_on_esc())
            .finish()
    }
}
impl Screen for RestrictionsScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        self.init_widgets();
    }
    fn reposition_elements(&mut self) {
        self.arrange();
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        if self
            .body_scroll
            .container_mut()
            .mouse_clicked(event, double_click)
        {
            return true;
        }
        screen_mouse_clicked(self, event, double_click)
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        if self.body_scroll.container().is_scrolling() {
            let handled = self
                .body_scroll
                .container_mut()
                .mouse_dragged(event, dx, dy);
            self.arrange_body();
            return handled;
        }
        screen_mouse_dragged(self, event, dx, dy)
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        if self.body_scroll.container().is_scrolling() {
            let handled = self.body_scroll.container_mut().mouse_released(event);
            self.arrange_body();
            return handled;
        }
        screen_mouse_released(self, event)
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        if GuiEventListener::is_mouse_over(&*self.body_scroll.container(), x, y) {
            let handled = self
                .body_scroll
                .container_mut()
                .mouse_scrolled(x, y, scroll_x, scroll_y);
            self.arrange_body();
            return handled;
        }
        crate::framework::screen::screen_mouse_scrolled(self, x, y, scroll_x, scroll_y)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        let mut handled = screen_key_pressed(self, event);
        if !handled {
            handled = self.handle_page_keys(&event);
        }
        self.scroll_focus_into_view();
        handled
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
        self.ensure_initialized(width, height);
        self.base
            .extract_background_backdrop(graphics, self.in_world);
        self.extract_list_background(graphics);
        self.extract_band_chrome(graphics, width, height);
        let row_handles: Vec<usize> = self.rows.iter().map(|row| row.handle).collect();
        let title_handle = self.title_handle;
        let done_handle = self.done_handle;
        self.paint(title_handle, graphics, mouse_x, mouse_y, partial_tick);
        if let Some(rect) = self.body_scroll.container().content_scissor() {
            graphics.enable_scissor(rect.left(), rect.top(), rect.right(), rect.bottom());
        }
        for handle in &row_handles {
            self.paint(*handle, graphics, mouse_x, mouse_y, partial_tick);
        }
        if self.body_scroll.container().content_scissor().is_some() {
            graphics.disable_scissor();
        }
        self.body_scroll.container_mut().extract_scrollbar(graphics);
        self.paint(done_handle, graphics, mouse_x, mouse_y, partial_tick);
    }
    fn update_narration(&mut self, output: &mut NarrationElementOutput) {
        self.base_ref().update_narration(output);
    }
}
impl RestrictionsScreen {
    fn paint(
        &mut self,
        handle: usize,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        if let Some(widget) = self.base.widgets_mut().get_mut(handle) {
            widget.extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
        }
    }
}
impl RestrictionsScreen {
    pub fn body_scroll(&self) -> &ScrollableLayout {
        &self.body_scroll
    }
    pub fn rows(&self) -> Vec<RowKind> {
        self.rows.iter().map(|row| row.kind.clone()).collect()
    }
    pub fn scroll_amount(&self) -> f32 {
        self.body_scroll.scroll_amount()
    }
    pub fn viewport_height(&self) -> i32 {
        self.layout.content_height()
    }
}
impl RestrictionsScreen {
    fn handle_page_keys(&mut self, event: &KeyEvent) -> bool {
        match event.key {
            KeyEvent::KEY_HOME => {
                let target = resolve_tab(self.base.widgets(), None, true);
                self.base.set_focused(target);
                target.is_some()
            }
            KeyEvent::KEY_END => {
                let target = resolve_tab(self.base.widgets(), None, false);
                self.base.set_focused(target);
                target.is_some()
            }
            KEY_PAGE_UP => self.page_scroll(-1.0),
            KEY_PAGE_DOWN => self.page_scroll(1.0),
            _ => false,
        }
    }
    fn page_scroll(&mut self, direction: f32) -> bool {
        let viewport = self.body_scroll.container().viewport();
        if viewport <= 0 {
            return false;
        }
        self.body_scroll
            .container_mut()
            .scroll_by(direction * viewport as f32, viewport);
        self.arrange_body();
        true
    }
    fn scroll_focus_into_view(&mut self) {
        let focused = match self.base.focused_index() {
            Some(handle) => match self.base.widgets().get(handle) {
                Some(widget) => GuiEventListener::rectangle(&**widget),
                None => return,
            },
            None => return,
        };
        let keyboard = self.base_ref().last_input_was_keyboard();
        self.body_scroll.scroll_focused_into_view(focused, keyboard);
        self.arrange_body();
    }
    fn extract_list_background(&self, graphics: &mut GuiGraphicsExtractor) {
        if !self.body_scroll.container().draws_list_background() {
            return;
        }
        let texture = if self.in_world {
            INWORLD_MENU_LIST_BACKGROUND
        } else {
            MENU_LIST_BACKGROUND
        };
        let texture = ResourceLocation::from_string(texture);
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        let mut y = BAND_HEIGHT;
        while y < height - BAND_HEIGHT {
            let tile_h = 32.min(height - BAND_HEIGHT - y);
            let mut x = 0;
            while x < width {
                let tile_w = 32.min(width - x);
                graphics.blit_region(
                    texture.clone(),
                    x,
                    y,
                    tile_w,
                    tile_h,
                    0.0,
                    y as f32 / 32.0,
                    tile_w as f32 / 32.0,
                    tile_h as f32 / 32.0,
                    0xFFFF_FFFFu32 as i32,
                );
                x += tile_w;
            }
            y += tile_h;
        }
    }
    fn extract_band_chrome(&self, graphics: &mut GuiGraphicsExtractor, width: i32, height: i32) {
        graphics.draw_menu_background(self.in_world, width, BAND_HEIGHT);
        let footer_y = height - BAND_HEIGHT;
        graphics.pose_mut().push_pose();
        graphics.pose_mut().translate(0.0, footer_y as f32);
        graphics.draw_menu_background(self.in_world, width, BAND_HEIGHT);
        graphics.pose_mut().pop_pose();
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/header_separator"),
            0,
            BAND_HEIGHT - SEPARATOR_HEIGHT,
            width,
            SEPARATOR_HEIGHT,
        );
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/footer_separator"),
            0,
            footer_y,
            width,
            SEPARATOR_HEIGHT,
        );
    }
}
const KEY_PAGE_UP: i32 = 266;
const KEY_PAGE_DOWN: i32 = 267;
struct RestrictionText {
    text: MultiLineTextWidget,
}
impl RestrictionText {
    fn new(message: Component, font: Arc<dyn FontWidthMeasurer>, color: i32) -> Self {
        let mut text = MultiLineTextWidget::from_message(message).with_font(font);
        text.set_max_width(TEXT_BOX_WIDTH);
        text.set_centered(true);
        text.color = color;
        text.base.active = true;
        Self { text }
    }
}
impl Renderable for RestrictionText {
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        self.text
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
    }
}
impl TabOrderedElement for RestrictionText {
    fn tab_order_group(&self) -> i32 {
        self.text.tab_order_group()
    }
}
impl GuiEventListener for RestrictionText {
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        None
    }
    fn mouse_clicked(&mut self, _event: MouseButtonEvent, _double_click: bool) -> bool {
        false
    }
    fn set_focused(&mut self, focused: bool) {
        self.text.set_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.text.is_focused()
    }
    fn next_focus_path(&self, navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        GuiEventListener::next_focus_path(&self.text, navigation_event)
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        GuiEventListener::is_mouse_over(&self.text, mouse_x, mouse_y)
    }
    fn should_take_focus_after_interaction(&self) -> bool {
        false
    }
}
impl NarratableEntry for RestrictionText {
    fn narration_priority(&self) -> NarrationPriority {
        if self.text.base.focused() {
            NarrationPriority::Focused
        } else if self.text.base.hovered() {
            NarrationPriority::Hovered
        } else {
            NarrationPriority::None
        }
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        output.add(
            NarratedElementType::Title,
            self.text.base.message().get_string(),
        );
    }
}
impl LayoutElement for RestrictionText {
    fn set_x(&mut self, x: i32) {
        self.text.set_x(x);
    }
    fn set_y(&mut self, y: i32) {
        self.text.set_y(y);
    }
    fn x(&self) -> i32 {
        self.text.x()
    }
    fn y(&self) -> i32 {
        self.text.y()
    }
    fn width(&self) -> i32 {
        self.text.width()
    }
    fn height(&self) -> i32 {
        self.text.height()
    }
}
struct SharedButton {
    button: Rc<RefCell<Button<RestrictionsAction>>>,
}
impl SharedButton {
    fn new(button: Rc<RefCell<Button<RestrictionsAction>>>) -> Self {
        Self { button }
    }
    fn with_button<T>(&self, apply: impl FnOnce(&Button<RestrictionsAction>) -> T) -> Option<T> {
        let button = self.button.try_borrow().ok()?;
        Some(apply(&button))
    }
}
impl Renderable for SharedButton {
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        if let Ok(mut button) = self.button.try_borrow_mut() {
            button.extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
        }
    }
}
impl TabOrderedElement for SharedButton {
    fn tab_order_group(&self) -> i32 {
        self.with_button(TabOrderedElement::tab_order_group)
            .unwrap_or(0)
    }
}
impl GuiEventListener for SharedButton {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        match self.button.try_borrow_mut() {
            Ok(mut button) => button.mouse_clicked(event, double_click),
            Err(_) => false,
        }
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        match self.button.try_borrow_mut() {
            Ok(mut button) => button.key_pressed(event),
            Err(_) => false,
        }
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        match self.button.try_borrow_mut() {
            Ok(mut button) => button.mouse_released(event),
            Err(_) => false,
        }
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        match self.button.try_borrow_mut() {
            Ok(mut button) => button.mouse_dragged(event, dx, dy),
            Err(_) => false,
        }
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        match self.button.try_borrow_mut() {
            Ok(mut button) => button.mouse_scrolled(x, y, scroll_x, scroll_y),
            Err(_) => false,
        }
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.with_button(GuiEventListener::play_down_sound)
            .flatten()
    }
    fn next_focus_path(&self, navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.with_button(|button| GuiEventListener::next_focus_path(button, navigation_event))
            .flatten()
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.with_button(|button| GuiEventListener::is_mouse_over(button, mouse_x, mouse_y))
            .unwrap_or(false)
    }
    fn set_focused(&mut self, focused: bool) {
        if let Ok(mut button) = self.button.try_borrow_mut() {
            button.set_focused(focused);
        }
    }
    fn is_focused(&self) -> bool {
        self.with_button(GuiEventListener::is_focused)
            .unwrap_or(false)
    }
    fn should_take_focus_after_interaction(&self) -> bool {
        self.with_button(GuiEventListener::should_take_focus_after_interaction)
            .unwrap_or(true)
    }
    fn rectangle(&self) -> ScreenRectangle {
        self.with_button(GuiEventListener::rectangle)
            .unwrap_or_default()
    }
    fn border_for_arrow_navigation(
        &self,
        opposite: crate::framework::navigation::ScreenDirection,
    ) -> ScreenRectangle {
        self.with_button(|button| GuiEventListener::border_for_arrow_navigation(button, opposite))
            .unwrap_or_default()
    }
}
impl NarratableEntry for SharedButton {
    fn narration_priority(&self) -> NarrationPriority {
        self.with_button(NarratableEntry::narration_priority)
            .unwrap_or(NarrationPriority::None)
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        if let Ok(button) = self.button.try_borrow() {
            button.update_narration(output);
        }
    }
}
impl LayoutElement for SharedButton {
    fn set_x(&mut self, x: i32) {
        if let Ok(mut button) = self.button.try_borrow_mut() {
            button.set_x(x);
        }
    }
    fn set_y(&mut self, y: i32) {
        if let Ok(mut button) = self.button.try_borrow_mut() {
            button.set_y(y);
        }
    }
    fn x(&self) -> i32 {
        self.with_button(LayoutElement::x).unwrap_or(0)
    }
    fn y(&self) -> i32 {
        self.with_button(LayoutElement::y).unwrap_or(0)
    }
    fn width(&self) -> i32 {
        self.with_button(LayoutElement::width).unwrap_or(0)
    }
    fn height(&self) -> i32 {
        self.with_button(LayoutElement::height).unwrap_or(0)
    }
}
