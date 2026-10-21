use crate::FontWidthMeasurer;
use crate::input::events::PreeditEvent;
pub const DEFAULT_TEXT_COLOR: i32 = -2039584;
pub const DEFAULT_TEXT_COLOR_UNEDITABLE: i32 = -9408400;
const CURSOR_BLINK_INTERVAL_MS: u64 = 300;
const CURSOR_APPEND_CHARACTER: &str = "_";
const FONT_LINE_HEIGHT: i32 = 9;
pub const BACKWARDS: i32 = -1;
pub const FORWARDS: i32 = 1;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditBoxKey {
    Backspace,
    Delete,
    Right,
    Left,
    Home,
    End,
}
pub trait Clipboard {
    fn get(&self) -> String;
    fn set(&mut self, value: String);
}
pub fn filter_text(input: &str) -> String {
    input
        .chars()
        .filter(|&c| is_allowed_chat_character(c))
        .collect()
}
pub fn is_allowed_chat_character(c: char) -> bool {
    c != '\u{a7}' && c >= ' ' && c != '\u{7f}'
}
pub fn is_cursor_visible(time_since_focus_ms: u64) -> bool {
    (time_since_focus_ms / CURSOR_BLINK_INTERVAL_MS) % 2 == 0
}
pub fn font_width(font: &dyn FontWidthMeasurer, text: &str) -> i32 {
    font.measure_width(text, 1.0).ceil() as i32
}
pub fn plain_substr_by_width(font: &dyn FontWidthMeasurer, text: &str, max_width: i32) -> String {
    let mut remaining = max_width as f32;
    let mut end = 0usize;
    for (offset, c) in text.char_indices() {
        remaining -= font.measure_char(c, 1.0);
        if remaining >= 0.0 {
            end = offset + c.len_utf8();
        } else {
            break;
        }
    }
    text[..end].to_string()
}
pub fn plain_substr_by_width_reverse(
    font: &dyn FontWidthMeasurer,
    text: &str,
    max_width: i32,
) -> String {
    let mut width = 0.0f32;
    let mut start = text.len();
    for (offset, c) in text.char_indices().rev() {
        width += font.measure_char(c, 1.0);
        if width > max_width as f32 {
            break;
        }
        start = offset;
    }
    text[start..].to_string()
}
#[derive(Clone, Debug, PartialEq)]
pub enum EditBoxDraw {
    Sprite {
        sprite: &'static str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    Text {
        text: String,
        x: i32,
        y: i32,
        color: i32,
        shadow: bool,
    },
    Highlight {
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        invert: bool,
    },
    InsertCursor {
        x: i32,
        y: i32,
        color: i32,
    },
    AppendCursor {
        x: i32,
        y: i32,
        color: i32,
        shadow: bool,
    },
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditBoxRenderPlan {
    pub draws: Vec<EditBoxDraw>,
    pub preedit_underline: Option<EditBoxPreeditUnderline>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditBoxPreeditUnderline {
    pub x0: i32,
    pub x1: i32,
    pub y: i32,
    pub color: i32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct PreeditComposition {
    text: Vec<char>,
    caret_position: usize,
    underline_start: usize,
    underline_end: usize,
    replace_start: usize,
    replace_end: usize,
    saved_display_pos: usize,
    saved_text_x: i32,
}
fn focused_preedit_block_range(event: &PreeditEvent, text_len: usize) -> (usize, usize) {
    if event.blocks.concat() != event.full_text || event.focused_block >= event.blocks.len() {
        return (0, text_len);
    }
    let start: usize = event.blocks[..event.focused_block]
        .iter()
        .map(|block| block.chars().count())
        .sum();
    let end = start + event.blocks[event.focused_block].chars().count();
    (start.min(text_len), end.min(text_len))
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditBox {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    value: Vec<char>,
    max_length: usize,
    bordered: bool,
    can_lose_focus: bool,
    is_editable: bool,
    centered: bool,
    text_shadow: bool,
    invert_highlighted_text_color: bool,
    visible: bool,
    focused: bool,
    display_pos: usize,
    cursor_pos: usize,
    highlight_pos: usize,
    text_color: i32,
    text_color_uneditable: i32,
    suggestion: Option<String>,
    hint: Option<String>,
    preedit: Option<PreeditComposition>,
    focused_at_ms: u64,
    text_x: i32,
    text_y: i32,
}
impl EditBox {
    pub fn new(font: &dyn FontWidthMeasurer, x: i32, y: i32, width: i32, height: i32) -> Self {
        let mut edit_box = Self {
            x,
            y,
            width,
            height,
            value: Vec::new(),
            max_length: 32,
            bordered: true,
            can_lose_focus: true,
            is_editable: true,
            centered: false,
            text_shadow: true,
            invert_highlighted_text_color: true,
            visible: true,
            focused: false,
            display_pos: 0,
            cursor_pos: 0,
            highlight_pos: 0,
            text_color: DEFAULT_TEXT_COLOR,
            text_color_uneditable: DEFAULT_TEXT_COLOR_UNEDITABLE,
            suggestion: None,
            hint: None,
            preedit: None,
            focused_at_ms: 0,
            text_x: 0,
            text_y: 0,
        };
        edit_box.update_text_position(font);
        edit_box
    }
    pub fn set_max_length(&mut self, font: &dyn FontWidthMeasurer, max_length: usize) {
        self.clear_preedit(font);
        self.max_length = max_length;
        if self.value.len() > max_length {
            self.value.truncate(max_length);
            self.cursor_pos = self.cursor_pos.min(self.value.len());
            self.highlight_pos = self.highlight_pos.min(self.value.len());
            self.update_text_position(font);
        }
    }
    pub fn set_bordered(&mut self, font: &dyn FontWidthMeasurer, bordered: bool) {
        self.clear_preedit(font);
        self.bordered = bordered;
        self.update_text_position(font);
    }
    pub fn set_centered(&mut self, font: &dyn FontWidthMeasurer, centered: bool) {
        self.clear_preedit(font);
        self.centered = centered;
        self.update_text_position(font);
    }
    pub fn set_text_shadow(&mut self, text_shadow: bool) {
        self.text_shadow = text_shadow;
    }
    pub fn set_invert_highlighted_text_color(&mut self, invert: bool) {
        self.invert_highlighted_text_color = invert;
    }
    pub fn set_text_color(&mut self, color: i32) {
        self.text_color = color;
    }
    pub fn set_text_color_uneditable(&mut self, color: i32) {
        self.text_color_uneditable = color;
    }
    pub fn set_editable(&mut self, editable: bool) {
        self.is_editable = editable;
        if !editable {
            self.clear_preedit_without_font();
        }
    }
    pub fn set_can_lose_focus(&mut self, can_lose_focus: bool) {
        self.can_lose_focus = can_lose_focus;
    }
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        if !visible {
            self.clear_preedit_without_font();
        }
    }
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    pub fn set_hint(&mut self, hint: Option<String>) {
        self.hint = hint;
    }
    pub fn set_suggestion(&mut self, suggestion: Option<String>) {
        self.suggestion = suggestion;
    }
    pub fn set_position(&mut self, font: &dyn FontWidthMeasurer, x: i32, y: i32) {
        self.clear_preedit(font);
        self.x = x;
        self.y = y;
        self.update_text_position(font);
    }
    pub fn set_size(&mut self, font: &dyn FontWidthMeasurer, width: i32, height: i32) {
        self.clear_preedit(font);
        self.width = width;
        self.height = height;
        self.update_text_position(font);
    }
    pub fn x(&self) -> i32 {
        self.x
    }
    pub fn y(&self) -> i32 {
        self.y
    }
    pub fn width(&self) -> i32 {
        self.width
    }
    pub fn height(&self) -> i32 {
        self.height
    }
    pub fn set_focused(&mut self, focused: bool, now_ms: u64) {
        if !self.can_lose_focus && !focused {
            return;
        }
        self.focused = focused;
        if !focused {
            self.clear_preedit_without_font();
        }
        if focused {
            self.focused_at_ms = now_ms;
        }
    }
    pub fn is_focused(&self) -> bool {
        self.focused
    }
    pub fn preedit_updated(
        &mut self,
        font: &dyn FontWidthMeasurer,
        event: Option<PreeditEvent>,
    ) -> bool {
        let Some(event) = event else {
            self.clear_preedit(font);
            return self.focused;
        };
        if !self.can_consume_input() {
            self.clear_preedit(font);
            return false;
        }
        let text: Vec<char> = event.full_text.chars().collect();
        if text.is_empty() {
            self.clear_preedit(font);
            return true;
        }
        let (underline_start, underline_end) = focused_preedit_block_range(&event, text.len());
        let composition = self.preedit.take().unwrap_or_else(|| PreeditComposition {
            text: Vec::new(),
            caret_position: 0,
            underline_start: 0,
            underline_end: 0,
            replace_start: self.cursor_pos.min(self.highlight_pos),
            replace_end: self.cursor_pos.max(self.highlight_pos),
            saved_display_pos: self.display_pos,
            saved_text_x: self.text_x,
        });
        self.preedit = Some(PreeditComposition {
            caret_position: event.caret_position.min(text.len()),
            underline_start,
            underline_end,
            text,
            ..composition
        });
        let visual_value = self.visual_value();
        let visual_caret = self.preedit_caret_position();
        self.scroll_virtual_to(font, &visual_value, visual_caret);
        self.update_text_position(font);
        true
    }
    pub fn value(&self) -> String {
        self.value.iter().collect()
    }
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }
    pub fn cursor_position(&self) -> usize {
        self.cursor_pos
    }
    pub fn set_value(&mut self, font: &dyn FontWidthMeasurer, value: &str) {
        self.clear_preedit(font);
        self.value = value.chars().take(self.max_length).collect();
        self.move_cursor_to_end(font, false);
        self.set_highlight_pos(font, self.cursor_pos);
        self.update_text_position(font);
    }
    pub fn highlighted(&self) -> String {
        let start = self.cursor_pos.min(self.highlight_pos);
        let end = self.cursor_pos.max(self.highlight_pos);
        self.value[start..end].iter().collect()
    }
    pub fn insert_text(&mut self, font: &dyn FontWidthMeasurer, input: &str) {
        self.clear_preedit(font);
        let start = self.cursor_pos.min(self.highlight_pos);
        let end = self.cursor_pos.max(self.highlight_pos);
        let selected = end - start;
        let max_insertion = (self.max_length + selected).saturating_sub(self.value.len());
        if max_insertion == 0 {
            return;
        }
        let text: Vec<char> = filter_text(input).chars().take(max_insertion).collect();
        let insertion_length = text.len();
        self.value.splice(start..end, text);
        self.set_cursor_position(font, start + insertion_length);
        self.set_highlight_pos(font, self.cursor_pos);
        self.update_text_position(font);
    }
    pub fn delete_text(&mut self, font: &dyn FontWidthMeasurer, dir: i32, whole_word: bool) {
        if whole_word {
            self.delete_words(font, dir);
        } else {
            self.delete_chars(font, dir);
        }
    }
    pub fn delete_words(&mut self, font: &dyn FontWidthMeasurer, dir: i32) {
        if self.value.is_empty() {
            return;
        }
        if self.highlight_pos != self.cursor_pos {
            self.insert_text(font, "");
        } else {
            self.delete_chars_to_pos(font, self.word_position(dir));
        }
    }
    pub fn delete_chars(&mut self, font: &dyn FontWidthMeasurer, dir: i32) {
        self.delete_chars_to_pos(font, self.cursor_pos_offset(dir));
    }
    pub fn delete_chars_to_pos(&mut self, font: &dyn FontWidthMeasurer, pos: usize) {
        self.clear_preedit(font);
        if self.value.is_empty() {
            return;
        }
        if self.highlight_pos != self.cursor_pos {
            self.insert_text(font, "");
            return;
        }
        let start = pos.min(self.cursor_pos);
        let end = pos.max(self.cursor_pos);
        if start == end {
            return;
        }
        self.value.drain(start..end);
        self.set_cursor_position(font, start);
        self.update_text_position(font);
        self.move_cursor_to(font, start, false);
    }
    pub fn word_position(&self, dir: i32) -> usize {
        self.word_position_from(dir, self.cursor_pos, true)
    }
    fn word_position_from(&self, dir: i32, from: usize, strip_spaces: bool) -> usize {
        let mut result = from.min(self.value.len());
        let reverse = dir < 0;
        for _ in 0..dir.unsigned_abs() {
            if !reverse {
                let length = self.value.len();
                result = match self.value[result.min(length)..]
                    .iter()
                    .position(|&c| c == ' ')
                {
                    Some(offset) => result + offset,
                    None => length,
                };
                while strip_spaces && result < length && self.value[result] == ' ' {
                    result += 1;
                }
            } else {
                while strip_spaces && result > 0 && self.value[result - 1] == ' ' {
                    result -= 1;
                }
                while result > 0 && self.value[result - 1] != ' ' {
                    result -= 1;
                }
            }
        }
        result
    }
    fn cursor_pos_offset(&self, dir: i32) -> usize {
        if dir < 0 {
            self.cursor_pos.saturating_sub(dir.unsigned_abs() as usize)
        } else {
            (self.cursor_pos + dir as usize).min(self.value.len())
        }
    }
    pub fn move_cursor(&mut self, font: &dyn FontWidthMeasurer, dir: i32, extend_selection: bool) {
        self.move_cursor_to(font, self.cursor_pos_offset(dir), extend_selection);
    }
    pub fn move_cursor_to(
        &mut self,
        font: &dyn FontWidthMeasurer,
        pos: usize,
        extend_selection: bool,
    ) {
        self.set_cursor_position(font, pos);
        if !extend_selection {
            self.set_highlight_pos(font, self.cursor_pos);
        }
        self.update_text_position(font);
    }
    pub fn set_cursor_position(&mut self, font: &dyn FontWidthMeasurer, pos: usize) {
        self.clear_preedit(font);
        self.cursor_pos = pos.min(self.value.len());
        self.scroll_to(font, self.cursor_pos);
    }
    pub fn move_cursor_to_start(&mut self, font: &dyn FontWidthMeasurer, extend_selection: bool) {
        self.move_cursor_to(font, 0, extend_selection);
    }
    pub fn move_cursor_to_end(&mut self, font: &dyn FontWidthMeasurer, extend_selection: bool) {
        self.move_cursor_to(font, self.value.len(), extend_selection);
    }
    pub fn set_highlight_pos(&mut self, font: &dyn FontWidthMeasurer, pos: usize) {
        self.clear_preedit(font);
        self.highlight_pos = pos.min(self.value.len());
        self.scroll_to(font, self.highlight_pos);
    }
    pub fn inner_width(&self) -> i32 {
        if self.bordered {
            self.width - 8
        } else {
            self.width
        }
    }
    fn scroll_to(&mut self, font: &dyn FontWidthMeasurer, pos: usize) {
        self.display_pos = self.display_pos.min(self.value.len());
        let inner_width = self.inner_width();
        let tail: String = self.value[self.display_pos..].iter().collect();
        let displayed = plain_substr_by_width(font, &tail, inner_width);
        let last_pos = displayed.chars().count() + self.display_pos;
        if pos == self.display_pos {
            let whole: String = self.value.iter().collect();
            let back = plain_substr_by_width_reverse(font, &whole, inner_width);
            self.display_pos = self.display_pos.saturating_sub(back.chars().count());
        }
        if pos > last_pos {
            self.display_pos += pos - last_pos;
        } else if pos <= self.display_pos {
            self.display_pos -= self.display_pos - pos;
        }
        self.display_pos = self.display_pos.min(self.value.len());
    }
    fn update_text_position(&mut self, font: &dyn FontWidthMeasurer) {
        let visual_value = self.visual_value();
        let tail: String = visual_value[self.display_pos.min(visual_value.len())..]
            .iter()
            .collect();
        let displayed = plain_substr_by_width(font, &tail, self.inner_width());
        self.text_x = self.x
            + if self.centered {
                (self.width - font_width(font, &displayed)) / 2
            } else if self.bordered {
                4
            } else {
                0
            };
        self.text_y = if self.bordered {
            self.y + (self.height - 8) / 2
        } else {
            self.y
        };
    }
    fn clear_preedit_without_font(&mut self) {
        if let Some(composition) = self.preedit.take() {
            self.display_pos = composition.saved_display_pos.min(self.value.len());
            self.text_x = composition.saved_text_x;
        }
    }
    fn clear_preedit(&mut self, font: &dyn FontWidthMeasurer) {
        if self.preedit.is_some() {
            self.clear_preedit_without_font();
            self.scroll_to(font, self.cursor_pos);
            self.update_text_position(font);
        }
    }
    fn visual_value(&self) -> Vec<char> {
        let Some(composition) = &self.preedit else {
            return self.value.clone();
        };
        let mut visual = Vec::with_capacity(
            self.value.len() - (composition.replace_end - composition.replace_start)
                + composition.text.len(),
        );
        visual.extend_from_slice(&self.value[..composition.replace_start]);
        visual.extend_from_slice(&composition.text);
        visual.extend_from_slice(&self.value[composition.replace_end..]);
        visual
    }
    fn preedit_caret_position(&self) -> usize {
        self.preedit
            .as_ref()
            .map(|composition| composition.replace_start + composition.caret_position)
            .unwrap_or(self.cursor_pos)
    }
    fn scroll_virtual_to(
        &mut self,
        font: &dyn FontWidthMeasurer,
        visual_value: &[char],
        pos: usize,
    ) {
        self.display_pos = self.display_pos.min(visual_value.len());
        let inner_width = self.inner_width();
        let tail: String = visual_value[self.display_pos..].iter().collect();
        let displayed = plain_substr_by_width(font, &tail, inner_width);
        let last_pos = displayed.chars().count() + self.display_pos;
        if pos == self.display_pos {
            let whole: String = visual_value.iter().collect();
            let back = plain_substr_by_width_reverse(font, &whole, inner_width);
            self.display_pos = self.display_pos.saturating_sub(back.chars().count());
        }
        if pos > last_pos {
            self.display_pos += pos - last_pos;
        } else if pos <= self.display_pos {
            self.display_pos -= self.display_pos - pos;
        }
        self.display_pos = self.display_pos.min(visual_value.len());
    }
    pub fn key_pressed(
        &mut self,
        font: &dyn FontWidthMeasurer,
        key: EditBoxKey,
        ctrl: bool,
        shift: bool,
    ) -> bool {
        if !self.focused {
            return false;
        }
        match key {
            EditBoxKey::Backspace => {
                if self.is_editable {
                    self.delete_text(font, BACKWARDS, ctrl);
                }
                true
            }
            EditBoxKey::Delete => {
                if self.is_editable {
                    self.delete_text(font, FORWARDS, ctrl);
                }
                true
            }
            EditBoxKey::Right => {
                if ctrl {
                    self.move_cursor_to(font, self.word_position(FORWARDS), shift);
                } else {
                    self.move_cursor(font, FORWARDS, shift);
                }
                true
            }
            EditBoxKey::Left => {
                if ctrl {
                    self.move_cursor_to(font, self.word_position(BACKWARDS), shift);
                } else {
                    self.move_cursor(font, BACKWARDS, shift);
                }
                true
            }
            EditBoxKey::Home => {
                self.move_cursor_to_start(font, shift);
                true
            }
            EditBoxKey::End => {
                self.move_cursor_to_end(font, shift);
                true
            }
        }
    }
    pub fn select_all(&mut self, font: &dyn FontWidthMeasurer) -> bool {
        if !self.focused {
            return false;
        }
        self.move_cursor_to_end(font, false);
        self.set_highlight_pos(font, 0);
        true
    }
    pub fn copy(&self, clipboard: &mut dyn Clipboard) -> bool {
        if !self.focused {
            return false;
        }
        clipboard.set(self.highlighted());
        true
    }
    pub fn paste(&mut self, font: &dyn FontWidthMeasurer, clipboard: &dyn Clipboard) -> bool {
        if !self.focused {
            return false;
        }
        if self.is_editable {
            let pasted = clipboard.get();
            self.insert_text(font, &pasted);
        }
        true
    }
    pub fn cut(&mut self, font: &dyn FontWidthMeasurer, clipboard: &mut dyn Clipboard) -> bool {
        if !self.focused {
            return false;
        }
        clipboard.set(self.highlighted());
        if self.is_editable {
            self.insert_text(font, "");
        }
        true
    }
    pub fn can_consume_input(&self) -> bool {
        self.focused && self.is_editable
    }
    pub fn char_typed(&mut self, font: &dyn FontWidthMeasurer, c: char) -> bool {
        if !self.can_consume_input() {
            return false;
        }
        if !is_allowed_chat_character(c) {
            return false;
        }
        self.insert_text(font, c.encode_utf8(&mut [0u8; 4]));
        true
    }
    pub fn clicked_position_in_text(&self, font: &dyn FontWidthMeasurer, mouse_x: f64) -> usize {
        let position_in_text = ((mouse_x.floor() as i32) - self.text_x).min(self.inner_width());
        let displayed: String = self.value[self.display_pos.min(self.value.len())..]
            .iter()
            .collect();
        self.display_pos
            + plain_substr_by_width(font, &displayed, position_in_text)
                .chars()
                .count()
    }
    pub fn on_click(
        &mut self,
        font: &dyn FontWidthMeasurer,
        mouse_x: f64,
        shift: bool,
        double_click: bool,
    ) {
        self.clear_preedit(font);
        let clicked = self.clicked_position_in_text(font, mouse_x);
        if double_click {
            let word_start = self.word_position_from(BACKWARDS, clicked, true);
            let word_end = self.word_position_from(FORWARDS, clicked, true);
            self.move_cursor_to(font, word_start, false);
            self.move_cursor_to(font, word_end, true);
        } else {
            self.move_cursor_to(font, clicked, shift);
        }
    }
    pub fn on_drag(&mut self, font: &dyn FontWidthMeasurer, mouse_x: f64) {
        self.clear_preedit(font);
        let clicked = self.clicked_position_in_text(font, mouse_x);
        self.move_cursor_to(font, clicked, true);
    }
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x as f64
            && x < (self.x + self.width) as f64
            && y >= self.y as f64
            && y < (self.y + self.height) as f64
    }
    pub fn render_plan(&self, font: &dyn FontWidthMeasurer, now_ms: u64) -> EditBoxRenderPlan {
        let mut plan = EditBoxRenderPlan::default();
        if !self.visible {
            return plan;
        }
        if self.bordered {
            plan.draws.push(EditBoxDraw::Sprite {
                sprite: if self.focused {
                    "minecraft:widget/text_field_highlighted"
                } else {
                    "minecraft:widget/text_field"
                },
                x: self.x,
                y: self.y,
                width: self.width,
                height: self.height,
            });
        }
        let color = if self.is_editable {
            self.text_color
        } else {
            self.text_color_uneditable
        };
        if self.preedit.is_some() {
            self.append_preedit_render_plan(font, now_ms, color, &mut plan);
            return plan;
        }
        let rel_cursor_pos = self.cursor_pos as i64 - self.display_pos as i64;
        let tail: String = self.value[self.display_pos.min(self.value.len())..]
            .iter()
            .collect();
        let displayed = plain_substr_by_width(font, &tail, self.inner_width());
        let displayed_len = displayed.chars().count() as i64;
        let cursor_on_screen = rel_cursor_pos >= 0 && rel_cursor_pos <= displayed_len;
        let show_cursor = self.focused
            && is_cursor_visible(now_ms.saturating_sub(self.focused_at_ms))
            && cursor_on_screen;
        let mut draw_x = self.text_x;
        let rel_highlight_pos =
            (self.highlight_pos as i64 - self.display_pos as i64).clamp(0, displayed_len);
        if !displayed.is_empty() {
            let half: String = if cursor_on_screen {
                displayed.chars().take(rel_cursor_pos as usize).collect()
            } else {
                displayed.clone()
            };
            plan.draws.push(EditBoxDraw::Text {
                x: draw_x,
                y: self.text_y,
                color,
                shadow: self.text_shadow,
                text: half.clone(),
            });
            draw_x += font_width(font, &half) + 1;
        }
        let insert = self.cursor_pos < self.value.len() || self.value.len() >= self.max_length;
        let mut cursor_x = draw_x;
        if !cursor_on_screen {
            cursor_x = if rel_cursor_pos > 0 {
                self.text_x + self.width
            } else {
                self.text_x
            };
        } else if insert {
            cursor_x = draw_x - 1;
            draw_x -= 1;
        }
        if !displayed.is_empty() && cursor_on_screen && rel_cursor_pos < displayed_len {
            plan.draws.push(EditBoxDraw::Text {
                text: displayed.chars().skip(rel_cursor_pos as usize).collect(),
                x: draw_x,
                y: self.text_y,
                color,
                shadow: self.text_shadow,
            });
        }
        if let Some(hint) = &self.hint {
            if displayed.is_empty() && !self.focused {
                plan.draws.push(EditBoxDraw::Text {
                    text: hint.clone(),
                    x: draw_x,
                    y: self.text_y,
                    color,
                    shadow: false,
                });
            }
        }
        if let Some(suggestion) = &self.suggestion {
            if !insert {
                plan.draws.push(EditBoxDraw::Text {
                    text: suggestion.clone(),
                    x: cursor_x - 1,
                    y: self.text_y,
                    color: -8355712,
                    shadow: self.text_shadow,
                });
            }
        }
        if rel_highlight_pos != rel_cursor_pos {
            let prefix: String = displayed.chars().take(rel_highlight_pos as usize).collect();
            let highlight_x = self.text_x + font_width(font, &prefix);
            plan.draws.push(EditBoxDraw::Highlight {
                x0: cursor_x.min(self.x + self.width),
                y0: self.text_y - 1,
                x1: (highlight_x - 1).min(self.x + self.width),
                y1: self.text_y + 1 + FONT_LINE_HEIGHT,
                invert: self.invert_highlighted_text_color,
            });
        }
        if show_cursor {
            if insert {
                plan.draws.push(EditBoxDraw::InsertCursor {
                    x: cursor_x,
                    y: self.text_y,
                    color,
                });
            } else {
                plan.draws.push(EditBoxDraw::AppendCursor {
                    x: cursor_x,
                    y: self.text_y,
                    color,
                    shadow: self.text_shadow,
                });
            }
        }
        plan
    }
    fn append_preedit_render_plan(
        &self,
        font: &dyn FontWidthMeasurer,
        now_ms: u64,
        color: i32,
        plan: &mut EditBoxRenderPlan,
    ) {
        let composition = self.preedit.as_ref().expect("composition was checked");
        let visual_value = self.visual_value();
        let view_start = self.display_pos.min(visual_value.len());
        let tail: String = visual_value[view_start..].iter().collect();
        let displayed = plain_substr_by_width(font, &tail, self.inner_width());
        let displayed_chars: Vec<char> = displayed.chars().collect();
        let view_end = view_start + displayed_chars.len();
        let preedit_start = composition.replace_start;
        let preedit_end = preedit_start + composition.text.len();
        let visible_preedit_start = preedit_start.max(view_start);
        let visible_preedit_end = preedit_end.min(view_end);
        let has_visible_preedit = visible_preedit_start < visible_preedit_end;
        let mut draw_x = self.text_x;
        let push_text = |plan: &mut EditBoxRenderPlan, text: String, x: i32| {
            if !text.is_empty() {
                plan.draws.push(EditBoxDraw::Text {
                    text,
                    x,
                    y: self.text_y,
                    color,
                    shadow: self.text_shadow,
                });
            }
        };
        if has_visible_preedit {
            let before_len = visible_preedit_start - view_start;
            let preedit_len = visible_preedit_end - visible_preedit_start;
            let before: String = displayed_chars[..before_len].iter().collect();
            push_text(plan, before.clone(), draw_x);
            draw_x += font_width(font, &before);
            let composing: String = displayed_chars[before_len..before_len + preedit_len]
                .iter()
                .collect();
            push_text(plan, composing.clone(), draw_x);
            draw_x += font_width(font, &composing);
            let after: String = displayed_chars[before_len + preedit_len..].iter().collect();
            push_text(plan, after, draw_x);
        } else {
            push_text(plan, displayed, draw_x);
        }
        let underline_virtual_start = preedit_start + composition.underline_start;
        let underline_virtual_end = preedit_start + composition.underline_end;
        let visible_underline_start = underline_virtual_start.max(view_start);
        let visible_underline_end = underline_virtual_end.min(view_end);
        if visible_underline_start < visible_underline_end {
            let prefix: String = displayed_chars[..visible_underline_start - view_start]
                .iter()
                .collect();
            let underlined: String = displayed_chars
                [visible_underline_start - view_start..visible_underline_end - view_start]
                .iter()
                .collect();
            let x0 = self.text_x + font_width(font, &prefix);
            let x1 = x0 + font_width(font, &underlined);
            plan.preedit_underline = Some(EditBoxPreeditUnderline {
                x0,
                x1,
                y: self.text_y + FONT_LINE_HEIGHT - 1,
                color,
            });
        }
        let caret_position = self.preedit_caret_position();
        let cursor_on_screen = caret_position >= view_start && caret_position <= view_end;
        if self.focused
            && cursor_on_screen
            && is_cursor_visible(now_ms.saturating_sub(self.focused_at_ms))
        {
            let caret_prefix: String = visual_value[view_start..caret_position].iter().collect();
            plan.draws.push(EditBoxDraw::InsertCursor {
                x: self.text_x + font_width(font, &caret_prefix),
                y: self.text_y,
                color,
            });
        }
    }
    pub const fn append_cursor_character() -> &'static str {
        CURSOR_APPEND_CHARACTER
    }
}
