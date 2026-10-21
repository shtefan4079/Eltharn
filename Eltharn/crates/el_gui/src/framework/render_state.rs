use crate::framework::navigation::ScreenRectangle;
use el_core::{ItemStack, ResourceLocation};
use el_math::Matrix3x2f;
use std::collections::HashSet;
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RenderPipeline {
    Gui,
    GuiTextured,
    GuiTexturedPremultipliedAlpha,
    GuiTextHighlight,
    GuiInvert,
    EndPortal,
    Vignette,
    Custom(String),
}
impl Default for RenderPipeline {
    fn default() -> Self {
        Self::Gui
    }
}
impl RenderPipeline {
    pub fn sort_key(&self) -> u32 {
        match self {
            Self::EndPortal => 10,
            Self::Gui => 20,
            Self::GuiInvert => 30,
            Self::GuiTextHighlight => 40,
            Self::GuiTextured => 50,
            Self::GuiTexturedPremultipliedAlpha => 60,
            Self::Vignette => 80,
            Self::Custom(_) => 100,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextureSetup {
    pub texture0: Option<ResourceLocation>,
    pub texture1: Option<ResourceLocation>,
    pub texture2: Option<ResourceLocation>,
}
impl TextureSetup {
    pub fn no_texture() -> Self {
        Self::default()
    }
    pub fn single_texture(texture: ResourceLocation) -> Self {
        Self {
            texture0: Some(texture),
            texture1: None,
            texture2: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum GuiElementRenderState {
    ColoredRectangle {
        pipeline: RenderPipeline,
        texture_setup: TextureSetup,
        pose: Matrix3x2f,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        color1: i32,
        color2: i32,
        scissor: Option<ScreenRectangle>,
        bounds: Option<ScreenRectangle>,
    },
    Blit {
        pipeline: RenderPipeline,
        texture_setup: TextureSetup,
        pose: Matrix3x2f,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        u0: f32,
        u1: f32,
        v0: f32,
        v1: f32,
        color: i32,
        scissor: Option<ScreenRectangle>,
        bounds: Option<ScreenRectangle>,
    },
    ResourceTextureBlit {
        pipeline: RenderPipeline,
        texture_setup: TextureSetup,
        pose: Matrix3x2f,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        u0: f32,
        u1: f32,
        v0: f32,
        v1: f32,
        color: i32,
        scissor: Option<ScreenRectangle>,
        bounds: Option<ScreenRectangle>,
    },
    TiledBlit {
        pipeline: RenderPipeline,
        texture_setup: TextureSetup,
        pose: Matrix3x2f,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        u0: f32,
        u1: f32,
        v0: f32,
        v1: f32,
        color: i32,
        scissor: Option<ScreenRectangle>,
        bounds: Option<ScreenRectangle>,
    },
}
impl GuiElementRenderState {
    pub fn bounds(&self) -> Option<ScreenRectangle> {
        match self {
            Self::ColoredRectangle { bounds, .. }
            | Self::Blit { bounds, .. }
            | Self::ResourceTextureBlit { bounds, .. }
            | Self::TiledBlit { bounds, .. } => *bounds,
        }
    }
    pub fn pipeline(&self) -> &RenderPipeline {
        match self {
            Self::ColoredRectangle { pipeline, .. }
            | Self::Blit { pipeline, .. }
            | Self::ResourceTextureBlit { pipeline, .. }
            | Self::TiledBlit { pipeline, .. } => pipeline,
        }
    }
    pub fn texture_setup(&self) -> &TextureSetup {
        match self {
            Self::ColoredRectangle { texture_setup, .. }
            | Self::Blit { texture_setup, .. }
            | Self::ResourceTextureBlit { texture_setup, .. }
            | Self::TiledBlit { texture_setup, .. } => texture_setup,
        }
    }
    pub fn scissor(&self) -> &Option<ScreenRectangle> {
        match self {
            Self::ColoredRectangle { scissor, .. }
            | Self::Blit { scissor, .. }
            | Self::ResourceTextureBlit { scissor, .. }
            | Self::TiledBlit { scissor, .. } => scissor,
        }
    }
}
pub fn scissor_cmp(a: &Option<ScreenRectangle>, b: &Option<ScreenRectangle>) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(a), Some(b)) => a
            .top()
            .cmp(&b.top())
            .then_with(|| a.bottom().cmp(&b.bottom()))
            .then_with(|| a.left().cmp(&b.left()))
            .then_with(|| a.right().cmp(&b.right())),
    }
}
pub fn texture_setup_cmp(a: &TextureSetup, b: &TextureSetup) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    fn slot_cmp(x: &Option<ResourceLocation>, y: &Option<ResourceLocation>) -> Ordering {
        match (x, y) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(_), Some(_)) => Ordering::Equal,
        }
    }
    slot_cmp(&a.texture0, &b.texture0)
        .then_with(|| slot_cmp(&a.texture1, &b.texture1))
        .then_with(|| slot_cmp(&a.texture2, &b.texture2))
}
pub fn element_cmp(a: &GuiElementRenderState, b: &GuiElementRenderState) -> std::cmp::Ordering {
    scissor_cmp(a.scissor(), b.scissor())
        .then_with(|| a.pipeline().sort_key().cmp(&b.pipeline().sort_key()))
        .then_with(|| texture_setup_cmp(a.texture_setup(), b.texture_setup()))
}
#[derive(Clone, Debug, PartialEq)]
pub struct GuiTextRenderState {
    pub font: ResourceLocation,
    pub text: String,
    pub pose: Matrix3x2f,
    pub x: i32,
    pub y: i32,
    pub color: i32,
    pub background_color: i32,
    pub shadow: bool,
    pub obfuscated: bool,
    pub see_through: bool,
    pub scissor: Option<ScreenRectangle>,
    pub bounds: Option<ScreenRectangle>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GuiItemRenderState {
    pub item_stack: ItemStack,
    pub decorations: bool,
    pub pose: Matrix3x2f,
    pub x: i32,
    pub y: i32,
    pub scissor: Option<ScreenRectangle>,
    pub bounds: Option<ScreenRectangle>,
}
#[derive(Debug, Clone)]
pub struct PanoramaRenderState {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraverseRange {
    All,
    BeforeBlur,
    AfterBlur,
}
#[derive(Clone, Debug, Default)]
struct Node {
    element_states: Vec<GuiElementRenderState>,
    glyph_states: Vec<GuiElementRenderState>,
    item_states: Vec<GuiItemRenderState>,
    text_states: Vec<GuiTextRenderState>,
}
impl Node {
    fn sorted_element_states(&self) -> Vec<&GuiElementRenderState> {
        let mut elements: Vec<&GuiElementRenderState> = self.element_states.iter().collect();
        elements.sort_by(|a, b| element_cmp(*a, *b));
        elements
    }
}
#[derive(Clone, Debug, Default)]
struct Stratum {
    nodes: Vec<Node>,
}
#[derive(Clone, Debug)]
pub struct GuiRenderState {
    strata: Vec<Stratum>,
    current_stratum: usize,
    current_depth: usize,
    stratum_below_overlay: Option<usize>,
    first_stratum_after_blur: Option<usize>,
    item_model_identities: HashSet<String>,
    last_element_bounds: Option<ScreenRectangle>,
    pub panorama_render_state: Option<PanoramaRenderState>,
    pub clear_color_override: [f32; 4],
    pub is_hud_hidden: bool,
}
impl GuiRenderState {
    pub fn new() -> Self {
        let mut state = Self {
            strata: Vec::new(),
            current_stratum: 0,
            current_depth: 0,
            stratum_below_overlay: None,
            first_stratum_after_blur: None,
            item_model_identities: HashSet::new(),
            last_element_bounds: None,
            panorama_render_state: None,
            clear_color_override: [0.0; 4],
            is_hud_hidden: false,
        };
        state.next_stratum();
        state
    }
    pub fn next_stratum(&mut self) {
        self.strata.push(Stratum {
            nodes: vec![Node::default()],
        });
        self.current_stratum = self.strata.len() - 1;
        self.current_depth = 0;
        self.last_element_bounds = None;
    }
    pub fn blur_before_this_stratum(&mut self) {
        if self.first_stratum_after_blur.is_some() {
            panic!("Can only blur once per frame");
        }
        self.first_stratum_after_blur = Some(self.current_stratum);
    }
    pub fn begin_tooltip_overlay(&mut self) {
        if self.stratum_below_overlay.is_some() {
            return;
        }
        self.stratum_below_overlay = Some(self.current_stratum);
        self.next_stratum();
    }
    pub fn end_tooltip_overlay(&mut self) {
        if let Some(screen_stratum) = self.stratum_below_overlay.take() {
            self.current_stratum = screen_stratum;
            self.current_depth = 0;
            self.last_element_bounds = None;
        }
    }
    pub fn is_in_tooltip_overlay(&self) -> bool {
        self.stratum_below_overlay.is_some()
    }
    pub fn has_blur_request(&self) -> bool {
        self.first_stratum_after_blur.is_some()
    }
    pub fn reset(&mut self) {
        self.strata.clear();
        self.current_stratum = 0;
        self.current_depth = 0;
        self.stratum_below_overlay = None;
        self.first_stratum_after_blur = None;
        self.item_model_identities.clear();
        self.panorama_render_state = None;
        self.clear_color_override = [0.0; 4];
        self.next_stratum();
    }
    pub fn add_gui_element(&mut self, element: GuiElementRenderState) {
        if self.find_appropriate_node(element.bounds()) {
            self.current_node_mut().element_states.push(element);
        }
    }
    pub fn add_blit_to_current_layer(&mut self, element: GuiElementRenderState) {
        self.current_node_mut().element_states.push(element);
    }
    pub fn add_glyph_to_current_layer(&mut self, element: GuiElementRenderState) {
        self.current_node_mut().glyph_states.push(element);
    }
    pub fn add_text(&mut self, text: GuiTextRenderState) {
        if self.find_appropriate_node(text.bounds) {
            self.current_node_mut().text_states.push(text);
        }
    }
    pub fn add_item(&mut self, item: GuiItemRenderState) {
        if self.find_appropriate_node(item.bounds) {
            self.item_model_identities
                .insert(item.item_stack.item.location.to_string());
            self.current_node_mut().item_states.push(item);
        }
    }
    pub fn item_model_identities(&self) -> &HashSet<String> {
        &self.item_model_identities
    }
    pub fn gui_elements(&self, range: TraverseRange) -> Vec<&GuiElementRenderState> {
        let mut out = Vec::new();
        self.traverse(range, |node| {
            out.extend(node.sorted_element_states());
            out.extend(node.glyph_states.iter());
        });
        out
    }
    pub fn visit_render_nodes(
        &self,
        range: TraverseRange,
        mut visitor: impl FnMut(
            &[GuiElementRenderState],
            &[GuiElementRenderState],
            &[GuiTextRenderState],
            &[GuiItemRenderState],
        ),
    ) {
        self.traverse(range, |node| {
            let sorted: Vec<GuiElementRenderState>;
            let elements: &[GuiElementRenderState] = if node.element_states.len() > 1 {
                sorted = node.sorted_element_states().into_iter().cloned().collect();
                &sorted
            } else {
                &node.element_states
            };
            visitor(
                elements,
                &node.glyph_states,
                &node.text_states,
                &node.item_states,
            );
        });
    }
    pub fn texts(&self) -> Vec<&GuiTextRenderState> {
        let mut out = Vec::new();
        self.traverse(TraverseRange::All, |node| {
            out.extend(node.text_states.iter())
        });
        out
    }
    pub fn items(&self) -> Vec<&GuiItemRenderState> {
        let mut out = Vec::new();
        self.traverse(TraverseRange::All, |node| {
            out.extend(node.item_states.iter())
        });
        out
    }
    pub fn stratum_count(&self) -> usize {
        self.strata.len()
    }
    fn current_node_mut(&mut self) -> &mut Node {
        let stratum = &mut self.strata[self.current_stratum];
        while stratum.nodes.len() <= self.current_depth {
            stratum.nodes.push(Node::default());
        }
        &mut stratum.nodes[self.current_depth]
    }
    fn find_appropriate_node(&mut self, bounds: Option<ScreenRectangle>) -> bool {
        let Some(bounds) = bounds else {
            return false;
        };
        if self
            .last_element_bounds
            .is_some_and(|last| last.encompasses(bounds))
        {
            self.current_depth += 1;
        } else {
            self.current_depth = self
                .highest_intersecting_depth(bounds)
                .map_or(0, |depth| depth + 1);
        }
        self.last_element_bounds = Some(bounds);
        true
    }
    fn highest_intersecting_depth(&self, bounds: ScreenRectangle) -> Option<usize> {
        self.strata
            .get(self.current_stratum)?
            .nodes
            .iter()
            .enumerate()
            .rev()
            .find(|(_, node)| node_intersects(node, bounds))
            .map(|(depth, _)| depth)
    }
    fn traverse<'a>(&'a self, range: TraverseRange, mut visitor: impl FnMut(&'a Node)) {
        let start = match range {
            TraverseRange::All | TraverseRange::BeforeBlur => 0,
            TraverseRange::AfterBlur => self.first_stratum_after_blur.unwrap_or(self.strata.len()),
        };
        let end = match range {
            TraverseRange::All | TraverseRange::AfterBlur => self.strata.len(),
            TraverseRange::BeforeBlur => self.first_stratum_after_blur.unwrap_or(self.strata.len()),
        };
        for stratum in &self.strata[start.min(self.strata.len())..end.min(self.strata.len())] {
            for node in &stratum.nodes {
                visitor(node);
            }
        }
    }
}
impl Default for GuiRenderState {
    fn default() -> Self {
        Self::new()
    }
}
fn node_intersects(node: &Node, bounds: ScreenRectangle) -> bool {
    node.element_states
        .iter()
        .filter_map(GuiElementRenderState::bounds)
        .any(|existing| existing.intersects(bounds))
        || node
            .glyph_states
            .iter()
            .filter_map(GuiElementRenderState::bounds)
            .any(|existing| existing.intersects(bounds))
        || node
            .text_states
            .iter()
            .filter_map(|state| state.bounds)
            .any(|existing| existing.intersects(bounds))
        || node
            .item_states
            .iter()
            .filter_map(|state| state.bounds)
            .any(|existing| existing.intersects(bounds))
}
pub fn rect_bounds(
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    pose: Matrix3x2f,
    scissor: Option<ScreenRectangle>,
) -> Option<ScreenRectangle> {
    let left = x0.min(x1);
    let top = y0.min(y1);
    let right = x0.max(x1);
    let bottom = y0.max(y1);
    let bounds =
        ScreenRectangle::new(left, top, right - left, bottom - top).transform_max_bounds(pose);
    scissor.map_or(Some(bounds), |s| s.intersection(bounds))
}
