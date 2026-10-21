use crate::FontWidthMeasurer;
use crate::components::edit_box::{Clipboard, EditBox, EditBoxDraw, EditBoxKey, EditBoxRenderPlan};
use crate::components::multiline_edit_box::MultilineEditBox;
use crate::components::{Button, GuiEventListener, Renderable, TabOrderedElement};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{NarratedElementType, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::framework::screen::{Screen, ScreenBase, screen_init, screen_key_pressed};
use crate::input::events::{CharacterEvent, CursorType, KeyEvent, MouseButtonEvent, PreeditEvent};
use crate::screens::confirm_screen::SharedButton;
use el_core::{Component, ResourceLocation};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
const WHITE: i32 = 0xFFFF_FFFFu32 as i32;
const MUTED: i32 = 0xFFAAAAAAu32 as i32;
const ROW_HEIGHT: i32 = 20;
const ROW_GAP: i32 = 4;
const BODY_TOP: i32 = 42;
const MAX_BODY_LINES: usize = 10;
const PAGE_ROW_COUNT: usize = 8;
#[derive(Clone, Debug, PartialEq)]
pub enum DialogClickAction {
    RunCommand {
        command: String,
        dynamic: bool,
    },
    OpenUrl(String),
    CopyToClipboard(String),
    SuggestCommand(String),
    Custom {
        id: String,
        payload: Option<Vec<u8>>,
        dynamic_inputs: bool,
    },
    OpenDialog(Box<ServerDialogModel>),
    LocalSignUpdate {
        pos: [i32; 3],
        is_front_text: bool,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct DialogMultilineOptions {
    pub max_lines: Option<usize>,
    pub height: i32,
}
#[derive(Clone, Debug, PartialEq)]
pub enum DialogInputControl {
    Text {
        key: String,
        label: String,
        initial: String,
        max_length: usize,
        width: i32,
        label_visible: bool,
        multiline: Option<DialogMultilineOptions>,
    },
    Boolean {
        key: String,
        label: String,
        value: bool,
        on_true: String,
        on_false: String,
    },
    NumberRange {
        key: String,
        label: String,
        start: f32,
        end: f32,
        value: f32,
        step: Option<f32>,
    },
    SingleOption {
        key: String,
        label: String,
        options: Vec<(String, String)>,
        selected: usize,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub enum DialogInputValue {
    Text(String),
    Boolean { value: bool, substitution: String },
    Number(f32),
    Option(String),
}
impl DialogInputValue {
    pub fn template_value(&self) -> String {
        match self {
            Self::Text(value) | Self::Option(value) => value.clone(),
            Self::Boolean { substitution, .. } => substitution.clone(),
            Self::Number(value) => format_dialog_number(*value),
        }
    }
}
fn format_dialog_number(value: f32) -> String {
    if value.is_finite() && value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct DialogButtonModel {
    pub label: String,
    pub width: i32,
    pub action: Option<DialogClickAction>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct DialogItemVisual {
    pub stack: el_core::item::ItemStack,
    pub width: i32,
    pub height: i32,
    pub show_decorations: bool,
    pub show_tooltip: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DialogAfterAction {
    #[default]
    Close,
    KeepOpen,
    WaitForResponse,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerDialogModel {
    pub title: String,
    pub body: Vec<String>,
    pub body_items: Vec<DialogItemVisual>,
    pub buttons: Vec<DialogButtonModel>,
    pub footer_buttons: Vec<DialogButtonModel>,
    pub cancel_action: Option<DialogClickAction>,
    pub inputs: Vec<DialogInputControl>,
    pub can_close_with_escape: bool,
    pub pause: bool,
    pub after_action: DialogAfterAction,
    pub columns: usize,
}
pub fn sign_editor_dialog_model(
    pos: [i32; 3],
    is_front_text: bool,
    initial_lines: [String; 4],
) -> ServerDialogModel {
    let action = DialogClickAction::LocalSignUpdate { pos, is_front_text };
    let inputs = initial_lines
        .into_iter()
        .enumerate()
        .map(|(index, initial)| DialogInputControl::Text {
            key: format!("line{index}"),
            label: format!("Line {}", index + 1),
            initial,
            max_length: 256,
            width: 240,
            label_visible: false,
            multiline: None,
        })
        .collect();
    ServerDialogModel {
        title: "Edit Sign".to_string(),
        body: Vec::new(),
        body_items: Vec::new(),
        buttons: vec![DialogButtonModel {
            label: "Done".to_string(),
            width: 200,
            action: Some(action.clone()),
        }],
        footer_buttons: Vec::new(),
        cancel_action: Some(action),
        inputs,
        can_close_with_escape: true,
        pause: false,
        after_action: DialogAfterAction::Close,
        columns: 1,
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerDialogRequest {
    pub action: Option<DialogClickAction>,
    pub after_action: DialogAfterAction,
    pub inputs: HashMap<String, DialogInputValue>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum ButtonRequest {
    Action(usize),
    Footer(usize),
    Input { index: usize, delta: i8 },
    Done,
    Previous,
    Next,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ServerDialogBackground {
    #[default]
    MenuDialogBase,
    InWorldDialogBase,
    TransparentSignInWorld,
}
pub struct ServerDialogScreen {
    base: ScreenBase,
    model: ServerDialogModel,
    background_route: ServerDialogBackground,
    font: Arc<dyn FontWidthMeasurer>,
    buttons: Vec<(ButtonRequest, Rc<RefCell<Button<ButtonRequest>>>)>,
    page: usize,
    nested_models: Vec<(
        ServerDialogModel,
        Vec<Option<Rc<RefCell<DialogTextBox>>>>,
        usize,
    )>,
    text_boxes: Vec<Option<Rc<RefCell<DialogTextBox>>>>,
    clipboard: Option<Rc<RefCell<dyn Clipboard>>>,
    clock_ms: Rc<Cell<u64>>,
    body_lines: Vec<String>,
    input_top: i32,
    input_rows: Vec<i32>,
    pending_request: Option<ServerDialogRequest>,
    page_count: usize,
    waiting_for_response: bool,
    waiting_ticks: u32,
}
impl ServerDialogScreen {
    pub fn new(model: ServerDialogModel, font: Arc<dyn FontWidthMeasurer>) -> Self {
        let title = model.title.clone();
        let text_boxes = model
            .inputs
            .iter()
            .map(|input| DialogTextBox::from_input(font.as_ref(), input))
            .collect();
        let mut base = ScreenBase::new(Component::literal(&title));
        base.set_close_on_esc(model.can_close_with_escape);
        base.set_pause_screen(model.pause);
        Self {
            base,
            model,
            background_route: ServerDialogBackground::default(),
            font,
            buttons: Vec::new(),
            page: 0,
            nested_models: Vec::new(),
            text_boxes,
            clipboard: None,
            clock_ms: Rc::new(Cell::new(0)),
            body_lines: Vec::new(),
            input_top: BODY_TOP,
            input_rows: Vec::new(),
            pending_request: None,
            page_count: 1,
            waiting_for_response: false,
            waiting_ticks: 0,
        }
    }
    pub fn model(&self) -> &ServerDialogModel {
        &self.model
    }
    pub fn set_background_route(&mut self, route: ServerDialogBackground) {
        self.background_route = route;
    }
    pub fn background_route(&self) -> ServerDialogBackground {
        self.background_route
    }
    pub fn set_clipboard(&mut self, clipboard: Rc<RefCell<dyn Clipboard>>) {
        self.clipboard = Some(clipboard);
        self.rebuild();
    }
    pub fn page(&self) -> usize {
        self.page
    }
    pub fn page_count(&self) -> usize {
        self.page_count
    }
    pub fn waiting_phase(&self) -> u32 {
        self.waiting_ticks / 20
    }
    pub fn begin_waiting_for_response(&mut self) {
        self.waiting_for_response = true;
        self.waiting_ticks = 0;
        self.page = 0;
        self.base.set_pause_screen(false);
        self.base.set_close_on_esc(false);
        self.rebuild();
    }
    pub fn button_rectangle(&self, visible_index: usize) -> Option<ScreenRectangle> {
        self.buttons.get(visible_index).map(|(_, button)| {
            crate::framework::layouts::LayoutElement::rectangle(&*button.borrow())
        })
    }
    pub fn take_request(&mut self) -> Option<ServerDialogRequest> {
        if let Some(request) = self.pending_request.take() {
            return self.handle_nested_request(request);
        }
        if self.waiting_for_response {
            if self.base.take_close_request() {
                return Some(ServerDialogRequest {
                    action: None,
                    after_action: DialogAfterAction::Close,
                    inputs: HashMap::new(),
                });
            }
            for (_, button) in &self.buttons {
                if button.borrow_mut().take_press_action().is_some() {
                    return Some(ServerDialogRequest {
                        action: None,
                        after_action: DialogAfterAction::Close,
                        inputs: HashMap::new(),
                    });
                }
            }
            return None;
        }
        if self.base.take_close_request() {
            self.pending_request = Some(ServerDialogRequest {
                action: self.model.cancel_action.clone(),
                after_action: self.model.after_action,
                inputs: self.current_inputs(),
            });
            return self
                .pending_request
                .take()
                .and_then(|request| self.handle_nested_request(request));
        }
        for (_, button) in &self.buttons {
            let Some(action) = button.borrow_mut().take_press_action() else {
                continue;
            };
            match action {
                ButtonRequest::Action(index) => {
                    let button = &self.model.buttons[index];
                    return self.handle_nested_request(ServerDialogRequest {
                        action: button.action.clone(),
                        after_action: self.model.after_action,
                        inputs: self.current_inputs(),
                    });
                }
                ButtonRequest::Footer(index) => {
                    let button = &self.model.footer_buttons[index];
                    return self.handle_nested_request(ServerDialogRequest {
                        action: button.action.clone(),
                        after_action: self.model.after_action,
                        inputs: self.current_inputs(),
                    });
                }
                ButtonRequest::Input { index, delta } => {
                    if let Some(input) = self.model.inputs.get_mut(index) {
                        change_dialog_input(input, delta);
                    }
                    self.rebuild();
                    return None;
                }
                ButtonRequest::Done => {
                    return self.handle_nested_request(ServerDialogRequest {
                        action: None,
                        after_action: DialogAfterAction::Close,
                        inputs: self.current_inputs(),
                    });
                }
                ButtonRequest::Previous => {
                    self.page = self.page.saturating_sub(1);
                    self.rebuild();
                    return None;
                }
                ButtonRequest::Next => {
                    self.page = (self.page + 1).min(self.page_count.saturating_sub(1));
                    self.rebuild();
                    return None;
                }
            }
        }
        None
    }
    fn handle_nested_request(
        &mut self,
        mut request: ServerDialogRequest,
    ) -> Option<ServerDialogRequest> {
        match request.action.take() {
            Some(DialogClickAction::OpenDialog(child)) => {
                self.nested_models.push((
                    self.model.clone(),
                    std::mem::take(&mut self.text_boxes),
                    self.page,
                ));
                self.model = *child;
                self.text_boxes = self.make_text_boxes();
                self.page = 0;
                self.waiting_for_response = false;
                self.base.set_pause_screen(self.model.pause);
                self.base.set_close_on_esc(self.model.can_close_with_escape);
                self.rebuild();
                return None;
            }
            action => request.action = action,
        }
        if request.after_action == DialogAfterAction::Close
            && let Some((previous, text_boxes, page)) = self.nested_models.pop()
        {
            self.model = previous;
            self.text_boxes = text_boxes;
            self.page = page;
            self.waiting_for_response = false;
            self.base.set_pause_screen(self.model.pause);
            self.base.set_close_on_esc(self.model.can_close_with_escape);
            self.rebuild();
            request.after_action = DialogAfterAction::KeepOpen;
            return request.action.is_some().then_some(request);
        }
        Some(request)
    }
    fn make_text_boxes(&self) -> Vec<Option<Rc<RefCell<DialogTextBox>>>> {
        self.model
            .inputs
            .iter()
            .map(|input| DialogTextBox::from_input(self.font.as_ref(), input))
            .collect()
    }
    fn current_inputs(&self) -> HashMap<String, DialogInputValue> {
        self.model
            .inputs
            .iter()
            .enumerate()
            .filter_map(|(index, input)| match input {
                DialogInputControl::Text { key, .. } => self
                    .text_boxes
                    .get(index)
                    .and_then(Option::as_ref)
                    .map(|edit| (key.clone(), DialogInputValue::Text(edit.borrow().value()))),
                DialogInputControl::Boolean {
                    key,
                    value,
                    on_true,
                    on_false,
                    ..
                } => Some((
                    key.clone(),
                    DialogInputValue::Boolean {
                        value: *value,
                        substitution: if *value {
                            on_true.clone()
                        } else {
                            on_false.clone()
                        },
                    },
                )),
                DialogInputControl::NumberRange { key, value, .. } => {
                    Some((key.clone(), DialogInputValue::Number(*value)))
                }
                DialogInputControl::SingleOption {
                    key,
                    options,
                    selected,
                    ..
                } => options
                    .get(*selected)
                    .map(|(id, _)| (key.clone(), DialogInputValue::Option(id.clone()))),
            })
            .collect()
    }
    fn rebuild(&mut self) {
        let (width, height) = (self.base.width, self.base.height);
        if width > 0 && height > 0 {
            screen_init(self, width, height);
        }
    }
    fn add_button(&mut self, request: ButtonRequest, label: &str, x: i32, y: i32, width: i32) {
        let button = Rc::new(RefCell::new(
            Button::<ButtonRequest>::builder(Component::literal(label))
                .bounds(x, y, width, ROW_HEIGHT)
                .with_action(request.clone())
                .with_font(Arc::clone(&self.font))
                .build(),
        ));
        self.base.add_widget(SharedButton::new(Rc::clone(&button)));
        self.buttons.push((request, button));
    }
    fn init_widgets(&mut self) {
        self.base.clear_widgets();
        self.buttons.clear();
        self.body_lines.clear();
        self.input_rows.clear();
        let width = self.base.width;
        let height = self.base.height;
        if width <= 0 || height <= 0 {
            return;
        }
        self.page = self.page.min(self.page_count().saturating_sub(1));
        let text_width = (width - 48).max(48) as f32;
        for text in &self.model.body {
            for line in crate::wrap_text(text, text_width, self.font.as_ref()) {
                self.body_lines.push(line);
            }
        }
        self.body_lines.truncate(MAX_BODY_LINES);
        let item_rows_height: i32 = self
            .model
            .body_items
            .iter()
            .map(|item| item.height.max(16) + 4)
            .sum();
        self.input_top = BODY_TOP + self.body_lines.len() as i32 * 10 + 8 + item_rows_height;
        let mut input_y = self.input_top;
        for index in 0..self.model.inputs.len() {
            let input = &self.model.inputs[index];
            self.input_rows.push(input_y);
            let row_height = dialog_input_row_height(input);
            match input {
                DialogInputControl::Text {
                    width: requested_width,
                    label_visible,
                    multiline,
                    ..
                } => {
                    if let Some(edit) = self.text_boxes.get(index).and_then(Option::as_ref) {
                        let edit_width = (*requested_width).min((width - 48).max(8));
                        edit.borrow_mut().set_size(
                            self.font.as_ref(),
                            edit_width,
                            multiline
                                .as_ref()
                                .map_or(ROW_HEIGHT, |options| options.height),
                        );
                        edit.borrow_mut().set_position(
                            self.font.as_ref(),
                            (width - edit_width) / 2,
                            input_y + if *label_visible { 12 } else { 0 },
                        );
                        self.base.add_widget(DialogEditBoxHandle {
                            edit_box: Rc::clone(edit),
                            font: Arc::clone(&self.font),
                            clock_ms: Rc::clone(&self.clock_ms),
                            clipboard: self.clipboard.clone(),
                        });
                    }
                }
                DialogInputControl::NumberRange { .. } => {
                    let control_width = 220.min((width - 48).max(90));
                    let left = (width - control_width) / 2;
                    let side_width = 32.min(control_width / 4);
                    self.add_button(
                        ButtonRequest::Input { index, delta: -1 },
                        "−",
                        left,
                        input_y,
                        side_width,
                    );
                    self.add_button(
                        ButtonRequest::Input { index, delta: 1 },
                        "+",
                        left + control_width - side_width,
                        input_y,
                        side_width,
                    );
                }
                DialogInputControl::Boolean { .. } | DialogInputControl::SingleOption { .. } => {
                    let label = dialog_input_button_label(input);
                    let button_width = 300.min((width - 48).max(80));
                    self.add_button(
                        ButtonRequest::Input { index, delta: 1 },
                        &label,
                        (width - button_width) / 2,
                        input_y,
                        button_width,
                    );
                }
            }
            input_y += row_height;
        }
        if self.waiting_for_response {
            let mut body = translated(
                "gui.waitingForResponse.message",
                "Waiting for the server to respond...",
            );
            if body == "gui.waitingForResponse.message" {
                body = "Waiting for the server to respond...".to_string();
            }
            self.body_lines.clear();
            self.body_lines.push(body);
            let available = self.waiting_ticks >= 20;
            let enabled = self.waiting_ticks >= 100;
            if available {
                let label = if enabled {
                    translated("gui.back", "Back")
                } else {
                    format!(
                        "{} ({})",
                        translated("gui.back", "Back"),
                        5 - self.waiting_ticks / 20
                    )
                };
                self.add_button(
                    ButtonRequest::Done,
                    &label,
                    (width - 200) / 2,
                    (height - 26).max(4),
                    200.min((width - 8).max(1)),
                );
                if !enabled {
                    if let Some((_, button)) = self.buttons.last() {
                        button.borrow_mut().base.active = false;
                    }
                }
            }
            self.base.set_close_on_esc(enabled);
            self.page_count = 1;
            return;
        }
        let columns = self.model.columns.clamp(1, 4);
        let footer_y = (height - 26).max(4);
        let action_top = (self.input_top + self.form_height() + 4)
            .min((footer_y - ROW_HEIGHT - ROW_GAP).max(BODY_TOP));
        let action_bottom = footer_y - ROW_HEIGHT - ROW_GAP;
        let rows_that_fit =
            ((action_bottom - action_top).max(ROW_HEIGHT) / (ROW_HEIGHT + ROW_GAP)).max(1) as usize;
        let page_size = PAGE_ROW_COUNT
            .min(rows_that_fit.saturating_mul(columns))
            .max(1);
        let page_count = self.model.buttons.len().max(1).div_ceil(page_size);
        self.page_count = page_count;
        self.page = self.page.min(page_count.saturating_sub(1));
        let first = self.page * page_size;
        let last = (first + page_size).min(self.model.buttons.len());
        let available_width = (width - 24).max(1);
        let cell_gap = 4;
        let cell_width =
            ((available_width - (columns as i32 - 1) * cell_gap) / columns as i32).max(1);
        let grid_width = columns as i32 * cell_width + (columns as i32 - 1) * cell_gap;
        let x0 = (width - grid_width) / 2;
        for index in first..last {
            let visible = index - first;
            let x = x0 + (visible % columns) as i32 * (cell_width + cell_gap);
            let y = action_top + (visible / columns) as i32 * (ROW_HEIGHT + ROW_GAP);
            let button = &self.model.buttons[index];
            let label = button.label.clone();
            let button_width = button.width.clamp(1, cell_width).min(cell_width);
            let button_x = x + (cell_width - button_width) / 2;
            self.add_button(
                ButtonRequest::Action(index),
                &label,
                button_x,
                y,
                button_width,
            );
        }
        if page_count > 1 {
            let pager_width = 200.min((width - 16).max(1));
            let half = ((pager_width - 4) / 2).max(1);
            self.add_button(
                ButtonRequest::Previous,
                &translated("gui.previous", "Previous"),
                (width - pager_width) / 2,
                footer_y - ROW_HEIGHT - ROW_GAP,
                half,
            );
            self.add_button(
                ButtonRequest::Next,
                &translated("gui.next", "Next"),
                (width - pager_width) / 2 + half + 4,
                footer_y - ROW_HEIGHT - ROW_GAP,
                pager_width - half - 4,
            );
        }
        if !self.model.footer_buttons.is_empty() {
            let columns = self.model.footer_buttons.len().clamp(1, 4);
            let footer_gap = 4;
            let cell_width =
                ((width - 24 - (columns as i32 - 1) * footer_gap) / columns as i32).max(1);
            let total_width = columns as i32 * cell_width + (columns as i32 - 1) * footer_gap;
            let left = (width - total_width) / 2;
            let footer_buttons: Vec<(String, i32)> = self
                .model
                .footer_buttons
                .iter()
                .map(|button| (button.label.clone(), button.width))
                .collect();
            for (index, (label, requested_width)) in footer_buttons.iter().enumerate() {
                let button_width = (*requested_width).clamp(1, cell_width);
                let cell_left = left + index as i32 * (cell_width + footer_gap);
                self.add_button(
                    ButtonRequest::Footer(index),
                    label,
                    cell_left + (cell_width - button_width) / 2,
                    footer_y,
                    button_width,
                );
            }
        }
    }
    fn render_content(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
    ) -> Option<String> {
        let title = if self.waiting_for_response {
            translated("gui.waitingForResponse.title", "Waiting for response")
        } else {
            self.model.title.clone()
        };
        let title_width = self.font.measure_width(&title, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            title,
            (self.base.width - title_width) / 2,
            12,
            WHITE,
        );
        let body_bottom = (self.base.height - 61).max(BODY_TOP);
        let x = (self.base.width / 2 - (self.base.width - 48).max(48) / 2).max(4);
        graphics.enable_scissor(x, BODY_TOP, self.base.width - x, body_bottom);
        for (index, line) in self.body_lines.iter().enumerate() {
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                line.clone(),
                x,
                BODY_TOP + index as i32 * 10,
                MUTED,
            );
        }
        let mut item_y = BODY_TOP + self.body_lines.len() as i32 * 10 + 8;
        let mut hovered_item = None;
        for item in &self.model.body_items {
            let rect = ScreenRectangle::new(x, item_y, item.width.max(1), item.height.max(1));
            graphics.render_item_with_decorations(
                item.stack.clone(),
                x,
                item_y,
                item.show_decorations,
            );
            if item.show_tooltip && rect.contains_point(mouse_x, mouse_y) {
                let key = format!(
                    "item.{}.{}",
                    item.stack.item.location.namespace, item.stack.item.location.path
                );
                let name = translated(&key, &item.stack.item.location.path.replace('_', " "));
                hovered_item = Some(name);
            }
            item_y += item.height.max(16) + 4;
        }
        graphics.disable_scissor();
        for (index, input) in self.model.inputs.iter().enumerate() {
            let Some(y) = self.input_rows.get(index).copied() else {
                continue;
            };
            match input {
                DialogInputControl::Text {
                    label,
                    label_visible,
                    ..
                } => {
                    if *label_visible {
                        graphics.text(
                            ResourceLocation::from_string("minecraft:default"),
                            label.clone(),
                            (self.base.width - self.font.measure_width(label, 1.0).round() as i32)
                                / 2,
                            y,
                            WHITE,
                        );
                    }
                    if let Some(edit) = self.text_boxes.get(index).and_then(Option::as_ref) {
                        let hovered = graphics.contains_point_in_scissor(mouse_x, mouse_y)
                            && edit.borrow().contains(mouse_x as f64, mouse_y as f64);
                        if hovered {
                            graphics.request_cursor(CursorType::Text);
                        }
                        draw_edit_box_plan(
                            graphics,
                            &edit
                                .borrow()
                                .render_plan(self.font.as_ref(), self.clock_ms.get()),
                        );
                    }
                }
                DialogInputControl::NumberRange { label, value, .. } => {
                    let text = format!("{label}: {}", format_dialog_number(*value));
                    let x =
                        (self.base.width - self.font.measure_width(&text, 1.0).round() as i32) / 2;
                    graphics.text(
                        ResourceLocation::from_string("minecraft:default"),
                        text,
                        x,
                        y + 6,
                        WHITE,
                    );
                }
                _ => {}
            }
        }
        hovered_item
    }
    fn form_height(&self) -> i32 {
        self.model.inputs.iter().map(dialog_input_row_height).sum()
    }
}
struct DialogEditBoxHandle {
    edit_box: Rc<RefCell<DialogTextBox>>,
    font: Arc<dyn FontWidthMeasurer>,
    clock_ms: Rc<Cell<u64>>,
    clipboard: Option<Rc<RefCell<dyn Clipboard>>>,
}
impl Renderable for DialogEditBoxHandle {
    fn extract_render_state(
        &mut self,
        _graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
    }
}
impl TabOrderedElement for DialogEditBoxHandle {}
impl GuiEventListener for DialogEditBoxHandle {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        if event.button() != 0 || !self.edit_box.borrow().contains(event.x, event.y) {
            return false;
        }
        self.edit_box.borrow_mut().on_click(
            self.font.as_ref(),
            event.x,
            event.y,
            event.button_info.modifiers & KeyEvent::MOD_SHIFT != 0,
            double_click,
        );
        true
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, _dx: f64, _dy: f64) -> bool {
        if event.button() != 0 || !self.edit_box.borrow().is_focused() {
            return false;
        }
        self.edit_box
            .borrow_mut()
            .on_drag(self.font.as_ref(), event.x, event.y);
        true
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if !self.edit_box.borrow().is_focused() {
            return false;
        }
        if let Some(clipboard) = self.clipboard.as_ref() {
            let mut clipboard = clipboard.borrow_mut();
            self.edit_box
                .borrow_mut()
                .handle_key(self.font.as_ref(), event, Some(&mut *clipboard))
        } else {
            self.edit_box
                .borrow_mut()
                .handle_key(self.font.as_ref(), event, None)
        }
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, _sx: f64, sy: f64) -> bool {
        if !self.edit_box.borrow().contains(x, y) {
            return false;
        }
        match &mut *self.edit_box.borrow_mut() {
            DialogTextBox::Multi(edit) => edit.mouse_scrolled(self.font.as_ref(), sy),
            _ => false,
        }
    }
    fn char_typed(&mut self, event: CharacterEvent) -> bool {
        self.edit_box
            .borrow_mut()
            .char_typed(self.font.as_ref(), event.codepoint)
    }
    fn preedit_updated(&mut self, event: Option<PreeditEvent>) -> bool {
        self.edit_box
            .borrow_mut()
            .preedit_updated(self.font.as_ref(), event)
    }
    fn play_down_sound(&self) -> Option<crate::components::WidgetClickSound> {
        None
    }
    fn next_focus_path(&self, _event: &FocusNavigationEvent) -> Option<ComponentPath> {
        (!self.edit_box.borrow().is_focused()).then(|| ComponentPath::leaf(0))
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.edit_box.borrow().contains(mouse_x, mouse_y)
    }
    fn set_focused(&mut self, focused: bool) {
        self.edit_box
            .borrow_mut()
            .set_focused(focused, self.clock_ms.get());
    }
    fn is_focused(&self) -> bool {
        self.edit_box.borrow().is_focused()
    }
    fn rectangle(&self) -> ScreenRectangle {
        let edit = self.edit_box.borrow();
        ScreenRectangle::new(edit.x(), edit.y(), edit.width(), edit.height())
    }
}
enum DialogTextBox {
    Single(EditBox),
    Multi(MultilineEditBox),
}
impl DialogTextBox {
    fn from_input(
        font: &dyn FontWidthMeasurer,
        input: &DialogInputControl,
    ) -> Option<Rc<RefCell<Self>>> {
        let DialogInputControl::Text {
            initial,
            max_length,
            width,
            multiline,
            ..
        } = input
        else {
            return None;
        };
        let edit = if let Some(options) = multiline {
            Self::Multi(MultilineEditBox::new(
                font,
                *width,
                options.height,
                *max_length,
                options.max_lines,
                initial,
            ))
        } else {
            let mut edit = EditBox::new(font, 0, 0, *width, ROW_HEIGHT);
            edit.set_max_length(font, *max_length);
            edit.set_value(font, initial);
            Self::Single(edit)
        };
        Some(Rc::new(RefCell::new(edit)))
    }
    fn value(&self) -> String {
        match self {
            Self::Single(e) => e.value(),
            Self::Multi(e) => e.value(),
        }
    }
    fn x(&self) -> i32 {
        match self {
            Self::Single(e) => e.x(),
            Self::Multi(e) => e.x(),
        }
    }
    fn y(&self) -> i32 {
        match self {
            Self::Single(e) => e.y(),
            Self::Multi(e) => e.y(),
        }
    }
    fn width(&self) -> i32 {
        match self {
            Self::Single(e) => e.width(),
            Self::Multi(e) => e.width(),
        }
    }
    fn height(&self) -> i32 {
        match self {
            Self::Single(e) => e.height(),
            Self::Multi(e) => e.height(),
        }
    }
    fn is_focused(&self) -> bool {
        match self {
            Self::Single(e) => e.is_focused(),
            Self::Multi(e) => e.is_focused(),
        }
    }
    fn contains(&self, x: f64, y: f64) -> bool {
        match self {
            Self::Single(e) => e.contains(x, y),
            Self::Multi(e) => e.contains(x, y),
        }
    }
    fn set_focused(&mut self, value: bool, now: u64) {
        match self {
            Self::Single(e) => e.set_focused(value, now),
            Self::Multi(e) => e.set_focused(value, now),
        }
    }
    fn set_size(&mut self, font: &dyn FontWidthMeasurer, w: i32, h: i32) {
        match self {
            Self::Single(e) => e.set_size(font, w, h),
            Self::Multi(e) => e.set_size(font, w, h),
        }
    }
    fn set_position(&mut self, font: &dyn FontWidthMeasurer, x: i32, y: i32) {
        match self {
            Self::Single(e) => e.set_position(font, x, y),
            Self::Multi(e) => e.set_position(x, y),
        }
    }
    fn on_click(
        &mut self,
        font: &dyn FontWidthMeasurer,
        x: f64,
        y: f64,
        shift: bool,
        double: bool,
    ) {
        match self {
            Self::Single(e) => e.on_click(font, x, shift, double),
            Self::Multi(e) => e.on_click(font, x, y, shift, double),
        }
    }
    fn on_drag(&mut self, font: &dyn FontWidthMeasurer, x: f64, y: f64) {
        match self {
            Self::Single(e) => e.on_drag(font, x),
            Self::Multi(e) => e.on_drag(font, x, y),
        }
    }
    fn char_typed(&mut self, font: &dyn FontWidthMeasurer, c: char) -> bool {
        match self {
            Self::Single(e) => e.char_typed(font, c),
            Self::Multi(e) => e.char_typed(font, c),
        }
    }
    fn preedit_updated(
        &mut self,
        font: &dyn FontWidthMeasurer,
        event: Option<PreeditEvent>,
    ) -> bool {
        match self {
            Self::Single(e) => e.preedit_updated(font, event),
            Self::Multi(e) => e.preedit_updated(event),
        }
    }
    fn render_plan(&self, font: &dyn FontWidthMeasurer, now: u64) -> EditBoxRenderPlan {
        match self {
            Self::Single(e) => e.render_plan(font, now),
            Self::Multi(e) => e.render_plan(font, now),
        }
    }
    fn handle_key(
        &mut self,
        font: &dyn FontWidthMeasurer,
        event: KeyEvent,
        clipboard: Option<&mut dyn Clipboard>,
    ) -> bool {
        match self {
            Self::Multi(edit) => edit.key_pressed(font, event, clipboard),
            Self::Single(edit) => {
                if let Some(clipboard) = clipboard {
                    if event.is_copy() {
                        return edit.copy(clipboard);
                    }
                    if event.is_cut() {
                        return edit.cut(font, clipboard);
                    }
                    if event.is_paste() {
                        return edit.paste(font, clipboard);
                    }
                }
                let key = match event.key {
                    KeyEvent::KEY_BACKSPACE => Some(EditBoxKey::Backspace),
                    KeyEvent::KEY_DELETE => Some(EditBoxKey::Delete),
                    KeyEvent::KEY_LEFT => Some(EditBoxKey::Left),
                    KeyEvent::KEY_RIGHT => Some(EditBoxKey::Right),
                    KeyEvent::KEY_HOME => Some(EditBoxKey::Home),
                    KeyEvent::KEY_END => Some(EditBoxKey::End),
                    _ => None,
                };
                if let Some(key) = key {
                    return edit.key_pressed(
                        font,
                        key,
                        event.has_control_down(),
                        event.has_shift_down(),
                    );
                }
                event.is_select_all() && edit.select_all(font)
            }
            #[allow(unreachable_patterns)]
            _ => false,
        }
    }
}
impl LayoutElement for DialogEditBoxHandle {
    fn set_x(&mut self, x: i32) {
        let y = self.edit_box.borrow().y();
        self.edit_box
            .borrow_mut()
            .set_position(self.font.as_ref(), x, y);
    }
    fn set_y(&mut self, y: i32) {
        let x = self.edit_box.borrow().x();
        self.edit_box
            .borrow_mut()
            .set_position(self.font.as_ref(), x, y);
    }
    fn x(&self) -> i32 {
        self.edit_box.borrow().x()
    }
    fn y(&self) -> i32 {
        self.edit_box.borrow().y()
    }
    fn width(&self) -> i32 {
        self.edit_box.borrow().width()
    }
    fn height(&self) -> i32 {
        self.edit_box.borrow().height()
    }
}
impl crate::framework::narration::NarratableEntry for DialogEditBoxHandle {
    fn narration_priority(&self) -> NarrationPriority {
        if self.edit_box.borrow().is_focused() {
            NarrationPriority::Focused
        } else {
            NarrationPriority::None
        }
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        output.add(NarratedElementType::Hint, "editBox");
        output.add(
            NarratedElementType::Position,
            self.edit_box.borrow().value(),
        );
    }
}
fn dialog_input_row_height(input: &DialogInputControl) -> i32 {
    match input {
        DialogInputControl::Text {
            multiline,
            label_visible,
            ..
        } => {
            multiline
                .as_ref()
                .map_or(ROW_HEIGHT, |options| options.height)
                + if *label_visible { 12 } else { 0 }
                + ROW_GAP
        }
        _ => ROW_HEIGHT + ROW_GAP,
    }
}
fn dialog_input_button_label(input: &DialogInputControl) -> String {
    match input {
        DialogInputControl::Boolean { label, value, .. } => {
            format!("{label}: {}", if *value { "On" } else { "Off" })
        }
        DialogInputControl::SingleOption {
            label,
            options,
            selected,
            ..
        } => {
            format!(
                "{label}: {}",
                options
                    .get(*selected)
                    .map(|(_, display)| display.as_str())
                    .unwrap_or("")
            )
        }
        DialogInputControl::Text { label, .. } => label.clone(),
        DialogInputControl::NumberRange { label, value, .. } => {
            format!("{label}: {}", format_dialog_number(*value))
        }
    }
}
fn change_dialog_input(input: &mut DialogInputControl, delta: i8) {
    match input {
        DialogInputControl::Boolean { value, .. } => *value = !*value,
        DialogInputControl::NumberRange {
            start,
            end,
            value,
            step,
            ..
        } => {
            let increment = (*step).unwrap_or((*end - *start).abs() / 20.0);
            let direction = if *end < *start { -1.0 } else { 1.0 };
            *value = (*value + f32::from(delta) * increment * direction)
                .clamp((*start).min(*end), (*start).max(*end));
        }
        DialogInputControl::SingleOption {
            options, selected, ..
        } if !options.is_empty() => {
            *selected = (*selected as isize + isize::from(delta)).rem_euclid(options.len() as isize)
                as usize;
        }
        DialogInputControl::Text { .. } | DialogInputControl::SingleOption { .. } => {}
    }
}
fn draw_edit_box_plan(
    graphics: &mut GuiGraphicsExtractor,
    plan: &crate::components::edit_box::EditBoxRenderPlan,
) {
    for draw in &plan.draws {
        match draw {
            EditBoxDraw::Sprite {
                sprite,
                x,
                y,
                width,
                height,
            } => graphics.blit(
                ResourceLocation::from_string(sprite),
                *x,
                *y,
                *width,
                *height,
            ),
            EditBoxDraw::Text {
                text,
                x,
                y,
                color,
                shadow,
            } => graphics.text_with_shadow(
                ResourceLocation::from_string("minecraft:default"),
                text.clone(),
                *x,
                *y,
                *color,
                *shadow,
            ),
            EditBoxDraw::Highlight {
                x0,
                y0,
                x1,
                y1,
                invert,
            } => {
                let (left, right) = (*x0.min(x1), *x0.max(x1));
                let (top, bottom) = (*y0.min(y1), *y0.max(y1));
                graphics.text_highlight(left, top, right, bottom, *invert);
            }
            EditBoxDraw::InsertCursor { x, y, color } => {
                graphics.fill(*x, *y - 1, *x + 1, *y + 9, *color)
            }
            EditBoxDraw::AppendCursor {
                x,
                y,
                color,
                shadow,
            } => graphics.text_with_shadow(
                ResourceLocation::from_string("minecraft:default"),
                EditBox::append_cursor_character(),
                *x,
                *y,
                *color,
                *shadow,
            ),
        }
    }
    if let Some(underline) = plan.preedit_underline {
        if underline.x1 > underline.x0 {
            graphics.fill(
                underline.x0,
                underline.y,
                underline.x1,
                underline.y + 1,
                underline.color,
            );
        }
    }
}
impl Screen for ServerDialogScreen {
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
        self.init_widgets();
    }
    fn on_close(&mut self) {
        self.base.request_close();
    }
    fn tick(&mut self) {
        self.clock_ms.set(self.clock_ms.get().saturating_add(50));
        if !self.waiting_for_response || self.waiting_ticks >= 100 {
            return;
        }
        self.waiting_ticks += 1;
        if self.waiting_ticks % 20 == 0 {
            self.rebuild();
        }
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        screen_key_pressed(self, event)
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        match self.background_route {
            ServerDialogBackground::MenuDialogBase => {
                self.base_ref().extract_background_layers(graphics, false);
            }
            ServerDialogBackground::InWorldDialogBase => {
                self.base_ref().extract_background_layers(graphics, true);
            }
            ServerDialogBackground::TransparentSignInWorld => {
                self.base_ref().extract_transparent_background(graphics);
            }
        }
        let hovered_item = self.render_content(graphics, mouse_x, mouse_y);
        self.base
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
        if let Some(item_name) = hovered_item {
            graphics.draw_tooltip(self.font.as_ref(), &[item_name], mouse_x, mouse_y);
        }
    }
}
fn translated(key: &str, fallback: &str) -> String {
    let translated = crate::translate(key);
    if translated == key {
        fallback.to_string()
    } else {
        translated
    }
}
