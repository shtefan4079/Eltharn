use crate::FontWidthMeasurer;
use crate::components::checkbox::Checkbox;
use crate::components::scroll_area::{AbstractScrollArea, ScrollbarSettings};
use crate::components::{
    Button, GuiEventListener, Renderable, TabOrderedElement, Widget, WidgetClickSound,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{NarratableEntry, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{
    ComponentPath, FocusNavigationEvent, ScreenDirection, ScreenRectangle,
};
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{CharacterEvent, KeyEvent, MouseButtonEvent, PreeditEvent};
use crate::wrap_text;
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
pub const TITLE_Y: i32 = 8;
pub const DESCRIPTION_TOP: i32 = 21;
pub const LINE_HEIGHT: i32 = 9;
pub const HEADER_BUTTON_HEIGHT: i32 = 20;
pub const FOOTER_BUTTON_WIDTH: i32 = 150;
pub const FOOTER_BUTTON_GAP: i32 = 8;
pub const FOOTER_BUTTON_HEIGHT: i32 = 20;
pub const FOOTER_HEIGHT_WITH_CONSENT: i32 = 50;
pub const FOOTER_HEIGHT_WITHOUT_CONSENT: i32 = 33;
pub const AREA_SIDE_MARGIN: i32 = 20;
pub const CONTENT_PADDING: i32 = 4;
pub const SCROLL_RATE: i32 = 9;
const WHITE: i32 = 0xFFFF_FFFFu32 as i32;
const GRAY: i32 = 0xFFAAAAAAu32 as i32;
const TITLE_COLOR: i32 = 0xFFFF_FFFFu32 as i32;
const OPTIONAL_DISABLED_COLOR: i32 = 0xFFAAAAAAu32 as i32;
const GLOBAL_PROPERTIES: &[&str] = &[
    "user_id",
    "client_id",
    "minecraft_session_id",
    "game_version",
    "operating_system",
    "platform",
    "client_modded",
    "launcher_name",
    "event_timestamp_utc",
    "opt_in",
];
const WORLD_SESSION_PROPERTIES: &[&str] = &[
    "user_id",
    "client_id",
    "minecraft_session_id",
    "game_version",
    "operating_system",
    "platform",
    "client_modded",
    "launcher_name",
    "event_timestamp_utc",
    "opt_in",
    "world_session_id",
    "server_modded",
    "server_type",
    "server_session_id",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelemetryPropertyScope {
    Global,
    WorldSession,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TelemetryEventDescriptor {
    pub id: &'static str,
    pub optional: bool,
    pub property_scope: TelemetryPropertyScope,
    pub extra_properties: &'static [&'static str],
}
impl TelemetryEventDescriptor {
    pub fn title_key(self) -> String {
        format!("telemetry.event.{}.title", self.id)
    }
    pub fn description_key(self) -> String {
        format!("telemetry.event.{}.description", self.id)
    }
    pub fn properties(self) -> Vec<&'static str> {
        let common = match self.property_scope {
            TelemetryPropertyScope::Global => GLOBAL_PROPERTIES,
            TelemetryPropertyScope::WorldSession => WORLD_SESSION_PROPERTIES,
        };
        common
            .iter()
            .copied()
            .chain(self.extra_properties.iter().copied())
            .collect()
    }
}
pub const TELEMETRY_EVENTS: [TelemetryEventDescriptor; 7] = [
    TelemetryEventDescriptor {
        id: "graphics_capabilities",
        optional: false,
        property_scope: TelemetryPropertyScope::Global,
        extra_properties: &[
            "backend_name",
            "backend_failure_message",
            "backend_failure_reason",
            "backend_failure_missing_capabilities",
        ],
    },
    TelemetryEventDescriptor {
        id: "world_loaded",
        optional: false,
        property_scope: TelemetryPropertyScope::WorldSession,
        extra_properties: &["game_mode", "realms_map_content"],
    },
    TelemetryEventDescriptor {
        id: "world_unloaded",
        optional: false,
        property_scope: TelemetryPropertyScope::WorldSession,
        extra_properties: &["seconds_since_load", "ticks_since_load"],
    },
    TelemetryEventDescriptor {
        id: "performance_metrics",
        optional: true,
        property_scope: TelemetryPropertyScope::WorldSession,
        extra_properties: &[
            "frame_rate_samples",
            "render_time_samples",
            "used_memory_samples",
            "number_of_samples",
            "render_distance",
            "dedicated_memory_kb",
        ],
    },
    TelemetryEventDescriptor {
        id: "world_load_times",
        optional: true,
        property_scope: TelemetryPropertyScope::WorldSession,
        extra_properties: &["world_load_time_ms", "new_world"],
    },
    TelemetryEventDescriptor {
        id: "advancement_made",
        optional: true,
        property_scope: TelemetryPropertyScope::WorldSession,
        extra_properties: &["advancement_id", "advancement_game_time"],
    },
    TelemetryEventDescriptor {
        id: "game_load_times",
        optional: true,
        property_scope: TelemetryPropertyScope::Global,
        extra_properties: &[
            "load_time_total_time_ms",
            "load_time_pre_window_ms",
            "load_time_bootstrap_ms",
            "load_time_loading_overlay_ms",
        ],
    },
];
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TelemetryInfoCapabilities {
    pub optional_telemetry_available: bool,
    pub telemetry_log_directory_available: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TelemetryInfoAction {
    Done,
    OptionalTelemetryConsentChanged(bool),
    PrivacyStatementRequested,
    FeedbackRequested,
    TelemetryLogDirectoryRequested,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ButtonRequest {
    PrivacyStatement,
    Feedback,
    ViewData,
    Done,
}
struct SharedWidget<W: Widget> {
    inner: Rc<RefCell<W>>,
}
impl<W: Widget> SharedWidget<W> {
    fn new(inner: Rc<RefCell<W>>) -> Self {
        Self { inner }
    }
}
impl<W: Widget> Renderable for SharedWidget<W> {
    fn extract_render_state(
        &mut self,
        _graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
    }
}
impl<W: Widget> TabOrderedElement for SharedWidget<W> {
    fn tab_order_group(&self) -> i32 {
        self.inner.borrow().tab_order_group()
    }
}
impl<W: Widget> GuiEventListener for SharedWidget<W> {
    fn mouse_moved(&mut self, x: f64, y: f64) {
        self.inner.borrow_mut().mouse_moved(x, y);
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.inner.borrow_mut().mouse_clicked(event, double_click)
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.inner.borrow().play_down_sound()
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        self.inner.borrow_mut().mouse_released(event)
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        self.inner.borrow_mut().mouse_dragged(event, dx, dy)
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        self.inner
            .borrow_mut()
            .mouse_scrolled(x, y, scroll_x, scroll_y)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        self.inner.borrow_mut().key_pressed(event)
    }
    fn key_released(&mut self, event: KeyEvent) -> bool {
        self.inner.borrow_mut().key_released(event)
    }
    fn char_typed(&mut self, event: CharacterEvent) -> bool {
        self.inner.borrow_mut().char_typed(event)
    }
    fn preedit_updated(&mut self, event: Option<PreeditEvent>) -> bool {
        self.inner.borrow_mut().preedit_updated(event)
    }
    fn next_focus_path(&self, event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.inner.borrow().next_focus_path(event)
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.inner.borrow().is_mouse_over(mouse_x, mouse_y)
    }
    fn set_focused(&mut self, focused: bool) {
        self.inner.borrow_mut().set_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.inner.borrow().is_focused()
    }
    fn should_take_focus_after_interaction(&self) -> bool {
        self.inner.borrow().should_take_focus_after_interaction()
    }
    fn rectangle(&self) -> ScreenRectangle {
        GuiEventListener::rectangle(&*self.inner.borrow())
    }
    fn border_for_arrow_navigation(&self, opposite: ScreenDirection) -> ScreenRectangle {
        self.inner.borrow().border_for_arrow_navigation(opposite)
    }
}
impl<W: Widget> NarratableEntry for SharedWidget<W> {
    fn narration_priority(&self) -> NarrationPriority {
        self.inner.borrow().narration_priority()
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        self.inner.borrow().update_narration(output);
    }
}
impl<W: Widget> LayoutElement for SharedWidget<W> {
    fn set_x(&mut self, x: i32) {
        self.inner.borrow_mut().set_x(x);
    }
    fn set_y(&mut self, y: i32) {
        self.inner.borrow_mut().set_y(y);
    }
    fn x(&self) -> i32 {
        self.inner.borrow().x()
    }
    fn y(&self) -> i32 {
        self.inner.borrow().y()
    }
    fn width(&self) -> i32 {
        self.inner.borrow().width()
    }
    fn height(&self) -> i32 {
        self.inner.borrow().height()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct TextLine {
    text: String,
    y: i32,
    centered: bool,
    color: i32,
    underline: bool,
}
pub struct TelemetryInfoScreen {
    base: ScreenBase,
    in_world: bool,
    capabilities: TelemetryInfoCapabilities,
    optional_telemetry_opt_in: bool,
    font: Arc<dyn FontWidthMeasurer>,
    area: Rc<RefCell<AbstractScrollArea>>,
    buttons: Vec<(ButtonRequest, Rc<RefCell<Button<ButtonRequest>>>)>,
    checkbox: Option<Rc<RefCell<Checkbox>>>,
    lines: Vec<TextLine>,
    content_height: i32,
    header_height: i32,
    footer_height: i32,
    pending_action: Option<TelemetryInfoAction>,
    last_width: i32,
    last_height: i32,
}
impl TelemetryInfoScreen {
    pub fn new(
        in_world: bool,
        capabilities: TelemetryInfoCapabilities,
        optional_telemetry_opt_in: bool,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let optional_telemetry_opt_in =
            capabilities.optional_telemetry_available && optional_telemetry_opt_in;
        let title = translated_or("telemetry_info.screen.title", "Telemetry Data Collection");
        let area = AbstractScrollArea::new(
            0,
            0,
            0,
            0,
            Component::literal(""),
            ScrollbarSettings::default_settings(SCROLL_RATE),
        );
        let mut screen = Self {
            base: ScreenBase::new(Component::literal(&title)),
            in_world,
            capabilities,
            optional_telemetry_opt_in,
            font: Arc::clone(&font),
            area: Rc::new(RefCell::new(area)),
            buttons: Vec::new(),
            checkbox: None,
            lines: Vec::new(),
            content_height: 0,
            header_height: 0,
            footer_height: 0,
            pending_action: None,
            last_width: 0,
            last_height: 0,
        };
        screen.create_controls(font);
        screen
    }
    fn create_controls(&mut self, font: Arc<dyn FontWidthMeasurer>) {
        let labels = [
            (
                ButtonRequest::PrivacyStatement,
                translated_or(
                    "telemetry_info.button.privacy_statement",
                    "Privacy Statement",
                ),
                true,
            ),
            (
                ButtonRequest::Feedback,
                translated_or("telemetry_info.button.give_feedback", "Give Feedback"),
                true,
            ),
            (
                ButtonRequest::ViewData,
                if self.capabilities.telemetry_log_directory_available {
                    translated_or("telemetry_info.button.show_data", "View My Data")
                } else {
                    format!(
                        "{} ({})",
                        translated_or("telemetry_info.button.show_data", "View My Data"),
                        translated_or("eltharn.settings.option_unavailable", "Unavailable")
                    )
                },
                self.capabilities.telemetry_log_directory_available,
            ),
            (ButtonRequest::Done, translated_or("gui.done", "Done"), true),
        ];
        self.buttons = labels
            .into_iter()
            .map(|(request, label, active)| {
                let mut button = Button::<ButtonRequest>::builder(Component::literal(&label))
                    .bounds(0, 0, FOOTER_BUTTON_WIDTH, FOOTER_BUTTON_HEIGHT)
                    .with_action(request)
                    .with_font(Arc::clone(&font))
                    .build();
                button.base.active = active;
                (request, Rc::new(RefCell::new(button)))
            })
            .collect();
        self.checkbox = self.capabilities.optional_telemetry_available.then(|| {
            Rc::new(RefCell::new(
                Checkbox::builder(Component::literal(&translated_or(
                    "telemetry_info.opt_in.description",
                    "I consent to sending optional telemetry data",
                )))
                .with_font(Arc::clone(&font))
                .selected(self.optional_telemetry_opt_in)
                .build(),
            ))
        });
    }
    fn description() -> String {
        translated_or(
            "eltharn.telemetry.catalog_description",
            "This is the telemetry event catalog declared by Minecraft Java 26.2. It lists field types, not collected values; Eltharn collection and transmission are controlled by the client service.",
        )
    }
    fn header_description_lines(&self, width: i32) -> Vec<String> {
        wrap_text(
            &Self::description(),
            (width - 16).max(1) as f32,
            self.font.as_ref(),
        )
    }
    fn rebuild_content(&mut self, width: i32) {
        let content_width = (width - AREA_SIDE_MARGIN * 2 - 18).max(1);
        let line_width = (content_width - 64).max(1) as f32;
        let mut lines = Vec::new();
        let mut y = 0;
        for (index, event) in TELEMETRY_EVENTS.iter().copied().enumerate() {
            let disabled = event.optional && !self.optional_telemetry_opt_in;
            let title = translated_or(&event.title_key(), event_title_fallback(event.id));
            let title_key = if event.optional {
                if disabled {
                    "telemetry.event.optional.disabled"
                } else {
                    "telemetry.event.optional"
                }
            } else {
                "telemetry.event.required"
            };
            let title_fallback = if event.optional {
                if disabled {
                    "%s (Optional) - Disabled"
                } else {
                    "%s (Optional)"
                }
            } else {
                "%s (Required)"
            };
            let header = format_template(title_key, title_fallback, &title);
            let header_color = if disabled {
                OPTIONAL_DISABLED_COLOR
            } else {
                WHITE
            };
            y = Self::append_wrapped(
                &mut lines,
                &header,
                line_width,
                y,
                true,
                header_color,
                false,
                self.font.as_ref(),
            );
            let description = translated_or(
                &event.description_key(),
                event_description_fallback(event.id),
            );
            y = Self::append_wrapped(
                &mut lines,
                &description,
                line_width,
                y,
                true,
                GRAY,
                false,
                self.font.as_ref(),
            );
            y += 4;
            let property_title = translated_or("telemetry_info.property_title", "Included Data");
            y = Self::append_wrapped(
                &mut lines,
                &property_title,
                content_width as f32,
                y,
                false,
                if disabled {
                    OPTIONAL_DISABLED_COLOR
                } else {
                    WHITE
                },
                true,
                self.font.as_ref(),
            );
            for property in event.properties() {
                let key = format!("telemetry.property.{property}.title");
                let fallback = property_title_fallback(property);
                y = Self::append_wrapped(
                    &mut lines,
                    &translated_or(&key, fallback),
                    content_width as f32,
                    y,
                    false,
                    if disabled {
                        OPTIONAL_DISABLED_COLOR
                    } else {
                        WHITE
                    },
                    false,
                    self.font.as_ref(),
                );
            }
            if index + 1 < TELEMETRY_EVENTS.len() {
                y += 9;
            }
        }
        self.lines = lines;
        self.content_height = y;
        let mut area = self.area.borrow_mut();
        area.content_height = self.content_height;
        area.clamp_scroll(
            self.base
                .height
                .saturating_sub(self.header_height + self.footer_height),
        );
    }
    #[allow(clippy::too_many_arguments)]
    fn append_wrapped(
        lines: &mut Vec<TextLine>,
        text: &str,
        max_width: f32,
        start_y: i32,
        centered: bool,
        color: i32,
        underline: bool,
        font: &dyn FontWidthMeasurer,
    ) -> i32 {
        let wrapped = wrap_text(text, max_width, font);
        let mut y = start_y;
        for text in wrapped {
            lines.push(TextLine {
                text,
                y,
                centered,
                color,
                underline,
            });
            y += LINE_HEIGHT;
        }
        y
    }
    fn ensure_layout(&mut self, width: i32, height: i32) {
        if self.last_width == width && self.last_height == height && !self.buttons.is_empty() {
            return;
        }
        self.last_width = width;
        self.last_height = height;
        self.base.clear_widgets();
        let description_lines = self.header_description_lines(width);
        let description_bottom = DESCRIPTION_TOP + description_lines.len() as i32 * LINE_HEIGHT;
        let links_y = description_bottom + 4;
        self.header_height = links_y + HEADER_BUTTON_HEIGHT + 8;
        self.footer_height = if self.checkbox.is_some() {
            FOOTER_HEIGHT_WITH_CONSENT
        } else {
            FOOTER_HEIGHT_WITHOUT_CONSENT
        };
        let viewport_height = (height - self.header_height - self.footer_height).max(0);
        let area_x = AREA_SIDE_MARGIN;
        let area_width = (width - AREA_SIDE_MARGIN * 2).max(0);
        {
            let mut area = self.area.borrow_mut();
            area.base.set_x(area_x);
            area.base.set_y(self.header_height);
            area.base.set_width(area_width);
            area.base.set_height(viewport_height);
        }
        self.rebuild_content(width);
        self.area.borrow_mut().clamp_scroll(viewport_height);
        self.base
            .add_widget(SharedWidget::new(Rc::clone(&self.area)));
        let button_width = FOOTER_BUTTON_WIDTH.min((width - 16).max(1));
        let button_x = (width - button_width * 2 - FOOTER_BUTTON_GAP) / 2;
        let top_button_x = (width - button_width * 2 - FOOTER_BUTTON_GAP) / 2;
        for (request, button) in &self.buttons {
            let column = match request {
                ButtonRequest::PrivacyStatement => 0,
                ButtonRequest::Feedback => 1,
                ButtonRequest::ViewData => 0,
                ButtonRequest::Done => 1,
            };
            let y = match request {
                ButtonRequest::PrivacyStatement | ButtonRequest::Feedback => links_y,
                ButtonRequest::ViewData | ButtonRequest::Done => height - 26,
            };
            let x = if matches!(
                request,
                ButtonRequest::PrivacyStatement | ButtonRequest::Feedback
            ) {
                top_button_x + column * (button_width + FOOTER_BUTTON_GAP)
            } else {
                button_x + column * (button_width + FOOTER_BUTTON_GAP)
            };
            let shared_button = Rc::clone(button);
            {
                let mut button = button.borrow_mut();
                button.set_position(x, y);
                button.base.set_width(button_width);
                button.base.active = *request != ButtonRequest::ViewData
                    || self.capabilities.telemetry_log_directory_available;
            }
            self.base.add_widget(SharedWidget::new(shared_button));
        }
        if let Some(checkbox) = &self.checkbox {
            let shared_checkbox = Rc::clone(checkbox);
            {
                let mut checkbox = checkbox.borrow_mut();
                checkbox.set_max_width((width - 40).max(1));
                let checkbox_x = (width - checkbox.width()) / 2;
                let checkbox_y = height - self.footer_height + 2;
                checkbox.set_position(checkbox_x, checkbox_y);
                checkbox.base.active = self.capabilities.optional_telemetry_available;
            }
            self.base.add_widget(SharedWidget::new(shared_checkbox));
        }
    }
    fn button(&self, request: ButtonRequest) -> Option<&Rc<RefCell<Button<ButtonRequest>>>> {
        self.buttons
            .iter()
            .find(|(candidate, _)| *candidate == request)
            .map(|(_, button)| button)
    }
    fn drain_requests(&mut self) {
        let requests: Vec<_> = self
            .buttons
            .iter()
            .filter_map(|(_, button)| button.borrow_mut().take_press_action())
            .collect();
        for request in requests {
            self.handle_button_request(request);
        }
        let consent_changed = self.checkbox.as_ref().and_then(|checkbox| {
            let mut checkbox = checkbox.borrow_mut();
            checkbox.take_value_changed().then_some(checkbox.selected())
        });
        if let Some(value) = consent_changed {
            self.optional_telemetry_opt_in = value;
            self.pending_action = Some(TelemetryInfoAction::OptionalTelemetryConsentChanged(value));
            self.rebuild_content(self.base.width);
        }
    }
    fn handle_button_request(&mut self, request: ButtonRequest) {
        self.pending_action = Some(match request {
            ButtonRequest::PrivacyStatement => TelemetryInfoAction::PrivacyStatementRequested,
            ButtonRequest::Feedback => TelemetryInfoAction::FeedbackRequested,
            ButtonRequest::ViewData if self.capabilities.telemetry_log_directory_available => {
                TelemetryInfoAction::TelemetryLogDirectoryRequested
            }
            ButtonRequest::ViewData => return,
            ButtonRequest::Done => TelemetryInfoAction::Done,
        });
    }
    fn viewport_height(&self) -> i32 {
        (self.base_ref().height - self.header_height - self.footer_height).max(0)
    }
    pub fn take_action(&mut self) -> Option<TelemetryInfoAction> {
        self.pending_action.take()
    }
    pub fn capabilities(&self) -> TelemetryInfoCapabilities {
        self.capabilities
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn optional_telemetry_opt_in(&self) -> bool {
        self.optional_telemetry_opt_in
    }
    pub fn optional_event_rows_disabled(&self) -> bool {
        !self.optional_telemetry_opt_in
    }
    pub fn event_descriptors(&self) -> &'static [TelemetryEventDescriptor] {
        &TELEMETRY_EVENTS
    }
    pub fn content_height(&self) -> i32 {
        self.content_height
    }
    pub fn header_height(&self) -> i32 {
        self.header_height
    }
    pub fn footer_height(&self) -> i32 {
        self.footer_height
    }
    pub fn scroll_amount(&self) -> f32 {
        self.area.borrow().scroll_amount
    }
    pub fn button_rectangle(&self, request: TelemetryInfoButton) -> Option<ScreenRectangle> {
        let request = match request {
            TelemetryInfoButton::PrivacyStatement => ButtonRequest::PrivacyStatement,
            TelemetryInfoButton::Feedback => ButtonRequest::Feedback,
            TelemetryInfoButton::ViewData => ButtonRequest::ViewData,
            TelemetryInfoButton::Done => ButtonRequest::Done,
        };
        self.button(request)
            .map(|button| LayoutElement::rectangle(&*button.borrow()))
    }
    pub fn checkbox_rectangle(&self) -> Option<ScreenRectangle> {
        self.checkbox
            .as_ref()
            .map(|checkbox| LayoutElement::rectangle(&*checkbox.borrow()))
    }
    fn render_header(&self, graphics: &mut GuiGraphicsExtractor, width: i32) {
        let title = translated_or("telemetry_info.screen.title", "Telemetry Data Collection");
        let title_width = self.font.measure_width(&title, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            title,
            (width - title_width) / 2,
            TITLE_Y,
            TITLE_COLOR,
        );
        let lines = self.header_description_lines(width);
        let mut y = DESCRIPTION_TOP;
        for line in lines {
            let line_width = self.font.measure_width(&line, 1.0).round() as i32;
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                line,
                (width - line_width) / 2,
                y,
                GRAY,
            );
            y += LINE_HEIGHT;
        }
    }
    fn render_event_catalog(&self, graphics: &mut GuiGraphicsExtractor, width: i32) {
        let mut area = self.area.borrow_mut();
        let area_x = area.base.x();
        let viewport_top = area.base.y();
        let viewport_bottom = viewport_top + area.base.height();
        let scroll = area.scroll_amount as i32;
        let left = area_x + CONTENT_PADDING;
        let right = area.base.x() + area.base.width() - CONTENT_PADDING;
        if viewport_bottom <= viewport_top || right <= left {
            return;
        }
        graphics.enable_scissor(
            area_x,
            viewport_top,
            area_x + area.base.width(),
            viewport_bottom,
        );
        for line in &self.lines {
            let y = viewport_top + line.y - scroll;
            if y + LINE_HEIGHT < viewport_top || y >= viewport_bottom {
                continue;
            }
            let line_width = self.font.measure_width(&line.text, 1.0).round() as i32;
            let x = if line.centered {
                (width - line_width) / 2
            } else {
                left
            };
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                line.text.clone(),
                x,
                y,
                line.color,
            );
            if line.underline {
                graphics.fill(
                    x,
                    y + LINE_HEIGHT - 1,
                    x + line_width,
                    y + LINE_HEIGHT,
                    line.color,
                );
            }
        }
        graphics.disable_scissor();
        area.extract_scrollbar(graphics);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelemetryInfoButton {
    PrivacyStatement,
    Feedback,
    ViewData,
    Done,
}
impl Screen for TelemetryInfoScreen {
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
            self.pending_action = Some(TelemetryInfoAction::Done);
            return true;
        }
        let handled = crate::framework::screen::screen_key_pressed(self, event);
        if handled {
            self.drain_requests();
        }
        handled
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        let viewport = self.viewport_height();
        let on_footer = event.y >= (self.base.height - self.footer_height) as f64;
        if !on_footer
            && self.area.borrow_mut().scrollbar_clicked_with_button(
                event.x,
                event.y,
                event.button(),
                viewport,
            )
        {
            return true;
        }
        let handled = crate::framework::screen::screen_mouse_clicked(self, event, double_click);
        if handled {
            self.drain_requests();
        }
        handled
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        let was_scrolling = self.area.borrow().is_scrolling();
        self.area.borrow_mut().scrollbar_released();
        crate::framework::screen::screen_mouse_released(self, event) || was_scrolling
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        if self.area.borrow().is_scrolling() {
            self.area
                .borrow_mut()
                .scrollbar_dragged(event.y, dy, self.viewport_height());
            return true;
        }
        crate::framework::screen::screen_mouse_dragged(self, event, dx, dy)
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, _scroll_x: f64, scroll_y: f64) -> bool {
        let area_rect = self.area.borrow().base.rectangle();
        if !area_rect.contains_point(x as i32, y as i32) {
            return false;
        }
        self.area
            .borrow_mut()
            .scroll(scroll_y as f32, self.viewport_height());
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
        self.base
            .extract_background_backdrop(graphics, self.in_world);
        graphics.draw_menu_background(self.in_world, width, height);
        self.render_header(graphics, width);
        self.render_event_catalog(graphics, width);
        graphics.draw_menu_background(self.in_world, width, self.header_height);
        graphics.pose_mut().push_pose();
        graphics
            .pose_mut()
            .translate(0.0, (height - self.footer_height) as f32);
        graphics.draw_menu_background(self.in_world, width, self.footer_height);
        graphics.pose_mut().pop_pose();
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/header_separator"),
            0,
            self.header_height - 2,
            width,
            2,
        );
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/footer_separator"),
            0,
            height - self.footer_height,
            width,
            2,
        );
        self.base
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
        self.drain_requests();
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
fn format_template(key: &str, fallback: &str, value: &str) -> String {
    translated_or(key, fallback).replacen("%s", value, 1)
}
fn event_title_fallback(id: &str) -> &'static str {
    match id {
        "graphics_capabilities" => "Graphics Capabilities",
        "world_loaded" => "World Loaded",
        "world_unloaded" => "World Unloaded",
        "performance_metrics" => "Performance Metrics",
        "world_load_times" => "World Load Times",
        "advancement_made" => "Advancement Made",
        "game_load_times" => "Game Load Times",
        _ => "Unknown Event",
    }
}
fn event_description_fallback(id: &str) -> &'static str {
    match id {
        "graphics_capabilities" => {
            "Describes the graphics device and supported rendering features."
        }
        "world_loaded" => {
            "Records the version, game mode, and modification status when a session begins. The matching session-end event provides its duration."
        }
        "world_unloaded" => {
            "Records elapsed seconds and ticks when a session ends, together with its matching session-start event."
        }
        "performance_metrics" => {
            "Summarizes runtime performance with the hardware, operating system, and game version needed to interpret the measurements."
        }
        "world_load_times" => {
            "Measures how long joining a world takes so load performance can be compared across versions."
        }
        "advancement_made" => {
            "Records when an advancement is earned and the surrounding progression context."
        }
        "game_load_times" => "Measures the duration of individual startup phases.",
        _ => "",
    }
}
fn property_title_fallback(id: &str) -> &'static str {
    match id {
        "user_id" => "User ID",
        "client_id" => "Client ID",
        "minecraft_session_id" => "Minecraft Session ID",
        "game_version" => "Game Version",
        "operating_system" => "Operating System",
        "platform" => "Platform",
        "client_modded" => "Client Modded",
        "launcher_name" => "Launcher Name",
        "event_timestamp_utc" => "Event Timestamp (UTC)",
        "opt_in" => "Opt-In",
        "world_session_id" => "World Session ID",
        "server_modded" => "Server Modded",
        "server_type" => "Server Type",
        "server_session_id" => "Server Session ID",
        "backend_name" => "Backend Name",
        "backend_failure_message" => "Backend Creation Failure Message",
        "backend_failure_reason" => "Backend Creation Failure Reason",
        "backend_failure_missing_capabilities" => "Backend Creation Failure Missing Capabilities",
        "game_mode" => "Game Mode",
        "realms_map_content" => "Realms Map Content (Minigame Name)",
        "seconds_since_load" => "Time Since Load (Seconds)",
        "ticks_since_load" => "Time Since Load (Ticks)",
        "frame_rate_samples" => "Frame Rate Samples (FPS)",
        "render_time_samples" => "Render Time Samples",
        "used_memory_samples" => "Used Random Access Memory",
        "number_of_samples" => "Sample Count",
        "render_distance" => "Render Distance",
        "dedicated_memory_kb" => "Dedicated Memory (kB)",
        "world_load_time_ms" => "World Load Time (Milliseconds)",
        "new_world" => "New World",
        "advancement_id" => "Advancement ID",
        "advancement_game_time" => "Game Time (Ticks)",
        "load_time_total_time_ms" => "Total Load Time (Milliseconds)",
        "load_time_pre_window_ms" => "Time Before Window Opens (Milliseconds)",
        "load_time_bootstrap_ms" => "Bootstrap Time (Milliseconds)",
        "load_time_loading_overlay_ms" => "Time in Loading Screen (Milliseconds)",
        _ => "Unknown Property",
    }
}
