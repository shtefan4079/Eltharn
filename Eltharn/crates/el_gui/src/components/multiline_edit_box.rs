use super::edit_box::{
    Clipboard, EditBoxDraw, EditBoxRenderPlan, is_allowed_chat_character, is_cursor_visible,
};
use crate::{
    FontWidthMeasurer,
    input::events::{KeyEvent, PreeditEvent},
};
use std::ops::Range;
#[derive(Clone)]
pub struct MultilineEditBox {
    value: Vec<char>,
    cursor: usize,
    anchor: usize,
    max_length: usize,
    max_lines: Option<usize>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    scroll: f64,
    focused: bool,
    focused_at: u64,
    preedit: Option<PreeditEvent>,
}
impl MultilineEditBox {
    pub fn new(
        font: &dyn FontWidthMeasurer,
        width: i32,
        height: i32,
        max_length: usize,
        max_lines: Option<usize>,
        initial: &str,
    ) -> Self {
        let mut edit = Self {
            value: Vec::new(),
            cursor: 0,
            anchor: 0,
            max_length,
            max_lines,
            x: 0,
            y: 0,
            width,
            height,
            scroll: 0.0,
            focused: false,
            focused_at: 0,
            preedit: None,
        };
        let value = truncate_utf16(initial.chars(), max_length);
        if edit.accepts(font, &value) {
            edit.cursor = value.len();
            edit.anchor = edit.cursor;
            edit.value = value;
        }
        edit.scroll_to_cursor(font);
        edit
    }
    pub fn value(&self) -> String {
        self.value.iter().collect()
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
    pub fn is_focused(&self) -> bool {
        self.focused
    }
    pub fn set_focused(&mut self, focused: bool, now: u64) {
        if focused && !self.focused {
            self.focused_at = now;
        }
        self.focused = focused;
        if !focused {
            self.preedit = None;
        }
    }
    pub fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }
    pub fn set_size(&mut self, font: &dyn FontWidthMeasurer, width: i32, height: i32) {
        self.width = width;
        self.height = height;
        self.scroll_to_cursor(font);
    }
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x as f64
            && y >= self.y as f64
            && x < (self.x + self.width) as f64
            && y < (self.y + self.height) as f64
    }
    fn selection(&self) -> Range<usize> {
        self.cursor.min(self.anchor)..self.cursor.max(self.anchor)
    }
    fn accepts(&self, font: &dyn FontWidthMeasurer, value: &[char]) -> bool {
        self.max_lines
            .is_none_or(|limit| lines(font, value, self.width - 8).len() <= limit)
    }
    pub fn insert_text(&mut self, font: &dyn FontWidthMeasurer, text: &str) {
        self.preedit = None;
        let remaining = self
            .max_length
            .saturating_sub(self.value.iter().map(|c| c.len_utf16()).sum());
        let inserted = truncate_utf16(
            text.chars()
                .filter(|c| *c == '\n' || is_allowed_chat_character(*c)),
            remaining,
        );
        let selected = self.selection();
        let mut candidate = self.value.clone();
        candidate.splice(selected.clone(), inserted.iter().copied());
        if self.accepts(font, &candidate) {
            self.value = candidate;
            self.cursor = selected.start + inserted.len();
            self.anchor = self.cursor;
            self.scroll_to_cursor(font);
        }
    }
    fn move_to(&mut self, font: &dyn FontWidthMeasurer, position: usize, shift: bool) {
        self.preedit = None;
        self.cursor = position.min(self.value.len());
        if !shift {
            self.anchor = self.cursor;
        }
        self.scroll_to_cursor(font);
    }
    fn word_position(&self, forward: bool) -> usize {
        let mut position = self.cursor;
        if forward {
            while position < self.value.len() && !self.value[position].is_whitespace() {
                position += 1;
            }
            while position < self.value.len() && self.value[position].is_whitespace() {
                position += 1;
            }
        } else {
            while position > 0 && self.value[position - 1].is_whitespace() {
                position -= 1;
            }
            while position > 0 && !self.value[position - 1].is_whitespace() {
                position -= 1;
            }
        }
        position
    }
    pub fn key_pressed(
        &mut self,
        font: &dyn FontWidthMeasurer,
        event: KeyEvent,
        clipboard: Option<&mut dyn Clipboard>,
    ) -> bool {
        if !self.focused {
            return false;
        }
        if event.is_select_all() {
            self.anchor = 0;
            self.cursor = self.value.len();
            self.scroll_to_cursor(font);
            return true;
        }
        if event.is_copy() || event.is_cut() || event.is_paste() {
            if let Some(clipboard) = clipboard {
                if event.is_paste() {
                    self.insert_text(font, &clipboard.get());
                } else {
                    clipboard.set(self.value[self.selection()].iter().collect());
                    if event.is_cut() {
                        self.insert_text(font, "");
                    }
                }
            }
            return true;
        }
        let shift = event.has_shift_down();
        let ctrl = event.has_control_down();
        let display = lines(font, &self.value, self.width - 8);
        let current = line_at(&display, self.cursor);
        let target = match event.key {
            257 | 335 => {
                self.insert_text(font, "\n");
                return true;
            }
            KeyEvent::KEY_BACKSPACE | KeyEvent::KEY_DELETE => {
                if self.cursor == self.anchor {
                    let forward = event.key == KeyEvent::KEY_DELETE;
                    self.anchor = if ctrl {
                        self.word_position(forward)
                    } else if forward {
                        (self.cursor + 1).min(self.value.len())
                    } else {
                        self.cursor.saturating_sub(1)
                    };
                }
                self.insert_text(font, "");
                return true;
            }
            KeyEvent::KEY_LEFT => {
                if ctrl {
                    self.word_position(false)
                } else {
                    self.cursor.saturating_sub(1)
                }
            }
            KeyEvent::KEY_RIGHT => {
                if ctrl {
                    self.word_position(true)
                } else {
                    self.cursor + 1
                }
            }
            KeyEvent::KEY_HOME => {
                if ctrl {
                    0
                } else {
                    display[current].start
                }
            }
            KeyEvent::KEY_END => {
                if ctrl {
                    self.value.len()
                } else {
                    display[current].end
                }
            }
            266 => 0,
            267 => self.value.len(),
            KeyEvent::KEY_UP | KeyEvent::KEY_DOWN => {
                if ctrl {
                    return true;
                }
                let target_line = if event.key == KeyEvent::KEY_UP {
                    current.saturating_sub(1)
                } else {
                    (current + 1).min(display.len() - 1)
                };
                let left = width(font, &self.value[display[current].start..self.cursor]) + 2;
                let range = &display[target_line];
                range.start + column_at(font, &self.value[range.clone()], left)
            }
            _ => return false,
        };
        self.move_to(font, target, shift);
        true
    }
    pub fn char_typed(&mut self, font: &dyn FontWidthMeasurer, character: char) -> bool {
        if !self.focused || !is_allowed_chat_character(character) {
            return false;
        }
        self.insert_text(font, character.encode_utf8(&mut [0; 4]));
        true
    }
    pub fn preedit_updated(&mut self, event: Option<PreeditEvent>) -> bool {
        if !self.focused {
            return false;
        }
        self.preedit = event;
        true
    }
    pub fn on_click(
        &mut self,
        font: &dyn FontWidthMeasurer,
        x: f64,
        y: f64,
        shift: bool,
        double: bool,
    ) {
        if double {
            let start = self.word_position(false);
            let mut end = start;
            while end < self.value.len() && !self.value[end].is_whitespace() {
                end += 1;
            }
            self.anchor = start;
            self.cursor = end;
        } else {
            self.seek_point(font, x, y, shift);
        }
        self.scroll_to_cursor(font);
    }
    pub fn on_drag(&mut self, font: &dyn FontWidthMeasurer, x: f64, y: f64) {
        self.seek_point(font, x, y, true);
    }
    fn seek_point(&mut self, font: &dyn FontWidthMeasurer, x: f64, y: f64, shift: bool) {
        let display = lines(font, &self.value, self.width - 8);
        let row = (((y - self.y as f64 - 4.0 + self.scroll) / 9.0)
            .floor()
            .max(0.0) as usize)
            .min(display.len() - 1);
        let range = &display[row];
        let column = column_at(
            font,
            &self.value[range.clone()],
            (x - self.x as f64 - 4.0).floor() as i32,
        );
        self.move_to(font, range.start + column, shift);
    }
    pub fn mouse_scrolled(&mut self, font: &dyn FontWidthMeasurer, delta: f64) -> bool {
        let count = lines(font, &self.value, self.width - 8).len();
        let maximum = (count as i32 * 9 + 8 - self.height).max(0) as f64;
        self.scroll = (self.scroll - delta * 4.0).clamp(0.0, maximum);
        maximum > 0.0
    }
    fn scroll_to_cursor(&mut self, font: &dyn FontWidthMeasurer) {
        let display = lines(font, &self.value, self.width - 8);
        let top = (line_at(&display, self.cursor) * 9) as f64;
        self.scroll = self
            .scroll
            .min(top)
            .max(top + 9.0 - (self.height - 8).max(1) as f64)
            .max(0.0);
        self.scroll = self
            .scroll
            .min((display.len() as i32 * 9 + 8 - self.height).max(0) as f64);
    }
    pub fn render_plan(&self, font: &dyn FontWidthMeasurer, now: u64) -> EditBoxRenderPlan {
        let mut plan = EditBoxRenderPlan::default();
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
        let mut visual = self.value.clone();
        let mut cursor = self.cursor;
        let composition_range = self.preedit.as_ref().map(|preedit| {
            let range = self.selection();
            let text: Vec<char> = preedit.full_text.chars().collect();
            visual.splice(range.clone(), text.iter().copied());
            cursor = range.start + preedit.caret_position;
            cursor = cursor.min(range.start + text.len());
            range.start..range.start + text.len()
        });
        let display = lines(font, &visual, self.width - 8);
        let cursor_line = line_at(&display, cursor);
        let cursor_top = (cursor_line * 9) as f64;
        let scroll = if composition_range.is_some() {
            self.scroll
                .min(cursor_top)
                .max(cursor_top + 9.0 - (self.height - 8).max(1) as f64)
                .max(0.0)
        } else {
            self.scroll
        };
        for (row, range) in display.iter().enumerate() {
            let y = self.y + 4 + (row * 9) as i32 - scroll as i32;
            if y < self.y + 4 || y + 9 > self.y + self.height - 4 {
                continue;
            }
            let x = self.x + 4;
            plan.draws.push(EditBoxDraw::Text {
                text: visual[range.clone()].iter().collect(),
                x,
                y,
                color: -2039584,
                shadow: true,
            });
            if self.focused
                && is_cursor_visible(now.saturating_sub(self.focused_at))
                && row == cursor_line
            {
                let cursor_x = x + width(font, &visual[range.start..cursor.min(range.end)]);
                plan.draws.push(if cursor < visual.len() {
                    EditBoxDraw::InsertCursor {
                        x: cursor_x,
                        y,
                        color: -3092272,
                    }
                } else {
                    EditBoxDraw::AppendCursor {
                        x: cursor_x,
                        y,
                        color: -3092272,
                        shadow: true,
                    }
                });
            }
            let selected = composition_range
                .clone()
                .unwrap_or_else(|| self.selection());
            if selected.start < selected.end
                && selected.start <= range.end
                && selected.end >= range.start
            {
                let x0 = x + width(
                    font,
                    &visual[range.start..selected.start.max(range.start).min(range.end)],
                );
                let x1 = if selected.end > range.end {
                    self.x + self.width - 4
                } else {
                    x + width(font, &visual[range.start..selected.end.min(range.end)])
                };
                if composition_range.is_some() {
                    plan.draws.push(EditBoxDraw::Highlight {
                        x0,
                        y0: y + 8,
                        x1,
                        y1: y + 9,
                        invert: false,
                    });
                } else {
                    plan.draws.push(EditBoxDraw::Highlight {
                        x0,
                        y0: y,
                        x1,
                        y1: y + 9,
                        invert: true,
                    });
                }
            }
        }
        plan
    }
}
fn truncate_utf16(input: impl Iterator<Item = char>, maximum: usize) -> Vec<char> {
    let mut remaining = maximum;
    input
        .take_while(|c| {
            if c.len_utf16() > remaining {
                false
            } else {
                remaining -= c.len_utf16();
                true
            }
        })
        .collect()
}
fn width(font: &dyn FontWidthMeasurer, text: &[char]) -> i32 {
    font.measure_width(&text.iter().collect::<String>(), 1.0)
        .ceil() as i32
}
fn column_at(font: &dyn FontWidthMeasurer, text: &[char], pixels: i32) -> usize {
    let mut remaining = pixels as f32;
    text.iter()
        .take_while(|c| {
            remaining -= font.measure_char(**c, 1.0);
            remaining >= 0.0
        })
        .count()
}
fn line_at(lines: &[Range<usize>], cursor: usize) -> usize {
    lines
        .iter()
        .position(|line| line.start <= cursor && cursor <= line.end)
        .unwrap_or(lines.len() - 1)
}
fn lines(font: &dyn FontWidthMeasurer, value: &[char], max_width: i32) -> Vec<Range<usize>> {
    let mut output = Vec::new();
    let mut start = 0;
    while start < value.len() {
        let mut end = start;
        let mut measured = 0.0;
        let mut space = None;
        let mut nonzero = false;
        while end < value.len() {
            let c = value[end];
            if c == '\n' {
                break;
            }
            if c == ' ' {
                space = Some(end);
            }
            let advance = font.measure_char(c, 1.0);
            measured += advance;
            if nonzero && measured > max_width.max(1) as f32 {
                end = space.unwrap_or(end);
                break;
            }
            nonzero |= advance != 0.0;
            end += 1;
        }
        output.push(start..end);
        start = end + usize::from(value.get(end).is_some_and(|c| *c == '\n' || *c == ' '));
    }
    if output.is_empty() || value.last() == Some(&'\n') {
        output.push(value.len()..value.len());
    }
    output
}
