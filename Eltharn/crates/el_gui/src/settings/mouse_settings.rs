use crate::FontWidthMeasurer;
use crate::components::click_sound_sink;
use crate::components::slider_button::SliderInputKind;
use crate::components::{
    Button, DEFAULT_HEIGHT, GuiEventListener, Renderable, TabOrderedElement, WidgetClickSound,
};
use crate::framework::graphics::{GuiGraphicsExtractor, WidgetTooltipPosition};
use crate::framework::layouts::LayoutElement;
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
const MOUSE_WHEEL_MIN_INDEX: i32 = -200;
const MOUSE_WHEEL_MAX_INDEX: i32 = 100;
fn mouse_wheel_index_from_value(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    (value.clamp(0.01, 10.0).log10() * 100.0)
        .floor()
        .clamp(MOUSE_WHEEL_MIN_INDEX as f64, MOUSE_WHEEL_MAX_INDEX as f64) as i32
}
fn mouse_wheel_value_from_slider(slider: f64) -> f64 {
    let slider = slider.clamp(0.0, 1.0);
    let slider = if slider >= 1.0 {
        f64::from(0.99999_f32)
    } else {
        slider
    };
    let index = (MOUSE_WHEEL_MIN_INDEX as f64
        + slider * f64::from(MOUSE_WHEEL_MAX_INDEX - MOUSE_WHEEL_MIN_INDEX + 1))
    .floor() as i32;
    10.0_f64.powf(f64::from(index) / 100.0)
}
fn mouse_wheel_slider_from_value(value: f64) -> f64 {
    let index = mouse_wheel_index_from_value(value);
    match index {
        MOUSE_WHEEL_MIN_INDEX => 0.0,
        MOUSE_WHEEL_MAX_INDEX => 1.0,
        _ => {
            (f64::from(index) + 0.5 - f64::from(MOUSE_WHEEL_MIN_INDEX))
                / f64::from(MOUSE_WHEEL_MAX_INDEX - MOUSE_WHEEL_MIN_INDEX + 1)
        }
    }
}
#[derive(Clone, Debug)]
pub struct MouseSettingsOption {
    pub key: String,
    pub label: String,
    pub is_slider: bool,
    pub min_val: f64,
    pub max_val: f64,
    pub step: f64,
    pub value: f64,
}
impl MouseSettingsOption {
    pub fn get_display_string(&self) -> String {
        let name = crate::translate(&self.label);
        match self.key.as_str() {
            "discreteMouseScroll"
            | "invertMouseX"
            | "invertMouseY"
            | "allowCursorChanges"
            | "rawMouseInput" => {
                let state = if self.value > 0.5 {
                    crate::translate("options.on")
                } else {
                    crate::translate("options.off")
                };
                format!("{}: {}", name, state)
            }
            "sensitivity" => {
                if self.value == 0.0 {
                    format!("{}: {}", name, crate::translate("options.sensitivity.min"))
                } else if self.value == 1.0 {
                    format!("{}: {}", name, crate::translate("options.sensitivity.max"))
                } else {
                    let pct = (self.value * 200.0) as i32;
                    format!("{}: {}%", name, pct)
                }
            }
            "mouseWheelSensitivity" => format!("{}: {:.2}", name, self.value),
            _ => format!("{}: {}", name, self.value),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseSettingsAction {
    Done,
}
#[derive(Clone, Debug)]
pub enum MouseSettingsListItem {
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
    pub kind: MouseSettingsListItem,
    pub relative_y: i32,
    pub height: i32,
}
pub struct MouseSettingsScreen {
    base: ScreenBase,
    rows: Vec<Rc<RefCell<RowWidget>>>,
    pub scroll_y: f32,
    pub dragging_scrollbar: bool,
    pub dragging_slider_index: Option<usize>,
    pub last_width: i32,
    pub last_height: i32,
    pub options: Vec<MouseSettingsOption>,
    pub in_world: bool,
    font: Arc<dyn crate::FontWidthMeasurer>,
    pending_action: Option<MouseSettingsAction>,
}
struct RowWidget {
    rect: ScreenRectangle,
    option_index: usize,
    is_slider: bool,
    focused: bool,
    can_change_value: bool,
    last_input_kind: SliderInputKind,
    dragging: bool,
    pressed: bool,
    pending_action: Option<RowInputAction>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowInputAction {
    Activate,
    AdjustSlider(i8),
}
impl RowWidget {
    fn new(rect: ScreenRectangle, option_index: usize, is_slider: bool) -> Self {
        Self {
            rect,
            option_index,
            is_slider,
            focused: false,
            can_change_value: false,
            last_input_kind: SliderInputKind::Mouse,
            dragging: false,
            pressed: false,
            pending_action: None,
        }
    }
    fn takes_press(&mut self) -> bool {
        let pressed = self.pressed;
        self.pressed = false;
        pressed
    }
    fn take_pending_action(&mut self) -> Option<RowInputAction> {
        self.pending_action.take()
    }
    fn set_input_kind(&mut self, kind: SliderInputKind) {
        self.last_input_kind = kind;
    }
    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        if !focused {
            self.can_change_value = false;
        } else if self.last_input_kind.enables_value_change_on_focus() {
            self.can_change_value = true;
        }
    }
}
struct RowHandle {
    row: Rc<RefCell<RowWidget>>,
}
impl RowHandle {
    fn new(row: Rc<RefCell<RowWidget>>) -> Self {
        Self { row }
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
impl GuiEventListener for RowHandle {
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        match self.row.try_borrow() {
            Ok(row) => row.rect.contains_point(mouse_x as i32, mouse_y as i32),
            Err(_) => false,
        }
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        if event.button() != 0 {
            return false;
        }
        let Ok(mut row) = self.row.try_borrow_mut() else {
            return false;
        };
        if !row.rect.contains_point(event.x as i32, event.y as i32) {
            return false;
        }
        row.pressed = true;
        if row.is_slider {
            row.dragging = true;
        }
        true
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        if event.button() != 0 {
            return false;
        }
        match self.row.try_borrow_mut() {
            Ok(mut row) if row.is_slider && row.dragging => {
                row.dragging = false;
                true
            }
            _ => false,
        }
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, _dx: f64, _dy: f64) -> bool {
        event.button() == 0
            && self
                .row
                .try_borrow()
                .is_ok_and(|row| row.is_slider && row.dragging)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        let Ok(mut row) = self.row.try_borrow_mut() else {
            return false;
        };
        if row.is_slider {
            if event.is_selection() {
                row.can_change_value = !row.can_change_value;
                return true;
            }
            if row.can_change_value {
                let direction = match event.key {
                    KeyEvent::KEY_LEFT => -1,
                    KeyEvent::KEY_RIGHT => 1,
                    _ => return false,
                };
                row.pending_action = Some(RowInputAction::AdjustSlider(direction));
                return true;
            }
            return false;
        }
        if event.is_selection() {
            row.pending_action = Some(RowInputAction::Activate);
            return true;
        }
        false
    }
    fn set_focused(&mut self, focused: bool) {
        if let Ok(mut row) = self.row.try_borrow_mut() {
            row.set_focused(focused);
        }
    }
    fn is_focused(&self) -> bool {
        self.row.try_borrow().is_ok_and(|row| row.focused)
    }
    fn rectangle(&self) -> ScreenRectangle {
        match self.row.try_borrow() {
            Ok(row) => row.rect,
            Err(_) => ScreenRectangle::empty(),
        }
    }
    fn next_focus_path(&self, _event: &FocusNavigationEvent) -> Option<ComponentPath> {
        Some(ComponentPath::leaf(0))
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        match self.row.try_borrow() {
            Ok(row) if row.is_slider => None,
            _ => Some(WidgetClickSound::UiButtonClick),
        }
    }
}
impl crate::framework::narration::NarratableEntry for RowHandle {
    fn narration_priority(&self) -> crate::framework::narration::NarrationPriority {
        crate::framework::narration::NarrationPriority::None
    }
    fn update_narration(&self, _output: &mut crate::framework::narration::NarrationElementOutput) {}
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
impl RowHandle {
    fn with_row<T>(&self, read: impl FnOnce(&RowWidget) -> T, fallback: T) -> T {
        match self.row.try_borrow() {
            Ok(row) => read(&row),
            Err(_) => fallback,
        }
    }
}
impl MouseSettingsScreen {
    pub fn new(
        in_world: bool,
        extra_options: &std::collections::HashMap<String, String>,
        sensitivity: f64,
        mouse_wheel_sensitivity: f64,
        font: Arc<dyn crate::FontWidthMeasurer>,
    ) -> Self {
        let mut options = Vec::new();
        let _get_bool = |key: &str, default: bool| -> bool {
            extra_options
                .get(key)
                .map(|s| s == "true")
                .unwrap_or(default)
        };
        options.push(MouseSettingsOption {
            key: "sensitivity".to_string(),
            label: "options.sensitivity".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.0,
            value: if sensitivity.is_finite() && (0.0..=1.0).contains(&sensitivity) {
                sensitivity
            } else {
                0.5
            },
        });
        options.push(MouseSettingsOption {
            key: "mouseWheelSensitivity".to_string(),
            label: "options.mouseWheelSensitivity".to_string(),
            is_slider: true,
            min_val: 0.01,
            max_val: 10.0,
            step: 0.0,
            value: 10.0_f64
                .powf(f64::from(mouse_wheel_index_from_value(mouse_wheel_sensitivity)) / 100.0),
        });
        options.push(MouseSettingsOption {
            key: "discreteMouseScroll".to_string(),
            label: "options.discrete_mouse_scroll".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("discreteMouseScroll", false) {
                1.0
            } else {
                0.0
            },
        });
        options.push(MouseSettingsOption {
            key: "invertMouseX".to_string(),
            label: "options.invertMouseX".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("invertMouseX", false) {
                1.0
            } else {
                0.0
            },
        });
        options.push(MouseSettingsOption {
            key: "invertMouseY".to_string(),
            label: "options.invertMouseY".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("invertMouseY", false) {
                1.0
            } else {
                0.0
            },
        });
        options.push(MouseSettingsOption {
            key: "allowCursorChanges".to_string(),
            label: "options.allowCursorChanges".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("allowCursorChanges", true) {
                1.0
            } else {
                0.0
            },
        });
        options.push(MouseSettingsOption {
            key: "rawMouseInput".to_string(),
            label: "options.rawMouseInput".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("rawMouseInput", true) {
                1.0
            } else {
                0.0
            },
        });
        Self {
            base: ScreenBase::new(Component::literal(&crate::translate(
                "options.mouseSettings.title",
            ))),
            rows: Vec::new(),
            font,
            pending_action: None,
            scroll_y: 0.0,
            dragging_scrollbar: false,
            dragging_slider_index: None,
            last_width: 0,
            last_height: 0,
            options,
            in_world,
        }
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    fn reposition_rows(&mut self) {
        let scroll = self.scroll_y.round() as i32;
        let mut row_idx = 0;
        for item in self.get_layout_items() {
            let item_y = 33 + 4 + item.relative_y - scroll;
            match item.kind {
                MouseSettingsListItem::Header { .. } => {}
                MouseSettingsListItem::Big { .. } => {
                    if let Some(row) = self.rows.get(row_idx) {
                        row.borrow_mut().rect.position.y = item_y;
                        row_idx += 1;
                    }
                }
                MouseSettingsListItem::SmallPair { option_index_b, .. } => {
                    if let Some(row) = self.rows.get(row_idx) {
                        row.borrow_mut().rect.position.y = item_y;
                        row_idx += 1;
                    }
                    if option_index_b.is_some()
                        && let Some(row) = self.rows.get(row_idx)
                    {
                        row.borrow_mut().rect.position.y = item_y;
                        row_idx += 1;
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
    pub fn take_action(&mut self) -> Option<MouseSettingsAction> {
        self.pending_action.take()
    }
    pub fn row_rects(&self) -> Vec<ScreenRectangle> {
        self.rows.iter().map(|row| row.borrow().rect).collect()
    }
    pub fn get_layout_items(&self) -> Vec<LayoutItem> {
        let list_items = vec![
            MouseSettingsListItem::SmallPair {
                option_index_a: 0,
                option_index_b: Some(1),
            },
            MouseSettingsListItem::SmallPair {
                option_index_a: 2,
                option_index_b: Some(3),
            },
            MouseSettingsListItem::SmallPair {
                option_index_a: 4,
                option_index_b: Some(5),
            },
            MouseSettingsListItem::SmallPair {
                option_index_a: 6,
                option_index_b: None,
            },
        ];
        let mut items = Vec::new();
        let mut current_y = 0;
        for (i, kind) in list_items.into_iter().enumerate() {
            let height = match &kind {
                MouseSettingsListItem::Header { .. } => {
                    if i == 0 {
                        13
                    } else {
                        31
                    }
                }
                _ => 25,
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
    fn update_slider_from_mouse(
        &mut self,
        option_idx: usize,
        mouse_x: f64,
        slider_x: i32,
        slider_w: i32,
    ) {
        let usable_width = slider_w - 8;
        let slider_value = if usable_width > 0 {
            (mouse_x - f64::from(slider_x + 4)) / f64::from(usable_width)
        } else {
            0.0
        };
        self.update_slider_from_ratio(option_idx, slider_value);
    }
    fn update_slider_from_ratio(&mut self, option_idx: usize, slider_value: f64) {
        let opt = &mut self.options[option_idx];
        let clamped = slider_value.clamp(0.0, 1.0);
        if opt.key == "mouseWheelSensitivity" {
            opt.value = mouse_wheel_value_from_slider(clamped);
            return;
        }
        let raw_val = opt.min_val + clamped * (opt.max_val - opt.min_val);
        opt.value = if opt.step > 0.0 {
            let steps = ((raw_val - opt.min_val) / opt.step).round();
            (opt.min_val + steps * opt.step).clamp(opt.min_val, opt.max_val)
        } else {
            raw_val.clamp(opt.min_val, opt.max_val)
        };
    }
    fn update_slider_from_keyboard(&mut self, option_idx: usize, direction: i8, slider_width: i32) {
        let Some(option) = self.options.get(option_idx) else {
            return;
        };
        let slider_value = if option.key == "mouseWheelSensitivity" {
            mouse_wheel_slider_from_value(option.value)
        } else if option.max_val > option.min_val {
            ((option.value - option.min_val) / (option.max_val - option.min_val)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let usable_width = slider_width - 8;
        let direction = f32::from(direction);
        let delta = if usable_width > 0 {
            f64::from(direction / usable_width as f32)
        } else {
            f64::from(direction)
        };
        self.update_slider_from_ratio(option_idx, slider_value + delta);
    }
    fn set_row_input_kind(&mut self, kind: SliderInputKind) {
        for row in &self.rows {
            if let Ok(mut row) = row.try_borrow_mut() {
                row.set_input_kind(kind);
            }
        }
    }
    fn cycle_option(&mut self, option_idx: usize) {
        let opt = &mut self.options[option_idx];
        opt.value += opt.step;
        if opt.value > opt.max_val {
            opt.value = opt.min_val;
        }
    }
    fn render_menu(&mut self, graphics: &mut GuiGraphicsExtractor, font: &dyn FontWidthMeasurer) {
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
        let mut y = 33;
        while y < height - 33 {
            let tile_h = 32.min(height - 33 - y);
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
        graphics.enable_scissor(0, 33, width, height - 33);
        let layout_items = self.get_layout_items();
        let mouse_x = graphics.mouse_x();
        let mouse_y = graphics.mouse_y();
        for item in layout_items {
            let item_y = 33 + 4 + item.relative_y - self.scroll_y as i32;
            match item.kind {
                MouseSettingsListItem::Header { label_key } => {
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
                MouseSettingsListItem::Big { option_index } => {
                    let btn_x = width / 2 - 155;
                    let btn_y = item_y;
                    self.draw_option(
                        graphics,
                        font,
                        option_index,
                        btn_x,
                        btn_y,
                        310,
                        mouse_x,
                        mouse_y,
                    );
                }
                MouseSettingsListItem::SmallPair {
                    option_index_a,
                    option_index_b,
                } => {
                    let btn_x_a = width / 2 - 155;
                    let btn_y = item_y;
                    self.draw_option(
                        graphics,
                        font,
                        option_index_a,
                        btn_x_a,
                        btn_y,
                        150,
                        mouse_x,
                        mouse_y,
                    );
                    if let Some(option_index_b) = option_index_b {
                        let btn_x_b = width / 2 + 5;
                        self.draw_option(
                            graphics,
                            font,
                            option_index_b,
                            btn_x_b,
                            btn_y,
                            150,
                            mouse_x,
                            mouse_y,
                        );
                    }
                }
            }
        }
        graphics.disable_scissor();
        graphics.draw_menu_background(self.in_world, width, 33);
        graphics.pose_mut().push_pose();
        graphics.pose_mut().translate(0.0, (height - 33) as f32);
        graphics.draw_menu_background(self.in_world, width, 33);
        graphics.pose_mut().pop_pose();
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/header_separator"),
            0,
            31,
            width,
            2,
        );
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/footer_separator"),
            0,
            height - 33,
            width,
            2,
        );
        let title_text = crate::translate("options.mouse_settings.title");
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
            let bar_y =
                33 + (self.scroll_y * (scroll_area_h - bar_h) as f32 / max_scroll).round() as i32;
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller_background"),
                scrollbar_x,
                33,
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
        let done_y = height - 26;
        graphics.draw_button_with_focus(
            font,
            done_x,
            done_y,
            200,
            20,
            &crate::translate("gui.done"),
            true,
            self.base_ref().focused_index() == Some(0),
        );
        let target = if mouse_y >= 33 && mouse_y < height - 33 {
            self.hovered_row(mouse_x, mouse_y)
                .map(|(idx, rect)| (idx, rect, WidgetTooltipPosition::Menu))
        } else {
            None
        }
        .or_else(|| self.focused_row());
        if let Some((option_idx, rect, position)) = target
            && let Some(option) = self.options.get(option_idx)
            && let Some(lines) = crate::settings::option_tooltips::mouse_tooltip(&option.key)
        {
            graphics.draw_widget_tooltip(font, &lines, mouse_x, mouse_y, rect, position);
        }
    }
    fn draw_option(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
        option_index: usize,
        x: i32,
        y: i32,
        w: i32,
        mouse_x: i32,
        mouse_y: i32,
    ) {
        let Some(option) = self.options.get(option_index) else {
            return;
        };
        let (focused, can_change_value, dragging) = self
            .rows
            .iter()
            .find_map(|row| {
                row.try_borrow().ok().and_then(|row| {
                    (row.option_index == option_index).then_some((
                        row.focused,
                        row.can_change_value,
                        row.dragging,
                    ))
                })
            })
            .unwrap_or((false, false, false));
        if option.is_slider {
            let pct = if option.key == "mouseWheelSensitivity" {
                mouse_wheel_slider_from_value(option.value)
            } else if option.max_val > option.min_val {
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
                20,
                &option.get_display_string(),
                pct as f32,
                true,
                focused,
                can_change_value,
                dragging || self.dragging_slider_index == Some(option_index),
            );
        } else {
            let _ = (mouse_x, mouse_y);
            graphics.draw_button_with_focus(
                font,
                x,
                y,
                w,
                20,
                &option.get_display_string(),
                true,
                focused,
            );
        }
    }
    fn ensure_layout(&mut self, width: i32, height: i32) {
        if self.last_width == width && self.last_height == height && !self.rows.is_empty() {
            return;
        }
        self.last_width = width;
        self.last_height = height;
        let done = Button::<MouseSettingsAction>::builder(el_core::Component::literal(
            &crate::translate("gui.done"),
        ))
        .bounds(width / 2 - 100, height - 26, 200, DEFAULT_HEIGHT)
        .with_action(MouseSettingsAction::Done)
        .with_font(Arc::clone(&self.font))
        .build();
        self.base.clear_widgets();
        self.base.add_widget(done);
        self.rows.clear();
        for item in self.get_layout_items() {
            let item_y = 33 + 4 + item.relative_y;
            match item.kind {
                MouseSettingsListItem::Header { .. } => {}
                MouseSettingsListItem::Big { option_index } => {
                    self.push_row(width / 2 - 155, item_y, 310, option_index);
                }
                MouseSettingsListItem::SmallPair {
                    option_index_a,
                    option_index_b,
                } => {
                    self.push_row(width / 2 - 155, item_y, 150, option_index_a);
                    if let Some(option_index_b) = option_index_b {
                        self.push_row(width / 2 + 5, item_y, 150, option_index_b);
                    }
                }
            }
        }
        for row in &self.rows {
            self.base.add_widget(RowHandle::new(Rc::clone(row)));
        }
    }
    fn push_row(&mut self, x: i32, y: i32, width: i32, option_index: usize) {
        let is_slider = self
            .options
            .get(option_index)
            .is_some_and(|option| option.is_slider);
        let row = Rc::new(RefCell::new(RowWidget::new(
            ScreenRectangle::new(x, y, width, 20),
            option_index,
            is_slider,
        )));
        self.rows.push(row);
    }
}
impl Screen for MouseSettingsScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
        let font = Arc::clone(&self.font);
        self.render_menu(graphics, font.as_ref());
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
            self.pending_action = Some(MouseSettingsAction::Done);
            return true;
        }
        match event.key {
            KeyEvent::KEY_TAB => self.set_row_input_kind(SliderInputKind::KeyboardTab),
            KeyEvent::KEY_LEFT | KeyEvent::KEY_RIGHT | KeyEvent::KEY_UP | KeyEvent::KEY_DOWN => {
                self.set_row_input_kind(SliderInputKind::KeyboardArrow)
            }
            _ => {}
        }
        let handled = crate::framework::screen::screen_key_pressed(self, event);
        let Some(focused_index) = self.base_ref().focused_index() else {
            return handled;
        };
        if focused_index == 0 {
            if event.is_selection() {
                self.pending_action = Some(MouseSettingsAction::Done);
                return true;
            }
            return handled;
        }
        let Some(row) = self.rows.get(focused_index - 1) else {
            return handled;
        };
        let (option_index, rect, action) = {
            let mut row = row.borrow_mut();
            (row.option_index, row.rect, row.take_pending_action())
        };
        match action {
            Some(RowInputAction::Activate) => {
                click_sound_sink::record_button_click();
                self.cycle_option(option_index);
                true
            }
            Some(RowInputAction::AdjustSlider(direction)) => {
                self.update_slider_from_keyboard(option_index, direction, rect.width);
                true
            }
            None => handled,
        }
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.set_row_input_kind(SliderInputKind::Mouse);
        let handled = crate::framework::screen::screen_mouse_clicked(self, event, double_click);
        if !handled {
            return false;
        }
        let width = self.base_ref().width;
        let height = self.base_ref().height;
        let scrollbar_x = width / 2 + 166;
        let bar_h = self.scrollbar_height(height);
        let max_scroll = self.max_scroll_amount(height);
        let bar_y = if max_scroll == 0.0 {
            33
        } else {
            let scroll_area_h = height - 66;
            33 + (self.scroll_y * (scroll_area_h - bar_h) as f32 / max_scroll).round() as i32
        };
        let (mx, my) = (event.x as i32, event.y as i32);
        if event.button() == 0
            && mx >= scrollbar_x
            && mx < scrollbar_x + 6
            && my >= 33
            && my < height - 33
        {
            self.dragging_scrollbar = true;
            if (event.y as i32) < bar_y {
                self.scroll_y = (self.scroll_y - 25.0).max(0.0);
            } else if (event.y as i32) > bar_y + bar_h {
                self.scroll_y = (self.scroll_y + 25.0).min(max_scroll);
            }
            return true;
        }
        let done_x = width / 2 - 100;
        let done_y = height - 26;
        if event.button() == 0
            && event.x >= f64::from(done_x)
            && event.x < f64::from(done_x + 200)
            && event.y >= f64::from(done_y)
            && event.y < f64::from(done_y + 20)
        {
            self.pending_action = Some(MouseSettingsAction::Done);
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
                self.dragging_slider_index = Some(option_index);
                self.update_slider_from_mouse(option_index, event.x, rect.left(), rect.width);
            } else {
                click_sound_sink::record_button_click();
                self.cycle_option(option_index);
            }
        }
        true
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        let handled = crate::framework::screen::screen_mouse_released(self, event);
        if event.button() == 0 && self.dragging_slider_index.take().is_some() {
            click_sound_sink::record_button_click();
        }
        if event.button() == 0 {
            self.dragging_scrollbar = false;
        }
        handled
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        let handled = crate::framework::screen::screen_mouse_dragged(self, event, dx, dy);
        if self.dragging_scrollbar {
            self.scrollbar_drag(dy);
        } else if let Some(slider_idx) = self.dragging_slider_index {
            self.slider_drag(slider_idx, event.x);
        }
        handled
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        let _ = (x, y, scroll_x);
        let height = self.base_ref().height;
        let max_scroll = self.max_scroll_amount(height);
        self.scroll_y = (self.scroll_y - scroll_y as f32 * 12.0).clamp(0.0, max_scroll);
        self.reposition_rows();
        true
    }
}
impl MouseSettingsScreen {
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
    fn slider_drag(&mut self, option_index: usize, mouse_x: f64) {
        let rect = self
            .rows
            .iter()
            .find(|row| row.borrow().option_index == option_index)
            .map(|row| row.borrow().rect)
            .unwrap_or_else(ScreenRectangle::empty);
        self.update_slider_from_mouse(option_index, mouse_x, rect.left(), rect.width);
    }
}
