use crate::FontWidthMeasurer;
use crate::components::scroll_area::{
    AbstractScrollArea, SCROLLABLE_LAYOUT_SCROLL_RATE, ScrollMargins, ScrollbarSettings,
};
use crate::components::text_widget::{LINE_HEIGHT, StringWidget};
use crate::components::{
    Button, DEFAULT_HEIGHT, GuiEventListener, TabOrderedElement, WidgetClickSound,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::{
    CONTENT_MARGIN_TOP, DEFAULT_HEADER_AND_FOOTER_HEIGHT, FrameLayout, HeaderAndFooterLayout,
    LayoutElement, LinearLayout,
};
use crate::framework::narration::{NarratableEntry, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{
    ComponentPath, FocusNavigationEvent, ScreenDirection, ScreenRectangle,
};
use crate::framework::screen::{
    Screen, ScreenBase, screen_key_pressed, screen_mouse_clicked, screen_mouse_dragged,
    screen_mouse_released, screen_mouse_scrolled,
};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::Component;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
pub const BUTTON_WIDTH: i32 = 210;
pub const BUTTON_SPACING: i32 = 8;
pub const ROW_HEIGHT: i32 = DEFAULT_HEIGHT;
pub const DONE_WIDTH: i32 = 200;
const DONE_KEY: &str = "gui.done";
const TITLE_KEY: &str = "credits_and_attribution.screen.title";
const CREDITS_KEY: &str = "credits_and_attribution.button.credits";
const ATTRIBUTION_KEY: &str = "credits_and_attribution.button.attribution";
const LICENSES_KEY: &str = "credits_and_attribution.button.licenses";
pub const ROW_OFFSETS: [i32; 3] = [
    0,
    ROW_HEIGHT + BUTTON_SPACING,
    2 * (ROW_HEIGHT + BUTTON_SPACING),
];
pub const CONTENT_HEIGHT: i32 = 3 * ROW_HEIGHT + 2 * BUTTON_SPACING;
#[derive(Clone, Debug)]
pub struct CreditsAndAttributionOption {
    pub key: String,
    pub label: String,
    pub is_slider: bool,
    pub min_val: f32,
    pub max_val: f32,
    pub step: f32,
    pub value: f32,
}
impl CreditsAndAttributionOption {
    pub fn get_display_string(&self) -> String {
        crate::translate(&self.label)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreditsAndAttributionAction {
    Done,
    CreditsClicked,
    AttributionClicked,
    LicensesClicked,
}
fn build_options() -> Vec<CreditsAndAttributionOption> {
    let rows = [
        (CREDITS_KEY, CreditsAndAttributionAction::CreditsClicked),
        (
            ATTRIBUTION_KEY,
            CreditsAndAttributionAction::AttributionClicked,
        ),
        (LICENSES_KEY, CreditsAndAttributionAction::LicensesClicked),
    ];
    rows.iter()
        .map(|(key, _)| CreditsAndAttributionOption {
            key: key.split('.').next_back().unwrap_or_default().to_string(),
            label: (*key).to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: 0.0,
        })
        .collect()
}
pub struct CreditsAndAttributionScreen {
    base: ScreenBase,
    pub in_world: bool,
    pub opened_from_title: bool,
    pub last_width: i32,
    pub last_height: i32,
    pub options: Vec<CreditsAndAttributionOption>,
    font: Arc<dyn FontWidthMeasurer>,
    title_handle: usize,
    row_handles: [usize; 3],
    done_handle: usize,
    row_buttons: [Rc<RefCell<Button<CreditsAndAttributionAction>>>; 3],
    done_button: Rc<RefCell<Button<CreditsAndAttributionAction>>>,
    scroll: AbstractScrollArea,
}
pub const HEADER_AND_FOOTER_HEIGHT: i32 = DEFAULT_HEADER_AND_FOOTER_HEIGHT;
pub const CONTENT_MARGIN: i32 = CONTENT_MARGIN_TOP;
pub const CONTENT_TOP: i32 = HEADER_AND_FOOTER_HEIGHT + CONTENT_MARGIN;
pub const CONTENT_LEFT: i32 = -BUTTON_WIDTH / 2;
pub const DONE_BOTTOM_OFFSET: i32 = 26;
impl CreditsAndAttributionScreen {
    pub fn new(in_world: bool, opened_from_title: bool, font: Arc<dyn FontWidthMeasurer>) -> Self {
        let row_buttons = [
            Rc::new(RefCell::new(Self::build_button(
                CREDITS_KEY,
                CreditsAndAttributionAction::CreditsClicked,
                &font,
            ))),
            Rc::new(RefCell::new(Self::build_button(
                ATTRIBUTION_KEY,
                CreditsAndAttributionAction::AttributionClicked,
                &font,
            ))),
            Rc::new(RefCell::new(Self::build_button(
                LICENSES_KEY,
                CreditsAndAttributionAction::LicensesClicked,
                &font,
            ))),
        ];
        let done_button = Rc::new(RefCell::new(Self::build_button(
            DONE_KEY,
            CreditsAndAttributionAction::Done,
            &font,
        )));
        let mut scroll = AbstractScrollArea::new(
            0,
            CONTENT_TOP,
            0,
            0,
            Component::literal(""),
            ScrollbarSettings::default_settings(SCROLLABLE_LAYOUT_SCROLL_RATE),
        );
        scroll.margins = ScrollMargins::default();
        scroll.content_height = CONTENT_HEIGHT;
        let mut screen = Self {
            base: ScreenBase::new(Component::literal(&crate::translate(TITLE_KEY))),
            in_world,
            opened_from_title,
            last_width: 0,
            last_height: 0,
            options: build_options(),
            font,
            title_handle: 0,
            row_handles: [0; 3],
            done_handle: 0,
            row_buttons,
            done_button,
            scroll,
        };
        screen.base.set_close_on_esc(true);
        screen
    }
    fn build_button(
        key: &str,
        action: CreditsAndAttributionAction,
        font: &Arc<dyn FontWidthMeasurer>,
    ) -> Button<CreditsAndAttributionAction> {
        let width = if action == CreditsAndAttributionAction::Done {
            DONE_WIDTH
        } else {
            BUTTON_WIDTH
        };
        Button::<CreditsAndAttributionAction>::builder(Component::literal(&crate::translate(key)))
            .with_action(action)
            .with_font(font.clone())
            .size(width, ROW_HEIGHT)
            .build()
    }
    fn build_title(&self) -> StringWidget {
        let message = Component::literal(&crate::translate(TITLE_KEY));
        let mut title =
            StringWidget::new(0, 0, 0, LINE_HEIGHT, message.clone()).with_font(self.font.clone());
        title.set_message(message);
        title
    }
    pub fn take_action(&mut self) -> Option<CreditsAndAttributionAction> {
        for button in &self.row_buttons {
            let taken = match button.try_borrow_mut() {
                Ok(mut button) => button.take_press_action(),
                Err(_) => None,
            };
            if taken.is_some() {
                return taken;
            }
        }
        self.done_button
            .try_borrow_mut()
            .ok()
            .and_then(|mut button| button.take_press_action())
    }
    fn widget_size(&self, handle: usize) -> (i32, i32) {
        match self.base.widgets().get(handle) {
            Some(widget) => (
                LayoutElement::width(&**widget),
                LayoutElement::height(&**widget),
            ),
            None => (0, 0),
        }
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn opened_from_title(&self) -> bool {
        self.opened_from_title
    }
    pub fn scroll_amount(&self) -> f32 {
        self.scroll.scroll_amount
    }
    pub fn scrollbar_x(&self) -> i32 {
        self.scroll.scrollbar_x()
    }
    pub fn scrollbar_height(&self) -> i32 {
        self.scroll
            .scrollbar_height(self.scroll.height_for_viewport())
    }
    pub fn scrollbar_y(&self) -> i32 {
        self.scroll.scrollbar_y(self.scroll.height_for_viewport())
    }
    pub fn viewport(&self) -> ScreenRectangle {
        ScreenRectangle::new(
            0,
            self.content_top(),
            self.base.width,
            self.viewport_height(),
        )
    }
    fn content_top(&self) -> i32 {
        (HEADER_AND_FOOTER_HEIGHT + CONTENT_MARGIN)
            .min(self.base.height - HEADER_AND_FOOTER_HEIGHT - CONTENT_HEIGHT)
    }
    fn viewport_height(&self) -> i32 {
        (self.base.height - HEADER_AND_FOOTER_HEIGHT - self.content_top()).max(0)
    }
    pub fn set_content_height(&mut self, content_height: i32) {
        self.scroll.content_height = content_height.max(0);
        self.scroll.clamp_scroll(self.scroll.height_for_viewport());
        self.apply_scroll_offset();
    }
    fn apply_scroll_offset(&mut self) {
        let content_top = self.content_top();
        let offset = self.scroll.content_y_offset();
        for (index, button) in self.row_buttons.iter().enumerate() {
            let row_y = content_top + ROW_OFFSETS[index] + offset;
            if let Ok(mut button) = button.try_borrow_mut() {
                button.set_y(row_y);
            }
        }
    }
    fn arrange(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        self.last_width = width;
        self.last_height = height;
        self.scroll.base.set_x(0);
        self.scroll.base.set_y(self.content_top());
        self.scroll.base.set_width(width);
        self.scroll.base.set_height(self.viewport_height());
        self.scroll.clamp_scroll(self.scroll.height_for_viewport());
        let mut bands = self.build_bands(width);
        let column_x = FrameLayout::align_in_dimension(0, width, BUTTON_WIDTH, 0.5);
        let mut column = self.build_column();
        let content_top = self.content_top();
        {
            let mut widgets = self.widget_store();
            bands.arrange_widgets(&mut widgets);
        }
        {
            let mut widgets = self.widget_store();
            column.arrange_widgets(&mut widgets);
        }
        column.set_position(column_x, content_top);
        {
            let mut widgets = self.widget_store();
            column.arrange_widgets(&mut widgets);
        }
        self.apply_scroll_offset();
    }
    fn widget_store(&mut self) -> impl Iterator<Item = (usize, &mut dyn LayoutElement)> + '_ {
        self.base
            .widgets_mut()
            .iter_mut()
            .enumerate()
            .map(|(index, widget)| (index, widget.as_mut() as &mut dyn LayoutElement))
    }
    fn build_bands(&self, width: i32) -> HeaderAndFooterLayout {
        let mut bands = HeaderAndFooterLayout::new(width, self.base.height);
        let (title_width, title_height) = self.widget_size(self.title_handle);
        bands.add_to_header_handle(self.title_handle, title_width, title_height);
        bands.add_to_footer_handle(self.done_handle, DONE_WIDTH, ROW_HEIGHT);
        bands
    }
    fn build_column(&self) -> LinearLayout {
        let mut column = LinearLayout::vertical().spacing(BUTTON_SPACING);
        {
            let cell = column.default_cell_settings();
            cell.align_horizontally_center();
        }
        for handle in self.row_handles {
            column.add_handle(handle, BUTTON_WIDTH, ROW_HEIGHT);
        }
        column
    }
}
impl std::fmt::Debug for CreditsAndAttributionScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreditsAndAttributionScreen")
            .field("in_world", &self.in_world)
            .field("opened_from_title", &self.opened_from_title)
            .field("widgets", &self.base.widgets_len())
            .field("title_handle", &self.title_handle)
            .field("row_handles", &self.row_handles)
            .field("done_handle", &self.done_handle)
            .field("close_on_esc", &self.base.should_close_on_esc())
            .field("viewport", &self.viewport())
            .field("scroll_amount", &self.scroll_amount())
            .finish()
    }
}
impl Screen for CreditsAndAttributionScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        self.title_handle = self.base.add_widget(self.build_title());
        for (index, button) in self.row_buttons.iter().enumerate() {
            self.row_handles[index] = self.base.add_widget(SharedButton::new(button.clone()));
        }
        self.done_handle = self
            .base
            .add_widget(SharedButton::new(self.done_button.clone()));
        self.arrange();
    }
    fn reposition_elements(&mut self) {
        self.arrange();
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        let in_world = self.in_world;
        self.base().extract_background_layers(graphics, in_world);
        self.base()
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
        self.scroll.extract_scrollbar(graphics);
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if screen_key_pressed(self, event) {
            return true;
        }
        self.scroll_by_key(event)
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        if screen_mouse_clicked(self, event, double_click) {
            return true;
        }
        self.scroll.scrollbar_clicked_with_button(
            event.x,
            event.y,
            event.button(),
            self.scroll.height_for_viewport(),
        )
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        if self.scroll.is_scrolling() {
            self.scroll
                .scrollbar_dragged(event.y, dy, self.scroll.height_for_viewport());
            self.after_scroll();
            return true;
        }
        screen_mouse_dragged(self, event, dx, dy)
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        if self.scroll.is_scrolling() {
            self.scroll.scrollbar_released();
            return true;
        }
        screen_mouse_released(self, event)
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        if screen_mouse_scrolled(self, x, y, scroll_x, scroll_y) {
            return true;
        }
        let viewport = self.viewport();
        if !viewport.contains_point(x as i32, y as i32) {
            return false;
        }
        let scrolled = self.scroll.mouse_scrolled(x, y, scroll_x, scroll_y);
        if scrolled {
            self.after_scroll();
        }
        scrolled
    }
}
impl CreditsAndAttributionScreen {
    fn scroll_by_key(&mut self, event: KeyEvent) -> bool {
        const KEY_PAGE_UP: i32 = 266;
        const KEY_PAGE_DOWN: i32 = 267;
        let height = self.scroll.height_for_viewport();
        let page = self.scroll.viewport().max(1);
        let before = self.scroll.scroll_amount;
        match event.key {
            KEY_PAGE_UP => {
                self.scroll.scroll_by(-page as f32, height);
            }
            KEY_PAGE_DOWN => {
                self.scroll.scroll_by(page as f32, height);
            }
            KeyEvent::KEY_HOME => {
                self.scroll.set_scroll_amount(0.0, height);
            }
            KeyEvent::KEY_END => {
                self.scroll
                    .set_scroll_amount(self.scroll.max_scroll_amount(height), height);
            }
            _ => return false,
        }
        let moved = self.scroll.scroll_amount != before;
        if moved {
            self.after_scroll();
        }
        moved
    }
    fn after_scroll(&mut self) {
        self.apply_scroll_offset();
    }
}
struct SharedButton {
    button: Rc<RefCell<Button<CreditsAndAttributionAction>>>,
}
impl SharedButton {
    fn new(button: Rc<RefCell<Button<CreditsAndAttributionAction>>>) -> Self {
        Self { button }
    }
}
impl crate::components::Renderable for SharedButton {
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
        match self.button.try_borrow() {
            Ok(button) => button.tab_order_group(),
            Err(_) => 0,
        }
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
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        match self.button.try_borrow() {
            Ok(button) => button.play_down_sound(),
            Err(_) => None,
        }
    }
    fn next_focus_path(&self, navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        match self.button.try_borrow() {
            Ok(button) => GuiEventListener::next_focus_path(&*button, navigation_event),
            Err(_) => None,
        }
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        match self.button.try_borrow() {
            Ok(button) => button.is_mouse_over(mouse_x, mouse_y),
            Err(_) => false,
        }
    }
    fn set_focused(&mut self, focused: bool) {
        if let Ok(mut button) = self.button.try_borrow_mut() {
            button.set_focused(focused);
        }
    }
    fn is_focused(&self) -> bool {
        match self.button.try_borrow() {
            Ok(button) => button.is_focused(),
            Err(_) => false,
        }
    }
    fn rectangle(&self) -> ScreenRectangle {
        match self.button.try_borrow() {
            Ok(button) => GuiEventListener::rectangle(&*button),
            Err(_) => ScreenRectangle::empty(),
        }
    }
    fn border_for_arrow_navigation(&self, opposite: ScreenDirection) -> ScreenRectangle {
        match self.button.try_borrow() {
            Ok(button) => button.border_for_arrow_navigation(opposite),
            Err(_) => GuiEventListener::rectangle(self),
        }
    }
}
impl NarratableEntry for SharedButton {
    fn is_active(&self) -> bool {
        self.button
            .try_borrow()
            .map(|button| button.is_active())
            .unwrap_or(false)
    }
    fn narration_priority(&self) -> NarrationPriority {
        match self.button.try_borrow() {
            Ok(button) => button.narration_priority(),
            Err(_) => NarrationPriority::None,
        }
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
        match self.button.try_borrow() {
            Ok(button) => button.x(),
            Err(_) => 0,
        }
    }
    fn y(&self) -> i32 {
        match self.button.try_borrow() {
            Ok(button) => button.y(),
            Err(_) => 0,
        }
    }
    fn width(&self) -> i32 {
        match self.button.try_borrow() {
            Ok(button) => button.width(),
            Err(_) => 0,
        }
    }
    fn height(&self) -> i32 {
        match self.button.try_borrow() {
            Ok(button) => button.height(),
            Err(_) => 0,
        }
    }
}
