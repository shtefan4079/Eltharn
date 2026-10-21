use crate::framework::navigation::ScreenRectangle;
pub trait LayoutElement {
    fn set_x(&mut self, x: i32);
    fn set_y(&mut self, y: i32);
    fn x(&self) -> i32;
    fn y(&self) -> i32;
    fn width(&self) -> i32;
    fn height(&self) -> i32;
    fn set_position(&mut self, x: i32, y: i32) {
        self.set_x(x);
        self.set_y(y);
    }
    fn rectangle(&self) -> ScreenRectangle {
        ScreenRectangle::new(self.x(), self.y(), self.width(), self.height())
    }
    fn arrange(&mut self) {}
}
pub trait Layout: LayoutElement {
    fn arrange_elements(&mut self);
    fn remove_children(&mut self);
}
fn lerp_f32(alpha: f32, p0: f32, p1: f32) -> f32 {
    p0 + alpha * (p1 - p0)
}
fn round_f32(value: f32) -> f32 {
    value.round()
}
fn positive_ceil_div(input: i32, divisor: i32) -> i32 {
    if divisor == 0 {
        return input;
    }
    input.wrapping_neg().div_euclid(divisor).saturating_mul(-1)
}
fn round_toward(input: i32, multiple: i32) -> i32 {
    if multiple == 0 {
        return input;
    }
    positive_ceil_div(input, multiple).saturating_mul(multiple)
}
#[derive(Debug)]
struct Divisor {
    denominator: i32,
    quotient: i32,
    remainder: i32,
    rest: i32,
    returned: i32,
}
impl Divisor {
    fn new(numerator: i32, denominator: i32) -> Self {
        if denominator > 0 {
            Self {
                denominator,
                quotient: numerator / denominator,
                remainder: 0,
                rest: numerator % denominator,
                returned: 0,
            }
        } else {
            Self {
                denominator,
                quotient: 0,
                remainder: 0,
                rest: 0,
                returned: 0,
            }
        }
    }
    fn has_next(&self) -> bool {
        self.returned < self.denominator
    }
    fn next_int(&mut self) -> Option<i32> {
        if !self.has_next() {
            return None;
        }
        let mut value = self.quotient;
        self.remainder += self.rest;
        if self.remainder >= self.denominator {
            self.remainder -= self.denominator;
            value += 1;
        }
        self.returned += 1;
        Some(value)
    }
    fn parts(numerator: i32, denominator: i32) -> Vec<i32> {
        let mut divisor = Self::new(numerator, denominator);
        let mut parts = Vec::new();
        while let Some(value) = divisor.next_int() {
            parts.push(value);
        }
        parts
    }
}
enum Store<'a> {
    Detached,
    Slice(&'a mut [Box<dyn LayoutElement>]),
    Owned(Vec<Box<dyn LayoutElement + 'a>>),
}
impl<'a> Store<'a> {
    fn widget_mut<'store>(
        &'store mut self,
        handle: usize,
    ) -> Option<&'store mut (dyn LayoutElement + 'store)> {
        match self {
            Self::Detached => None,
            Self::Slice(widgets) => widgets
                .get_mut(handle)
                .map(|widget| widget.as_mut() as &mut (dyn LayoutElement + 'store)),
            Self::Owned(widgets) => widgets
                .get_mut(handle)
                .map(|widget| widget.as_mut() as &mut (dyn LayoutElement + 'store)),
        }
    }
}
struct Borrowed<'a> {
    widget: &'a mut dyn LayoutElement,
}
impl LayoutElement for Borrowed<'_> {
    fn set_x(&mut self, x: i32) {
        self.widget.set_x(x);
    }
    fn set_y(&mut self, y: i32) {
        self.widget.set_y(y);
    }
    fn x(&self) -> i32 {
        self.widget.x()
    }
    fn y(&self) -> i32 {
        self.widget.y()
    }
    fn width(&self) -> i32 {
        self.widget.width()
    }
    fn height(&self) -> i32 {
        self.widget.height()
    }
    fn arrange(&mut self) {
        self.widget.arrange();
    }
}
fn store_from_pairs<'a>(
    widgets: &mut dyn Iterator<Item = (usize, &'a mut dyn LayoutElement)>,
) -> Vec<Box<dyn LayoutElement + 'a>> {
    let mut pairs: Vec<(usize, &'a mut dyn LayoutElement)> = widgets.collect();
    let highest = match pairs.iter().map(|(handle, _)| *handle).max() {
        Some(highest) => highest,
        None => return Vec::new(),
    };
    let mut store: Vec<Box<dyn LayoutElement>> = Vec::with_capacity(highest + 1);
    while store.len() <= highest {
        let position = pairs.iter().position(|(handle, _)| *handle == store.len());
        match position {
            Some(position) => {
                let (_, widget) = pairs.swap_remove(position);
                store.push(Box::new(Borrowed { widget }));
            }
            None => store.push(Box::new(SpacerElement::new(0, 0))),
        }
    }
    store
}
enum ChildSlot {
    Owned(Box<dyn LayoutElement>),
    Handle(usize, i32, i32),
}
struct Child {
    slot: ChildSlot,
    settings: LayoutSettings,
}
impl Child {
    fn owned(element: Box<dyn LayoutElement>, settings: LayoutSettings) -> Self {
        Self {
            slot: ChildSlot::Owned(element),
            settings,
        }
    }
    fn new_handle(handle: usize, width: i32, height: i32, settings: LayoutSettings) -> Self {
        Self {
            slot: ChildSlot::Handle(handle, width, height),
            settings,
        }
    }
    fn handle_index(&self) -> Option<usize> {
        match self.slot {
            ChildSlot::Handle(handle, _, _) => Some(handle),
            ChildSlot::Owned(_) => None,
        }
    }
    fn element(&self) -> Option<&dyn LayoutElement> {
        match &self.slot {
            ChildSlot::Owned(element) => Some(element.as_ref()),
            ChildSlot::Handle(_, _, _) => None,
        }
    }
    fn outer_width(&self) -> i32 {
        self.inner_width() + self.settings.padding_left + self.settings.padding_right
    }
    fn outer_height(&self) -> i32 {
        self.inner_height() + self.settings.padding_top + self.settings.padding_bottom
    }
    fn inner_width(&self) -> i32 {
        match &self.slot {
            ChildSlot::Owned(element) => element.width(),
            ChildSlot::Handle(_, width, _) => *width,
        }
    }
    fn inner_height(&self) -> i32 {
        match &self.slot {
            ChildSlot::Owned(element) => element.height(),
            ChildSlot::Handle(_, _, height) => *height,
        }
    }
    fn set_x(&mut self, x: i32, available: i32, store: &mut Store<'_>) {
        let least = self.settings.padding_left as f32;
        let most = (available - self.inner_width() - self.settings.padding_right) as f32;
        let offset = lerp_f32(self.settings.x_alignment, least, most) as i32;
        self.write(|target| target.set_x(x + offset), store);
    }
    fn set_y(&mut self, y: i32, available: i32, store: &mut Store<'_>) {
        let least = self.settings.padding_top as f32;
        let most = (available - self.inner_height() - self.settings.padding_bottom) as f32;
        let offset = round_f32(lerp_f32(self.settings.y_alignment, least, most)) as i32;
        self.write(|target| target.set_y(y + offset), store);
    }
    fn arrange(&mut self, store: &mut Store<'_>) {
        self.write(|target| target.arrange(), store);
    }
    fn write(&mut self, apply: impl FnOnce(&mut dyn LayoutElement), store: &mut Store<'_>) {
        match &mut self.slot {
            ChildSlot::Owned(element) => apply(element.as_mut()),
            ChildSlot::Handle(handle, _, _) => {
                if let Some(target) = store.widget_mut(*handle) {
                    apply(target);
                }
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutSettings {
    pub padding_left: i32,
    pub padding_top: i32,
    pub padding_right: i32,
    pub padding_bottom: i32,
    pub x_alignment: f32,
    pub y_alignment: f32,
}
impl LayoutSettings {
    pub fn new() -> Self {
        Self {
            padding_left: 0,
            padding_top: 0,
            padding_right: 0,
            padding_bottom: 0,
            x_alignment: 0.0,
            y_alignment: 0.0,
        }
    }
    pub fn defaults() -> Self {
        Self::new()
    }
    pub fn padding(&mut self, padding: i32) -> Self {
        self.padding_left = padding;
        self.padding_top = padding;
        self.padding_right = padding;
        self.padding_bottom = padding;
        *self
    }
    pub fn padding_xy(&mut self, horizontal: i32, vertical: i32) -> Self {
        self.padding_left = horizontal;
        self.padding_right = horizontal;
        self.padding_top = vertical;
        self.padding_bottom = vertical;
        *self
    }
    pub fn padding_edges(&mut self, left: i32, top: i32, right: i32, bottom: i32) -> Self {
        self.padding_left = left;
        self.padding_top = top;
        self.padding_right = right;
        self.padding_bottom = bottom;
        *self
    }
    pub fn align(&mut self, x_alignment: f32, y_alignment: f32) -> Self {
        self.x_alignment = x_alignment;
        self.y_alignment = y_alignment;
        *self
    }
    pub fn align_horizontally_center(&mut self) -> Self {
        self.align_horizontally(0.5)
    }
    pub fn align_horizontally(&mut self, x_alignment: f32) -> Self {
        self.x_alignment = x_alignment;
        *self
    }
    pub fn align_vertically_middle(&mut self) -> Self {
        self.align_vertically(0.5)
    }
    pub fn align_vertically(&mut self, y_alignment: f32) -> Self {
        self.y_alignment = y_alignment;
        *self
    }
}
impl Default for LayoutSettings {
    fn default() -> Self {
        Self::new()
    }
}
pub struct SpacerElement {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}
impl SpacerElement {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }
}
impl LayoutElement for SpacerElement {
    fn set_x(&mut self, x: i32) {
        self.x = x;
    }
    fn set_y(&mut self, y: i32) {
        self.y = y;
    }
    fn x(&self) -> i32 {
        self.x
    }
    fn y(&self) -> i32 {
        self.y
    }
    fn width(&self) -> i32 {
        self.width
    }
    fn height(&self) -> i32 {
        self.height
    }
}
pub struct FrameLayout {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    min_width: i32,
    min_height: i32,
    default_child_settings: LayoutSettings,
    children: Vec<Child>,
}
impl Default for FrameLayout {
    fn default() -> Self {
        Self::empty()
    }
}
impl FrameLayout {
    pub fn new(x: i32, y: i32, min_width: i32, min_height: i32) -> Self {
        Self {
            x,
            y,
            width: min_width,
            height: min_height,
            min_width,
            min_height,
            default_child_settings: LayoutSettings::default().align(0.5, 0.5),
            children: Vec::new(),
        }
    }
    pub fn with_min_dimensions(min_width: i32, min_height: i32) -> Self {
        Self::new(0, 0, min_width, min_height)
    }
    pub fn empty() -> Self {
        Self::new(0, 0, 0, 0)
    }
    pub fn set_min_width(&mut self, min_width: i32) -> &mut Self {
        self.min_width = min_width;
        self
    }
    pub fn set_min_height(&mut self, min_height: i32) -> &mut Self {
        self.min_height = min_height;
        self
    }
    pub fn set_min_dimensions(&mut self, min_width: i32, min_height: i32) -> &mut Self {
        self.set_min_width(min_width).set_min_height(min_height)
    }
    pub fn new_child_layout_settings(&self) -> LayoutSettings {
        self.default_child_settings
    }
    pub fn default_child_layout_settings(&mut self) -> &mut LayoutSettings {
        &mut self.default_child_settings
    }
    pub fn add_child(&mut self, child: impl LayoutElement + 'static) -> usize {
        self.add_child_with_settings(child, self.new_child_layout_settings())
    }
    pub fn add_child_with_settings(
        &mut self,
        child: impl LayoutElement + 'static,
        settings: LayoutSettings,
    ) -> usize {
        self.children.push(Child::owned(Box::new(child), settings));
        self.children.len() - 1
    }
    pub fn add_handle(&mut self, handle: usize, width: i32, height: i32) -> usize {
        self.add_handle_with_settings(handle, width, height, self.new_child_layout_settings())
    }
    pub fn add_handle_with_settings(
        &mut self,
        handle: usize,
        width: i32,
        height: i32,
        settings: LayoutSettings,
    ) -> usize {
        self.children
            .push(Child::new_handle(handle, width, height, settings));
        self.children.len() - 1
    }
    pub fn child(&self, index: usize) -> Option<&dyn LayoutElement> {
        self.children.get(index).and_then(Child::element)
    }
    pub fn child_mut(&mut self, index: usize) -> Option<&mut dyn LayoutElement> {
        self.children
            .get_mut(index)
            .and_then(|child| match &mut child.slot {
                ChildSlot::Owned(element) => Some(element.as_mut() as &mut dyn LayoutElement),
                ChildSlot::Handle(_, _, _) => None,
            })
    }
    pub fn each_handle(&self) -> impl Iterator<Item = usize> {
        self.children.iter().filter_map(Child::handle_index)
    }
    pub fn arrange_into(&mut self, widgets: &mut [Box<dyn LayoutElement>]) {
        self.arrange_in(&mut Store::Slice(widgets));
    }
    pub fn arrange_widgets<'a>(
        &mut self,
        widgets: &mut dyn Iterator<Item = (usize, &'a mut dyn LayoutElement)>,
    ) {
        let mut store = Store::Owned(store_from_pairs(widgets));
        self.arrange_in(&mut store);
    }
    fn arrange_in(&mut self, store: &mut Store<'_>) {
        for child in &mut self.children {
            child.arrange(store);
        }
        let mut result_width = self.min_width;
        let mut result_height = self.min_height;
        for child in &self.children {
            result_width = result_width.max(child.outer_width());
            result_height = result_height.max(child.outer_height());
        }
        for child in &mut self.children {
            child.set_x(self.x, result_width, store);
            child.set_y(self.y, result_height, store);
        }
        self.width = result_width;
        self.height = result_height;
    }
    pub fn align_in_dimension(pos: i32, length: i32, child_length: i32, alignment: f32) -> i32 {
        let offset = lerp_f32(alignment, 0.0, (length - child_length) as f32) as i32;
        pos + offset
    }
    pub fn align_in_dimension_y(pos: i32, length: i32, child_length: i32, alignment: f32) -> i32 {
        let offset = round_f32(lerp_f32(alignment, 0.0, (length - child_length) as f32)) as i32;
        pos + offset
    }
    pub fn align_in_rectangle(
        element: &mut dyn LayoutElement,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        align_x: f32,
        align_y: f32,
    ) {
        let new_x = Self::align_in_dimension(x, width, element.width(), align_x);
        let new_y = Self::align_in_dimension_y(y, height, element.height(), align_y);
        element.set_position(new_x, new_y);
    }
    pub fn center_in_rectangle(
        element: &mut dyn LayoutElement,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        Self::align_in_rectangle(element, x, y, width, height, 0.5, 0.5);
    }
    pub fn center_in_rect(element: &mut dyn LayoutElement, rect: ScreenRectangle) {
        Self::center_in_rectangle(element, rect.left(), rect.top(), rect.width, rect.height);
    }
}
impl LayoutElement for FrameLayout {
    fn set_x(&mut self, x: i32) {
        self.x = x;
    }
    fn set_y(&mut self, y: i32) {
        self.y = y;
    }
    fn x(&self) -> i32 {
        self.x
    }
    fn y(&self) -> i32 {
        self.y
    }
    fn width(&self) -> i32 {
        self.width
    }
    fn height(&self) -> i32 {
        self.height
    }
    fn arrange(&mut self) {
        self.arrange_in(&mut Store::Detached);
    }
}
impl Layout for FrameLayout {
    fn arrange_elements(&mut self) {
        self.arrange_in(&mut Store::Detached);
    }
    fn remove_children(&mut self) {
        self.children.clear();
    }
}
struct GridChild {
    child: Child,
    row: usize,
    column: usize,
    occupied_rows: usize,
    occupied_columns: usize,
}
impl GridChild {
    fn last_row(&self) -> usize {
        self.row
            .saturating_add(self.occupied_rows.saturating_sub(1))
    }
    fn last_column(&self) -> usize {
        self.column
            .saturating_add(self.occupied_columns.saturating_sub(1))
    }
}
pub struct GridLayout {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    row_spacing: i32,
    column_spacing: i32,
    default_cell_settings: LayoutSettings,
    children: Vec<GridChild>,
}
impl Default for GridLayout {
    fn default() -> Self {
        Self::new(0, 0)
    }
}
impl GridLayout {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            width: 0,
            height: 0,
            row_spacing: 0,
            column_spacing: 0,
            default_cell_settings: LayoutSettings::default(),
            children: Vec::new(),
        }
    }
    pub fn row_spacing(&mut self, spacing: i32) -> &mut Self {
        self.row_spacing = spacing;
        self
    }
    pub fn column_spacing(&mut self, spacing: i32) -> &mut Self {
        self.column_spacing = spacing;
        self
    }
    pub fn spacing(&mut self, spacing: i32) -> &mut Self {
        self.column_spacing(spacing).row_spacing(spacing)
    }
    pub fn new_cell_settings(&self) -> LayoutSettings {
        self.default_cell_settings
    }
    pub fn default_cell_settings(&mut self) -> &mut LayoutSettings {
        &mut self.default_cell_settings
    }
    pub fn add_child(
        &mut self,
        child: impl LayoutElement + 'static,
        row: usize,
        column: usize,
    ) -> usize {
        self.add_child_with_settings(child, row, column, self.new_cell_settings())
    }
    pub fn add_child_with_settings(
        &mut self,
        child: impl LayoutElement + 'static,
        row: usize,
        column: usize,
        settings: LayoutSettings,
    ) -> usize {
        self.add_child_spanned_with_settings(child, row, column, 1, 1, settings)
    }
    pub fn add_child_spanned(
        &mut self,
        child: impl LayoutElement + 'static,
        row: usize,
        column: usize,
        occupied_rows: usize,
        occupied_columns: usize,
    ) -> usize {
        self.add_child_spanned_with_settings(
            child,
            row,
            column,
            occupied_rows,
            occupied_columns,
            self.new_cell_settings(),
        )
    }
    pub fn add_child_spanned_with_settings(
        &mut self,
        child: impl LayoutElement + 'static,
        row: usize,
        column: usize,
        occupied_rows: usize,
        occupied_columns: usize,
        settings: LayoutSettings,
    ) -> usize {
        self.push(GridChild {
            child: Child::owned(Box::new(child), settings),
            row,
            column,
            occupied_rows: occupied_rows.max(1),
            occupied_columns: occupied_columns.max(1),
        })
    }
    pub fn add_handle(
        &mut self,
        handle: usize,
        width: i32,
        height: i32,
        row: usize,
        column: usize,
    ) -> usize {
        self.add_handle_with_settings(handle, width, height, row, column, self.new_cell_settings())
    }
    pub fn add_handle_with_settings(
        &mut self,
        handle: usize,
        width: i32,
        height: i32,
        row: usize,
        column: usize,
        settings: LayoutSettings,
    ) -> usize {
        self.add_handle_spanned_with_settings(handle, width, height, row, column, 1, 1, settings)
    }
    pub fn add_handle_spanned(
        &mut self,
        handle: usize,
        width: i32,
        height: i32,
        row: usize,
        column: usize,
        occupied_rows: usize,
        occupied_columns: usize,
    ) -> usize {
        self.add_handle_spanned_with_settings(
            handle,
            width,
            height,
            row,
            column,
            occupied_rows,
            occupied_columns,
            self.new_cell_settings(),
        )
    }
    pub fn add_handle_spanned_with_settings(
        &mut self,
        handle: usize,
        width: i32,
        height: i32,
        row: usize,
        column: usize,
        occupied_rows: usize,
        occupied_columns: usize,
        settings: LayoutSettings,
    ) -> usize {
        self.push(GridChild {
            child: Child::new_handle(handle, width, height, settings),
            row,
            column,
            occupied_rows: occupied_rows.max(1),
            occupied_columns: occupied_columns.max(1),
        })
    }
    fn push(&mut self, child: GridChild) -> usize {
        self.children.push(child);
        self.children.len() - 1
    }
    pub fn child(&self, index: usize) -> Option<&dyn LayoutElement> {
        self.children
            .get(index)
            .and_then(|child| child.child.element())
    }
    pub fn child_mut(&mut self, index: usize) -> Option<&mut dyn LayoutElement> {
        self.children
            .get_mut(index)
            .and_then(|child| match &mut child.child.slot {
                ChildSlot::Owned(element) => Some(element.as_mut() as &mut dyn LayoutElement),
                ChildSlot::Handle(_, _, _) => None,
            })
    }
    pub fn each_handle(&self) -> impl Iterator<Item = usize> {
        self.children
            .iter()
            .filter_map(|child| child.child.handle_index())
    }
    pub fn arrange_into(&mut self, widgets: &mut [Box<dyn LayoutElement>]) {
        self.arrange_in(&mut Store::Slice(widgets));
    }
    pub fn arrange_widgets<'a>(
        &mut self,
        widgets: &mut dyn Iterator<Item = (usize, &'a mut dyn LayoutElement)>,
    ) {
        let mut store = Store::Owned(store_from_pairs(widgets));
        self.arrange_in(&mut store);
    }
    fn arrange_in(&mut self, store: &mut Store<'_>) {
        let row_spacing = self.row_spacing;
        let column_spacing = self.column_spacing;
        let origin_x = self.x;
        let origin_y = self.y;
        for child in &mut self.children {
            child.child.arrange(store);
        }
        let row_count = self
            .children
            .iter()
            .map(GridChild::last_row)
            .max()
            .map_or(1, |row| row + 1);
        let column_count = self
            .children
            .iter()
            .map(GridChild::last_column)
            .max()
            .map_or(1, |column| column + 1);
        let mut row_heights = vec![0; row_count];
        let mut column_widths = vec![0; column_count];
        for child in &self.children {
            let child_height =
                child.child.outer_height() - (child.occupied_rows as i32 - 1) * row_spacing;
            let shares = (child.row..=child.last_row())
                .zip(Divisor::parts(child_height, child.occupied_rows as i32));
            for (row, share) in shares {
                row_heights[row] = row_heights[row].max(share);
            }
            let child_width =
                child.child.outer_width() - (child.occupied_columns as i32 - 1) * column_spacing;
            let shares = (child.column..=child.last_column())
                .zip(Divisor::parts(child_width, child.occupied_columns as i32));
            for (column, share) in shares {
                column_widths[column] = column_widths[column].max(share);
            }
        }
        let mut column_offsets = vec![0; column_count];
        for column in 1..column_count {
            column_offsets[column] =
                column_offsets[column - 1] + column_widths[column - 1] + column_spacing;
        }
        let mut row_offsets = vec![0; row_count];
        for row in 1..row_count {
            row_offsets[row] = row_offsets[row - 1] + row_heights[row - 1] + row_spacing;
        }
        for child in &mut self.children {
            let mut available_width = column_spacing * (child.occupied_columns as i32 - 1);
            for column in child.column..=child.last_column() {
                available_width += column_widths[column];
            }
            let cell_x = origin_x + column_offsets[child.column];
            child.child.set_x(cell_x, available_width, store);
            let mut available_height = row_spacing * (child.occupied_rows as i32 - 1);
            for row in child.row..=child.last_row() {
                available_height += row_heights[row];
            }
            let cell_y = origin_y + row_offsets[child.row];
            child.child.set_y(cell_y, available_height, store);
        }
        self.width = column_offsets[column_count - 1] + column_widths[column_count - 1];
        self.height = row_offsets[row_count - 1] + row_heights[row_count - 1];
    }
    pub fn create_row_helper(&mut self, columns: usize) -> RowHelper<'_> {
        RowHelper::new(self, columns.max(1))
    }
}
impl LayoutElement for GridLayout {
    fn set_x(&mut self, x: i32) {
        self.x = x;
    }
    fn set_y(&mut self, y: i32) {
        self.y = y;
    }
    fn x(&self) -> i32 {
        self.x
    }
    fn y(&self) -> i32 {
        self.y
    }
    fn width(&self) -> i32 {
        self.width
    }
    fn height(&self) -> i32 {
        self.height
    }
    fn arrange(&mut self) {
        self.arrange_in(&mut Store::Detached);
    }
}
impl Layout for GridLayout {
    fn arrange_elements(&mut self) {
        self.arrange_in(&mut Store::Detached);
    }
    fn remove_children(&mut self) {
        self.children.clear();
    }
}
pub struct RowHelper<'a> {
    grid: &'a mut GridLayout,
    columns: usize,
    index: usize,
}
impl<'a> RowHelper<'a> {
    fn new(grid: &'a mut GridLayout, columns: usize) -> Self {
        Self {
            grid,
            columns,
            index: 0,
        }
    }
    pub fn add_child(&mut self, child: impl LayoutElement + 'static) -> usize {
        self.add_child_with_span(child, 1)
    }
    pub fn add_child_with_span(
        &mut self,
        child: impl LayoutElement + 'static,
        column_width: usize,
    ) -> usize {
        let settings = self.grid.new_cell_settings();
        self.add_child_with_span_and_settings(child, column_width, settings)
    }
    pub fn add_child_with_settings(
        &mut self,
        child: impl LayoutElement + 'static,
        settings: LayoutSettings,
    ) -> usize {
        self.add_child_with_span_and_settings(child, 1, settings)
    }
    pub fn add_child_with_span_and_settings(
        &mut self,
        child: impl LayoutElement + 'static,
        column_width: usize,
        settings: LayoutSettings,
    ) -> usize {
        let (row, column) = self.claim(column_width.max(1));
        self.grid.add_child_spanned_with_settings(
            child,
            row,
            column,
            1,
            column_width.max(1),
            settings,
        )
    }
    pub fn add_handle(&mut self, handle: usize, width: i32, height: i32) -> usize {
        let settings = self.grid.new_cell_settings();
        self.add_handle_with_span(handle, width, height, 1, settings)
    }
    pub fn add_handle_with_span(
        &mut self,
        handle: usize,
        width: i32,
        height: i32,
        column_width: usize,
        settings: LayoutSettings,
    ) -> usize {
        let column_width = column_width.max(1);
        let (row, column) = self.claim(column_width);
        self.grid.add_handle_spanned_with_settings(
            handle,
            width,
            height,
            row,
            column,
            1,
            column_width,
            settings,
        )
    }
    pub fn new_row(&mut self) -> &mut Self {
        let next = round_toward((self.index + 1) as i32, self.columns as i32);
        self.index = next.max(0) as usize;
        self
    }
    pub fn get_grid(&self) -> &GridLayout {
        self.grid
    }
    pub fn new_cell_settings(&self) -> LayoutSettings {
        self.grid.new_cell_settings()
    }
    pub fn default_cell_setting(&mut self) -> &mut LayoutSettings {
        self.grid.default_cell_settings()
    }
    fn claim(&mut self, column_width: usize) -> (usize, usize) {
        let mut row = self.index / self.columns;
        let mut column_begin = self.index % self.columns;
        if column_begin + column_width > self.columns {
            row += 1;
            column_begin = 0;
            self.index = round_toward(self.index as i32, self.columns as i32).max(0) as usize;
        }
        self.index += column_width;
        (row, column_begin)
    }
}
impl Child {
    fn primary_len(&self, orientation: Orientation) -> i32 {
        match orientation {
            Orientation::Horizontal => self.outer_width(),
            Orientation::Vertical => self.outer_height(),
        }
    }
    fn secondary_len(&self, orientation: Orientation) -> i32 {
        match orientation {
            Orientation::Horizontal => self.outer_height(),
            Orientation::Vertical => self.outer_width(),
        }
    }
    fn set_primary(&mut self, orientation: Orientation, position: i32, store: &mut Store<'_>) {
        match orientation {
            Orientation::Horizontal => {
                let available = self.outer_width();
                self.set_x(position, available, store);
            }
            Orientation::Vertical => {
                let available = self.outer_height();
                self.set_y(position, available, store);
            }
        }
    }
    fn set_secondary(
        &mut self,
        orientation: Orientation,
        position: i32,
        available: i32,
        store: &mut Store<'_>,
    ) {
        match orientation {
            Orientation::Horizontal => self.set_y(position, available, store),
            Orientation::Vertical => self.set_x(position, available, store),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}
impl Orientation {
    fn apply_spacing(self, grid: &mut GridLayout, spacing: i32) {
        match self {
            Self::Horizontal => {
                grid.column_spacing(spacing);
            }
            Self::Vertical => {
                grid.row_spacing(spacing);
            }
        }
    }
    fn cell(self, index: usize) -> (usize, usize) {
        match self {
            Self::Horizontal => (0, index),
            Self::Vertical => (index, 0),
        }
    }
}
pub struct LinearLayout {
    wrapped: GridLayout,
    orientation: Orientation,
    next_child_index: usize,
}
impl LinearLayout {
    pub fn vertical() -> Self {
        Self::new(0, 0, Orientation::Vertical)
    }
    pub fn horizontal() -> Self {
        Self::new(0, 0, Orientation::Horizontal)
    }
    pub fn new(x: i32, y: i32, orientation: Orientation) -> Self {
        Self {
            wrapped: GridLayout::new(x, y),
            orientation,
            next_child_index: 0,
        }
    }
    pub fn spacing(mut self, spacing: i32) -> Self {
        self.orientation.apply_spacing(&mut self.wrapped, spacing);
        self
    }
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }
    pub fn grid(&self) -> &GridLayout {
        &self.wrapped
    }
    pub fn grid_mut(&mut self) -> &mut GridLayout {
        &mut self.wrapped
    }
    pub fn new_cell_settings(&self) -> LayoutSettings {
        self.wrapped.new_cell_settings()
    }
    pub fn default_cell_settings(&mut self) -> &mut LayoutSettings {
        self.wrapped.default_cell_settings()
    }
    pub fn add_child(&mut self, child: impl LayoutElement + 'static) -> usize {
        self.add_child_with_settings(child, self.new_cell_settings())
    }
    pub fn add_child_with_settings(
        &mut self,
        child: impl LayoutElement + 'static,
        settings: LayoutSettings,
    ) -> usize {
        let index = self.next_child_index;
        self.next_child_index += 1;
        let (row, column) = self.orientation.cell(index);
        self.wrapped
            .add_child_with_settings(child, row, column, settings)
    }
    pub fn add_handle(&mut self, handle: usize, width: i32, height: i32) -> usize {
        self.add_handle_with_settings(handle, width, height, self.new_cell_settings())
    }
    pub fn add_handle_with_settings(
        &mut self,
        handle: usize,
        width: i32,
        height: i32,
        settings: LayoutSettings,
    ) -> usize {
        let index = self.next_child_index;
        self.next_child_index += 1;
        let (row, column) = self.orientation.cell(index);
        self.wrapped
            .add_handle_with_settings(handle, width, height, row, column, settings)
    }
    pub fn child(&self, index: usize) -> Option<&dyn LayoutElement> {
        self.wrapped.child(index)
    }
    pub fn child_mut(&mut self, index: usize) -> Option<&mut dyn LayoutElement> {
        self.wrapped.child_mut(index)
    }
    pub fn each_handle(&self) -> impl Iterator<Item = usize> {
        self.wrapped.each_handle()
    }
    pub fn arrange_into(&mut self, widgets: &mut [Box<dyn LayoutElement>]) {
        self.wrapped.arrange_into(widgets);
    }
    pub fn arrange_widgets<'a>(
        &mut self,
        widgets: &mut dyn Iterator<Item = (usize, &'a mut dyn LayoutElement)>,
    ) {
        self.wrapped.arrange_widgets(widgets);
    }
}
impl LayoutElement for LinearLayout {
    fn set_x(&mut self, x: i32) {
        self.wrapped.set_x(x);
    }
    fn set_y(&mut self, y: i32) {
        self.wrapped.set_y(y);
    }
    fn x(&self) -> i32 {
        self.wrapped.x()
    }
    fn y(&self) -> i32 {
        self.wrapped.y()
    }
    fn width(&self) -> i32 {
        self.wrapped.width()
    }
    fn height(&self) -> i32 {
        self.wrapped.height()
    }
    fn arrange(&mut self) {
        self.arrange_elements();
    }
}
impl Layout for LinearLayout {
    fn arrange_elements(&mut self) {
        self.wrapped.arrange_elements();
    }
    fn remove_children(&mut self) {
        self.wrapped.remove_children();
        self.next_child_index = 0;
    }
}
pub struct EqualSpacingLayout {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    orientation: Orientation,
    default_child_settings: LayoutSettings,
    children: Vec<Child>,
}
impl EqualSpacingLayout {
    pub fn new(width: i32, height: i32, orientation: Orientation) -> Self {
        Self::at(0, 0, width, height, orientation)
    }
    pub fn at(x: i32, y: i32, width: i32, height: i32, orientation: Orientation) -> Self {
        Self {
            x,
            y,
            width,
            height,
            orientation,
            default_child_settings: LayoutSettings::default(),
            children: Vec::new(),
        }
    }
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }
    pub fn new_child_layout_settings(&self) -> LayoutSettings {
        self.default_child_settings
    }
    pub fn default_child_layout_settings(&mut self) -> &mut LayoutSettings {
        &mut self.default_child_settings
    }
    pub fn add_child(&mut self, child: impl LayoutElement + 'static) -> usize {
        self.add_child_with_settings(child, self.new_child_layout_settings())
    }
    pub fn add_child_with_settings(
        &mut self,
        child: impl LayoutElement + 'static,
        settings: LayoutSettings,
    ) -> usize {
        self.children.push(Child::owned(Box::new(child), settings));
        self.children.len() - 1
    }
    pub fn add_handle(&mut self, handle: usize, width: i32, height: i32) -> usize {
        self.add_handle_with_settings(handle, width, height, self.new_child_layout_settings())
    }
    pub fn add_handle_with_settings(
        &mut self,
        handle: usize,
        width: i32,
        height: i32,
        settings: LayoutSettings,
    ) -> usize {
        self.children
            .push(Child::new_handle(handle, width, height, settings));
        self.children.len() - 1
    }
    pub fn child(&self, index: usize) -> Option<&dyn LayoutElement> {
        self.children.get(index).and_then(Child::element)
    }
    pub fn child_mut(&mut self, index: usize) -> Option<&mut dyn LayoutElement> {
        self.children
            .get_mut(index)
            .and_then(|child| match &mut child.slot {
                ChildSlot::Owned(element) => Some(element.as_mut() as &mut dyn LayoutElement),
                ChildSlot::Handle(_, _, _) => None,
            })
    }
    pub fn each_handle(&self) -> impl Iterator<Item = usize> {
        self.children.iter().filter_map(Child::handle_index)
    }
    pub fn arrange_into(&mut self, widgets: &mut [Box<dyn LayoutElement>]) {
        self.arrange_in(&mut Store::Slice(widgets));
    }
    pub fn arrange_widgets<'a>(
        &mut self,
        widgets: &mut dyn Iterator<Item = (usize, &'a mut dyn LayoutElement)>,
    ) {
        let mut store = Store::Owned(store_from_pairs(widgets));
        self.arrange_in(&mut store);
    }
    fn arrange_in(&mut self, store: &mut Store<'_>) {
        for child in &mut self.children {
            child.arrange(store);
        }
        if self.children.is_empty() {
            return;
        }
        let orientation = self.orientation;
        let origin_x = self.x;
        let origin_y = self.y;
        let layout_primary = match orientation {
            Orientation::Horizontal => self.width,
            Orientation::Vertical => self.height,
        };
        let mut total_primary = 0;
        let mut max_secondary = match orientation {
            Orientation::Horizontal => self.height,
            Orientation::Vertical => self.width,
        };
        for child in &self.children {
            total_primary += child.primary_len(orientation);
            max_secondary = max_secondary.max(child.secondary_len(orientation));
        }
        let remaining = layout_primary - total_primary;
        let child_count = self.children.len();
        let mut position = match orientation {
            Orientation::Horizontal => origin_x,
            Orientation::Vertical => origin_y,
        };
        let first_primary = self.children[0].primary_len(orientation);
        self.children[0].set_primary(orientation, position, store);
        position += first_primary;
        if child_count >= 2 {
            let gaps = Divisor::parts(remaining, (child_count - 1) as i32);
            for (offset, child) in self.children.iter_mut().enumerate().skip(1) {
                let gap = match gaps.get(offset - 1) {
                    Some(gap) => *gap,
                    None => 0,
                };
                position += gap;
                child.set_primary(orientation, position, store);
                let primary = child.primary_len(orientation);
                position += primary;
            }
        }
        let secondary_position = match orientation {
            Orientation::Horizontal => origin_y,
            Orientation::Vertical => origin_x,
        };
        for child in &mut self.children {
            child.set_secondary(orientation, secondary_position, max_secondary, store);
        }
        match orientation {
            Orientation::Horizontal => self.height = max_secondary,
            Orientation::Vertical => self.width = max_secondary,
        }
    }
}
impl LayoutElement for EqualSpacingLayout {
    fn set_x(&mut self, x: i32) {
        self.x = x;
    }
    fn set_y(&mut self, y: i32) {
        self.y = y;
    }
    fn x(&self) -> i32 {
        self.x
    }
    fn y(&self) -> i32 {
        self.y
    }
    fn width(&self) -> i32 {
        self.width
    }
    fn height(&self) -> i32 {
        self.height
    }
    fn arrange(&mut self) {
        self.arrange_elements();
    }
}
impl Layout for EqualSpacingLayout {
    fn arrange_elements(&mut self) {
        self.arrange_in(&mut Store::Detached);
    }
    fn remove_children(&mut self) {
        self.children.clear();
    }
}
use crate::FontWidthMeasurer;
use crate::components::text_widget::{LINE_HEIGHT, StringWidget};
use el_core::Component;
pub const MAGIC_PADDING: i32 = 13;
pub const DEFAULT_HEADER_AND_FOOTER_HEIGHT: i32 = 33;
pub const CONTENT_MARGIN_TOP: i32 = 30;
pub struct HeaderAndFooterLayout {
    screen_width: i32,
    screen_height: i32,
    header_frame: FrameLayout,
    footer_frame: FrameLayout,
    contents_frame: FrameLayout,
    header_height: i32,
    footer_height: i32,
}
impl HeaderAndFooterLayout {
    pub fn new(screen_width: i32, screen_height: i32) -> Self {
        Self::with_heights(
            screen_width,
            screen_height,
            DEFAULT_HEADER_AND_FOOTER_HEIGHT,
            DEFAULT_HEADER_AND_FOOTER_HEIGHT,
        )
    }
    pub fn with_header_and_footer_height(
        screen_width: i32,
        screen_height: i32,
        header_and_footer_height: i32,
    ) -> Self {
        Self::with_heights(
            screen_width,
            screen_height,
            header_and_footer_height,
            header_and_footer_height,
        )
    }
    pub fn with_heights(
        screen_width: i32,
        screen_height: i32,
        header_height: i32,
        footer_height: i32,
    ) -> Self {
        let mut header_frame = FrameLayout::empty();
        let mut footer_frame = FrameLayout::empty();
        header_frame.default_child_layout_settings().align(0.5, 0.5);
        footer_frame.default_child_layout_settings().align(0.5, 0.5);
        Self {
            screen_width,
            screen_height,
            header_frame,
            footer_frame,
            contents_frame: FrameLayout::empty(),
            header_height,
            footer_height,
        }
    }
    pub fn header_frame(&self) -> &FrameLayout {
        &self.header_frame
    }
    pub fn header_frame_mut(&mut self) -> &mut FrameLayout {
        &mut self.header_frame
    }
    pub fn footer_frame(&self) -> &FrameLayout {
        &self.footer_frame
    }
    pub fn footer_frame_mut(&mut self) -> &mut FrameLayout {
        &mut self.footer_frame
    }
    pub fn contents_frame(&self) -> &FrameLayout {
        &self.contents_frame
    }
    pub fn contents_frame_mut(&mut self) -> &mut FrameLayout {
        &mut self.contents_frame
    }
    pub fn footer_height(&self) -> i32 {
        self.footer_height
    }
    pub fn set_footer_height(&mut self, footer_height: i32) -> &mut Self {
        self.footer_height = footer_height;
        self
    }
    pub fn header_height(&self) -> i32 {
        self.header_height
    }
    pub fn set_header_height(&mut self, header_height: i32) -> &mut Self {
        self.header_height = header_height;
        self
    }
    pub fn content_height(&self) -> i32 {
        self.screen_height - self.header_height - self.footer_height
    }
    pub fn screen_width(&self) -> i32 {
        self.screen_width
    }
    pub fn screen_height(&self) -> i32 {
        self.screen_height
    }
    pub fn add_to_header(&mut self, child: impl LayoutElement + 'static) -> usize {
        self.header_frame.add_child(child)
    }
    pub fn add_to_header_with_settings(
        &mut self,
        child: impl LayoutElement + 'static,
        settings: LayoutSettings,
    ) -> usize {
        self.header_frame.add_child_with_settings(child, settings)
    }
    pub fn add_to_header_handle(&mut self, handle: usize, width: i32, height: i32) -> usize {
        self.header_frame.add_handle(handle, width, height)
    }
    pub fn add_to_header_handle_with_settings(
        &mut self,
        handle: usize,
        width: i32,
        height: i32,
        settings: LayoutSettings,
    ) -> usize {
        self.header_frame
            .add_handle_with_settings(handle, width, height, settings)
    }
    pub fn add_title_header(&mut self, label: Component) -> usize {
        self.add_to_header(StringWidget::from_message(label))
    }
    pub fn add_to_footer(&mut self, child: impl LayoutElement + 'static) -> usize {
        self.footer_frame.add_child(child)
    }
    pub fn add_to_footer_with_settings(
        &mut self,
        child: impl LayoutElement + 'static,
        settings: LayoutSettings,
    ) -> usize {
        self.footer_frame.add_child_with_settings(child, settings)
    }
    pub fn add_to_footer_handle(&mut self, handle: usize, width: i32, height: i32) -> usize {
        self.footer_frame.add_handle(handle, width, height)
    }
    pub fn add_to_footer_handle_with_settings(
        &mut self,
        handle: usize,
        width: i32,
        height: i32,
        settings: LayoutSettings,
    ) -> usize {
        self.footer_frame
            .add_handle_with_settings(handle, width, height, settings)
    }
    pub fn add_to_contents(&mut self, child: impl LayoutElement + 'static) -> usize {
        self.contents_frame.add_child(child)
    }
    pub fn add_to_contents_with_settings(
        &mut self,
        child: impl LayoutElement + 'static,
        settings: LayoutSettings,
    ) -> usize {
        self.contents_frame.add_child_with_settings(child, settings)
    }
    pub fn add_to_contents_handle(&mut self, handle: usize, width: i32, height: i32) -> usize {
        self.contents_frame.add_handle(handle, width, height)
    }
    pub fn add_to_contents_handle_with_settings(
        &mut self,
        handle: usize,
        width: i32,
        height: i32,
        settings: LayoutSettings,
    ) -> usize {
        self.contents_frame
            .add_handle_with_settings(handle, width, height, settings)
    }
    pub fn each_header_handle(&self) -> impl Iterator<Item = usize> {
        self.header_frame.each_handle()
    }
    pub fn each_footer_handle(&self) -> impl Iterator<Item = usize> {
        self.footer_frame.each_handle()
    }
    pub fn each_content_handle(&self) -> impl Iterator<Item = usize> {
        self.contents_frame.each_handle()
    }
    pub fn each_handle(&self) -> impl Iterator<Item = usize> {
        self.header_frame
            .each_handle()
            .chain(self.contents_frame.each_handle())
            .chain(self.footer_frame.each_handle())
    }
    pub fn arrange_into(&mut self, widgets: &mut [Box<dyn LayoutElement>]) {
        self.arrange_in(&mut Store::Slice(widgets));
    }
    pub fn arrange_widgets<'a>(
        &mut self,
        widgets: &mut dyn Iterator<Item = (usize, &'a mut dyn LayoutElement)>,
    ) {
        let mut store = Store::Owned(store_from_pairs(widgets));
        self.arrange_in(&mut store);
    }
    fn arrange_in(&mut self, store: &mut Store<'_>) {
        let screen_width = self.screen_width;
        let screen_height = self.screen_height;
        let header_height = self.header_height;
        let footer_height = self.footer_height;
        self.header_frame
            .set_min_dimensions(screen_width, header_height);
        self.header_frame.set_position(0, 0);
        self.header_frame.arrange_in(store);
        self.footer_frame
            .set_min_dimensions(screen_width, footer_height);
        self.footer_frame
            .set_position(0, screen_height - footer_height);
        self.footer_frame.arrange_in(store);
        self.contents_frame.set_min_width(screen_width);
        self.contents_frame.set_position(0, 0);
        self.contents_frame.arrange_in(store);
        let preferred_content_y = header_height + CONTENT_MARGIN_TOP;
        let max_content_y = screen_height - footer_height - self.contents_frame.height();
        self.contents_frame
            .set_position(0, preferred_content_y.min(max_content_y));
        self.contents_frame.arrange_in(store);
    }
}
impl LayoutElement for HeaderAndFooterLayout {
    fn set_x(&mut self, _x: i32) {}
    fn set_y(&mut self, _y: i32) {}
    fn x(&self) -> i32 {
        0
    }
    fn y(&self) -> i32 {
        0
    }
    fn width(&self) -> i32 {
        self.screen_width
    }
    fn height(&self) -> i32 {
        self.screen_height
    }
    fn arrange(&mut self) {
        self.arrange_elements();
    }
}
impl Layout for HeaderAndFooterLayout {
    fn arrange_elements(&mut self) {
        self.arrange_in(&mut Store::Detached);
    }
    fn remove_children(&mut self) {
        self.header_frame.remove_children();
        self.contents_frame.remove_children();
        self.footer_frame.remove_children();
    }
}
pub struct CommonLayouts;
impl CommonLayouts {
    const LABEL_SPACING: i32 = 4;
    pub fn labeled_element(
        font: &dyn FontWidthMeasurer,
        element: impl LayoutElement + 'static,
        label: Component,
    ) -> LinearLayout {
        Self::labeled_element_with_settings(font, element, label, LayoutSettings::default())
    }
    pub fn labeled_element_with_settings(
        font: &dyn FontWidthMeasurer,
        element: impl LayoutElement + 'static,
        label: Component,
        settings: LayoutSettings,
    ) -> LinearLayout {
        let mut layout = LinearLayout::vertical().spacing(Self::LABEL_SPACING);
        let label_width = font.measure_width(&label.get_string(), 1.0).ceil() as i32;
        layout.add_child(StringWidget::with_size(label_width, LINE_HEIGHT, label));
        layout.add_child_with_settings(element, settings);
        layout
    }
}
