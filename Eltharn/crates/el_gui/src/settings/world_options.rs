use crate::FontWidthMeasurer;
use crate::components::cycle_button::CycleButton;
use crate::components::scroll_area::{AbstractScrollArea, ScrollbarSettings};
use crate::components::sprites::{self, sprites as sprite_sets};
use crate::components::text_widget::{LINE_HEIGHT, StringWidget};
use crate::components::{
    Button, DEFAULT_HEIGHT, DEFAULT_WIDTH, GuiEventListener, Renderable, TabOrderedElement,
    WidgetClickSound,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::{
    FrameLayout, GridLayout, HeaderAndFooterLayout, Layout, LayoutElement,
};
use crate::framework::narration::{
    NarratableEntry, NarratedElementType, NarrationElementOutput, NarrationPriority,
};
use crate::framework::navigation::{
    ComponentPath, FocusNavigationEvent, ScreenDirection, ScreenRectangle,
};
use crate::framework::screen::{Screen, ScreenBase, screen_init};
use crate::input::events::{CursorType, KeyEvent, MouseButtonEvent, MouseButtonInfo};
use el_core::Component;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
pub const TITLE_KEY: &str = "options.worldOptions.title";
pub const GAME_RULES_KEY: &str = "editGamerule.inGame.button";
pub const GAME_MODE_KEY: &str = "selectWorld.gameMode";
pub const ALLOW_COMMANDS_KEY: &str = "selectWorld.allowCommands";
pub const RESTRICTIONS_KEY: &str = "restrictions_screen.button";
pub const DONE_KEY: &str = "gui.done";
pub const DIFFICULTY_KEY: &str = "options.difficulty";
pub const LOCK_NARRATION_KEY: &str = "narrator.button.difficulty_lock";
pub const LOCK_NARRATION_LOCKED_KEY: &str = "narrator.button.difficulty_lock.locked";
pub const LOCK_NARRATION_UNLOCKED_KEY: &str = "narrator.button.difficulty_lock.unlocked";
pub const COLUMN_SPACING: i32 = 8;
pub const ROW_SPACING: i32 = 4;
pub const COLUMNS: usize = 2;
pub const LOCK_WIDTH: i32 = 20;
pub const DIFFICULTY_WIDTH: i32 = DEFAULT_WIDTH - LOCK_WIDTH;
pub const DONE_WIDTH: i32 = 200;
pub const BAND_HEIGHT: i32 = 33;
pub const CONTENT_MARGIN_TOP: i32 = 30;
pub const CONTENT_TOP: i32 = BAND_HEIGHT + CONTENT_MARGIN_TOP;
pub const GRID_WIDTH: i32 = DEFAULT_WIDTH + COLUMN_SPACING + DEFAULT_WIDTH;
pub const CONTENT_HEIGHT: i32 = 3 * DEFAULT_HEIGHT + 2 * ROW_SPACING;
pub const SCROLL_RATE: i32 = 12;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}
impl Difficulty {
    pub const VALUES: [Self; 4] = [Self::Peaceful, Self::Easy, Self::Normal, Self::Hard];
    pub fn id(self) -> i32 {
        match self {
            Self::Peaceful => 0,
            Self::Easy => 1,
            Self::Normal => 2,
            Self::Hard => 3,
        }
    }
    pub fn from_id(id: i32) -> Self {
        match id {
            0 => Self::Peaceful,
            1 => Self::Easy,
            3 => Self::Hard,
            _ => Self::Normal,
        }
    }
    pub fn name_key(self) -> &'static str {
        match self {
            Self::Peaceful => "options.difficulty.peaceful",
            Self::Easy => "options.difficulty.easy",
            Self::Normal => "options.difficulty.normal",
            Self::Hard => "options.difficulty.hard",
        }
    }
    pub fn display_name(self) -> Component {
        Component::literal(&crate::translate(self.name_key()))
    }
    pub fn world_name(self) -> &'static str {
        match self {
            Self::Peaceful => "Peaceful",
            Self::Easy => "Easy",
            Self::Normal => "Normal",
            Self::Hard => "Hard",
        }
    }
    pub fn from_world_name(name: &str) -> Self {
        match name {
            "Peaceful" => Self::Peaceful,
            "Easy" => Self::Easy,
            "Hard" => Self::Hard,
            _ => Self::Normal,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
    Spectator,
}
impl GameMode {
    pub const CYCLE_VALUES: [Self; 4] = [
        Self::Survival,
        Self::Spectator,
        Self::Creative,
        Self::Adventure,
    ];
    pub fn id(self) -> i32 {
        match self {
            Self::Survival => 0,
            Self::Creative => 1,
            Self::Adventure => 2,
            Self::Spectator => 3,
        }
    }
    pub fn from_id(id: i32) -> Self {
        match id {
            1 => Self::Creative,
            2 => Self::Adventure,
            3 => Self::Spectator,
            _ => Self::Survival,
        }
    }
    pub fn name_key(self) -> &'static str {
        match self {
            Self::Survival => "selectWorld.gameMode.survival",
            Self::Creative => "selectWorld.gameMode.creative",
            Self::Adventure => "selectWorld.gameMode.adventure",
            Self::Spectator => "selectWorld.gameMode.spectator",
        }
    }
    pub fn display_name(self) -> Component {
        Component::literal(&crate::translate(self.name_key()))
    }
    pub fn world_name(self) -> &'static str {
        match self {
            Self::Survival => "Survival",
            Self::Creative => "Creative",
            Self::Adventure => "Adventure",
            Self::Spectator => "Spectator",
        }
    }
    pub fn from_world_name(name: &str) -> Self {
        match name {
            "Creative" => Self::Creative,
            "Adventure" => Self::Adventure,
            "Spectator" => Self::Spectator,
            _ => Self::Survival,
        }
    }
}
#[derive(Clone, Debug)]
pub struct WorldOptionsOption {
    pub key: String,
    pub label: String,
    pub is_slider: bool,
    pub min_val: f32,
    pub max_val: f32,
    pub step: f32,
    pub value: f32,
}
impl WorldOptionsOption {
    pub fn get_display_string(&self) -> String {
        let name = crate::translate(&self.label);
        match self.key.as_str() {
            "gameRules" => name,
            "difficulty" => {
                let val = self.value.round() as i32;
                let mode = match val {
                    0 => crate::translate("options.difficulty.peaceful"),
                    1 => crate::translate("options.difficulty.easy"),
                    2 => crate::translate("options.difficulty.normal"),
                    3 => crate::translate("options.difficulty.hard"),
                    _ => "".to_string(),
                };
                format!("{}: {}", name, mode)
            }
            "gameMode" => {
                let val = self.value.round() as i32;
                let mode = match val {
                    0 => crate::translate("selectWorld.gameMode.survival"),
                    1 => crate::translate("selectWorld.gameMode.creative"),
                    2 => crate::translate("selectWorld.gameMode.adventure"),
                    3 => crate::translate("selectWorld.gameMode.spectator"),
                    _ => "".to_string(),
                };
                format!("{}: {}", name, mode)
            }
            "allowCommands" => {
                let state = if self.value > 0.5 {
                    crate::translate("options.on")
                } else {
                    crate::translate("options.off")
                };
                format!("{}: {}", name, state)
            }
            "restrictions" => name,
            _ => format!("{}: {}", name, self.value),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldOptionsAction {
    Done,
    GameRulesClicked,
    RestrictionsClicked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LockAction {
    Lock,
}
type CycleCell<T> = Option<Rc<RefCell<CycleButton<T>>>>;
#[derive(Clone, Debug, Default)]
struct WidgetHandles {
    title: usize,
    row_cells: Vec<RowCell>,
    lock: Option<usize>,
    done: usize,
}
#[derive(Clone, Copy, Debug)]
struct RowCell {
    handle: usize,
    width: i32,
}
pub struct WorldOptionsScreen {
    base: ScreenBase,
    font: Arc<dyn FontWidthMeasurer>,
    pub last_width: i32,
    pub last_height: i32,
    pub options: Vec<WorldOptionsOption>,
    pub in_world: bool,
    pub difficulty_locked: bool,
    has_singleplayer_server: bool,
    game_rules: Rc<RefCell<Button<WorldOptionsAction>>>,
    difficulty: CycleCell<Difficulty>,
    lock: Rc<RefCell<Button<LockAction>>>,
    lock_locked: Rc<Cell<bool>>,
    game_mode: CycleCell<GameMode>,
    allow_commands: CycleCell<bool>,
    restrictions: Rc<RefCell<Button<WorldOptionsAction>>>,
    done: Rc<RefCell<Button<WorldOptionsAction>>>,
    handles: WidgetHandles,
    layout: HeaderAndFooterLayout,
    grid: GridLayout,
    scroll: AbstractScrollArea,
}
impl WorldOptionsScreen {
    pub fn new(
        in_world: bool,
        extra_options: &HashMap<String, String>,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        Self::new_with_server_context(in_world, extra_options, true, font)
    }
    pub fn new_with_server_context(
        in_world: bool,
        extra_options: &HashMap<String, String>,
        has_singleplayer_server: bool,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let options = Self::build_options(extra_options, has_singleplayer_server);
        let difficulty_locked = Self::read_difficulty_locked(extra_options);
        let initial_difficulty = Difficulty::from_world_name(
            extra_options
                .get("difficulty")
                .map_or("Normal", String::as_str),
        );
        let initial_game_mode = GameMode::from_world_name(
            extra_options
                .get("gameMode")
                .map_or("Survival", String::as_str),
        );
        let initial_allow_commands = extra_options
            .get("allowCommands")
            .is_some_and(|value| value == "true");
        let game_rules =
            Self::build_action_button(WorldOptionsAction::GameRulesClicked, GAME_RULES_KEY, &font);
        let difficulty = Self::build_cycle_button(
            Difficulty::VALUES.to_vec(),
            initial_difficulty,
            |value: &Difficulty| Component::literal(&crate::translate(value.name_key())),
            DIFFICULTY_KEY,
            DIFFICULTY_WIDTH,
        )
        .map(|button| Rc::new(RefCell::new(button)));
        let lock = Rc::new(RefCell::new(
            Button::<LockAction>::builder(Component::literal(&crate::translate(
                LOCK_NARRATION_KEY,
            )))
            .bounds(0, 0, LOCK_WIDTH, LOCK_WIDTH)
            .with_action(LockAction::Lock)
            .with_font(Arc::clone(&font))
            .build(),
        ));
        let game_mode = has_singleplayer_server
            .then(|| {
                Self::build_cycle_button(
                    GameMode::CYCLE_VALUES.to_vec(),
                    initial_game_mode,
                    |value: &GameMode| Component::literal(&crate::translate(value.name_key())),
                    GAME_MODE_KEY,
                    DEFAULT_WIDTH,
                )
                .map(|button| Rc::new(RefCell::new(button)))
            })
            .flatten();
        let allow_commands = has_singleplayer_server
            .then(|| {
                CycleButton::<bool>::on_off_builder(initial_allow_commands)
                    .create_at(
                        0,
                        0,
                        DEFAULT_WIDTH,
                        DEFAULT_HEIGHT,
                        Component::literal(&crate::translate(ALLOW_COMMANDS_KEY)),
                    )
                    .map(|button| Rc::new(RefCell::new(button)))
            })
            .flatten();
        let restrictions = Self::build_action_button(
            WorldOptionsAction::RestrictionsClicked,
            RESTRICTIONS_KEY,
            &font,
        );
        let done = Rc::new(RefCell::new(
            Button::builder(Component::literal(&crate::translate(DONE_KEY)))
                .bounds(0, 0, DONE_WIDTH, DEFAULT_HEIGHT)
                .with_action(WorldOptionsAction::Done)
                .with_font(Arc::clone(&font))
                .build(),
        ));
        let mut screen = Self {
            base: ScreenBase::new(Component::literal(&crate::translate(TITLE_KEY))),
            font,
            last_width: 0,
            last_height: 0,
            options,
            in_world,
            difficulty_locked,
            has_singleplayer_server,
            game_rules,
            difficulty,
            lock,
            lock_locked: Rc::new(Cell::new(difficulty_locked)),
            game_mode,
            allow_commands,
            restrictions,
            done,
            handles: WidgetHandles::default(),
            layout: HeaderAndFooterLayout::new(0, 0),
            grid: GridLayout::new(0, 0),
            scroll: AbstractScrollArea::new(
                0,
                0,
                GRID_WIDTH,
                0,
                Component::literal(""),
                ScrollbarSettings::default_settings(SCROLL_RATE),
            ),
        };
        screen.push_option_rows();
        screen.apply_difficulty_gate();
        screen
    }
    pub fn can_edit_world_data(&self) -> bool {
        self.has_singleplayer_server
    }
    fn build_options(
        extra_options: &HashMap<String, String>,
        has_singleplayer_server: bool,
    ) -> Vec<WorldOptionsOption> {
        let difficulty_val = extra_options
            .get("difficulty")
            .map_or(2.0, |name| Difficulty::from_world_name(name).id() as f32);
        let game_mode_val = extra_options
            .get("gameMode")
            .map_or(0.0, |name| GameMode::from_world_name(name).id() as f32);
        let allow_commands_val = extra_options
            .get("allowCommands")
            .is_some_and(|value| value == "true");
        let mut options = vec![
            WorldOptionsOption {
                key: "gameRules".to_string(),
                label: GAME_RULES_KEY.to_string(),
                is_slider: false,
                min_val: 0.0,
                max_val: 1.0,
                step: 1.0,
                value: 0.0,
            },
            WorldOptionsOption {
                key: "difficulty".to_string(),
                label: DIFFICULTY_KEY.to_string(),
                is_slider: false,
                min_val: 0.0,
                max_val: 3.0,
                step: 1.0,
                value: difficulty_val,
            },
            WorldOptionsOption {
                key: "restrictions".to_string(),
                label: RESTRICTIONS_KEY.to_string(),
                is_slider: false,
                min_val: 0.0,
                max_val: 1.0,
                step: 1.0,
                value: 0.0,
            },
        ];
        if has_singleplayer_server {
            options.splice(
                2..2,
                [
                    WorldOptionsOption {
                        key: "gameMode".to_string(),
                        label: GAME_MODE_KEY.to_string(),
                        is_slider: false,
                        min_val: 0.0,
                        max_val: 3.0,
                        step: 1.0,
                        value: game_mode_val,
                    },
                    WorldOptionsOption {
                        key: "allowCommands".to_string(),
                        label: ALLOW_COMMANDS_KEY.to_string(),
                        is_slider: false,
                        min_val: 0.0,
                        max_val: 1.0,
                        step: 1.0,
                        value: if allow_commands_val { 1.0 } else { 0.0 },
                    },
                ],
            );
        }
        options
    }
    fn read_difficulty_locked(extra_options: &HashMap<String, String>) -> bool {
        extra_options
            .get("difficultyLocked")
            .is_some_and(|value| value == "true")
    }
    fn build_action_button(
        action: WorldOptionsAction,
        label_key: &str,
        font: &Arc<dyn FontWidthMeasurer>,
    ) -> Rc<RefCell<Button<WorldOptionsAction>>> {
        Rc::new(RefCell::new(
            Button::builder(Component::literal(&crate::translate(label_key)))
                .with_action(action)
                .with_font(Arc::clone(font))
                .build(),
        ))
    }
    fn build_cycle_button<T: Clone + PartialEq + 'static>(
        values: Vec<T>,
        initial: T,
        stringify: impl Fn(&T) -> Component + 'static,
        name: &str,
        width: i32,
    ) -> Option<CycleButton<T>> {
        CycleButton::<T>::builder(stringify, move || initial.clone())
            .with_values(values)
            .create_at(
                0,
                0,
                width,
                DEFAULT_HEIGHT,
                Component::literal(&crate::translate(name)),
            )
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn options(&self) -> &[WorldOptionsOption] {
        &self.options
    }
    pub fn take_action(&mut self) -> Option<WorldOptionsAction> {
        self.sync_state();
        for cell in [&self.game_rules, &self.restrictions, &self.done] {
            if let Some(action) = Self::drain_action(cell) {
                return Some(action);
            }
        }
        None
    }
    pub fn refresh(&mut self) {
        self.push_option_rows();
        self.apply_difficulty_gate();
    }
    pub fn on_difficulty_changed(&mut self) {
        self.refresh();
    }
    pub fn mouse_clicked(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        width: i32,
        height: i32,
    ) -> Option<WorldOptionsAction> {
        self.ensure_initialized(width, height);
        let event =
            MouseButtonEvent::new(mouse_x as f64, mouse_y as f64, MouseButtonInfo::new(0, 0));
        let _ = Screen::mouse_clicked(self, event, false);
        self.take_action()
    }
    pub fn mouse_released(&mut self) {
        let event = MouseButtonEvent::new(0.0, 0.0, MouseButtonInfo::new(0, 0));
        let _ = Screen::mouse_released(self, event);
    }
    pub fn mouse_dragged(&mut self, mouse_x: f32, mouse_y: f32, dy: f32, width: i32, height: i32) {
        self.ensure_initialized(width, height);
        let event =
            MouseButtonEvent::new(mouse_x as f64, mouse_y as f64, MouseButtonInfo::new(0, 0));
        let _ = Screen::mouse_dragged(self, event, 0.0, dy as f64);
    }
    pub fn mouse_scrolled(&mut self, scroll_y: f32, _height: i32) {
        self.scroll.scroll(scroll_y, self.scroll.viewport());
        self.arrange_grid();
    }
    pub fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        _font: &dyn FontWidthMeasurer,
    ) {
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        self.ensure_initialized(width, height);
        let mouse_x = graphics.mouse_x();
        let mouse_y = graphics.mouse_y();
        Screen::extract_render_state(self, graphics, mouse_x, mouse_y, 0.0);
    }
    fn init_widgets(&mut self) {
        if self.base.initialized() {
            self.arrange();
            return;
        }
        let message = Component::literal(&crate::translate(TITLE_KEY));
        let mut title = StringWidget::new(0, 0, 0, LINE_HEIGHT, message.clone())
            .with_font(Arc::clone(&self.font));
        title.set_message(message);
        self.handles.title = self.base.add_widget(title);
        let mut row_cells = Vec::with_capacity(self.options.len());
        row_cells.push(RowCell {
            handle: self.add_cell(Rc::clone(&self.game_rules)),
            width: DEFAULT_WIDTH,
        });
        if let Some(handle) = self.add_cycle_cell(self.difficulty.as_ref().map(Rc::clone)) {
            row_cells.push(RowCell {
                handle,
                width: DIFFICULTY_WIDTH,
            });
            self.handles.lock = Some(self.base.add_widget(LockButtonView::new(
                Rc::clone(&self.lock),
                Rc::clone(&self.lock_locked),
            )));
        }
        if let Some(handle) = self.add_cycle_cell(self.game_mode.as_ref().map(Rc::clone)) {
            row_cells.push(RowCell {
                handle,
                width: DEFAULT_WIDTH,
            });
        }
        if let Some(handle) = self.add_cycle_cell(self.allow_commands.as_ref().map(Rc::clone)) {
            row_cells.push(RowCell {
                handle,
                width: DEFAULT_WIDTH,
            });
        }
        row_cells.push(RowCell {
            handle: self.add_cell(Rc::clone(&self.restrictions)),
            width: DEFAULT_WIDTH,
        });
        self.handles.row_cells = row_cells;
        self.handles.done = self.base.add_widget(Shared::new(Rc::clone(&self.done)));
        self.arrange();
        self.apply_difficulty_gate();
    }
    fn add_cell<W>(&mut self, cell: Rc<RefCell<W>>) -> usize
    where
        W: Renderable + GuiEventListener + NarratableEntry + LayoutElement + 'static,
    {
        self.base.add_widget(Shared::new(cell))
    }
    fn add_cycle_cell<T: Clone + PartialEq + 'static>(
        &mut self,
        cell: Option<Rc<RefCell<CycleButton<T>>>>,
    ) -> Option<usize> {
        let cell = cell?;
        Some(self.add_cell(cell))
    }
    fn ensure_initialized(&mut self, width: i32, height: i32) {
        if self.base.initialized() && self.base.width == width && self.base.height == height {
            return;
        }
        screen_init(self, width, height);
        self.last_width = width;
        self.last_height = height;
    }
    fn drain_action(cell: &Rc<RefCell<Button<WorldOptionsAction>>>) -> Option<WorldOptionsAction> {
        match cell.try_borrow_mut() {
            Ok(mut button) => button.take_press_action(),
            Err(_) => None,
        }
    }
    fn cycle_value<T: Clone + PartialEq, R>(
        cell: &CycleCell<T>,
        read: impl FnOnce(&T) -> R,
    ) -> Option<R> {
        let cell = cell.as_ref()?;
        let button = cell.try_borrow().ok()?;
        Some(read(button.value()))
    }
    fn row_value(options: &[WorldOptionsOption], key: &str) -> f32 {
        options
            .iter()
            .find(|row| row.key == key)
            .map_or(0.0, |row| row.value)
    }
    fn set_row_value(options: &mut [WorldOptionsOption], key: &str, value: f32) {
        if let Some(row) = options.iter_mut().find(|row| row.key == key) {
            row.value = value;
        }
    }
    fn sync_option_rows(&mut self) {
        if let Some(id) = Self::cycle_value(&self.difficulty, |value| value.id()) {
            Self::set_row_value(&mut self.options, "difficulty", id as f32);
        }
        if let Some(id) = Self::cycle_value(&self.game_mode, |value| value.id()) {
            Self::set_row_value(&mut self.options, "gameMode", id as f32);
        }
        if let Some(on) = Self::cycle_value(&self.allow_commands, |value| *value) {
            Self::set_row_value(
                &mut self.options,
                "allowCommands",
                if on { 1.0 } else { 0.0 },
            );
        }
    }
    fn push_option_rows(&mut self) {
        let difficulty =
            Difficulty::from_id(Self::row_value(&self.options, "difficulty").round() as i32);
        if let Some(cell) = &self.difficulty {
            if let Ok(mut button) = cell.try_borrow_mut() {
                button.set_value(difficulty);
            }
        }
        let game_mode =
            GameMode::from_id(Self::row_value(&self.options, "gameMode").round() as i32);
        if let Some(cell) = &self.game_mode {
            if let Ok(mut button) = cell.try_borrow_mut() {
                button.set_value(game_mode);
            }
        }
        let allow_commands = Self::row_value(&self.options, "allowCommands") > 0.5;
        if let Some(cell) = &self.allow_commands {
            if let Ok(mut button) = cell.try_borrow_mut() {
                button.set_value(allow_commands);
            }
        }
    }
    fn drain_lock(&mut self) {
        let pressed = match self.lock.try_borrow_mut() {
            Ok(mut button) => button.take_press_action(),
            Err(_) => None,
        };
        if pressed == Some(LockAction::Lock) {
            self.difficulty_locked = true;
        }
        self.apply_difficulty_gate();
    }
    fn sync_state(&mut self) {
        self.drain_lock();
        self.sync_option_rows();
    }
    fn apply_difficulty_gate(&mut self) {
        let active = self.has_singleplayer_server && !self.difficulty_locked;
        if let Some(cell) = &self.difficulty {
            if let Ok(mut button) = cell.try_borrow_mut() {
                button.base.active = active;
            }
        }
        if let Ok(mut button) = self.lock.try_borrow_mut() {
            button.base.active = active;
        }
        self.lock_locked.set(self.difficulty_locked);
    }
    fn title_width(&self) -> i32 {
        match self.base.widgets().get(self.handles.title) {
            Some(widget) => LayoutElement::width(&**widget),
            None => 0,
        }
    }
    fn viewport_height(height: i32) -> i32 {
        (height - CONTENT_TOP - BAND_HEIGHT).max(0)
    }
    fn arrange(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        self.last_width = width;
        self.last_height = height;
        let mut layout = HeaderAndFooterLayout::new(width, height);
        layout.add_to_header_handle(self.handles.title, self.title_width(), LINE_HEIGHT);
        layout.add_to_footer_handle(self.handles.done, DONE_WIDTH, DEFAULT_HEIGHT);
        {
            let band_handles = [self.handles.title, self.handles.done];
            let mut bands = self
                .base
                .widgets_mut()
                .iter_mut()
                .enumerate()
                .filter(|(index, _)| band_handles.contains(index))
                .map(|(index, widget)| (index, widget.as_mut() as &mut dyn LayoutElement));
            layout.arrange_widgets(&mut bands);
        }
        self.layout = layout;
        self.arrange_grid();
    }
    fn arrange_grid(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        let mut grid = GridLayout::new(0, 0);
        grid.column_spacing(COLUMN_SPACING).row_spacing(ROW_SPACING);
        {
            let mut helper = grid.create_row_helper(COLUMNS);
            for cell in &self.handles.row_cells {
                helper.add_handle(cell.handle, cell.width, DEFAULT_HEIGHT);
            }
        }
        grid.arrange_elements();
        let viewport = Self::viewport_height(height);
        self.scroll.content_height = grid.height();
        self.scroll.base.set_width(grid.width());
        self.scroll.arrange(
            FrameLayout::align_in_dimension(0, width, grid.width(), 0.5),
            CONTENT_TOP,
            viewport,
        );
        let content_x = FrameLayout::align_in_dimension(0, width, grid.width(), 0.5);
        let content_y = CONTENT_TOP - self.scroll.scroll_amount as i32;
        grid.set_position(content_x, content_y);
        let cells: Vec<(usize, i32, i32)> = self
            .handles
            .row_cells
            .iter()
            .map(|cell| (cell.handle, cell.width, DEFAULT_HEIGHT))
            .collect();
        {
            let mut rows = self
                .base
                .widgets_mut()
                .iter_mut()
                .enumerate()
                .filter(|(index, _)| cells.iter().any(|(handle, _, _)| handle == index))
                .map(|(index, widget)| (index, widget.as_mut() as &mut dyn LayoutElement));
            grid.arrange_widgets(&mut rows);
        }
        self.grid = grid;
        if let Some(lock) = self.handles.lock {
            if self.difficulty.is_some() {
                let lock_x = content_x + DEFAULT_WIDTH + COLUMN_SPACING + DIFFICULTY_WIDTH;
                if let Some(widget) = self.base.widgets_mut().get_mut(lock) {
                    LayoutElement::set_position(&mut **widget, lock_x, content_y);
                }
            }
        }
    }
}
impl std::fmt::Debug for WorldOptionsScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorldOptionsScreen")
            .field("in_world", &self.in_world)
            .field("difficulty_locked", &self.difficulty_locked)
            .field("has_singleplayer_server", &self.has_singleplayer_server)
            .field("options", &self.options)
            .field("width", &self.base.width)
            .field("height", &self.base.height)
            .field("widgets", &self.base.widgets_len())
            .field("focused", &self.base.focused_index())
            .field("scroll_amount", &self.scroll.scroll_amount)
            .field("content_height", &self.scroll.content_height)
            .finish()
    }
}
impl Screen for WorldOptionsScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        self.init_widgets();
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        let handled = crate::framework::screen::screen_key_pressed(self, event);
        self.sync_state();
        handled
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        let on_scrollbar = self.scroll.scrollbar_clicked_with_button(
            event.x,
            event.y,
            event.button(),
            self.scroll.height_for_viewport(),
        );
        if on_scrollbar {
            self.sync_state();
            return true;
        }
        let handled = crate::framework::screen::screen_mouse_clicked(self, event, double_click);
        self.sync_state();
        handled
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        let handled =
            crate::framework::screen::screen_mouse_scrolled(self, x, y, scroll_x, scroll_y);
        self.arrange_grid();
        self.sync_state();
        handled
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        if self.scroll.is_scrolling() {
            self.scroll
                .scrollbar_dragged(event.y, dy, self.scroll.height_for_viewport());
            self.arrange_grid();
            return true;
        }
        let handled = crate::framework::screen::screen_mouse_dragged(self, event, dx, dy);
        self.arrange_grid();
        handled
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        if self.scroll.is_scrolling() {
            self.scroll.scrollbar_released();
            return true;
        }
        crate::framework::screen::screen_mouse_released(self, event)
    }
    fn reposition_elements(&mut self) {
        self.arrange();
    }
    fn added(&mut self) {
        self.refresh();
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        self.base.extract_background_layers(graphics, self.in_world);
        let scissor = self.scroll.content_scissor();
        if let Some(rect) = scissor {
            graphics.enable_scissor(rect.left(), rect.top(), rect.right(), rect.bottom());
        }
        let painted: Vec<usize> = self
            .handles
            .row_cells
            .iter()
            .map(|cell| cell.handle)
            .chain(self.handles.lock)
            .collect();
        for handle in &painted {
            if let Some(widget) = self.base.widgets_mut().get_mut(*handle) {
                widget.extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
            }
        }
        if scissor.is_some() {
            graphics.disable_scissor();
        }
        self.scroll.extract_scrollbar(graphics);
        for (index, widget) in self.base.widgets_mut().iter_mut().enumerate() {
            if painted.contains(&index) {
                continue;
            }
            widget.extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
        }
    }
}
struct Shared<W> {
    inner: Rc<RefCell<W>>,
}
impl<W> Shared<W> {
    fn new(inner: Rc<RefCell<W>>) -> Self {
        Self { inner }
    }
    fn with_inner<T>(&self, apply: impl FnOnce(&W) -> T) -> Option<T> {
        self.inner.try_borrow().ok().map(|inner| apply(&inner))
    }
    fn with_inner_mut<T>(&mut self, apply: impl FnOnce(&mut W) -> T) -> Option<T> {
        self.inner
            .try_borrow_mut()
            .ok()
            .map(|mut inner| apply(&mut inner))
    }
}
impl<W: Renderable> Renderable for Shared<W> {
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        if let Ok(mut inner) = self.inner.try_borrow_mut() {
            inner.extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
        }
    }
}
impl<W: TabOrderedElement> TabOrderedElement for Shared<W> {
    fn tab_order_group(&self) -> i32 {
        self.with_inner(TabOrderedElement::tab_order_group)
            .unwrap_or(0)
    }
}
impl<W: GuiEventListener> GuiEventListener for Shared<W> {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.with_inner_mut(|inner| inner.mouse_clicked(event, double_click))
            .unwrap_or(false)
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.with_inner(GuiEventListener::play_down_sound).flatten()
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        self.with_inner_mut(|inner| inner.mouse_released(event))
            .unwrap_or(false)
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        self.with_inner_mut(|inner| inner.mouse_dragged(event, dx, dy))
            .unwrap_or(false)
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        self.with_inner_mut(|inner| inner.mouse_scrolled(x, y, scroll_x, scroll_y))
            .unwrap_or(false)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        self.with_inner_mut(|inner| inner.key_pressed(event))
            .unwrap_or(false)
    }
    fn next_focus_path(&self, navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.with_inner(|inner| GuiEventListener::next_focus_path(inner, navigation_event))
            .flatten()
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.with_inner(|inner| GuiEventListener::is_mouse_over(inner, mouse_x, mouse_y))
            .unwrap_or(false)
    }
    fn set_focused(&mut self, focused: bool) {
        self.with_inner_mut(|inner| inner.set_focused(focused));
    }
    fn is_focused(&self) -> bool {
        self.with_inner(GuiEventListener::is_focused)
            .unwrap_or(false)
    }
    fn should_take_focus_after_interaction(&self) -> bool {
        self.with_inner(GuiEventListener::should_take_focus_after_interaction)
            .unwrap_or(true)
    }
    fn rectangle(&self) -> ScreenRectangle {
        self.with_inner(GuiEventListener::rectangle)
            .unwrap_or_else(ScreenRectangle::empty)
    }
    fn border_for_arrow_navigation(&self, opposite: ScreenDirection) -> ScreenRectangle {
        self.with_inner(|inner| GuiEventListener::border_for_arrow_navigation(inner, opposite))
            .unwrap_or_else(ScreenRectangle::empty)
    }
}
impl<W: NarratableEntry> NarratableEntry for Shared<W> {
    fn narration_priority(&self) -> NarrationPriority {
        self.with_inner(NarratableEntry::narration_priority)
            .unwrap_or(NarrationPriority::None)
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        if let Ok(inner) = self.inner.try_borrow() {
            inner.update_narration(output);
        }
    }
}
impl<W: LayoutElement> LayoutElement for Shared<W> {
    fn set_x(&mut self, x: i32) {
        self.with_inner_mut(|inner| inner.set_x(x));
    }
    fn set_y(&mut self, y: i32) {
        self.with_inner_mut(|inner| inner.set_y(y));
    }
    fn x(&self) -> i32 {
        self.with_inner(LayoutElement::x).unwrap_or(0)
    }
    fn y(&self) -> i32 {
        self.with_inner(LayoutElement::y).unwrap_or(0)
    }
    fn width(&self) -> i32 {
        self.with_inner(LayoutElement::width).unwrap_or(0)
    }
    fn height(&self) -> i32 {
        self.with_inner(LayoutElement::height).unwrap_or(0)
    }
}
struct LockButtonView {
    inner: Rc<RefCell<Button<LockAction>>>,
    locked: Rc<Cell<bool>>,
}
impl LockButtonView {
    fn new(inner: Rc<RefCell<Button<LockAction>>>, locked: Rc<Cell<bool>>) -> Self {
        Self { inner, locked }
    }
}
impl Renderable for LockButtonView {
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        _partial_tick: f32,
    ) {
        let mut inner = match self.inner.try_borrow_mut() {
            Ok(inner) => inner,
            Err(_) => return,
        };
        let hovered = graphics.contains_point_in_scissor(mouse_x, mouse_y)
            && inner
                .base
                .are_coordinates_in_rectangle(mouse_x as f64, mouse_y as f64);
        inner.base.set_hovered(hovered);
        if !inner.base.visible {
            return;
        }
        let set = if self.locked.get() {
            sprite_sets::LOCK_BUTTON
        } else {
            sprite_sets::UNLOCK_BUTTON
        };
        let path = set.get(inner.base.active, inner.is_hovered_or_focused());
        graphics.blit(
            sprites::widget_sprite(path),
            inner.base.x(),
            inner.base.y(),
            inner.base.width(),
            inner.base.height(),
        );
        if hovered {
            graphics.request_cursor(if inner.base.is_active() {
                CursorType::PointingHand
            } else {
                CursorType::NotAllowed
            });
        }
    }
}
impl TabOrderedElement for LockButtonView {
    fn tab_order_group(&self) -> i32 {
        match self.inner.try_borrow() {
            Ok(inner) => TabOrderedElement::tab_order_group(&*inner),
            Err(_) => 0,
        }
    }
}
impl GuiEventListener for LockButtonView {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        match self.inner.try_borrow_mut() {
            Ok(mut inner) => inner.mouse_clicked(event, _double_click),
            Err(_) => false,
        }
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        match self.inner.try_borrow_mut() {
            Ok(mut inner) => inner.key_pressed(event),
            Err(_) => false,
        }
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        match self.inner.try_borrow() {
            Ok(inner) => GuiEventListener::play_down_sound(&*inner),
            Err(_) => None,
        }
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        match self.inner.try_borrow_mut() {
            Ok(mut inner) => inner.mouse_released(event),
            Err(_) => false,
        }
    }
    fn mouse_scrolled(&mut self, x: f64, y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        match self.inner.try_borrow_mut() {
            Ok(mut inner) => inner.mouse_scrolled(x, y, scroll_x, scroll_y),
            Err(_) => false,
        }
    }
    fn next_focus_path(&self, navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        match self.inner.try_borrow() {
            Ok(inner) => GuiEventListener::next_focus_path(&*inner, navigation_event),
            Err(_) => None,
        }
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        match self.inner.try_borrow() {
            Ok(inner) => GuiEventListener::is_mouse_over(&*inner, mouse_x, mouse_y),
            Err(_) => false,
        }
    }
    fn set_focused(&mut self, focused: bool) {
        if let Ok(mut inner) = self.inner.try_borrow_mut() {
            inner.set_focused(focused);
        }
    }
    fn is_focused(&self) -> bool {
        match self.inner.try_borrow() {
            Ok(inner) => GuiEventListener::is_focused(&*inner),
            Err(_) => false,
        }
    }
    fn rectangle(&self) -> ScreenRectangle {
        match self.inner.try_borrow() {
            Ok(inner) => GuiEventListener::rectangle(&*inner),
            Err(_) => ScreenRectangle::empty(),
        }
    }
    fn border_for_arrow_navigation(&self, opposite: ScreenDirection) -> ScreenRectangle {
        match self.inner.try_borrow() {
            Ok(inner) => GuiEventListener::border_for_arrow_navigation(&*inner, opposite),
            Err(_) => ScreenRectangle::empty(),
        }
    }
}
impl NarratableEntry for LockButtonView {
    fn narration_priority(&self) -> NarrationPriority {
        match self.inner.try_borrow() {
            Ok(inner) => NarratableEntry::narration_priority(&*inner),
            Err(_) => NarrationPriority::None,
        }
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        if let Ok(inner) = self.inner.try_borrow() {
            inner.update_narration(output);
        }
        let key = if self.locked.get() {
            LOCK_NARRATION_LOCKED_KEY
        } else {
            LOCK_NARRATION_UNLOCKED_KEY
        };
        output.add(NarratedElementType::Title, crate::translate(key));
    }
}
impl LayoutElement for LockButtonView {
    fn set_x(&mut self, x: i32) {
        if let Ok(mut inner) = self.inner.try_borrow_mut() {
            inner.set_x(x);
        }
    }
    fn set_y(&mut self, y: i32) {
        if let Ok(mut inner) = self.inner.try_borrow_mut() {
            inner.set_y(y);
        }
    }
    fn x(&self) -> i32 {
        match self.inner.try_borrow() {
            Ok(inner) => LayoutElement::x(&*inner),
            Err(_) => 0,
        }
    }
    fn y(&self) -> i32 {
        match self.inner.try_borrow() {
            Ok(inner) => LayoutElement::y(&*inner),
            Err(_) => 0,
        }
    }
    fn width(&self) -> i32 {
        match self.inner.try_borrow() {
            Ok(inner) => LayoutElement::width(&*inner),
            Err(_) => 0,
        }
    }
    fn height(&self) -> i32 {
        match self.inner.try_borrow() {
            Ok(inner) => LayoutElement::height(&*inner),
            Err(_) => 0,
        }
    }
}
