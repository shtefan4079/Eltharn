use crate::FontWidthMeasurer;
use crate::components::click_sound_sink;
use crate::components::{
    Button, DEFAULT_HEIGHT, GuiEventListener, Renderable, TabOrderedElement, WidgetClickSound,
};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{NarratableEntry, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{
    ComponentPath, FocusNavigationEvent, ScreenDirection, ScreenRectangle,
};
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use crate::screens::pause_menu::PauseSubmenu;
use el_core::ResourceLocation;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
const BUTTON_HEIGHT: i32 = 20;
pub const JAVA_FOV_MIN_DEGREES: i32 = 30;
pub const JAVA_FOV_NORMAL_DEGREES: i32 = 70;
pub const JAVA_FOV_MAX_DEGREES: i32 = 110;
const JAVA_FOV_SLIDER_RANGE: f64 = (JAVA_FOV_MAX_DEGREES + 1 - JAVA_FOV_MIN_DEGREES) as f64;
pub fn fov_degrees_from_slider(slider: f32) -> i32 {
    let slider = if slider.is_finite() {
        slider.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let java_slider = if slider >= 1.0 {
        f64::from(0.99999_f32)
    } else {
        f64::from(slider)
    };
    (JAVA_FOV_MIN_DEGREES as f64 + java_slider * JAVA_FOV_SLIDER_RANGE).floor() as i32
}
pub fn fov_slider_from_degrees(degrees: i32) -> f32 {
    let degrees = degrees.clamp(JAVA_FOV_MIN_DEGREES, JAVA_FOV_MAX_DEGREES);
    if degrees == JAVA_FOV_MIN_DEGREES {
        0.0
    } else if degrees == JAVA_FOV_MAX_DEGREES {
        1.0
    } else {
        ((degrees as f64 + 0.5 - JAVA_FOV_MIN_DEGREES as f64) / JAVA_FOV_SLIDER_RANGE) as f32
    }
}
pub fn fov_slider_from_java_option(serialized: f32) -> f32 {
    let degrees = if serialized.is_finite() {
        (serialized * 40.0 + JAVA_FOV_NORMAL_DEGREES as f32).trunc() as i32
    } else {
        JAVA_FOV_NORMAL_DEGREES
    };
    fov_slider_from_degrees(degrees)
}
pub fn java_fov_option_from_slider(slider: f32) -> f32 {
    (fov_degrees_from_slider(slider) - JAVA_FOV_NORMAL_DEGREES) as f32 / 40.0
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionsAction {
    OpenSubmenu(PauseSubmenu),
    Done,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionButtonKind {
    Normal,
    Fov,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionsFovSliderKey {
    Selection,
    Left,
    Right,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionButton {
    pub kind: OptionButtonKind,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub label: String,
    pub label_key: &'static str,
    pub active: bool,
    pub action: Option<OptionsAction>,
}
impl OptionButton {
    pub fn new(
        kind: OptionButtonKind,
        x: i32,
        y: i32,
        width: i32,
        label_key: &'static str,
        action: Option<OptionsAction>,
    ) -> Self {
        Self {
            kind,
            x,
            y,
            width,
            height: BUTTON_HEIGHT,
            label: crate::translate(label_key),
            label_key,
            active: true,
            action,
        }
    }
}
pub struct OptionsScreen {
    base: ScreenBase,
    buttons: Vec<Rc<RefCell<OptionRow>>>,
    dragging_fov: bool,
    pub fov_focused: bool,
    pub fov_can_change_value: bool,
    pub fov_widget_value: Option<f32>,
    pub last_width: i32,
    pub last_height: i32,
    pub in_world: bool,
    font: Arc<dyn crate::FontWidthMeasurer>,
    pending_action: Option<OptionsAction>,
    fov_drag_pending: Option<f32>,
}
struct OptionRow {
    kind: OptionButtonKind,
    button: Option<Button<OptionsAction>>,
    click_sound: Option<WidgetClickSound>,
    rect: ScreenRectangle,
}
struct OptionRowHandle {
    row: Rc<RefCell<OptionRow>>,
}
impl OptionRowHandle {
    fn new(row: Rc<RefCell<OptionRow>>) -> Self {
        Self { row }
    }
    fn with_row<T>(&self, read: impl FnOnce(&OptionRow) -> T, fallback: T) -> T {
        match self.row.try_borrow() {
            Ok(row) => read(&row),
            Err(_) => fallback,
        }
    }
    fn with_row_mut<T>(&mut self, write: impl FnOnce(&mut OptionRow) -> T, fallback: T) -> T {
        match self.row.try_borrow_mut() {
            Ok(mut row) => write(&mut row),
            Err(_) => fallback,
        }
    }
}
impl Renderable for OptionRowHandle {
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        if let Ok(mut row) = self.row.try_borrow_mut() {
            if let Some(button) = row.button.as_mut() {
                button.extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
            }
        }
    }
}
impl TabOrderedElement for OptionRowHandle {
    fn tab_order_group(&self) -> i32 {
        0
    }
}
impl GuiEventListener for OptionRowHandle {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.with_row_mut(
            |row| match row.button.as_mut() {
                Some(button) => button.mouse_clicked(event, double_click),
                None => false,
            },
            false,
        )
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        self.with_row_mut(
            |row| match row.button.as_mut() {
                Some(button) => button.mouse_released(event),
                None => false,
            },
            false,
        )
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        self.with_row_mut(
            |row| match row.button.as_mut() {
                Some(button) => button.mouse_dragged(event, dx, dy),
                None => false,
            },
            false,
        )
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        self.with_row_mut(
            |row| match row.button.as_mut() {
                Some(button) => button.key_pressed(event),
                None => false,
            },
            false,
        )
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.with_row(|row| row.click_sound, None)
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.with_row(
            |row| {
                row.button
                    .as_ref()
                    .is_some_and(|button| GuiEventListener::is_mouse_over(button, mouse_x, mouse_y))
            },
            false,
        )
    }
    fn set_focused(&mut self, focused: bool) {
        if let Ok(mut row) = self.row.try_borrow_mut() {
            if let Some(button) = row.button.as_mut() {
                button.set_focused(focused);
            }
        }
    }
    fn is_focused(&self) -> bool {
        self.with_row(
            |row| {
                row.button
                    .as_ref()
                    .is_some_and(GuiEventListener::is_focused)
            },
            false,
        )
    }
    fn next_focus_path(&self, navigation_event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.with_row(
            |row| {
                row.button
                    .as_ref()
                    .and_then(|button| button.next_focus_path(navigation_event))
            },
            None,
        )
    }
    fn border_for_arrow_navigation(&self, opposite: ScreenDirection) -> ScreenRectangle {
        self.with_row(
            |row| {
                row.button
                    .as_ref()
                    .map(|button| button.border_for_arrow_navigation(opposite))
                    .unwrap_or(row.rect)
            },
            ScreenRectangle::empty(),
        )
    }
    fn rectangle(&self) -> ScreenRectangle {
        self.with_row(|row| row.rect, ScreenRectangle::empty())
    }
}
impl NarratableEntry for OptionRowHandle {
    fn narration_priority(&self) -> NarrationPriority {
        self.with_row(
            |row| {
                row.button
                    .as_ref()
                    .map(NarratableEntry::narration_priority)
                    .unwrap_or(NarrationPriority::None)
            },
            NarrationPriority::None,
        )
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        if let Ok(row) = self.row.try_borrow() {
            if let Some(button) = row.button.as_ref() {
                button.update_narration(output);
            }
        }
    }
}
impl LayoutElement for OptionRowHandle {
    fn set_x(&mut self, x: i32) {
        if let Ok(mut row) = self.row.try_borrow_mut() {
            row.rect.position.x = x;
            if let Some(button) = row.button.as_mut() {
                button.set_x(x);
            }
        }
    }
    fn set_y(&mut self, y: i32) {
        if let Ok(mut row) = self.row.try_borrow_mut() {
            row.rect.position.y = y;
            if let Some(button) = row.button.as_mut() {
                button.set_y(y);
            }
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
impl OptionsScreen {
    pub fn new(in_world: bool, font: Arc<dyn crate::FontWidthMeasurer>) -> Self {
        Self {
            base: ScreenBase::new(el_core::Component::literal(&crate::translate(
                "options.title",
            ))),
            buttons: Vec::new(),
            dragging_fov: false,
            fov_focused: false,
            fov_can_change_value: false,
            fov_widget_value: Some(fov_slider_from_degrees(JAVA_FOV_NORMAL_DEGREES)),
            last_width: 0,
            last_height: 0,
            in_world,
            font,
            pending_action: None,
            fov_drag_pending: None,
        }
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn take_action(&mut self) -> Option<OptionsAction> {
        self.pending_action.take()
    }
    pub fn set_fov_value(&mut self, value: f32) {
        if !self.dragging_fov {
            self.fov_widget_value = Some(value);
        }
        self.fov_drag_pending = None;
    }
    pub fn take_fov_drag(&mut self) -> Option<f32> {
        self.fov_drag_pending.take()
    }
    fn fov_row_rect(&self) -> Option<ScreenRectangle> {
        self.buttons
            .iter()
            .find(|row| row.borrow().kind == OptionButtonKind::Fov)
            .map(|row| row.borrow().rect())
    }
    fn fov_drag(&mut self, mouse_x: f64) -> bool {
        let Some(rect) = self.fov_row_rect() else {
            return false;
        };
        let raw_value = Self::fov_value_from_mouse(mouse_x, rect);
        self.fov_widget_value = Some(raw_value);
        self.fov_drag_pending = Some(raw_value);
        true
    }
    fn fov_value_from_mouse(mouse_x: f64, rect: ScreenRectangle) -> f32 {
        let usable_width = rect.width - 8;
        if usable_width > 0 {
            ((mouse_x as f32 - (rect.left() + 4) as f32) / usable_width as f32).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
    fn fov_widget_index(&self) -> Option<usize> {
        self.buttons
            .iter()
            .position(|row| row.borrow().kind == OptionButtonKind::Fov)
    }
    fn fov_has_container_focus(&self) -> bool {
        self.fov_widget_index() == self.base.focused_index() && self.fov_widget_index().is_some()
    }
    fn sync_fov_focus_from_base(&mut self, focused_by_tab: bool) {
        if self.fov_has_container_focus() {
            if !self.fov_focused {
                self.fov_can_change_value = false;
            }
            self.fov_focused = true;
            if focused_by_tab {
                self.fov_can_change_value = true;
            }
        } else {
            self.clear_fov_focus();
        }
    }
    pub fn fov_slider_key_pressed(
        &mut self,
        key: OptionsFovSliderKey,
        fov_setting: &mut f32,
    ) -> bool {
        if !self.fov_focused {
            return false;
        }
        match key {
            OptionsFovSliderKey::Selection => {
                self.fov_can_change_value = !self.fov_can_change_value;
                true
            }
            OptionsFovSliderKey::Left | OptionsFovSliderKey::Right if self.fov_can_change_value => {
                let widget_value = self.fov_widget_value.unwrap_or(*fov_setting);
                let current = fov_degrees_from_slider(widget_value);
                let direction = if key == OptionsFovSliderKey::Left {
                    -1
                } else {
                    1
                };
                let next = (current + direction).clamp(JAVA_FOV_MIN_DEGREES, JAVA_FOV_MAX_DEGREES);
                let canonical_value = fov_slider_from_degrees(next);
                *fov_setting = canonical_value;
                self.fov_widget_value = Some(canonical_value);
                self.fov_drag_pending = Some(canonical_value);
                true
            }
            OptionsFovSliderKey::Left | OptionsFovSliderKey::Right => false,
        }
    }
    fn focus_fov_from_mouse(&mut self) {
        self.fov_focused = true;
        self.fov_can_change_value = true;
    }
    fn clear_fov_focus(&mut self) {
        self.fov_focused = false;
        self.fov_can_change_value = false;
    }
    fn ensure_layout(&mut self, width: i32, height: i32) {
        if self.last_width == width && self.last_height == height && !self.buttons.is_empty() {
            return;
        }
        self.last_width = width;
        self.last_height = height;
        let mut buttons = Vec::new();
        let left_x = width / 2 - 154;
        let right_x = width / 2 + 4;
        buttons.push(OptionRow::fov(left_x, 29, 150, "options.fov", None));
        if self.in_world {
            buttons.push(OptionRow::button(
                right_x,
                29,
                150,
                "options.worldOptions.button",
                Some(OptionsAction::OpenSubmenu(PauseSubmenu::WorldOptions)),
            ));
        } else {
            buttons.push(OptionRow::button(
                right_x,
                29,
                150,
                "options.online",
                Some(OptionsAction::OpenSubmenu(PauseSubmenu::OnlineOptions)),
            ));
        }
        let grid_y = 91.min(height - 153);
        buttons.push(OptionRow::button(
            left_x,
            grid_y,
            150,
            "options.skinCustomisation",
            Some(OptionsAction::OpenSubmenu(PauseSubmenu::SkinCustomization)),
        ));
        buttons.push(OptionRow::button(
            right_x,
            grid_y,
            150,
            "options.sounds",
            Some(OptionsAction::OpenSubmenu(PauseSubmenu::MusicAndSounds)),
        ));
        buttons.push(OptionRow::button(
            left_x,
            grid_y + 24,
            150,
            "options.video",
            Some(OptionsAction::OpenSubmenu(PauseSubmenu::VideoSettings)),
        ));
        buttons.push(OptionRow::button(
            right_x,
            grid_y + 24,
            150,
            "options.controls",
            Some(OptionsAction::OpenSubmenu(PauseSubmenu::Controls)),
        ));
        buttons.push(OptionRow::button(
            left_x,
            grid_y + 48,
            150,
            "options.language",
            Some(OptionsAction::OpenSubmenu(PauseSubmenu::Language)),
        ));
        buttons.push(OptionRow::button(
            right_x,
            grid_y + 48,
            150,
            "options.chat",
            Some(OptionsAction::OpenSubmenu(PauseSubmenu::ChatSettings)),
        ));
        buttons.push(OptionRow::button(
            left_x,
            grid_y + 72,
            150,
            "options.resourcepack",
            Some(OptionsAction::OpenSubmenu(PauseSubmenu::ResourcePacks)),
        ));
        buttons.push(OptionRow::button(
            right_x,
            grid_y + 72,
            150,
            "options.accessibility",
            Some(OptionsAction::OpenSubmenu(PauseSubmenu::Accessibility)),
        ));
        buttons.push(OptionRow::button_with_active(
            left_x,
            grid_y + 96,
            150,
            "options.telemetry",
            Some(OptionsAction::OpenSubmenu(PauseSubmenu::Telemetry)),
            false,
        ));
        buttons.push(OptionRow::button(
            right_x,
            grid_y + 96,
            150,
            "options.credits_and_attribution",
            Some(OptionsAction::OpenSubmenu(
                PauseSubmenu::CreditsAndAttribution,
            )),
        ));
        buttons.push(OptionRow::button(
            width / 2 - 100,
            height - 26,
            200,
            "gui.done",
            Some(OptionsAction::Done),
        ));
        for row in &buttons {
            if let Ok(mut row) = row.try_borrow_mut()
                && let Some(button) = row.button.as_mut()
            {
                button.set_font(Arc::clone(&self.font));
            }
        }
        self.buttons = buttons;
        self.base.clear_widgets();
        for row in &self.buttons {
            self.base.add_widget(OptionRowHandle::new(Rc::clone(row)));
        }
    }
}
fn unwrap_action(action: Option<OptionsAction>) -> OptionsAction {
    action.unwrap_or(OptionsAction::Done)
}
impl OptionRow {
    fn button(
        x: i32,
        y: i32,
        width: i32,
        translation_key: &str,
        action: Option<OptionsAction>,
    ) -> Rc<RefCell<OptionRow>> {
        Self::button_with_active(x, y, width, translation_key, action, true)
    }
    fn button_with_active(
        x: i32,
        y: i32,
        width: i32,
        translation_key: &str,
        action: Option<OptionsAction>,
        active: bool,
    ) -> Rc<RefCell<OptionRow>> {
        let mut button = Button::<OptionsAction>::builder(el_core::Component::literal(
            &crate::translate(translation_key),
        ))
        .bounds(x, y, width, DEFAULT_HEIGHT)
        .with_action(unwrap_action(action))
        .build();
        button.base.active = active;
        Rc::new(RefCell::new(Self {
            kind: OptionButtonKind::Normal,
            button: Some(button),
            click_sound: Some(WidgetClickSound::UiButtonClick),
            rect: ScreenRectangle::new(x, y, width, DEFAULT_HEIGHT),
        }))
    }
    fn fov(
        x: i32,
        y: i32,
        width: i32,
        translation_key: &str,
        _action: Option<OptionsAction>,
    ) -> Rc<RefCell<OptionRow>> {
        let button = Button::<OptionsAction>::builder(el_core::Component::literal(
            &crate::translate(translation_key),
        ))
        .bounds(x, y, width, DEFAULT_HEIGHT)
        .build();
        Rc::new(RefCell::new(Self {
            kind: OptionButtonKind::Fov,
            button: Some(button),
            click_sound: None,
            rect: ScreenRectangle::new(x, y, width, DEFAULT_HEIGHT),
        }))
    }
    fn rect(&self) -> ScreenRectangle {
        self.rect
    }
}
impl Screen for OptionsScreen {
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
        self.buttons.clear();
        self.ensure_layout(width, height);
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        if self.last_width != width || self.last_height != height {
            self.init();
        }
        self.base.extract_background_layers(graphics, self.in_world);
        self.sync_fov_focus_from_base(false);
        let title_text = crate::translate("options.title");
        let title_w = self.font.measure_width(&title_text, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            title_text,
            (width - title_w) / 2,
            12,
            0xFFFF_FFFFu32 as i32,
        );
        let fov_widget_value = self
            .fov_widget_value
            .unwrap_or_else(|| fov_slider_from_degrees(JAVA_FOV_NORMAL_DEGREES));
        for row in &mut self.buttons {
            let kind = row.borrow().kind;
            match kind {
                OptionButtonKind::Normal => {
                    if let Ok(mut r) = row.try_borrow_mut() {
                        if let Some(button) = r.button.as_mut() {
                            button.extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
                        }
                    }
                }
                OptionButtonKind::Fov => {
                    let rect = row.borrow().rect();
                    graphics.draw_slider_with_interaction_state(
                        self.font.as_ref(),
                        rect.left(),
                        rect.top(),
                        rect.width,
                        rect.height,
                        &format_fov(fov_widget_value),
                        fov_widget_value,
                        true,
                        self.fov_focused,
                        self.fov_can_change_value,
                        self.dragging_fov,
                    );
                }
            }
        }
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if event.is_escape() {
            self.base.set_last_input_keyboard(true);
            self.base.after_keyboard_action();
            self.pending_action = Some(OptionsAction::Done);
            return true;
        }
        self.sync_fov_focus_from_base(false);
        if self.fov_has_container_focus() {
            let key = match event.key {
                KeyEvent::KEY_ENTER | KeyEvent::KEY_SPACE => Some(OptionsFovSliderKey::Selection),
                KeyEvent::KEY_LEFT => Some(OptionsFovSliderKey::Left),
                KeyEvent::KEY_RIGHT => Some(OptionsFovSliderKey::Right),
                _ => None,
            };
            if let Some(key) = key {
                let is_slider_value_key =
                    matches!(key, OptionsFovSliderKey::Selection) || self.fov_can_change_value;
                if is_slider_value_key {
                    self.base.set_last_input_keyboard(true);
                    self.base.after_keyboard_action();
                    let mut fov = self
                        .fov_widget_value
                        .unwrap_or_else(|| fov_slider_from_degrees(JAVA_FOV_NORMAL_DEGREES));
                    let consumed = self.fov_slider_key_pressed(key, &mut fov);
                    self.fov_widget_value = Some(fov);
                    return consumed;
                }
            }
        }
        let handled = crate::framework::screen::screen_key_pressed(self, event);
        self.sync_fov_focus_from_base(event.key == KeyEvent::KEY_TAB);
        handled
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        let handled = crate::framework::screen::screen_mouse_clicked(self, event, double_click);
        if !handled {
            return false;
        }
        let mut pressed = None;
        let mut fov_after_drag = None;
        let mut fov_focused = false;
        let mut dragging = false;
        for row in &self.buttons {
            let mut row = row.borrow_mut();
            let is_fov = row.kind == OptionButtonKind::Fov;
            if is_fov {
                let hit =
                    event.button() == 0 && row.rect.contains_point(event.x as i32, event.y as i32);
                let _ = row.button.as_mut().map(Button::take_press_action);
                if !hit {
                    continue;
                }
                click_sound_sink::record(None);
                fov_focused = true;
                dragging = true;
                let rect = row.rect();
                let raw = Self::fov_value_from_mouse(event.x, rect);
                fov_after_drag = Some(raw);
            } else if let Some(action) = row.button.as_mut().and_then(Button::take_press_action) {
                pressed = Some(action);
            }
        }
        self.sync_fov_focus_from_base(false);
        if fov_focused {
            click_sound_sink::record(None);
            self.focus_fov_from_mouse();
        }
        self.dragging_fov = dragging;
        if let Some(fov) = fov_after_drag {
            self.fov_widget_value = Some(fov);
            self.fov_drag_pending = Some(fov);
        }
        if let Some(action) = pressed {
            self.pending_action = Some(action);
        }
        true
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        let handled = crate::framework::screen::screen_mouse_released(self, event);
        let ends_fov_drag = self.dragging_fov;
        if ends_fov_drag {
            self.dragging_fov = false;
            click_sound_sink::record_button_click();
        }
        handled || ends_fov_drag
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        let handled = crate::framework::screen::screen_mouse_dragged(self, event, dx, dy);
        let is_fov_drag = self.dragging_fov && event.button() == 0;
        if is_fov_drag {
            self.fov_drag(event.x);
        }
        handled || is_fov_drag
    }
}
pub fn format_fov(fov: f32) -> String {
    let rounded = fov_degrees_from_slider(fov);
    let fov_label = crate::translate("options.fov");
    if rounded == 70 {
        format!("{}: {}", fov_label, crate::translate("options.fov.min"))
    } else if rounded == 110 {
        format!("{}: {}", fov_label, crate::translate("options.fov.max"))
    } else {
        format!("{}: {}", fov_label, rounded)
    }
}
