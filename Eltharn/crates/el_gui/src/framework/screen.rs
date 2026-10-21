use crate::components::Widget;
use crate::framework::focus;
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::narration::{
    NarratableEntry, NarratedElementType, NarrationElementOutput, NarrationPriority,
};
use crate::framework::navigation::{FocusNavigationEvent, ScreenDirection, ScreenRectangle};
use crate::input::events::{CharacterEvent, KeyEvent, MouseButtonEvent, PreeditEvent};
use el_core::Component;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenAction {
    None,
    Close,
}
pub const NARRATE_SUPPRESS_AFTER_INIT_TIME: i64 = 2000;
pub const NARRATE_DELAY_NARRATOR_ENABLED: i64 = NARRATE_SUPPRESS_AFTER_INIT_TIME;
pub const NARRATE_DELAY_MOUSE_MOVE: i64 = 750;
pub const NARRATE_DELAY_MOUSE_ACTION: i64 = 200;
pub const NARRATE_DELAY_KEYBOARD_ACTION: i64 = 200;
const SCREEN_USAGE_NARRATION_KEY: &str = "narrator.screen.usage";
const COMPONENT_LIST_USAGE_KEY: &str = "narration.component_list.usage";
static NEXT_SCREEN_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);
const NARRATED_ELEMENT_TYPES: [NarratedElementType; 4] = [
    NarratedElementType::Title,
    NarratedElementType::Position,
    NarratedElementType::Hint,
    NarratedElementType::Usage,
];
pub struct ScreenBase {
    instance_id: u64,
    title: Component,
    pub width: i32,
    pub height: i32,
    widgets: Vec<Box<dyn Widget>>,
    focused: Option<usize>,
    initialized: bool,
    dragging: bool,
    pause_screen: bool,
    close_on_esc: bool,
    last_input_was_keyboard: bool,
    pending_double_click: bool,
    close_requested: bool,
    ticks: u64,
    clock_ms: i64,
    narration_suppress_time: i64,
    next_narration_time: i64,
    narrator_active: bool,
    last_narration_time: i64,
    last_narratable: Cell<Option<usize>>,
    narration_state: NarrationCollector,
    pending_click_sound: Option<crate::components::WidgetClickSound>,
    menu_background_blur_enabled: bool,
}
impl ScreenBase {
    pub fn new(title: Component) -> Self {
        Self {
            instance_id: NEXT_SCREEN_INSTANCE_ID.fetch_add(1, Ordering::Relaxed),
            title,
            width: 0,
            height: 0,
            widgets: Vec::new(),
            focused: None,
            initialized: false,
            dragging: false,
            pause_screen: true,
            close_on_esc: true,
            last_input_was_keyboard: false,
            pending_double_click: false,
            close_requested: false,
            ticks: 0,
            clock_ms: 0,
            narration_suppress_time: i64::MIN,
            next_narration_time: i64::MAX,
            narrator_active: false,
            last_narration_time: i64::MIN,
            last_narratable: Cell::new(None),
            narration_state: NarrationCollector::new(),
            pending_click_sound: None,
            menu_background_blur_enabled: true,
        }
    }
    pub fn title(&self) -> &Component {
        &self.title
    }
    pub fn instance_id(&self) -> u64 {
        self.instance_id
    }
    pub fn set_pending_double_click(&mut self, double_click: bool) {
        self.pending_double_click = double_click;
    }
    pub fn take_pending_double_click(&mut self) -> bool {
        std::mem::take(&mut self.pending_double_click)
    }
    pub fn narration_message(&self) -> String {
        self.title.get_string()
    }
    pub fn init(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
        self.initialized = true;
    }
    pub fn initialized(&self) -> bool {
        self.initialized
    }
    pub fn set_initialized(&mut self, initialized: bool) {
        self.initialized = initialized;
    }
    pub fn add_widget(&mut self, widget: impl Widget + 'static) -> usize {
        self.widgets.push(Box::new(widget));
        self.widgets.len() - 1
    }
    pub fn remove_widget(&mut self, index: usize) {
        if index >= self.widgets.len() {
            return;
        }
        self.widgets.remove(index);
        match self.focused {
            Some(focused) if focused == index => self.set_focused(None),
            Some(focused) if focused > index => self.set_founded_shifted(focused - 1),
            _ => {}
        }
        self.last_narratable.set(None);
    }
    pub fn clear_widgets(&mut self) {
        self.widgets.clear();
        self.focused = None;
        self.last_narratable.set(None);
    }
    pub fn widgets_len(&self) -> usize {
        self.widgets.len()
    }
    pub fn widgets(&self) -> &[Box<dyn Widget>] {
        &self.widgets
    }
    pub fn widgets_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.widgets
    }
    pub fn narratables(&self) -> impl Iterator<Item = &dyn NarratableEntry> + '_ {
        self.widgets
            .iter()
            .map(|widget| &**widget as &dyn NarratableEntry)
    }
    pub fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        for widget in &mut self.widgets {
            widget.extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
        }
    }
    pub fn child_at(&self, x: f64, y: f64) -> Option<usize> {
        self.widgets
            .iter()
            .position(|widget| widget.is_mouse_over(x, y))
    }
    pub fn key_pressed(&mut self, event: KeyEvent) -> ScreenAction {
        match self.dispatch_key_pressed(event, self.close_on_esc) {
            KeyDispatch::Close => ScreenAction::Close,
            KeyDispatch::Consumed | KeyDispatch::Ignored => ScreenAction::None,
        }
    }
    pub fn dispatch_focused_key_pressed(&mut self, event: KeyEvent) -> bool {
        match self.focused_widget() {
            Some(index) => self.widgets[index].key_pressed(event),
            None => false,
        }
    }
    pub fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        let Some(index) = self.child_at(event.x, event.y) else {
            return false;
        };
        if self.widgets[index].mouse_clicked(event, double_click) {
            self.pending_click_sound = self.widgets[index].play_down_sound();
            if self.widgets[index].should_take_focus_after_interaction() {
                self.set_focused(Some(index));
                if event.button() == 0 {
                    self.set_dragging(true);
                }
            }
        }
        true
    }
    pub fn take_pending_click_sound(&mut self) -> Option<crate::components::WidgetClickSound> {
        self.pending_click_sound.take()
    }
    pub fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        if event.button() == 0 && self.dragging {
            self.set_dragging(false);
            if let Some(index) = self.focused_widget() {
                return self.widgets[index].mouse_released(event);
            }
        }
        false
    }
    pub fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        if event.button() != 0 || !self.dragging {
            return false;
        }
        match self.focused_widget() {
            Some(index) => self.widgets[index].mouse_dragged(event, dx, dy),
            None => false,
        }
    }
    pub fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        match self.child_at(x, y) {
            Some(index) => self.widgets[index].mouse_scrolled(x, y, scroll_x, scroll_y),
            None => false,
        }
    }
    pub fn dispatch_focused_char_typed(&mut self, event: CharacterEvent) -> bool {
        match self.focused_widget() {
            Some(index) => self.widgets[index].char_typed(event),
            None => false,
        }
    }
    pub fn dispatch_focused_preedit_updated(&mut self, event: Option<PreeditEvent>) -> bool {
        match self.focused_widget() {
            Some(index) => self.widgets[index].preedit_updated(event),
            None => false,
        }
    }
    pub fn set_dragging(&mut self, dragging: bool) {
        self.dragging = dragging;
    }
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }
    pub fn is_pause_screen(&self) -> bool {
        self.pause_screen
    }
    pub fn set_pause_screen(&mut self, pause_screen: bool) {
        self.pause_screen = pause_screen;
    }
    pub fn should_close_on_esc(&self) -> bool {
        self.close_on_esc
    }
    pub fn set_close_on_esc(&mut self, close_on_esc: bool) {
        self.close_on_esc = close_on_esc;
    }
    pub fn set_initial_focus(&mut self) {
        self.focused = None;
        if !self.last_input_was_keyboard {
            return;
        }
        if let Some(index) =
            self.resolve_focus_path(&FocusNavigationEvent::TabNavigation { forward: true })
        {
            self.set_focused(Some(index));
        }
    }
    pub fn clear_focus(&mut self) {
        self.set_focused(None);
    }
    pub fn focused_index(&self) -> Option<usize> {
        self.focused
    }
    pub fn set_focused(&mut self, focused: Option<usize>) {
        if let Some(old) = self.focused.take() {
            if old < self.widgets.len() {
                self.widgets[old].set_focused(false);
            }
        }
        self.focused = focused;
        if let Some(new) = focused {
            self.widgets[new].set_focused(true);
        }
    }
    fn set_founded_shifted(&mut self, focused: usize) {
        self.set_focused(Some(focused));
    }
    fn focused_widget(&self) -> Option<usize> {
        self.focused.filter(|index| *index < self.widgets.len())
    }
    pub fn set_clock(&mut self, now_ms: i64) {
        self.clock_ms = now_ms;
    }
    pub fn clock_ms(&self) -> i64 {
        self.clock_ms
    }
    pub fn set_last_input_keyboard(&mut self, keyboard: bool) {
        self.last_input_was_keyboard = keyboard;
    }
    pub fn last_input_was_keyboard(&self) -> bool {
        self.last_input_was_keyboard
    }
    pub fn set_narrator_active(&mut self, active: bool) {
        self.narrator_active = active;
    }
    pub fn narrator_active(&self) -> bool {
        self.narrator_active
    }
    pub fn should_run_narration(&self) -> bool {
        self.narrator_active
    }
    pub fn request_close(&mut self) {
        self.close_requested = true;
    }
    pub fn take_close_request(&mut self) -> bool {
        let requested = self.close_requested;
        self.close_requested = false;
        requested
    }
    pub fn tick(&mut self) {
        self.ticks = self.ticks.saturating_add(1);
    }
    pub fn ticks(&self) -> u64 {
        self.ticks
    }
    fn schedule_narration(&mut self, delay: i64, ignore_suppression: bool) {
        self.next_narration_time = self.clock_ms.saturating_add(delay);
        if ignore_suppression {
            self.narration_suppress_time = i64::MIN;
        }
    }
    fn suppress_narration(&mut self, duration: i64) {
        self.narration_suppress_time = self.clock_ms.saturating_add(duration);
    }
    fn set_narration_suppress_time(&mut self, time: i64) {
        self.narration_suppress_time = time;
    }
    pub fn narration_suppress_time(&self) -> i64 {
        self.narration_suppress_time
    }
    pub fn next_narration_time(&self) -> i64 {
        self.next_narration_time
    }
    pub fn last_narration_time(&self) -> i64 {
        self.last_narration_time
    }
    pub fn after_mouse_move(&mut self) {
        self.schedule_narration(NARRATE_DELAY_MOUSE_MOVE, false);
    }
    pub fn after_mouse_action(&mut self) {
        self.schedule_narration(NARRATE_DELAY_MOUSE_ACTION, true);
    }
    pub fn after_keyboard_action(&mut self) {
        self.schedule_narration(NARRATE_DELAY_KEYBOARD_ACTION, true);
    }
    pub fn narration_due(&self) -> bool {
        self.clock_ms > self.next_narration_time && self.clock_ms > self.narration_suppress_time
    }
    pub fn handle_delayed_narration(&mut self) -> bool {
        if !self.should_run_narration() || !self.narration_due() {
            return false;
        }
        self.run_narration(true);
        self.next_narration_time = i64::MAX;
        self.last_narration_time = self.clock_ms;
        true
    }
    pub fn trigger_immediate_narration(&mut self, only_changed: bool) -> bool {
        if !self.should_run_narration() {
            return false;
        }
        self.run_narration(only_changed);
        self.last_narration_time = self.clock_ms;
        true
    }
    fn run_narration(&mut self, only_changed: bool) {
        let mut output = NarrationElementOutput::default();
        self.update_narration(&mut output);
        let narration = self.narration_state.update(&output, !only_changed);
        if !narration.is_empty() {
            self.narration_state.set_last_text(narration);
        }
    }
    pub fn take_narration(&mut self) -> Option<String> {
        self.narration_state.take_last_text()
    }
    pub fn update_narration(&self, output: &mut NarrationElementOutput) {
        output.add(NarratedElementType::Title, self.narration_message());
        self.update_narrated_widget(output);
    }
    fn update_narrated_widget(&self, output: &mut NarrationElementOutput) {
        let narratables: Vec<&dyn Widget> = self
            .widgets
            .iter()
            .map(|widget| &**widget as &dyn Widget)
            .collect();
        let mut sorted: Vec<usize> = (0..narratables.len())
            .filter(|index| narratables[*index].is_active())
            .collect();
        sorted.sort_by_key(|index| narratables[*index].tab_order_group());
        let winner =
            find_narratable_widget_indices(&sorted, &narratables, self.last_narratable.get());
        let Some(winner) = winner else {
            if self.should_narrate_navigation() {
                output.add(
                    NarratedElementType::Usage,
                    crate::translate(SCREEN_USAGE_NARRATION_KEY),
                );
            }
            return;
        };
        if is_terminal(narratables[winner].narration_priority()) {
            self.last_narratable.set(Some(winner));
        }
        if sorted.len() > 1 {
            let position = sorted
                .iter()
                .position(|index| *index == winner)
                .unwrap_or(0);
            output.add(
                NarratedElementType::Position,
                format!("narrator.position.screen {} {}", position + 1, sorted.len()),
            );
            if narratables[winner].narration_priority() == NarrationPriority::Focused {
                output.add(
                    NarratedElementType::Usage,
                    crate::translate(COMPONENT_LIST_USAGE_KEY),
                );
            }
        }
        narratables[winner].update_narration(output);
    }
    pub fn should_narrate_navigation(&self) -> bool {
        true
    }
    pub fn menu_background_blur_enabled(&self) -> bool {
        self.menu_background_blur_enabled
    }
    pub fn set_menu_background_blur_enabled(&mut self, enabled: bool) {
        self.menu_background_blur_enabled = enabled;
    }
    pub fn extract_panorama(&self, graphics: &mut GuiGraphicsExtractor) {
        graphics.panorama();
    }
    pub fn extract_blurred_background(&self, graphics: &mut GuiGraphicsExtractor) {
        if self.menu_background_blur_enabled {
            graphics.blur_before_this_stratum();
            graphics.next_stratum();
        }
    }
    pub fn extract_background_layers(&self, graphics: &mut GuiGraphicsExtractor, in_world: bool) {
        self.extract_background_backdrop(graphics, in_world);
        graphics.draw_menu_background(in_world, self.width, self.height);
    }
    pub fn extract_background_backdrop(&self, graphics: &mut GuiGraphicsExtractor, in_world: bool) {
        if !in_world {
            self.extract_panorama(graphics);
        }
        self.extract_blurred_background(graphics);
    }
    pub fn extract_transparent_background(&self, graphics: &mut GuiGraphicsExtractor) {
        graphics.fill_gradient(0, 0, self.width, self.height, -1072689136, -804253680);
    }
}
pub trait Screen {
    fn base(&mut self) -> &mut ScreenBase;
    fn base_ref(&self) -> &ScreenBase;
    fn init(&mut self) {}
    fn tick(&mut self) {}
    fn added(&mut self) {}
    fn removed(&mut self) {}
    fn reposition_elements(&mut self) {
        screen_rebuild_widgets(self);
    }
    fn on_close(&mut self) {
        self.base().request_close();
    }
    fn should_close_on_esc(&self) -> bool {
        self.base_ref().should_close_on_esc()
    }
    fn is_pause_screen(&self) -> bool {
        self.base_ref().is_pause_screen()
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        self.base()
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
    }
    fn extract_background(
        &mut self,
        _graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        screen_key_pressed(self, event)
    }
    fn key_released(&mut self, event: KeyEvent) -> bool {
        match self.base().focused_widget() {
            Some(index) => self.base().widgets[index].key_released(event),
            None => false,
        }
    }
    fn char_typed(&mut self, event: CharacterEvent) -> bool {
        screen_char_typed(self, event)
    }
    fn preedit_updated(&mut self, event: Option<PreeditEvent>) -> bool {
        screen_preedit_updated(self, event)
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        screen_mouse_clicked(self, event, double_click)
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        screen_mouse_released(self, event)
    }
    fn mouse_moved(&mut self, _x: f64, _y: f64) {}
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        screen_mouse_dragged(self, event, dx, dy)
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        screen_mouse_scrolled(self, x, y, scroll_x, scroll_y)
    }
    fn handle_delayed_narration(&mut self) -> bool {
        self.base().handle_delayed_narration()
    }
    fn update_narration(&mut self, output: &mut NarrationElementOutput) {
        self.base().update_narration(output);
    }
    fn fill_crash_details(&self, details: &mut Vec<(&'static str, String)>) {
        let name = std::any::type_name::<Self>();
        details.push(("Screen name", name.to_string()));
    }
}
pub fn screen_init(screen: &mut (impl Screen + ?Sized), width: i32, height: i32) {
    {
        let base = screen.base();
        base.width = width;
        base.height = height;
    }
    if screen.base_ref().initialized() {
        screen.reposition_elements();
    } else {
        screen.init();
        screen.base().set_initial_focus();
    }
    screen.base().set_initialized(true);
    screen.base().trigger_immediate_narration(false);
    if screen.base_ref().last_input_was_keyboard() {
        screen.base().set_narration_suppress_time(i64::MAX);
    } else {
        screen
            .base()
            .suppress_narration(NARRATE_SUPPRESS_AFTER_INIT_TIME);
    }
}
pub fn screen_resize(screen: &mut (impl Screen + ?Sized), width: i32, height: i32) {
    {
        let base = screen.base();
        base.width = width;
        base.height = height;
    }
    screen.reposition_elements();
}
pub fn screen_rebuild_widgets(screen: &mut (impl Screen + ?Sized)) {
    screen.base().clear_focus();
    screen.base().clear_widgets();
    screen.init();
    screen.base().set_initial_focus();
}
pub fn screen_tick(screen: &mut (impl Screen + ?Sized)) {
    screen.base().tick();
    screen.tick();
}
pub fn screen_added(screen: &mut (impl Screen + ?Sized)) {
    screen.added();
}
pub fn screen_removed(screen: &mut (impl Screen + ?Sized)) {
    screen.removed();
}
pub fn screen_extract_render_state_with_tooltip_and_subtitles(
    screen: &mut (impl Screen + ?Sized),
    graphics: &mut GuiGraphicsExtractor,
    mouse_x: i32,
    mouse_y: i32,
    partial_tick: f32,
) {
    graphics.next_stratum();
    screen.extract_background(graphics, mouse_x, mouse_y, partial_tick);
    graphics.next_stratum();
    screen.extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
}
pub fn screen_set_initial_focus(screen: &mut (impl Screen + ?Sized)) {
    screen.base().set_initial_focus();
}
pub fn screen_key_pressed(screen: &mut (impl Screen + ?Sized), event: KeyEvent) -> bool {
    screen.base().set_last_input_keyboard(true);
    screen.base().after_keyboard_action();
    let close_on_esc = screen.base_ref().should_close_on_esc();
    match screen.base().dispatch_key_pressed(event, close_on_esc) {
        KeyDispatch::Close => {
            screen.on_close();
            true
        }
        KeyDispatch::Consumed => true,
        KeyDispatch::Ignored => false,
    }
}
pub fn screen_key_released(screen: &mut (impl Screen + ?Sized), event: KeyEvent) -> bool {
    match screen.base().focused_widget() {
        Some(index) => screen.base().widgets[index].key_released(event),
        None => false,
    }
}
pub fn screen_char_typed(screen: &mut (impl Screen + ?Sized), event: CharacterEvent) -> bool {
    match screen.base().focused_widget() {
        Some(index) => screen.base().widgets[index].char_typed(event),
        None => false,
    }
}
pub fn screen_preedit_updated(
    screen: &mut (impl Screen + ?Sized),
    event: Option<PreeditEvent>,
) -> bool {
    match screen.base().focused_widget() {
        Some(index) => screen.base().widgets[index].preedit_updated(event),
        None => false,
    }
}
pub fn screen_mouse_clicked(
    screen: &mut (impl Screen + ?Sized),
    event: MouseButtonEvent,
    double_click: bool,
) -> bool {
    screen.base().set_last_input_keyboard(false);
    screen.base().after_mouse_action();
    screen.base().mouse_clicked(event, double_click)
}
pub fn screen_mouse_released(screen: &mut (impl Screen + ?Sized), event: MouseButtonEvent) -> bool {
    screen.base().mouse_released(event)
}
pub fn screen_mouse_moved(screen: &mut (impl Screen + ?Sized), x: f64, y: f64) {
    screen.base().set_last_input_keyboard(false);
    screen.mouse_moved(x, y);
    screen.base().after_mouse_move();
}
pub fn screen_mouse_dragged(
    screen: &mut (impl Screen + ?Sized),
    event: MouseButtonEvent,
    dx: f64,
    dy: f64,
) -> bool {
    let handled = screen.base().mouse_dragged(event, dx, dy);
    screen.base().after_mouse_move();
    handled
}
pub fn screen_mouse_scrolled(
    screen: &mut (impl Screen + ?Sized),
    x: f64,
    y: f64,
    scroll_x: f64,
    scroll_y: f64,
) -> bool {
    screen.base().set_last_input_keyboard(false);
    let handled = screen.base().mouse_scrolled(x, y, scroll_x, scroll_y);
    screen.base().after_mouse_action();
    handled
}
pub fn screen_update_narration(
    screen: &mut (impl Screen + ?Sized),
    output: &mut NarrationElementOutput,
) {
    screen.update_narration(output);
}
pub fn screen_handle_delayed_narration(screen: &mut (impl Screen + ?Sized)) -> bool {
    screen.handle_delayed_narration()
}
pub fn screen_narration_message(screen: &(impl Screen + ?Sized)) -> String {
    screen.base_ref().narration_message()
}
pub fn screen_usage_narration(screen: &(impl Screen + ?Sized)) -> String {
    let _ = screen;
    crate::translate(COMPONENT_LIST_USAGE_KEY)
}
pub fn screen_fill_crash_details(screen: &(impl Screen + ?Sized)) -> Vec<(&'static str, String)> {
    let mut details = Vec::new();
    screen.fill_crash_details(&mut details);
    details
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyDispatch {
    Close,
    Consumed,
    Ignored,
}
impl ScreenBase {
    fn dispatch_key_pressed(&mut self, event: KeyEvent, close_on_esc: bool) -> KeyDispatch {
        if event.is_escape() && close_on_esc {
            self.request_close();
            return KeyDispatch::Close;
        }
        if self.dispatch_focused_key_pressed(event) {
            return KeyDispatch::Consumed;
        }
        let navigation_event = match event.key {
            KeyEvent::KEY_TAB => Some(FocusNavigationEvent::TabNavigation {
                forward: !event.has_shift_down(),
            }),
            KeyEvent::KEY_LEFT => Some(FocusNavigationEvent::ArrowNavigation {
                direction: ScreenDirection::Left,
            }),
            KeyEvent::KEY_RIGHT => Some(FocusNavigationEvent::ArrowNavigation {
                direction: ScreenDirection::Right,
            }),
            KeyEvent::KEY_UP => Some(FocusNavigationEvent::ArrowNavigation {
                direction: ScreenDirection::Up,
            }),
            KeyEvent::KEY_DOWN => Some(FocusNavigationEvent::ArrowNavigation {
                direction: ScreenDirection::Down,
            }),
            _ => None,
        };
        if let Some(navigation_event) = navigation_event {
            let mut focus_path = self.resolve_focus_path(&navigation_event);
            if focus_path.is_none()
                && matches!(navigation_event, FocusNavigationEvent::TabNavigation { .. })
            {
                self.clear_focus();
                focus_path = self.resolve_focus_path(&navigation_event);
            }
            if let Some(index) = focus_path {
                self.set_focused(Some(index));
            }
        }
        KeyDispatch::Ignored
    }
    fn resolve_focus_path(&self, event: &FocusNavigationEvent) -> Option<usize> {
        match event {
            FocusNavigationEvent::TabNavigation { forward } => self.handle_tab_navigation(*forward),
            FocusNavigationEvent::ArrowNavigation { direction } => {
                self.handle_arrow_navigation(*direction)
            }
            FocusNavigationEvent::InitialFocus => None,
        }
    }
    fn handle_tab_navigation(&self, forward: bool) -> Option<usize> {
        focus::resolve_tab(&self.widgets, self.focused_widget(), forward)
    }
    fn handle_arrow_navigation(&self, direction: ScreenDirection) -> Option<usize> {
        let Some(focused) = self.focused_widget() else {
            let border = focus::java_border(
                ScreenRectangle::new(0, 0, self.width, self.height),
                direction.opposite(),
            );
            return focus::resolve_arrow_from(&self.widgets, border, direction);
        };
        focus::resolve_arrow(&self.widgets, focused, direction, None)
    }
}
fn is_terminal(priority: NarrationPriority) -> bool {
    matches!(priority, NarrationPriority::Focused)
}
fn find_narratable_widget_indices(
    sorted: &[usize],
    narratables: &[&dyn Widget],
    last: Option<usize>,
) -> Option<usize> {
    let mut result: Option<usize> = None;
    let mut low_priority: Option<usize> = None;
    for index in sorted.iter().copied() {
        let priority = narratables[index].narration_priority();
        if is_terminal(priority) {
            if Some(index) != last {
                return Some(index);
            }
            low_priority = Some(index);
        } else if priority
            > result.map_or(NarrationPriority::None, |position| {
                narratables[position].narration_priority()
            })
        {
            result = Some(index);
        }
    }
    result.or(low_priority)
}
struct NarrationCollector {
    generation: u32,
    entries: BTreeMap<NarratedElementType, NarrationEntry>,
    last_text: Option<String>,
}
struct NarrationEntry {
    contents: Vec<String>,
    generation: u32,
    already_narrated: bool,
}
impl NarrationEntry {
    fn new() -> Self {
        Self {
            contents: Vec::new(),
            generation: 0,
            already_narrated: false,
        }
    }
    fn update(&mut self, generation: u32, contents: Vec<String>) {
        if self.contents != contents {
            self.contents = contents;
            self.already_narrated = false;
        } else if self.generation + 1 != generation {
            self.already_narrated = false;
        }
        self.generation = generation;
    }
}
impl NarrationCollector {
    fn new() -> Self {
        Self {
            generation: 0,
            entries: BTreeMap::new(),
            last_text: None,
        }
    }
    fn update(&mut self, output: &NarrationElementOutput, force: bool) -> String {
        self.generation = self.generation.wrapping_add(1);
        for element_type in NARRATED_ELEMENT_TYPES {
            let contents = output.get(element_type).to_vec();
            self.entries
                .entry(element_type)
                .or_insert_with(NarrationEntry::new)
                .update(self.generation, contents);
        }
        let mut narration = String::new();
        for entry in self.entries.values_mut() {
            if entry.generation != self.generation {
                continue;
            }
            if !force && entry.already_narrated {
                continue;
            }
            for line in &entry.contents {
                if !narration.is_empty() {
                    narration.push_str(". ");
                }
                narration.push_str(line);
            }
            entry.already_narrated = true;
        }
        narration
    }
    fn set_last_text(&mut self, text: String) {
        self.last_text = Some(text);
    }
    fn take_last_text(&mut self) -> Option<String> {
        self.last_text.take()
    }
}
pub struct ScreenStack {
    screens: Vec<Box<dyn Screen>>,
    width: i32,
    height: i32,
}
impl ScreenStack {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            screens: Vec::new(),
            width,
            height,
        }
    }
    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
    pub fn set_size(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
        if let Some(top) = self.top_mut() {
            screen_resize(top, width, height);
        }
    }
    pub fn len(&self) -> usize {
        self.screens.len()
    }
    pub fn is_empty(&self) -> bool {
        self.screens.is_empty()
    }
    pub fn top(&self) -> Option<&dyn Screen> {
        self.screens.last().map(|screen| &**screen)
    }
    pub fn top_base(&self) -> Option<&ScreenBase> {
        self.screens.last().map(|screen| screen.base_ref())
    }
    pub fn top_mut(&mut self) -> Option<&mut (dyn Screen + '_)> {
        let screen = self.screens.last_mut()?;
        Some(&mut **screen)
    }
    pub fn iter(&self) -> impl Iterator<Item = &dyn Screen> + '_ {
        self.screens.iter().map(|screen| screen.as_ref())
    }
    pub fn replace(&mut self, screen: Box<dyn Screen>) -> Option<Box<dyn Screen>> {
        let replaced = self.pop();
        self.push(screen);
        replaced
    }
    pub fn push(&mut self, screen: Box<dyn Screen>) {
        if let Some(previous) = self.top_mut() {
            screen_removed(previous);
        }
        self.screens.push(screen);
        self.activate_top();
    }
    pub fn pop(&mut self) -> Option<Box<dyn Screen>> {
        let mut removed = self.screens.pop();
        if let Some(screen) = removed.as_mut() {
            screen_removed(&mut **screen);
        }
        self.activate_top();
        removed
    }
    pub fn close(&mut self) -> bool {
        let requested = self
            .top_mut()
            .map(|top| top.base().take_close_request())
            .unwrap_or(false);
        if requested {
            self.pop();
        }
        requested
    }
    pub fn clear(&mut self) {
        while let Some(mut screen) = self.screens.pop() {
            screen_removed(&mut *screen);
        }
    }
    pub fn tick_top(&mut self) {
        if let Some(top) = self.top_mut() {
            screen_tick(top);
        }
    }
    pub fn tick_all(&mut self) {
        for screen in &mut self.screens {
            screen_tick(&mut **screen);
        }
    }
    pub fn render_top(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) -> bool {
        if let Some(top) = self.top_mut() {
            screen_extract_render_state_with_tooltip_and_subtitles(
                &mut *top,
                graphics,
                mouse_x,
                mouse_y,
                partial_tick,
            );
            true
        } else {
            false
        }
    }
    fn activate_top(&mut self) {
        let (width, height) = (self.width, self.height);
        if let Some(top) = self.top_mut() {
            screen_added(top);
            screen_init(top, width, height);
        }
    }
    pub fn handle_delayed_narration(&mut self) -> bool {
        self.top_mut()
            .map(|top| top.handle_delayed_narration())
            .unwrap_or(false)
    }
    pub fn key_pressed(&mut self, event: KeyEvent) -> Option<bool> {
        self.top_mut().map(|top| top.key_pressed(event))
    }
    pub fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> Option<bool> {
        self.top_mut()
            .map(|top| top.mouse_clicked(event, double_click))
    }
    pub fn mouse_released(&mut self, event: MouseButtonEvent) -> Option<bool> {
        self.top_mut().map(|top| top.mouse_released(event))
    }
    pub fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> Option<bool> {
        self.top_mut().map(|top| top.mouse_dragged(event, dx, dy))
    }
    pub fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> Option<bool> {
        self.top_mut()
            .map(|top| top.mouse_scrolled(x, y, scroll_x, scroll_y))
    }
    pub fn take_pending_click_sounds(&mut self) -> Vec<crate::components::WidgetClickSound> {
        let mut sounds = Vec::new();
        for screen in self.screens.iter_mut().rev() {
            if let Some(sound) = screen.base().take_pending_click_sound() {
                sounds.push(sound);
            }
        }
        sounds
    }
}
