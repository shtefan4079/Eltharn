use crate::FontWidthMeasurer;
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::narration::{NarratedElementType, NarrationElementOutput};
use crate::framework::navigation::ScreenRectangle;
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{CharacterEvent, KeyEvent, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
use std::collections::BTreeSet;
use std::sync::Arc;
pub const CONTENT_TOP: i32 = 40;
pub const FOOTER_HEIGHT: i32 = 33;
pub const ROW_HEIGHT: i32 = 20;
pub const ROW_SPACING: i32 = 4;
pub const DONE_WIDTH: i32 = 200;
pub const SCROLLBAR_WIDTH: i32 = 6;
const CONTENT_WIDTH: i32 = 420;
const VALUE_WIDTH: i32 = 184;
const SCROLLBAR_MIN_THUMB: i32 = 20;
const MAX_EDIT_LENGTH: usize = 48;
#[derive(Clone, Debug, PartialEq)]
pub enum GameRuleValue {
    Boolean(bool),
    Integer(i64),
    Decimal(f64),
    String(String),
}
impl GameRuleValue {
    pub fn display_value(&self) -> String {
        match self {
            Self::Boolean(value) => {
                crate::translate(if *value { "options.on" } else { "options.off" })
            }
            Self::Integer(value) => value.to_string(),
            Self::Decimal(value) => value.to_string(),
            Self::String(value) => value.clone(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum GameRuleEditMetadata {
    Boolean,
    Integer {
        minimum: i64,
        maximum: i64,
        step: i64,
    },
    Decimal {
        minimum: f64,
        maximum: f64,
        step: f64,
    },
    Enum {
        values: Vec<String>,
    },
}
impl GameRuleEditMetadata {
    fn supports(&self, value: &GameRuleValue) -> bool {
        match (self, value) {
            (Self::Boolean, GameRuleValue::Boolean(_)) => true,
            (
                Self::Integer {
                    minimum,
                    maximum,
                    step,
                },
                GameRuleValue::Integer(_),
            ) => minimum <= maximum && *step > 0,
            (
                Self::Decimal {
                    minimum,
                    maximum,
                    step,
                },
                GameRuleValue::Decimal(value),
            ) => {
                minimum.is_finite()
                    && maximum.is_finite()
                    && step.is_finite()
                    && *minimum <= *maximum
                    && *step > 0.0
                    && value.is_finite()
            }
            (Self::Enum { values }, GameRuleValue::String(_)) => !values.is_empty(),
            _ => false,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct GameRuleRow {
    pub key: String,
    pub label: String,
    pub value: GameRuleValue,
    pub edit: Option<GameRuleEditMetadata>,
}
impl GameRuleRow {
    pub fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        value: GameRuleValue,
        edit: Option<GameRuleEditMetadata>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            value,
            edit,
        }
    }
    pub fn is_editable(&self) -> bool {
        self.edit
            .as_ref()
            .is_some_and(|metadata| metadata.supports(&self.value))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameRulesAction {
    Done,
    Cancel,
}
pub struct GameRulesScreen {
    base: ScreenBase,
    title: String,
    font: Arc<dyn FontWidthMeasurer>,
    rows: Vec<GameRuleRow>,
    original_values: Vec<GameRuleValue>,
    focused: usize,
    scroll_y: f32,
    dragging_scrollbar: bool,
    editing_row: Option<usize>,
    edit_buffer: String,
    edit_error: Option<String>,
    pending_action: Option<GameRulesAction>,
    in_world: bool,
    loading: bool,
}
impl GameRulesScreen {
    pub fn new(
        title: impl Into<String>,
        rows: Vec<GameRuleRow>,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let title = title.into();
        Self {
            base: ScreenBase::new(Component::literal(&title)),
            title,
            font,
            original_values: rows.iter().map(|row| row.value.clone()).collect(),
            rows,
            focused: 0,
            scroll_y: 0.0,
            dragging_scrollbar: false,
            editing_row: None,
            edit_buffer: String::new(),
            edit_error: None,
            pending_action: None,
            in_world: false,
            loading: false,
        }
    }
    pub fn set_in_world(&mut self, in_world: bool) {
        self.in_world = in_world;
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }
    pub fn rows(&self) -> &[GameRuleRow] {
        &self.rows
    }
    pub fn replace_rows(&mut self, rows: Vec<GameRuleRow>) {
        self.loading = false;
        self.original_values = rows.iter().map(|row| row.value.clone()).collect();
        self.rows = rows;
        self.editing_row = None;
        self.edit_buffer.clear();
        self.edit_error = None;
        self.scroll_y = self.scroll_y.min(self.max_scroll());
        self.focused = self.focused.min(self.rows.len());
    }
    pub fn changed_keys(&self) -> Vec<String> {
        self.rows
            .iter()
            .zip(&self.original_values)
            .filter(|(row, original)| row.value != **original)
            .map(|(row, _)| row.key.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    pub fn changed_rows(&self) -> Vec<GameRuleRow> {
        self.rows
            .iter()
            .zip(&self.original_values)
            .filter(|(row, original)| row.value != **original)
            .map(|(row, _)| row.clone())
            .collect()
    }
    pub fn mark_changes_accepted(&mut self) {
        self.original_values = self.rows.iter().map(|row| row.value.clone()).collect();
    }
    pub fn take_action(&mut self) -> Option<GameRulesAction> {
        self.pending_action.take()
    }
    pub fn take_edit_error(&mut self) -> Option<String> {
        self.edit_error.take()
    }
    pub fn focused_index(&self) -> usize {
        self.focused
    }
    pub fn scroll_amount(&self) -> f32 {
        self.scroll_y
    }
    pub fn is_editing(&self) -> bool {
        self.editing_row.is_some()
    }
    fn done_index(&self) -> usize {
        self.rows.len()
    }
    fn cancel_index(&self) -> usize {
        self.rows.len() + 1
    }
    fn row_stride(&self) -> i32 {
        ROW_HEIGHT + ROW_SPACING
    }
    fn content_height(&self) -> i32 {
        if self.rows.is_empty() {
            0
        } else {
            self.rows.len() as i32 * self.row_stride() - ROW_SPACING
        }
    }
    fn viewport_top(&self) -> i32 {
        CONTENT_TOP.min(self.base.height.saturating_sub(FOOTER_HEIGHT).max(0))
    }
    fn viewport_bottom(&self) -> i32 {
        (self.base.height - FOOTER_HEIGHT).max(self.viewport_top())
    }
    fn viewport_height(&self) -> i32 {
        self.viewport_bottom() - self.viewport_top()
    }
    fn max_scroll(&self) -> f32 {
        (self.content_height() - self.viewport_height()).max(0) as f32
    }
    fn clamp_scroll(&mut self) {
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll());
    }
    fn ensure_focus_visible(&mut self) {
        if self.focused >= self.rows.len() {
            return;
        }
        let row_top = self.focused as f32 * self.row_stride() as f32;
        let row_bottom = row_top + ROW_HEIGHT as f32;
        let viewport = self.viewport_height() as f32;
        if row_top < self.scroll_y {
            self.scroll_y = row_top;
        } else if row_bottom > self.scroll_y + viewport {
            self.scroll_y = row_bottom - viewport;
        }
        self.clamp_scroll();
    }
    fn label_x(&self) -> i32 {
        self.base.width / 2 - CONTENT_WIDTH / 2
    }
    fn scrollbar_rects(&self) -> Option<(ScreenRectangle, ScreenRectangle)> {
        let max_scroll = self.max_scroll();
        let viewport = self.viewport_height();
        if max_scroll <= 0.0 || viewport <= 0 {
            return None;
        }
        let track = ScreenRectangle::new(
            self.base.width / 2 + CONTENT_WIDTH / 2 + 8,
            self.viewport_top(),
            SCROLLBAR_WIDTH,
            viewport,
        );
        let thumb_height = ((viewport as i64 * viewport as i64
            / self.content_height().max(1) as i64) as i32)
            .clamp(SCROLLBAR_MIN_THUMB.min(viewport), viewport);
        let travel = viewport - thumb_height;
        let thumb_y = self.viewport_top()
            + if travel == 0 {
                0
            } else {
                (self.scroll_y * travel as f32 / max_scroll).round() as i32
            };
        Some((
            track,
            ScreenRectangle::new(track.left(), thumb_y, SCROLLBAR_WIDTH, thumb_height),
        ))
    }
    fn done_rect(&self) -> ScreenRectangle {
        let button_width = ((self.base.width - 24) / 2).clamp(40, DONE_WIDTH);
        ScreenRectangle::new(
            self.base.width / 2 - button_width - 4,
            self.base.height - 26,
            button_width,
            ROW_HEIGHT,
        )
    }
    fn cancel_rect(&self) -> ScreenRectangle {
        let button_width = ((self.base.width - 24) / 2).clamp(40, DONE_WIDTH);
        ScreenRectangle::new(
            self.base.width / 2 + 4,
            self.base.height - 26,
            button_width,
            ROW_HEIGHT,
        )
    }
    fn advance_focus(&mut self, backwards: bool) {
        let item_count = self.rows.len() + 2;
        if backwards {
            self.focused = if self.focused == 0 {
                item_count - 1
            } else {
                self.focused - 1
            };
        } else {
            self.focused = (self.focused + 1) % item_count;
        }
        self.ensure_focus_visible();
    }
    fn move_focus_vertical(&mut self, direction: i32) {
        let item_count = self.rows.len() + 2;
        self.focused = if direction < 0 {
            if self.focused == 0 {
                item_count - 1
            } else {
                self.focused - 1
            }
        } else {
            (self.focused + 1) % item_count
        };
        self.ensure_focus_visible();
    }
    fn activate_focused(&mut self) {
        if self.focused == self.done_index() {
            self.pending_action = Some(GameRulesAction::Done);
            return;
        }
        if self.focused == self.cancel_index() {
            self.pending_action = Some(GameRulesAction::Cancel);
            return;
        }
        self.activate_row(self.focused, true);
    }
    fn activate_row(&mut self, index: usize, begin_numeric_edit: bool) {
        let Some(row) = self.rows.get(index) else {
            return;
        };
        if !row.is_editable() {
            self.edit_error = None;
            return;
        }
        let edit = row.edit.clone().expect("editable rows have metadata");
        match edit {
            GameRuleEditMetadata::Boolean => {
                let GameRuleValue::Boolean(value) = &mut self.rows[index].value else {
                    return;
                };
                *value = !*value;
            }
            GameRuleEditMetadata::Enum { values } => {
                let GameRuleValue::String(value) = &mut self.rows[index].value else {
                    return;
                };
                if let Some(position) = values.iter().position(|candidate| candidate == value) {
                    *value = values[(position + 1) % values.len()].clone();
                } else {
                    *value = values[0].clone();
                }
            }
            GameRuleEditMetadata::Integer { .. } | GameRuleEditMetadata::Decimal { .. } => {
                if begin_numeric_edit {
                    self.begin_numeric_edit(index);
                }
            }
        }
    }
    fn cycle_row(&mut self, index: usize, direction: i32) {
        let Some(row) = self.rows.get(index) else {
            return;
        };
        if !row.is_editable() {
            return;
        }
        let Some(edit) = row.edit.clone() else {
            return;
        };
        match (&mut self.rows[index].value, edit) {
            (GameRuleValue::Boolean(value), GameRuleEditMetadata::Boolean) => {
                *value = direction > 0;
            }
            (GameRuleValue::String(value), GameRuleEditMetadata::Enum { values }) => {
                if let Some(position) = values.iter().position(|candidate| candidate == value) {
                    let len = values.len();
                    let next = if direction < 0 {
                        (position + len - 1) % len
                    } else {
                        (position + 1) % len
                    };
                    *value = values[next].clone();
                } else {
                    *value = if direction < 0 {
                        values[values.len() - 1].clone()
                    } else {
                        values[0].clone()
                    };
                }
            }
            (
                GameRuleValue::Integer(value),
                GameRuleEditMetadata::Integer {
                    minimum,
                    maximum,
                    step,
                },
            ) => {
                let next = if direction < 0 {
                    value.saturating_sub(step)
                } else {
                    value.saturating_add(step)
                };
                *value = next.clamp(minimum, maximum);
            }
            (
                GameRuleValue::Decimal(value),
                GameRuleEditMetadata::Decimal {
                    minimum,
                    maximum,
                    step,
                },
            ) => {
                let next = if !value.is_finite() {
                    minimum
                } else if direction < 0 {
                    *value - step
                } else {
                    *value + step
                };
                *value = next.clamp(minimum, maximum);
            }
            _ => {}
        }
    }
    fn begin_numeric_edit(&mut self, index: usize) {
        let Some(row) = self.rows.get(index) else {
            return;
        };
        if !row.is_editable()
            || !matches!(
                row.value,
                GameRuleValue::Integer(_) | GameRuleValue::Decimal(_)
            )
        {
            return;
        }
        self.editing_row = Some(index);
        self.edit_buffer = row.value.display_value();
        self.edit_error = None;
    }
    fn cancel_numeric_edit(&mut self) {
        self.editing_row = None;
        self.edit_buffer.clear();
        self.edit_error = None;
    }
    fn commit_numeric_edit(&mut self) -> bool {
        let Some(index) = self.editing_row else {
            return false;
        };
        let Some(row) = self.rows.get(index) else {
            self.cancel_numeric_edit();
            return false;
        };
        let parsed = match row.value {
            GameRuleValue::Integer(_) => self
                .edit_buffer
                .parse::<i64>()
                .ok()
                .map(GameRuleValue::Integer),
            GameRuleValue::Decimal(_) => self
                .edit_buffer
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(GameRuleValue::Decimal),
            _ => None,
        };
        let in_range = match (&row.edit, parsed.as_ref()) {
            (
                Some(GameRuleEditMetadata::Integer {
                    minimum, maximum, ..
                }),
                Some(GameRuleValue::Integer(value)),
            ) => value >= minimum && value <= maximum,
            (
                Some(GameRuleEditMetadata::Decimal {
                    minimum, maximum, ..
                }),
                Some(GameRuleValue::Decimal(value)),
            ) => value.is_finite() && value >= minimum && value <= maximum,
            _ => false,
        };
        if !in_range {
            self.edit_error = Some("Value is invalid or outside the server-provided range".into());
            return false;
        }
        self.rows[index].value = parsed.expect("validated parsed value");
        self.cancel_numeric_edit();
        true
    }
    fn append_numeric_character(&mut self, ch: char) -> bool {
        let Some(index) = self.editing_row else {
            return false;
        };
        let is_integer = matches!(self.rows[index].value, GameRuleValue::Integer(_));
        let allowed = ch.is_ascii_digit()
            || (ch == '-' && self.edit_buffer.is_empty())
            || (!is_integer && ch == '.' && !self.edit_buffer.contains('.'));
        if !allowed || self.edit_buffer.len() >= MAX_EDIT_LENGTH {
            return false;
        }
        self.edit_buffer.push(ch);
        self.edit_error = None;
        true
    }
    fn delete_edit_character(&mut self) -> bool {
        if self.editing_row.is_none() {
            return false;
        }
        self.edit_buffer.pop();
        self.edit_error = None;
        true
    }
    fn hit_row(&self, x: i32, y: i32) -> Option<usize> {
        if y < self.viewport_top() || y >= self.viewport_bottom() {
            return None;
        }
        let content_y = y - self.viewport_top() + self.scroll_y as i32;
        let stride = self.row_stride();
        let index = (content_y / stride) as usize;
        if index >= self.rows.len() || content_y % stride >= ROW_HEIGHT {
            return None;
        }
        let left = self.label_x();
        let right = self.base.width / 2 + VALUE_WIDTH + 10;
        (x >= left && x < right).then_some(index)
    }
    fn label_for_draw(&self, label: &str, max_width: i32) -> String {
        if self.font.measure_width(label, 1.0) <= max_width as f32 {
            return label.to_owned();
        }
        let mut shortened = String::new();
        for ch in label.chars() {
            let candidate = format!("{shortened}{ch}…");
            if self.font.measure_width(&candidate, 1.0) > max_width as f32 {
                break;
            }
            shortened.push(ch);
        }
        format!("{shortened}…")
    }
    fn edit_display(&self, index: usize) -> String {
        if self.editing_row == Some(index) {
            format!("{}_", self.edit_buffer)
        } else {
            self.rows[index].value.display_value()
        }
    }
}
impl Screen for GameRulesScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        self.clamp_scroll();
        if self.rows.is_empty() {
            self.focused = 0;
        } else {
            self.focused = self.focused.min(self.done_index());
        }
        self.ensure_focus_visible();
    }
    fn reposition_elements(&mut self) {
        self.clamp_scroll();
        self.ensure_focus_visible();
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if self.editing_row.is_some() {
            match event.key {
                KeyEvent::KEY_ESCAPE => {
                    self.cancel_numeric_edit();
                    return true;
                }
                KeyEvent::KEY_ENTER | KeyEvent::KEY_SPACE => {
                    self.commit_numeric_edit();
                    return true;
                }
                KeyEvent::KEY_BACKSPACE => return self.delete_edit_character(),
                KeyEvent::KEY_TAB => {
                    if self.commit_numeric_edit() {
                        self.advance_focus(event.has_shift_down());
                    }
                    return true;
                }
                _ => return false,
            }
        }
        if event.is_escape() {
            self.pending_action = Some(GameRulesAction::Cancel);
            return true;
        }
        if event.key == KeyEvent::KEY_TAB {
            self.advance_focus(event.has_shift_down());
            return true;
        }
        match event.key {
            KeyEvent::KEY_UP => {
                self.move_focus_vertical(-1);
                true
            }
            KeyEvent::KEY_DOWN => {
                self.move_focus_vertical(1);
                true
            }
            KeyEvent::KEY_ENTER | KeyEvent::KEY_SPACE => {
                self.activate_focused();
                crate::components::click_sound_sink::record_button_click();
                true
            }
            KeyEvent::KEY_LEFT | KeyEvent::KEY_RIGHT if self.focused < self.rows.len() => {
                let direction = if event.key == KeyEvent::KEY_LEFT {
                    -1
                } else {
                    1
                };
                self.cycle_row(self.focused, direction);
                true
            }
            _ => false,
        }
    }
    fn char_typed(&mut self, event: CharacterEvent) -> bool {
        self.append_numeric_character(event.codepoint)
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        if event.button() != 0 {
            return false;
        }
        let x = event.x as i32;
        let y = event.y as i32;
        if let Some((track, _thumb)) = self.scrollbar_rects() {
            if track.contains_point(x, y) {
                self.cancel_numeric_edit();
                self.dragging_scrollbar = true;
                self.scrollbar_drag_to(y);
                return true;
            }
        }
        if self.done_rect().contains_point(x, y) {
            if self.editing_row.is_some() && !self.commit_numeric_edit() {
                return true;
            }
            self.focused = self.done_index();
            self.pending_action = Some(GameRulesAction::Done);
            crate::components::click_sound_sink::record_button_click();
            return true;
        }
        if self.cancel_rect().contains_point(x, y) {
            self.cancel_numeric_edit();
            self.focused = self.cancel_index();
            self.pending_action = Some(GameRulesAction::Cancel);
            crate::components::click_sound_sink::record_button_click();
            return true;
        }
        if let Some(index) = self.hit_row(x, y) {
            crate::components::click_sound_sink::record_button_click();
            if self.editing_row != Some(index) {
                self.cancel_numeric_edit();
            }
            self.focused = index;
            self.edit_error = None;
            match self.rows[index].value {
                GameRuleValue::Integer(_) | GameRuleValue::Decimal(_) => {
                    self.begin_numeric_edit(index)
                }
                _ => self.activate_row(index, false),
            }
            return true;
        }
        false
    }
    fn mouse_released(&mut self, _event: MouseButtonEvent) -> bool {
        let was_dragging = self.dragging_scrollbar;
        self.dragging_scrollbar = false;
        was_dragging
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, _dx: f64, dy: f64) -> bool {
        if !self.dragging_scrollbar {
            return false;
        }
        self.scrollbar_drag_to(event.y as i32);
        let _ = dy;
        true
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, _scroll_x: f64, scroll_y: f64) -> bool {
        let x = x as i32;
        let y = y as i32;
        if x < self.label_x()
            || x >= self.base.width / 2 + CONTENT_WIDTH / 2
            || y < self.viewport_top()
            || y >= self.viewport_bottom()
            || self.max_scroll() <= 0.0
        {
            return false;
        }
        self.scroll_y = (self.scroll_y - scroll_y as f32 * 12.0).clamp(0.0, self.max_scroll());
        true
    }
    fn update_narration(&mut self, output: &mut NarrationElementOutput) {
        self.base_ref().update_narration(output);
        if self.focused == self.done_index() || self.focused == self.cancel_index() {
            let label = if self.focused == self.done_index() {
                crate::translate("gui.done")
            } else {
                crate::translate("gui.cancel")
            };
            output.add(NarratedElementType::Position, label);
            return;
        }
        let Some(row) = self.rows.get(self.focused) else {
            return;
        };
        output.add(
            NarratedElementType::Position,
            format!("{}: {}", row.label, self.edit_display(self.focused)),
        );
        if row.is_editable() {
            output.add(
                NarratedElementType::Hint,
                if self.editing_row == Some(self.focused) {
                    "Enter to apply, Escape to cancel"
                } else {
                    "Use left and right arrows or Enter to change this value"
                },
            );
        }
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        _partial_tick: f32,
    ) {
        let width = graphics.gui_width();
        self.base.extract_background_layers(graphics, self.in_world);
        let title_width = self.font.measure_width(&self.title, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            self.title.clone(),
            (width - title_width) / 2,
            12,
            0xFFFF_FFFFu32 as i32,
        );
        graphics.enable_scissor(0, self.viewport_top(), width, self.viewport_bottom());
        let label_x = self.label_x();
        let value_x = width / 2 + 10;
        for (index, row) in self.rows.iter().enumerate() {
            let y = self.viewport_top() + index as i32 * self.row_stride() - self.scroll_y as i32;
            if y + ROW_HEIGHT <= self.viewport_top() || y >= self.viewport_bottom() {
                continue;
            }
            let label = self.label_for_draw(&row.label, CONTENT_WIDTH - VALUE_WIDTH - 24);
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                label,
                label_x,
                y + 6,
                0xFFFF_FFFFu32 as i32,
            );
            let value = self.edit_display(index);
            graphics.draw_button_with_focus(
                self.font.as_ref(),
                value_x,
                y,
                VALUE_WIDTH,
                ROW_HEIGHT,
                &value,
                row.is_editable(),
                self.focused == index,
            );
            if self.editing_row == Some(index) && self.edit_error.is_some() {
                graphics.text(
                    ResourceLocation::from_string("minecraft:default"),
                    "Invalid value",
                    value_x,
                    y + ROW_HEIGHT,
                    0xFFFF_5555u32 as i32,
                );
            }
        }
        if self.rows.is_empty() {
            let message_key = if self.loading {
                "editGamerule.inGame.downloadingGamerules"
            } else {
                "eltharn.menu.game_rules.empty"
            };
            let translated = crate::translate(message_key);
            let message = if translated == message_key {
                if self.loading {
                    "Downloading game rules…".to_owned()
                } else {
                    "No game rules available".to_owned()
                }
            } else {
                translated
            };
            let message_width = self.font.measure_width(&message, 1.0).round() as i32;
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                message,
                (width - message_width) / 2,
                self.viewport_top() + 12,
                0xFFAAAAAAu32 as i32,
            );
        }
        graphics.disable_scissor();
        if let Some((track, thumb)) = self.scrollbar_rects() {
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller_background"),
                track.left(),
                track.top(),
                track.width,
                track.height,
            );
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller"),
                thumb.left(),
                thumb.top(),
                thumb.width,
                thumb.height,
            );
        }
        let done = self.done_rect();
        graphics.draw_button_with_focus(
            self.font.as_ref(),
            done.left(),
            done.top(),
            done.width,
            done.height,
            &crate::translate("gui.done"),
            true,
            self.focused == self.done_index(),
        );
        let cancel = self.cancel_rect();
        graphics.draw_button_with_focus(
            self.font.as_ref(),
            cancel.left(),
            cancel.top(),
            cancel.width,
            cancel.height,
            &crate::translate("gui.cancel"),
            true,
            self.focused == self.cancel_index(),
        );
        let _ = (mouse_x, mouse_y);
    }
}
impl GameRulesScreen {
    fn scrollbar_drag_to(&mut self, mouse_y: i32) {
        let Some((track, thumb)) = self.scrollbar_rects() else {
            return;
        };
        let travel = track.height - thumb.height;
        if travel <= 0 {
            self.scroll_y = 0.0;
        } else {
            let thumb_top = (mouse_y - thumb.height / 2 - track.top()).clamp(0, travel);
            self.scroll_y = thumb_top as f32 / travel as f32 * self.max_scroll();
        }
    }
}
impl GameRulesScreen {
    pub fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        <Self as Screen>::extract_render_state(self, graphics, mouse_x, mouse_y, partial_tick);
    }
}
