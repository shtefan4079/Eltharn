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
use std::rc::Rc;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct ChatOptionsOption {
    pub key: String,
    pub label: String,
    pub is_slider: bool,
    pub min_val: f32,
    pub max_val: f32,
    pub step: f32,
    pub value: f32,
}
impl ChatOptionsOption {
    pub fn get_display_string(&self) -> String {
        let name = crate::translate(&self.label);
        match self.key.as_str() {
            "chatColors" | "chatLinks" | "chatLinksPrompt" | "autoSuggestions"
            | "hideMatchedNames" | "reducedDebugInfo" | "onlyShowSecureChat" | "saveChatDrafts" => {
                let state = if self.value > 0.5 {
                    crate::translate("options.on")
                } else {
                    crate::translate("options.off")
                };
                format!("{}: {}", name, state)
            }
            "chatVisibility" => {
                let val = self.value.round() as i32;
                let mode = match val {
                    0 => crate::translate("options.chat.visibility.full"),
                    1 => crate::translate("options.chat.visibility.system"),
                    2 => crate::translate("options.chat.visibility.hidden"),
                    _ => "".to_string(),
                };
                format!("{}: {}", name, mode)
            }
            "chatOpacity" => {
                let pct = ((self.value * 0.9 + 0.1) * 100.0).round() as i32;
                format!("{}: {}%", name, pct)
            }
            "textBackgroundOpacity" => {
                let pct = (self.value * 100.0).round() as i32;
                format!("{}: {}%", name, pct)
            }
            "chatScale" => {
                let pct = (self.value * 100.0).round() as i32;
                if pct == 0 {
                    format!("{}: {}", name, crate::translate("options.off"))
                } else {
                    format!("{}: {}%", name, pct)
                }
            }
            "chatLineSpacing" => {
                let pct = (self.value * 100.0).round() as i32;
                format!("{}: {}%", name, pct)
            }
            "chatDelay" => {
                let val = self.value;
                if val <= 0.0 {
                    crate::translate("options.chat.delay_none")
                } else {
                    let formatted = format!("{:.1}", val);
                    let translation = crate::translate("options.chat.delay");
                    translation.replace("%s", &formatted)
                }
            }
            "chatWidth" => {
                let val = (self.value * 280.0 + 40.0).floor() as i32;
                format!("{}: {}px", name, val)
            }
            "chatHeightFocused" => {
                let val = (self.value * 160.0 + 20.0).floor() as i32;
                format!("{}: {}px", name, val)
            }
            "chatHeightUnfocused" => {
                let val = (self.value * 160.0 + 20.0).floor() as i32;
                format!("{}: {}px", name, val)
            }
            "narrator" => {
                let val = self.value.round() as i32;
                let mode = match val {
                    0 => crate::translate("options.narrator.off"),
                    1 => crate::translate("options.narrator.all"),
                    2 => crate::translate("options.narrator.chat"),
                    3 => crate::translate("options.narrator.system"),
                    _ => "".to_string(),
                };
                format!("{}: {}", name, mode)
            }
            _ => format!("{}: {}", name, self.value),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatOptionsAction {
    Done,
}
#[derive(Clone, Debug)]
pub enum ChatOptionsListItem {
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
    pub kind: ChatOptionsListItem,
    pub relative_y: i32,
    pub height: i32,
}
pub struct ChatOptionsScreen {
    base: ScreenBase,
    rows: Vec<Rc<RefCell<RowWidget>>>,
    pub scroll_y: f32,
    pub dragging_scrollbar: bool,
    pub dragging_slider_index: Option<usize>,
    pub last_width: i32,
    pub last_height: i32,
    pub options: Vec<ChatOptionsOption>,
    pub in_world: bool,
    font: Arc<dyn crate::FontWidthMeasurer>,
    pending_action: Option<ChatOptionsAction>,
}
struct RowWidget {
    rect: ScreenRectangle,
    option_index: usize,
    is_slider: bool,
    pressed: bool,
}
impl RowWidget {
    fn new(rect: ScreenRectangle, option_index: usize, is_slider: bool) -> Self {
        Self {
            rect,
            option_index,
            is_slider,
            pressed: false,
        }
    }
    fn takes_press(&mut self) -> bool {
        let pressed = self.pressed;
        self.pressed = false;
        pressed
    }
}
struct RowHandle {
    row: Rc<RefCell<RowWidget>>,
}
impl RowHandle {
    fn new(row: Rc<RefCell<RowWidget>>) -> Self {
        Self { row }
    }
    fn with_row<T>(&self, read: impl FnOnce(&RowWidget) -> T, fallback: T) -> T {
        match self.row.try_borrow() {
            Ok(row) => read(&row),
            Err(_) => fallback,
        }
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
        row.pressed = row.rect.contains_point(event.x as i32, event.y as i32);
        row.pressed
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.with_row(
            |row| row.rect.contains_point(mouse_x as i32, mouse_y as i32),
            false,
        )
    }
    fn set_focused(&mut self, _focused: bool) {}
    fn is_focused(&self) -> bool {
        false
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
impl ChatOptionsScreen {
    pub fn new(
        in_world: bool,
        extra_options: &std::collections::HashMap<String, String>,
        font: Arc<dyn crate::FontWidthMeasurer>,
    ) -> Self {
        let mut options = Vec::new();
        let _get_bool = |key: &str, default: bool| -> bool {
            extra_options
                .get(key)
                .map(|s| s == "true")
                .unwrap_or(default)
        };
        let _get_int = |key: &str, default: i32| -> i32 {
            extra_options
                .get(key)
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(default)
        };
        let _get_float = |key: &str, default: f32| -> f32 {
            extra_options
                .get(key)
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(default)
        };
        let _get_slider = |key: &str, default: f32, min_val: f32, max_val: f32| -> f32 {
            if let Some(s) = extra_options.get(key) {
                if let Ok(v) = s.parse::<f32>() {
                    v.clamp(min_val, max_val)
                } else {
                    default
                }
            } else {
                default
            }
        };
        let chat_visibility_val = match extra_options.get("chatVisibility").map(|s| s.as_str()) {
            Some("0") | Some("full") => 0.0,
            Some("1") | Some("system") => 1.0,
            Some("2") | Some("hidden") => 2.0,
            _ => 0.0,
        };
        options.push(ChatOptionsOption {
            key: "chatVisibility".to_string(),
            label: "options.chat.visibility".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 2.0,
            step: 1.0,
            value: chat_visibility_val,
        });
        options.push(ChatOptionsOption {
            key: "chatColors".to_string(),
            label: "options.chat.color".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("chatColors", true) {
                1.0
            } else {
                0.0
            },
        });
        options.push(ChatOptionsOption {
            key: "chatLinks".to_string(),
            label: "options.chat.links".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("chatLinks", true) {
                1.0
            } else {
                0.0
            },
        });
        options.push(ChatOptionsOption {
            key: "chatLinksPrompt".to_string(),
            label: "options.chat.links.prompt".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("chatLinksPrompt", true) {
                1.0
            } else {
                0.0
            },
        });
        options.push(ChatOptionsOption {
            key: "chatOpacity".to_string(),
            label: "options.chat.opacity".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("chatOpacity", 1.0, 0.0, 1.0),
        });
        options.push(ChatOptionsOption {
            key: "textBackgroundOpacity".to_string(),
            label: "options.accessibility.text_background_opacity".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("textBackgroundOpacity", 0.5, 0.0, 1.0),
        });
        options.push(ChatOptionsOption {
            key: "chatScale".to_string(),
            label: "options.chat.scale".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("chatScale", 1.0, 0.0, 1.0),
        });
        options.push(ChatOptionsOption {
            key: "chatLineSpacing".to_string(),
            label: "options.chat.line_spacing".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("chatLineSpacing", 0.0, 0.0, 1.0),
        });
        options.push(ChatOptionsOption {
            key: "chatDelay".to_string(),
            label: "options.chat.delay_instant".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 6.0,
            step: 0.1,
            value: _get_slider("chatDelay", 0.0, 0.0, 6.0),
        });
        options.push(ChatOptionsOption {
            key: "chatWidth".to_string(),
            label: "options.chat.width".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("chatWidth", 1.0, 0.0, 1.0),
        });
        options.push(ChatOptionsOption {
            key: "chatHeightFocused".to_string(),
            label: "options.chat.height.focused".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("chatHeightFocused", 1.0, 0.0, 1.0),
        });
        options.push(ChatOptionsOption {
            key: "chatHeightUnfocused".to_string(),
            label: "options.chat.height.unfocused".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("chatHeightUnfocused", 0.4375, 0.0, 1.0),
        });
        let narrator_val = match extra_options.get("narrator").map(|s| s.as_str()) {
            Some("0") | Some("off") => 0.0,
            Some("1") | Some("all") => 1.0,
            Some("2") | Some("chat") => 2.0,
            Some("3") | Some("system") => 3.0,
            _ => 0.0,
        };
        options.push(ChatOptionsOption {
            key: "narrator".to_string(),
            label: "options.narrator".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 3.0,
            step: 1.0,
            value: narrator_val,
        });
        options.push(ChatOptionsOption {
            key: "autoSuggestions".to_string(),
            label: "options.autoSuggestCommands".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("autoSuggestions", true) {
                1.0
            } else {
                0.0
            },
        });
        options.push(ChatOptionsOption {
            key: "hideMatchedNames".to_string(),
            label: "options.hideMatchedNames".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("hideMatchedNames", true) {
                1.0
            } else {
                0.0
            },
        });
        options.push(ChatOptionsOption {
            key: "reducedDebugInfo".to_string(),
            label: "options.reducedDebugInfo".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("reducedDebugInfo", false) {
                1.0
            } else {
                0.0
            },
        });
        options.push(ChatOptionsOption {
            key: "onlyShowSecureChat".to_string(),
            label: "options.onlyShowSecureChat".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("onlyShowSecureChat", false) {
                1.0
            } else {
                0.0
            },
        });
        options.push(ChatOptionsOption {
            key: "saveChatDrafts".to_string(),
            label: "options.chat.drafts".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("saveChatDrafts", false) {
                1.0
            } else {
                0.0
            },
        });
        Self {
            base: ScreenBase::new(Component::literal(&crate::translate("options.chat.title"))),
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
                ChatOptionsListItem::Header { .. } => {}
                ChatOptionsListItem::Big { .. } => {
                    if let Some(row) = self.rows.get(row_idx) {
                        row.borrow_mut().rect.position.y = item_y;
                        row_idx += 1;
                    }
                }
                ChatOptionsListItem::SmallPair { option_index_b, .. } => {
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
        None
    }
    pub fn take_action(&mut self) -> Option<ChatOptionsAction> {
        self.pending_action.take()
    }
    pub fn row_rects(&self) -> Vec<ScreenRectangle> {
        self.rows.iter().map(|row| row.borrow().rect).collect()
    }
    pub fn get_layout_items(&self) -> Vec<LayoutItem> {
        let list_items = vec![
            ChatOptionsListItem::SmallPair {
                option_index_a: 0,
                option_index_b: Some(1),
            },
            ChatOptionsListItem::SmallPair {
                option_index_a: 2,
                option_index_b: Some(3),
            },
            ChatOptionsListItem::SmallPair {
                option_index_a: 4,
                option_index_b: Some(5),
            },
            ChatOptionsListItem::SmallPair {
                option_index_a: 6,
                option_index_b: Some(7),
            },
            ChatOptionsListItem::SmallPair {
                option_index_a: 8,
                option_index_b: Some(9),
            },
            ChatOptionsListItem::SmallPair {
                option_index_a: 10,
                option_index_b: Some(11),
            },
            ChatOptionsListItem::SmallPair {
                option_index_a: 12,
                option_index_b: Some(13),
            },
            ChatOptionsListItem::SmallPair {
                option_index_a: 14,
                option_index_b: Some(15),
            },
            ChatOptionsListItem::SmallPair {
                option_index_a: 16,
                option_index_b: Some(17),
            },
        ];
        let mut items = Vec::new();
        let mut current_y = 0;
        for (i, kind) in list_items.into_iter().enumerate() {
            let height = match &kind {
                ChatOptionsListItem::Header { .. } => {
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
        mx: i32,
        slider_x: i32,
        slider_w: i32,
    ) {
        let opt = &mut self.options[option_idx];
        let min_x = slider_x;
        let max_x = slider_x + slider_w - 8;
        let click_x = mx - 4;
        let pct = if max_x > min_x {
            (click_x - min_x) as f32 / (max_x - min_x) as f32
        } else {
            0.0
        };
        let clamped = pct.clamp(0.0, 1.0);
        let raw_val = opt.min_val + clamped * (opt.max_val - opt.min_val);
        let steps = ((raw_val - opt.min_val) / opt.step).round();
        opt.value = (opt.min_val + steps * opt.step).clamp(opt.min_val, opt.max_val);
    }
    fn cycle_option(&mut self, option_idx: usize) {
        let opt = &mut self.options[option_idx];
        opt.value += opt.step;
        if opt.value > opt.max_val {
            opt.value = opt.min_val;
        }
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
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
                ChatOptionsListItem::Header { label_key } => {
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
                ChatOptionsListItem::Big { option_index } => {
                    let btn_x = width / 2 - 155;
                    let btn_y = item_y;
                    self.draw_button(
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
                ChatOptionsListItem::SmallPair {
                    option_index_a,
                    option_index_b,
                } => {
                    let btn_x_a = width / 2 - 155;
                    let btn_y = item_y;
                    self.draw_button(
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
                        self.draw_button(
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
        let title_text = crate::translate("options.chat.title");
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
        graphics.draw_button(
            font,
            done_x,
            done_y,
            200,
            20,
            &crate::translate("gui.done"),
            true,
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
            && let Some(lines) = crate::settings::option_tooltips::chat_tooltip(&option.key)
        {
            graphics.draw_widget_tooltip(font, &lines, mouse_x, mouse_y, rect, position);
        }
    }
    fn draw_button(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
        option_idx: usize,
        x: i32,
        y: i32,
        w: i32,
        _mouse_x: i32,
        _mouse_y: i32,
    ) {
        let option = &self.options[option_idx];
        if option.is_slider {
            let pct = (option.value - option.min_val) / (option.max_val - option.min_val);
            graphics.draw_slider(
                font,
                x,
                y,
                w,
                20,
                &option.get_display_string(),
                pct,
                self.dragging_slider_index == Some(option_idx),
            );
        } else {
            graphics.draw_button(font, x, y, w, 20, &option.get_display_string(), true);
        }
    }
    fn ensure_layout(&mut self, width: i32, height: i32) {
        if self.last_width == width && self.last_height == height && !self.rows.is_empty() {
            return;
        }
        self.last_width = width;
        self.last_height = height;
        let done = Button::<ChatOptionsAction>::builder(el_core::Component::literal(
            &crate::translate("gui.done"),
        ))
        .bounds(width / 2 - 100, height - 26, 200, DEFAULT_HEIGHT)
        .with_action(ChatOptionsAction::Done)
        .with_font(Arc::clone(&self.font))
        .build();
        self.base.clear_widgets();
        self.base.add_widget(done);
        self.rows.clear();
        for item in self.get_layout_items() {
            let item_y = 33 + 4 + item.relative_y;
            match item.kind {
                ChatOptionsListItem::Header { .. } => {}
                ChatOptionsListItem::Big { option_index } => {
                    self.push_row(width / 2 - 155, item_y, 310, option_index);
                }
                ChatOptionsListItem::SmallPair {
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
        self.rows.push(Rc::new(RefCell::new(RowWidget::new(
            ScreenRectangle::new(x, y, width, 20),
            option_index,
            is_slider,
        ))));
    }
}
impl Screen for ChatOptionsScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        self.last_width = 0;
        self.last_height = 0;
        self.rows.clear();
        self.ensure_layout(width, height);
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
        let font = Arc::clone(&self.font);
        ChatOptionsScreen::extract_render_state(self, graphics, font.as_ref());
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if event.is_escape() {
            self.pending_action = Some(ChatOptionsAction::Done);
            return true;
        }
        if !event.is_selection() {
            return false;
        }
        let Some(index) = self.base_ref().focused_index() else {
            return false;
        };
        let Some(row) = self.rows.get(index - 1) else {
            return false;
        };
        let (option_index, is_slider) = {
            let row = row.borrow();
            (row.option_index, row.is_slider)
        };
        if is_slider {
            return false;
        }
        click_sound_sink::record_button_click();
        self.cycle_option(option_index);
        true
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
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
            if my < bar_y {
                self.scroll_y = (self.scroll_y - 25.0).max(0.0);
            } else if my > bar_y + bar_h {
                self.scroll_y = (self.scroll_y + 25.0).min(max_scroll);
            }
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
                self.update_slider_from_mouse(
                    option_index,
                    event.x as i32,
                    rect.left(),
                    rect.width,
                );
            } else {
                click_sound_sink::record_button_click();
                self.cycle_option(option_index);
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
        } else if let Some(slider_idx) = self.dragging_slider_index {
            self.slider_drag(slider_idx, event.x);
        }
        handled
    }
    fn mouse_scrolled(&mut self, _x: f64, _y: f64, _scroll_x: f64, scroll_y: f64) -> bool {
        let height = self.base_ref().height;
        let max_scroll = self.max_scroll_amount(height);
        self.scroll_y = (self.scroll_y - scroll_y as f32 * 12.0).clamp(0.0, max_scroll);
        self.reposition_rows();
        true
    }
}
impl ChatOptionsScreen {
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
        self.update_slider_from_mouse(option_index, mouse_x as i32, rect.left(), rect.width);
    }
}
