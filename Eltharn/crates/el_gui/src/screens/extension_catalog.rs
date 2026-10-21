use crate::components::{
    Button,
    scroll_area::{AbstractScrollArea, ScrollbarSettings},
};
use crate::framework::screen::ScreenBase;
use crate::{FontWidthMeasurer, GuiGraphicsExtractor};
use el_core::{Component, ResourceLocation};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionCatalogKind {
    Mods,
    ShaderPacks,
}
impl ExtensionCatalogKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::Mods => "Mods",
            Self::ShaderPacks => "Shader Packs",
        }
    }
}
pub(super) fn cycle_extension_focus(
    current: Option<ExtensionCatalogKind>,
    mods: bool,
    shaders: bool,
    reverse: bool,
) -> Option<ExtensionCatalogKind> {
    let kinds: Vec<_> = [
        (mods, ExtensionCatalogKind::Mods),
        (shaders, ExtensionCatalogKind::ShaderPacks),
    ]
    .into_iter()
    .filter_map(|(enabled, kind)| enabled.then_some(kind))
    .collect();
    if kinds.is_empty() {
        return None;
    }
    let index = current.and_then(|current| kinds.iter().position(|kind| *kind == current));
    Some(
        kinds[match index {
            Some(index) if reverse => (index + kinds.len() - 1) % kinds.len(),
            Some(index) => (index + 1) % kinds.len(),
            None if reverse => kinds.len() - 1,
            None => 0,
        }],
    )
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExtensionCatalog {
    pub mods_enabled: bool,
    pub shaderpacks_enabled: bool,
    pub mods: Vec<ExtensionCatalogEntry>,
    pub shaderpacks: Vec<ExtensionCatalogEntry>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExtensionCatalogEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub content_id: String,
    pub description: String,
    pub status: String,
    pub selected: bool,
    pub settings_owner: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtensionCatalogAction {
    Back,
    OpenSettings(String),
    OpenFolder(ExtensionCatalogKind),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionCatalogKey {
    Previous,
    Next,
    Home,
    End,
    Tab,
    Activate,
}
pub struct ExtensionCatalogScreen {
    pub kind: ExtensionCatalogKind,
    pub in_world: bool,
    entries: Vec<ExtensionCatalogEntry>,
    selected: Option<usize>,
    focus: usize,
    width: i32,
    height: i32,
    base: ScreenBase,
    scroll: AbstractScrollArea,
    last_mouse_y: f32,
}
const HEADER: i32 = 33;
const FOOTER: i32 = 60;
const ROW_HEIGHT: i32 = 44;
impl ExtensionCatalogScreen {
    pub fn new(
        kind: ExtensionCatalogKind,
        in_world: bool,
        entries: Vec<ExtensionCatalogEntry>,
    ) -> Self {
        let selected = (!entries.is_empty()).then_some(0);
        let mut result = Self {
            kind,
            in_world,
            entries,
            selected,
            focus: 0,
            width: 320,
            height: 240,
            base: ScreenBase::new(Component::literal(kind.title())),
            scroll: AbstractScrollArea::new(
                0,
                HEADER,
                320,
                147,
                Component::literal(""),
                ScrollbarSettings::default_settings(22),
            ),
            last_mouse_y: 0.0,
        };
        result.resize();
        result
    }
    pub fn replace_entries(&mut self, entries: Vec<ExtensionCatalogEntry>) {
        let key = self
            .selected_entry()
            .map(|e| (e.id.clone(), e.content_id.clone()));
        self.entries = entries;
        self.selected = key
            .and_then(|key| {
                self.entries
                    .iter()
                    .position(|e| e.id == key.0 && e.content_id == key.1)
            })
            .or_else(|| (!self.entries.is_empty()).then_some(0));
        self.resize();
    }
    pub fn page(&self) -> usize {
        (self.scroll.scroll_amount as usize) << 16 | self.selected.unwrap_or(0)
    }
    pub fn set_size(&mut self, width: i32, height: i32) {
        let (width, height) = (width.max(40), height.max(80));
        if (width, height) != (self.width, self.height) {
            self.width = width;
            self.height = height;
            self.resize();
        }
    }
    fn list_height(&self) -> i32 {
        (self.height - HEADER - FOOTER).max(0)
    }
    fn row_width(&self) -> i32 {
        (self.width - 32).clamp(1, 310)
    }
    fn row_left(&self) -> i32 {
        (self.width - self.row_width()) / 2
    }
    fn resize(&mut self) {
        self.scroll.base.set_width(self.width);
        self.scroll.base.set_height(self.list_height());
        self.scroll.content_height = self.entries.len() as i32 * ROW_HEIGHT + 4;
        self.scroll.clamp_scroll(self.list_height());
        self.base.init(self.width, self.height);
        self.rebuild_footer();
    }
    fn selected_entry(&self) -> Option<&ExtensionCatalogEntry> {
        self.selected.and_then(|i| self.entries.get(i))
    }
    fn controls(&self) -> Vec<Control> {
        let folder = cfg!(all(not(target_arch = "wasm32"), not(target_os = "android")));
        let count = 1 + i32::from(self.kind == ExtensionCatalogKind::Mods) + i32::from(folder);
        let total = (self.width - 24).clamp(count, 310);
        let width = (total - (count - 1) * 8).max(count) / count;
        let left = (self.width - total) / 2;
        let y = self.height - 27;
        let mut controls = Vec::new();
        if folder {
            controls.push(Control {
                x: left,
                y,
                width,
                label: "Open Folder",
                action: ExtensionCatalogAction::OpenFolder(self.kind),
                enabled: true,
            });
        }
        if self.kind == ExtensionCatalogKind::Mods {
            let owner = self
                .selected_entry()
                .and_then(|e| e.settings_owner.clone())
                .filter(|o| !o.is_empty());
            controls.push(Control {
                x: left + controls.len() as i32 * (width + 8),
                y,
                width,
                label: "Settings",
                enabled: owner.is_some(),
                action: ExtensionCatalogAction::OpenSettings(owner.unwrap_or_default()),
            });
        }
        controls.push(Control {
            x: left + (count - 1) * (width + 8),
            y,
            width,
            label: "Done",
            enabled: true,
            action: ExtensionCatalogAction::Back,
        });
        controls
    }
    fn rebuild_footer(&mut self) {
        self.base.clear_widgets();
        for c in self.controls() {
            let mut button = Button::<String>::builder(Component::literal(c.label))
                .bounds(c.x, c.y, c.width, 20)
                .build();
            button.base.active = c.enabled;
            self.base.add_widget(button);
        }
        self.focus = self.focus.min(self.controls().len());
        self.base.set_focused(self.focus.checked_sub(1));
    }
    fn select(&mut self, index: usize) {
        if self.entries.is_empty() {
            return;
        }
        let index = index.min(self.entries.len() - 1);
        self.selected = Some(index);
        let top = index as i32 * ROW_HEIGHT;
        let bottom = top + ROW_HEIGHT + 4;
        let scroll = self.scroll.scroll_amount;
        let height = self.list_height();
        if (top as f32) < scroll {
            self.scroll.set_scroll_amount(top as f32, height);
        } else if bottom as f32 > scroll + height as f32 {
            self.scroll
                .set_scroll_amount((bottom - height) as f32, height);
        }
        self.focus = 0;
        self.rebuild_footer();
    }
    pub fn mouse_scrolled(&mut self, delta: f64) {
        if delta.is_finite() && delta != 0.0 {
            self.scroll.scroll(delta as f32, self.list_height());
        }
    }
    pub fn mouse_moved(&mut self, _x: f32, y: f32) {
        let dy = y - self.last_mouse_y;
        self.last_mouse_y = y;
        self.scroll
            .scrollbar_dragged(y as f64, dy as f64, self.list_height());
    }
    pub fn mouse_released(&mut self) {
        self.scroll.scrollbar_released();
    }
    pub fn key_pressed(
        &mut self,
        key: ExtensionCatalogKey,
        reverse: bool,
    ) -> Option<ExtensionCatalogAction> {
        match key {
            ExtensionCatalogKey::Previous => {
                self.select(self.selected.unwrap_or(0).saturating_sub(1))
            }
            ExtensionCatalogKey::Next => self.select(self.selected.map_or(0, |i| i + 1)),
            ExtensionCatalogKey::Home => self.select(0),
            ExtensionCatalogKey::End => self.select(self.entries.len().saturating_sub(1)),
            ExtensionCatalogKey::Tab => {
                let count = self.controls().len() + 1;
                loop {
                    self.focus = if reverse {
                        (self.focus + count - 1) % count
                    } else {
                        (self.focus + 1) % count
                    };
                    if self.focus == 0 || self.controls()[self.focus - 1].enabled {
                        break;
                    }
                }
                self.base.set_focused(self.focus.checked_sub(1));
            }
            ExtensionCatalogKey::Activate => {
                let control = if self.focus == 0 {
                    self.controls().into_iter().find(|c| {
                        matches!(c.action, ExtensionCatalogAction::OpenSettings(_)) && c.enabled
                    })?
                } else {
                    self.controls().get(self.focus - 1).cloned()?
                };
                if control.enabled {
                    crate::components::click_sound_sink::record_button_click();
                    return Some(control.action);
                }
            }
        }
        None
    }
    pub fn mouse_clicked(&mut self, x: f32, y: f32) -> Option<ExtensionCatalogAction> {
        if self
            .scroll
            .scrollbar_clicked(x as f64, y as f64, self.list_height())
        {
            return None;
        }
        if y >= HEADER as f32
            && y < (HEADER + self.list_height()) as f32
            && x >= self.row_left() as f32
            && x < (self.row_left() + self.row_width()) as f32
        {
            let index = ((y - HEADER as f32 - 2.0 + self.scroll.scroll_amount) / ROW_HEIGHT as f32)
                .floor() as usize;
            if index < self.entries.len() {
                self.selected = Some(index);
                self.focus = 0;
                self.rebuild_footer();
            }
            return None;
        }
        let (index, c) = self.controls().into_iter().enumerate().find(|(_, c)| {
            c.enabled
                && x >= c.x as f32
                && x < (c.x + c.width) as f32
                && y >= c.y as f32
                && y < (c.y + 20) as f32
        })?;
        self.focus = index + 1;
        self.base.set_focused(Some(index));
        crate::components::click_sound_sink::record_button_click();
        Some(c.action)
    }
    pub fn extract_render_state(
        &mut self,
        g: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
    ) {
        self.set_size(g.gui_width(), g.gui_height());
        let (w, h) = (self.width, self.height);
        self.base.extract_background_backdrop(g, self.in_world);
        self.scroll.extract_list_background(g, self.in_world);
        g.draw_menu_background(self.in_world, w, HEADER);
        g.pose_mut().push_pose();
        g.pose_mut().translate(0.0, (h - FOOTER) as f32);
        g.draw_menu_background(self.in_world, w, FOOTER);
        g.pose_mut().pop_pose();
        for (sprite, y) in [
            (
                if self.in_world {
                    "minecraft:gui/inworld_header_separator"
                } else {
                    "minecraft:gui/header_separator"
                },
                HEADER - 2,
            ),
            (
                if self.in_world {
                    "minecraft:gui/inworld_footer_separator"
                } else {
                    "minecraft:gui/footer_separator"
                },
                h - FOOTER - 2,
            ),
        ] {
            g.blit(ResourceLocation::from_string(sprite), 0, y, w, 2);
        }
        centered(g, font, self.kind.title(), w, 12, 0xffffffff);
        let bottom = HEADER + self.list_height();
        g.enable_scissor(0, HEADER, w, bottom);
        if self.entries.is_empty() {
            let (title, message) = match self.kind {
                ExtensionCatalogKind::Mods => (
                    "No mods installed",
                    "Place mod ZIP files in the Mods folder.",
                ),
                ExtensionCatalogKind::ShaderPacks => (
                    "No shader packs installed",
                    "Place packs in the Shader Packs folder.",
                ),
            };
            let y = HEADER + self.list_height() / 2;
            centered(g, font, title, w, y - 10, 0xffffffff);
            centered(
                g,
                font,
                &fit(font, message, (w - 24).max(1)),
                w,
                y + 5,
                0xffaaaaaa,
            );
        }
        let left = self.row_left();
        let rw = self.row_width();
        for (index, e) in self.entries.iter().enumerate() {
            let y = HEADER + 2 + index as i32 * ROW_HEIGHT - self.scroll.scroll_amount as i32;
            if y + ROW_HEIGHT <= HEADER || y >= bottom {
                continue;
            }
            if self.selected == Some(index) {
                g.fill(
                    left - 2,
                    y - 2,
                    left + rw + 2,
                    y + ROW_HEIGHT - 2,
                    if self.focus == 0 {
                        0xffffffffu32 as i32
                    } else {
                        0xff808080u32 as i32
                    },
                );
                g.fill(
                    left - 1,
                    y - 1,
                    left + rw + 1,
                    y + ROW_HEIGHT - 3,
                    0xff000000u32 as i32,
                );
            }
            let name = if e.name.is_empty() { &e.id } else { &e.name };
            let version = if e.version.is_empty() {
                String::new()
            } else {
                format!("v{}", e.version)
            };
            let vw = (font.measure_width(&version, 1.0).ceil() as i32).min(rw / 3);
            g.text_with_shadow(
                default_font(),
                fit(font, name, (rw - vw - 16).max(1)),
                left + 5,
                y + 3,
                -1,
                true,
            );
            if !version.is_empty() {
                g.text(
                    default_font(),
                    fit(font, &version, rw / 3),
                    left + rw - vw - 5,
                    y + 3,
                    0xffaaaaaau32 as i32,
                );
            }
            g.text(
                default_font(),
                fit(font, &e.description, (rw - 10).max(1)),
                left + 5,
                y + 16,
                0xff808080u32 as i32,
            );
            g.text(
                default_font(),
                fit(font, &e.status, (rw - 10).max(1)),
                left + 5,
                y + 29,
                if e.selected {
                    0xff55ff55u32 as i32
                } else {
                    0xffaaaaaau32 as i32
                },
            );
        }
        g.disable_scissor();
        self.scroll.extract_scrollbar(g);
        let summary = self
            .selected_entry()
            .filter(|e| !e.description.is_empty())
            .map(|e| e.description.clone())
            .unwrap_or_else(|| format!("{} installed", self.entries.len()));
        centered(
            g,
            font,
            &fit(font, &summary, (w - 24).max(1)),
            w,
            h - 49,
            0xffaaaaaa,
        );
        self.base
            .extract_render_state(g, g.mouse_x(), g.mouse_y(), 0.0);
    }
}
#[derive(Clone)]
struct Control {
    x: i32,
    y: i32,
    width: i32,
    label: &'static str,
    enabled: bool,
    action: ExtensionCatalogAction,
}
fn default_font() -> ResourceLocation {
    ResourceLocation::from_string("minecraft:default")
}
fn centered(
    g: &mut GuiGraphicsExtractor,
    font: &dyn FontWidthMeasurer,
    text: &str,
    width: i32,
    y: i32,
    color: u32,
) {
    g.text(
        default_font(),
        text.to_owned(),
        (width - font.measure_width(text, 1.0).round() as i32) / 2,
        y,
        color as i32,
    );
}
fn fit(font: &dyn FontWidthMeasurer, text: &str, width: i32) -> String {
    let clean: String = text
        .chars()
        .filter(|c| !c.is_control())
        .take(1024)
        .collect();
    if font.measure_width(&clean, 1.0) <= width as f32 {
        return clean;
    }
    let mut out = String::new();
    for ch in clean.chars() {
        if font.measure_width(&format!("{out}{ch}..."), 1.0) > width as f32 {
            break;
        }
        out.push(ch);
    }
    if width >= font.measure_width("...", 1.0) as i32 {
        out.push_str("...")
    }
    out
}
