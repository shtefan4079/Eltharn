use crate::FontWidthMeasurer;
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::narration::{NarratedElementType, NarrationElementOutput};
use crate::framework::navigation::ScreenRectangle;
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
use std::sync::Arc;
pub const LIST_WIDTH: i32 = 280;
pub const FOOTER_HEIGHT: i32 = 33;
pub const DONE_WIDTH: i32 = 200;
pub const GENERAL_ROW_HEIGHT: i32 = 14;
pub const ITEM_ROW_HEIGHT: i32 = 22;
pub const ITEM_HEADER_HEIGHT: i32 = 22;
pub const MOB_LINE_HEIGHT: i32 = 9;
pub const MOB_TITLE_HEIGHT: i32 = 18;
pub const TAB_BAR_HEIGHT: i32 = 24;
pub const TAB_BAR_MAX_WIDTH: i32 = 400;
pub const TAB_BAR_MARGIN: i32 = 14;
const ITEM_LABEL_WIDTH: i32 = 75;
const ITEM_COLUMN_WIDTH: i32 = 40;
const SCROLLER_WIDTH: i32 = 6;
const MIN_SCROLLER_HEIGHT: i32 = 20;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatisticsCategory {
    General,
    Items,
    Mobs,
}
impl StatisticsCategory {
    pub const ALL: [Self; 3] = [Self::General, Self::Items, Self::Mobs];
    pub fn index(self) -> usize {
        match self {
            Self::General => 0,
            Self::Items => 1,
            Self::Mobs => 2,
        }
    }
    pub fn translation_key(self) -> &'static str {
        match self {
            Self::General => "stat.generalButton",
            Self::Items => "stat.itemsButton",
            Self::Mobs => "stat.mobsButton",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatisticsColumn {
    pub key: String,
    pub label: String,
}
impl StatisticsColumn {
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatisticsCell {
    pub display: String,
    pub sort_value: Option<i64>,
    pub muted: bool,
}
impl StatisticsCell {
    pub fn new(display: impl Into<String>, sort_value: Option<i64>) -> Self {
        Self {
            display: display.into(),
            sort_value,
            muted: false,
        }
    }
    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatisticsRow {
    pub key: String,
    pub label: String,
    pub cells: Vec<StatisticsCell>,
}
impl StatisticsRow {
    pub fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        cells: Vec<StatisticsCell>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            cells,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatisticsGroup {
    pub columns: Vec<StatisticsColumn>,
    pub rows: Vec<StatisticsRow>,
}
impl StatisticsGroup {
    pub fn new(columns: Vec<StatisticsColumn>, rows: Vec<StatisticsRow>) -> Self {
        Self { columns, rows }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatisticsData {
    pub general: StatisticsGroup,
    pub items: StatisticsGroup,
    pub mobs: StatisticsGroup,
}
impl StatisticsData {
    pub fn group(&self, category: StatisticsCategory) -> &StatisticsGroup {
        match category {
            StatisticsCategory::General => &self.general,
            StatisticsCategory::Items => &self.items,
            StatisticsCategory::Mobs => &self.mobs,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatisticsFocus {
    Tab(StatisticsCategory),
    ItemColumn(usize),
    Row(usize),
    Done,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatisticsAction {
    Done,
}
pub struct StatsScreen {
    base: ScreenBase,
    font: Arc<dyn FontWidthMeasurer>,
    data: StatisticsData,
    selected: StatisticsCategory,
    focus: StatisticsFocus,
    scroll_y: f32,
    dragging_scrollbar: bool,
    item_sort_column: Option<usize>,
    item_sort_order: i8,
    pending_action: Option<StatisticsAction>,
    in_world: bool,
}
impl StatsScreen {
    pub fn new(data: StatisticsData, font: Arc<dyn FontWidthMeasurer>) -> Self {
        let data = normalize_general_rows(data);
        Self {
            base: ScreenBase::new(Component::literal(&crate::translate("gui.stats"))),
            font,
            data,
            selected: StatisticsCategory::General,
            focus: StatisticsFocus::Tab(StatisticsCategory::General),
            scroll_y: 0.0,
            dragging_scrollbar: false,
            item_sort_column: None,
            item_sort_order: 0,
            pending_action: None,
            in_world: true,
        }
    }
    pub fn set_in_world(&mut self, in_world: bool) {
        self.in_world = in_world;
    }
    pub fn selected_category(&self) -> StatisticsCategory {
        self.selected
    }
    pub fn focus(&self) -> StatisticsFocus {
        self.focus
    }
    pub fn data(&self) -> &StatisticsData {
        &self.data
    }
    pub fn replace_data(&mut self, data: StatisticsData) {
        self.data = normalize_general_rows(data);
        self.scroll_y = 0.0;
        if !self.tab_enabled(self.selected) {
            self.selected = StatisticsCategory::General;
            self.focus = StatisticsFocus::Tab(self.selected);
        }
        if self
            .item_sort_column
            .is_some_and(|column| column >= self.data.items.columns.len())
        {
            self.item_sort_column = None;
            self.item_sort_order = 0;
        }
        let focus_valid = match self.focus {
            StatisticsFocus::Tab(category) => self.tab_enabled(category),
            StatisticsFocus::ItemColumn(column) => {
                self.selected == StatisticsCategory::Items && column < self.data.items.columns.len()
            }
            StatisticsFocus::Row(row) => row < self.data.group(self.selected).rows.len(),
            StatisticsFocus::Done => true,
        };
        if !focus_valid {
            self.focus = StatisticsFocus::Tab(self.selected);
        }
        self.clamp_scroll();
    }
    pub fn item_sort_state(&self) -> (Option<usize>, i8) {
        (self.item_sort_column, self.item_sort_order)
    }
    pub fn take_action(&mut self) -> Option<StatisticsAction> {
        self.pending_action.take()
    }
    pub fn displayed_row_keys(&self, category: StatisticsCategory) -> Vec<&str> {
        let group = self.data.group(category);
        self.ordered_row_indices(category)
            .into_iter()
            .map(|index| group.rows[index].key.as_str())
            .collect()
    }
    fn tab_enabled(&self, category: StatisticsCategory) -> bool {
        category == StatisticsCategory::General || !self.data.group(category).rows.is_empty()
    }
    fn round_toward(input: i32, multiple: i32) -> i32 {
        (input + multiple - 1) / multiple * multiple
    }
    fn tab_rect(&self, category: StatisticsCategory) -> ScreenRectangle {
        let tabs = StatisticsCategory::ALL.len() as i32;
        let tabs_width = TAB_BAR_MAX_WIDTH.min(self.base.width) - 2 * TAB_BAR_MARGIN;
        let tab_width = Self::round_toward(tabs_width / tabs, 2);
        let bar_left = Self::round_toward((self.base.width - tabs_width) / 2, 2);
        ScreenRectangle::new(
            bar_left + category.index() as i32 * tab_width,
            0,
            tab_width,
            TAB_BAR_HEIGHT,
        )
    }
    fn done_rect(&self) -> ScreenRectangle {
        ScreenRectangle::new(
            self.base.width / 2 - DONE_WIDTH / 2,
            self.base.height - FOOTER_HEIGHT + 6,
            DONE_WIDTH,
            20,
        )
    }
    fn list_top(&self) -> i32 {
        TAB_BAR_HEIGHT
    }
    fn list_bottom(&self) -> i32 {
        (self.base.height - FOOTER_HEIGHT).max(self.list_top())
    }
    fn viewport_height(&self) -> i32 {
        self.list_bottom() - self.list_top()
    }
    fn list_width(&self) -> i32 {
        let columns = self.data.items.columns.len() as i32;
        let preferred = if self.selected == StatisticsCategory::Items {
            LIST_WIDTH.max(ITEM_LABEL_WIDTH - ITEM_COLUMN_WIDTH + ITEM_COLUMN_WIDTH * columns)
        } else {
            LIST_WIDTH
        };
        preferred.min((self.base.width - 16).max(40))
    }
    fn list_left(&self) -> i32 {
        (self.base.width - self.list_width()) / 2
    }
    fn group_content_height(&self, category: StatisticsCategory) -> i32 {
        let group = self.data.group(category);
        match category {
            StatisticsCategory::General => group.rows.len() as i32 * GENERAL_ROW_HEIGHT,
            StatisticsCategory::Items => {
                let header = if group.rows.is_empty() || group.columns.is_empty() {
                    0
                } else {
                    ITEM_HEADER_HEIGHT
                };
                header + group.rows.len() as i32 * ITEM_ROW_HEIGHT
            }
            StatisticsCategory::Mobs => {
                group.rows.iter().map(|row| Self::mob_row_height(row)).sum()
            }
        }
    }
    fn mob_row_height(row: &StatisticsRow) -> i32 {
        (MOB_TITLE_HEIGHT + MOB_LINE_HEIGHT * row.cells.len() as i32).max(ITEM_ROW_HEIGHT)
    }
    fn max_scroll(&self) -> f32 {
        (self.group_content_height(self.selected) - self.viewport_height()).max(0) as f32
    }
    fn clamp_scroll(&mut self) {
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll());
    }
    fn ordered_row_indices(&self, category: StatisticsCategory) -> Vec<usize> {
        let group = self.data.group(category);
        let mut indices: Vec<_> = (0..group.rows.len()).collect();
        if category != StatisticsCategory::Items {
            return indices;
        }
        let Some(column) = self.item_sort_column else {
            return indices;
        };
        let order = self.item_sort_order;
        indices.sort_by(|left, right| {
            let left_row = &group.rows[*left];
            let right_row = &group.rows[*right];
            let left_value = left_row.cells.get(column).and_then(|cell| cell.sort_value);
            let right_value = right_row.cells.get(column).and_then(|cell| cell.sort_value);
            let value_order = match (left_value, right_value) {
                (Some(left), Some(right)) => left.cmp(&right),
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
            };
            if value_order == std::cmp::Ordering::Equal {
                left_row.key.cmp(&right_row.key)
            } else if order < 0 {
                value_order.reverse()
            } else {
                value_order
            }
        });
        indices
    }
    fn item_header_rect(&self, column: usize) -> ScreenRectangle {
        ScreenRectangle::new(
            self.list_left() + ITEM_LABEL_WIDTH - ITEM_COLUMN_WIDTH
                + column as i32 * ITEM_COLUMN_WIDTH,
            self.list_top() - self.scroll_y as i32,
            ITEM_COLUMN_WIDTH,
            ITEM_HEADER_HEIGHT,
        )
    }
    fn activate_item_column(&mut self, column: usize) {
        if column >= self.data.items.columns.len() {
            return;
        }
        if self.item_sort_column != Some(column) {
            self.item_sort_column = Some(column);
            self.item_sort_order = -1;
        } else if self.item_sort_order == -1 {
            self.item_sort_order = 1;
        } else {
            self.item_sort_column = None;
            self.item_sort_order = 0;
        }
        self.focus = StatisticsFocus::ItemColumn(column);
    }
    fn row_top(&self, index: usize) -> i32 {
        let position = self
            .ordered_row_indices(self.selected)
            .iter()
            .position(|candidate| *candidate == index)
            .unwrap_or(index);
        self.row_top_at(position)
    }
    fn row_top_at(&self, position: usize) -> i32 {
        let rows = &self.data.group(self.selected).rows;
        let content_y = match self.selected {
            StatisticsCategory::General => position as i32 * GENERAL_ROW_HEIGHT,
            StatisticsCategory::Items => {
                let header_height = if rows.is_empty() || self.data.items.columns.is_empty() {
                    0
                } else {
                    ITEM_HEADER_HEIGHT
                };
                header_height + position as i32 * ITEM_ROW_HEIGHT
            }
            StatisticsCategory::Mobs => self
                .ordered_row_indices(StatisticsCategory::Mobs)
                .iter()
                .take(position)
                .map(|index| Self::mob_row_height(&rows[*index]))
                .sum(),
        };
        self.list_top() + content_y - self.scroll_y as i32
    }
    fn row_height(&self, index: usize) -> i32 {
        let row = &self.data.group(self.selected).rows[index];
        match self.selected {
            StatisticsCategory::General => GENERAL_ROW_HEIGHT,
            StatisticsCategory::Items => ITEM_ROW_HEIGHT,
            StatisticsCategory::Mobs => Self::mob_row_height(row),
        }
    }
    fn ensure_focus_visible(&mut self) {
        let StatisticsFocus::Row(index) = self.focus else {
            return;
        };
        if index >= self.data.group(self.selected).rows.len() {
            return;
        }
        let top = self.row_top(index) + self.scroll_y as i32;
        let bottom = top + self.row_height(index);
        let viewport_top = self.list_top();
        let viewport_bottom = self.list_bottom();
        if top < viewport_top {
            self.scroll_y = (top - viewport_top) as f32;
        } else if bottom > viewport_bottom {
            self.scroll_y = (bottom - viewport_bottom) as f32;
        }
        self.clamp_scroll();
    }
    fn change_tab(&mut self, direction: i32) {
        let current = self.selected.index() as i32;
        for offset in 1..=StatisticsCategory::ALL.len() as i32 {
            let index =
                (current + direction * offset).rem_euclid(StatisticsCategory::ALL.len() as i32);
            let candidate = StatisticsCategory::ALL[index as usize];
            if self.tab_enabled(candidate) {
                self.selected = candidate;
                self.focus = StatisticsFocus::Tab(candidate);
                self.scroll_y = 0.0;
                return;
            }
        }
    }
    fn move_vertical(&mut self, direction: i32) {
        let row_count = self.data.group(self.selected).rows.len();
        self.focus = match self.focus {
            StatisticsFocus::Tab(_) if direction > 0 => {
                if self.selected == StatisticsCategory::Items
                    && !self.data.items.columns.is_empty()
                    && row_count > 0
                {
                    StatisticsFocus::ItemColumn(0)
                } else if row_count > 0 {
                    StatisticsFocus::Row(0)
                } else {
                    StatisticsFocus::Done
                }
            }
            StatisticsFocus::Tab(_) => StatisticsFocus::Done,
            StatisticsFocus::ItemColumn(_) if direction > 0 && row_count > 0 => {
                StatisticsFocus::Row(0)
            }
            StatisticsFocus::ItemColumn(_) if direction < 0 => StatisticsFocus::Tab(self.selected),
            StatisticsFocus::ItemColumn(column) => StatisticsFocus::ItemColumn(
                column.min(self.data.items.columns.len().saturating_sub(1)),
            ),
            StatisticsFocus::Row(index) if direction < 0 && index == 0 => {
                if self.selected == StatisticsCategory::Items && !self.data.items.columns.is_empty()
                {
                    StatisticsFocus::ItemColumn(0)
                } else {
                    StatisticsFocus::Tab(self.selected)
                }
            }
            StatisticsFocus::Row(index) if direction < 0 => StatisticsFocus::Row(index - 1),
            StatisticsFocus::Row(index) if direction > 0 && index + 1 < row_count => {
                StatisticsFocus::Row(index + 1)
            }
            StatisticsFocus::Row(_) => StatisticsFocus::Done,
            StatisticsFocus::Done if direction < 0 && row_count > 0 => {
                StatisticsFocus::Row(row_count - 1)
            }
            StatisticsFocus::Done => StatisticsFocus::Tab(self.selected),
        };
        self.ensure_focus_visible();
    }
    fn tab_navigation_order(&self) -> Vec<StatisticsFocus> {
        let mut focusables: Vec<_> = StatisticsCategory::ALL
            .into_iter()
            .filter(|category| self.tab_enabled(*category))
            .map(StatisticsFocus::Tab)
            .collect();
        if self.selected == StatisticsCategory::Items {
            focusables.extend((0..self.data.items.columns.len()).map(StatisticsFocus::ItemColumn));
        }
        focusables.extend((0..self.data.group(self.selected).rows.len()).map(StatisticsFocus::Row));
        focusables.push(StatisticsFocus::Done);
        focusables
    }
    fn move_tab_focus(&mut self, backwards: bool) {
        let order = self.tab_navigation_order();
        if order.is_empty() {
            return;
        }
        let current = order
            .iter()
            .position(|target| *target == self.focus)
            .unwrap_or(0);
        let next = if backwards {
            (current + order.len() - 1) % order.len()
        } else {
            (current + 1) % order.len()
        };
        self.focus = order[next];
        self.ensure_focus_visible();
    }
    fn hit_row(&self, x: i32, y: i32) -> Option<usize> {
        if y < self.list_top() || y >= self.list_bottom() {
            return None;
        }
        let left = self.list_left();
        let right = left + self.list_width();
        if x < left || x >= right {
            return None;
        }
        self.ordered_row_indices(self.selected)
            .into_iter()
            .enumerate()
            .find_map(|(position, index)| {
                let top = self.row_top_at(position);
                (y >= top && y < top + self.row_height(index)).then_some(index)
            })
    }
    fn scrollbar_rects(&self) -> Option<(ScreenRectangle, ScreenRectangle)> {
        let viewport = self.viewport_height();
        let max_scroll = self.max_scroll();
        let content_height = self.group_content_height(self.selected);
        if viewport <= 0 || max_scroll <= 0.0 {
            return None;
        }
        let track = ScreenRectangle::new(
            self.list_left() + self.list_width() - SCROLLER_WIDTH,
            self.list_top(),
            SCROLLER_WIDTH,
            viewport,
        );
        let height = ((viewport as i64 * viewport as i64 / content_height.max(1) as i64) as i32)
            .clamp(MIN_SCROLLER_HEIGHT.min(viewport), viewport);
        let travel = viewport - height;
        let top = self.list_top()
            + if travel == 0 {
                0
            } else {
                (self.scroll_y * travel as f32 / max_scroll).round() as i32
            };
        Some((
            track,
            ScreenRectangle::new(track.left(), top, track.width, height),
        ))
    }
    fn scroll_to_thumb(&mut self, mouse_y: i32) {
        let Some((track, thumb)) = self.scrollbar_rects() else {
            return;
        };
        let travel = track.height - thumb.height;
        self.scroll_y = if travel <= 0 {
            0.0
        } else {
            let offset = (mouse_y - thumb.height / 2 - track.top()).clamp(0, travel);
            offset as f32 / travel as f32 * self.max_scroll()
        };
    }
    fn activate_focused(&mut self) {
        match self.focus {
            StatisticsFocus::Tab(category) => {
                if self.tab_enabled(category) {
                    if category != self.selected {
                        self.selected = category;
                        self.scroll_y = 0.0;
                    }
                    self.focus = StatisticsFocus::Tab(category);
                }
            }
            StatisticsFocus::ItemColumn(column) => self.activate_item_column(column),
            StatisticsFocus::Row(_) => {}
            StatisticsFocus::Done => self.pending_action = Some(StatisticsAction::Done),
        }
    }
    fn select_tab(&mut self, category: StatisticsCategory) {
        if !self.tab_enabled(category) {
            return;
        }
        self.selected = category;
        self.focus = StatisticsFocus::Tab(category);
        self.scroll_y = 0.0;
    }
    fn fit_text(&self, text: &str, max_width: i32) -> String {
        let max_width = max_width.max(0) as f32;
        if self.font.measure_width(text, 1.0) <= max_width {
            return text.to_owned();
        }
        let mut prefix = String::new();
        for ch in text.chars() {
            let candidate = format!("{prefix}{ch}…");
            if self.font.measure_width(&candidate, 1.0) > max_width {
                break;
            }
            prefix.push(ch);
        }
        if prefix.is_empty() && self.font.measure_width("…", 1.0) > max_width {
            String::new()
        } else {
            format!("{prefix}…")
        }
    }
}
fn normalize_general_rows(mut data: StatisticsData) -> StatisticsData {
    data.general.rows.sort_by(|left, right| {
        left.label
            .cmp(&right.label)
            .then_with(|| left.key.cmp(&right.key))
    });
    data
}
fn draw_menu_background_region(
    graphics: &mut GuiGraphicsExtractor,
    in_world: bool,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    let texture = if in_world {
        "minecraft:gui/inworld_menu_background"
    } else {
        "minecraft:gui/menu_background"
    };
    draw_tiled_sprite(graphics, texture, x, y, width, height, 32);
}
fn draw_tiled_sprite(
    graphics: &mut GuiGraphicsExtractor,
    texture: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    tile_size: i32,
) {
    if width <= 0 || height <= 0 || tile_size <= 0 {
        return;
    }
    let texture = ResourceLocation::from_string(texture);
    let mut cursor_y = y;
    while cursor_y < y + height {
        let tile_h = tile_size.min(y + height - cursor_y);
        let mut cursor_x = x;
        while cursor_x < x + width {
            let tile_w = tile_size.min(x + width - cursor_x);
            graphics.blit_region(
                texture.clone(),
                cursor_x,
                cursor_y,
                tile_w,
                tile_h,
                0.0,
                0.0,
                tile_w as f32 / tile_size as f32,
                tile_h as f32 / tile_size as f32,
                -1,
            );
            cursor_x += tile_w;
        }
        cursor_y += tile_h;
    }
}
impl Screen for StatsScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        self.clamp_scroll();
    }
    fn reposition_elements(&mut self) {
        self.clamp_scroll();
        self.ensure_focus_visible();
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if event.is_escape() {
            self.pending_action = Some(StatisticsAction::Done);
            return true;
        }
        if event.key == KeyEvent::KEY_TAB {
            self.move_tab_focus(event.has_shift_down());
            return true;
        }
        match event.key {
            KeyEvent::KEY_LEFT => match self.focus {
                StatisticsFocus::Tab(_) => self.change_tab(-1),
                StatisticsFocus::ItemColumn(column) if column > 0 => {
                    self.focus = StatisticsFocus::ItemColumn(column - 1);
                }
                StatisticsFocus::ItemColumn(_) => {
                    self.focus = StatisticsFocus::Tab(self.selected);
                }
                _ => return false,
            },
            KeyEvent::KEY_RIGHT => match self.focus {
                StatisticsFocus::Tab(_) => self.change_tab(1),
                StatisticsFocus::ItemColumn(column)
                    if column + 1 < self.data.items.columns.len() =>
                {
                    self.focus = StatisticsFocus::ItemColumn(column + 1);
                }
                StatisticsFocus::ItemColumn(_) => self.focus = StatisticsFocus::Done,
                _ => return false,
            },
            KeyEvent::KEY_UP => self.move_vertical(-1),
            KeyEvent::KEY_DOWN => self.move_vertical(1),
            KeyEvent::KEY_ENTER | KeyEvent::KEY_SPACE => self.activate_focused(),
            _ => return false,
        }
        true
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        if event.button() != 0 {
            return false;
        }
        let x = event.x as i32;
        let y = event.y as i32;
        for category in StatisticsCategory::ALL {
            let tab = self.tab_rect(category);
            if tab.contains_point(x, y) {
                if !self.tab_enabled(category) {
                    return false;
                }
                let changed = self.selected != category;
                self.select_tab(category);
                if changed {
                    crate::components::click_sound_sink::record_button_click();
                }
                return true;
            }
        }
        if self.done_rect().contains_point(x, y) {
            crate::components::click_sound_sink::record_button_click();
            self.focus = StatisticsFocus::Done;
            self.pending_action = Some(StatisticsAction::Done);
            return true;
        }
        if self.selected == StatisticsCategory::Items {
            for column in 0..self.data.items.columns.len() {
                let header = self.item_header_rect(column);
                if header.bottom() > self.list_top()
                    && header.top() < self.list_bottom()
                    && header.contains_point(x, y)
                {
                    crate::components::click_sound_sink::record_button_click();
                    self.activate_item_column(column);
                    return true;
                }
            }
        }
        if let Some((track, _thumb)) = self.scrollbar_rects() {
            if track.contains_point(x, y) {
                self.dragging_scrollbar = true;
                self.scroll_to_thumb(y);
                return true;
            }
        }
        if let Some(row) = self.hit_row(x, y) {
            self.focus = StatisticsFocus::Row(row);
            return true;
        }
        false
    }
    fn mouse_released(&mut self, _event: MouseButtonEvent) -> bool {
        let was_dragging = self.dragging_scrollbar;
        self.dragging_scrollbar = false;
        was_dragging
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, _dx: f64, _dy: f64) -> bool {
        if !self.dragging_scrollbar {
            return false;
        }
        self.scroll_to_thumb(event.y as i32);
        true
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, _scroll_x: f64, scroll_y: f64) -> bool {
        let left = self.list_left();
        if x < left as f64
            || x >= (left + self.list_width()) as f64
            || y < self.list_top() as f64
            || y >= self.list_bottom() as f64
            || self.max_scroll() <= 0.0
        {
            return false;
        }
        self.scroll_y = (self.scroll_y - scroll_y as f32 * 10.0).clamp(0.0, self.max_scroll());
        true
    }
    fn update_narration(&mut self, output: &mut NarrationElementOutput) {
        self.base_ref().update_narration(output);
        match self.focus {
            StatisticsFocus::Tab(category) => output.add(
                NarratedElementType::Position,
                crate::translate(category.translation_key()),
            ),
            StatisticsFocus::ItemColumn(column) => {
                if let Some(column) = self.data.items.columns.get(column) {
                    output.add(NarratedElementType::Position, column.label.clone());
                    output.add(NarratedElementType::Hint, "Press Enter to sort this column");
                }
            }
            StatisticsFocus::Row(index) => {
                if let Some(row) = self.data.group(self.selected).rows.get(index) {
                    let cells = row
                        .cells
                        .iter()
                        .map(|cell| cell.display.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    output.add(
                        NarratedElementType::Position,
                        if cells.is_empty() {
                            row.label.clone()
                        } else {
                            format!("{}, {}", row.label, cells)
                        },
                    );
                }
            }
            StatisticsFocus::Done => {
                output.add(NarratedElementType::Position, crate::translate("gui.done"));
            }
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
        let height = graphics.gui_height();
        self.base
            .extract_background_backdrop(graphics, self.in_world);
        draw_menu_background_region(graphics, self.in_world, 0, TAB_BAR_HEIGHT, width, height);
        draw_tiled_sprite(
            graphics,
            "minecraft:gui/tab_header_background",
            0,
            0,
            width,
            TAB_BAR_HEIGHT,
            16,
        );
        let first_tab = self.tab_rect(StatisticsCategory::General);
        let after_last_tab = self.tab_rect(StatisticsCategory::Mobs).right();
        draw_tiled_sprite(
            graphics,
            "minecraft:gui/header_separator",
            0,
            TAB_BAR_HEIGHT - 2,
            first_tab.left(),
            2,
            32,
        );
        draw_tiled_sprite(
            graphics,
            "minecraft:gui/header_separator",
            after_last_tab,
            TAB_BAR_HEIGHT - 2,
            width - after_last_tab,
            2,
            32,
        );
        for category in StatisticsCategory::ALL {
            let rect = self.tab_rect(category);
            let tab_label = crate::translate(category.translation_key());
            graphics.draw_button_with_focus(
                self.font.as_ref(),
                rect.left(),
                rect.top(),
                rect.width,
                rect.height,
                &tab_label,
                self.tab_enabled(category),
                self.focus == StatisticsFocus::Tab(category),
            );
        }
        graphics.enable_scissor(0, self.list_top(), width, self.list_bottom());
        match self.selected {
            StatisticsCategory::General => self.draw_general(graphics),
            StatisticsCategory::Items => self.draw_items(graphics),
            StatisticsCategory::Mobs => self.draw_mobs(graphics),
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
        draw_tiled_sprite(
            graphics,
            "minecraft:gui/footer_separator",
            0,
            height - FOOTER_HEIGHT,
            width,
            2,
            32,
        );
        let done = self.done_rect();
        graphics.draw_button_with_focus(
            self.font.as_ref(),
            done.left(),
            done.top(),
            done.width,
            done.height,
            &crate::translate("gui.done"),
            true,
            self.focus == StatisticsFocus::Done,
        );
        let _ = (mouse_x, mouse_y);
    }
}
impl StatsScreen {
    fn draw_general(&self, graphics: &mut GuiGraphicsExtractor) {
        let group = &self.data.general;
        let indices = self.ordered_row_indices(StatisticsCategory::General);
        let left = (graphics.gui_width() - LIST_WIDTH) / 2;
        let right = left + LIST_WIDTH - SCROLLER_WIDTH - 4;
        for (position, index) in indices.into_iter().enumerate() {
            let row = &group.rows[index];
            let y = self.row_top_at(position);
            let color = if position % 2 == 0 {
                0xFFFF_FFFFu32 as i32
            } else {
                0xFFBA_BABAu32 as i32
            };
            if self.focus == StatisticsFocus::Row(index) {
                graphics.fill(
                    left,
                    y,
                    left + LIST_WIDTH - SCROLLER_WIDTH,
                    y + GENERAL_ROW_HEIGHT,
                    0x40FF_FFFF,
                );
            }
            if let Some(cell) = row.cells.first() {
                let value = self.fit_text(&cell.display, right - left - 8);
                let value_width = self.font.measure_width(&value, 1.0).round() as i32;
                let label_width = (right - left - value_width - 12).max(0);
                graphics.text(
                    ResourceLocation::from_string("minecraft:default"),
                    self.fit_text(&row.label, label_width),
                    left + 2,
                    y + 2,
                    color,
                );
                graphics.text(
                    ResourceLocation::from_string("minecraft:default"),
                    value,
                    right - value_width,
                    y + 2,
                    if cell.muted {
                        0xFF7F7F7Fu32 as i32
                    } else {
                        color
                    },
                );
            } else {
                graphics.text(
                    ResourceLocation::from_string("minecraft:default"),
                    self.fit_text(&row.label, right - left - 8),
                    left + 2,
                    y + 2,
                    color,
                );
            }
        }
    }
    fn draw_items(&self, graphics: &mut GuiGraphicsExtractor) {
        let group = &self.data.items;
        let left = self.list_left();
        let indices = self.ordered_row_indices(StatisticsCategory::Items);
        let header_visible = !group.rows.is_empty() && !group.columns.is_empty();
        if header_visible {
            let header_y = self.list_top() - self.scroll_y as i32;
            for (column_index, column) in group.columns.iter().enumerate() {
                let rect = self.item_header_rect(column_index);
                let label = compact_header_label(&column.label);
                let marker = if self.item_sort_column == Some(column_index) {
                    if self.item_sort_order < 0 {
                        " ▼"
                    } else {
                        " ▲"
                    }
                } else {
                    ""
                };
                let button_text = self.fit_text(&format!("{label}{marker}"), rect.width - 4);
                graphics.draw_button_with_focus(
                    self.font.as_ref(),
                    rect.left(),
                    header_y,
                    rect.width,
                    rect.height,
                    &button_text,
                    true,
                    self.focus == StatisticsFocus::ItemColumn(column_index),
                );
            }
        }
        for (position, index) in indices.into_iter().enumerate() {
            let row = &group.rows[index];
            let y = self.row_top_at(position);
            let color = if position % 2 == 0 {
                0xFFFF_FFFFu32 as i32
            } else {
                0xFFBA_BABAu32 as i32
            };
            if self.focus == StatisticsFocus::Row(index) {
                graphics.fill(
                    left,
                    y,
                    left + self.list_width() - SCROLLER_WIDTH,
                    y + ITEM_ROW_HEIGHT,
                    0x40FF_FFFF,
                );
            }
            let item_location = ResourceLocation::try_from_string(&row.key);
            let item = item_location.as_ref().and_then(|location| {
                let candidate =
                    el_core::registry::BuiltInRegistries::items().get_by_location(location);
                (candidate.location == *location).then(|| candidate.clone())
            });
            if let Some(item) = item {
                graphics.blit(
                    ResourceLocation::from_string("minecraft:container/slot"),
                    left + 1,
                    y + 2,
                    18,
                    18,
                );
                graphics.render_item(el_core::item::ItemStack::new(item, 1), left + 2, y + 3);
            }
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                self.fit_text(&row.label, ITEM_LABEL_WIDTH - ITEM_COLUMN_WIDTH - 4),
                left + 2,
                y + 7,
                color,
            );
            for (column_index, cell) in row.cells.iter().enumerate().take(group.columns.len()) {
                let x = left + ITEM_LABEL_WIDTH - ITEM_COLUMN_WIDTH
                    + column_index as i32 * ITEM_COLUMN_WIDTH;
                let value = self.fit_text(&cell.display, ITEM_COLUMN_WIDTH - 5);
                let value_width = self.font.measure_width(&value, 1.0).round() as i32;
                graphics.text(
                    ResourceLocation::from_string("minecraft:default"),
                    value,
                    x + ITEM_COLUMN_WIDTH - 3 - value_width,
                    y + 7,
                    if cell.muted {
                        0xFF7F7F7Fu32 as i32
                    } else {
                        color
                    },
                );
            }
        }
    }
    fn draw_mobs(&self, graphics: &mut GuiGraphicsExtractor) {
        let group = &self.data.mobs;
        let left = self.list_left();
        for (position, index) in self
            .ordered_row_indices(StatisticsCategory::Mobs)
            .into_iter()
            .enumerate()
        {
            let row = &group.rows[index];
            let y = self.row_top_at(position);
            let height = Self::mob_row_height(row);
            let color = if position % 2 == 0 {
                0xFFFF_FFFFu32 as i32
            } else {
                0xFFBA_BABAu32 as i32
            };
            if self.focus == StatisticsFocus::Row(index) {
                graphics.fill(
                    left,
                    y,
                    left + self.list_width() - SCROLLER_WIDTH,
                    y + height,
                    0x40FF_FFFF,
                );
            }
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                self.fit_text(&row.label, self.list_width() - SCROLLER_WIDTH - 8),
                left + 2,
                y + 1,
                color,
            );
            for (line, cell) in row.cells.iter().enumerate() {
                graphics.text(
                    ResourceLocation::from_string("minecraft:default"),
                    self.fit_text(&cell.display, self.list_width() - SCROLLER_WIDTH - 20),
                    left + 12,
                    y + MOB_TITLE_HEIGHT - 8 + line as i32 * MOB_LINE_HEIGHT,
                    if cell.muted {
                        0xFF7F7F7Fu32 as i32
                    } else {
                        0xFFBA_BABAu32 as i32
                    },
                );
            }
        }
    }
}
fn compact_header_label(label: &str) -> String {
    let mut shortened: String = label.chars().take(3).collect();
    if label.chars().count() > 3 {
        shortened.push('…');
    }
    shortened
}
