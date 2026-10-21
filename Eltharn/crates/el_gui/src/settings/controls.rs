use crate::FontWidthMeasurer;
use crate::components::click_sound_sink;
use crate::components::{
    Button, DEFAULT_HEIGHT, GuiEventListener, Renderable, TabOrderedElement, WidgetClickSound,
};
use crate::framework::graphics::{GuiGraphicsExtractor, WidgetTooltipPosition};
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{NarratableEntry, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
pub const HEADER_HEIGHT: i32 = 33;
pub const FOOTER_HEIGHT: i32 = 33;
pub const ROW_HEIGHT: i32 = 20;
pub const ROW_SPACING: i32 = 25;
pub const BIG_BUTTON_WIDTH: i32 = 310;
pub const SMALL_BUTTON_WIDTH: i32 = 150;
pub const DONE_BUTTON_WIDTH: i32 = 200;
pub const FOOTER_BUTTON_Y_OFFSET: i32 = 26;
const SPRINT_WINDOW_MIN: i32 = 0;
const SPRINT_WINDOW_MAX: i32 = 10;
const SPRINT_WINDOW_DEFAULT: i32 = 7;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControlsToggleOptions {
    pub toggle_crouch: bool,
    pub toggle_sprint: bool,
    pub toggle_attack: bool,
    pub toggle_use: bool,
}
impl ControlsToggleOptions {
    pub fn from_options(extra_options: &HashMap<String, String>) -> Self {
        let get = |key: &str| extra_options.get(key).is_some_and(|value| value == "true");
        Self {
            toggle_crouch: get("toggleCrouch"),
            toggle_sprint: get("toggleSprint"),
            toggle_attack: get("toggleAttack"),
            toggle_use: get("toggleUse"),
        }
    }
}
pub fn decode_sprint_window_option(encoded: &str) -> Option<f32> {
    let value = encoded.parse::<f64>().ok()?;
    if !value.is_finite() {
        return None;
    }
    let has_fractional_syntax =
        encoded.contains('.') || encoded.contains('e') || encoded.contains('E');
    let ticks = if has_fractional_syntax && (0.0..=1.0).contains(&value) {
        value * f64::from(SPRINT_WINDOW_MAX)
    } else {
        value
    };
    let rounded = ticks.round();
    if (ticks - rounded).abs() > 1.0e-6
        || !(f64::from(SPRINT_WINDOW_MIN)..=f64::from(SPRINT_WINDOW_MAX)).contains(&rounded)
    {
        return None;
    }
    Some(rounded as f32)
}
pub fn encode_sprint_window_option(value: f32) -> String {
    let ticks = if value.is_finite() {
        value
            .round()
            .clamp(SPRINT_WINDOW_MIN as f32, SPRINT_WINDOW_MAX as f32) as i32
    } else {
        SPRINT_WINDOW_DEFAULT
    };
    ticks.to_string()
}
#[derive(Clone, Debug)]
pub struct ControlsOption {
    pub key: String,
    pub label: String,
    pub is_slider: bool,
    pub min_val: f32,
    pub max_val: f32,
    pub step: f32,
    pub value: f32,
}
impl ControlsOption {
    pub fn get_display_string(&self) -> String {
        let name = crate::translate(&self.label);
        match self.key.as_str() {
            "toggleCrouch" | "toggleSprint" | "toggleAttack" | "toggleUse" => {
                let (key, fallback) = if self.value > 0.5 {
                    ("options.key.toggle", "Toggle")
                } else {
                    ("options.key.hold", "Hold")
                };
                let state = translate_or_fallback(key, fallback);
                format!("{}: {}", name, state)
            }
            "autoJump" | "operatorItemsTab" => {
                let state = if self.value > 0.5 {
                    crate::translate("options.on")
                } else {
                    crate::translate("options.off")
                };
                format!("{}: {}", name, state)
            }
            "mouseSettings" => name,
            "keybinds" => name,
            "sprintWindow" => {
                let val = self.value.round() as i32;
                if val == 0 {
                    format!("{}: {}", name, crate::translate("options.off"))
                } else {
                    format!("{}: {}", name, val)
                }
            }
            _ => format!("{}: {}", name, self.value),
        }
    }
    pub fn option_text_value(&self) -> String {
        match self.key.as_str() {
            "toggleCrouch" | "toggleSprint" | "toggleAttack" | "toggleUse" | "autoJump"
            | "operatorItemsTab" => (self.value > 0.5).to_string(),
            "sprintWindow" => encode_sprint_window_option(self.value),
            _ => format!("{:.4}", self.value),
        }
    }
}
fn translate_or_fallback(key: &str, fallback: &str) -> String {
    let translated = crate::translate(key);
    if translated == key {
        fallback.to_string()
    } else {
        translated
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlsAction {
    Done,
    MouseSettingsClicked,
    KeybindsClicked,
    OptionChanged,
}
#[derive(Clone, Debug)]
pub enum ControlsListItem {
    Header {
        label_key: &'static str,
    },
    Big {
        option_index: usize,
    },
    SmallPair {
        option_index_a: usize,
        option_index_b: Option<usize>,
    },
}
#[derive(Clone, Debug)]
pub struct LayoutItem {
    pub kind: ControlsListItem,
    pub relative_y: i32,
    pub height: i32,
}
struct RowWidget {
    rect: ScreenRectangle,
    option_index: usize,
    is_slider: bool,
    focused: bool,
    pressed: bool,
}
impl RowWidget {
    fn new(rect: ScreenRectangle, option_index: usize, is_slider: bool) -> Self {
        Self {
            rect,
            option_index,
            is_slider,
            focused: false,
            pressed: false,
        }
    }
    fn takes_press(&mut self) -> bool {
        let pressed = self.pressed;
        self.pressed = false;
        pressed
    }
}
#[derive(Clone)]
struct RowHandle {
    row: Rc<RefCell<RowWidget>>,
}
impl RowHandle {
    fn new(row: Rc<RefCell<RowWidget>>) -> Self {
        Self { row }
    }
    fn with_row<R>(&self, f: impl FnOnce(&RowWidget) -> R, fallback: R) -> R {
        self.row.try_borrow().map(|r| f(&r)).unwrap_or(fallback)
    }
}
impl Renderable for RowHandle {
    fn extract_render_state(
        &mut self,
        _graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
    }
}
impl TabOrderedElement for RowHandle {
    fn tab_order_group(&self) -> i32 {
        0
    }
}
impl NarratableEntry for RowHandle {
    fn narration_priority(&self) -> NarrationPriority {
        NarrationPriority::None
    }
    fn update_narration(&self, _output: &mut NarrationElementOutput) {}
}
impl GuiEventListener for RowHandle {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        if event.button() != 0 {
            return false;
        }
        let Ok(mut row) = self.row.try_borrow_mut() else {
            return false;
        };
        if row.rect.contains_point(event.x as i32, event.y as i32) {
            row.pressed = true;
            return true;
        }
        false
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.with_row(
            |row| row.rect.contains_point(mouse_x as i32, mouse_y as i32),
            false,
        )
    }
    fn set_focused(&mut self, focused: bool) {
        if let Ok(mut row) = self.row.try_borrow_mut() {
            row.focused = focused;
        }
    }
    fn is_focused(&self) -> bool {
        self.with_row(|row| row.focused, false)
    }
    fn rectangle(&self) -> ScreenRectangle {
        self.with_row(|row| row.rect, ScreenRectangle::empty())
    }
    fn next_focus_path(&self, _event: &FocusNavigationEvent) -> Option<ComponentPath> {
        Some(ComponentPath::leaf(0))
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.with_row(
            |row| {
                if row.is_slider {
                    None
                } else {
                    Some(WidgetClickSound::UiButtonClick)
                }
            },
            Some(WidgetClickSound::UiButtonClick),
        )
    }
}
impl LayoutElement for RowHandle {
    fn set_x(&mut self, x: i32) {
        if let Ok(mut row) = self.row.try_borrow_mut() {
            row.rect.position.x = x;
        }
    }
    fn set_y(&mut self, y: i32) {
        if let Ok(mut row) = self.row.try_borrow_mut() {
            row.rect.position.y = y;
        }
    }
    fn x(&self) -> i32 {
        self.with_row(|row| row.rect.position.x, 0)
    }
    fn y(&self) -> i32 {
        self.with_row(|row| row.rect.position.y, 0)
    }
    fn width(&self) -> i32 {
        self.with_row(|row| row.rect.width, 0)
    }
    fn height(&self) -> i32 {
        self.with_row(|row| row.rect.height, 0)
    }
}
struct FooterButtonHandle {
    button: Rc<RefCell<Button<ControlsAction>>>,
}
impl FooterButtonHandle {
    fn new(button: Rc<RefCell<Button<ControlsAction>>>) -> Self {
        Self { button }
    }
}
impl Renderable for FooterButtonHandle {
    fn extract_render_state(
        &mut self,
        _graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
    }
}
impl TabOrderedElement for FooterButtonHandle {
    fn tab_order_group(&self) -> i32 {
        self.button.borrow().tab_order_group()
    }
}
impl GuiEventListener for FooterButtonHandle {
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
impl NarratableEntry for FooterButtonHandle {
    fn narration_priority(&self) -> NarrationPriority {
        self.button.borrow().narration_priority()
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        self.button.borrow().update_narration(output);
    }
}
impl LayoutElement for FooterButtonHandle {
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
pub struct ControlsScreen {
    base: ScreenBase,
    font: Arc<dyn FontWidthMeasurer>,
    pub options: Vec<ControlsOption>,
    pub in_world: bool,
    pub scroll_y: f32,
    pub dragging_scrollbar: bool,
    pub dragging_slider_index: Option<usize>,
    slider_can_change_value: bool,
    focused_slider_index: Option<usize>,
    pending_action: Option<ControlsAction>,
    value_changed: bool,
    done: Rc<RefCell<Button<ControlsAction>>>,
    rows: Vec<Rc<RefCell<RowWidget>>>,
    pub last_width: i32,
    pub last_height: i32,
}
impl ControlsScreen {
    pub fn new(
        in_world: bool,
        extra_options: &HashMap<String, String>,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let toggle_options = ControlsToggleOptions::from_options(extra_options);
        Self::new_with_toggle_options(in_world, extra_options, toggle_options, font)
    }
    pub fn new_with_toggle_options(
        in_world: bool,
        extra_options: &HashMap<String, String>,
        toggle_options: ControlsToggleOptions,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let mut options = Vec::new();
        let get_bool = |key: &str, default: bool| -> bool {
            extra_options
                .get(key)
                .map(|s| s == "true")
                .unwrap_or(default)
        };
        options.push(ControlsOption {
            key: "mouseSettings".to_string(),
            label: "options.mouse_settings".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: 0.0,
        });
        options.push(ControlsOption {
            key: "keybinds".to_string(),
            label: "controls.keybinds".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: 0.0,
        });
        options.push(ControlsOption {
            key: "toggleCrouch".to_string(),
            label: "key.sneak".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if toggle_options.toggle_crouch {
                1.0
            } else {
                0.0
            },
        });
        options.push(ControlsOption {
            key: "toggleSprint".to_string(),
            label: "key.sprint".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if toggle_options.toggle_sprint {
                1.0
            } else {
                0.0
            },
        });
        options.push(ControlsOption {
            key: "toggleAttack".to_string(),
            label: "key.attack".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if toggle_options.toggle_attack {
                1.0
            } else {
                0.0
            },
        });
        options.push(ControlsOption {
            key: "toggleUse".to_string(),
            label: "key.use".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if toggle_options.toggle_use { 1.0 } else { 0.0 },
        });
        options.push(ControlsOption {
            key: "autoJump".to_string(),
            label: "options.autoJump".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if get_bool("autoJump", false) {
                1.0
            } else {
                0.0
            },
        });
        options.push(ControlsOption {
            key: "sprintWindow".to_string(),
            label: "options.sprintWindow".to_string(),
            is_slider: true,
            min_val: SPRINT_WINDOW_MIN as f32,
            max_val: SPRINT_WINDOW_MAX as f32,
            step: 1.0,
            value: extra_options
                .get("sprintWindow")
                .and_then(|encoded| decode_sprint_window_option(encoded))
                .unwrap_or(SPRINT_WINDOW_DEFAULT as f32),
        });
        options.push(ControlsOption {
            key: "operatorItemsTab".to_string(),
            label: "options.operatorItemsTab".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if get_bool("operatorItemsTab", false) {
                1.0
            } else {
                0.0
            },
        });
        let done = Rc::new(RefCell::new(
            Button::<ControlsAction>::builder(Component::literal(&crate::translate("gui.done")))
                .bounds(0, 0, DONE_BUTTON_WIDTH, DEFAULT_HEIGHT)
                .with_action(ControlsAction::Done)
                .with_font(Arc::clone(&font))
                .build(),
        ));
        Self {
            base: ScreenBase::new(Component::literal(&crate::translate("controls.title"))),
            font,
            options,
            in_world,
            scroll_y: 0.0,
            dragging_scrollbar: false,
            dragging_slider_index: None,
            slider_can_change_value: false,
            focused_slider_index: None,
            pending_action: None,
            value_changed: false,
            done,
            rows: Vec::new(),
            last_width: 0,
            last_height: 0,
        }
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn take_action(&mut self) -> Option<ControlsAction> {
        self.pending_action.take()
    }
    pub fn take_value_changed(&mut self) -> bool {
        let changed = self.value_changed;
        self.value_changed = false;
        changed
    }
    pub fn mouse_clicked(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        width: i32,
        height: i32,
    ) -> Option<ControlsAction> {
        self.base.width = width;
        self.base.height = height;
        self.ensure_layout(width, height);
        let event = MouseButtonEvent::new(
            mouse_x as f64,
            mouse_y as f64,
            crate::input::events::MouseButtonInfo::new(0, 0),
        );
        let _ = <Self as Screen>::mouse_clicked(self, event, false);
        match self.take_action() {
            Some(ControlsAction::Done) => {
                click_sound_sink::record_button_click();
                return Some(ControlsAction::Done);
            }
            Some(ControlsAction::MouseSettingsClicked) => {
                click_sound_sink::record_button_click();
                return Some(ControlsAction::MouseSettingsClicked);
            }
            Some(ControlsAction::KeybindsClicked) => {
                click_sound_sink::record_button_click();
                return Some(ControlsAction::KeybindsClicked);
            }
            Some(ControlsAction::OptionChanged) => {
                click_sound_sink::record_button_click();
                return Some(ControlsAction::OptionChanged);
            }
            None => None,
        }
    }
    pub fn get_layout_items(&self) -> Vec<LayoutItem> {
        let list_items = vec![
            ControlsListItem::SmallPair {
                option_index_a: 0,
                option_index_b: Some(1),
            },
            ControlsListItem::SmallPair {
                option_index_a: 2,
                option_index_b: Some(3),
            },
            ControlsListItem::SmallPair {
                option_index_a: 4,
                option_index_b: Some(5),
            },
            ControlsListItem::SmallPair {
                option_index_a: 6,
                option_index_b: Some(7),
            },
            ControlsListItem::SmallPair {
                option_index_a: 8,
                option_index_b: None,
            },
        ];
        let mut items = Vec::new();
        let mut current_y = 0;
        for (i, kind) in list_items.into_iter().enumerate() {
            let height = match &kind {
                ControlsListItem::Header { .. } => {
                    if i == 0 {
                        13
                    } else {
                        31
                    }
                }
                _ => ROW_SPACING,
            };
            items.push(LayoutItem {
                kind,
                relative_y: current_y,
                height,
            });
            current_y += height;
        }
        items
    }
    pub fn max_scroll_amount(&self, height: i32) -> f32 {
        let items = self.get_layout_items();
        if let Some(last) = items.last() {
            let total_height = last.relative_y + last.height + 4;
            let visible_height = height - 66;
            (total_height - visible_height).max(0) as f32
        } else {
            0.0
        }
    }
    pub fn scrollbar_height(&self, height: i32) -> i32 {
        let scroll_area_h = height - 66;
        if scroll_area_h <= 0 {
            return 0;
        }
        let items = self.get_layout_items();
        let total_content_height = if let Some(last) = items.last() {
            last.relative_y + last.height + 4
        } else {
            1
        };
        let bar_h =
            ((scroll_area_h * scroll_area_h) as f32 / total_content_height as f32).round() as i32;
        let upper = (scroll_area_h - 8).max(1);
        let lower = 32.min(upper);
        bar_h.clamp(lower, upper).min(scroll_area_h)
    }
    fn focus_slider_from_mouse(
        &mut self,
        option_idx: usize,
        mx: f32,
        slider_x: i32,
        slider_w: i32,
    ) {
        self.focused_slider_index = Some(option_idx);
        self.slider_can_change_value = true;
        self.dragging_slider_index = Some(option_idx);
        self.update_slider_from_mouse(option_idx, mx, slider_x, slider_w);
    }
    fn update_slider_from_mouse(
        &mut self,
        option_idx: usize,
        mx: f32,
        slider_x: i32,
        slider_w: i32,
    ) {
        let opt = &mut self.options[option_idx];
        let min_x = slider_x + 4;
        let max_x = slider_x + slider_w - 4;
        let pct = if max_x > min_x {
            (mx - min_x as f32) / (max_x - min_x) as f32
        } else {
            0.0
        };
        let clamped = pct.clamp(0.0, 1.0);
        let raw_val = opt.min_val + clamped * (opt.max_val - opt.min_val);
        let steps = ((raw_val - opt.min_val) / opt.step).round();
        let new_val = (opt.min_val + steps * opt.step).clamp(opt.min_val, opt.max_val);
        if (new_val - opt.value).abs() > 0.0001 {
            opt.value = new_val;
            self.value_changed = true;
            self.pending_action = Some(ControlsAction::OptionChanged);
        }
    }
    fn cycle_option(&mut self, option_idx: usize) {
        let opt = &mut self.options[option_idx];
        opt.value += opt.step;
        if opt.value > opt.max_val {
            opt.value = opt.min_val;
        }
    }
    fn scrollbar_drag(&mut self, dy: f64) {
        let height = self.base_ref().height;
        let max_scroll = self.max_scroll_amount(height);
        let bar_h = self.scrollbar_height(height);
        let scroll_area_h = height - 66;
        let y_drag_scale = if scroll_area_h - bar_h > 0 {
            max_scroll / (scroll_area_h - bar_h) as f32
        } else {
            1.0
        };
        self.scroll_y = (self.scroll_y + dy as f32 * y_drag_scale).clamp(0.0, max_scroll);
        self.reposition_rows();
    }
    fn slider_drag(&mut self, option_index: usize, mouse_x: f32) {
        let rect = self
            .rows
            .iter()
            .find(|row| row.borrow().option_index == option_index)
            .map(|row| row.borrow().rect);
        if let Some(rect) = rect {
            self.update_slider_from_mouse(option_index, mouse_x, rect.left(), rect.width);
        }
    }
    fn ensure_layout(&mut self, width: i32, height: i32) {
        if self.last_width == width && self.last_height == height && !self.rows.is_empty() {
            return;
        }
        self.last_width = width;
        self.last_height = height;
        self.base.clear_widgets();
        {
            let mut d = self.done.borrow_mut();
            d.set_position(width / 2 - 100, height - FOOTER_BUTTON_Y_OFFSET);
            d.base.set_width(DONE_BUTTON_WIDTH);
        }
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.done)));
        self.rows.clear();
        for item in self.get_layout_items() {
            let item_y = HEADER_HEIGHT + 4 + item.relative_y - self.scroll_y.round() as i32;
            match item.kind {
                ControlsListItem::Header { .. } => {}
                ControlsListItem::Big { option_index } => {
                    let is_slider = self.options.get(option_index).is_some_and(|o| o.is_slider);
                    self.rows.push(Rc::new(RefCell::new(RowWidget::new(
                        ScreenRectangle::new(width / 2 - 155, item_y, BIG_BUTTON_WIDTH, ROW_HEIGHT),
                        option_index,
                        is_slider,
                    ))));
                }
                ControlsListItem::SmallPair {
                    option_index_a,
                    option_index_b,
                } => {
                    let is_slider_a = self
                        .options
                        .get(option_index_a)
                        .is_some_and(|o| o.is_slider);
                    self.rows.push(Rc::new(RefCell::new(RowWidget::new(
                        ScreenRectangle::new(
                            width / 2 - 155,
                            item_y,
                            SMALL_BUTTON_WIDTH,
                            ROW_HEIGHT,
                        ),
                        option_index_a,
                        is_slider_a,
                    ))));
                    if let Some(option_index_b) = option_index_b {
                        let is_slider_b = self
                            .options
                            .get(option_index_b)
                            .is_some_and(|o| o.is_slider);
                        self.rows.push(Rc::new(RefCell::new(RowWidget::new(
                            ScreenRectangle::new(
                                width / 2 + 5,
                                item_y,
                                SMALL_BUTTON_WIDTH,
                                ROW_HEIGHT,
                            ),
                            option_index_b,
                            is_slider_b,
                        ))));
                    }
                }
            }
        }
        for row in &self.rows {
            self.base.add_widget(RowHandle::new(Rc::clone(row)));
        }
    }
    fn reposition_rows(&mut self) {
        let scroll = self.scroll_y.round() as i32;
        let mut row_idx = 0;
        for item in self.get_layout_items() {
            let item_y = HEADER_HEIGHT + 4 + item.relative_y - scroll;
            match item.kind {
                ControlsListItem::Header { .. } => {}
                ControlsListItem::Big { .. } => {
                    if let Some(row) = self.rows.get(row_idx) {
                        row.borrow_mut().rect.position.y = item_y;
                        row_idx += 1;
                    }
                }
                ControlsListItem::SmallPair { option_index_b, .. } => {
                    if let Some(row) = self.rows.get(row_idx) {
                        row.borrow_mut().rect.position.y = item_y;
                        row_idx += 1;
                    }
                    if option_index_b.is_some() {
                        if let Some(row) = self.rows.get(row_idx) {
                            row.borrow_mut().rect.position.y = item_y;
                            row_idx += 1;
                        }
                    }
                }
            }
        }
    }
    fn hovered_row(&self, mouse_x: i32, mouse_y: i32) -> Option<(usize, ScreenRectangle)> {
        self.rows.iter().find_map(|row| {
            let r = row.borrow();
            r.rect
                .contains_point(mouse_x, mouse_y)
                .then_some((r.option_index, r.rect))
        })
    }
    fn focused_row(&self) -> Option<(usize, ScreenRectangle, WidgetTooltipPosition)> {
        self.rows.iter().find_map(|row| {
            let r = row.borrow();
            r.focused
                .then_some((r.option_index, r.rect, WidgetTooltipPosition::BelowOrAbove))
        })
    }
    fn draw_option(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
        option_index: usize,
        x: i32,
        y: i32,
        w: i32,
    ) {
        let option = &self.options[option_index];
        if option.is_slider {
            let pct = if option.max_val > option.min_val {
                ((option.value - option.min_val) / (option.max_val - option.min_val))
                    .clamp(0.0, 1.0)
            } else {
                0.0
            };
            graphics.draw_slider_with_interaction_state(
                font,
                x,
                y,
                w,
                ROW_HEIGHT,
                &option.get_display_string(),
                pct,
                true,
                self.focused_slider_index == Some(option_index),
                self.slider_can_change_value && self.focused_slider_index == Some(option_index),
                self.dragging_slider_index == Some(option_index),
            );
        } else {
            let focused = self
                .rows
                .iter()
                .find(|r| r.borrow().option_index == option_index)
                .map(|r| r.borrow().focused)
                .unwrap_or(false);
            graphics.draw_button_with_focus(
                font,
                x,
                y,
                w,
                ROW_HEIGHT,
                &option.get_display_string(),
                true,
                focused,
            );
        }
    }
}
impl Screen for ControlsScreen {
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
        let width = self.base.width;
        let height = self.base.height;
        self.last_width = 0;
        self.last_height = 0;
        self.rows.clear();
        self.ensure_layout(width, height);
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if event.is_escape() {
            self.pending_action = Some(ControlsAction::Done);
            return true;
        }
        let focused_idx = self.base_ref().focused_index();
        if let Some(index) = focused_idx {
            if index == 0 {
                if event.is_selection() {
                    self.pending_action = Some(ControlsAction::Done);
                    click_sound_sink::record_button_click();
                    return true;
                }
            } else if let Some(row) = self.rows.get(index - 1) {
                let (option_index, is_slider) = {
                    let row = row.borrow();
                    (row.option_index, row.is_slider)
                };
                if is_slider {
                    if event.is_selection() {
                        self.slider_can_change_value = !self.slider_can_change_value;
                        return true;
                    }
                    if self.slider_can_change_value {
                        let dir = match event.key {
                            KeyEvent::KEY_LEFT => Some(-1.0),
                            KeyEvent::KEY_RIGHT => Some(1.0),
                            _ => None,
                        };
                        if let Some(dir) = dir {
                            let opt = &mut self.options[option_index];
                            let span = opt.max_val - opt.min_val;
                            if span > 0.0 {
                                let old_val = opt.value;
                                opt.value =
                                    (opt.value + dir * opt.step).clamp(opt.min_val, opt.max_val);
                                if (opt.value - old_val).abs() > 0.0001 {
                                    self.value_changed = true;
                                    self.pending_action = Some(ControlsAction::OptionChanged);
                                }
                            }
                            return true;
                        }
                    }
                } else if event.is_selection() {
                    let key = self.options[option_index].key.clone();
                    match key.as_str() {
                        "mouseSettings" => {
                            click_sound_sink::record_button_click();
                            self.pending_action = Some(ControlsAction::MouseSettingsClicked);
                        }
                        "keybinds" => {
                            click_sound_sink::record_button_click();
                            self.pending_action = Some(ControlsAction::KeybindsClicked);
                        }
                        _ => {
                            click_sound_sink::record_button_click();
                            self.cycle_option(option_index);
                            self.value_changed = true;
                            self.pending_action = Some(ControlsAction::OptionChanged);
                        }
                    }
                    return true;
                }
            }
        }
        let handled = crate::framework::screen::screen_key_pressed(self, event);
        handled || (event.key == KeyEvent::KEY_TAB && self.base_ref().focused_index().is_some())
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        let width = self.base_ref().width;
        let height = self.base_ref().height;
        self.ensure_layout(width, height);
        self.reposition_rows();
        let scrollbar_x = width / 2 + 166;
        let bar_h = self.scrollbar_height(height);
        let max_scroll = self.max_scroll_amount(height);
        let bar_y = if max_scroll == 0.0 {
            HEADER_HEIGHT
        } else {
            let scroll_area_h = height - 66;
            HEADER_HEIGHT
                + (self.scroll_y * (scroll_area_h - bar_h) as f32 / max_scroll).round() as i32
        };
        let (mx, my) = (event.x as i32, event.y as i32);
        if event.button() == 0
            && mx >= scrollbar_x
            && mx < scrollbar_x + 6
            && my >= HEADER_HEIGHT
            && my < height - FOOTER_HEIGHT
        {
            self.dragging_scrollbar = true;
            if my < bar_y {
                self.scroll_y = (self.scroll_y - 25.0).max(0.0);
            } else if my > bar_y + bar_h {
                self.scroll_y = (self.scroll_y + 25.0).min(max_scroll);
            }
            self.reposition_rows();
            return true;
        }
        let _ = crate::framework::screen::screen_mouse_clicked(self, event, double_click);
        if let Some(action) = self.done.borrow_mut().take_press_action() {
            self.pending_action = Some(action);
            return true;
        }
        let done_x = width / 2 - 100;
        let done_y = height - FOOTER_BUTTON_Y_OFFSET;
        if mx >= done_x
            && mx < done_x + DONE_BUTTON_WIDTH
            && my >= done_y
            && my < done_y + DEFAULT_HEIGHT
        {
            click_sound_sink::record_button_click();
            self.pending_action = Some(ControlsAction::Done);
            return true;
        }
        let pressed: Vec<(usize, bool, ScreenRectangle)> = self
            .rows
            .iter()
            .filter_map(|row| {
                let mut row = row.borrow_mut();
                row.takes_press()
                    .then(|| (row.option_index, row.is_slider, row.rect))
            })
            .collect();
        for (option_index, is_slider, rect) in pressed {
            if is_slider {
                click_sound_sink::record(None);
                self.focus_slider_from_mouse(option_index, event.x as f32, rect.left(), rect.width);
            } else {
                let key = self.options[option_index].key.clone();
                match key.as_str() {
                    "mouseSettings" => {
                        click_sound_sink::record_button_click();
                        self.pending_action = Some(ControlsAction::MouseSettingsClicked);
                    }
                    "keybinds" => {
                        click_sound_sink::record_button_click();
                        self.pending_action = Some(ControlsAction::KeybindsClicked);
                    }
                    "toggleCrouch" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                        self.pending_action = Some(ControlsAction::OptionChanged);
                    }
                    "toggleSprint" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                        self.pending_action = Some(ControlsAction::OptionChanged);
                    }
                    "toggleAttack" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                        self.pending_action = Some(ControlsAction::OptionChanged);
                    }
                    "toggleUse" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                        self.pending_action = Some(ControlsAction::OptionChanged);
                    }
                    "autoJump" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                        self.pending_action = Some(ControlsAction::OptionChanged);
                    }
                    "operatorItemsTab" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                        self.pending_action = Some(ControlsAction::OptionChanged);
                    }
                    _ => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                        self.pending_action = Some(ControlsAction::OptionChanged);
                    }
                }
            }
        }
        true
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        let handled = crate::framework::screen::screen_mouse_released(self, event);
        if self.dragging_slider_index.take().is_some() {
            click_sound_sink::record_button_click();
        }
        self.dragging_scrollbar = false;
        handled
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        let handled = crate::framework::screen::screen_mouse_dragged(self, event, dx, dy);
        if self.dragging_scrollbar {
            self.scrollbar_drag(dy);
            true
        } else if let Some(slider_idx) = self.dragging_slider_index {
            self.slider_drag(slider_idx, event.x as f32);
            true
        } else {
            handled
        }
    }
    fn mouse_scrolled(&mut self, _x: f64, _y: f64, _scroll_x: f64, scroll_y: f64) -> bool {
        let height = self.base_ref().height;
        let max_scroll = self.max_scroll_amount(height);
        self.scroll_y = (self.scroll_y - scroll_y as f32 * 12.0).clamp(0.0, max_scroll);
        self.reposition_rows();
        true
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
        self.ensure_layout(width, height);
        self.reposition_rows();
        self.base
            .extract_background_backdrop(graphics, self.in_world);
        let list_bg_tex = if self.in_world {
            "minecraft:gui/inworld_menu_list_background"
        } else {
            "minecraft:gui/menu_list_background"
        };
        let list_background = ResourceLocation::from_string(list_bg_tex);
        let mut y = HEADER_HEIGHT;
        while y < height - FOOTER_HEIGHT {
            let tile_h = 32.min(height - FOOTER_HEIGHT - y);
            let mut x = 0;
            while x < width {
                let tile_w = 32.min(width - x);
                graphics.blit_region(
                    list_background.clone(),
                    x,
                    y,
                    tile_w,
                    tile_h,
                    0.0,
                    (y as f32 + self.scroll_y) / 32.0,
                    tile_w as f32 / 32.0,
                    tile_h as f32 / 32.0,
                    0xFFFF_FFFFu32 as i32,
                );
                x += 32;
            }
            y += tile_h;
        }
        graphics.enable_scissor(0, HEADER_HEIGHT, width, height - FOOTER_HEIGHT);
        let layout_items = self.get_layout_items();
        let font = Arc::clone(&self.font);
        for item in layout_items {
            let item_y = HEADER_HEIGHT + 4 + item.relative_y - self.scroll_y.round() as i32;
            match item.kind {
                ControlsListItem::Header { label_key } => {
                    let header_text = crate::translate(label_key);
                    let text_y = if item.height == 13 {
                        item_y
                    } else {
                        item_y + 18
                    };
                    graphics.text(
                        ResourceLocation::from_string("minecraft:default"),
                        header_text,
                        width / 2 - 155,
                        text_y,
                        0xFFFF_FFFFu32 as i32,
                    );
                }
                ControlsListItem::Big { option_index } => {
                    let btn_x = width / 2 - 155;
                    let btn_y = item_y;
                    self.draw_option(
                        graphics,
                        font.as_ref(),
                        option_index,
                        btn_x,
                        btn_y,
                        BIG_BUTTON_WIDTH,
                    );
                }
                ControlsListItem::SmallPair {
                    option_index_a,
                    option_index_b,
                } => {
                    let btn_x_a = width / 2 - 155;
                    let btn_y = item_y;
                    self.draw_option(
                        graphics,
                        font.as_ref(),
                        option_index_a,
                        btn_x_a,
                        btn_y,
                        SMALL_BUTTON_WIDTH,
                    );
                    if let Some(option_index_b) = option_index_b {
                        let btn_x_b = width / 2 + 5;
                        self.draw_option(
                            graphics,
                            font.as_ref(),
                            option_index_b,
                            btn_x_b,
                            btn_y,
                            SMALL_BUTTON_WIDTH,
                        );
                    }
                }
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
        let title_text = crate::translate("controls.title");
        let title_w = font.measure_width(&title_text, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            title_text,
            (width - title_w) / 2,
            12,
            0xFFFF_FFFFu32 as i32,
        );
        let max_scroll = self.max_scroll_amount(height);
        if max_scroll > 0.0 {
            let scrollbar_x = width / 2 + 166;
            let scroll_area_h = height - 66;
            let bar_h = self.scrollbar_height(height);
            let bar_y = HEADER_HEIGHT
                + (self.scroll_y * (scroll_area_h - bar_h) as f32 / max_scroll).round() as i32;
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller_background"),
                scrollbar_x,
                HEADER_HEIGHT,
                6,
                scroll_area_h,
            );
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller"),
                scrollbar_x,
                bar_y,
                6,
                bar_h,
            );
        }
        let done_x = width / 2 - 100;
        let done_y = height - FOOTER_BUTTON_Y_OFFSET;
        let done_focused = self.done.borrow().is_focused();
        graphics.draw_button_with_focus(
            self.font.as_ref(),
            done_x,
            done_y,
            DONE_BUTTON_WIDTH,
            DEFAULT_HEIGHT,
            &crate::translate("gui.done"),
            true,
            done_focused,
        );
        let mouse_x = graphics.mouse_x();
        let mouse_y = graphics.mouse_y();
        let target = if mouse_y >= HEADER_HEIGHT && mouse_y < height - FOOTER_HEIGHT {
            self.hovered_row(mouse_x, mouse_y)
                .map(|(idx, rect)| (idx, rect, WidgetTooltipPosition::Menu))
        } else {
            None
        }
        .or_else(|| self.focused_row());
        if let Some((option_idx, rect, position)) = target
            && let Some(option) = self.options.get(option_idx)
            && let Some(lines) = crate::settings::option_tooltips::controls_tooltip(&option.key)
        {
            graphics.draw_widget_tooltip(
                self.font.as_ref(),
                &lines,
                mouse_x,
                mouse_y,
                rect,
                position,
            );
        }
    }
}
