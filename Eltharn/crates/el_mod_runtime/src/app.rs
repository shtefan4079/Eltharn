use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const READ_CAP: &str = "client.app.read";
pub const WRITE_CAP: &str = "client.app.write";
pub const MAX_WIDGETS: usize = 32;
pub const MAX_EVENTS: usize = 32;
pub const MAX_FOOTER_BUTTONS: usize = 3;
pub const SETTINGS_MENU: &str = "mod_settings";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonStyle {
    #[default]
    Flat,
    Vanilla,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Widget {
    Image {
        id: u32,
        rect: [i32; 4],
        texture: String,
        #[serde(default = "full_uv")]
        uv: [u16; 4],
        #[serde(default = "opaque_white")]
        color: u32,
    },
    Panel {
        id: u32,
        rect: [i32; 4],
        color: u32,
    },
    Label {
        id: u32,
        rect: [i32; 4],
        text: String,
        color: u32,
    },
    Button {
        id: u32,
        rect: [i32; 4],
        text: String,
        color: u32,
        #[serde(default)]
        style: ButtonStyle,
    },
    Action {
        id: u32,
        rect: [i32; 4],
        text: String,
        enabled: bool,
    },
    Toggle {
        id: u32,
        rect: [i32; 4],
        text: String,
        enabled: bool,
        value: bool,
    },
    Slider {
        id: u32,
        rect: [i32; 4],
        text: String,
        enabled: bool,
        value: i32,
        min: i32,
        max: i32,
        step: i32,
    },
    TextField {
        id: u32,
        rect: [i32; 4],
        text: String,
        enabled: bool,
        value: String,
        max_length: usize,
    },
    KeyBinding {
        id: u32,
        rect: [i32; 4],
        text: String,
        enabled: bool,
        value: String,
    },
}
impl Widget {
    pub fn id(&self) -> u32 {
        match self {
            Self::Image { id, .. }
            | Self::Panel { id, .. }
            | Self::Label { id, .. }
            | Self::Button { id, .. }
            | Self::Action { id, .. }
            | Self::Toggle { id, .. }
            | Self::Slider { id, .. }
            | Self::TextField { id, .. }
            | Self::KeyBinding { id, .. } => *id,
        }
    }
    pub fn rect(&self) -> [i32; 4] {
        match self {
            Self::Image { rect, .. }
            | Self::Panel { rect, .. }
            | Self::Label { rect, .. }
            | Self::Button { rect, .. }
            | Self::Action { rect, .. }
            | Self::Toggle { rect, .. }
            | Self::Slider { rect, .. }
            | Self::TextField { rect, .. }
            | Self::KeyBinding { rect, .. } => *rect,
        }
    }
    pub fn contains(&self, [x, y]: [i32; 2]) -> bool {
        let [left, top, width, height] = self.rect();
        x >= left && y >= top && x < left + width && y < top + height
    }
    pub fn set_rect(&mut self, value: [i32; 4]) {
        match self {
            Self::Image { rect, .. }
            | Self::Panel { rect, .. }
            | Self::Label { rect, .. }
            | Self::Button { rect, .. }
            | Self::Action { rect, .. }
            | Self::Toggle { rect, .. }
            | Self::Slider { rect, .. }
            | Self::TextField { rect, .. }
            | Self::KeyBinding { rect, .. } => *rect = value,
        }
    }
    pub fn interactive(&self) -> bool {
        match self {
            Self::Button { .. } => true,
            Self::Action { enabled, .. }
            | Self::Toggle { enabled, .. }
            | Self::Slider { enabled, .. }
            | Self::TextField { enabled, .. }
            | Self::KeyBinding { enabled, .. } => *enabled,
            _ => false,
        }
    }
    pub fn same_interaction(&self, other: &Self) -> bool {
        if self.id() != other.id()
            || self.rect() != other.rect()
            || self.interactive() != other.interactive()
        {
            return false;
        }
        match (self, other) {
            (Self::Action { .. }, Self::Action { .. })
            | (Self::Toggle { .. }, Self::Toggle { .. })
            | (Self::KeyBinding { .. }, Self::KeyBinding { .. }) => true,
            (
                Self::Slider { min, max, step, .. },
                Self::Slider {
                    min: bmin,
                    max: bmax,
                    step: bstep,
                    ..
                },
            ) => (min, max, step) == (bmin, bmax, bstep),
            (
                Self::TextField { max_length, .. },
                Self::TextField {
                    max_length: other, ..
                },
            ) => max_length == other,
            _ => self == other,
        }
    }
}
fn full_uv() -> [u16; 4] {
    [0, 0, 10_000, 10_000]
}
fn opaque_white() -> u32 {
    0xffff_ffff
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuLayout {
    pub version: u32,
    pub title: String,
    pub columns: u8,
    #[serde(default)]
    pub close_request: bool,
    #[serde(default)]
    pub footer: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub presentation: Vec<ControlPresentation>,
}
impl MenuLayout {
    pub fn same_interaction(&self, other: &Self) -> bool {
        self.version == other.version
            && self.title == other.title
            && self.columns == other.columns
            && self.close_request == other.close_request
            && self.footer == other.footer
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPresentation {
    pub id: u32,
    #[serde(default)]
    pub invalid: bool,
    #[serde(default)]
    pub suggestion: String,
    #[serde(default)]
    pub tooltip: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MenuControl {
    Label {
        id: u32,
        text: String,
    },
    Button {
        id: u32,
        text: String,
        #[serde(default = "enabled_default")]
        enabled: bool,
    },
    Toggle {
        id: u32,
        text: String,
        value: bool,
        #[serde(default = "enabled_default")]
        enabled: bool,
    },
    Slider {
        id: u32,
        text: String,
        value: i32,
        min: i32,
        max: i32,
        #[serde(default = "step_default")]
        step: i32,
        #[serde(default = "enabled_default")]
        enabled: bool,
    },
    TextField {
        id: u32,
        text: String,
        value: String,
        #[serde(default = "length_default")]
        max_length: usize,
        #[serde(default = "enabled_default")]
        enabled: bool,
    },
    KeyBinding {
        id: u32,
        text: String,
        value: String,
        #[serde(default = "enabled_default")]
        enabled: bool,
    },
}
fn enabled_default() -> bool {
    true
}
fn step_default() -> i32 {
    1
}
fn length_default() -> usize {
    128
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Menu {
    pub version: u32,
    pub title: String,
    #[serde(default = "step_default")]
    pub columns: i32,
    pub controls: Vec<MenuControl>,
    #[serde(default)]
    pub close_request: bool,
    #[serde(default)]
    pub footer: Vec<MenuControl>,
    #[serde(default)]
    pub presentation: Vec<ControlPresentation>,
}
impl Menu {
    pub fn into_ui(self) -> Result<Ui, String> {
        if !matches!(self.version, 1 | 2) || !(1..=2).contains(&self.columns) {
            return Err("Unsupported menu version or columns".into());
        }
        if self.footer.len() > MAX_FOOTER_BUTTONS {
            return Err("Menu footer button limit".into());
        }
        let footer = self
            .footer
            .iter()
            .map(|control| match control {
                MenuControl::Button { id, .. } => Ok(*id),
                _ => Err("Menu footer supports buttons only"),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let widgets = self
            .controls
            .into_iter()
            .chain(self.footer)
            .map(|control| {
                let rect = [0, 0, 1, 1];
                match control {
                    MenuControl::Label { id, text } => Widget::Label {
                        id,
                        rect,
                        text,
                        color: 0xffaaaaaa,
                    },
                    MenuControl::Button { id, text, enabled } => Widget::Action {
                        id,
                        rect,
                        text,
                        enabled,
                    },
                    MenuControl::Toggle {
                        id,
                        text,
                        value,
                        enabled,
                    } => Widget::Toggle {
                        id,
                        rect,
                        text,
                        value,
                        enabled,
                    },
                    MenuControl::Slider {
                        id,
                        text,
                        value,
                        min,
                        max,
                        step,
                        enabled,
                    } => Widget::Slider {
                        id,
                        rect,
                        text,
                        value,
                        min,
                        max,
                        step,
                        enabled,
                    },
                    MenuControl::TextField {
                        id,
                        text,
                        value,
                        max_length,
                        enabled,
                    } => Widget::TextField {
                        id,
                        rect,
                        text,
                        value,
                        max_length,
                        enabled,
                    },
                    MenuControl::KeyBinding {
                        id,
                        text,
                        value,
                        enabled,
                    } => Widget::KeyBinding {
                        id,
                        rect,
                        text,
                        value,
                        enabled,
                    },
                }
            })
            .collect();
        let ui = Ui {
            widgets,
            layout: Some(MenuLayout {
                version: self.version,
                title: self.title,
                columns: self.columns as u8,
                close_request: self.close_request,
                footer,
                presentation: self.presentation,
            }),
        };
        ui.validate()?;
        Ok(ui)
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ui {
    pub widgets: Vec<Widget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<MenuLayout>,
}
impl Ui {
    pub fn presentation(&self, id: u32) -> Option<&ControlPresentation> {
        self.layout
            .as_ref()?
            .presentation
            .iter()
            .find(|p| p.id == id)
    }
    pub fn same_layout_interaction(&self, other: &Self) -> bool {
        match (&self.layout, &other.layout) {
            (Some(a), Some(b)) => a.same_interaction(b),
            (None, None) => true,
            _ => false,
        }
    }
    pub fn is_footer(&self, id: u32) -> bool {
        self.layout
            .as_ref()
            .is_some_and(|layout| layout.footer.contains(&id))
    }
    pub fn body_widget_count(&self) -> usize {
        self.widgets
            .iter()
            .filter(|widget| !self.is_footer(widget.id()))
            .count()
    }
    pub fn requests_close(&self) -> bool {
        self.layout
            .as_ref()
            .is_some_and(|layout| layout.version == 2 && layout.close_request)
    }
    pub fn validate_input_context(&self, input: &InputFrame) -> Result<(), String> {
        if (self.requests_close()
            || self
                .layout
                .as_ref()
                .is_some_and(|layout| !layout.footer.is_empty()))
            && (input.menu != SETTINGS_MENU || input.settings_session == 0)
        {
            return Err("Menu footer and close requests require selected settings".into());
        }
        Ok(())
    }
    pub fn validate_owner(&self, actor: &str) -> Result<(), String> {
        if !self
            .widgets
            .iter()
            .any(|w| matches!(w, Widget::Image { .. }))
        {
            return Ok(());
        }
        let owner: el_extension_api::ExtensionId = actor
            .split('/')
            .next()
            .unwrap_or_default()
            .parse()
            .map_err(|_| "Invalid app owner")?;
        for widget in &self.widgets {
            if let Widget::Image { texture, .. } = widget {
                let image: el_mod_content::ContentId =
                    texture.parse().map_err(|_| "Invalid app image")?;
                if image.namespace() != owner.namespace() {
                    return Err("Foreign app image namespace".into());
                }
            }
        }
        Ok(())
    }
    pub fn content_height(&self) -> i32 {
        self.layout.as_ref().map_or(0, |layout| {
            self.body_widget_count().div_ceil(layout.columns as usize) as i32 * 36 + 8
        })
    }
    pub fn content_bounds(&self, viewport: [u32; 2]) -> [i32; 4] {
        [
            0,
            if self.layout.is_some() {
                33.min(viewport[1] as i32)
            } else {
                0
            },
            viewport[0] as i32,
            viewport[1] as i32,
        ]
    }
    pub fn footer_bounds(&self, viewport: [u32; 2]) -> [i32; 4] {
        [
            0,
            viewport[1] as i32,
            viewport[0] as i32,
            viewport[1] as i32 + 32,
        ]
    }
    pub fn resolved(&self, viewport: [u32; 2], scroll: i32) -> Self {
        let mut result = self.clone();
        if let Some(layout) = &self.layout {
            let columns = if viewport[0] < 300 {
                1
            } else {
                layout.columns as i32
            };
            let width = (viewport[0] as i32 - 28).clamp(1, 310);
            let x = (viewport[0] as i32 - width) / 2;
            let cell = (width - (columns - 1) * 10) / columns;
            let footer_count = layout.footer.len() as i32;
            let footer_available = (viewport[0] as i32 - 28).max(1);
            let footer_gap = if footer_count > 1 {
                ((footer_available - footer_count).max(0) / (footer_count - 1)).min(8)
            } else {
                0
            };
            let footer_width = if footer_count > 0 {
                ((footer_available - (footer_count - 1) * footer_gap) / footer_count).clamp(1, 200)
            } else {
                0
            };
            let footer_x = ((viewport[0] as i32
                - footer_count * footer_width
                - (footer_count - 1).max(0) * footer_gap)
                / 2)
            .max(0);
            let mut index = 0;
            for widget in &mut result.widgets {
                if let Some(footer_index) = layout.footer.iter().position(|id| *id == widget.id()) {
                    widget.set_rect([
                        footer_x + footer_index as i32 * (footer_width + footer_gap),
                        viewport[1] as i32 + 6,
                        footer_width,
                        20,
                    ]);
                    continue;
                }
                let field_offset = if matches!(widget, Widget::TextField { .. }) {
                    10
                } else {
                    0
                };
                widget.set_rect([
                    x + index % columns * (cell + 10),
                    37 + index / columns * 36 - scroll + field_offset,
                    cell,
                    20,
                ]);
                index += 1;
            }
        }
        result
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.layout.as_ref().is_some_and(|layout| {
            !matches!(layout.version, 1 | 2)
                || !(1..=2).contains(&layout.columns)
                || layout.title.len() > 128
                || layout.title.chars().any(char::is_control)
                || layout.footer.len() > MAX_FOOTER_BUTTONS
                || (layout.version == 1 && (layout.close_request || !layout.footer.is_empty()))
                || layout.footer.iter().collect::<BTreeSet<_>>().len() != layout.footer.len()
                || layout.footer.iter().any(|id| {
                    !self
                        .widgets
                        .iter()
                        .any(|widget| matches!(widget, Widget::Action { id: action, .. } if action == id))
                })
        }) {
            return Err("Invalid menu layout".into());
        }
        if self.widgets.len() > MAX_WIDGETS {
            return Err("App widget limit".into());
        }
        if let Some(layout) = &self.layout {
            if layout.presentation.len() > MAX_WIDGETS
                || (layout.version == 1 && !layout.presentation.is_empty())
            {
                return Err("Invalid menu presentation count/version".into());
            }
            let mut decorated = BTreeSet::new();
            for p in &layout.presentation {
                let widget = self.widgets.iter().find(|widget| widget.id() == p.id);
                if p.id == 0
                    || !decorated.insert(p.id)
                    || !matches!(
                        widget,
                        Some(
                            Widget::Action { .. }
                                | Widget::Toggle { .. }
                                | Widget::Slider { .. }
                                | Widget::TextField { .. }
                                | Widget::KeyBinding { .. }
                        )
                    )
                    || ((p.invalid || !p.suggestion.is_empty())
                        && !matches!(widget, Some(Widget::TextField { .. })))
                    || p.suggestion.len() > 256
                    || p.suggestion.chars().any(char::is_control)
                    || p.tooltip.len() > 1024
                    || p.tooltip.lines().count() > 8
                    || p.tooltip.chars().any(|c| c.is_control() && c != '\n')
                {
                    return Err("Invalid native control presentation".into());
                }
            }
        }
        let mut ids = BTreeSet::new();
        for widget in &self.widgets {
            if widget.id() == 0 || !ids.insert(widget.id()) {
                return Err("App widget IDs must be unique and nonzero".into());
            }
            let [x, y, w, h] = widget.rect();
            if !(-8192..=8192).contains(&x)
                || !(-8192..=8192).contains(&y)
                || !(1..=8192).contains(&w)
                || !(1..=8192).contains(&h)
            {
                return Err("App widget rectangle limit".into());
            }
            if let Widget::Label { text, .. }
            | Widget::Button { text, .. }
            | Widget::Action { text, .. }
            | Widget::Toggle { text, .. }
            | Widget::Slider { text, .. }
            | Widget::TextField { text, .. }
            | Widget::KeyBinding { text, .. } = widget
            {
                if text.len() > 256 || text.chars().any(char::is_control) {
                    return Err("App label limit".into());
                }
            }
            match widget {
                Widget::Image { texture, uv, .. } => {
                    let id: el_mod_content::ContentId =
                        texture.parse().map_err(|_| "Invalid app image resource")?;
                    if id.path().ends_with(".png")
                        || uv.iter().any(|v| *v > 10_000)
                        || uv[0] >= uv[2]
                        || uv[1] >= uv[3]
                    {
                        return Err("Invalid app image UV/resource".into());
                    }
                }
                Widget::Slider {
                    value,
                    min,
                    max,
                    step,
                    ..
                } if *min < -1_000_000
                    || *max > 1_000_000
                    || min >= max
                    || step <= &0
                    || *step > *max - *min
                    || value < min
                    || value > max =>
                {
                    return Err("Invalid menu slider range".into());
                }
                Widget::TextField {
                    value, max_length, ..
                } if !(1..=1024).contains(max_length)
                    || value.chars().count() > *max_length
                    || value.len() > 4096
                    || value.chars().any(char::is_control) =>
                {
                    return Err("Invalid menu text field".into());
                }
                Widget::KeyBinding { value, .. }
                    if value.len() > 64 || !value.chars().all(|c| c.is_ascii_alphanumeric()) =>
                {
                    return Err("Invalid menu key binding".into());
                }
                _ => {}
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > crate::host::MAX_ITEM {
            return Err("App UI byte limit".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Press,
    Release,
    Click,
    Move,
    Change,
    Submit,
    CloseRequest,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventValue {
    Bool(bool),
    Number(i32),
    Text(String),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub sequence: u64,
    pub widget: u32,
    pub kind: EventKind,
    pub position: [i32; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<EventValue>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputFrame {
    pub sequence: u64,
    pub context: u64,
    pub ui_revision: u64,
    #[serde(default)]
    pub settings_session: u64,
    pub menu: String,
    pub viewport: [u32; 2],
    pub pointer: [i32; 2],
    pub events: Vec<Event>,
}
impl Default for InputFrame {
    fn default() -> Self {
        Self {
            sequence: 0,
            context: 0,
            ui_revision: 0,
            settings_session: 0,
            menu: "other".into(),
            viewport: [1, 1],
            pointer: [0, 0],
            events: Vec::new(),
        }
    }
}
impl InputFrame {
    pub fn validate(&self) -> Result<(), String> {
        if self.sequence > 9_007_199_254_740_991
            || self.context > 9_007_199_254_740_991
            || self.ui_revision > 9_007_199_254_740_991
            || self.settings_session > 9_007_199_254_740_991
            || (self.menu != SETTINGS_MENU && self.settings_session != 0)
            || !matches!(
                self.menu.as_str(),
                "title" | "multiplayer" | "options" | "connecting" | "other" | SETTINGS_MENU
            )
            || self.viewport.iter().any(|v| !(1..=8192).contains(v))
            || self.pointer.iter().any(|v| !(-8192..=8192).contains(v))
            || self.events.len() > MAX_EVENTS
        {
            return Err("Invalid app input".into());
        }
        let mut previous = 0;
        let mut close_requested = false;
        for event in &self.events {
            if event.kind == EventKind::CloseRequest {
                if close_requested {
                    return Err("Duplicate app close request".into());
                }
                close_requested = true;
            }
            if event.sequence <= previous
                || event.sequence > 9_007_199_254_740_991
                || if event.kind == EventKind::CloseRequest {
                    event.widget != 0
                        || event.value.is_some()
                        || self.menu != SETTINGS_MENU
                        || self.settings_session == 0
                } else {
                    event.widget == 0
                }
                || event.position.iter().any(|v| !(-8192..=8192).contains(v))
            {
                return Err("Invalid app input event".into());
            }
            previous = event.sequence;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub input: InputFrame,
    pub ui: Ui,
}
impl Snapshot {
    pub fn validate(&self) -> Result<(), String> {
        self.input.validate()?;
        self.ui.validate()
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub context: u64,
    pub menu: String,
    pub readers: Vec<String>,
    pub scene: BTreeMap<String, Ui>,
    pub revisions: BTreeMap<String, u64>,
}
#[derive(Clone, Default)]
pub(crate) struct State {
    pub inputs: BTreeMap<String, InputFrame>,
    pub scene: BTreeMap<String, Ui>,
    pub revisions: BTreeMap<String, u64>,
    sequence: u64,
    context: u64,
}
impl State {
    pub fn set_input(&mut self, frames: BTreeMap<String, InputFrame>) -> Result<(), String> {
        if frames.is_empty() {
            return Ok(());
        }
        if frames.len() > 16 {
            return Err("App input owner limit".into());
        }
        if frames.values().any(|frame| frame.menu == SETTINGS_MENU) && frames.len() != 1 {
            return Err("Settings input requires exactly one owner".into());
        }
        let sequence = frames.values().next().unwrap().sequence;
        if sequence <= self.sequence {
            return Err("Stale app input".into());
        }
        let context = frames.values().next().unwrap().context;
        if context < self.context {
            return Err("Stale app context".into());
        }
        let changed_context = context != self.context;
        if !changed_context && let Some(previous) = self.inputs.values().next() {
            let first = frames.values().next().unwrap();
            if previous.menu != first.menu || previous.viewport != first.viewport {
                return Err("App screen change requires a new context".into());
            }
            if previous.settings_session != first.settings_session {
                return Err("Settings session change requires a new context".into());
            }
            if first.menu == SETTINGS_MENU && self.inputs.keys().ne(frames.keys()) {
                return Err("Settings owner change requires a new context".into());
            }
        }
        for (owner, frame) in &frames {
            frame.validate()?;
            if frame.sequence != sequence {
                return Err("Mixed app input sequences".into());
            }
            let first = frames.values().next().unwrap();
            if frame.context != first.context
                || frame.menu != first.menu
                || frame.viewport != first.viewport
                || frame.settings_session != first.settings_session
            {
                return Err("Mixed app input contexts".into());
            }
            let revision = if changed_context {
                0
            } else {
                self.revisions.get(owner).copied().unwrap_or(0)
            };
            if frame.ui_revision != revision {
                return Err("Stale app UI revision".into());
            }
            for event in &frame.events {
                if event.kind == EventKind::CloseRequest {
                    if changed_context
                        || frame.menu != SETTINGS_MENU
                        || frame.settings_session == 0
                        || self.inputs.get(owner).is_none_or(|previous| {
                            previous.settings_session != frame.settings_session
                        })
                        || !self.scene.get(owner).is_some_and(Ui::requests_close)
                    {
                        return Err("App close request target no longer exists".into());
                    }
                    continue;
                }
                if changed_context
                    || !self.scene.get(owner).is_some_and(|ui| {
                        ui.widgets.iter().any(|w| {
                            w.id() == event.widget
                                && w.interactive()
                                && match (&event.kind, &event.value, w) {
                                    (
                                        EventKind::Change,
                                        Some(EventValue::Bool(_)),
                                        Widget::Toggle { .. },
                                    ) => true,
                                    (
                                        EventKind::Change,
                                        Some(EventValue::Text(value)),
                                        Widget::KeyBinding { .. },
                                    ) => {
                                        value.len() <= 64
                                            && value.chars().all(|c| c.is_ascii_alphanumeric())
                                    }
                                    (
                                        EventKind::Change,
                                        Some(EventValue::Number(value)),
                                        Widget::Slider { min, max, .. },
                                    ) => value >= min && value <= max,
                                    (
                                        EventKind::Change | EventKind::Submit,
                                        Some(EventValue::Text(value)),
                                        Widget::TextField { max_length, .. },
                                    ) => {
                                        value.chars().count() <= *max_length
                                            && value.len() <= 4096
                                            && !value.chars().any(char::is_control)
                                    }
                                    (EventKind::Change | EventKind::Submit, _, _) => false,
                                    (_, None, _) => true,
                                    _ => false,
                                }
                        })
                    })
                {
                    return Err("App event target no longer exists".into());
                }
            }
        }
        if changed_context {
            self.clear();
        }
        self.inputs = frames;
        self.sequence = sequence;
        self.context = context;
        Ok(())
    }
    pub fn apply_ui(&mut self, owner: &str, ui: &Ui) -> Result<(), String> {
        ui.validate()?;
        ui.validate_input_context(self.inputs.get(owner).unwrap_or(&InputFrame::default()))?;
        if self.scene.get(owner) != Some(ui) {
            let buttons_changed = self.scene.get(owner).is_none_or(|previous| {
                !previous
                    .widgets
                    .iter()
                    .filter(|widget| widget.interactive())
                    .zip(ui.widgets.iter().filter(|widget| widget.interactive()))
                    .all(|(old, new)| old.same_interaction(new))
                    || previous.widgets.iter().filter(|w| w.interactive()).count()
                        != ui.widgets.iter().filter(|w| w.interactive()).count()
                    || !previous.same_layout_interaction(ui)
                    || (ui.layout.is_some()
                        && previous
                            .widgets
                            .iter()
                            .map(Widget::id)
                            .ne(ui.widgets.iter().map(Widget::id)))
            });
            if buttons_changed {
                let revision = self
                    .revisions
                    .get(owner)
                    .copied()
                    .unwrap_or(0)
                    .checked_add(1)
                    .filter(|v| *v <= 9_007_199_254_740_991)
                    .ok_or("App UI revision exhausted")?;
                self.revisions.insert(owner.into(), revision);
            }
            self.scene.insert(owner.into(), ui.clone());
        }
        Ok(())
    }
    pub fn snapshot(&self, key: &str) -> Snapshot {
        Snapshot {
            input: self.inputs.get(key).cloned().unwrap_or_default(),
            ui: self.scene.get(key).cloned().unwrap_or_default(),
        }
    }
    pub fn finish_tick(&mut self) {
        for (owner, frame) in &mut self.inputs {
            frame.events.clear();
            frame.ui_revision = self.revisions.get(owner).copied().unwrap_or(0);
        }
    }
    pub fn clear(&mut self) {
        self.scene.clear();
        self.inputs.clear();
        self.revisions.clear();
    }
    pub fn disable(&mut self, key: &str) {
        self.scene.remove(key);
        self.inputs.remove(key);
        self.revisions.remove(key);
    }
}
