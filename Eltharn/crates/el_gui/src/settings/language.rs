use crate::FontWidthMeasurer;
use crate::components::click_sound_sink;
use crate::components::edit_box::{Clipboard, EditBox, EditBoxDraw, EditBoxKey, EditBoxRenderPlan};
use crate::components::{
    Button, GuiEventListener, Renderable, TabOrderedElement, WidgetClickSound,
};
use crate::framework::graphics::{GuiGraphicsExtractor, WidgetTooltipPosition};
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{NarratableEntry, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{
    CharacterEvent, CursorType, KeyEvent, MouseButtonEvent, MouseButtonInfo, PreeditEvent,
};
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use web_time::Instant;
const EDIT_BOX_INSERT_CURSOR_LINE_HEIGHT: i32 = 10;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageInfo {
    pub code: String,
    pub name: String,
    pub region: String,
    pub bidirectional: bool,
}
impl LanguageInfo {
    pub fn new(
        code: impl Into<String>,
        name: impl Into<String>,
        region: impl Into<String>,
        bidirectional: bool,
    ) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            region: region.into(),
            bidirectional,
        }
    }
}
pub fn default_languages() -> Vec<LanguageInfo> {
    vec![LanguageInfo::new("en_us", "English", "US", false)]
}
pub const HEADER_HEIGHT: i32 = 33;
pub const FOOTER_HEIGHT: i32 = 53;
pub const ITEM_HEIGHT: i32 = 18;
pub const ROW_WIDTH: i32 = 270;
pub const SEARCH_BOX_WIDTH: i32 = 200;
pub const SEARCH_BOX_HEIGHT: i32 = 15;
pub const SEARCH_BOX_Y: i32 = 17;
pub const FOOTER_BUTTON_WIDTH: i32 = 150;
pub const FOOTER_BUTTON_HEIGHT: i32 = 20;
pub const FOOTER_BUTTON_Y_OFFSET: i32 = 28;
pub const SCROLLBAR_WIDTH: i32 = 6;
pub const SCROLLBAR_OFFSET_X: i32 = 143;
pub const WARNING_COLOR: i32 = 0xFFBA_BABAu32 as i32;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LanguageSelectAction {
    Done,
    FontSettingsClicked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LanguageSelectFocus {
    Search,
    List,
    FontButton,
    DoneButton,
}
struct FooterButtonHandle {
    button: Rc<RefCell<Button<LanguageSelectAction>>>,
}
impl FooterButtonHandle {
    fn new(button: Rc<RefCell<Button<LanguageSelectAction>>>) -> Self {
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
impl NarratableEntry for FooterButtonHandle {
    fn narration_priority(&self) -> NarrationPriority {
        self.button.borrow().narration_priority()
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        self.button.borrow().update_narration(output);
    }
}
pub struct LanguageSelectScreen {
    base: ScreenBase,
    font_button: Rc<RefCell<Button<LanguageSelectAction>>>,
    done_button: Rc<RefCell<Button<LanguageSelectAction>>>,
    pub search_query: String,
    search_box: EditBox,
    active_search_preedit: Option<PreeditEvent>,
    clipboard: Option<Rc<RefCell<dyn Clipboard>>>,
    screen_started_at: Instant,
    search_dragging: bool,
    pub scroll_y: f32,
    pub dragging_scrollbar: bool,
    pub selected_code: String,
    original_language_code: String,
    has_selection: bool,
    focus: LanguageSelectFocus,
    last_mouse_click: Option<(Instant, i32)>,
    pub in_world: bool,
    pub last_width: i32,
    pub last_height: i32,
    pub parent_is_title: bool,
    pub languages: Vec<LanguageInfo>,
    font: Arc<dyn FontWidthMeasurer>,
    pending_action: Option<LanguageSelectAction>,
}
impl LanguageSelectScreen {
    pub fn new(in_world: bool, current_language: String, font: Arc<dyn FontWidthMeasurer>) -> Self {
        Self::with_languages(in_world, current_language, default_languages(), font)
    }
    pub fn with_languages(
        in_world: bool,
        current_language: String,
        mut languages: Vec<LanguageInfo>,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        languages.sort_by(|a, b| a.code.cmp(&b.code));
        let has_selection = languages
            .iter()
            .any(|language| language.code.eq_ignore_ascii_case(&current_language));
        let font_btn = Button::builder(Component::literal(&crate::translate("options.font")))
            .bounds(0, 0, FOOTER_BUTTON_WIDTH, FOOTER_BUTTON_HEIGHT)
            .with_action(LanguageSelectAction::FontSettingsClicked)
            .with_font(Arc::clone(&font))
            .build();
        let done_btn = Button::builder(Component::literal(&crate::translate("gui.done")))
            .bounds(0, 0, FOOTER_BUTTON_WIDTH, FOOTER_BUTTON_HEIGHT)
            .with_action(LanguageSelectAction::Done)
            .with_font(Arc::clone(&font))
            .build();
        let mut search_box = EditBox::new(
            font.as_ref(),
            0,
            SEARCH_BOX_Y,
            SEARCH_BOX_WIDTH,
            SEARCH_BOX_HEIGHT,
        );
        search_box.set_hint(Some(crate::translate("gui.language.search")));
        Self {
            base: ScreenBase::new(Component::literal(&crate::translate(
                "options.language.title",
            ))),
            font_button: Rc::new(RefCell::new(font_btn)),
            done_button: Rc::new(RefCell::new(done_btn)),
            search_query: String::new(),
            search_box,
            active_search_preedit: None,
            clipboard: None,
            screen_started_at: Instant::now(),
            search_dragging: false,
            scroll_y: 0.0,
            dragging_scrollbar: false,
            selected_code: current_language.clone(),
            original_language_code: current_language,
            has_selection,
            focus: LanguageSelectFocus::Search,
            last_mouse_click: None,
            in_world,
            last_width: 0,
            last_height: 0,
            parent_is_title: false,
            languages,
            font,
            pending_action: None,
        }
    }
    pub fn take_action(&mut self) -> Option<LanguageSelectAction> {
        self.pending_action.take()
    }
    pub fn set_clipboard(&mut self, clipboard: Rc<RefCell<dyn Clipboard>>) {
        self.clipboard = Some(clipboard);
    }
    fn now_ms(&self) -> u64 {
        self.screen_started_at
            .elapsed()
            .as_millis()
            .min(u64::MAX as u128) as u64
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn scrollbar_x(width: i32) -> i32 {
        width / 2 + SCROLLBAR_OFFSET_X
    }
    pub fn font_button_x(width: i32) -> i32 {
        width / 2 - 154
    }
    pub fn done_button_x(width: i32) -> i32 {
        width / 2 + 4
    }
    pub fn tick(&mut self) {}
    pub fn get_filtered_languages(&self) -> Vec<&LanguageInfo> {
        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            self.languages.iter().collect()
        } else {
            self.languages
                .iter()
                .filter(|lang| {
                    lang.name.to_lowercase().contains(&query)
                        || lang.region.to_lowercase().contains(&query)
                })
                .collect()
        }
    }
    pub fn max_scroll_amount(&self, height: i32) -> f32 {
        let visible_height = height - HEADER_HEIGHT - FOOTER_HEIGHT;
        if visible_height <= 0 {
            return 0.0;
        }
        let filtered_len = self.get_filtered_languages().len() as i32;
        let total_content_height = filtered_len * ITEM_HEIGHT + 4;
        (total_content_height - visible_height).max(0) as f32
    }
    pub fn scrollbar_height(&self, height: i32) -> i32 {
        let scroll_area_h = height - HEADER_HEIGHT - FOOTER_HEIGHT;
        if scroll_area_h <= 0 {
            return 0;
        }
        let filtered_len = self.get_filtered_languages().len() as i32;
        let total_content_height = (filtered_len * ITEM_HEIGHT + 4).max(1);
        let bar_h =
            ((scroll_area_h * scroll_area_h) as f32 / total_content_height as f32).round() as i32;
        let upper = (scroll_area_h - 8).max(1);
        let lower = 32.min(upper);
        bar_h.clamp(lower, upper).min(scroll_area_h)
    }
    fn set_focus(&mut self, focus: LanguageSelectFocus) {
        let now_ms = self.now_ms();
        if self.focus == LanguageSelectFocus::Search {
            self.search_box.set_focused(false, now_ms);
        }
        if focus != LanguageSelectFocus::Search {
            self.search_dragging = false;
            self.active_search_preedit = None;
        }
        self.focus = focus;
        if focus == LanguageSelectFocus::Search {
            self.search_box.set_focused(true, now_ms);
        }
        match focus {
            LanguageSelectFocus::Search | LanguageSelectFocus::List => {
                self.base.clear_focus();
            }
            LanguageSelectFocus::FontButton => self.base.set_focused(Some(0)),
            LanguageSelectFocus::DoneButton => self.base.set_focused(Some(1)),
        }
    }
    fn move_focus(&mut self, forward: bool) {
        use LanguageSelectFocus::{DoneButton, FontButton, List, Search};
        let next = match (self.focus, forward) {
            (Search, true) => List,
            (List, true) => FontButton,
            (FontButton, true) => DoneButton,
            (DoneButton, true) => Search,
            (Search, false) => DoneButton,
            (DoneButton, false) => FontButton,
            (FontButton, false) => List,
            (List, false) => Search,
        };
        self.set_focus(next);
    }
    fn clear_selection_after_filter_change(&mut self) {
        self.has_selection = false;
        self.selected_code.clone_from(&self.original_language_code);
        self.scroll_y = 0.0;
    }
    fn sync_search_query(&mut self) {
        let query = self.search_box.value();
        if self.search_query != query {
            self.search_query = query;
            self.clear_selection_after_filter_change();
        }
    }
    fn queue_done(&mut self) {
        if !self.has_selection {
            self.selected_code.clone_from(&self.original_language_code);
        }
        self.pending_action = Some(LanguageSelectAction::Done);
    }
    fn queue_escape(&mut self) {
        self.selected_code.clone_from(&self.original_language_code);
        self.pending_action = Some(LanguageSelectAction::Done);
    }
    fn center_selected_language(&mut self, height: i32) {
        if !self.has_selection {
            return;
        }
        let Some(index) = self
            .get_filtered_languages()
            .iter()
            .position(|language| language.code.eq_ignore_ascii_case(&self.selected_code))
        else {
            return;
        };
        let viewport_height = height - HEADER_HEIGHT - FOOTER_HEIGHT;
        if viewport_height <= 0 {
            return;
        }
        let item_center = HEADER_HEIGHT + 2 + index as i32 * ITEM_HEIGHT + ITEM_HEIGHT / 2;
        let viewport_center = HEADER_HEIGHT + viewport_height / 2;
        self.scroll_y = (item_center - viewport_center) as f32;
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_amount(height));
    }
    fn keep_language_visible(&mut self, index: usize, height: i32) {
        let row_top = HEADER_HEIGHT + 2 + index as i32 * ITEM_HEIGHT;
        let row_bottom = row_top + ITEM_HEIGHT;
        let viewport_bottom = height - FOOTER_HEIGHT;
        let visible_row_top = row_top as f32 - self.scroll_y;
        let visible_row_bottom = row_bottom as f32 - self.scroll_y;
        if visible_row_top < HEADER_HEIGHT as f32 {
            self.scroll_y = (row_top - HEADER_HEIGHT) as f32;
        } else if visible_row_bottom > viewport_bottom as f32 {
            self.scroll_y = (row_bottom - viewport_bottom) as f32;
        }
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_amount(height));
    }
    fn select_filtered_language(&mut self, index: usize, height: i32) -> bool {
        let Some(code) = self
            .get_filtered_languages()
            .get(index)
            .map(|language| language.code.clone())
        else {
            return false;
        };
        self.selected_code = code;
        self.has_selection = true;
        self.keep_language_visible(index, height);
        true
    }
    fn move_language_selection(&mut self, down: bool, height: i32) -> bool {
        let filtered = self.get_filtered_languages();
        if filtered.is_empty() {
            self.has_selection = false;
            self.selected_code.clone_from(&self.original_language_code);
            return true;
        }
        let current = self
            .has_selection
            .then(|| {
                filtered
                    .iter()
                    .position(|language| language.code.eq_ignore_ascii_case(&self.selected_code))
            })
            .flatten();
        let next = match (current, down) {
            (Some(index), true) => (index + 1).min(filtered.len() - 1),
            (Some(index), false) => index.saturating_sub(1),
            (None, true) => 0,
            (None, false) => filtered.len() - 1,
        };
        self.select_filtered_language(next, height)
    }
    fn is_double_click(&self, event: MouseButtonEvent, supplied: bool) -> bool {
        supplied
            || self
                .last_mouse_click
                .is_some_and(|(last_time, last_button)| {
                    last_button == event.button() && last_time.elapsed().as_millis() < 250
                })
    }
    fn record_handled_mouse_click(&mut self, event: MouseButtonEvent) {
        self.last_mouse_click = Some((Instant::now(), event.button()));
    }
    fn prepare_custom_mouse_action(&mut self) {
        self.base.set_last_input_keyboard(false);
        self.base.after_mouse_action();
    }
    pub fn mouse_clicked(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        width: i32,
        height: i32,
    ) -> Option<LanguageSelectAction> {
        let event =
            MouseButtonEvent::new(mouse_x as f64, mouse_y as f64, MouseButtonInfo::new(0, 0));
        self.ensure_layout(width, height);
        let handled = Screen::mouse_clicked(self, event, false);
        if handled {
            if let Some(action) = self.take_action() {
                match action {
                    LanguageSelectAction::Done => {
                        click_sound_sink::record_button_click();
                        return Some(LanguageSelectAction::Done);
                    }
                    LanguageSelectAction::FontSettingsClicked => {
                        click_sound_sink::record_button_click();
                        return Some(LanguageSelectAction::FontSettingsClicked);
                    }
                }
            }
        }
        None
    }
    pub fn mouse_released(&mut self) {
        self.dragging_scrollbar = false;
    }
    pub fn key_typed(&mut self, c: char) {
        if self.focus == LanguageSelectFocus::Search
            && self.search_box.char_typed(self.font.as_ref(), c)
        {
            self.sync_search_query();
        }
    }
    pub fn backspace(&mut self) {
        if self.focus == LanguageSelectFocus::Search {
            self.search_box
                .key_pressed(self.font.as_ref(), EditBoxKey::Backspace, false, false);
            self.sync_search_query();
        }
    }
    fn ensure_layout(&mut self, width: i32, height: i32) {
        if self.last_width == width && self.last_height == height {
            return;
        }
        self.last_width = width;
        self.last_height = height;
        self.base.clear_widgets();
        let font_x = Self::font_button_x(width);
        let done_x = Self::done_button_x(width);
        let footer_y = height - FOOTER_BUTTON_Y_OFFSET;
        let search_x = width / 2 - SEARCH_BOX_WIDTH / 2;
        let mut search_geometry_changed = false;
        if self.search_box.x() != search_x || self.search_box.y() != SEARCH_BOX_Y {
            self.search_box
                .set_position(self.font.as_ref(), search_x, SEARCH_BOX_Y);
            search_geometry_changed = true;
        }
        if self.search_box.width() != SEARCH_BOX_WIDTH
            || self.search_box.height() != SEARCH_BOX_HEIGHT
        {
            self.search_box
                .set_size(self.font.as_ref(), SEARCH_BOX_WIDTH, SEARCH_BOX_HEIGHT);
            search_geometry_changed = true;
        }
        if search_geometry_changed
            && self.focus == LanguageSelectFocus::Search
            && let Some(preedit) = self.active_search_preedit.clone()
        {
            self.search_box
                .preedit_updated(self.font.as_ref(), Some(preedit));
        }
        {
            let mut f = self.font_button.borrow_mut();
            f.set_position(font_x, footer_y);
            f.base.set_width(FOOTER_BUTTON_WIDTH);
            f.base.active = true;
        }
        {
            let mut d = self.done_button.borrow_mut();
            d.set_position(done_x, footer_y);
            d.base.set_width(FOOTER_BUTTON_WIDTH);
            d.base.active = true;
        }
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.font_button)));
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.done_button)));
        self.center_selected_language(height);
    }
    fn render_screen(&mut self, graphics: &mut GuiGraphicsExtractor, font: &dyn FontWidthMeasurer) {
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        self.ensure_layout(width, height);
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_amount(height));
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
        let filtered = self.get_filtered_languages();
        for (i, lang) in filtered.iter().enumerate() {
            let item_y = HEADER_HEIGHT + 2 + i as i32 * ITEM_HEIGHT - self.scroll_y as i32;
            let display_str = format!("{} ({})", lang.name, lang.region);
            let is_selected =
                self.has_selection && lang.code.eq_ignore_ascii_case(&self.selected_code);
            if is_selected {
                let outline_color = if self.focus == LanguageSelectFocus::List {
                    0xFFFF_FFFFu32
                } else {
                    0xFF80_8080u32
                };
                graphics.fill(
                    width / 2 - ROW_WIDTH / 2,
                    item_y,
                    width / 2 + ROW_WIDTH / 2,
                    item_y + ITEM_HEIGHT,
                    0xFF00_0000u32 as i32,
                );
                graphics.outline(
                    width / 2 - ROW_WIDTH / 2,
                    item_y,
                    ROW_WIDTH,
                    ITEM_HEIGHT,
                    outline_color as i32,
                );
            }
            let text_color = 0xFFFF_FFFFu32 as i32;
            let text_w = font.measure_width(&display_str, 1.0).round() as i32;
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                display_str,
                width / 2 - text_w / 2,
                item_y + 5,
                text_color,
            );
        }
        graphics.disable_scissor();
        graphics.draw_menu_background(self.in_world, width, HEADER_HEIGHT);
        graphics.pose_mut().push_pose();
        graphics
            .pose_mut()
            .translate(0.0, (height - FOOTER_HEIGHT) as f32);
        graphics.draw_menu_background(self.in_world, width, FOOTER_HEIGHT);
        graphics.pose_mut().pop_pose();
        let title_text = crate::translate("options.language.title");
        let title_w = font.measure_width(&title_text, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            title_text,
            (width - title_w) / 2,
            4,
            0xFFFF_FFFFu32 as i32,
        );
        let mouse_x = graphics.mouse_x();
        let mouse_y = graphics.mouse_y();
        if graphics.contains_point_in_scissor(mouse_x, mouse_y)
            && self.search_box.contains(mouse_x as f64, mouse_y as f64)
        {
            graphics.request_cursor(CursorType::Text);
        }
        let search_plan = self.search_box.render_plan(font, self.now_ms());
        draw_edit_box_plan(graphics, &search_plan);
        let max_scroll = self.max_scroll_amount(height);
        if max_scroll > 0.0 {
            let scrollbar_x = Self::scrollbar_x(width);
            let scroll_area_h = height - HEADER_HEIGHT - FOOTER_HEIGHT;
            let bar_h = self.scrollbar_height(height);
            let bar_y = HEADER_HEIGHT
                + (self.scroll_y * (scroll_area_h - bar_h) as f32 / max_scroll).round() as i32;
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller_background"),
                scrollbar_x,
                HEADER_HEIGHT,
                SCROLLBAR_WIDTH,
                scroll_area_h,
            );
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller"),
                scrollbar_x,
                bar_y,
                SCROLLBAR_WIDTH,
                bar_h,
            );
        }
        let warning_text = crate::translate("options.languageAccuracyWarning");
        let warning_w = font.measure_width(&warning_text, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            warning_text,
            width / 2 - warning_w / 2,
            height - 45,
            WARNING_COLOR,
        );
        let font_x = Self::font_button_x(width);
        let font_y = height - FOOTER_BUTTON_Y_OFFSET;
        graphics.draw_button_with_focus(
            font,
            font_x,
            font_y,
            FOOTER_BUTTON_WIDTH,
            FOOTER_BUTTON_HEIGHT,
            &crate::translate("options.font"),
            true,
            self.focus == LanguageSelectFocus::FontButton,
        );
        let done_x = Self::done_button_x(width);
        let done_y = height - FOOTER_BUTTON_Y_OFFSET;
        graphics.draw_button_with_focus(
            font,
            done_x,
            done_y,
            FOOTER_BUTTON_WIDTH,
            FOOTER_BUTTON_HEIGHT,
            &crate::translate("gui.done"),
            true,
            self.focus == LanguageSelectFocus::DoneButton,
        );
    }
}
impl Screen for LanguageSelectScreen {
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
        self.ensure_layout(width, height);
        self.set_focus(LanguageSelectFocus::Search);
        self.center_selected_language(height);
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if event.is_escape() {
            self.queue_escape();
            return true;
        }
        if event.key == KeyEvent::KEY_TAB {
            self.move_focus(!event.has_shift_down());
            return true;
        }
        let height = self.base_ref().height;
        match self.focus {
            LanguageSelectFocus::Search => {
                if event.key == KeyEvent::KEY_DOWN {
                    self.set_focus(LanguageSelectFocus::List);
                    return true;
                }
                if event.is_select_all() {
                    self.active_search_preedit = None;
                    let consumed = self.search_box.select_all(self.font.as_ref());
                    return consumed;
                }
                if event.is_copy() {
                    if let Some(clipboard) = self.clipboard.clone() {
                        self.search_box.copy(&mut *clipboard.borrow_mut());
                    }
                    return true;
                }
                if event.is_paste() {
                    self.active_search_preedit = None;
                    if let Some(clipboard) = self.clipboard.clone() {
                        self.search_box
                            .paste(self.font.as_ref(), &*clipboard.borrow());
                        self.sync_search_query();
                    }
                    return true;
                }
                if event.is_cut() {
                    self.active_search_preedit = None;
                    if let Some(clipboard) = self.clipboard.clone() {
                        self.search_box
                            .cut(self.font.as_ref(), &mut *clipboard.borrow_mut());
                        self.sync_search_query();
                    }
                    return true;
                }
                let edit_key = match event.key {
                    KeyEvent::KEY_BACKSPACE => Some(EditBoxKey::Backspace),
                    KeyEvent::KEY_DELETE => Some(EditBoxKey::Delete),
                    KeyEvent::KEY_LEFT => Some(EditBoxKey::Left),
                    KeyEvent::KEY_RIGHT => Some(EditBoxKey::Right),
                    KeyEvent::KEY_HOME => Some(EditBoxKey::Home),
                    KeyEvent::KEY_END => Some(EditBoxKey::End),
                    _ => None,
                };
                if let Some(key) = edit_key {
                    self.active_search_preedit = None;
                    let consumed = self.search_box.key_pressed(
                        self.font.as_ref(),
                        key,
                        event.has_control_down(),
                        event.has_shift_down(),
                    );
                    self.sync_search_query();
                    return consumed;
                }
            }
            LanguageSelectFocus::List => match event.key {
                KeyEvent::KEY_UP => {
                    return self.move_language_selection(false, height);
                }
                KeyEvent::KEY_DOWN => {
                    return self.move_language_selection(true, height);
                }
                _ if event.is_selection() && self.has_selection => {
                    click_sound_sink::record_button_click();
                    self.queue_done();
                    return true;
                }
                _ => {}
            },
            LanguageSelectFocus::FontButton if event.is_selection() => {
                click_sound_sink::record_button_click();
                self.pending_action = Some(LanguageSelectAction::FontSettingsClicked);
                return true;
            }
            LanguageSelectFocus::DoneButton if event.is_selection() => {
                click_sound_sink::record_button_click();
                self.queue_done();
                return true;
            }
            _ => {}
        }
        false
    }
    fn char_typed(&mut self, event: CharacterEvent) -> bool {
        if self.focus == LanguageSelectFocus::Search
            && self
                .search_box
                .char_typed(self.font.as_ref(), event.codepoint)
        {
            self.active_search_preedit = None;
            self.sync_search_query();
            return true;
        }
        false
    }
    fn preedit_updated(&mut self, event: Option<PreeditEvent>) -> bool {
        if self.focus == LanguageSelectFocus::Search {
            self.active_search_preedit = event.clone();
            self.search_box.preedit_updated(self.font.as_ref(), event)
        } else {
            false
        }
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        let width = self.base_ref().width;
        let height = self.base_ref().height;
        let double_click = self.is_double_click(event, double_click);
        let scrollbar_x = Self::scrollbar_x(width);
        let bar_h = self.scrollbar_height(height);
        let max_scroll = self.max_scroll_amount(height);
        let bar_y = if max_scroll == 0.0 {
            HEADER_HEIGHT
        } else {
            let scroll_area_h = height - HEADER_HEIGHT - FOOTER_HEIGHT;
            HEADER_HEIGHT
                + (self.scroll_y * (scroll_area_h - bar_h) as f32 / max_scroll).round() as i32
        };
        let (mx, my) = (event.x as i32, event.y as i32);
        if event.button() == 0
            && mx >= scrollbar_x
            && mx < scrollbar_x + SCROLLBAR_WIDTH
            && my >= HEADER_HEIGHT
            && my < height - FOOTER_HEIGHT
        {
            self.prepare_custom_mouse_action();
            self.set_focus(LanguageSelectFocus::List);
            self.dragging_scrollbar = true;
            if my < bar_y {
                self.scroll_y = (self.scroll_y - 18.0).max(0.0);
            } else if my > bar_y + bar_h {
                self.scroll_y = (self.scroll_y + 18.0).min(max_scroll);
            }
            self.record_handled_mouse_click(event);
            return true;
        }
        if event.button() == 0 && self.search_box.contains(event.x, event.y) {
            self.prepare_custom_mouse_action();
            self.set_focus(LanguageSelectFocus::Search);
            self.active_search_preedit = None;
            let shift = event.button_info.modifiers & KeyEvent::MOD_SHIFT != 0;
            self.search_box
                .on_click(self.font.as_ref(), event.x, shift, double_click);
            self.search_dragging = true;
            self.record_handled_mouse_click(event);
            return true;
        }
        if event.button() == 0 && my >= HEADER_HEIGHT && my < height - FOOTER_HEIGHT {
            self.prepare_custom_mouse_action();
            self.set_focus(LanguageSelectFocus::List);
            let filtered = self.get_filtered_languages();
            let click_y = my - HEADER_HEIGHT - 2 + self.scroll_y as i32;
            let item_idx = click_y / ITEM_HEIGHT;
            if item_idx >= 0 && (item_idx as usize) < filtered.len() {
                self.set_focus(LanguageSelectFocus::List);
                self.select_filtered_language(item_idx as usize, height);
                if double_click {
                    self.queue_done();
                }
                click_sound_sink::record_button_click();
                self.record_handled_mouse_click(event);
                return true;
            }
            self.record_handled_mouse_click(event);
            return true;
        }
        let handled = crate::framework::screen::screen_mouse_clicked(self, event, double_click);
        if !handled {
            return false;
        }
        let font_action = { self.font_button.borrow_mut().take_press_action() };
        if let Some(action) = font_action {
            click_sound_sink::record_button_click();
            self.set_focus(LanguageSelectFocus::FontButton);
            self.pending_action = Some(action);
            self.record_handled_mouse_click(event);
            return true;
        }
        let done_action = { self.done_button.borrow_mut().take_press_action() };
        if let Some(action) = done_action {
            click_sound_sink::record_button_click();
            self.set_focus(LanguageSelectFocus::DoneButton);
            if action == LanguageSelectAction::Done {
                self.queue_done();
            } else {
                self.pending_action = Some(action);
            }
            self.record_handled_mouse_click(event);
            return true;
        }
        self.record_handled_mouse_click(event);
        true
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        let handled = crate::framework::screen::screen_mouse_released(self, event);
        self.dragging_scrollbar = false;
        let was_search_dragging = std::mem::take(&mut self.search_dragging);
        handled || was_search_dragging
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        let handled = crate::framework::screen::screen_mouse_dragged(self, event, dx, dy);
        if self.search_dragging && self.focus == LanguageSelectFocus::Search && event.button() == 0
        {
            self.search_box.on_drag(self.font.as_ref(), event.x);
            true
        } else if self.dragging_scrollbar {
            let height = self.base_ref().height;
            let max_scroll = self.max_scroll_amount(height);
            let bar_h = self.scrollbar_height(height);
            let scroll_area_h = height - HEADER_HEIGHT - FOOTER_HEIGHT;
            let y_drag_scale = if scroll_area_h - bar_h > 0 {
                (max_scroll / (scroll_area_h - bar_h) as f32).max(1.0)
            } else {
                1.0
            };
            self.scroll_y = (self.scroll_y + dy as f32 * y_drag_scale).clamp(0.0, max_scroll);
            true
        } else {
            handled
        }
    }
    fn mouse_scrolled(&mut self, _x: f64, _y: f64, _scroll_x: f64, scroll_y: f64) -> bool {
        let height = self.base_ref().height;
        let max_scroll = self.max_scroll_amount(height);
        self.scroll_y = (self.scroll_y - scroll_y as f32 * 12.0).clamp(0.0, max_scroll);
        true
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
        let font = Arc::clone(&self.font);
        self.render_screen(graphics, font.as_ref());
    }
}
fn draw_edit_box_plan(graphics: &mut GuiGraphicsExtractor, plan: &EditBoxRenderPlan) {
    for draw in &plan.draws {
        match draw {
            EditBoxDraw::Sprite {
                sprite,
                x,
                y,
                width,
                height,
            } => graphics.blit(
                ResourceLocation::from_string(sprite),
                *x,
                *y,
                *width,
                *height,
            ),
            EditBoxDraw::Text {
                text,
                x,
                y,
                color,
                shadow,
            } => graphics.text_with_shadow(
                ResourceLocation::from_string("minecraft:default"),
                text.clone(),
                *x,
                *y,
                *color,
                *shadow,
            ),
            EditBoxDraw::Highlight {
                x0,
                y0,
                x1,
                y1,
                invert,
            } => {
                let (left, right) = (*x0.min(x1), *x0.max(x1));
                let (top, bottom) = (*y0.min(y1), *y0.max(y1));
                graphics.text_highlight(left, top, right, bottom, *invert);
            }
            EditBoxDraw::InsertCursor { x, y, color } => graphics.fill(
                *x,
                *y - 1,
                *x + 1,
                *y + EDIT_BOX_INSERT_CURSOR_LINE_HEIGHT,
                *color,
            ),
            EditBoxDraw::AppendCursor {
                x,
                y,
                color,
                shadow,
            } => graphics.text_with_shadow(
                ResourceLocation::from_string("minecraft:default"),
                EditBox::append_cursor_character(),
                *x,
                *y,
                *color,
                *shadow,
            ),
        }
    }
    if let Some(underline) = plan.preedit_underline {
        if underline.x1 > underline.x0 {
            graphics.fill(
                underline.x0,
                underline.y,
                underline.x1,
                underline.y + 1,
                underline.color,
            );
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontSettingsAction {
    Done,
}
#[derive(Clone, Debug)]
pub struct FontSettingsScreen {
    pub force_unicode: bool,
    pub japanese_glyphs: bool,
    pub in_world: bool,
    focused_button: usize,
}
impl FontSettingsScreen {
    pub fn new(in_world: bool, force_unicode: bool, japanese_glyphs: bool) -> Self {
        Self {
            force_unicode,
            japanese_glyphs,
            in_world,
            focused_button: 0,
        }
    }
    pub fn key_pressed(&mut self, event: KeyEvent) -> (bool, Option<FontSettingsAction>) {
        if event.key == KeyEvent::KEY_TAB {
            self.focused_button = if event.has_shift_down() {
                (self.focused_button + 2) % 3
            } else {
                (self.focused_button + 1) % 3
            };
            return (true, None);
        }
        let selected = event.is_selection();
        let direction = match event.key {
            KeyEvent::KEY_LEFT => Some(false),
            KeyEvent::KEY_RIGHT => Some(true),
            _ => None,
        };
        match self.focused_button {
            0 | 1 if selected || direction.is_some() => {
                let target = direction.unwrap_or_else(|| {
                    let current = if self.focused_button == 0 {
                        self.force_unicode
                    } else {
                        self.japanese_glyphs
                    };
                    !current
                });
                if self.focused_button == 0 {
                    self.force_unicode = target;
                } else {
                    self.japanese_glyphs = target;
                }
                crate::components::click_sound_sink::record_button_click();
                (true, None)
            }
            2 if selected => {
                crate::components::click_sound_sink::record_button_click();
                (true, Some(FontSettingsAction::Done))
            }
            _ => (false, None),
        }
    }
    pub fn mouse_clicked(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        width: i32,
        height: i32,
    ) -> Option<FontSettingsAction> {
        let mx = mouse_x.floor() as i32;
        let my = mouse_y.floor() as i32;
        let btn_y = height / 2 - 10;
        let force_x = width / 2 - 155;
        let jap_x = width / 2 + 5;
        if mx >= force_x && mx < force_x + 150 && my >= btn_y && my < btn_y + 20 {
            self.focused_button = 0;
            self.force_unicode = !self.force_unicode;
            crate::components::click_sound_sink::record_button_click();
            return None;
        }
        if mx >= jap_x && mx < jap_x + 150 && my >= btn_y && my < btn_y + 20 {
            self.focused_button = 1;
            self.japanese_glyphs = !self.japanese_glyphs;
            crate::components::click_sound_sink::record_button_click();
            return None;
        }
        let done_x = width / 2 - 100;
        let done_y = height - 30;
        if mx >= done_x && mx < done_x + 200 && my >= done_y && my < done_y + 20 {
            self.focused_button = 2;
            crate::components::click_sound_sink::record_button_click();
            return Some(FontSettingsAction::Done);
        }
        None
    }
    pub fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
    ) {
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        if !self.in_world {
            graphics.panorama();
        }
        graphics.blur_before_this_stratum();
        graphics.next_stratum();
        graphics.draw_menu_background(self.in_world, width, height);
        let title_text = crate::translate("options.font.title");
        let title_w = font.measure_width(&title_text, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            title_text,
            (width - title_w) / 2,
            12,
            0xFFFF_FFFFu32 as i32,
        );
        let btn_y = height / 2 - 10;
        let force_x = width / 2 - 155;
        let jap_x = width / 2 + 5;
        let force_label = crate::translate("options.forceUnicodeFont");
        let force_val = if self.force_unicode {
            crate::translate("options.on")
        } else {
            crate::translate("options.off")
        };
        let force_text = format!("{}: {}", force_label, force_val);
        graphics.draw_button_with_focus(
            font,
            force_x,
            btn_y,
            150,
            20,
            &force_text,
            true,
            self.focused_button == 0,
        );
        let jap_label = crate::translate("options.japaneseGlyphVariants");
        let jap_val = if self.japanese_glyphs {
            crate::translate("options.on")
        } else {
            crate::translate("options.off")
        };
        let jap_text = format!("{}: {}", jap_label, jap_val);
        graphics.draw_button_with_focus(
            font,
            jap_x,
            btn_y,
            150,
            20,
            &jap_text,
            true,
            self.focused_button == 1,
        );
        let done_x = width / 2 - 100;
        let done_y = height - 30;
        graphics.draw_button_with_focus(
            font,
            done_x,
            done_y,
            200,
            20,
            &crate::translate("gui.done"),
            true,
            self.focused_button == 2,
        );
        let mouse_x = graphics.mouse_x();
        let mouse_y = graphics.mouse_y();
        if let Some(lines) = crate::settings::option_tooltips::font_tooltip("japaneseGlyphVariants")
            && ScreenRectangle::new(jap_x, btn_y, 150, 20).contains_point(mouse_x, mouse_y)
        {
            graphics.draw_widget_tooltip(
                font,
                &lines,
                mouse_x,
                mouse_y,
                ScreenRectangle::new(jap_x, btn_y, 150, 20),
                WidgetTooltipPosition::Menu,
            );
        }
    }
}
