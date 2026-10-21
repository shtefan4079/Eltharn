use crate::components::text_widget::{MultiLineTextWidget, StringWidgetBuilder};
use crate::components::{BIG_WIDTH, Button, DEFAULT_HEIGHT, Widget, click_sound_sink};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::{FrameLayout, LayoutElement, LinearLayout};
use crate::framework::navigation::ScreenRectangle;
use crate::framework::screen::{Screen, ScreenBase, screen_key_pressed, screen_mouse_clicked};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::Component;
use std::sync::Arc;
const TEXT_CELL_PADDING: i32 = 10;
const BUTTON_CELL_PADDING: i32 = 2;
const REASON_WIDTH_MARGIN: i32 = 50;
const REASON_MIN_WIDTH: i32 = 100;
const NARRATION_SEPARATOR: &str = ". ";
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisconnectedAction {
    Back,
    OpenLink(String),
    OpenPath(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ButtonRow {
    BugReport,
    Report,
    Back,
}
pub struct DisconnectedScreen {
    base: ScreenBase,
    pub in_world: bool,
    pub title: String,
    pub reason: String,
    pub button_text: String,
    pub bug_report_link: Option<String>,
    pub report_path: Option<String>,
    pub allows_multiplayer: bool,
    layout: LinearLayout,
    buttons: Vec<(usize, ButtonRow)>,
    pending_actions: Vec<DisconnectedAction>,
    font: Arc<dyn crate::FontWidthMeasurer>,
}
impl DisconnectedScreen {
    pub fn new(
        in_world: bool,
        title: String,
        reason: String,
        button_text: Option<String>,
        bug_report_link: Option<String>,
        report_path: Option<String>,
        allows_multiplayer: bool,
        font: Arc<dyn crate::FontWidthMeasurer>,
    ) -> Self {
        let button_text = if allows_multiplayer {
            button_text.unwrap_or_else(|| "gui.toMenu".to_string())
        } else {
            "gui.toTitle".to_string()
        };
        let narration = format!(
            "{}{}{}",
            crate::translate(&title),
            NARRATION_SEPARATOR,
            crate::translate(&reason)
        );
        let mut base = ScreenBase::new(Component::literal(&narration));
        base.set_close_on_esc(false);
        Self {
            base,
            in_world,
            title,
            reason,
            button_text,
            bug_report_link,
            report_path,
            allows_multiplayer,
            layout: LinearLayout::vertical(),
            buttons: Vec::new(),
            pending_actions: Vec::new(),
            font,
        }
    }
    pub fn take_action(&mut self) -> Option<DisconnectedAction> {
        if self.pending_actions.is_empty() {
            None
        } else {
            Some(self.pending_actions.remove(0))
        }
    }
    fn button(&self, message: String, action: DisconnectedAction) -> Button<DisconnectedAction> {
        Button::<DisconnectedAction>::builder(Component::literal(&message))
            .bounds(0, 0, BIG_WIDTH, DEFAULT_HEIGHT)
            .with_action(action)
            .with_font(self.font.clone())
            .build()
    }
    fn add_layout_child(&mut self, widget: impl Widget + 'static) -> usize {
        let index = self.base.widgets_len();
        self.layout
            .add_handle(index, widget.width(), widget.height());
        self.base.add_widget(widget)
    }
    fn add_layout_button(&mut self, button: Button<DisconnectedAction>, row: ButtonRow) {
        let index = self.add_layout_child(button);
        self.buttons.push((index, row));
    }
    fn row_of(&self, index: usize) -> Option<ButtonRow> {
        self.buttons
            .iter()
            .find(|(widget_index, _)| *widget_index == index)
            .map(|(_, row)| *row)
    }
    fn action_of(&self, row: ButtonRow) -> Option<DisconnectedAction> {
        match row {
            ButtonRow::BugReport => self
                .bug_report_link
                .clone()
                .map(DisconnectedAction::OpenLink),
            ButtonRow::Report => self.report_path.clone().map(DisconnectedAction::OpenPath),
            ButtonRow::Back => Some(DisconnectedAction::Back),
        }
    }
    fn arrange_layout(&mut self) {
        let Self { base, layout, .. } = self;
        let mut children = base
            .widgets_mut()
            .iter_mut()
            .enumerate()
            .map(|(handle, widget)| {
                let element: &mut dyn LayoutElement = &mut **widget;
                (handle, element)
            });
        layout.arrange_widgets(&mut children);
    }
}
impl crate::framework::screen::Screen for DisconnectedScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        let font = self.font.clone();
        let width = self.base.width;
        self.layout
            .default_cell_settings()
            .align_horizontally_center();
        self.layout
            .default_cell_settings()
            .padding(TEXT_CELL_PADDING);
        let title = crate::translate(&self.title);
        self.add_layout_child(
            StringWidgetBuilder::new(Component::literal(&title))
                .with_font(font.clone())
                .build(),
        );
        let mut reason =
            MultiLineTextWidget::from_message(Component::literal(&crate::translate(&self.reason)))
                .with_font(font.clone());
        reason.set_max_width(reason_max_width(width));
        reason.set_centered(true);
        self.add_layout_child(reason);
        self.layout
            .default_cell_settings()
            .padding(BUTTON_CELL_PADDING);
        if let Some(link) = self.bug_report_link.clone() {
            let button = self.button(
                crate::translate("gui.report_to_server"),
                DisconnectedAction::OpenLink(link),
            );
            self.add_layout_button(button, ButtonRow::BugReport);
        }
        if let Some(path) = self.report_path.clone() {
            let button = self.button(
                crate::translate("gui.open_report_dir"),
                DisconnectedAction::OpenPath(path),
            );
            self.add_layout_button(button, ButtonRow::Report);
        }
        let message = if self.allows_multiplayer {
            crate::translate(&self.button_text)
        } else {
            crate::translate("gui.toTitle")
        };
        let button = self.button(message, DisconnectedAction::Back);
        self.add_layout_button(button, ButtonRow::Back);
        self.reposition_elements();
    }
    fn reposition_elements(&mut self) {
        self.arrange_layout();
        let rectangle = ScreenRectangle::new(0, 0, self.base.width, self.base.height);
        FrameLayout::center_in_rect(&mut self.layout, rectangle);
        self.arrange_layout();
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if screen_key_pressed(self, event) {
            self.press_focused_button(event, false);
            true
        } else {
            self.press_focused_button(event, true)
        }
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        let target = if event.button() == 0 {
            self.base_ref().child_at(event.x, event.y)
        } else {
            None
        };
        let consumed = screen_mouse_clicked(self, event, double_click);
        if let Some(row) = target.and_then(|index| self.row_of(index)) {
            if let Some(action) = self.action_of(row) {
                self.pending_actions.push(action);
            }
        }
        consumed
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        self.base.extract_background_layers(graphics, false);
        self.base
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
    }
    fn should_close_on_esc(&self) -> bool {
        false
    }
}
impl DisconnectedScreen {
    fn press_focused_button(&mut self, event: KeyEvent, sound: bool) -> bool {
        if !event.is_selection() {
            return false;
        }
        let Some(focused) = self.base_ref().focused_index() else {
            return false;
        };
        let Some(row) = self.row_of(focused) else {
            return false;
        };
        if sound {
            click_sound_sink::record_button_click();
        }
        if let Some(action) = self.action_of(row) {
            self.pending_actions.push(action);
        }
        true
    }
}
fn reason_max_width(width: i32) -> i32 {
    (width - REASON_WIDTH_MARGIN).max(REASON_MIN_WIDTH)
}
