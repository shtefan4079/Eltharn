use crate::components::scroll_area::{AbstractScrollArea, ScrollbarSettings};
use crate::components::{
    GuiEventListener, NarratableEntry, Renderable, TabOrderedElement, WidgetBase,
};
use crate::framework::focus::{resolve_arrow, resolve_arrow_from, resolve_tab};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::{HeaderAndFooterLayout, LayoutElement};
use crate::framework::narration::{NarratedElementType, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{
    ComponentPath, FocusNavigationEvent, ScreenDirection, ScreenRectangle,
};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
pub const DEFAULT_ROW_WIDTH: i32 = 220;
pub const CONTENT_PADDING: i32 = 2;
pub const SEPARATOR_HEIGHT: i32 = 2;
pub const CONTENT_HEIGHT_PADDING: i32 = 4;
pub const HEADER_SEPARATOR: &str = "minecraft:gui/header_separator";
pub const FOOTER_SEPARATOR: &str = "minecraft:gui/footer_separator";
pub const INWORLD_HEADER_SEPARATOR: &str = "minecraft:gui/inworld_header_separator";
pub const INWORLD_FOOTER_SEPARATOR: &str = "minecraft:gui/inworld_footer_separator";
pub const SELECTION_FOCUSED_OUTLINE: i32 = -1;
pub const SELECTION_UNFOCUSED_OUTLINE: i32 = -8355712;
pub const SELECTION_INNER_FILL: i32 = -16777216;
pub const DEFAULT_ENTRY_HEIGHT: i32 = 20;
const BACKGROUND_TEXTURE_SIZE: i32 = 32;
const SCROLLBAR_GAP: i32 = 2;
fn mth_clamp(value: i32, min: i32, max: i32) -> i32 {
    value.max(min).min(max)
}
fn bound_in_direction(rect: ScreenRectangle, direction: ScreenDirection) -> i32 {
    let (start, length) = match direction.axis() {
        crate::framework::navigation::ScreenAxis::Horizontal => (rect.left(), rect.width),
        crate::framework::navigation::ScreenAxis::Vertical => (rect.top(), rect.height),
    };
    if direction.is_positive() {
        start + length - 1
    } else {
        start
    }
}
fn rectangle_border(rect: ScreenRectangle, direction: ScreenDirection) -> ScreenRectangle {
    let axis = direction.axis();
    let orthogonal = axis.orthogonal();
    let start_first = bound_in_direction(rect, direction);
    let start_second = bound_in_direction(rect, orthogonal.negative());
    let length = match orthogonal {
        crate::framework::navigation::ScreenAxis::Horizontal => rect.width,
        crate::framework::navigation::ScreenAxis::Vertical => rect.height,
    };
    let (x, y, width, height) = match axis {
        crate::framework::navigation::ScreenAxis::Horizontal => {
            (start_first, start_second, 1, length)
        }
        crate::framework::navigation::ScreenAxis::Vertical => {
            (start_second, start_first, length, 1)
        }
    };
    let (x, y) = match direction {
        ScreenDirection::Left => (x - 1, y),
        ScreenDirection::Right => (x + 1, y),
        ScreenDirection::Up => (x, y - 1),
        ScreenDirection::Down => (x, y + 1),
    };
    ScreenRectangle::new(x, y, width, height)
}
fn vertical_direction_for_initial_focus(event: &FocusNavigationEvent) -> ScreenDirection {
    match event {
        FocusNavigationEvent::TabNavigation { forward: true } => ScreenDirection::Down,
        FocusNavigationEvent::TabNavigation { forward: false } => ScreenDirection::Up,
        FocusNavigationEvent::ArrowNavigation { direction } => {
            if direction.axis() == crate::framework::navigation::ScreenAxis::Vertical {
                *direction
            } else {
                ScreenDirection::Down
            }
        }
        FocusNavigationEvent::InitialFocus => ScreenDirection::Down,
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryLayout {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    padding: i32,
}
impl Default for EntryLayout {
    fn default() -> Self {
        Self::new()
    }
}
impl EntryLayout {
    pub fn new() -> Self {
        Self::with_padding(CONTENT_PADDING)
    }
    pub fn with_padding(padding: i32) -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            padding,
        }
    }
    pub fn set_rectangle(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
    }
    pub fn set_from_rectangle(&mut self, rectangle: ScreenRectangle) {
        self.set_rectangle(
            rectangle.left(),
            rectangle.top(),
            rectangle.width,
            rectangle.height,
        );
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
    pub fn padding(&self) -> i32 {
        self.padding
    }
    pub fn set_padding(&mut self, padding: i32) {
        self.padding = padding;
    }
    pub fn rectangle(&self) -> ScreenRectangle {
        ScreenRectangle::new(self.x, self.y, self.width, self.height)
    }
    pub fn contains_point(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.rectangle()
            .contains_point(mouse_x as i32, mouse_y as i32)
    }
    pub fn content_x(&self) -> i32 {
        self.x + self.padding
    }
    pub fn content_y(&self) -> i32 {
        self.y + self.padding
    }
    pub fn content_width(&self) -> i32 {
        self.width - 2 * self.padding
    }
    pub fn content_height(&self) -> i32 {
        self.height - 2 * self.padding
    }
    pub fn content_y_middle(&self) -> i32 {
        self.content_y() + self.content_height() / 2
    }
    pub fn content_x_middle(&self) -> i32 {
        self.content_x() + self.content_width() / 2
    }
    pub fn content_bottom(&self) -> i32 {
        self.content_y() + self.content_height()
    }
    pub fn content_right(&self) -> i32 {
        self.content_x() + self.content_width()
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntryRenderContext {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub row_index: usize,
    pub selected: bool,
    pub focused: bool,
    pub hovered: bool,
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub partial_tick: f32,
}
impl EntryRenderContext {
    pub fn content_x(&self) -> i32 {
        self.x + CONTENT_PADDING
    }
    pub fn content_y(&self) -> i32 {
        self.y + CONTENT_PADDING
    }
    pub fn content_width(&self) -> i32 {
        self.width - 2 * CONTENT_PADDING
    }
    pub fn content_height(&self) -> i32 {
        self.height - 2 * CONTENT_PADDING
    }
    pub fn content_y_middle(&self) -> i32 {
        self.content_y() + self.content_height() / 2
    }
    pub fn content_x_middle(&self) -> i32 {
        self.content_x() + self.content_width() / 2
    }
    pub fn content_bottom(&self) -> i32 {
        self.content_y() + self.content_height()
    }
    pub fn content_right(&self) -> i32 {
        self.content_x() + self.content_width()
    }
    pub fn rectangle(&self) -> ScreenRectangle {
        ScreenRectangle::new(self.x, self.y, self.width, self.height)
    }
}
pub trait SelectionListEntry: Renderable {
    fn height(&self) -> i32;
    fn extract_content(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        context: &EntryRenderContext,
    );
    fn entry_rectangle(&self) -> ScreenRectangle;
    fn set_entry_rectangle(&mut self, rectangle: ScreenRectangle);
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.entry_rectangle()
            .contains_point(mouse_x as i32, mouse_y as i32)
    }
    fn is_entry_focused(&self) -> bool {
        false
    }
    fn set_entry_focused(&mut self, _focused: bool) {}
    fn mouse_clicked_entry(&mut self, _event: MouseButtonEvent, _double_click: bool) -> bool {
        true
    }
    fn event_listener(&mut self) -> Option<&mut dyn GuiEventListener> {
        None
    }
    fn narrate_entry(&self, _output: &mut NarrationElementOutput) {}
    fn has_children(&self) -> bool {
        false
    }
}
#[derive(Clone, Debug)]
struct ListCore<E: SelectionListEntry> {
    base: WidgetBase,
    entries: Vec<E>,
    selected: Option<usize>,
    hovered: Option<usize>,
    focused: Option<usize>,
    dragging: bool,
    default_entry_height: i32,
    row_width: i32,
    render_header: bool,
    render_background: bool,
    in_world: bool,
    center_list_vertically: bool,
    selectable: bool,
    clear_selection_on_defocus: bool,
    scroll: AbstractScrollArea,
}
impl<E: SelectionListEntry> ListCore<E> {
    fn new(x: i32, y: i32, width: i32, height: i32, default_entry_height: i32) -> Self {
        let message = Component::default();
        let mut scroll = AbstractScrollArea::new(
            0,
            y,
            2 * crate::components::scroll_area::SCROLLBAR_WIDTH,
            height,
            message,
            ScrollbarSettings::default_settings(default_entry_height / 2),
        );
        scroll.visible = true;
        let mut core = Self {
            base: WidgetBase::new(x, y, width, height, Component::default()),
            entries: Vec::new(),
            selected: None,
            hovered: None,
            focused: None,
            dragging: false,
            default_entry_height,
            row_width: DEFAULT_ROW_WIDTH,
            render_header: true,
            render_background: true,
            in_world: false,
            center_list_vertically: true,
            selectable: true,
            clear_selection_on_defocus: false,
            scroll,
        };
        core.sync_scroll_area();
        core
    }
    fn first_entry_y(&self) -> i32 {
        self.base.y() + SEPARATOR_HEIGHT
    }
    fn base_bottom(&self) -> i32 {
        self.base.y() + self.base.height()
    }
    fn base_right(&self) -> i32 {
        self.base.x() + self.base.width()
    }
    fn next_y(&self) -> i32 {
        let mut y = self.first_entry_y() - self.scroll.scroll_amount as i32;
        for entry in &self.entries {
            y += entry.height();
        }
        y
    }
    fn reposition_entries(&mut self) {
        let row_left = self.row_left();
        let row_width = self.row_width;
        let mut y = self.first_entry_y() - self.scroll.scroll_amount as i32;
        for entry in &mut self.entries {
            let height = entry.height();
            entry.set_entry_rectangle(ScreenRectangle::new(row_left, y, row_width, height));
            y += height;
        }
    }
    fn sync_scroll_area(&mut self) {
        let scrollbar_width = self.scroll.scrollbar_width();
        let row_right = self.row_right();
        let height = self.base.height();
        self.scroll.base.set_x(row_right + SCROLLBAR_GAP);
        self.scroll.base.set_y(self.base.y());
        self.scroll.base.set_width(2 * scrollbar_width);
        self.scroll.base.set_height(height);
        self.scroll.base.visible = self.base.visible;
        self.scroll.base.active = self.base.active;
        self.scroll.base.set_hovered(self.base.hovered());
        self.scroll.base.set_focused(self.base.focused());
        self.scroll.content_height = self.content_height();
    }
    fn content_height(&self) -> i32 {
        let mut total = 0;
        for entry in &self.entries {
            total += entry.height();
        }
        total + CONTENT_HEIGHT_PADDING
    }
    fn row_left(&self) -> i32 {
        self.base.x() + self.base.width() / 2 - self.row_width / 2
    }
    fn row_right(&self) -> i32 {
        self.row_left() + self.row_width
    }
    fn row_y(&self, index: usize) -> i32 {
        self.entries
            .get(index)
            .map_or(0, |entry| entry.entry_rectangle().top())
    }
    fn row_bottom(&self, index: usize) -> i32 {
        match self.entries.get(index) {
            Some(entry) => entry.entry_rectangle().top() + entry.height(),
            None => 0,
        }
    }
    fn row_rectangle(&self, index: usize) -> Option<ScreenRectangle> {
        self.entries.get(index).map(|entry| entry.entry_rectangle())
    }
    fn entry_at(&self, mouse_x: f64, mouse_y: f64) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.is_mouse_over(mouse_x, mouse_y))
    }
    fn add_entry(&mut self, entry: E, height: i32) -> usize {
        let x = self.row_left();
        let width = self.row_width;
        let y = self.next_y();
        let mut entry = entry;
        entry.set_entry_rectangle(ScreenRectangle::new(x, y, width, height));
        self.entries.push(entry);
        self.sync_scroll_area();
        self.entries.len() - 1
    }
    fn clear_entries(&mut self) {
        self.entries.clear();
        self.selected = None;
        self.sync_scroll_area();
    }
    fn clear_entries_except(&mut self, index: usize) {
        let kept = if index < self.entries.len() {
            let kept = self.entries.remove(index);
            self.entries.clear();
            Some(kept)
        } else {
            self.entries.clear();
            None
        };
        self.entries.extend(kept);
        let kept_selected = self.selected == Some(index);
        self.selected = self.selected.filter(|_| kept_selected).map(|_| 0);
        self.focused = self
            .focused
            .filter(|&focused| kept_selected && focused == index)
            .map(|_| 0);
        self.hovered = self
            .hovered
            .filter(|&hovered| kept_selected && hovered == index)
            .map(|_| 0);
        self.sync_scroll_area();
    }
    fn add_entry_to_top(&mut self, mut entry: E, height: i32) {
        let scroll_from_bottom = self.max_scroll_amount() - self.scroll.scroll_amount;
        let y = self.next_y();
        entry.set_entry_rectangle(ScreenRectangle::new(
            self.row_left(),
            y,
            self.row_width,
            height,
        ));
        self.entries.insert(0, entry);
        self.reposition_entries();
        self.sync_scroll_area();
        self.set_scroll_amount(self.max_scroll_amount() - scroll_from_bottom);
    }
    fn remove_entry_from_top(&mut self, index: usize) {
        let scroll_from_bottom = self.max_scroll_amount() - self.scroll.scroll_amount;
        self.remove_entry(index);
        self.set_scroll_amount(self.max_scroll_amount() - scroll_from_bottom);
    }
    fn swap_entries(&mut self, first: usize, second: usize) {
        if first == second || first >= self.entries.len() || second >= self.entries.len() {
            return;
        }
        self.entries.swap(first, second);
        let follow = |index: Option<usize>| -> Option<usize> {
            index.map(|index| {
                if index == first {
                    second
                } else if index == second {
                    first
                } else {
                    index
                }
            })
        };
        self.selected = follow(self.selected);
        self.focused = follow(self.focused);
        self.hovered = follow(self.hovered);
        self.reposition_entries();
        self.sync_scroll_area();
        self.scroll_to_entry(second);
    }
    fn sort_entries_by(&mut self, mut compare: impl FnMut(&E, &E) -> std::cmp::Ordering) {
        let length = self.entries.len();
        let mut buckets: Vec<Option<E>> = std::mem::take(&mut self.entries)
            .into_iter()
            .map(Some)
            .collect();
        let mut order: Vec<usize> = (0..length).collect();
        order.sort_by(|&left, &right| {
            compare(
                buckets[left].as_ref().expect("each slot holds a row"),
                buckets[right].as_ref().expect("each slot holds a row"),
            )
        });
        let mut new_index = vec![0usize; length];
        for (position, &from) in order.iter().enumerate() {
            new_index[from] = position;
        }
        self.entries = order
            .into_iter()
            .map(|index| buckets[index].take().expect("each row is placed once"))
            .collect();
        self.selected = self.selected.map(|index| new_index[index]);
        self.focused = self.focused.map(|index| new_index[index]);
        self.hovered = self.hovered.map(|index| new_index[index]);
        self.reposition_entries();
        self.sync_scroll_area();
    }
    fn remove_entries(&mut self, indices: &[usize]) -> usize {
        let length = self.entries.len();
        let mut marked = vec![false; length];
        for &index in indices {
            if index < length && !marked[index] {
                marked[index] = true;
            }
        }
        let mut removed = 0;
        for index in 0..length {
            if marked[index] {
                self.remove_entry(index - removed);
                removed += 1;
            }
        }
        removed
    }
    fn remove_entry(&mut self, index: usize) -> bool {
        if index >= self.entries.len() {
            return false;
        }
        let was_selected = self.selected == Some(index);
        self.entries.remove(index);
        self.selected = match self.selected {
            Some(selected) if selected > index => Some(selected - 1),
            Some(selected) if selected == index => None,
            other => other,
        };
        self.focused = match self.focused {
            Some(focused) if focused > index => Some(focused - 1),
            Some(focused) if focused == index => None,
            other => other,
        };
        self.hovered = None;
        if was_selected {
            self.set_selected_index(self.selected);
        }
        self.reposition_entries();
        self.sync_scroll_area();
        true
    }
    fn max_scroll_amount(&self) -> f32 {
        (self.content_height() - self.base.height()).max(0) as f32
    }
    fn set_scroll_amount(&mut self, scroll_amount: f32) {
        let viewport = self.scroll.height_for_viewport();
        self.scroll.set_scroll_amount(scroll_amount, viewport);
        self.reposition_entries();
    }
    fn scroll_by(&mut self, pixels: i32) {
        let target = self.scroll.scroll_amount + pixels as f32;
        self.set_scroll_amount(target);
    }
    fn scroll(&mut self, delta: f32, _height: i32) -> bool {
        if !self.base.visible {
            return false;
        }
        let rate = self.scroll.scroll_rate();
        let target = self.scroll.scroll_amount - delta * rate as f32;
        self.set_scroll_amount(target);
        true
    }
    fn scroll_to_entry(&mut self, index: usize) {
        let Some(entry_y) = self.row_rectangle(index).map(ScreenRectangle::top) else {
            return;
        };
        let height = self.entries.get(index).map_or(0, |entry| entry.height());
        let top_delta = entry_y - self.base.y() - SEPARATOR_HEIGHT;
        if top_delta < 0 {
            self.scroll_by(top_delta);
        }
        let bottom_delta = self.base_bottom() - entry_y - height - SEPARATOR_HEIGHT;
        if bottom_delta < 0 {
            self.scroll_by(-bottom_delta);
        }
    }
    fn center_scroll_on(&mut self, index: usize) {
        if index >= self.entries.len() {
            return;
        }
        let mut y = 0;
        for (i, candidate) in self.entries.iter().enumerate() {
            if i == index {
                y += candidate.height() / 2;
                break;
            }
            y += candidate.height();
        }
        let target = y as f32 - self.base.height() as f32 / 2.0;
        self.set_scroll_amount(target);
    }
    fn next_entry(&self, direction: ScreenDirection) -> Option<usize> {
        let len = self.entries.len();
        let delta = match direction {
            ScreenDirection::Up => -1,
            ScreenDirection::Down => 1,
            ScreenDirection::Left | ScreenDirection::Right => 0,
        };
        if len == 0 || delta == 0 {
            return None;
        }
        let mut index: isize = match self.selected {
            None if delta > 0 => 0,
            None => len as isize - 1,
            Some(selected) => selected as isize + delta,
        };
        while index >= 0 && (index as usize) < len {
            let candidate = index as usize;
            if self.can_select(candidate) {
                return Some(candidate);
            }
            index += delta;
        }
        None
    }
    fn can_select(&self, _index: usize) -> bool {
        true
    }
    fn set_selected_index(&mut self, index: Option<usize>) {
        self.selected = index.filter(|&i| i < self.entries.len());
        if let Some(selected) = self.selected {
            let entry_y = self.row_y(selected);
            let height = self.entries.get(selected).map_or(0, |entry| entry.height());
            let top_clipped = entry_y + CONTENT_PADDING < self.base.y();
            let bottom_clipped =
                entry_y + CONTENT_PADDING + height - 2 * CONTENT_PADDING > self.base_bottom();
            if top_clipped || bottom_clipped {
                self.scroll_to_entry(selected);
            }
        }
    }
    fn set_focused_entry(&mut self, index: Option<usize>) {
        let index = index.filter(|&i| i < self.entries.len());
        if let Some(old) = self.focused {
            if Some(old) != index {
                if let Some(entry) = self.entries.get_mut(old) {
                    entry.set_entry_focused(false);
                }
            }
        }
        if let Some(new) = index {
            if let Some(entry) = self.entries.get_mut(new) {
                entry.set_entry_focused(true);
            }
        }
        self.focused = index;
        if index.is_some() {
            self.set_selected_index(index);
        } else if self.clear_selection_on_defocus {
            self.set_selected_index(None);
        }
    }
    fn set_widget_focused(&mut self, focused: bool) {
        if focused {
            self.base.set_focused(true);
        } else {
            self.base.set_focused(false);
            self.set_focused_entry(None);
        }
    }
    fn read_only_next_focus_path(&self, event: &FocusNavigationEvent) -> Option<ComponentPath> {
        if self.entries.is_empty() {
            return None;
        }
        if self.is_focused() {
            let direction = match event {
                FocusNavigationEvent::ArrowNavigation { direction } => *direction,
                _ => return None,
            };
            return self.next_entry(direction).map(ComponentPath::leaf);
        }
        let entry = match self.selected {
            Some(selected) => Some(selected),
            None => self.next_entry(vertical_direction_for_initial_focus(event)),
        };
        entry.map(ComponentPath::leaf)
    }
    fn is_focused(&self) -> bool {
        self.focused.is_some()
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.base.is_active() && self.base.are_coordinates_in_rectangle(mouse_x, mouse_y)
    }
    fn is_dragging(&self) -> bool {
        self.dragging
    }
    fn item_count(&self) -> usize {
        self.entries.len()
    }
    fn set_row_width(&mut self, row_width: i32) {
        self.row_width = row_width;
        self.reposition_entries();
        self.sync_scroll_area();
    }
    fn update_size_and_position(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.base.set_x(x);
        self.base.set_y(y);
        self.base.set_width(width);
        self.base.set_height(height);
        self.reposition_entries();
        if let Some(selected) = self.selected {
            self.scroll_to_entry(selected);
        }
        self.sync_scroll_area();
        let amount = self.scroll.scroll_amount;
        self.set_scroll_amount(amount);
    }
}
impl<E: SelectionListEntry> ListCore<E> {
    fn extract_widget_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        let over_list = self.is_mouse_over(mouse_x as f64, mouse_y as f64);
        self.hovered = if over_list {
            self.entry_at(mouse_x as f64, mouse_y as f64)
        } else {
            None
        };
        if self.render_background {
            self.extract_list_background(graphics);
        }
        self.enable_scissor(graphics);
        self.extract_list_items(graphics, mouse_x, mouse_y, partial_tick);
        graphics.disable_scissor();
        if self.render_header {
            self.extract_list_separators(graphics);
        }
        self.scroll.extract_scrollbar(graphics);
    }
    fn extract_list_background(&self, graphics: &mut GuiGraphicsExtractor) {
        self.scroll.extract_list_background_at(
            graphics,
            self.in_world,
            self.base.x(),
            self.base.y(),
            self.base.width(),
            self.base.height(),
            self.scroll.scroll_amount,
        );
    }
    fn extract_list_separators(&self, graphics: &mut GuiGraphicsExtractor) {
        let (header, footer) = if self.in_world {
            (INWORLD_HEADER_SEPARATOR, INWORLD_FOOTER_SEPARATOR)
        } else {
            (HEADER_SEPARATOR, FOOTER_SEPARATOR)
        };
        let x = self.base.x();
        let width = self.base.width();
        let top = self.base.y() - SEPARATOR_HEIGHT;
        let bottom = self.base_bottom();
        let u1 = width as f32 / BACKGROUND_TEXTURE_SIZE as f32;
        let v1 = SEPARATOR_HEIGHT as f32 / SEPARATOR_HEIGHT as f32;
        for (sprite, y) in [(header, top), (footer, bottom)] {
            if width <= 0 {
                continue;
            }
            graphics.blit_region(
                ResourceLocation::from_string(sprite),
                x,
                y,
                width,
                SEPARATOR_HEIGHT,
                0.0,
                0.0,
                u1,
                v1,
                -1,
            );
        }
    }
    fn enable_scissor(&self, graphics: &mut GuiGraphicsExtractor) {
        graphics.enable_scissor(
            mth_clamp(self.base.x(), 0, graphics.gui_width()),
            mth_clamp(self.base.y(), 0, graphics.gui_height()),
            mth_clamp(self.base_right(), 0, graphics.gui_width()),
            mth_clamp(self.base_bottom(), 0, graphics.gui_height()),
        );
    }
    fn extract_list_items(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        let selected = self.selected;
        let hovered = self.hovered;
        let focused = self.is_focused();
        let list_top = self.base.y();
        let list_bottom = self.base_bottom();
        for (index, entry) in self.entries.iter_mut().enumerate() {
            let rect = entry.entry_rectangle();
            let height = entry.height();
            if rect.top() + height >= list_top && rect.top() <= list_bottom {
                let context = EntryRenderContext {
                    x: rect.left(),
                    y: rect.top(),
                    width: rect.width,
                    height,
                    row_index: index,
                    selected: self.selectable && selected == Some(index),
                    focused,
                    hovered: hovered == Some(index),
                    mouse_x,
                    mouse_y,
                    partial_tick,
                };
                if context.selected {
                    Self::extract_selection(graphics, &context, focused);
                }
                entry.extract_content(graphics, &context);
            }
        }
    }
    fn extract_selection(
        graphics: &mut GuiGraphicsExtractor,
        context: &EntryRenderContext,
        focused: bool,
    ) {
        let outline = if focused {
            SELECTION_FOCUSED_OUTLINE
        } else {
            SELECTION_UNFOCUSED_OUTLINE
        };
        graphics.fill(
            context.x,
            context.y,
            context.x + context.width,
            context.y + context.height,
            outline,
        );
        graphics.fill(
            context.x + 1,
            context.y + 1,
            context.x + context.width - 1,
            context.y + context.height - 1,
            SELECTION_INNER_FILL,
        );
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        let viewport = self.scroll.height_for_viewport();
        let scrolling =
            self.scroll
                .scrollbar_clicked_with_button(event.x, event.y, event.button(), viewport);
        let Some(index) = self.entry_at(event.x, event.y) else {
            return scrolling;
        };
        let accepted = match self.entries.get_mut(index) {
            Some(entry) => entry.mouse_clicked_entry(event, double_click),
            None => return scrolling,
        };
        if accepted {
            self.set_focused_entry(Some(index));
            if event.button() == 0 {
                self.dragging = true;
            }
        }
        true
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        if event.button() == 0 {
            self.scroll.scrollbar_released();
        }
        if event.button() == 0 && self.dragging {
            self.dragging = false;
            if let Some(focused) = self.focused {
                if let Some(listener) = self
                    .entries
                    .get_mut(focused)
                    .and_then(|e| e.event_listener())
                {
                    return listener.mouse_released(event);
                }
            }
        }
        false
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        let viewport = self.scroll.height_for_viewport();
        self.scroll.scrollbar_dragged(event.y, dy, viewport);
        if event.button() == 0 && self.dragging {
            if let Some(focused) = self.focused {
                if let Some(listener) = self
                    .entries
                    .get_mut(focused)
                    .and_then(|e| e.event_listener())
                {
                    return listener.mouse_dragged(event, dx, dy);
                }
            }
        }
        false
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if let Some(focused) = self.focused {
            if let Some(listener) = self
                .entries
                .get_mut(focused)
                .and_then(|e| e.event_listener())
            {
                return listener.key_pressed(event);
            }
        }
        false
    }
    fn char_typed(&mut self, event: crate::input::events::CharacterEvent) -> bool {
        if let Some(focused) = self.focused {
            if let Some(listener) = self
                .entries
                .get_mut(focused)
                .and_then(|e| e.event_listener())
            {
                return listener.char_typed(event);
            }
        }
        false
    }
    fn key_released(&mut self, event: KeyEvent) -> bool {
        if let Some(focused) = self.focused {
            if let Some(listener) = self
                .entries
                .get_mut(focused)
                .and_then(|e| e.event_listener())
            {
                return listener.key_released(event);
            }
        }
        false
    }
    fn preedit_updated(&mut self, event: Option<crate::input::events::PreeditEvent>) -> bool {
        if let Some(focused) = self.focused {
            if let Some(listener) = self
                .entries
                .get_mut(focused)
                .and_then(|e| e.event_listener())
            {
                return listener.preedit_updated(event);
            }
        }
        false
    }
    fn border_for_arrow_navigation(&self, opposite: ScreenDirection) -> ScreenRectangle {
        match self
            .focused
            .and_then(|focused| self.entries.get(focused))
            .map(|entry| rectangle_border(entry.entry_rectangle(), opposite))
        {
            Some(border) => border,
            None => self.base.rectangle().get_border(opposite),
        }
    }
    fn narrate_list_element_position(&self, output: &mut NarrationElementOutput, index: usize) {
        if self.entries.len() > 1 {
            output.add(
                NarratedElementType::Position,
                format!(
                    "{} {} {}",
                    crate::translate("narrator.position.list"),
                    index + 1,
                    self.entries.len()
                ),
            );
        }
    }
    fn narration_priority(&self) -> NarrationPriority {
        if self.is_focused() {
            NarrationPriority::Focused
        } else if self.hovered.is_some() {
            NarrationPriority::Hovered
        } else {
            NarrationPriority::None
        }
    }
}
#[derive(Clone, Debug)]
pub struct ObjectSelectionList<E: SelectionListEntry> {
    core: ListCore<E>,
}
impl<E: SelectionListEntry> ObjectSelectionList<E> {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self::with_default_entry_height(x, y, width, height, DEFAULT_ENTRY_HEIGHT)
    }
    pub fn with_default_entry_height(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        default_entry_height: i32,
    ) -> Self {
        Self {
            core: ListCore::new(x, y, width, height, default_entry_height),
        }
    }
    pub fn with_message(mut self, message: Component) -> Self {
        self.core.base.set_message(message);
        self
    }
    pub fn default_entry_height(&self) -> i32 {
        self.core.default_entry_height
    }
    pub fn set_default_entry_height(&mut self, default_entry_height: i32) {
        self.core.default_entry_height = default_entry_height;
        self.core.scroll.scrollbar_settings =
            ScrollbarSettings::default_settings(default_entry_height / 2);
    }
    pub fn base(&self) -> &WidgetBase {
        &self.core.base
    }
    pub fn base_mut(&mut self) -> &mut WidgetBase {
        &mut self.core.base
    }
    pub fn set_render_header(&mut self, render_header: bool) {
        self.core.render_header = render_header;
    }
    pub fn render_header(&self) -> bool {
        self.core.render_header
    }
    pub fn set_render_background(&mut self, render_background: bool) {
        self.core.render_background = render_background;
    }
    pub fn render_background(&self) -> bool {
        self.core.render_background
    }
    pub fn set_in_world(&mut self, in_world: bool) {
        self.core.in_world = in_world;
    }
    pub fn in_world(&self) -> bool {
        self.core.in_world
    }
    pub fn set_center_list_vertically(&mut self, center_list_vertically: bool) {
        self.core.center_list_vertically = center_list_vertically;
    }
    pub fn center_list_vertically(&self) -> bool {
        self.core.center_list_vertically
    }
    pub fn add_entry(&mut self, entry: E) -> usize {
        let height = self.core.default_entry_height;
        self.core.add_entry(entry, height)
    }
    pub fn add_entry_with_height(&mut self, entry: E, height: i32) -> usize {
        self.core.add_entry(entry, height)
    }
    pub fn replace_entries(&mut self, entries: Vec<E>) {
        self.core.clear_entries();
        for entry in entries {
            self.core.add_entry(entry, self.core.default_entry_height);
        }
    }
    pub fn clear_entries(&mut self) {
        self.core.clear_entries();
    }
    pub fn remove_entry(&mut self, index: usize) -> bool {
        self.core.remove_entry(index)
    }
    pub fn add_entry_to_top(&mut self, entry: E) {
        self.core
            .add_entry_to_top(entry, self.core.default_entry_height);
    }
    pub fn add_entry_to_top_with_height(&mut self, entry: E, height: i32) {
        self.core.add_entry_to_top(entry, height);
    }
    pub fn remove_entry_from_top(&mut self, index: usize) {
        self.core.remove_entry_from_top(index);
    }
    pub fn swap_entries(&mut self, first: usize, second: usize) {
        self.core.swap_entries(first, second);
    }
    pub fn sort_entries_by(&mut self, compare: impl FnMut(&E, &E) -> std::cmp::Ordering) {
        self.core.sort_entries_by(compare);
    }
    pub fn clear_entries_except(&mut self, index: usize) {
        self.core.clear_entries_except(index);
    }
    pub fn remove_entries(&mut self, indices: &[usize]) -> usize {
        self.core.remove_entries(indices)
    }
    pub fn update_size(&mut self, width: i32, layout: &HeaderAndFooterLayout) {
        self.update_size_and_position(0, layout.header_height(), width, layout.content_height());
    }
    pub fn entry_count(&self) -> usize {
        self.core.item_count()
    }
    pub fn is_empty(&self) -> bool {
        self.core.entries.is_empty()
    }
    pub fn entries(&self) -> &[E] {
        &self.core.entries
    }
    pub fn entries_mut(&mut self) -> &mut [E] {
        &mut self.core.entries
    }
    pub fn entry(&self, index: usize) -> Option<&E> {
        self.core.entries.get(index)
    }
    pub fn entry_mut(&mut self, index: usize) -> Option<&mut E> {
        self.core.entries.get_mut(index)
    }
    pub fn next_y(&self) -> i32 {
        self.core.next_y()
    }
    pub fn selected_index(&self) -> Option<usize> {
        self.core.selected
    }
    pub fn set_selected(&mut self, index: usize) {
        self.core.set_selected_index(Some(index));
    }
    pub fn clear_selection(&mut self) {
        self.core.set_selected_index(None);
    }
    pub fn focused_index(&self) -> Option<usize> {
        self.core.focused
    }
    pub fn set_focused_entry(&mut self, index: usize) {
        self.core.set_focused_entry(Some(index));
    }
    pub fn clear_focused_entry(&mut self) {
        self.core.set_focused_entry(None);
    }
    pub fn hovered_index(&self) -> Option<usize> {
        self.core.hovered
    }
    #[doc(alias = "nextFocusPath")]
    pub fn release_focus(&mut self) {
        self.core.set_focused_entry(None);
        self.core.set_selected_index(None);
    }
    pub fn take_next_focus_path(&mut self, event: &FocusNavigationEvent) -> Option<ComponentPath> {
        if self.core.item_count() == 0 {
            return None;
        }
        if self.core.is_focused() {
            let direction = match event {
                FocusNavigationEvent::ArrowNavigation { direction } => *direction,
                _ => return None,
            };
            return match self.core.next_entry(direction) {
                Some(index) => Some(ComponentPath::leaf(index)),
                None => {
                    self.release_focus();
                    None
                }
            };
        }
        let entry = match self.core.selected {
            Some(selected) => Some(selected),
            None => self
                .core
                .next_entry(vertical_direction_for_initial_focus(event)),
        };
        entry.map(ComponentPath::leaf)
    }
    pub fn select_neighbour(&mut self, direction: ScreenDirection) -> bool {
        if !self.core.base.is_active() {
            return false;
        }
        match self.core.next_entry(direction) {
            Some(index) => {
                self.core.set_focused_entry(Some(index));
                true
            }
            None => false,
        }
    }
    pub fn row_width(&self) -> i32 {
        self.core.row_width
    }
    pub fn set_row_width(&mut self, row_width: i32) {
        self.core.set_row_width(row_width);
    }
    pub fn row_left(&self) -> i32 {
        self.core.row_left()
    }
    pub fn row_right(&self) -> i32 {
        self.core.row_right()
    }
    pub fn row_y(&self, index: usize) -> i32 {
        self.core.row_y(index)
    }
    pub fn row_bottom(&self, index: usize) -> i32 {
        self.core.row_bottom(index)
    }
    pub fn row_rectangle(&self, index: usize) -> Option<ScreenRectangle> {
        self.core.row_rectangle(index)
    }
    pub fn entry_at(&self, mouse_x: f64, mouse_y: f64) -> Option<usize> {
        self.core.entry_at(mouse_x, mouse_y)
    }
    pub fn content_height(&self) -> i32 {
        self.core.content_height()
    }
    pub fn update_size_and_position(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.core.update_size_and_position(x, y, width, height);
    }
    pub fn scroll_y(&self) -> f32 {
        self.core.scroll.scroll_amount
    }
    pub fn set_scroll_y(&mut self, scroll_amount: f32) {
        self.core.set_scroll_amount(scroll_amount);
    }
    pub fn max_scroll_amount(&self) -> f32 {
        self.core.max_scroll_amount()
    }
    pub fn scroll(&mut self, delta: f32, height: i32) -> bool {
        self.core.scroll(delta, height)
    }
    pub fn scroll_by(&mut self, pixels: i32) {
        self.core.scroll_by(pixels);
    }
    pub fn scroll_entry_into_view(&mut self, index: usize) {
        self.core.scroll_to_entry(index);
    }
    pub fn center_scroll_on(&mut self, index: usize) {
        self.core.center_scroll_on(index);
    }
    pub fn scrollbar_x(&self) -> i32 {
        self.core.scroll.scrollbar_x()
    }
    pub fn scrollbar_y(&self) -> i32 {
        self.core.scroll.scrollbar_y(self.viewport_height())
    }
    pub fn scrollbar_height(&self) -> i32 {
        self.core.scroll.scrollbar_height(self.viewport_height())
    }
    pub fn has_scrollbar(&self) -> bool {
        self.core.max_scroll_amount() > 0.0
    }
    pub fn is_dragging(&self) -> bool {
        self.core.is_dragging()
    }
    pub fn scroll_area(&self) -> &AbstractScrollArea {
        &self.core.scroll
    }
    pub fn scroll_area_mut(&mut self) -> &mut AbstractScrollArea {
        &mut self.core.scroll
    }
    fn viewport_height(&self) -> i32 {
        self.core.base.height()
    }
}
impl<E: SelectionListEntry> TabOrderedElement for ObjectSelectionList<E> {
    fn tab_order_group(&self) -> i32 {
        self.core.base.tab_order_group()
    }
}
impl<E: SelectionListEntry> LayoutElement for ObjectSelectionList<E> {
    fn set_x(&mut self, x: i32) {
        let y = self.core.base.y();
        let width = self.core.base.width();
        let height = self.core.base.height();
        self.core.update_size_and_position(x, y, width, height);
    }
    fn set_y(&mut self, y: i32) {
        let x = self.core.base.x();
        let width = self.core.base.width();
        let height = self.core.base.height();
        self.core.update_size_and_position(x, y, width, height);
    }
    fn x(&self) -> i32 {
        self.core.base.x()
    }
    fn y(&self) -> i32 {
        self.core.base.y()
    }
    fn width(&self) -> i32 {
        self.core.base.width()
    }
    fn height(&self) -> i32 {
        self.core.base.height()
    }
}
impl<E: SelectionListEntry> GuiEventListener for ObjectSelectionList<E> {
    fn mouse_scrolled(&mut self, _x: f64, _y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        let _ = scroll_x;
        self.core.scroll(scroll_y as f32, self.core.base.height())
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.core.mouse_clicked(event, double_click)
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        self.core.mouse_released(event)
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        self.core.mouse_dragged(event, dx, dy)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if !self.core.base.is_active() {
            return false;
        }
        match event.key {
            KeyEvent::KEY_UP => self.select_neighbour(ScreenDirection::Up),
            KeyEvent::KEY_DOWN => self.select_neighbour(ScreenDirection::Down),
            _ => self.core.key_pressed(event),
        }
    }
    fn key_released(&mut self, event: KeyEvent) -> bool {
        self.core.key_released(event)
    }
    fn char_typed(&mut self, event: crate::input::events::CharacterEvent) -> bool {
        self.core.char_typed(event)
    }
    fn preedit_updated(&mut self, event: Option<crate::input::events::PreeditEvent>) -> bool {
        self.core.preedit_updated(event)
    }
    fn next_focus_path(&self, navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.core.read_only_next_focus_path(navigation_event)
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.core.is_mouse_over(mouse_x, mouse_y)
    }
    fn set_focused(&mut self, focused: bool) {
        self.core.set_widget_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.core.is_focused()
    }
    fn should_take_focus_after_interaction(&self) -> bool {
        true
    }
    fn rectangle(&self) -> ScreenRectangle {
        self.core.base.rectangle()
    }
    fn border_for_arrow_navigation(&self, opposite: ScreenDirection) -> ScreenRectangle {
        self.core.border_for_arrow_navigation(opposite)
    }
    fn play_down_sound(&self) -> Option<crate::components::WidgetClickSound> {
        None
    }
}
impl<E: SelectionListEntry> NarratableEntry for ObjectSelectionList<E> {
    fn narration_priority(&self) -> NarrationPriority {
        self.core.narration_priority()
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        let target = self.core.hovered.or(self.core.selected);
        if let Some(index) = target {
            self.core.narrate_list_element_position(output, index);
            if let Some(entry) = self.core.entries.get(index) {
                entry.narrate_entry(output);
            }
        }
        if self.core.is_focused() {
            output.add(
                NarratedElementType::Usage,
                crate::translate("narration.selection.usage"),
            );
        }
    }
}
impl<E: SelectionListEntry> Renderable for ObjectSelectionList<E> {
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        if !self.core.base.visible {
            return;
        }
        self.core.base.set_hovered(
            graphics.contains_point_in_scissor(mouse_x, mouse_y)
                && self
                    .core
                    .base
                    .are_coordinates_in_rectangle(mouse_x as f64, mouse_y as f64),
        );
        self.core
            .extract_widget_render_state(graphics, mouse_x, mouse_y, partial_tick);
    }
}
pub trait ContainerObjectSelectionListEntry: SelectionListEntry + GuiEventListener {
    fn children(&self) -> &[Box<dyn GuiEventListener>];
    fn has_children(&self) -> bool {
        !self.children().is_empty()
    }
    fn narratables(&self) -> Vec<&dyn NarratableEntry>;
    fn focus_path_at_index(
        &self,
        navigation_event: &FocusNavigationEvent,
        current_index: usize,
    ) -> Option<ComponentPath> {
        let children = self.children();
        if children.is_empty() {
            return None;
        }
        let clamped = current_index.min(children.len() - 1);
        let child = children.get(clamped)?;
        child.next_focus_path(navigation_event)
    }
    fn container_next_focus_path(
        &self,
        navigation_event: &FocusNavigationEvent,
        fallback: impl FnOnce(&Self) -> Option<ComponentPath>,
    ) -> Option<ComponentPath> {
        if let FocusNavigationEvent::ArrowNavigation { direction } = navigation_event {
            let delta = match direction {
                ScreenDirection::Left => -1,
                ScreenDirection::Right => 1,
                ScreenDirection::Up | ScreenDirection::Down => 0,
            };
            if delta != 0 {
                let children = self.children();
                if !children.is_empty() {
                    let focused_index = self.focused_child_index().map_or(-1, |i| i as isize);
                    let clamped = (focused_index + delta).clamp(0, children.len() as isize - 1);
                    let mut index = clamped;
                    while index >= 0 && (index as usize) < children.len() {
                        if let Some(child) = children.get(index as usize) {
                            if let Some(child_path) = child.next_focus_path(navigation_event) {
                                return Some(child_path);
                            }
                        }
                        index += delta;
                    }
                }
            }
        }
        fallback(self)
    }
    fn container_default_next_focus_path(
        &self,
        event: &FocusNavigationEvent,
    ) -> Option<ComponentPath> {
        if let Some(index) = self.focused_child_index() {
            if let Some(child) = self.children().get(index) {
                if let Some(path) = child.next_focus_path(event) {
                    return Some(path);
                }
            }
        }
        let children = self.children();
        match event {
            FocusNavigationEvent::TabNavigation { forward } => {
                let winner = resolve_tab(children, self.focused_child_index(), *forward)?;
                children.get(winner)?.next_focus_path(event)
            }
            FocusNavigationEvent::ArrowNavigation { direction } => {
                let winner = match self.focused_child_index() {
                    Some(index) => resolve_arrow(children, index, *direction, None),
                    None => resolve_arrow_from(
                        children,
                        self.entry_rectangle().get_border(direction.opposite()),
                        *direction,
                    ),
                };
                children.get(winner?)?.next_focus_path(event)
            }
            FocusNavigationEvent::InitialFocus => None,
        }
    }
    fn focused_child_index(&self) -> Option<usize> {
        self.children().iter().position(|child| child.is_focused())
    }
    fn narrate_entry_children(&self, output: &mut NarrationElementOutput) {
        let narratables = self.narratables();
        let Some((index, _)) = self.find_narration_index(&narratables) else {
            return;
        };
        let entry = match narratables.get(index) {
            Some(entry) => *entry,
            None => return,
        };
        if narratables.len() > 1 {
            output.add(
                NarratedElementType::Position,
                format!(
                    "{} {} {}",
                    crate::translate("narrator.position.object_list"),
                    index + 1,
                    narratables.len()
                ),
            );
        }
        entry.update_narration(output);
    }
    fn find_narration_index(
        &self,
        narratables: &[&dyn NarratableEntry],
    ) -> Option<(usize, NarrationPriority)> {
        let mut best: Option<(usize, NarrationPriority)> = None;
        for (index, entry) in narratables.iter().enumerate() {
            let priority = entry.narration_priority();
            if priority == NarrationPriority::Focused {
                return Some((index, priority));
            } else if best.map_or(true, |(_, best)| priority > best) {
                best = Some((index, priority));
            }
        }
        best
    }
}
fn row_path(this_row: usize, child: Option<ComponentPath>) -> Option<ComponentPath> {
    child.map(|child| ComponentPath::Path {
        component: this_row,
        child: Box::new(child),
    })
}
pub fn path_row_index(path: &ComponentPath) -> Option<usize> {
    match path {
        ComponentPath::Leaf(row) => Some(*row),
        ComponentPath::Path { component, child } => {
            Some(*component).or_else(|| path_row_index(child))
        }
    }
}
#[derive(Clone, Debug)]
pub struct ContainerObjectSelectionList<E: ContainerObjectSelectionListEntry> {
    core: ListCore<E>,
}
impl<E: ContainerObjectSelectionListEntry> ContainerObjectSelectionList<E> {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self::with_default_entry_height(x, y, width, height, DEFAULT_ENTRY_HEIGHT)
    }
    pub fn with_default_entry_height(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        default_entry_height: i32,
    ) -> Self {
        let mut list = Self {
            core: ListCore::new(x, y, width, height, default_entry_height),
        };
        list.core.selectable = false;
        list.core.clear_selection_on_defocus = true;
        list
    }
    pub fn base(&self) -> &WidgetBase {
        &self.core.base
    }
    pub fn base_mut(&mut self) -> &mut WidgetBase {
        &mut self.core.base
    }
    pub fn set_render_header(&mut self, render_header: bool) {
        self.core.render_header = render_header;
    }
    pub fn render_header(&self) -> bool {
        self.core.render_header
    }
    pub fn set_render_background(&mut self, render_background: bool) {
        self.core.render_background = render_background;
    }
    pub fn render_background(&self) -> bool {
        self.core.render_background
    }
    pub fn set_in_world(&mut self, in_world: bool) {
        self.core.in_world = in_world;
    }
    pub fn in_world(&self) -> bool {
        self.core.in_world
    }
    pub fn add_entry(&mut self, entry: E) -> usize {
        let height = self.core.default_entry_height;
        self.core.add_entry(entry, height)
    }
    pub fn add_entry_with_height(&mut self, entry: E, height: i32) -> usize {
        self.core.add_entry(entry, height)
    }
    pub fn replace_entries(&mut self, entries: Vec<E>) {
        self.core.clear_entries();
        for entry in entries {
            self.core.add_entry(entry, self.core.default_entry_height);
        }
    }
    pub fn clear_entries(&mut self) {
        self.core.clear_entries();
    }
    pub fn remove_entry(&mut self, index: usize) -> bool {
        self.core.remove_entry(index)
    }
    pub fn add_entry_to_top(&mut self, entry: E) {
        self.core
            .add_entry_to_top(entry, self.core.default_entry_height);
    }
    pub fn add_entry_to_top_with_height(&mut self, entry: E, height: i32) {
        self.core.add_entry_to_top(entry, height);
    }
    pub fn remove_entry_from_top(&mut self, index: usize) {
        self.core.remove_entry_from_top(index);
    }
    pub fn swap_entries(&mut self, first: usize, second: usize) {
        self.core.swap_entries(first, second);
    }
    pub fn sort_entries_by(&mut self, compare: impl FnMut(&E, &E) -> std::cmp::Ordering) {
        self.core.sort_entries_by(compare);
    }
    pub fn clear_entries_except(&mut self, index: usize) {
        self.core.clear_entries_except(index);
    }
    pub fn remove_entries(&mut self, indices: &[usize]) -> usize {
        self.core.remove_entries(indices)
    }
    pub fn update_size(&mut self, width: i32, layout: &HeaderAndFooterLayout) {
        self.update_size_and_position(0, layout.header_height(), width, layout.content_height());
    }
    pub fn entry_count(&self) -> usize {
        self.core.item_count()
    }
    pub fn is_empty(&self) -> bool {
        self.core.entries.is_empty()
    }
    pub fn entries(&self) -> &[E] {
        &self.core.entries
    }
    pub fn entries_mut(&mut self) -> &mut [E] {
        &mut self.core.entries
    }
    pub fn entry(&self, index: usize) -> Option<&E> {
        self.core.entries.get(index)
    }
    pub fn entry_mut(&mut self, index: usize) -> Option<&mut E> {
        self.core.entries.get_mut(index)
    }
    pub fn next_y(&self) -> i32 {
        self.core.next_y()
    }
    pub fn selected_index(&self) -> Option<usize> {
        self.core.selected
    }
    pub fn set_selected(&mut self, index: usize) {
        self.core.set_selected_index(Some(index));
    }
    pub fn clear_selection(&mut self) {
        self.core.set_selected_index(None);
    }
    pub fn focused_index(&self) -> Option<usize> {
        self.core.focused
    }
    pub fn set_focused_entry(&mut self, index: usize) {
        self.core.set_focused_entry(Some(index));
    }
    pub fn clear_focused_entry(&mut self) {
        self.core.set_focused_entry(None);
    }
    pub fn hovered_index(&self) -> Option<usize> {
        self.core.hovered
    }
    pub fn release_focus(&mut self) {
        self.core.set_focused_entry(None);
        self.core.set_selected_index(None);
    }
    pub fn take_next_focus_path(&mut self, event: &FocusNavigationEvent) -> Option<ComponentPath> {
        if self.core.item_count() == 0 {
            return None;
        }
        let FocusNavigationEvent::ArrowNavigation { direction } = event else {
            return self.super_next_focus_path(event);
        };
        if let Some(focused) = self.core.focused {
            if direction.axis() == crate::framework::navigation::ScreenAxis::Horizontal {
                if let Some(entry) = self.core.entries.get_mut(focused) {
                    return row_path(focused, entry.next_focus_path(event));
                }
                return None;
            }
        }
        let mut index: isize = -1;
        let mut direction = *direction;
        if let Some(focused) = self.core.focused {
            index = self.core.entries.get(focused).map_or(-1, |entry| {
                entry.focused_child_index().map_or(-1, |i| i as isize)
            });
        }
        if index == -1 {
            match direction {
                ScreenDirection::Left => {
                    index = isize::MAX;
                    direction = ScreenDirection::Down;
                }
                ScreenDirection::Right => {
                    index = 0;
                    direction = ScreenDirection::Down;
                }
                _ => index = 0,
            }
        }
        let mut entry_index = self.core.focused;
        loop {
            let next = self.container_next_entry(direction, entry_index);
            let Some(next) = next else {
                return None;
            };
            entry_index = Some(next);
            let clamped = index.max(0).min(isize::MAX) as usize;
            let path = self
                .core
                .entries
                .get(next)
                .and_then(|entry| entry.focus_path_at_index(event, clamped));
            if let Some(path) = path {
                return row_path(next, Some(path));
            }
        }
    }
    fn container_next_entry(
        &self,
        direction: ScreenDirection,
        start: Option<usize>,
    ) -> Option<usize> {
        let len = self.core.entries.len();
        let delta = match direction {
            ScreenDirection::Up => -1,
            ScreenDirection::Down => 1,
            ScreenDirection::Left | ScreenDirection::Right => 0,
        };
        if len == 0 || delta == 0 {
            return None;
        }
        let mut index: isize = match start {
            None if delta > 0 => 0,
            None => len as isize - 1,
            Some(start) => start as isize + delta,
        };
        while index >= 0 && (index as usize) < len {
            let candidate = index as usize;
            if self
                .core
                .entries
                .get(candidate)
                .is_some_and(|entry| !entry.children().is_empty())
            {
                return Some(candidate);
            }
            index += delta;
        }
        None
    }
    fn super_next_focus_path(&self, event: &FocusNavigationEvent) -> Option<ComponentPath> {
        if let Some(focused) = self.core.focused {
            if let Some(entry) = self.core.entries.get(focused) {
                if let Some(path) = entry.next_focus_path(event) {
                    return row_path(focused, Some(path));
                }
            }
        }
        let FocusNavigationEvent::TabNavigation { forward } = event else {
            return None;
        };
        let row_count = self.core.entries.len();
        let mut sorted: Vec<usize> = (0..row_count).collect();
        sorted.sort_by_key(|&index| self.core.entries[index].tab_order_group());
        let position = self
            .core
            .focused
            .and_then(|focused| sorted.iter().position(|&candidate| candidate == focused));
        let start = match position {
            Some(position) => position + usize::from(*forward),
            None if *forward => 0,
            None => sorted.len(),
        };
        let tab = FocusNavigationEvent::TabNavigation { forward: *forward };
        let candidates: Box<dyn Iterator<Item = usize>> = if *forward {
            Box::new(sorted.into_iter().skip(start))
        } else {
            Box::new(sorted.into_iter().take(start).rev())
        };
        for index in candidates {
            let Some(entry) = self.core.entries.get(index) else {
                continue;
            };
            if let Some(path) = entry.next_focus_path(&tab) {
                return row_path(index, Some(path));
            }
        }
        None
    }
    pub fn row_width(&self) -> i32 {
        self.core.row_width
    }
    pub fn set_row_width(&mut self, row_width: i32) {
        self.core.set_row_width(row_width);
    }
    pub fn row_left(&self) -> i32 {
        self.core.row_left()
    }
    pub fn row_right(&self) -> i32 {
        self.core.row_right()
    }
    pub fn row_y(&self, index: usize) -> i32 {
        self.core.row_y(index)
    }
    pub fn row_bottom(&self, index: usize) -> i32 {
        self.core.row_bottom(index)
    }
    pub fn row_rectangle(&self, index: usize) -> Option<ScreenRectangle> {
        self.core.row_rectangle(index)
    }
    pub fn entry_at(&self, mouse_x: f64, mouse_y: f64) -> Option<usize> {
        self.core.entry_at(mouse_x, mouse_y)
    }
    pub fn content_height(&self) -> i32 {
        self.core.content_height()
    }
    pub fn update_size_and_position(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.core.update_size_and_position(x, y, width, height);
    }
    pub fn scroll_y(&self) -> f32 {
        self.core.scroll.scroll_amount
    }
    pub fn set_scroll_y(&mut self, scroll_amount: f32) {
        self.core.set_scroll_amount(scroll_amount);
    }
    pub fn max_scroll_amount(&self) -> f32 {
        self.core.max_scroll_amount()
    }
    pub fn scroll(&mut self, delta: f32, height: i32) -> bool {
        self.core.scroll(delta, height)
    }
    pub fn scroll_by(&mut self, pixels: i32) {
        self.core.scroll_by(pixels);
    }
    pub fn scroll_entry_into_view(&mut self, index: usize) {
        self.core.scroll_to_entry(index);
    }
    pub fn center_scroll_on(&mut self, index: usize) {
        self.core.center_scroll_on(index);
    }
    pub fn scrollbar_x(&self) -> i32 {
        self.core.scroll.scrollbar_x()
    }
    pub fn scrollbar_y(&self) -> i32 {
        self.core.scroll.scrollbar_y(self.core.base.height())
    }
    pub fn scrollbar_height(&self) -> i32 {
        self.core.scroll.scrollbar_height(self.core.base.height())
    }
    pub fn has_scrollbar(&self) -> bool {
        self.core.max_scroll_amount() > 0.0
    }
    pub fn is_dragging(&self) -> bool {
        self.core.is_dragging()
    }
    pub fn scroll_area(&self) -> &AbstractScrollArea {
        &self.core.scroll
    }
    pub fn scroll_area_mut(&mut self) -> &mut AbstractScrollArea {
        &mut self.core.scroll
    }
}
impl<E: ContainerObjectSelectionListEntry> TabOrderedElement for ContainerObjectSelectionList<E> {
    fn tab_order_group(&self) -> i32 {
        self.core.base.tab_order_group()
    }
}
impl<E: ContainerObjectSelectionListEntry> LayoutElement for ContainerObjectSelectionList<E> {
    fn set_x(&mut self, x: i32) {
        let y = self.core.base.y();
        let width = self.core.base.width();
        let height = self.core.base.height();
        self.core.update_size_and_position(x, y, width, height);
    }
    fn set_y(&mut self, y: i32) {
        let x = self.core.base.x();
        let width = self.core.base.width();
        let height = self.core.base.height();
        self.core.update_size_and_position(x, y, width, height);
    }
    fn x(&self) -> i32 {
        self.core.base.x()
    }
    fn y(&self) -> i32 {
        self.core.base.y()
    }
    fn width(&self) -> i32 {
        self.core.base.width()
    }
    fn height(&self) -> i32 {
        self.core.base.height()
    }
}
impl<E: ContainerObjectSelectionListEntry> GuiEventListener for ContainerObjectSelectionList<E> {
    fn mouse_scrolled(&mut self, _x: f64, _y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        let _ = scroll_x;
        self.core.scroll(scroll_y as f32, self.core.base.height())
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.core.mouse_clicked(event, double_click)
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        self.core.mouse_released(event)
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        self.core.mouse_dragged(event, dx, dy)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if !self.core.base.is_active() {
            return false;
        }
        match event.key {
            KeyEvent::KEY_UP => self.select_neighbour(ScreenDirection::Up),
            KeyEvent::KEY_DOWN => self.select_neighbour(ScreenDirection::Down),
            _ => self.core.key_pressed(event),
        }
    }
    fn key_released(&mut self, event: KeyEvent) -> bool {
        self.core.key_released(event)
    }
    fn char_typed(&mut self, event: crate::input::events::CharacterEvent) -> bool {
        self.core.char_typed(event)
    }
    fn preedit_updated(&mut self, event: Option<crate::input::events::PreeditEvent>) -> bool {
        self.core.preedit_updated(event)
    }
    fn next_focus_path(&self, navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        if self.core.item_count() == 0 {
            return None;
        }
        let FocusNavigationEvent::ArrowNavigation { direction } = navigation_event else {
            return self.super_next_focus_path(navigation_event);
        };
        if let Some(focused) = self.core.focused {
            if direction.axis() == crate::framework::navigation::ScreenAxis::Horizontal {
                if let Some(entry) = self.core.entries.get(focused) {
                    return row_path(focused, entry.next_focus_path(navigation_event));
                }
                return None;
            }
        }
        let mut index: isize = -1;
        let mut direction = *direction;
        if let Some(focused) = self.core.focused {
            index = self.core.entries.get(focused).map_or(-1, |entry| {
                entry.focused_child_index().map_or(-1, |i| i as isize)
            });
        }
        if index == -1 {
            match direction {
                ScreenDirection::Left => {
                    index = isize::MAX;
                    direction = ScreenDirection::Down;
                }
                ScreenDirection::Right => {
                    index = 0;
                    direction = ScreenDirection::Down;
                }
                _ => index = 0,
            }
        }
        let clamped = index.max(0).min(isize::MAX) as usize;
        let mut start = self.core.focused;
        loop {
            let Some(next) = self.container_next_entry(direction, start) else {
                return None;
            };
            start = Some(next);
            if let Some(path) = self
                .core
                .entries
                .get(next)
                .and_then(|entry| entry.focus_path_at_index(navigation_event, clamped))
            {
                return row_path(next, Some(path));
            }
        }
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.core.is_mouse_over(mouse_x, mouse_y)
    }
    fn set_focused(&mut self, focused: bool) {
        self.core.set_widget_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.core.is_focused()
    }
    fn should_take_focus_after_interaction(&self) -> bool {
        true
    }
    fn rectangle(&self) -> ScreenRectangle {
        self.core.base.rectangle()
    }
    fn border_for_arrow_navigation(&self, opposite: ScreenDirection) -> ScreenRectangle {
        self.core.border_for_arrow_navigation(opposite)
    }
    fn play_down_sound(&self) -> Option<crate::components::WidgetClickSound> {
        None
    }
}
impl<E: ContainerObjectSelectionListEntry> NarratableEntry for ContainerObjectSelectionList<E> {
    fn narration_priority(&self) -> NarrationPriority {
        self.core.narration_priority()
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        let target = self.core.hovered.or(self.core.focused);
        if let Some(index) = target {
            if let Some(entry) = self.core.entries.get(index) {
                entry.narrate_entry_children(output);
            }
            self.core.narrate_list_element_position(output, index);
        }
    }
}
impl<E: ContainerObjectSelectionListEntry> Renderable for ContainerObjectSelectionList<E> {
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        if !self.core.base.visible {
            return;
        }
        self.core.base.set_hovered(
            graphics.contains_point_in_scissor(mouse_x, mouse_y)
                && self
                    .core
                    .base
                    .are_coordinates_in_rectangle(mouse_x as f64, mouse_y as f64),
        );
        self.core
            .extract_widget_render_state(graphics, mouse_x, mouse_y, partial_tick);
    }
}
impl<E: ContainerObjectSelectionListEntry> ContainerObjectSelectionList<E> {
    fn select_neighbour(&mut self, direction: ScreenDirection) -> bool {
        if !self.core.base.is_active() {
            return false;
        }
        match self.core.next_entry(direction) {
            Some(index) => {
                self.core.set_focused_entry(Some(index));
                true
            }
            None => false,
        }
    }
}
