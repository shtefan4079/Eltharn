use el_gui::{
    FontWidthMeasurer,
    components::edit_box::{Clipboard, EditBox, EditBoxKey, EditBoxRenderPlan},
    components::scroll_area::{AbstractScrollArea, ScrollbarSettings},
};
use el_mod_runtime::app::{
    Event, EventKind, EventValue, InputFrame, MAX_EVENTS, Report, Ui, Widget,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};
use std::{rc::Rc, sync::Arc};
use winit::event::{ElementState, MouseButton, TouchPhase, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
#[derive(Default)]
struct Surface {
    active: bool,
    menu: String,
    screen: Option<std::mem::Discriminant<crate::ui::gui::ActiveScreen>>,
    viewport: [u32; 2],
    pointer: [i32; 2],
    sequence: u64,
    context: u64,
    event_sequence: u64,
    readers: Vec<String>,
    owners: BTreeSet<String>,
    scene: BTreeMap<String, Ui>,
    revisions: BTreeMap<String, u64>,
    events: BTreeMap<String, Vec<Event>>,
    pressed: Option<(String, u32)>,
    captured: bool,
    touch: Option<u64>,
    settings_owner: Option<String>,
    settings_panel_seen: bool,
    settings_close_requested: bool,
    settings_session: u64,
    settings_close_intent: bool,
    focus: Option<(String, u32)>,
    key_capture: Option<(String, u32)>,
    binding_mouse_releases: BTreeSet<MouseButton>,
    scroll: BTreeMap<String, i32>,
    scrollbar_drag: Option<(String, AbstractScrollArea)>,
    editors: BTreeMap<(String, u32), EditBox>,
    font: Option<Arc<dyn FontWidthMeasurer>>,
    clipboard: Option<Rc<RefCell<dyn Clipboard>>>,
    shift: bool,
    ctrl: bool,
    last_input_keyboard: bool,
    tooltip_started: BTreeMap<(String, u32), u64>,
}
thread_local! {
    static SURFACE : RefCell < Surface > = RefCell::new(Surface::default()); static
    CLOCK_ORIGIN : web_time::Instant = web_time::Instant::now();
}
pub fn services<C: Clipboard + 'static>(
    font: Arc<dyn FontWidthMeasurer>,
    clipboard: Rc<RefCell<C>>,
) {
    SURFACE.with(|cell| {
        let mut s = cell.borrow_mut();
        s.font = Some(font);
        s.clipboard = Some(clipboard);
        s.sync_editors();
    });
}
pub fn settings_title() -> Option<String> {
    SURFACE.with(|cell| {
        let s = cell.borrow();
        s.settings_owner
            .as_ref()
            .and_then(|o| s.scene.get(o))
            .and_then(|ui| ui.layout.as_ref())
            .map(|l| l.title.clone())
    })
}
pub fn settings_source_footer() -> bool {
    SURFACE.with(|cell| {
        let s = cell.borrow();
        s.settings_owner
            .as_ref()
            .and_then(|owner| s.scene.get(owner))
            .and_then(|ui| ui.layout.as_ref())
            .is_some_and(|layout| !layout.footer.is_empty())
    })
}
pub fn request_settings_close() -> bool {
    SURFACE.with(|cell| cell.borrow_mut().request_close())
}
pub fn focused(owner: &str, id: u32) -> bool {
    SURFACE.with(|cell| {
        cell.borrow()
            .focus
            .as_ref()
            .is_some_and(|f| f.0 == owner && f.1 == id)
    })
}
pub fn keyboard_navigation() -> bool {
    SURFACE.with(|cell| cell.borrow().last_input_keyboard)
}
pub fn tooltip_visible(owner: &str, id: u32, hovered: bool) -> bool {
    SURFACE.with(|cell| {
        cell.borrow_mut()
            .tooltip_visible_at(owner, id, hovered, now_ms())
    })
}
pub fn dragging(owner: &str, id: u32) -> bool {
    SURFACE.with(|cell| {
        cell.borrow()
            .pressed
            .as_ref()
            .is_some_and(|f| f.0 == owner && f.1 == id)
    })
}
pub fn capturing(owner: &str, id: u32) -> bool {
    SURFACE.with(|cell| {
        cell.borrow()
            .key_capture
            .as_ref()
            .is_some_and(|f| f.0 == owner && f.1 == id)
    })
}
pub fn text_plan(owner: &str, id: u32) -> Option<EditBoxRenderPlan> {
    SURFACE.with(|cell| {
        let s = cell.borrow();
        let font = s.font.as_ref()?.as_ref();
        let editor = s.editors.get(&(owner.into(), id))?;
        let mut plan = editor.render_plan(font, now_ms());
        let ui = s.scene.get(owner)?;
        if let Some(p) = ui.presentation(id) {
            if p.invalid {
                for draw in &mut plan.draws {
                    use el_gui::components::edit_box::EditBoxDraw;
                    match draw {
                        EditBoxDraw::Text { color, .. }
                        | EditBoxDraw::InsertCursor { color, .. }
                        | EditBoxDraw::AppendCursor { color, .. } => {
                            *color = 0xffff0000u32 as i32;
                        }
                        _ => {}
                    }
                }
            }
            if !p.suggestion.is_empty() {
                use el_gui::components::edit_box::EditBoxDraw;
                let visible: String = plan
                    .draws
                    .iter()
                    .filter_map(|draw| match draw {
                        EditBoxDraw::Text { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect();
                if font.measure_width(&format!("{visible}{}", p.suggestion), 1.0)
                    < editor.inner_width() as f32
                {
                    if let Some(widget) =
                        s.resolved(owner, ui).widgets.iter().find(|w| w.id() == id)
                    {
                        let [x, y, w, h] = widget.rect();
                        plan.draws.push(EditBoxDraw::Text {
                            text: p.suggestion.clone(),
                            x: x + w - 4 - font.measure_width(&p.suggestion, 1.0).ceil() as i32,
                            y: y + (h - 8) / 2,
                            color: 0xff808080u32 as i32,
                            shadow: true,
                        });
                    }
                }
            }
        }
        Some(plan)
    })
}
fn now_ms() -> u64 {
    CLOCK_ORIGIN.with(|start| u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX))
}
pub fn menu_scrollbar(owner: &str) -> Option<AbstractScrollArea> {
    SURFACE.with(|cell| cell.borrow().scrollbar(owner))
}
pub fn context(
    menu: &str,
    screen: Option<std::mem::Discriminant<crate::ui::gui::ActiveScreen>>,
    viewport: [u32; 2],
    outside_world: bool,
) {
    SURFACE.with(|cell| {
        let mut s = cell.borrow_mut();
        let viewport = viewport.map(|v| v.clamp(1, 8192));
        let active = outside_world || s.settings_owner.is_some();
        let menu = if s.settings_owner.is_some() {
            el_mod_runtime::app::SETTINGS_MENU
        } else {
            menu
        };
        if s.menu != menu || s.screen != screen || s.active != active || s.viewport != viewport {
            s.scene.clear();
            s.revisions.clear();
            s.events.clear();
            s.pressed = None;
            s.touch = None;
            s.captured = false;
            if s.settings_owner.is_none() {
                s.settings_panel_seen = false;
            }
            s.settings_close_intent = false;
            s.focus = None;
            s.last_input_keyboard = false;
            s.tooltip_started.clear();
            s.key_capture = None;
            s.editors.clear();
            s.scroll.clear();
            s.scrollbar_drag = None;
            s.context = s.context.saturating_add(1);
        }
        s.menu = menu.to_owned();
        s.screen = screen;
        s.viewport = viewport;
        s.active = active;
    });
}
pub fn open_settings(owner: &str) -> Result<(), String> {
    SURFACE.with(|cell| {
        let mut s = cell.borrow_mut();
        if !s.owners.contains(owner) || !s.readers.iter().any(|reader| reader == owner) {
            return Err("Settings owner is disabled or unauthorized".into());
        }
        if s.settings_owner.as_deref() != Some(owner) {
            let next_session = s
                .settings_session
                .checked_add(1)
                .filter(|v| *v <= 9_007_199_254_740_991)
                .ok_or("Settings session exhausted")?;
            s.clear_view();
            s.context = s.context.saturating_add(1);
            s.settings_owner = Some(owner.into());
            s.settings_close_requested = false;
            s.settings_session = next_session;
        }
        Ok(())
    })
}
pub fn settings_owner() -> Option<String> {
    SURFACE.with(|cell| cell.borrow().settings_owner.clone())
}
pub fn is_settings_open() -> bool {
    settings_owner().is_some()
}
pub fn close_settings() -> bool {
    SURFACE.with(|cell| {
        let mut s = cell.borrow_mut();
        if s.settings_owner.take().is_none() {
            return false;
        }
        s.clear_view();
        s.menu.clear();
        s.context = s.context.saturating_add(1);
        s.settings_close_requested = false;
        true
    })
}
pub fn take_settings_close_request() -> bool {
    SURFACE.with(|cell| std::mem::take(&mut cell.borrow_mut().settings_close_requested))
}
pub fn configure(report: &serde_json::Value, owners: Vec<String>) -> Result<(), String> {
    clear();
    SURFACE.with(|cell| cell.borrow_mut().owners = owners.into_iter().collect());
    accept(report)
}
pub fn accept(report: &serde_json::Value) -> Result<(), String> {
    let app: Report = match report.get("app") {
        Some(value) => serde_json::from_value(value.clone()).map_err(|e| e.to_string())?,
        None => Report::default(),
    };
    if app.readers.len() > 16 || app.scene.len() > 16 {
        return Err("App report owner limit".into());
    }
    for (owner, ui) in &app.scene {
        ui.validate()?;
        ui.validate_owner(owner)?;
    }
    SURFACE.with(|cell| -> Result<(), String> {
        let mut s = cell.borrow_mut();
        let readers: BTreeSet<_> = app.readers.iter().cloned().collect();
        if readers.len() != app.readers.len()
            || !readers.is_subset(&s.owners)
            || app.scene.keys().any(|k| !readers.contains(k))
            || app.revisions.keys().ne(app.scene.keys())
            || app
                .revisions
                .values()
                .any(|v| *v == 0 || *v > 9_007_199_254_740_991)
        {
            return Err("Foreign app report owner".into());
        }
        s.readers = app.readers;
        if s.settings_owner
            .as_ref()
            .is_some_and(|owner| !readers.contains(owner))
        {
            s.invalidate_settings();
        }
        let old_scene = s.scene.clone();
        let old_revisions = s.revisions.clone();
        if s.active && app.menu == s.menu && app.context == s.context {
            s.scene = app.scene;
            s.revisions = app.revisions;
            if let Some(owner) = s.settings_owner.clone() {
                s.scene.retain(|key, _| key == &owner);
                s.revisions.retain(|key, _| key == &owner);
                let nonempty = s.scene.get(&owner).is_some_and(|ui| !ui.widgets.is_empty());
                if s.settings_panel_seen && !nonempty {
                    s.settings_close_requested = true;
                }
                s.settings_panel_seen |= nonempty;
            }
        } else if !s.active {
            s.scene.clear();
            s.revisions.clear();
        } else {
            s.scene.retain(|owner, _| readers.contains(owner));
            s.revisions.retain(|owner, _| readers.contains(owner));
        }
        let live = s.scene.clone();
        s.tooltip_started.retain(|(owner, id), _| {
            live.get(owner)
                .and_then(|ui| ui.presentation(*id))
                .is_some_and(|p| !p.tooltip.is_empty())
        });
        let revisions = s.revisions.clone();
        let continuous_owners: BTreeSet<_> = old_scene
            .iter()
            .filter_map(|(owner, previous)| {
                let current = live.get(owner)?;
                let old_revision = old_revisions.get(owner);
                let new_revision = revisions.get(owner);
                let unchanged_revision = old_revision == new_revision;
                let one_commit = old_revision
                    .zip(new_revision)
                    .is_some_and(|(old, new)| old.checked_add(1) == Some(*new));
                let continuous_commit = one_commit
                    && previous.same_layout_interaction(current)
                    && previous
                        .widgets
                        .iter()
                        .map(Widget::id)
                        .eq(current.widgets.iter().map(Widget::id))
                    && previous
                        .widgets
                        .iter()
                        .zip(&current.widgets)
                        .any(|(old, new)| {
                            old.interactive() != new.interactive()
                                || (old.interactive() && !old.same_interaction(new))
                        });
                (unchanged_revision || continuous_commit).then_some(owner.as_str())
            })
            .collect();
        let retains_target = |owner: &str, id: u32| {
            continuous_owners.contains(owner)
                && old_scene
                    .get(owner)
                    .and_then(|ui| ui.widgets.iter().find(|widget| widget.id() == id))
                    .zip(live.get(owner).and_then(|ui| {
                        ui.widgets
                            .iter()
                            .find(|widget| widget.id() == id && widget.interactive())
                    }))
                    .is_some_and(|(old, new)| old.same_interaction(new))
        };
        if s.settings_owner
            .as_ref()
            .is_none_or(|owner| !live.get(owner).is_some_and(Ui::requests_close))
        {
            s.settings_close_intent = false;
        }
        s.events.retain(|owner, events| {
            events.retain(|event| retains_target(owner, event.widget));
            !events.is_empty()
        });
        if s.pressed
            .as_ref()
            .is_some_and(|(owner, id)| !retains_target(owner, *id))
        {
            s.pressed = None;
        }
        if s.focus
            .as_ref()
            .is_some_and(|(owner, id)| !retains_target(owner, *id))
        {
            s.focus = None;
        }
        if s.key_capture
            .as_ref()
            .is_some_and(|(owner, id)| !retains_target(owner, *id))
        {
            s.key_capture = None;
        }
        s.clamp_scroll();
        s.sync_editors();
        Ok(())
    })
}
pub fn clear() {
    SURFACE.with(|cell| *cell.borrow_mut() = Surface::default());
}
pub fn hide() {
    SURFACE.with(|cell| {
        let mut s = cell.borrow_mut();
        s.active = false;
        s.settings_owner = None;
        s.settings_panel_seen = false;
        s.settings_close_requested = false;
        s.settings_close_intent = false;
        s.scene.clear();
        s.revisions.clear();
        s.events.clear();
        s.pressed = None;
        s.touch = None;
        s.captured = false;
        s.focus = None;
        s.key_capture = None;
        s.editors.clear();
        s.scroll.clear();
        s.scrollbar_drag = None;
    });
}
pub fn scene() -> BTreeMap<String, Ui> {
    SURFACE.with(|cell| {
        let s = cell.borrow();
        if s.active {
            if s.settings_owner.is_some() {
                s.scene
                    .iter()
                    .map(|(owner, ui)| {
                        if ui.layout.is_some() {
                            return (owner.clone(), s.resolved(owner, ui));
                        }
                        (
                            owner.clone(),
                            Ui {
                                widgets: ui
                                    .widgets
                                    .iter()
                                    .filter_map(|widget| {
                                        let mut clipped = widget.clone();
                                        let [x, y, w, h] = clipped.rect();
                                        let right = (x + w).min(s.viewport[0] as i32);
                                        let bottom = (y + h).min(s.viewport[1] as i32);
                                        let left = x.max(0);
                                        let top = y.max(0);
                                        if right <= left || bottom <= top {
                                            return None;
                                        }
                                        if !matches!(clipped, Widget::Image { .. }) {
                                            clipped.set_rect([
                                                left,
                                                top,
                                                right - left,
                                                bottom - top,
                                            ]);
                                        }
                                        Some(clipped)
                                    })
                                    .collect(),
                                ..Ui::default()
                            },
                        )
                    })
                    .collect()
            } else {
                s.scene
                    .iter()
                    .map(|(owner, ui)| (owner.clone(), s.resolved(owner, ui)))
                    .collect()
            }
        } else {
            BTreeMap::new()
        }
    })
}
pub fn pointer() -> [i32; 2] {
    SURFACE.with(|cell| cell.borrow().pointer)
}
pub fn take_input() -> Result<BTreeMap<String, InputFrame>, String> {
    SURFACE.with(|cell| {
        let mut s = cell.borrow_mut();
        if !s.active {
            return Ok(BTreeMap::new());
        }
        s.flush_close_intent()?;
        s.sequence = s
            .sequence
            .checked_add(1)
            .ok_or("App input sequence exhausted")?;
        let frame = InputFrame {
            sequence: s.sequence,
            context: s.context,
            settings_session: if s.settings_owner.is_some() {
                s.settings_session
            } else {
                0
            },
            ui_revision: 0,
            menu: s.menu.clone(),
            viewport: s.viewport,
            pointer: s.pointer,
            events: Vec::new(),
        };
        let events = std::mem::take(&mut s.events);
        s.readers
            .iter()
            .filter(|owner| {
                s.settings_owner
                    .as_ref()
                    .is_none_or(|selected| selected == *owner)
            })
            .map(|owner| {
                let mut frame = frame.clone();
                frame.ui_revision = s.revisions.get(owner).copied().unwrap_or(0);
                frame.events = events.get(owner).cloned().unwrap_or_default();
                frame.validate()?;
                Ok((owner.clone(), frame))
            })
            .collect()
    })
}
impl Surface {
    fn tooltip_visible_at(&mut self, owner: &str, id: u32, hovered: bool, now: u64) -> bool {
        let key = (owner.to_owned(), id);
        let focused = self.last_input_keyboard && self.focus.as_ref() == Some(&key);
        let allowed = self.active
            && self
                .settings_owner
                .as_ref()
                .is_none_or(|selected| selected == owner)
            && self
                .scene
                .get(owner)
                .and_then(|ui| ui.presentation(id))
                .is_some_and(|p| !p.tooltip.is_empty());
        if !allowed || !(hovered || focused) {
            self.tooltip_started.remove(&key);
            return false;
        }
        now > *self.tooltip_started.entry(key).or_insert(now)
    }
    fn observe_navigation_input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::MouseInput { .. } | WindowEvent::Touch(_) => {
                self.last_input_keyboard = false;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    self.observe_navigation_key(code);
                }
            }
            _ => {}
        }
    }
    fn observe_navigation_key(&mut self, code: KeyCode) {
        if matches!(
            code,
            KeyCode::Tab
                | KeyCode::ArrowLeft
                | KeyCode::ArrowRight
                | KeyCode::ArrowUp
                | KeyCode::ArrowDown
        ) {
            self.last_input_keyboard = true;
        }
    }
    fn focus_by_keyboard(&mut self, reverse: bool) {
        self.last_input_keyboard = true;
        self.focus_next(reverse);
    }
    fn request_close(&mut self) -> bool {
        if !self.active
            || self.settings_session == 0
            || !self
                .settings_owner
                .as_ref()
                .and_then(|owner| self.scene.get(owner))
                .is_some_and(Ui::requests_close)
        {
            return false;
        }
        self.settings_close_intent = true;
        true
    }
    fn flush_close_intent(&mut self) -> Result<(), String> {
        if !self.settings_close_intent {
            return Ok(());
        }
        let Some(owner) = self.settings_owner.clone() else {
            self.settings_close_intent = false;
            return Ok(());
        };
        if !self.scene.get(&owner).is_some_and(Ui::requests_close) {
            self.settings_close_intent = false;
            return Ok(());
        }
        if self
            .events
            .get(&owner)
            .is_some_and(|events| events.len() >= MAX_EVENTS)
        {
            return Ok(());
        }
        self.queue(&(owner, 0), EventKind::CloseRequest)?;
        self.settings_close_intent = false;
        Ok(())
    }
    fn scrollbar(&self, owner: &str) -> Option<AbstractScrollArea> {
        let ui = self.scene.get(owner)?;
        let layout = ui.layout.as_ref()?;
        let [_, top, viewport_width, bottom] = ui.content_bounds(self.viewport);
        let height = bottom - top;
        if height < 9 || viewport_width < 14 {
            return None;
        }
        let columns = if viewport_width < 300 {
            1
        } else {
            layout.columns as usize
        };
        let content_width = (viewport_width - 28).clamp(1, 310);
        let content_x = (viewport_width - content_width) / 2;
        let scrollbar_x = (content_x + content_width + 4).min(viewport_width - 6);
        let mut area = AbstractScrollArea::new(
            scrollbar_x,
            top,
            6,
            height,
            el_core::Component::literal(""),
            ScrollbarSettings::default_settings(15),
        );
        area.content_height = ui.body_widget_count().div_ceil(columns) as i32 * 36 + 8;
        area.set_scroll_amount(self.scroll.get(owner).copied().unwrap_or(0) as f32, height);
        area.scrolling = self
            .scrollbar_drag
            .as_ref()
            .is_some_and(|(o, _)| o == owner);
        area.scrollable(height).then_some(area)
    }
    fn clamp_scroll(&mut self) {
        let owners: Vec<_> = self.scene.keys().cloned().collect();
        self.scroll
            .retain(|owner, _| self.scene.contains_key(owner));
        for owner in owners {
            let value = self
                .scrollbar(&owner)
                .map_or(0, |area| area.scroll_amount as i32);
            self.scroll.insert(owner, value);
        }
        if let Some((owner, _)) = self.scrollbar_drag.take() {
            if let Some(mut area) = self.scrollbar(&owner) {
                area.scrolling = true;
                self.scrollbar_drag = Some((owner, area));
            }
        }
    }
    fn resolved(&self, owner: &str, ui: &Ui) -> Ui {
        ui.resolved(self.viewport, self.scroll.get(owner).copied().unwrap_or(0))
    }
    fn widget(&self, target: &(String, u32)) -> Option<Widget> {
        self.scene
            .get(&target.0)
            .map(|ui| self.resolved(&target.0, ui))?
            .widgets
            .into_iter()
            .find(|w| w.id() == target.1)
    }
    fn sync_editors(&mut self) {
        let Some(font) = self.font.clone() else {
            return;
        };
        let live: BTreeMap<_, _> = self
            .scene
            .iter()
            .flat_map(|(owner, ui)| {
                self.resolved(owner, ui)
                    .widgets
                    .into_iter()
                    .filter_map(|w| match w {
                        Widget::TextField {
                            id,
                            rect,
                            value,
                            max_length,
                            enabled,
                            ..
                        } => Some(((owner.clone(), id), (rect, value, max_length, enabled))),
                        _ => None,
                    })
            })
            .collect();
        self.editors.retain(|key, _| live.contains_key(key));
        for (key, (rect, value, max_length, enabled)) in live {
            let [x, y, w, h] = rect;
            let editor = self.editors.entry(key.clone()).or_insert_with(|| {
                let mut e = EditBox::new(font.as_ref(), x, y, w, h);
                e.set_max_length(font.as_ref(), max_length);
                e.set_value(font.as_ref(), &value);
                e
            });
            editor.set_position(font.as_ref(), x, y);
            editor.set_size(font.as_ref(), w, h);
            editor.set_max_length(font.as_ref(), max_length);
            editor.set_editable(enabled);
            let focused = self.focus.as_ref() == Some(&key);
            if !focused && editor.value() != value {
                editor.set_value(font.as_ref(), &value);
            }
            if editor.is_focused() != focused {
                editor.set_focused(focused, now_ms());
            }
        }
    }
    fn change(&mut self, target: &(String, u32), value: EventValue) -> Result<(), String> {
        if let Some(widget) = self
            .scene
            .get_mut(&target.0)
            .and_then(|ui| ui.widgets.iter_mut().find(|w| w.id() == target.1))
        {
            match (widget, &value) {
                (Widget::Toggle { value, .. }, EventValue::Bool(v)) => *value = *v,
                (Widget::Slider { value, .. }, EventValue::Number(v)) => *value = *v,
                (
                    Widget::TextField { value, .. } | Widget::KeyBinding { value, .. },
                    EventValue::Text(v),
                ) => *value = v.clone(),
                _ => {}
            }
        }
        self.queue_value(target, EventKind::Change, Some(value))
    }
    fn slider_pointer(&mut self, target: &(String, u32)) -> Result<(), String> {
        if let Some(Widget::Slider {
            rect: [x, _, w, _],
            min,
            max,
            step,
            ..
        }) = self.widget(target)
        {
            let percent =
                ((self.pointer[0] - x - 4) as f64 / (w - 8).max(1) as f64).clamp(0.0, 1.0);
            let value = (min
                + (((max - min) as f64 * percent) / step as f64).round() as i32 * step)
                .clamp(min, max);
            self.change(target, EventValue::Number(value))?;
        }
        Ok(())
    }
    fn activate(&mut self, target: &(String, u32)) -> Result<(), String> {
        match self.widget(target) {
            Some(Widget::Toggle { value, .. }) => self.change(target, EventValue::Bool(!value))?,
            Some(Widget::KeyBinding { .. }) => self.key_capture = Some(target.clone()),
            Some(Widget::Button { .. } | Widget::Action { .. }) => {
                self.queue(target, EventKind::Click)?;
                el_gui::components::click_sound_sink::record_button_click();
            }
            _ => {}
        }
        Ok(())
    }
    fn focus_next(&mut self, reverse: bool) {
        let targets: Vec<_> = self
            .scene
            .iter()
            .flat_map(|(owner, ui)| {
                ui.widgets
                    .iter()
                    .filter(|w| w.interactive())
                    .map(|w| (owner.clone(), w.id()))
            })
            .collect();
        if targets.is_empty() {
            self.focus = None;
            return;
        }
        let index = self
            .focus
            .as_ref()
            .and_then(|f| targets.iter().position(|t| t == f));
        let index = match index {
            Some(i) if reverse => (i + targets.len() - 1) % targets.len(),
            Some(i) => (i + 1) % targets.len(),
            None if reverse => targets.len() - 1,
            None => 0,
        };
        let target = targets[index].clone();
        self.focus = Some(target.clone());
        if let Some(widget) = self.widget(&target) {
            let [_, mut y, _, mut h] = widget.rect();
            if matches!(widget, Widget::TextField { .. }) {
                y -= 10;
                h += 10;
            }
            if self.scene[&target.0].layout.is_some() && !self.scene[&target.0].is_footer(target.1)
            {
                let current = self.scroll.get(&target.0).copied().unwrap_or(0);
                let top = 33;
                let bottom = self.viewport[1] as i32;
                let delta = if y < top {
                    y - top
                } else if y + h > bottom {
                    y + h - bottom
                } else {
                    0
                };
                self.scroll.insert(target.0, current + delta);
                self.clamp_scroll();
            }
        }
        self.sync_editors();
    }
    fn wheel(&mut self, delta: f64) -> bool {
        if !delta.is_finite() {
            return false;
        }
        let Some((owner, mut area)) = self
            .scene
            .iter()
            .rev()
            .find_map(|(owner, _)| self.scrollbar(owner).map(|area| (owner.clone(), area)))
        else {
            return false;
        };
        area.scroll(delta as f32, area.viewport());
        self.scroll.insert(owner, area.scroll_amount as i32);
        self.pressed = None;
        self.scrollbar_drag = None;
        self.sync_editors();
        true
    }
    fn keyboard(&mut self, event: &winit::event::KeyEvent) -> Result<bool, String> {
        if event.state != ElementState::Pressed {
            return Ok(self.focus.is_some());
        }
        let PhysicalKey::Code(code) = event.physical_key else {
            return Ok(false);
        };
        if code == KeyCode::Escape {
            return Ok(self.escape());
        }
        if let Some(target) = self.key_capture.take() {
            self.change(&target, EventValue::Text(format!("{code:?}")))?;
            return Ok(true);
        }
        if code == KeyCode::Tab {
            self.focus_by_keyboard(self.shift);
            return Ok(self.focus.is_some());
        }
        if matches!(code, KeyCode::PageUp | KeyCode::PageDown) {
            if let Some((owner, mut area)) = self
                .scene
                .iter()
                .rev()
                .find_map(|(owner, _)| self.scrollbar(owner).map(|area| (owner.clone(), area)))
            {
                let distance =
                    area.viewport() as f32 * if code == KeyCode::PageUp { -1.0 } else { 1.0 };
                area.scroll_by(distance, area.viewport());
                self.scroll.insert(owner, area.scroll_amount as i32);
                self.pressed = None;
                self.scrollbar_drag = None;
                self.sync_editors();
                return Ok(true);
            }
        }
        let Some(target) = self.focus.clone() else {
            return Ok(false);
        };
        if matches!(self.widget(&target), Some(Widget::TextField { .. })) {
            let Some(font) = self.font.clone() else {
                return Ok(false);
            };
            let clipboard = self.clipboard.clone();
            let ctrl = self.ctrl;
            let shift = self.shift;
            let editor = self
                .editors
                .get_mut(&target)
                .ok_or("Missing menu text editor")?;
            let before = editor.value();
            match code {
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    let value = editor.value();
                    self.queue_value(&target, EventKind::Submit, Some(EventValue::Text(value)))?;
                    return Ok(true);
                }
                KeyCode::KeyA if ctrl => {
                    editor.select_all(font.as_ref());
                }
                KeyCode::KeyC if ctrl => {
                    if let Some(c) = clipboard {
                        editor.copy(&mut *c.borrow_mut());
                    }
                }
                KeyCode::KeyX if ctrl => {
                    if let Some(c) = clipboard {
                        editor.cut(font.as_ref(), &mut *c.borrow_mut());
                    }
                }
                KeyCode::KeyV if ctrl => {
                    if let Some(c) = clipboard {
                        editor.paste(font.as_ref(), &*c.borrow());
                    }
                }
                _ => {
                    let key = match code {
                        KeyCode::Backspace => Some(EditBoxKey::Backspace),
                        KeyCode::Delete => Some(EditBoxKey::Delete),
                        KeyCode::ArrowLeft => Some(EditBoxKey::Left),
                        KeyCode::ArrowRight => Some(EditBoxKey::Right),
                        KeyCode::Home => Some(EditBoxKey::Home),
                        KeyCode::End => Some(EditBoxKey::End),
                        _ => None,
                    };
                    if let Some(key) = key {
                        editor.key_pressed(font.as_ref(), key, ctrl, shift);
                    } else if !ctrl {
                        if let Some(text) = &event.text {
                            for c in text.chars() {
                                editor.char_typed(font.as_ref(), c);
                            }
                        }
                    }
                }
            }
            let value = editor.value();
            if value != before {
                self.change(&target, EventValue::Text(value))?;
            }
            return Ok(true);
        }
        if matches!(code, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space) {
            self.activate(&target)?;
            return Ok(true);
        }
        if let Some(Widget::Slider {
            min,
            max,
            step,
            value,
            ..
        }) = self.widget(&target)
        {
            if matches!(
                code,
                KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::Home | KeyCode::End
            ) {
                let value = match code {
                    KeyCode::Home => min,
                    KeyCode::End => max,
                    KeyCode::ArrowLeft => (value - step).max(min),
                    _ => (value + step).min(max),
                };
                self.change(&target, EventValue::Number(value))?;
                return Ok(true);
            }
        }
        if matches!(code, KeyCode::ArrowUp | KeyCode::ArrowDown) {
            self.focus_next(code == KeyCode::ArrowUp);
            return Ok(true);
        }
        Ok(false)
    }
    fn escape(&mut self) -> bool {
        self.key_capture.take().is_some() || self.request_close()
    }
    fn invalidate_settings(&mut self) {
        self.settings_owner = None;
        self.clear_view();
        self.menu.clear();
        self.active = false;
        self.context = self.context.saturating_add(1);
        self.settings_close_requested = true;
    }
    fn clear_view(&mut self) {
        self.scene.clear();
        self.revisions.clear();
        self.events.clear();
        self.pressed = None;
        self.touch = None;
        self.captured = false;
        self.settings_panel_seen = false;
        self.settings_close_intent = false;
        self.focus = None;
        self.last_input_keyboard = false;
        self.tooltip_started.clear();
        self.key_capture = None;
        self.editors.clear();
        self.scroll.clear();
        self.scrollbar_drag = None;
    }
    fn hit(&self) -> Option<(String, u32)> {
        if self.settings_owner.is_some()
            && (self.pointer[0] < 0
                || self.pointer[1] < 0
                || self.pointer[0] >= self.viewport[0] as i32
                || self.pointer[1] >= self.viewport[1].saturating_add(32) as i32)
        {
            return None;
        }
        self.scene.iter().rev().find_map(|(owner, ui)| {
            let bounds = ui.content_bounds(self.viewport);
            self.resolved(owner, ui)
                .widgets
                .iter()
                .rev()
                .find(|w| {
                    let clip = if ui.is_footer(w.id()) {
                        ui.footer_bounds(self.viewport)
                    } else {
                        bounds
                    };
                    w.interactive()
                        && w.contains(self.pointer)
                        && self.pointer[0] >= clip[0]
                        && self.pointer[0] < clip[2]
                        && self.pointer[1] >= clip[1]
                        && self.pointer[1] < clip[3]
                })
                .map(|w| (owner.clone(), w.id()))
        })
    }
    fn queue(&mut self, target: &(String, u32), kind: EventKind) -> Result<(), String> {
        self.queue_value(target, kind, None)
    }
    fn queue_value(
        &mut self,
        target: &(String, u32),
        kind: EventKind,
        value: Option<EventValue>,
    ) -> Result<(), String> {
        let events = self.events.entry(target.0.clone()).or_default();
        if kind == EventKind::Move || kind == EventKind::Change {
            events.retain(|e| e.kind != kind || e.widget != target.1);
        }
        if events.len() >= MAX_EVENTS {
            return Err("App input backlog limit".into());
        }
        self.event_sequence = self
            .event_sequence
            .checked_add(1)
            .ok_or("App event sequence exhausted")?;
        events.push(Event {
            sequence: self.event_sequence,
            widget: target.1,
            kind,
            position: self.pointer,
            value,
        });
        Ok(())
    }
    fn press(&mut self) -> Result<bool, String> {
        if let Some((owner, mut area)) = self.scene.iter().rev().find_map(|(owner, _)| {
            let mut area = self.scrollbar(owner)?;
            area.scrollbar_clicked(
                self.pointer[0] as f64,
                self.pointer[1] as f64,
                area.viewport(),
            )
            .then_some((owner.clone(), area))
        }) {
            area.scrolling = true;
            self.scrollbar_drag = Some((owner, area));
            self.pressed = None;
            self.captured = true;
            return Ok(true);
        }
        let Some(target) = self.hit() else {
            self.focus = None;
            return Ok(false);
        };
        self.queue(&target, EventKind::Press)?;
        self.pressed = Some(target);
        self.focus = self.pressed.clone();
        self.sync_editors();
        if let Some(target) = self.pressed.clone() {
            if let (Some(font), Some(editor)) = (&self.font, self.editors.get_mut(&target)) {
                editor.on_click(font.as_ref(), self.pointer[0] as f64, self.shift, false);
            }
            self.slider_pointer(&target)?;
        }
        self.captured = true;
        Ok(true)
    }
    fn release(&mut self) -> Result<bool, String> {
        let captured = std::mem::take(&mut self.captured);
        if let Some((_, mut area)) = self.scrollbar_drag.take() {
            area.scrollbar_released();
            return Ok(true);
        }
        let Some(target) = self.pressed.take() else {
            return Ok(captured);
        };
        self.queue(&target, EventKind::Release)?;
        if self.hit().as_ref() == Some(&target) {
            self.activate(&target)?;
        }
        Ok(true)
    }
    fn move_to(&mut self, x: f64, y: f64, scale: f64) -> Result<bool, String> {
        let previous_y = self.pointer[1];
        self.pointer = [
            (x / scale).clamp(-8192.0, 8192.0) as i32,
            (y / scale).clamp(-8192.0, 8192.0) as i32,
        ];
        if let Some((owner, area)) = &mut self.scrollbar_drag {
            area.scrollbar_dragged(
                self.pointer[1] as f64,
                (self.pointer[1] - previous_y) as f64,
                area.viewport(),
            );
            self.scroll.insert(owner.clone(), area.scroll_amount as i32);
            self.sync_editors();
            return Ok(true);
        }
        if let Some(target) = self.pressed.clone().or_else(|| self.hit()) {
            self.queue(&target, EventKind::Move)?;
            if self.pressed.is_some() {
                self.slider_pointer(&target)?;
                if let (Some(font), Some(editor)) = (&self.font, self.editors.get_mut(&target)) {
                    editor.on_drag(font.as_ref(), self.pointer[0] as f64);
                }
            }
        }
        Ok(self.captured)
    }
}
pub fn handle(event: &WindowEvent, scale: f64) -> Result<bool, String> {
    let binding_release = SURFACE.with(|cell| {
        let mut s = cell.borrow_mut();
        match event {
            WindowEvent::Focused(false) => {
                s.key_capture = None;
                s.binding_mouse_releases.clear();
                false
            }
            WindowEvent::MouseInput { button, state, .. } => match state {
                ElementState::Released => s.binding_mouse_releases.remove(button),
                ElementState::Pressed => s.binding_mouse_releases.contains(button),
            },
            _ => false,
        }
    });
    if binding_release {
        return Ok(true);
    }
    if !SURFACE.with(|cell| cell.borrow().active) {
        return Ok(false);
    }
    if !is_settings_open() && !super::hooks::menu_ui_visible() {
        SURFACE.with(|cell| {
            let mut s = cell.borrow_mut();
            s.pressed = None;
            s.scrollbar_drag = None;
            s.touch = None;
            s.captured = false;
            s.key_capture = None;
            s.events.clear();
        });
        return Ok(false);
    }
    if !scale.is_finite() || scale <= 0.0 {
        return Err("Invalid app UI scale".into());
    }
    SURFACE.with(|cell| {
        let mut s = cell.borrow_mut();
        if !s.active {
            return Ok(false);
        }
        s.observe_navigation_input(event);
        match event {
            WindowEvent::ModifiersChanged(modifiers) => {
                s.shift = modifiers.state().shift_key();
                s.ctrl = modifiers.state().control_key() || modifiers.state().super_key();
                Ok(false)
            }
            WindowEvent::KeyboardInput { event, .. } => s.keyboard(event),
            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                let Some(target) = s.focus.clone() else {
                    return Ok(false);
                };
                let Some(font) = s.font.clone() else {
                    return Ok(false);
                };
                let Some(editor) = s.editors.get_mut(&target) else {
                    return Ok(false);
                };
                editor.insert_text(font.as_ref(), text);
                let value = editor.value();
                s.change(&target, EventValue::Text(value))?;
                Ok(true)
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let y = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => *y as f64,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y / 15.0,
                };
                Ok(s.wheel(y))
            }
            WindowEvent::CursorMoved { position, .. } => s.move_to(position.x, position.y, scale),
            WindowEvent::MouseInput { button, state, .. } => {
                if *state == ElementState::Pressed
                    && let Some(target) = s.key_capture.take()
                {
                    if !matches!(
                        s.widget(&target),
                        Some(Widget::KeyBinding { enabled: true, .. })
                    ) {
                        return Ok(false);
                    }
                    s.binding_mouse_releases.insert(*button);
                    s.pressed = None;
                    s.scrollbar_drag = None;
                    s.touch = None;
                    s.captured = false;
                    s.change(
                        &target,
                        EventValue::Text(crate::input::button_state::mouse_name(*button)),
                    )?;
                    return Ok(true);
                }
                if *button == MouseButton::Left {
                    if *state == ElementState::Pressed {
                        s.press()
                    } else {
                        s.release()
                    }
                } else {
                    Ok(false)
                }
            }
            WindowEvent::Touch(touch) => {
                if s.touch.is_some_and(|id| id != touch.id) {
                    return Ok(false);
                }
                s.move_to(touch.location.x, touch.location.y, scale)?;
                match touch.phase {
                    TouchPhase::Started => {
                        let consumed = s.press()?;
                        if consumed {
                            s.touch = Some(touch.id);
                        }
                        Ok(consumed)
                    }
                    TouchPhase::Ended => {
                        s.touch = None;
                        s.release()
                    }
                    TouchPhase::Cancelled => {
                        s.touch = None;
                        s.pressed = None;
                        s.scrollbar_drag = None;
                        Ok(std::mem::take(&mut s.captured))
                    }
                    TouchPhase::Moved => Ok(s.touch.is_some()),
                }
            }
            WindowEvent::Focused(false) => {
                s.pressed = None;
                s.scrollbar_drag = None;
                s.touch = None;
                s.captured = false;
                Ok(false)
            }
            _ => Ok(false),
        }
    })
}
