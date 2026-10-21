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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SoundOutputChoice {
    pub handle: String,
    pub name: String,
}
#[derive(Clone, Debug)]
pub struct SoundOptionsOption {
    pub key: String,
    pub label: String,
    pub is_slider: bool,
    pub min_val: f32,
    pub max_val: f32,
    pub step: f32,
    pub value: f32,
}
impl SoundOptionsOption {
    pub fn is_available(&self) -> bool {
        match self.key.as_str() {
            "soundDevice" => self.max_val >= self.min_val,
            "directionalAudio" => self.max_val >= self.min_val,
            _ => true,
        }
    }
    pub fn get_display_string(&self) -> String {
        let name = crate::translate(&self.label);
        if !self.is_available() {
            let unavailable = crate::translate("eltharn.settings.option_unavailable");
            let unavailable = if unavailable == "eltharn.settings.option_unavailable" {
                "Unavailable in this build".to_string()
            } else {
                unavailable
            };
            return format!("{name}: {unavailable}");
        }
        match self.key.as_str() {
            "showSubtitles" | "directionalAudio" => {
                let state = if self.value > 0.5 {
                    crate::translate("options.on")
                } else {
                    crate::translate("options.off")
                };
                format!("{}: {}", name, state)
            }
            key if key.starts_with("soundCategory_") => {
                if self.value == 0.0 {
                    format!("{}: {}", name, crate::translate("options.off"))
                } else {
                    format!("{}: {}%", name, (self.value * 100.0) as i32)
                }
            }
            "soundDevice" => {
                format!(
                    "{}: {}",
                    name,
                    crate::translate("options.audioDevice.default")
                )
            }
            "musicFrequency" => {
                let val = self.value.round() as i32;
                let mode = match val {
                    0 => crate::translate("options.music_frequency.default"),
                    1 => crate::translate("options.music_frequency.frequent"),
                    2 => crate::translate("options.music_frequency.constant"),
                    _ => "".to_string(),
                };
                format!("{}: {}", name, mode)
            }
            "musicToast" => {
                let val = self.value.round() as i32;
                let mode = match val {
                    0 => crate::translate("options.musicToast.never"),
                    1 => crate::translate("options.musicToast.pauseMenu"),
                    2 => crate::translate("options.musicToast.pauseMenuAndToast"),
                    _ => "".to_string(),
                };
                format!("{}: {}", name, mode)
            }
            _ => format!("{}: {}", name, self.value),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundOptionsAction {
    Done,
}
#[derive(Clone, Debug)]
pub enum SoundOptionsListItem {
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
    pub kind: SoundOptionsListItem,
    pub relative_y: i32,
    pub height: i32,
}
pub struct SoundOptionsScreen {
    base: ScreenBase,
    done: Rc<RefCell<Button<SoundOptionsAction>>>,
    rows: Vec<Rc<RefCell<RowWidget>>>,
    pub scroll_y: f32,
    pub dragging_scrollbar: bool,
    pub dragging_slider_index: Option<usize>,
    pub focused_slider_index: Option<usize>,
    pub slider_can_change_value: bool,
    pub last_width: i32,
    pub last_height: i32,
    pub options: Vec<SoundOptionsOption>,
    audio_output_choices: Vec<SoundOutputChoice>,
    audio_output_changed: bool,
    pub in_world: bool,
    value_changed: bool,
    font: Arc<dyn FontWidthMeasurer>,
    pending_action: Option<SoundOptionsAction>,
}
struct RowWidget {
    rect: ScreenRectangle,
    option_index: usize,
    is_slider: bool,
    enabled: bool,
    pressed: bool,
    focused: bool,
}
impl RowWidget {
    fn new(rect: ScreenRectangle, option_index: usize, is_slider: bool, enabled: bool) -> Self {
        Self {
            rect,
            option_index,
            is_slider,
            enabled,
            pressed: false,
            focused: false,
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
        if row.enabled && row.rect.contains_point(event.x as i32, event.y as i32) {
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
            row.focused = focused && row.enabled;
        }
    }
    fn is_focused(&self) -> bool {
        self.with_row(|row| row.enabled && row.focused, false)
    }
    fn rectangle(&self) -> ScreenRectangle {
        self.with_row(|row| row.rect, ScreenRectangle::empty())
    }
    fn next_focus_path(&self, _event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.with_row(|row| row.enabled.then_some(ComponentPath::leaf(0)), None)
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.with_row(
            |row| {
                if !row.enabled || row.is_slider {
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
    button: Rc<RefCell<Button<SoundOptionsAction>>>,
}
impl FooterButtonHandle {
    fn new(button: Rc<RefCell<Button<SoundOptionsAction>>>) -> Self {
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
impl SoundOptionsScreen {
    pub fn new(
        in_world: bool,
        extra_options: &HashMap<String, String>,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let mut options = Vec::new();
        let _get_bool = |key: &str, default: bool| -> bool {
            extra_options
                .get(key)
                .map(|s| s == "true")
                .unwrap_or(default)
        };
        let _get_slider = |key: &str, default: f32, min_val: f32, max_val: f32| -> f32 {
            if let Some(s) = extra_options.get(key) {
                if let Ok(v) = s.parse::<f32>() {
                    let real_val = if v > 1.0 && max_val > 1.0 {
                        v
                    } else {
                        min_val + v.clamp(0.0, 1.0) * (max_val - min_val)
                    };
                    real_val.clamp(min_val, max_val)
                } else {
                    default
                }
            } else {
                default
            }
        };
        options.push(SoundOptionsOption {
            key: "soundCategory_master".to_string(),
            label: "soundCategory.master".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("soundCategory_master", 1.0, 0.0, 1.0),
        });
        options.push(SoundOptionsOption {
            key: "soundCategory_music".to_string(),
            label: "soundCategory.music".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("soundCategory_music", 1.0, 0.0, 1.0),
        });
        options.push(SoundOptionsOption {
            key: "soundCategory_record".to_string(),
            label: "soundCategory.record".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("soundCategory_record", 1.0, 0.0, 1.0),
        });
        options.push(SoundOptionsOption {
            key: "soundCategory_weather".to_string(),
            label: "soundCategory.weather".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("soundCategory_weather", 1.0, 0.0, 1.0),
        });
        options.push(SoundOptionsOption {
            key: "soundCategory_block".to_string(),
            label: "soundCategory.block".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("soundCategory_block", 1.0, 0.0, 1.0),
        });
        options.push(SoundOptionsOption {
            key: "soundCategory_hostile".to_string(),
            label: "soundCategory.hostile".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("soundCategory_hostile", 1.0, 0.0, 1.0),
        });
        options.push(SoundOptionsOption {
            key: "soundCategory_neutral".to_string(),
            label: "soundCategory.neutral".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("soundCategory_neutral", 1.0, 0.0, 1.0),
        });
        options.push(SoundOptionsOption {
            key: "soundCategory_player".to_string(),
            label: "soundCategory.player".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("soundCategory_player", 1.0, 0.0, 1.0),
        });
        options.push(SoundOptionsOption {
            key: "soundCategory_ambient".to_string(),
            label: "soundCategory.ambient".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("soundCategory_ambient", 1.0, 0.0, 1.0),
        });
        options.push(SoundOptionsOption {
            key: "soundCategory_voice".to_string(),
            label: "soundCategory.voice".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("soundCategory_voice", 1.0, 0.0, 1.0),
        });
        options.push(SoundOptionsOption {
            key: "soundCategory_ui".to_string(),
            label: "soundCategory.ui".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("soundCategory_ui", 1.0, 0.0, 1.0),
        });
        options.push(SoundOptionsOption {
            key: "soundDevice".to_string(),
            label: "options.audioDevice".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: -1.0,
            step: 1.0,
            value: 0.0,
        });
        options.push(SoundOptionsOption {
            key: "showSubtitles".to_string(),
            label: "options.showSubtitles".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("showSubtitles", false) {
                1.0
            } else {
                0.0
            },
        });
        options.push(SoundOptionsOption {
            key: "directionalAudio".to_string(),
            label: "options.directionalAudio".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: -1.0,
            step: 1.0,
            value: if _get_bool("directionalAudio", false) {
                1.0
            } else {
                0.0
            },
        });
        let music_frequency_val = match extra_options.get("musicFrequency").map(|s| s.as_str()) {
            Some("DEFAULT") => 0.0,
            Some("FREQUENT") => 1.0,
            Some("CONSTANT") => 2.0,
            _ => 0.0,
        };
        options.push(SoundOptionsOption {
            key: "musicFrequency".to_string(),
            label: "options.music_frequency".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 2.0,
            step: 1.0,
            value: music_frequency_val,
        });
        let music_toast_val = match extra_options.get("musicToast").map(|s| s.as_str()) {
            Some("never") => 0.0,
            Some("pause") => 1.0,
            Some("pause_and_toast") => 2.0,
            _ => 0.0,
        };
        options.push(SoundOptionsOption {
            key: "musicToast".to_string(),
            label: "options.musicToast".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 2.0,
            step: 1.0,
            value: music_toast_val,
        });
        let done = Rc::new(RefCell::new(
            Button::<SoundOptionsAction>::builder(Component::literal(&crate::translate(
                "gui.done",
            )))
            .bounds(0, 0, DONE_BUTTON_WIDTH, DEFAULT_HEIGHT)
            .with_action(SoundOptionsAction::Done)
            .with_font(Arc::clone(&font))
            .build(),
        ));
        Self {
            base: ScreenBase::new(Component::literal(&crate::translate(
                "options.sounds.title",
            ))),
            done,
            rows: Vec::new(),
            scroll_y: 0.0,
            dragging_scrollbar: false,
            dragging_slider_index: None,
            focused_slider_index: None,
            slider_can_change_value: false,
            last_width: 0,
            last_height: 0,
            options,
            audio_output_choices: Vec::new(),
            audio_output_changed: false,
            in_world,
            value_changed: false,
            font,
            pending_action: None,
        }
    }
    pub fn take_value_changed(&mut self) -> bool {
        std::mem::take(&mut self.value_changed)
    }
    pub fn set_directional_audio_supported(&mut self, supported: bool) {
        if let Some(option) = self
            .options
            .iter_mut()
            .find(|option| option.key == "directionalAudio")
        {
            option.max_val = if supported { 1.0 } else { -1.0 };
        }
    }
    pub fn set_audio_output_choices(
        &mut self,
        choices: Vec<SoundOutputChoice>,
        preferred_name: &str,
    ) {
        self.audio_output_choices = choices;
        self.audio_output_changed = false;
        let option = &mut self.options[11];
        if self.audio_output_choices.is_empty() {
            option.min_val = 0.0;
            option.max_val = -1.0;
            option.value = 0.0;
            return;
        }
        option.min_val = 0.0;
        option.max_val = (self.audio_output_choices.len() - 1) as f32;
        option.value = self
            .audio_output_choices
            .iter()
            .position(|choice| {
                if preferred_name.is_empty() {
                    choice.handle.is_empty()
                } else {
                    choice.name == preferred_name
                }
            })
            .unwrap_or(0) as f32;
    }
    pub fn selected_audio_output_handle(&self) -> Option<&str> {
        if !self.options[11].is_available() {
            return None;
        }
        self.audio_output_choices
            .get(self.options[11].value.round() as usize)
            .map(|choice| choice.handle.as_str())
    }
    pub fn take_audio_output_changed(&mut self) -> bool {
        std::mem::take(&mut self.audio_output_changed)
    }
    fn option_display_string(&self, option_index: usize) -> String {
        let Some(option) = self.options.get(option_index) else {
            return String::new();
        };
        if option.key == "soundDevice" && option.is_available() {
            let name = crate::translate(&option.label);
            let choice = self
                .audio_output_choices
                .get(option.value.round() as usize)
                .map(|choice| choice.name.as_str())
                .unwrap_or("");
            format!("{name}: {choice}")
        } else {
            option.get_display_string()
        }
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    fn reposition_rows(&mut self) {
        let scroll = self.scroll_y.round() as i32;
        let mut row_idx = 0;
        for item in self.get_layout_items() {
            let item_y = HEADER_HEIGHT + 4 + item.relative_y - scroll;
            match item.kind {
                SoundOptionsListItem::Header { .. } => {}
                SoundOptionsListItem::Big { .. } => {
                    if let Some(row) = self.rows.get(row_idx) {
                        row.borrow_mut().rect.position.y = item_y;
                        row_idx += 1;
                    }
                }
                SoundOptionsListItem::SmallPair { option_index_b, .. } => {
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
    pub fn take_action(&mut self) -> Option<SoundOptionsAction> {
        self.pending_action.take()
    }
    pub fn get_layout_items(&self) -> Vec<LayoutItem> {
        let list_items = vec![
            SoundOptionsListItem::Big { option_index: 0 },
            SoundOptionsListItem::SmallPair {
                option_index_a: 1,
                option_index_b: Some(2),
            },
            SoundOptionsListItem::SmallPair {
                option_index_a: 3,
                option_index_b: Some(4),
            },
            SoundOptionsListItem::SmallPair {
                option_index_a: 5,
                option_index_b: Some(6),
            },
            SoundOptionsListItem::SmallPair {
                option_index_a: 7,
                option_index_b: Some(8),
            },
            SoundOptionsListItem::SmallPair {
                option_index_a: 9,
                option_index_b: Some(10),
            },
            SoundOptionsListItem::Big { option_index: 11 },
            SoundOptionsListItem::SmallPair {
                option_index_a: 12,
                option_index_b: Some(13),
            },
            SoundOptionsListItem::SmallPair {
                option_index_a: 14,
                option_index_b: Some(15),
            },
        ];
        let mut items = Vec::new();
        let mut current_y = 0;
        for (i, kind) in list_items.into_iter().enumerate() {
            let height = match &kind {
                SoundOptionsListItem::Header { .. } => {
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
    fn update_slider_from_mouse(
        &mut self,
        option_idx: usize,
        mouse_x: f32,
        slider_x: i32,
        slider_w: i32,
    ) {
        let span = (slider_w - 8) as f32;
        if !self
            .options
            .get(option_idx)
            .is_some_and(SoundOptionsOption::is_available)
        {
            return;
        }
        let opt = &mut self.options[option_idx];
        let pct = if span > 0.0 {
            (mouse_x - (slider_x + 4) as f32) / span
        } else {
            0.0
        };
        let value = (opt.min_val + pct.clamp(0.0, 1.0) * (opt.max_val - opt.min_val))
            .clamp(opt.min_val, opt.max_val);
        if opt.value != value {
            opt.value = value;
            self.value_changed = true;
        }
    }
    fn cycle_option(&mut self, option_idx: usize) {
        if !self
            .options
            .get(option_idx)
            .is_some_and(SoundOptionsOption::is_available)
        {
            return;
        }
        let opt = &mut self.options[option_idx];
        let old_value = opt.value;
        opt.value += opt.step;
        if opt.value > opt.max_val {
            opt.value = opt.min_val;
        }
        if option_idx == 11 && opt.value != old_value {
            self.audio_output_changed = true;
        }
        self.value_changed = true;
    }
    pub fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        _font: &dyn FontWidthMeasurer,
    ) {
        <Self as Screen>::extract_render_state(self, graphics, 0, 0, 0.0);
    }
    fn draw_option(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
        option_index: usize,
        x: i32,
        y: i32,
        w: i32,
    ) {
        let display = self.option_display_string(option_index);
        let Some(option) = self.options.get(option_index) else {
            return;
        };
        let active = option.is_available();
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
                &display,
                pct,
                active,
                self.focused_slider_index == Some(option_index),
                self.slider_can_change_value && self.focused_slider_index == Some(option_index),
                self.dragging_slider_index == Some(option_index),
            );
        } else {
            graphics.draw_button(font, x, y, w, ROW_HEIGHT, &display, active);
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
            let item_y = HEADER_HEIGHT + 4 + item.relative_y;
            match item.kind {
                SoundOptionsListItem::Header { .. } => {}
                SoundOptionsListItem::Big { option_index } => {
                    self.push_row(width / 2 - 155, item_y, BIG_BUTTON_WIDTH, option_index);
                }
                SoundOptionsListItem::SmallPair {
                    option_index_a,
                    option_index_b,
                } => {
                    self.push_row(width / 2 - 155, item_y, SMALL_BUTTON_WIDTH, option_index_a);
                    if let Some(option_index_b) = option_index_b {
                        self.push_row(width / 2 + 5, item_y, SMALL_BUTTON_WIDTH, option_index_b);
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
        let enabled = self
            .options
            .get(option_index)
            .is_some_and(SoundOptionsOption::is_available);
        self.rows.push(Rc::new(RefCell::new(RowWidget::new(
            ScreenRectangle::new(x, y, width, ROW_HEIGHT),
            option_index,
            is_slider,
            enabled,
        ))));
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
            .map(|row| row.borrow().rect)
            .unwrap_or_else(ScreenRectangle::empty);
        self.update_slider_from_mouse(option_index, mouse_x, rect.left(), rect.width);
    }
}
impl Screen for SoundOptionsScreen {
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
            self.pending_action = Some(SoundOptionsAction::Done);
            return true;
        }
        let focused_idx = self.base_ref().focused_index();
        if let Some(index) = focused_idx {
            if index == 0 {
                if event.is_selection() {
                    self.pending_action = Some(SoundOptionsAction::Done);
                    click_sound_sink::record_button_click();
                    return true;
                }
            } else if let Some(row) = self.rows.get(index - 1) {
                let (option_index, is_slider, slider_width) = {
                    let row = row.borrow();
                    (row.option_index, row.is_slider, row.rect.width)
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
                            let slider_span = slider_width - 8;
                            if span > 0.0 && slider_span > 0 {
                                let step = span / slider_span as f32;
                                opt.value =
                                    (opt.value + dir * step).clamp(opt.min_val, opt.max_val);
                                self.value_changed = true;
                                return true;
                            }
                        }
                    }
                } else if event.is_selection() {
                    click_sound_sink::record_button_click();
                    self.cycle_option(option_index);
                    self.value_changed = true;
                    return true;
                }
            }
        }
        let handled = crate::framework::screen::screen_key_pressed(self, event);
        if handled || (event.key == KeyEvent::KEY_TAB && self.base_ref().focused_index().is_some())
        {
            return true;
        }
        false
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
            return true;
        }
        if let Some(action) = self.done.borrow_mut().take_press_action() {
            self.pending_action = Some(action);
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
                self.focused_slider_index = Some(option_index);
                self.slider_can_change_value = true;
                self.dragging_slider_index = Some(option_index);
                self.update_slider_from_mouse(
                    option_index,
                    event.x as f32,
                    rect.left(),
                    rect.width,
                );
                self.value_changed = true;
            } else {
                click_sound_sink::record_button_click();
                self.cycle_option(option_index);
                self.value_changed = true;
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
            let item_y = HEADER_HEIGHT + 4 + item.relative_y - self.scroll_y as i32;
            match item.kind {
                SoundOptionsListItem::Header { label_key } => {
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
                SoundOptionsListItem::Big { option_index } => {
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
                SoundOptionsListItem::SmallPair {
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
            31,
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
        let title_text = crate::translate("options.sounds.title");
        let title_w = self.font.measure_width(&title_text, 1.0).round() as i32;
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
            && let Some(lines) =
                crate::settings::option_tooltips::sound_tooltip(&option.key, option.value)
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
