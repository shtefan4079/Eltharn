use crate::FontWidthMeasurer;
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::narration::{NarratedElementType, NarrationElementOutput};
use crate::framework::navigation::ScreenRectangle;
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::{Component, ResourceLocation};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
pub const WINDOW_WIDTH: i32 = 252;
pub const WINDOW_HEIGHT: i32 = 140;
pub const VIEWPORT_X: i32 = 9;
pub const VIEWPORT_Y: i32 = 18;
pub const VIEWPORT_WIDTH: i32 = 234;
pub const VIEWPORT_HEIGHT: i32 = 113;
pub const BACKGROUND_TILE_WIDTH: i32 = 16;
pub const BACKGROUND_TILE_HEIGHT: i32 = 16;
pub const SCROLL_INCREMENT: f64 = 16.0;
pub const NODE_FRAME_SIZE: i32 = 26;
pub const NODE_ICON_SIZE: i32 = 16;
pub const NODE_SPACING_X: f32 = 28.0;
pub const NODE_SPACING_Y: f32 = 27.0;
pub const DONE_WIDTH: i32 = 200;
const FOOTER_HEIGHT: i32 = 33;
const TITLE_X: i32 = 8;
const TITLE_Y: i32 = 6;
const TITLE_LINE_HEIGHT: i32 = 9;
const MAX_ROOT_TABS: usize = 26;
const TAB_SIDES: [AdvancementTabSide; 4] = [
    AdvancementTabSide::Above,
    AdvancementTabSide::Below,
    AdvancementTabSide::Left,
    AdvancementTabSide::Right,
];
const TAB_SIDE_CAPACITIES: [usize; 4] = [8, 8, 5, 5];
const TEST_SPLIT_OFFSETS: [i32; 5] = [0, 10, -10, 25, -25];
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdvancementFrame {
    Task,
    Challenge,
    Goal,
}
impl AdvancementFrame {
    pub fn frame_sprite_key(self, obtained: bool) -> String {
        let frame = match self {
            Self::Task => "task",
            Self::Challenge => "challenge",
            Self::Goal => "goal",
        };
        let state = if obtained { "obtained" } else { "unobtained" };
        format!("minecraft:advancements/{frame}_frame_{state}")
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct AdvancementDisplayData {
    pub title: String,
    pub description: String,
    pub icon_registry_id: String,
    pub icon_count: i32,
    pub icon_components: Option<serde_json::Value>,
    pub icon_sprite_key: Option<String>,
    pub frame: AdvancementFrame,
    pub hidden: bool,
    pub x: f32,
    pub y: f32,
    pub background_sprite_key: Option<String>,
}
impl AdvancementDisplayData {
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        icon_registry_id: impl Into<String>,
        frame: AdvancementFrame,
        x: f32,
        y: f32,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            icon_registry_id: icon_registry_id.into(),
            icon_count: 1,
            icon_components: None,
            icon_sprite_key: None,
            frame,
            hidden: false,
            x,
            y,
            background_sprite_key: None,
        }
    }
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }
    pub fn icon_sprite(mut self, key: impl Into<String>) -> Self {
        self.icon_sprite_key = Some(key.into());
        self
    }
    pub fn icon_stack(mut self, count: i32, components: Option<serde_json::Value>) -> Self {
        self.icon_count = count;
        self.icon_components = components;
        self
    }
    pub fn background_sprite(mut self, key: impl Into<String>) -> Self {
        self.background_sprite_key = Some(key.into());
        self
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct AdvancementNodeData {
    pub id: String,
    pub parent_id: Option<String>,
    pub display: Option<AdvancementDisplayData>,
    pub requirements: Vec<Vec<String>>,
}
impl AdvancementNodeData {
    pub fn new(
        id: impl Into<String>,
        parent_id: Option<String>,
        display: Option<AdvancementDisplayData>,
        requirements: Vec<Vec<String>>,
    ) -> Self {
        Self {
            id: id.into(),
            parent_id,
            display,
            requirements,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdvancementCriterionProgress {
    pub criterion_id: String,
    pub acquired_timestamp_millis: Option<i64>,
}
impl AdvancementCriterionProgress {
    pub fn new(criterion_id: impl Into<String>, acquired_timestamp_millis: Option<i64>) -> Self {
        Self {
            criterion_id: criterion_id.into(),
            acquired_timestamp_millis,
        }
    }
    pub fn acquired(&self) -> bool {
        self.acquired_timestamp_millis.is_some()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdvancementProgressData {
    pub advancement_id: String,
    pub criteria: Vec<AdvancementCriterionProgress>,
}
impl AdvancementProgressData {
    pub fn new(
        advancement_id: impl Into<String>,
        criteria: Vec<AdvancementCriterionProgress>,
    ) -> Self {
        Self {
            advancement_id: advancement_id.into(),
            criteria,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AdvancementsSnapshot {
    pub advancements: Vec<AdvancementNodeData>,
    pub progress: Vec<AdvancementProgressData>,
}
impl AdvancementsSnapshot {
    pub fn new(
        advancements: Vec<AdvancementNodeData>,
        progress: Vec<AdvancementProgressData>,
    ) -> Self {
        Self {
            advancements,
            progress,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AdvancementsUpdate {
    pub reset: bool,
    pub removed_ids: Vec<String>,
    pub added_or_replaced: Vec<AdvancementNodeData>,
    pub progress_updates: Vec<AdvancementProgressData>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AdvancementUpdateReport {
    pub ignored_advancement_ids: Vec<String>,
    pub ignored_progress_ids: Vec<String>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AdvancementProgressSummary {
    pub acquired_criteria: usize,
    pub total_criteria: usize,
    pub completed_requirements: usize,
    pub total_requirements: usize,
    pub percent: f32,
    pub done: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeenAdvancementsAction {
    OpenedTab(String),
    ClosedScreen,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdvancementsScreenAction {
    Done,
}
pub trait AdvancementIconRenderer: Send + Sync {
    fn extract_icon(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        display: &AdvancementDisplayData,
        x: i32,
        y: i32,
        size: i32,
    );
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdvancementTabSide {
    Above,
    Below,
    Left,
    Right,
}
impl AdvancementTabSide {
    fn width(self) -> i32 {
        match self {
            Self::Above | Self::Below => 28,
            Self::Left | Self::Right => 32,
        }
    }
    fn height(self) -> i32 {
        match self {
            Self::Above | Self::Below => 32,
            Self::Left | Self::Right => 28,
        }
    }
    fn sprite_side(self) -> &'static str {
        match self {
            Self::Above => "above",
            Self::Below => "below",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
    fn icon_offset(self) -> (i32, i32) {
        match self {
            Self::Above => (6, 9),
            Self::Below => (6, 6),
            Self::Left => (10, 5),
            Self::Right => (6, 5),
        }
    }
    fn rect(self, window: ScreenRectangle, local_index: usize) -> ScreenRectangle {
        let index = local_index as i32;
        let (x, y) = match self {
            Self::Above => (
                window.left() + (self.width() + 4) * index,
                window.top() - self.height() + 4,
            ),
            Self::Below => (
                window.left() + (self.width() + 4) * index,
                window.top() + 136,
            ),
            Self::Left => (
                window.left() - self.width() + 4,
                window.top() + self.height() * index,
            ),
            Self::Right => (window.left() + 248, window.top() + self.height() * index),
        };
        ScreenRectangle::new(x, y, self.width(), self.height())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct AdvancementTabPlacement {
    root_id: String,
    side: AdvancementTabSide,
    side_index: usize,
    side_count: usize,
    rect: ScreenRectangle,
}
#[derive(Clone, Debug)]
struct AdvancementWidgetText {
    title_lines: Vec<String>,
    description_lines: Vec<String>,
    width: i32,
    progress_text: Option<String>,
}
pub struct AdvancementsScreen {
    base: ScreenBase,
    font: Arc<dyn FontWidthMeasurer>,
    icon_renderer: Option<Arc<dyn AdvancementIconRenderer>>,
    advancements: Vec<AdvancementNodeData>,
    progress: HashMap<String, Vec<AdvancementCriterionProgress>>,
    selected_tab_id: Option<String>,
    scroll_x: f64,
    scroll_y: f64,
    centered: bool,
    dragging_scroll: bool,
    mouse_x: i32,
    mouse_y: i32,
    hovered_advancement_id: Option<String>,
    hover_fade: f32,
    pending_actions: VecDeque<AdvancementsScreenAction>,
    seen_advancement_actions: VecDeque<SeenAdvancementsAction>,
    in_world: bool,
}
impl AdvancementsScreen {
    pub fn new(snapshot: AdvancementsSnapshot, font: Arc<dyn FontWidthMeasurer>) -> Self {
        let mut screen = Self {
            base: ScreenBase::new(Component::literal(&crate::translate("gui.advancements"))),
            font,
            icon_renderer: None,
            advancements: Vec::new(),
            progress: HashMap::new(),
            selected_tab_id: None,
            scroll_x: 0.0,
            scroll_y: 0.0,
            centered: false,
            dragging_scroll: false,
            mouse_x: 0,
            mouse_y: 0,
            hovered_advancement_id: None,
            hover_fade: 0.0,
            pending_actions: VecDeque::new(),
            seen_advancement_actions: VecDeque::new(),
            in_world: true,
        };
        screen.replace_data(snapshot);
        screen
    }
    pub fn with_icon_renderer(mut self, renderer: Arc<dyn AdvancementIconRenderer>) -> Self {
        self.icon_renderer = Some(renderer);
        self
    }
    pub fn set_icon_renderer(&mut self, renderer: Option<Arc<dyn AdvancementIconRenderer>>) {
        self.icon_renderer = renderer;
    }
    pub fn set_in_world(&mut self, in_world: bool) {
        self.in_world = in_world;
    }
    pub fn advancements(&self) -> &[AdvancementNodeData] {
        &self.advancements
    }
    pub fn selected_tab_id(&self) -> Option<&str> {
        self.selected_tab_id.as_deref()
    }
    pub fn tab_ids(&self) -> Vec<String> {
        self.tab_placements()
            .iter()
            .map(|tab| tab.root_id.clone())
            .collect()
    }
    pub fn snapshot(&self) -> AdvancementsSnapshot {
        let mut progress = self
            .progress
            .iter()
            .map(|(advancement_id, criteria)| {
                AdvancementProgressData::new(advancement_id.clone(), criteria.clone())
            })
            .collect::<Vec<_>>();
        progress.sort_by(|left, right| left.advancement_id.cmp(&right.advancement_id));
        AdvancementsSnapshot::new(self.advancements.clone(), progress)
    }
    pub fn scroll_offset(&self) -> (f64, f64) {
        (self.scroll_x, self.scroll_y)
    }
    pub fn hovered_advancement_id(&self) -> Option<&str> {
        self.hovered_advancement_id.as_deref()
    }
    pub fn progress_summary(&self, advancement_id: &str) -> Option<AdvancementProgressSummary> {
        let node = self.node(advancement_id)?;
        let criteria = self.progress.get(advancement_id)?;
        Some(self.progress_summary_for(node, criteria))
    }
    pub fn visible_advancement_ids(&self) -> Vec<&str> {
        let Some(root_id) = self.selected_tab_id.as_deref() else {
            return Vec::new();
        };
        self.advancements
            .iter()
            .filter(|node| {
                node.display.is_some()
                    && self.root_id_for(node).as_deref() == Some(root_id)
                    && self.node_is_visible(node)
            })
            .map(|node| node.id.as_str())
            .collect()
    }
    pub fn displayed_parent_id(&self, advancement_id: &str) -> Option<&str> {
        let node = self.node(advancement_id)?;
        self.displayed_parent(node).map(|parent| parent.id.as_str())
    }
    pub fn replace_data(&mut self, snapshot: AdvancementsSnapshot) -> AdvancementUpdateReport {
        self.apply_update(AdvancementsUpdate {
            reset: true,
            removed_ids: Vec::new(),
            added_or_replaced: snapshot.advancements,
            progress_updates: snapshot.progress,
        })
    }
    pub fn apply_update(&mut self, update: AdvancementsUpdate) -> AdvancementUpdateReport {
        let old_selected = self.selected_tab_id.clone();
        if update.reset {
            self.advancements.clear();
            self.progress.clear();
            self.scroll_x = 0.0;
            self.scroll_y = 0.0;
            self.centered = false;
            self.hovered_advancement_id = None;
        }
        if !update.removed_ids.is_empty() {
            let mut removed: HashSet<String> = update.removed_ids.into_iter().collect();
            loop {
                let prior_size = removed.len();
                for node in &self.advancements {
                    if node
                        .parent_id
                        .as_ref()
                        .is_some_and(|id| removed.contains(id))
                    {
                        removed.insert(node.id.clone());
                    }
                }
                if removed.len() == prior_size {
                    break;
                }
            }
            self.advancements.retain(|node| !removed.contains(&node.id));
            self.progress.retain(|id, _| !removed.contains(id));
        }
        let mut report = AdvancementUpdateReport::default();
        let mut pending = update.added_or_replaced;
        let mut inserted_this_pass = true;
        while !pending.is_empty() && inserted_this_pass {
            inserted_this_pass = false;
            let mut index = 0;
            while index < pending.len() {
                let candidate = &pending[index];
                let parent_ready = candidate
                    .parent_id
                    .as_ref()
                    .is_none_or(|parent| self.node(parent).is_some());
                let no_self_parent = candidate.parent_id.as_deref() != Some(candidate.id.as_str());
                let no_cycle = no_self_parent
                    && candidate
                        .parent_id
                        .as_deref()
                        .is_none_or(|parent| !self.parent_chain_contains(parent, &candidate.id));
                if !parent_ready || !no_cycle {
                    index += 1;
                    continue;
                }
                let candidate = pending.remove(index);
                if let Some(existing) = self
                    .advancements
                    .iter()
                    .position(|node| node.id == candidate.id)
                {
                    self.advancements.remove(existing);
                }
                self.advancements.push(candidate);
                inserted_this_pass = true;
            }
        }
        report.ignored_advancement_ids = pending.into_iter().map(|node| node.id).collect();
        for progress in update.progress_updates {
            if let Some(node) = self.node(&progress.advancement_id) {
                self.progress.insert(
                    progress.advancement_id,
                    normalize_criteria(&node.requirements, progress.criteria),
                );
            } else {
                report.ignored_progress_ids.push(progress.advancement_id);
            }
        }
        if self
            .selected_tab_id
            .as_deref()
            .is_none_or(|id| !self.tab_placements().iter().any(|tab| tab.root_id == id))
        {
            self.selected_tab_id = self.tab_placements().first().map(|tab| tab.root_id.clone());
            if self.selected_tab_id != old_selected {
                self.centered = false;
                self.scroll_x = 0.0;
                self.scroll_y = 0.0;
            }
        }
        if self
            .hovered_advancement_id
            .as_deref()
            .is_some_and(|id| self.node(id).is_none_or(|node| !self.node_is_visible(node)))
        {
            self.hovered_advancement_id = None;
            self.hover_fade = 0.0;
        }
        self.clamp_scroll();
        report
    }
    pub fn take_action(&mut self) -> Option<AdvancementsScreenAction> {
        self.pending_actions.pop_front()
    }
    pub fn take_seen_advancement_actions(&mut self) -> Vec<SeenAdvancementsAction> {
        self.seen_advancement_actions.drain(..).collect()
    }
    pub fn set_selected_tab(&mut self, root_id: &str) -> bool {
        let Some(tab) = self
            .tab_placements()
            .into_iter()
            .find(|tab| tab.root_id == root_id)
        else {
            return false;
        };
        if self.selected_tab_id.as_deref() != Some(root_id) {
            self.selected_tab_id = Some(tab.root_id);
            self.scroll_x = 0.0;
            self.scroll_y = 0.0;
            self.centered = false;
        }
        true
    }
    pub fn clear_selected_tab(&mut self) {
        self.selected_tab_id = None;
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        self.centered = false;
        self.hovered_advancement_id = None;
    }
    pub fn select_tab(&mut self, root_id: &str) -> bool {
        let Some(tab) = self
            .tab_placements()
            .into_iter()
            .find(|tab| tab.root_id == root_id)
        else {
            return false;
        };
        self.set_selected_tab(&tab.root_id);
        self.seen_advancement_actions
            .push_back(SeenAdvancementsAction::OpenedTab(tab.root_id));
        true
    }
    pub fn scroll_by(&mut self, dx: f64, dy: f64) {
        self.center_selected_tab_if_needed();
        self.apply_scroll(dx, dy);
    }
    fn node(&self, id: &str) -> Option<&AdvancementNodeData> {
        self.advancements.iter().find(|node| node.id == id)
    }
    fn parent_chain_contains(&self, id: &str, candidate: &str) -> bool {
        let mut seen = HashSet::new();
        let mut current = id.to_owned();
        while seen.insert(current.clone()) {
            if current == candidate {
                return true;
            }
            let Some(parent) = self.node(&current).and_then(|node| node.parent_id.clone()) else {
                return false;
            };
            current = parent;
        }
        true
    }
    fn root_id_for<'a>(&'a self, node: &'a AdvancementNodeData) -> Option<String> {
        let mut current = node;
        let mut seen = HashSet::new();
        while let Some(parent_id) = current.parent_id.as_deref() {
            if !seen.insert(current.id.as_str()) {
                return None;
            }
            current = self.node(parent_id)?;
        }
        Some(current.id.clone())
    }
    fn root_nodes(&self) -> Vec<&AdvancementNodeData> {
        self.advancements
            .iter()
            .filter(|node| node.parent_id.is_none() && node.display.is_some())
            .take(MAX_ROOT_TABS)
            .collect()
    }
    fn tab_placements(&self) -> Vec<AdvancementTabPlacement> {
        let window = self.window_rect();
        let mut placements = Vec::new();
        let mut global_index = 0usize;
        let roots = self.root_nodes();
        for (side, capacity) in TAB_SIDES.into_iter().zip(TAB_SIDE_CAPACITIES) {
            for side_index in 0..capacity {
                let Some(root) = roots.get(global_index) else {
                    break;
                };
                placements.push(AdvancementTabPlacement {
                    root_id: root.id.clone(),
                    side,
                    side_index,
                    side_count: capacity,
                    rect: side.rect(window, side_index),
                });
                global_index += 1;
            }
        }
        placements
    }
    fn window_rect(&self) -> ScreenRectangle {
        ScreenRectangle::new(
            (self.base.width - WINDOW_WIDTH) / 2,
            (self.base.height - WINDOW_HEIGHT) / 2,
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
        )
    }
    pub fn viewport_rect(&self) -> ScreenRectangle {
        let window = self.window_rect();
        ScreenRectangle::new(
            window.left() + VIEWPORT_X,
            window.top() + VIEWPORT_Y,
            VIEWPORT_WIDTH,
            VIEWPORT_HEIGHT,
        )
    }
    fn done_rect(&self) -> ScreenRectangle {
        ScreenRectangle::new(
            self.base.width / 2 - DONE_WIDTH / 2,
            self.base.height - FOOTER_HEIGHT + 7,
            DONE_WIDTH.min((self.base.width - 16).max(40)),
            20,
        )
    }
    fn selected_root(&self) -> Option<&AdvancementNodeData> {
        let id = self.selected_tab_id.as_deref()?;
        self.node(id)
            .filter(|root| root.parent_id.is_none() && root.display.is_some())
    }
    fn selected_nodes(&self) -> Vec<&AdvancementNodeData> {
        let Some(root_id) = self.selected_tab_id.as_deref() else {
            return Vec::new();
        };
        self.advancements
            .iter()
            .filter(|node| self.root_id_for(node).as_deref() == Some(root_id))
            .collect()
    }
    fn node_is_done(&self, node: &AdvancementNodeData) -> bool {
        if node.requirements.is_empty() {
            return false;
        }
        let Some(criteria) = self.progress.get(&node.id) else {
            return false;
        };
        node.requirements.iter().all(|group| {
            group.iter().any(|required| {
                criteria
                    .iter()
                    .any(|criterion| criterion.criterion_id == *required && criterion.acquired())
            })
        })
    }
    fn node_is_visible(&self, node: &AdvancementNodeData) -> bool {
        node.display
            .as_ref()
            .is_some_and(|display| !display.hidden || self.node_is_done(node))
    }
    fn progress_summary_for(
        &self,
        node: &AdvancementNodeData,
        criteria: &[AdvancementCriterionProgress],
    ) -> AdvancementProgressSummary {
        let required: HashSet<&str> = node
            .requirements
            .iter()
            .flat_map(|group| group.iter().map(String::as_str))
            .collect();
        let acquired_criteria = criteria
            .iter()
            .filter(|criterion| {
                required.contains(criterion.criterion_id.as_str()) && criterion.acquired()
            })
            .count();
        let completed_requirements = node
            .requirements
            .iter()
            .filter(|group| {
                group.iter().any(|required| {
                    criteria.iter().any(|criterion| {
                        criterion.criterion_id == *required && criterion.acquired()
                    })
                })
            })
            .count();
        let total_requirements = node.requirements.len();
        let total_criteria = required.len();
        let percent = if total_criteria == 0 || total_requirements == 0 {
            0.0
        } else {
            completed_requirements as f32 / total_requirements as f32
        };
        AdvancementProgressSummary {
            acquired_criteria,
            total_criteria,
            completed_requirements,
            total_requirements,
            percent,
            done: total_requirements > 0 && completed_requirements == total_requirements,
        }
    }
    fn displayed_parent<'a>(
        &'a self,
        node: &'a AdvancementNodeData,
    ) -> Option<&'a AdvancementNodeData> {
        let mut parent_id = node.parent_id.as_deref();
        let mut seen = HashSet::new();
        while let Some(id) = parent_id {
            if !seen.insert(id) {
                return None;
            }
            let parent = self.node(id)?;
            if parent.display.is_some() {
                return Some(parent);
            }
            parent_id = parent.parent_id.as_deref();
        }
        None
    }
    fn node_grid_position(&self, node: &AdvancementNodeData) -> Option<(i32, i32)> {
        let display = node.display.as_ref()?;
        Some((
            (display.x * NODE_SPACING_X).floor() as i32,
            (display.y * NODE_SPACING_Y).floor() as i32,
        ))
    }
    fn node_bounds(&self, node: &AdvancementNodeData) -> Option<(i32, i32, i32, i32)> {
        let (x, y) = self.node_grid_position(node)?;
        Some((x, y, x + NODE_FRAME_SIZE + 2, y + NODE_FRAME_SIZE + 1))
    }
    fn selected_bounds(&self) -> Option<(i32, i32, i32, i32)> {
        let mut nodes = self
            .selected_nodes()
            .into_iter()
            .filter(|node| node.display.is_some());
        let first = self.node_bounds(nodes.next()?)?;
        Some(nodes.fold(first, |(min_x, min_y, max_x, max_y), node| {
            let (x0, y0, x1, y1) = self
                .node_bounds(node)
                .unwrap_or((min_x, min_y, max_x, max_y));
            (min_x.min(x0), min_y.min(y0), max_x.max(x1), max_y.max(y1))
        }))
    }
    fn center_selected_tab_if_needed(&mut self) {
        if self.centered {
            return;
        }
        self.centered = true;
        let Some((min_x, min_y, max_x, max_y)) = self.selected_bounds() else {
            return;
        };
        self.scroll_x = ((VIEWPORT_WIDTH - (max_x + min_x)) / 2) as f64;
        self.scroll_y = ((VIEWPORT_HEIGHT - (max_y + min_y)) / 2) as f64;
    }
    fn scroll_limits(&self) -> (f64, f64, f64, f64) {
        let Some((min_x, min_y, max_x, max_y)) = self.selected_bounds() else {
            return (0.0, 0.0, 0.0, 0.0);
        };
        let min_scroll_x = if max_x - min_x > VIEWPORT_WIDTH {
            -((max_x - VIEWPORT_WIDTH) as f64)
        } else {
            0.0
        };
        let min_scroll_y = if max_y - min_y > VIEWPORT_HEIGHT {
            -((max_y - VIEWPORT_HEIGHT) as f64)
        } else {
            0.0
        };
        (min_scroll_x, 0.0, min_scroll_y, 0.0)
    }
    fn apply_scroll(&mut self, dx: f64, dy: f64) {
        let (min_x, max_x, min_y, max_y) = self.scroll_limits();
        if max_x - min_x > VIEWPORT_WIDTH as f64 {
            self.scroll_x = (self.scroll_x + dx).clamp(min_x, max_x);
        }
        if max_y - min_y > VIEWPORT_HEIGHT as f64 {
            self.scroll_y = (self.scroll_y + dy).clamp(min_y, max_y);
        }
    }
    fn clamp_scroll(&mut self) {
        let (min_x, max_x, min_y, max_y) = self.scroll_limits();
        self.scroll_x = self.scroll_x.clamp(min_x, max_x);
        self.scroll_y = self.scroll_y.clamp(min_y, max_y);
    }
    fn widget_text(&self, node: &AdvancementNodeData) -> Option<AdvancementWidgetText> {
        let display = node.display.as_ref()?;
        let title_lines = wrap_text(&display.title, 163, self.font.as_ref());
        let title_width = title_lines
            .iter()
            .map(|line| self.font.measure_width(line, 1.0).round() as i32)
            .max()
            .unwrap_or(0)
            .max(80);
        let summary = self
            .progress
            .get(&node.id)
            .map(|criteria| self.progress_summary_for(node, criteria));
        let progress_text = summary.and_then(|summary| {
            (summary.total_criteria > 0 && summary.total_requirements > 1).then(|| {
                format!(
                    "{}/{}",
                    summary.completed_requirements, summary.total_requirements
                )
            })
        });
        let progress_width = if node.requirements.len() <= 1 {
            0
        } else {
            let widest = progress_text
                .as_deref()
                .map(|text| self.font.measure_width(text, 1.0).round() as i32)
                .unwrap_or(24);
            widest + 8
        };
        let preferred_width = 29 + title_width + progress_width;
        let description_lines =
            find_optimal_lines(&display.description, preferred_width, self.font.as_ref());
        let description_width = description_lines
            .iter()
            .map(|line| self.font.measure_width(line, 1.0).round() as i32)
            .max()
            .unwrap_or(0);
        Some(AdvancementWidgetText {
            title_lines,
            description_lines,
            width: preferred_width.max(description_width) + 8,
            progress_text,
        })
    }
    fn current_hover_target(&self, mouse_x: i32, mouse_y: i32) -> Option<String> {
        let viewport = self.viewport_rect();
        if !viewport.contains_point(mouse_x, mouse_y) {
            return None;
        }
        let content_x = mouse_x - viewport.left() - self.scroll_x.floor() as i32;
        let content_y = mouse_y - viewport.top() - self.scroll_y.floor() as i32;
        for node in self.selected_nodes() {
            if !self.node_is_visible(node) {
                continue;
            }
            let Some((x, y)) = self.node_grid_position(node) else {
                continue;
            };
            if (x..=x + NODE_FRAME_SIZE).contains(&content_x)
                && (y..=y + NODE_FRAME_SIZE).contains(&content_y)
            {
                return Some(node.id.clone());
            }
        }
        None
    }
    fn update_hover(&mut self) {
        let target = self.current_hover_target(self.mouse_x, self.mouse_y);
        if target.is_some() {
            self.hover_fade = (self.hover_fade + 0.06).clamp(0.0, 0.3);
            self.hovered_advancement_id = target;
        } else {
            self.hover_fade = (self.hover_fade - 0.12).clamp(0.0, 1.0);
            self.hovered_advancement_id = None;
        }
    }
    fn close(&mut self) {
        self.pending_actions
            .push_back(AdvancementsScreenAction::Done);
        self.base.request_close();
    }
    fn draw_icon(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        display: &AdvancementDisplayData,
        x: i32,
        y: i32,
        size: i32,
    ) {
        if let Some(renderer) = &self.icon_renderer {
            renderer.extract_icon(graphics, display, x, y, size);
        } else if let Some(sprite) = display.icon_sprite_key.as_deref() {
            graphics.blit(ResourceLocation::from_string(sprite), x, y, size, size);
        }
    }
    fn draw_viewport(&mut self, graphics: &mut GuiGraphicsExtractor) {
        self.center_selected_tab_if_needed();
        let viewport = self.viewport_rect();
        graphics.enable_scissor(
            viewport.left(),
            viewport.top(),
            viewport.right(),
            viewport.bottom(),
        );
        self.draw_background_tiles(graphics, viewport);
        let nodes = self.selected_nodes();
        for node in &nodes {
            self.draw_connectivity(graphics, node, viewport, true);
        }
        for node in &nodes {
            self.draw_connectivity(graphics, node, viewport, false);
        }
        for node in nodes {
            self.draw_node(graphics, node, viewport);
        }
        graphics.disable_scissor();
    }
    fn draw_background_tiles(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        viewport: ScreenRectangle,
    ) {
        let background = self
            .selected_root()
            .and_then(|root| root.display.as_ref())
            .and_then(|display| display.background_sprite_key.as_deref());
        let scroll_x = self.scroll_x.floor() as i32;
        let scroll_y = self.scroll_y.floor() as i32;
        let offset_x = scroll_x % BACKGROUND_TILE_WIDTH;
        let offset_y = scroll_y % BACKGROUND_TILE_HEIGHT;
        if let Some(background) = background {
            let key = ResourceLocation::from_string(background);
            for x in -1..=15 {
                for y in -1..=8 {
                    graphics.blit(
                        key.clone(),
                        viewport.left() + offset_x + BACKGROUND_TILE_WIDTH * x,
                        viewport.top() + offset_y + BACKGROUND_TILE_HEIGHT * y,
                        BACKGROUND_TILE_WIDTH,
                        BACKGROUND_TILE_HEIGHT,
                    );
                }
            }
        } else {
            graphics.fill(
                viewport.left(),
                viewport.top(),
                viewport.right(),
                viewport.bottom(),
                0xFF101010u32 as i32,
            );
        }
    }
    fn draw_connectivity(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        node: &AdvancementNodeData,
        viewport: ScreenRectangle,
        background: bool,
    ) {
        let Some(parent) = self.displayed_parent(node) else {
            return;
        };
        let Some((parent_x, parent_y)) = self.node_grid_position(parent) else {
            return;
        };
        let Some((node_x, node_y)) = self.node_grid_position(node) else {
            return;
        };
        let pan_x = self.scroll_x.floor() as i32;
        let pan_y = self.scroll_y.floor() as i32;
        let dep_x = viewport.left() + pan_x + parent_x + 13;
        let split_x = viewport.left() + pan_x + parent_x + 26 + 4;
        let dep_y = viewport.top() + pan_y + parent_y + 13;
        let my_x = viewport.left() + pan_x + node_x + 13;
        let my_y = viewport.top() + pan_y + node_y + 13;
        let color = if background {
            0xFF000000u32 as i32
        } else {
            0xFFFFFFFFu32 as i32
        };
        if background {
            graphics.horizontal_line(split_x, dep_x, dep_y - 1, color);
            graphics.horizontal_line(split_x + 1, dep_x, dep_y, color);
            graphics.horizontal_line(split_x, dep_x, dep_y + 1, color);
            graphics.horizontal_line(my_x, split_x - 1, my_y - 1, color);
            graphics.horizontal_line(my_x, split_x - 1, my_y, color);
            graphics.horizontal_line(my_x, split_x - 1, my_y + 1, color);
            graphics.vertical_line(split_x - 1, my_y, dep_y, color);
            graphics.vertical_line(split_x + 1, my_y, dep_y, color);
        } else {
            graphics.horizontal_line(split_x, dep_x, dep_y, color);
            graphics.horizontal_line(my_x, split_x, my_y, color);
            graphics.vertical_line(split_x, my_y, dep_y, color);
        }
    }
    fn draw_node(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        node: &AdvancementNodeData,
        viewport: ScreenRectangle,
    ) {
        let Some(display) = node.display.as_ref() else {
            return;
        };
        if !self.node_is_visible(node) {
            return;
        }
        let Some((x, y)) = self.node_grid_position(node) else {
            return;
        };
        let pan_x = self.scroll_x.floor() as i32;
        let pan_y = self.scroll_y.floor() as i32;
        let screen_x = viewport.left() + pan_x + x;
        let screen_y = viewport.top() + pan_y + y;
        let obtained = self.node_is_done(node);
        graphics.blit(
            ResourceLocation::from_string(&display.frame.frame_sprite_key(obtained)),
            screen_x + 3,
            screen_y,
            NODE_FRAME_SIZE,
            NODE_FRAME_SIZE,
        );
        self.draw_icon(
            graphics,
            display,
            screen_x + 8,
            screen_y + 5,
            NODE_ICON_SIZE,
        );
    }
    fn draw_root_tabs(&self, graphics: &mut GuiGraphicsExtractor, mouse_x: i32, mouse_y: i32) {
        let tabs = self.tab_placements();
        if tabs.len() <= 1 {
            return;
        }
        for tab in &tabs {
            let selected = self.selected_tab_id.as_deref() == Some(tab.root_id.as_str());
            let edge = if tab.side_index == 0 {
                "left"
            } else if tab.side_index + 1 == tab.side_count {
                "right"
            } else {
                "middle"
            };
            let selection = if selected { "_selected" } else { "" };
            let sprite_key = format!(
                "minecraft:advancements/tab_{}_{}{selection}",
                tab.side.sprite_side(),
                edge
            );
            graphics.blit(
                ResourceLocation::from_string(&sprite_key),
                tab.rect.left(),
                tab.rect.top(),
                tab.rect.width,
                tab.rect.height,
            );
            if let Some(root) = self.node(&tab.root_id)
                && let Some(display) = root.display.as_ref()
            {
                let (icon_x, icon_y) = tab.side.icon_offset();
                self.draw_icon(
                    graphics,
                    display,
                    tab.rect.left() + icon_x,
                    tab.rect.top() + icon_y,
                    NODE_ICON_SIZE,
                );
            }
            if tab.rect.contains_point(mouse_x, mouse_y) {
                if let Some(root) = self.node(&tab.root_id)
                    && let Some(display) = root.display.as_ref()
                {
                    graphics.draw_tooltip(
                        self.font.as_ref(),
                        std::slice::from_ref(&display.title),
                        mouse_x,
                        mouse_y,
                    );
                }
            }
        }
    }
    fn draw_window(&self, graphics: &mut GuiGraphicsExtractor) {
        let window = self.window_rect();
        graphics.blit_resource_region(
            ResourceLocation::from_string("minecraft:textures/gui/advancements/window.png"),
            window.left(),
            window.top(),
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            0.0,
            0.0,
            WINDOW_WIDTH as f32 / 256.0,
            WINDOW_HEIGHT as f32 / 256.0,
            -1,
        );
        let title = self
            .selected_root()
            .and_then(|root| root.display.as_ref())
            .map(|display| display.title.clone())
            .unwrap_or_else(|| crate::translate("gui.advancements"));
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            title,
            window.left() + TITLE_X,
            window.top() + TITLE_Y,
            0xFF404040u32 as i32,
        );
    }
    fn draw_empty_message(&self, graphics: &mut GuiGraphicsExtractor) {
        if self.selected_root().is_some() {
            return;
        }
        let viewport = self.viewport_rect();
        graphics.fill(
            viewport.left(),
            viewport.top(),
            viewport.right(),
            viewport.bottom(),
            0xFF000000u32 as i32,
        );
        let empty = crate::translate("advancements.empty");
        let sad = crate::translate("advancements.sad_label");
        let center_x = viewport.left() + viewport.width / 2;
        let empty_x = center_x - self.font.measure_width(&empty, 1.0).round() as i32 / 2;
        let sad_x = center_x - self.font.measure_width(&sad, 1.0).round() as i32 / 2;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            empty,
            empty_x,
            viewport.top() + 56 - 9 / 2,
            0xFFFFFFFFu32 as i32,
        );
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            sad,
            sad_x,
            viewport.bottom() - 9,
            0xFFFFFFFFu32 as i32,
        );
    }
    fn draw_node_tooltip(&self, graphics: &mut GuiGraphicsExtractor, node: &AdvancementNodeData) {
        let Some(display) = node.display.as_ref() else {
            return;
        };
        let Some((x, y)) = self.node_grid_position(node) else {
            return;
        };
        let Some(text) = self.widget_text(node) else {
            return;
        };
        let viewport = self.viewport_rect();
        let pan_x = self.scroll_x.floor() as i32;
        let pan_y = self.scroll_y.floor() as i32;
        let node_x = viewport.left() + pan_x + x;
        let node_y = viewport.top() + pan_y + y;
        let title_height = TITLE_LINE_HEIGHT * text.title_lines.len() as i32 + 17;
        let title_top = node_y + (NODE_FRAME_SIZE - title_height) / 2;
        let title_bottom = title_top + title_height;
        let description_text_height = text.description_lines.len() as i32 * TITLE_LINE_HEIGHT;
        let description_height = 6 + description_text_height;
        let screen_is_left_side = node_x + text.width + NODE_FRAME_SIZE >= self.base.width;
        let top_side = title_bottom + description_height >= viewport.bottom();
        let title_left = if screen_is_left_side {
            node_x - text.width + 32
        } else {
            node_x
        };
        let background_height = title_height + description_height;
        let background_y = if top_side {
            title_bottom - background_height
        } else {
            title_top
        };
        graphics.fill(
            title_left - 1,
            background_y - 1,
            title_left + text.width + 1,
            background_y + background_height + 1,
            0xF0101010u32 as i32,
        );
        graphics.outline(
            title_left - 1,
            background_y - 1,
            text.width + 2,
            background_height + 2,
            0xFF3F3F3Fu32 as i32,
        );
        let summary = self
            .progress
            .get(&node.id)
            .map(|criteria| self.progress_summary_for(node, criteria))
            .unwrap_or_default();
        let obtained = summary.done;
        let bar_color = if obtained {
            0xFF60B040u32 as i32
        } else {
            0xFF707070u32 as i32
        };
        let empty_color = 0xFF303030u32 as i32;
        graphics.fill(
            title_left,
            title_top,
            title_left + text.width,
            title_top + title_height,
            if obtained {
                0xFF315631u32 as i32
            } else {
                0xFF383838u32 as i32
            },
        );
        graphics.fill(
            title_left + 2,
            title_top + title_height - 4,
            title_left + text.width - 2,
            title_top + title_height - 1,
            empty_color,
        );
        let completed_width = ((text.width - 4) as f32 * summary.percent).floor() as i32;
        if completed_width > 0 {
            graphics.fill(
                title_left + 2,
                title_top + title_height - 4,
                title_left + 2 + completed_width,
                title_top + title_height - 1,
                bar_color,
            );
        }
        let title_x = if screen_is_left_side {
            title_left + 5
        } else {
            node_x + 32
        };
        for (line, title_line) in text.title_lines.iter().enumerate() {
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                title_line.clone(),
                title_x,
                title_top + 9 + TITLE_LINE_HEIGHT * line as i32,
                0xFFFFFFFFu32 as i32,
            );
        }
        if let Some(progress_text) = text.progress_text.as_deref() {
            let progress_width = self.font.measure_width(progress_text, 1.0).round() as i32;
            let progress_x = if screen_is_left_side {
                node_x - progress_width
            } else {
                node_x + text.width - progress_width - 5
            };
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                progress_text,
                progress_x,
                title_top + 9,
                0xFFFFFFFFu32 as i32,
            );
        }
        let description_top = if top_side {
            title_top - description_text_height + 1
        } else {
            title_bottom
        };
        for (line, description_line) in text.description_lines.iter().enumerate() {
            graphics.text(
                ResourceLocation::from_string("minecraft:default"),
                description_line.clone(),
                title_left + 5,
                description_top + TITLE_LINE_HEIGHT * line as i32,
                0xFF00FF00u32 as i32,
            );
        }
        graphics.blit(
            ResourceLocation::from_string(&display.frame.frame_sprite_key(obtained)),
            node_x + 3,
            node_y,
            NODE_FRAME_SIZE,
            NODE_FRAME_SIZE,
        );
        self.draw_icon(graphics, display, node_x + 8, node_y + 5, NODE_ICON_SIZE);
    }
}
impl Screen for AdvancementsScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        if self.selected_tab_id.is_none() {
            self.selected_tab_id = self.tab_placements().first().map(|tab| tab.root_id.clone());
            self.centered = false;
        }
        if let Some(selected_tab_id) = self.selected_tab_id.clone() {
            self.seen_advancement_actions
                .push_back(SeenAdvancementsAction::OpenedTab(selected_tab_id));
        }
        self.base.set_close_on_esc(true);
    }
    fn removed(&mut self) {
        self.seen_advancement_actions
            .push_back(SeenAdvancementsAction::ClosedScreen);
    }
    fn reposition_elements(&mut self) {
        self.clamp_scroll();
    }
    fn tick(&mut self) {
        self.update_hover();
    }
    fn on_close(&mut self) {
        self.close();
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if event.is_escape() || event.is_selection() {
            self.close();
            return true;
        }
        false
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        self.mouse_x = event.x.floor() as i32;
        self.mouse_y = event.y.floor() as i32;
        if event.button() != 0 {
            return false;
        }
        if self.done_rect().contains_point(self.mouse_x, self.mouse_y) {
            self.close();
            return true;
        }
        if self.tab_placements().len() > 1
            && let Some(tab) = self
                .tab_placements()
                .into_iter()
                .find(|tab| tab.rect.contains_point(self.mouse_x, self.mouse_y))
        {
            return self.select_tab(&tab.root_id);
        }
        false
    }
    fn mouse_released(&mut self, _event: MouseButtonEvent) -> bool {
        let was_dragging = self.dragging_scroll;
        self.dragging_scroll = false;
        was_dragging
    }
    fn mouse_moved(&mut self, x: f64, y: f64) {
        self.mouse_x = x.floor() as i32;
        self.mouse_y = y.floor() as i32;
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        if event.button() != 0 {
            self.dragging_scroll = false;
            return false;
        }
        if !self.dragging_scroll {
            self.dragging_scroll = true;
        } else {
            self.center_selected_tab_if_needed();
            self.apply_scroll(dx, dy);
        }
        true
    }
    fn mouse_scrolled(&mut self, _x: f64, _y: f64, scroll_x: f64, scroll_y: f64) -> bool {
        if self.selected_tab_id.is_none() {
            return false;
        }
        self.center_selected_tab_if_needed();
        self.apply_scroll(scroll_x * SCROLL_INCREMENT, scroll_y * SCROLL_INCREMENT);
        true
    }
    fn update_narration(&mut self, output: &mut NarrationElementOutput) {
        self.base_ref().update_narration(output);
        if let Some(selected) = self.selected_root().and_then(|root| root.display.as_ref()) {
            output.add(NarratedElementType::Position, selected.title.clone());
        }
        if let Some(id) = self.hovered_advancement_id.as_deref()
            && let Some(node) = self.node(id)
            && let Some(display) = node.display.as_ref()
        {
            output.add(NarratedElementType::Hint, display.title.clone());
            if let Some(summary) = self.progress_summary(id) {
                output.add(
                    NarratedElementType::Hint,
                    format!(
                        "{} / {}",
                        summary.completed_requirements, summary.total_requirements
                    ),
                );
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
        self.mouse_x = mouse_x;
        self.mouse_y = mouse_y;
        self.center_selected_tab_if_needed();
        self.base.extract_background_layers(graphics, self.in_world);
        graphics.next_stratum();
        self.draw_viewport(graphics);
        self.draw_empty_message(graphics);
        graphics.next_stratum();
        self.draw_window(graphics);
        self.draw_root_tabs(graphics, mouse_x, mouse_y);
        if self.hover_fade > 0.0 {
            let viewport = self.viewport_rect();
            let alpha = (self.hover_fade * 255.0).round() as u32;
            graphics.fill(
                viewport.left(),
                viewport.top(),
                viewport.right(),
                viewport.bottom(),
                (alpha << 24) as i32,
            );
        }
        if let Some(id) = self.hovered_advancement_id.clone()
            && let Some(node) = self.node(&id)
        {
            graphics.next_stratum();
            self.draw_node_tooltip(graphics, node);
        }
        let done = self.done_rect();
        graphics.draw_button_with_focus(
            self.font.as_ref(),
            done.left(),
            done.top(),
            done.width,
            done.height,
            &crate::translate("gui.done"),
            true,
            false,
        );
    }
}
fn normalize_criteria(
    requirements: &[Vec<String>],
    supplied: Vec<AdvancementCriterionProgress>,
) -> Vec<AdvancementCriterionProgress> {
    let mut supplied_by_id: HashMap<String, Option<i64>> = supplied
        .into_iter()
        .map(|criterion| (criterion.criterion_id, criterion.acquired_timestamp_millis))
        .collect();
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();
    for criterion_id in requirements.iter().flatten() {
        if seen.insert(criterion_id.clone()) {
            normalized.push(AdvancementCriterionProgress::new(
                criterion_id.clone(),
                supplied_by_id.remove(criterion_id).flatten(),
            ));
        }
    }
    normalized
}
fn wrap_text(text: &str, max_width: i32, font: &dyn FontWidthMeasurer) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let max_width = max_width.max(1) as f32;
    let mut output = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            if line.is_empty() {
                if font.measure_width(word, 1.0) <= max_width {
                    line.push_str(word);
                } else {
                    for character in word.chars() {
                        let candidate = format!("{line}{character}");
                        if !line.is_empty() && font.measure_width(&candidate, 1.0) > max_width {
                            output.push(std::mem::take(&mut line));
                        }
                        line.push(character);
                    }
                }
                continue;
            }
            let candidate = format!("{line} {word}");
            if font.measure_width(&candidate, 1.0) <= max_width {
                line = candidate;
            } else {
                output.push(std::mem::take(&mut line));
                if font.measure_width(word, 1.0) <= max_width {
                    line.push_str(word);
                } else {
                    for character in word.chars() {
                        let candidate = format!("{line}{character}");
                        if !line.is_empty() && font.measure_width(&candidate, 1.0) > max_width {
                            output.push(std::mem::take(&mut line));
                        }
                        line.push(character);
                    }
                }
            }
        }
        if !line.is_empty() {
            output.push(line);
        }
    }
    output
}
fn find_optimal_lines(
    input: &str,
    preferred_width: i32,
    font: &dyn FontWidthMeasurer,
) -> Vec<String> {
    let mut best: Option<(f32, Vec<String>)> = None;
    for margin in TEST_SPLIT_OFFSETS {
        let candidate = wrap_text(input, preferred_width - margin, font);
        let max_width = candidate
            .iter()
            .map(|line| font.measure_width(line, 1.0))
            .fold(0.0f32, f32::max);
        let distance = (max_width - preferred_width as f32).abs();
        if distance <= 10.0 {
            return candidate;
        }
        if best
            .as_ref()
            .is_none_or(|(best_distance, _)| distance < *best_distance)
        {
            best = Some((distance, candidate));
        }
    }
    best.map(|(_, lines)| lines).unwrap_or_default()
}
