#![allow(dead_code)]
use std::borrow::Cow;
#[cfg(feature = "extensions")]
use std::collections::BTreeMap;
use std::collections::HashMap;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAction {
    Attack,
    Use,
    Forward,
    Left,
    Back,
    Right,
    Jump,
    Sneak,
    Sprint,
    Drop,
    Inventory,
    Chat,
    PlayerList,
    PickItem,
    Command,
    Friends,
    SocialInteractions,
    ToggleGui,
    ToggleSpectatorShaderEffects,
    Screenshot,
    TogglePerspective,
    SmoothCamera,
    Fullscreen,
    SpectatorOutlines,
    SpectatorHotbar,
    SwapOffhand,
    SaveHotbarActivator,
    LoadHotbarActivator,
    Advancements,
    QuickActions,
    HotbarSlot(usize),
    DebugOverlay,
    DebugModifier,
    DebugCrash,
    DebugReloadChunk,
    DebugShowHitboxes,
    DebugClearChat,
    DebugShowChunkBorders,
    DebugShowAdvancedTooltips,
    DebugCopyRecreateCommand,
    DebugSpectate,
    DebugSwitchGameMode,
    DebugDebugOptions,
    DebugFocusPause,
    DebugDumpDynamicTextures,
    DebugReloadResourcePacks,
    DebugProfiling,
    DebugCopyLocation,
    DebugDumpVersion,
    DebugProfilingChart,
    DebugFpsCharts,
    DebugNetworkCharts,
    DebugLightmapTexture,
    #[cfg(feature = "extensions")]
    Extension(u32),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoundInput {
    Key(KeyCode),
    Mouse(u16),
    Unbound,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyCategory {
    Movement,
    Misc,
    Multiplayer,
    Gameplay,
    Inventory,
    Creative,
    Spectator,
    Debug,
}
impl KeyCategory {
    pub const fn language_key(self) -> &'static str {
        match self {
            Self::Movement => "key.categories.movement",
            Self::Misc => "key.categories.misc",
            Self::Multiplayer => "key.categories.multiplayer",
            Self::Gameplay => "key.categories.gameplay",
            Self::Inventory => "key.categories.inventory",
            Self::Creative => "key.categories.creative",
            Self::Spectator => "key.categories.spectator",
            Self::Debug => "key.categories.debug",
        }
    }
}
#[derive(Debug, Clone)]
pub struct KeyMapping {
    pub name: Cow<'static, str>,
    pub action: KeyAction,
    pub default_input: BoundInput,
    pub input: BoundInput,
    pub category: KeyCategory,
    pub order: i32,
    down: bool,
    held_sources: u8,
    click_count: u32,
    #[cfg(feature = "extensions")]
    extension: Option<ModKeyMapping>,
}
#[cfg(feature = "extensions")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ModKeyMapping {
    owner: String,
    id: String,
    label: String,
    category_key: String,
    category_label: String,
    category_order: usize,
    saved_option: String,
}
impl KeyMapping {
    pub const fn new(name: &'static str, action: KeyAction, input: BoundInput) -> Self {
        Self::new_with_category(name, action, input, category_for_action(action), 0)
    }
    pub const fn new_with_category(
        name: &'static str,
        action: KeyAction,
        input: BoundInput,
        category: KeyCategory,
        order: i32,
    ) -> Self {
        Self {
            name: Cow::Borrowed(name),
            action,
            default_input: input,
            input,
            category,
            order,
            down: false,
            held_sources: 0,
            click_count: 0,
            #[cfg(feature = "extensions")]
            extension: None,
        }
    }
    pub const fn is_down(&self) -> bool {
        self.down
    }
    pub const fn category(&self) -> KeyCategory {
        self.category
    }
    pub fn set_down(&mut self, down: bool) {
        self.down = down;
    }
    pub fn click(&mut self) {
        self.click_count = self.click_count.saturating_add(1);
    }
    pub fn consume_click(&mut self) -> bool {
        if self.click_count == 0 {
            return false;
        }
        self.click_count -= 1;
        true
    }
    pub fn release(&mut self) {
        self.down = false;
        self.held_sources = 0;
        self.click_count = 0;
    }
    fn set_source_down(&mut self, source: ActionInputSource, down: bool) {
        let mask = 1 << source as u8;
        if down {
            self.held_sources |= mask;
        } else {
            self.held_sources &= !mask;
        }
        self.down = self.held_sources != 0;
    }
    pub const fn should_set_on_ingame_focus(&self) -> bool {
        matches!(self.input, BoundInput::Key(_))
    }
    pub const fn default_input(&self) -> BoundInput {
        self.default_input
    }
    pub fn set_input(&mut self, input: BoundInput) {
        if self.input != input {
            self.release();
        }
        self.input = input;
    }
    pub fn display_label(&self) -> Option<&str> {
        #[cfg(feature = "extensions")]
        if let Some(extension) = &self.extension {
            return Some(&extension.label);
        }
        None
    }
    pub fn category_key(&self) -> &str {
        #[cfg(feature = "extensions")]
        if let Some(extension) = &self.extension {
            return &extension.category_key;
        }
        self.category.language_key()
    }
    pub fn category_label(&self) -> Option<&str> {
        #[cfg(feature = "extensions")]
        if let Some(extension) = &self.extension {
            return Some(&extension.category_label);
        }
        None
    }
    pub fn category_order(&self) -> usize {
        #[cfg(feature = "extensions")]
        if let Some(extension) = &self.extension {
            return extension.category_order;
        }
        self.category as usize
    }
    pub fn conflicts_with(&self, other: &Self) -> bool {
        !self.is_unbound() && self.same(other) && (!self.is_default() || !other.is_default())
    }
    pub fn same(&self, other: &Self) -> bool {
        self.input == other.input
    }
    pub const fn is_unbound(&self) -> bool {
        matches!(self.input, BoundInput::Unbound)
    }
    pub fn matches(&self, input: BoundInput) -> bool {
        self.input == input
    }
    pub fn translated_input(&self) -> String {
        format_bound_input(self.input)
    }
    pub fn is_default(&self) -> bool {
        self.input == self.default_input
    }
    pub fn save_string(&self) -> String {
        serialize_bound_input(self.input)
    }
}
const fn category_for_action(action: KeyAction) -> KeyCategory {
    match action {
        KeyAction::Forward
        | KeyAction::Left
        | KeyAction::Back
        | KeyAction::Right
        | KeyAction::Jump
        | KeyAction::Sneak
        | KeyAction::Sprint => KeyCategory::Movement,
        KeyAction::Attack | KeyAction::Use | KeyAction::PickItem => KeyCategory::Gameplay,
        KeyAction::Drop
        | KeyAction::Inventory
        | KeyAction::SwapOffhand
        | KeyAction::HotbarSlot(_) => KeyCategory::Inventory,
        KeyAction::Chat
        | KeyAction::PlayerList
        | KeyAction::Command
        | KeyAction::Friends
        | KeyAction::SocialInteractions => KeyCategory::Multiplayer,
        KeyAction::Advancements | KeyAction::QuickActions => KeyCategory::Misc,
        KeyAction::SaveHotbarActivator | KeyAction::LoadHotbarActivator => KeyCategory::Creative,
        KeyAction::SpectatorOutlines | KeyAction::SpectatorHotbar => KeyCategory::Spectator,
        KeyAction::DebugOverlay
        | KeyAction::DebugModifier
        | KeyAction::DebugCrash
        | KeyAction::DebugReloadChunk
        | KeyAction::DebugShowHitboxes
        | KeyAction::DebugClearChat
        | KeyAction::DebugShowChunkBorders
        | KeyAction::DebugShowAdvancedTooltips
        | KeyAction::DebugCopyRecreateCommand
        | KeyAction::DebugSpectate
        | KeyAction::DebugSwitchGameMode
        | KeyAction::DebugDebugOptions
        | KeyAction::DebugFocusPause
        | KeyAction::DebugDumpDynamicTextures
        | KeyAction::DebugReloadResourcePacks
        | KeyAction::DebugProfiling
        | KeyAction::DebugCopyLocation
        | KeyAction::DebugDumpVersion
        | KeyAction::DebugProfilingChart
        | KeyAction::DebugFpsCharts
        | KeyAction::DebugNetworkCharts
        | KeyAction::DebugLightmapTexture => KeyCategory::Debug,
        _ => KeyCategory::Misc,
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionInputSource {
    Native,
    PlatformPointer,
    HardwareAdapter,
}
#[derive(Debug, Clone)]
pub struct VanillaKeyMappings {
    mappings: Vec<KeyMapping>,
    by_action: HashMap<KeyAction, usize>,
    scroll_accum_x: f64,
    scroll_accum_y: f64,
    logical_physical_down: HashMap<(KeyAction, ActionInputSource), bool>,
    #[cfg(feature = "extensions")]
    next_extension_action: u32,
    #[cfg(feature = "extensions")]
    extension_categories: HashMap<String, usize>,
    #[cfg(feature = "extensions")]
    next_extension_category: usize,
    #[cfg(feature = "extensions")]
    extension_sequences: HashMap<String, u64>,
    #[cfg(feature = "extensions")]
    extension_registrations: BTreeMap<String, Vec<el_mod_runtime::keybindings::Binding>>,
}
impl Default for VanillaKeyMappings {
    fn default() -> Self {
        Self::new()
    }
}
impl VanillaKeyMappings {
    pub fn new() -> Self {
        let mappings = vanilla_defaults();
        let by_action = mappings
            .iter()
            .enumerate()
            .map(|(idx, mapping)| (mapping.action, idx))
            .collect();
        Self {
            mappings,
            by_action,
            scroll_accum_x: 0.0,
            scroll_accum_y: 0.0,
            logical_physical_down: HashMap::new(),
            #[cfg(feature = "extensions")]
            next_extension_action: 0,
            #[cfg(feature = "extensions")]
            extension_categories: HashMap::new(),
            #[cfg(feature = "extensions")]
            next_extension_category: 8,
            #[cfg(feature = "extensions")]
            extension_sequences: HashMap::new(),
            #[cfg(feature = "extensions")]
            extension_registrations: BTreeMap::new(),
        }
    }
    pub fn is_down(&self, action: KeyAction) -> bool {
        self.mapping(action).is_some_and(|mapping| mapping.down)
    }
    pub fn mappings(&self) -> &[KeyMapping] {
        &self.mappings
    }
    pub fn mappings_mut(&mut self) -> &mut [KeyMapping] {
        &mut self.mappings
    }
    #[cfg(feature = "extensions")]
    pub fn sync_mod_bindings(
        &mut self,
        registrations: &BTreeMap<String, Vec<el_mod_runtime::keybindings::Binding>>,
        saved: &HashMap<String, String>,
    ) -> bool {
        if self.extension_registrations == *registrations
            && self.mappings.iter().all(|mapping| {
                mapping.extension.as_ref().is_none_or(|extension| {
                    mapping.input
                        == saved
                            .get(&extension.saved_option)
                            .map(|value| deserialize_bound_input(value))
                            .unwrap_or_else(|| {
                                registrations
                                    .get(&extension.owner)
                                    .into_iter()
                                    .flatten()
                                    .find(|binding| binding.id == extension.id)
                                    .and_then(|binding| binding.initial_binding.as_deref())
                                    .map(deserialize_bound_input)
                                    .unwrap_or(mapping.default_input)
                            })
                })
            })
        {
            return false;
        }
        let mut previous = HashMap::new();
        let mut old_names = Vec::new();
        let mut mappings = Vec::with_capacity(self.mappings.len());
        for mapping in std::mem::take(&mut self.mappings) {
            if mapping.extension.is_some() {
                old_names.push(mapping.name.to_string());
                previous.insert(mapping.name.to_string(), mapping);
            } else {
                mappings.push(mapping);
            }
        }
        let mut changed = false;
        let mut new_names = Vec::new();
        let mut active_categories = std::collections::HashSet::new();
        for (owner, bindings) in registrations {
            if el_mod_runtime::keybindings::validate_bindings(bindings).is_err() {
                continue;
            }
            for binding in bindings {
                let name = format!(
                    "key.mod.{}.{}",
                    hex_component(owner),
                    hex_component(&binding.id)
                );
                let category_key = format!(
                    "key.mod.category.{}.{}",
                    hex_component(owner),
                    hex_component(&binding.category)
                );
                active_categories.insert(category_key.clone());
                let category_order = *self
                    .extension_categories
                    .entry(category_key.clone())
                    .or_insert_with(|| {
                        let order = self.next_extension_category;
                        self.next_extension_category = order.saturating_add(1);
                        order
                    });
                let metadata = ModKeyMapping {
                    owner: owner.clone(),
                    id: binding.id.clone(),
                    label: binding.label.clone(),
                    category_key,
                    category_label: binding.category.clone(),
                    category_order,
                    saved_option: format!("key_{name}"),
                };
                let default_input = deserialize_bound_input(&binding.default_binding);
                let input = saved
                    .get(&metadata.saved_option)
                    .map(|value| deserialize_bound_input(value))
                    .unwrap_or_else(|| {
                        binding
                            .initial_binding
                            .as_deref()
                            .map(deserialize_bound_input)
                            .unwrap_or(default_input)
                    });
                let mapping = if let Some(mut mapping) = previous.remove(&name) {
                    changed |= mapping.extension.as_ref() != Some(&metadata)
                        || mapping.default_input != default_input
                        || mapping.input != input
                        || mapping.order != binding.order;
                    mapping.extension = Some(metadata);
                    mapping.default_input = default_input;
                    mapping.order = binding.order;
                    mapping.set_input(input);
                    mapping
                } else {
                    let Some(next_action) = self.next_extension_action.checked_add(1) else {
                        continue;
                    };
                    let action = KeyAction::Extension(self.next_extension_action);
                    self.next_extension_action = next_action;
                    changed = true;
                    KeyMapping {
                        name: Cow::Owned(name.clone()),
                        action,
                        default_input,
                        input,
                        category: KeyCategory::Misc,
                        order: binding.order,
                        down: false,
                        held_sources: 0,
                        click_count: 0,
                        extension: Some(metadata),
                    }
                };
                new_names.push(name);
                mappings.push(mapping);
            }
        }
        self.extension_categories
            .retain(|category, _| active_categories.contains(category));
        changed |= old_names != new_names;
        self.mappings = mappings;
        self.extension_registrations.clone_from(registrations);
        self.reset_mapping();
        self.logical_physical_down
            .retain(|(action, _), _| self.by_action.contains_key(action));
        changed
    }
    #[cfg(feature = "extensions")]
    pub fn drain_mod_input(&mut self) -> BTreeMap<String, el_mod_runtime::keybindings::InputFrame> {
        use el_mod_runtime::keybindings::{InputFrame, InputState, MAX_SEQUENCE};
        let mut frames = BTreeMap::<String, InputFrame>::new();
        for mapping in &mut self.mappings {
            let Some(extension) = &mapping.extension else {
                continue;
            };
            frames
                .entry(extension.owner.clone())
                .or_default()
                .states
                .push(InputState {
                    id: extension.id.clone(),
                    binding: mapping.save_string(),
                    down: mapping.down,
                    clicks: mapping.click_count.min(65535),
                });
            mapping.click_count = 0;
        }
        frames.retain(|owner, frame| {
            let sequence = self.extension_sequences.entry(owner.clone()).or_default();
            if *sequence >= MAX_SEQUENCE {
                return false;
            }
            *sequence += 1;
            frame.sequence = *sequence;
            true
        });
        frames
    }
    pub fn get(&self, name: &str) -> Option<&KeyMapping> {
        self.mappings.iter().find(|mapping| mapping.name == name)
    }
    pub fn get_mut(&mut self, name: &str) -> Option<&mut KeyMapping> {
        self.mappings
            .iter_mut()
            .find(|mapping| mapping.name == name)
    }
    pub fn reset_mapping(&mut self) {
        self.by_action = self
            .mappings
            .iter()
            .enumerate()
            .map(|(idx, mapping)| (mapping.action, idx))
            .collect();
    }
    pub fn consume_click(&mut self, action: KeyAction) -> bool {
        self.mapping_mut(action)
            .is_some_and(KeyMapping::consume_click)
    }
    pub fn release_all(&mut self) {
        self.logical_physical_down.clear();
        for mapping in &mut self.mappings {
            mapping.release();
        }
    }
    pub fn set_action_down(&mut self, action: KeyAction, down: bool) {
        if let Some(mapping) = self.mapping_mut(action) {
            mapping.set_down(down);
        }
    }
    pub fn click_action(&mut self, action: KeyAction) {
        if let Some(mapping) = self.mapping_mut(action) {
            mapping.click();
        }
    }
    pub fn apply_action_input(
        &mut self,
        action: KeyAction,
        pressed: bool,
        queue_click: bool,
    ) -> bool {
        self.apply_action_input_from(
            ActionInputSource::PlatformPointer,
            action,
            pressed,
            queue_click,
        )
    }
    pub fn apply_action_input_from(
        &mut self,
        source: ActionInputSource,
        action: KeyAction,
        pressed: bool,
        queue_click: bool,
    ) -> bool {
        let repeated =
            self.logical_physical_down.insert((action, source), pressed) == Some(pressed);
        let Some(mapping) = self.mapping_mut(action) else {
            return false;
        };
        if repeated && !queue_click {
            if !pressed {
                mapping.set_source_down(source, false);
            }
            return true;
        }
        Self::apply_mapping_input(mapping, source, pressed, queue_click);
        true
    }
    fn apply_mapping_input(
        mapping: &mut KeyMapping,
        source: ActionInputSource,
        pressed: bool,
        queue_click: bool,
    ) {
        #[cfg(feature = "extensions")]
        let hook_pressed = crate::modding::hooks::input_action(mapping.name.as_ref(), pressed);
        #[cfg(not(feature = "extensions"))]
        let hook_pressed = Some(pressed);
        let effective_pressed = pressed && hook_pressed == Some(true);
        mapping.set_source_down(source, effective_pressed);
        if effective_pressed && queue_click {
            mapping.click();
        }
    }
    pub fn process_event(&mut self, event: &WindowEvent, game_input_enabled: bool) -> bool {
        if matches!(event, WindowEvent::Focused(false)) {
            self.release_all();
            return false;
        }
        let Some(input_event) = InputEvent::from_window_event(event) else {
            return false;
        };
        match input_event.kind {
            InputEventKind::Button { input, pressed } => {
                self.apply_button_input(input, pressed, game_input_enabled)
            }
            InputEventKind::Scroll { x, y } => {
                if !game_input_enabled {
                    return false;
                }
                let scroll = self.accumulate_scroll(x, y);
                scroll != 0
            }
        }
    }
    pub fn take_hotbar_scroll(&mut self, current_selected: usize, limit: usize) -> usize {
        self.take_hotbar_scroll_with_options(current_selected, limit, 1.0, false)
    }
    pub fn take_hotbar_scroll_with_options(
        &mut self,
        current_selected: usize,
        limit: usize,
        mouse_wheel_sensitivity: f32,
        discrete_mouse_scroll: bool,
    ) -> usize {
        if limit == 0 {
            return current_selected;
        }
        let sensitivity = mouse_wheel_sensitivity.clamp(0.01, 10.0) as f64;
        let scaled_x = self.scroll_accum_x * sensitivity;
        let scaled_y = self.scroll_accum_y * sensitivity;
        let mut y = scaled_y.trunc() as i32;
        let mut x = scaled_x.trunc() as i32;
        if x == 0 && y == 0 {
            return current_selected;
        }
        if discrete_mouse_scroll {
            x = x.signum();
            y = y.signum();
            if x != 0 {
                self.scroll_accum_x = 0.0;
            }
            if y != 0 {
                self.scroll_accum_y = 0.0;
            }
        } else {
            self.scroll_accum_x -= (x as f64) / sensitivity;
            self.scroll_accum_y -= (y as f64) / sensitivity;
        }
        let wheel = if y == 0 { -x } else { y };
        next_scroll_wheel_selection(wheel as f64, current_selected, limit)
    }
    pub fn handle_hotbar_scroll_event(
        &mut self,
        event: &WindowEvent,
        current_selected: usize,
        limit: usize,
        mouse_wheel_sensitivity: f32,
        discrete_mouse_scroll: bool,
    ) -> Option<usize> {
        let Some(InputEvent {
            kind: InputEventKind::Scroll { x, y },
        }) = InputEvent::from_window_event(event)
        else {
            return None;
        };
        self.handle_hotbar_scroll_delta(
            x,
            y,
            current_selected,
            limit,
            mouse_wheel_sensitivity,
            discrete_mouse_scroll,
        )
    }
    pub fn handle_hotbar_scroll_delta(
        &mut self,
        raw_x: f64,
        raw_y: f64,
        current_selected: usize,
        limit: usize,
        mouse_wheel_sensitivity: f32,
        discrete_mouse_scroll: bool,
    ) -> Option<usize> {
        let sensitivity = mouse_wheel_sensitivity.clamp(0.01, 10.0) as f64;
        let scaled_x = (if discrete_mouse_scroll {
            java_signum(raw_x)
        } else {
            raw_x
        }) * sensitivity;
        let scaled_y = (if discrete_mouse_scroll {
            java_signum(raw_y)
        } else {
            raw_y
        }) * sensitivity;
        if self.scroll_accum_x != 0.0 && java_signum(scaled_x) != java_signum(self.scroll_accum_x) {
            self.scroll_accum_x = 0.0;
        }
        if self.scroll_accum_y != 0.0 && java_signum(scaled_y) != java_signum(self.scroll_accum_y) {
            self.scroll_accum_y = 0.0;
        }
        self.scroll_accum_x += scaled_x;
        self.scroll_accum_y += scaled_y;
        let wheel_x = self.scroll_accum_x.trunc() as i32;
        let wheel_y = self.scroll_accum_y.trunc() as i32;
        if wheel_x == 0 && wheel_y == 0 {
            return None;
        }
        self.scroll_accum_x -= wheel_x as f64;
        self.scroll_accum_y -= wheel_y as f64;
        let wheel = if wheel_y == 0 { -wheel_x } else { wheel_y };
        if discrete_mouse_scroll {
            let step = wheel.signum();
            Some(next_scroll_wheel_selection(
                step as f64,
                current_selected,
                limit,
            ))
        } else {
            Some(next_scroll_wheel_selection(
                wheel as f64,
                current_selected,
                limit,
            ))
        }
    }
    pub fn matches_input(&self, action: KeyAction, input: BoundInput) -> bool {
        self.mapping(action)
            .is_some_and(|mapping| mapping.input == input)
    }
    fn mapping(&self, action: KeyAction) -> Option<&KeyMapping> {
        self.by_action
            .get(&action)
            .and_then(|idx| self.mappings.get(*idx))
    }
    fn mapping_mut(&mut self, action: KeyAction) -> Option<&mut KeyMapping> {
        self.by_action
            .get(&action)
            .and_then(|idx| self.mappings.get_mut(*idx))
    }
    fn apply_button_input(
        &mut self,
        input: BoundInput,
        pressed: bool,
        game_input_enabled: bool,
    ) -> bool {
        let mut matched = false;
        for mapping in self
            .mappings
            .iter_mut()
            .filter(|mapping| mapping.input == input)
        {
            matched = true;
            if game_input_enabled {
                Self::apply_mapping_input(mapping, ActionInputSource::Native, pressed, true);
            } else if !pressed {
                mapping.set_source_down(ActionInputSource::Native, false);
            }
        }
        matched && game_input_enabled
    }
    fn accumulate_scroll(&mut self, x: f64, y: f64) -> i32 {
        if self.scroll_accum_x != 0.0 && java_signum(x) != java_signum(self.scroll_accum_x) {
            self.scroll_accum_x = 0.0;
        }
        if self.scroll_accum_y != 0.0 && java_signum(y) != java_signum(self.scroll_accum_y) {
            self.scroll_accum_y = 0.0;
        }
        self.scroll_accum_x += x;
        self.scroll_accum_y += y;
        self.scroll_accum_x.trunc() as i32 + self.scroll_accum_y.trunc() as i32
    }
}
#[derive(Debug, Clone, Copy)]
struct InputEvent {
    kind: InputEventKind,
}
#[derive(Debug, Clone, Copy)]
enum InputEventKind {
    Button { input: BoundInput, pressed: bool },
    Scroll { x: f64, y: f64 },
}
impl InputEvent {
    fn from_window_event(event: &WindowEvent) -> Option<Self> {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(code),
                        ..
                    },
                ..
            } => Some(Self {
                kind: InputEventKind::Button {
                    input: BoundInput::Key(*code),
                    pressed: *state == ElementState::Pressed,
                },
            }),
            WindowEvent::MouseInput { state, button, .. } => {
                mouse_button_id(*button).map(|id| Self {
                    kind: InputEventKind::Button {
                        input: BoundInput::Mouse(id),
                        pressed: *state == ElementState::Pressed,
                    },
                })
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (x, y) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => (*x as f64, *y as f64),
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        (pos.x.signum(), pos.y.signum())
                    }
                };
                Some(Self {
                    kind: InputEventKind::Scroll { x, y },
                })
            }
            _ => None,
        }
    }
}
fn mouse_button_id(button: MouseButton) -> Option<u16> {
    match button {
        MouseButton::Left => Some(0),
        MouseButton::Right => Some(1),
        MouseButton::Middle => Some(2),
        MouseButton::Back => Some(3),
        MouseButton::Forward => Some(4),
        MouseButton::Other(id) => Some(id),
    }
}
#[cfg(feature = "extensions")]
fn hex_component(value: &str) -> String {
    use std::fmt::Write;
    let mut escaped = String::with_capacity(value.len() * 2);
    for byte in value.bytes() {
        write!(escaped, "{byte:02x}").expect("formatting into String is infallible");
    }
    escaped
}
fn next_scroll_wheel_selection(wheel: f64, current_selected: usize, limit: usize) -> usize {
    let step = wheel.signum() as isize;
    let mut selected = current_selected as isize - step;
    selected = selected.max(-1);
    while selected < 0 {
        selected += limit as isize;
    }
    while selected >= limit as isize {
        selected -= limit as isize;
    }
    selected as usize
}
fn java_signum(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}
fn vanilla_defaults() -> Vec<KeyMapping> {
    use BoundInput::{Key, Mouse, Unbound};
    use KeyAction::*;
    vec![
        KeyMapping::new("key.attack", Attack, Mouse(0)),
        KeyMapping::new("key.use", Use, Mouse(1)),
        KeyMapping::new("key.forward", Forward, Key(KeyCode::KeyW)),
        KeyMapping::new("key.left", Left, Key(KeyCode::KeyA)),
        KeyMapping::new("key.back", Back, Key(KeyCode::KeyS)),
        KeyMapping::new("key.right", Right, Key(KeyCode::KeyD)),
        KeyMapping::new("key.jump", Jump, Key(KeyCode::Space)),
        KeyMapping::new("key.sneak", Sneak, Key(KeyCode::ShiftLeft)),
        KeyMapping::new("key.sprint", Sprint, Key(KeyCode::ControlLeft)),
        KeyMapping::new("key.drop", Drop, Key(KeyCode::KeyQ)),
        KeyMapping::new("key.inventory", Inventory, Key(KeyCode::KeyE)),
        KeyMapping::new("key.chat", Chat, Key(KeyCode::KeyT)),
        KeyMapping::new("key.playerlist", PlayerList, Key(KeyCode::Tab)),
        KeyMapping::new("key.pickItem", PickItem, Mouse(2)),
        KeyMapping::new("key.command", Command, Key(KeyCode::Slash)),
        KeyMapping::new("key.friends", Friends, Key(KeyCode::KeyO)),
        KeyMapping::new(
            "key.socialInteractions",
            SocialInteractions,
            Key(KeyCode::KeyP),
        ),
        KeyMapping::new("key.toggleGui", ToggleGui, Key(KeyCode::F1)),
        KeyMapping::new(
            "key.toggleSpectatorShaderEffects",
            ToggleSpectatorShaderEffects,
            Key(KeyCode::F4),
        ),
        KeyMapping::new("key.screenshot", Screenshot, Key(KeyCode::F2)),
        KeyMapping::new("key.togglePerspective", TogglePerspective, Key(KeyCode::F5)),
        KeyMapping::new("key.smoothCamera", SmoothCamera, Unbound),
        KeyMapping::new("key.fullscreen", Fullscreen, Key(KeyCode::F11)),
        KeyMapping::new("key.spectatorOutlines", SpectatorOutlines, Unbound),
        KeyMapping::new("key.spectatorHotbar", SpectatorHotbar, Mouse(2)),
        KeyMapping::new("key.swapOffhand", SwapOffhand, Key(KeyCode::KeyF)),
        KeyMapping::new(
            "key.saveToolbarActivator",
            SaveHotbarActivator,
            Key(KeyCode::KeyC),
        ),
        KeyMapping::new(
            "key.loadToolbarActivator",
            LoadHotbarActivator,
            Key(KeyCode::KeyX),
        ),
        KeyMapping::new("key.advancements", Advancements, Key(KeyCode::KeyL)),
        KeyMapping::new("key.quickActions", QuickActions, Key(KeyCode::KeyG)),
        KeyMapping::new("key.hotbar.1", HotbarSlot(0), Key(KeyCode::Digit1)),
        KeyMapping::new("key.hotbar.2", HotbarSlot(1), Key(KeyCode::Digit2)),
        KeyMapping::new("key.hotbar.3", HotbarSlot(2), Key(KeyCode::Digit3)),
        KeyMapping::new("key.hotbar.4", HotbarSlot(3), Key(KeyCode::Digit4)),
        KeyMapping::new("key.hotbar.5", HotbarSlot(4), Key(KeyCode::Digit5)),
        KeyMapping::new("key.hotbar.6", HotbarSlot(5), Key(KeyCode::Digit6)),
        KeyMapping::new("key.hotbar.7", HotbarSlot(6), Key(KeyCode::Digit7)),
        KeyMapping::new("key.hotbar.8", HotbarSlot(7), Key(KeyCode::Digit8)),
        KeyMapping::new("key.hotbar.9", HotbarSlot(8), Key(KeyCode::Digit9)),
        KeyMapping::new_with_category(
            "key.debug.overlay",
            DebugOverlay,
            Key(KeyCode::F3),
            KeyCategory::Debug,
            -2,
        ),
        KeyMapping::new_with_category(
            "key.debug.modifier",
            DebugModifier,
            Key(KeyCode::F3),
            KeyCategory::Debug,
            -1,
        ),
        KeyMapping::new("key.debug.crash", DebugCrash, Key(KeyCode::KeyC)),
        KeyMapping::new(
            "key.debug.reloadChunk",
            DebugReloadChunk,
            Key(KeyCode::KeyA),
        ),
        KeyMapping::new(
            "key.debug.showHitboxes",
            DebugShowHitboxes,
            Key(KeyCode::KeyB),
        ),
        KeyMapping::new("key.debug.clearChat", DebugClearChat, Key(KeyCode::KeyD)),
        KeyMapping::new(
            "key.debug.showChunkBorders",
            DebugShowChunkBorders,
            Key(KeyCode::KeyG),
        ),
        KeyMapping::new(
            "key.debug.showAdvancedTooltips",
            DebugShowAdvancedTooltips,
            Key(KeyCode::KeyH),
        ),
        KeyMapping::new(
            "key.debug.copyRecreateCommand",
            DebugCopyRecreateCommand,
            Key(KeyCode::KeyI),
        ),
        KeyMapping::new("key.debug.spectate", DebugSpectate, Key(KeyCode::KeyN)),
        KeyMapping::new(
            "key.debug.switchGameMode",
            DebugSwitchGameMode,
            Key(KeyCode::F4),
        ),
        KeyMapping::new(
            "key.debug.debugOptions",
            DebugDebugOptions,
            Key(KeyCode::F6),
        ),
        KeyMapping::new("key.debug.focusPause", DebugFocusPause, Key(KeyCode::KeyP)),
        KeyMapping::new(
            "key.debug.dumpDynamicTextures",
            DebugDumpDynamicTextures,
            Key(KeyCode::KeyS),
        ),
        KeyMapping::new(
            "key.debug.reloadResourcePacks",
            DebugReloadResourcePacks,
            Key(KeyCode::KeyT),
        ),
        KeyMapping::new("key.debug.profiling", DebugProfiling, Key(KeyCode::KeyL)),
        KeyMapping::new(
            "key.debug.copyLocation",
            DebugCopyLocation,
            Key(KeyCode::KeyC),
        ),
        KeyMapping::new(
            "key.debug.dumpVersion",
            DebugDumpVersion,
            Key(KeyCode::KeyV),
        ),
        KeyMapping::new_with_category(
            "key.debug.profilingChart",
            DebugProfilingChart,
            Key(KeyCode::Digit1),
            KeyCategory::Debug,
            1,
        ),
        KeyMapping::new_with_category(
            "key.debug.fpsCharts",
            DebugFpsCharts,
            Key(KeyCode::Digit2),
            KeyCategory::Debug,
            2,
        ),
        KeyMapping::new_with_category(
            "key.debug.networkCharts",
            DebugNetworkCharts,
            Key(KeyCode::Digit3),
            KeyCategory::Debug,
            3,
        ),
        KeyMapping::new_with_category(
            "key.debug.lightmapTexture",
            DebugLightmapTexture,
            Key(KeyCode::Digit4),
            KeyCategory::Debug,
            4,
        ),
    ]
}
pub fn serialize_bound_input(input: BoundInput) -> String {
    match input {
        BoundInput::Key(code) => format!("key.keyboard.{}", serialize_key_code(code)),
        BoundInput::Mouse(btn) => format!("key.mouse.{}", u32::from(btn) + 1),
        BoundInput::Unbound => "key.keyboard.unknown".to_string(),
    }
}
pub fn deserialize_bound_input(s: &str) -> BoundInput {
    if s == "key.keyboard.unknown" {
        return BoundInput::Unbound;
    }
    if let Some(rest) = s.strip_prefix("key.keyboard.") {
        if let Some(code) = deserialize_key_code(rest) {
            return BoundInput::Key(code);
        }
    }
    if let Some(rest) = s.strip_prefix("key.mouse.") {
        if let Ok(num) = rest.parse::<u32>() {
            if let Some(button) = num
                .checked_sub(1)
                .and_then(|value| u16::try_from(value).ok())
            {
                return BoundInput::Mouse(button);
            }
        }
        match rest {
            "left" => return BoundInput::Mouse(0),
            "right" => return BoundInput::Mouse(1),
            "middle" => return BoundInput::Mouse(2),
            _ => {}
        }
    }
    if let Some(rest) = s.strip_prefix("Button ") {
        if let Ok(num) = rest.parse::<u32>() {
            if let Some(button) = num
                .checked_sub(1)
                .and_then(|value| u16::try_from(value).ok())
            {
                return BoundInput::Mouse(button);
            }
        }
    }
    let lower = s.to_lowercase();
    if let Some(code) = deserialize_key_code(&lower) {
        return BoundInput::Key(code);
    }
    match lower.as_str() {
        "left control" | "lcontrol" => return BoundInput::Key(KeyCode::ControlLeft),
        "right control" | "rcontrol" => return BoundInput::Key(KeyCode::ControlRight),
        "left shift" | "lshift" => return BoundInput::Key(KeyCode::ShiftLeft),
        "right shift" | "rshift" => return BoundInput::Key(KeyCode::ShiftRight),
        "left alt" | "lalt" => return BoundInput::Key(KeyCode::AltLeft),
        "right alt" | "ralt" => return BoundInput::Key(KeyCode::AltRight),
        _ => {}
    }
    BoundInput::Unbound
}
pub fn format_bound_input(input: BoundInput) -> String {
    match input {
        BoundInput::Key(code) => {
            let key = format!("key.keyboard.{}", serialize_key_code(code));
            let trans = crate::ui::i18n::translate(&key);
            if trans == key {
                serialize_key_code(code).to_uppercase()
            } else {
                trans.to_string()
            }
        }
        BoundInput::Mouse(btn) => {
            let key = match btn {
                0 => "key.mouse.left",
                1 => "key.mouse.right",
                2 => "key.mouse.middle",
                _ => return format!("Mouse {}", u32::from(btn) + 1),
            };
            crate::ui::i18n::translate(key).to_string()
        }
        BoundInput::Unbound => crate::ui::i18n::translate("key.keyboard.unknown").to_string(),
    }
}
pub fn get_keybind_category(name: &str) -> &'static str {
    if name.starts_with("key.hotbar.") {
        return "key.categories.inventory";
    }
    match name {
        "key.forward" | "key.left" | "key.back" | "key.right" | "key.jump" | "key.sneak"
        | "key.sprint" => "key.categories.movement",
        "key.attack" | "key.use" | "key.drop" | "key.pickItem" | "key.swapOffhand"
        | "key.advancements" => "key.categories.gameplay",
        "key.inventory" => "key.categories.inventory",
        "key.chat"
        | "key.playerlist"
        | "key.command"
        | "key.friends"
        | "key.socialInteractions"
        | "key.quickActions" => "key.categories.multiplayer",
        "key.saveToolbarActivator" | "key.loadToolbarActivator" => "key.categories.creative",
        _ => "key.categories.misc",
    }
}
fn serialize_key_code(code: KeyCode) -> &'static str {
    KEYBOARD_CODES
        .iter()
        .find_map(|(key, name)| (*key == code).then_some(*name))
        .unwrap_or("unknown")
}
fn deserialize_key_code(name: &str) -> Option<KeyCode> {
    KEYBOARD_CODES
        .iter()
        .find_map(|(key, token)| (*token == name).then_some(*key))
}
const KEYBOARD_CODES: &[(KeyCode, &str)] = &[
    (KeyCode::Backquote, "grave.accent"),
    (KeyCode::Backslash, "backslash"),
    (KeyCode::BracketLeft, "left.bracket"),
    (KeyCode::BracketRight, "right.bracket"),
    (KeyCode::Comma, "comma"),
    (KeyCode::Digit0, "0"),
    (KeyCode::Digit1, "1"),
    (KeyCode::Digit2, "2"),
    (KeyCode::Digit3, "3"),
    (KeyCode::Digit4, "4"),
    (KeyCode::Digit5, "5"),
    (KeyCode::Digit6, "6"),
    (KeyCode::Digit7, "7"),
    (KeyCode::Digit8, "8"),
    (KeyCode::Digit9, "9"),
    (KeyCode::Equal, "equal"),
    (KeyCode::IntlBackslash, "intl.backslash"),
    (KeyCode::IntlRo, "intl.ro"),
    (KeyCode::IntlYen, "intl.yen"),
    (KeyCode::KeyA, "a"),
    (KeyCode::KeyB, "b"),
    (KeyCode::KeyC, "c"),
    (KeyCode::KeyD, "d"),
    (KeyCode::KeyE, "e"),
    (KeyCode::KeyF, "f"),
    (KeyCode::KeyG, "g"),
    (KeyCode::KeyH, "h"),
    (KeyCode::KeyI, "i"),
    (KeyCode::KeyJ, "j"),
    (KeyCode::KeyK, "k"),
    (KeyCode::KeyL, "l"),
    (KeyCode::KeyM, "m"),
    (KeyCode::KeyN, "n"),
    (KeyCode::KeyO, "o"),
    (KeyCode::KeyP, "p"),
    (KeyCode::KeyQ, "q"),
    (KeyCode::KeyR, "r"),
    (KeyCode::KeyS, "s"),
    (KeyCode::KeyT, "t"),
    (KeyCode::KeyU, "u"),
    (KeyCode::KeyV, "v"),
    (KeyCode::KeyW, "w"),
    (KeyCode::KeyX, "x"),
    (KeyCode::KeyY, "y"),
    (KeyCode::KeyZ, "z"),
    (KeyCode::Minus, "minus"),
    (KeyCode::Period, "period"),
    (KeyCode::Quote, "apostrophe"),
    (KeyCode::Semicolon, "semicolon"),
    (KeyCode::Slash, "slash"),
    (KeyCode::AltLeft, "left.alt"),
    (KeyCode::AltRight, "right.alt"),
    (KeyCode::Backspace, "backspace"),
    (KeyCode::CapsLock, "caps.lock"),
    (KeyCode::ContextMenu, "menu"),
    (KeyCode::ControlLeft, "left.control"),
    (KeyCode::ControlRight, "right.control"),
    (KeyCode::Enter, "enter"),
    (KeyCode::SuperLeft, "left.win"),
    (KeyCode::SuperRight, "right.win"),
    (KeyCode::ShiftLeft, "left.shift"),
    (KeyCode::ShiftRight, "right.shift"),
    (KeyCode::Space, "space"),
    (KeyCode::Tab, "tab"),
    (KeyCode::Convert, "convert"),
    (KeyCode::KanaMode, "kana.mode"),
    (KeyCode::Lang1, "lang1"),
    (KeyCode::Lang2, "lang2"),
    (KeyCode::Lang3, "lang3"),
    (KeyCode::Lang4, "lang4"),
    (KeyCode::Lang5, "lang5"),
    (KeyCode::NonConvert, "non.convert"),
    (KeyCode::Delete, "delete"),
    (KeyCode::End, "end"),
    (KeyCode::Help, "help"),
    (KeyCode::Home, "home"),
    (KeyCode::Insert, "insert"),
    (KeyCode::PageDown, "page.down"),
    (KeyCode::PageUp, "page.up"),
    (KeyCode::ArrowDown, "down"),
    (KeyCode::ArrowLeft, "left"),
    (KeyCode::ArrowRight, "right"),
    (KeyCode::ArrowUp, "up"),
    (KeyCode::NumLock, "num.lock"),
    (KeyCode::Numpad0, "keypad.0"),
    (KeyCode::Numpad1, "keypad.1"),
    (KeyCode::Numpad2, "keypad.2"),
    (KeyCode::Numpad3, "keypad.3"),
    (KeyCode::Numpad4, "keypad.4"),
    (KeyCode::Numpad5, "keypad.5"),
    (KeyCode::Numpad6, "keypad.6"),
    (KeyCode::Numpad7, "keypad.7"),
    (KeyCode::Numpad8, "keypad.8"),
    (KeyCode::Numpad9, "keypad.9"),
    (KeyCode::NumpadAdd, "keypad.add"),
    (KeyCode::NumpadBackspace, "numpad.backspace"),
    (KeyCode::NumpadClear, "numpad.clear"),
    (KeyCode::NumpadClearEntry, "numpad.clear.entry"),
    (KeyCode::NumpadComma, "numpad.comma"),
    (KeyCode::NumpadDecimal, "keypad.decimal"),
    (KeyCode::NumpadDivide, "keypad.divide"),
    (KeyCode::NumpadEnter, "keypad.enter"),
    (KeyCode::NumpadEqual, "keypad.equal"),
    (KeyCode::NumpadHash, "numpad.hash"),
    (KeyCode::NumpadMemoryAdd, "numpad.memory.add"),
    (KeyCode::NumpadMemoryClear, "numpad.memory.clear"),
    (KeyCode::NumpadMemoryRecall, "numpad.memory.recall"),
    (KeyCode::NumpadMemoryStore, "numpad.memory.store"),
    (KeyCode::NumpadMemorySubtract, "numpad.memory.subtract"),
    (KeyCode::NumpadMultiply, "keypad.multiply"),
    (KeyCode::NumpadParenLeft, "numpad.paren.left"),
    (KeyCode::NumpadParenRight, "numpad.paren.right"),
    (KeyCode::NumpadStar, "numpad.star"),
    (KeyCode::NumpadSubtract, "keypad.subtract"),
    (KeyCode::Escape, "escape"),
    (KeyCode::Fn, "fn"),
    (KeyCode::FnLock, "fn.lock"),
    (KeyCode::PrintScreen, "print.screen"),
    (KeyCode::ScrollLock, "scroll.lock"),
    (KeyCode::Pause, "pause"),
    (KeyCode::BrowserBack, "browser.back"),
    (KeyCode::BrowserFavorites, "browser.favorites"),
    (KeyCode::BrowserForward, "browser.forward"),
    (KeyCode::BrowserHome, "browser.home"),
    (KeyCode::BrowserRefresh, "browser.refresh"),
    (KeyCode::BrowserSearch, "browser.search"),
    (KeyCode::BrowserStop, "browser.stop"),
    (KeyCode::Eject, "eject"),
    (KeyCode::LaunchApp1, "launch.app1"),
    (KeyCode::LaunchApp2, "launch.app2"),
    (KeyCode::LaunchMail, "launch.mail"),
    (KeyCode::MediaPlayPause, "media.play.pause"),
    (KeyCode::MediaSelect, "media.select"),
    (KeyCode::MediaStop, "media.stop"),
    (KeyCode::MediaTrackNext, "media.track.next"),
    (KeyCode::MediaTrackPrevious, "media.track.previous"),
    (KeyCode::Power, "power"),
    (KeyCode::Sleep, "sleep"),
    (KeyCode::AudioVolumeDown, "audio.volume.down"),
    (KeyCode::AudioVolumeMute, "audio.volume.mute"),
    (KeyCode::AudioVolumeUp, "audio.volume.up"),
    (KeyCode::WakeUp, "wake.up"),
    (KeyCode::Meta, "meta"),
    (KeyCode::Hyper, "hyper"),
    (KeyCode::Turbo, "turbo"),
    (KeyCode::Abort, "abort"),
    (KeyCode::Resume, "resume"),
    (KeyCode::Suspend, "suspend"),
    (KeyCode::Again, "again"),
    (KeyCode::Copy, "copy"),
    (KeyCode::Cut, "cut"),
    (KeyCode::Find, "find"),
    (KeyCode::Open, "open"),
    (KeyCode::Paste, "paste"),
    (KeyCode::Props, "props"),
    (KeyCode::Select, "select"),
    (KeyCode::Undo, "undo"),
    (KeyCode::Hiragana, "hiragana"),
    (KeyCode::Katakana, "katakana"),
    (KeyCode::F1, "f1"),
    (KeyCode::F2, "f2"),
    (KeyCode::F3, "f3"),
    (KeyCode::F4, "f4"),
    (KeyCode::F5, "f5"),
    (KeyCode::F6, "f6"),
    (KeyCode::F7, "f7"),
    (KeyCode::F8, "f8"),
    (KeyCode::F9, "f9"),
    (KeyCode::F10, "f10"),
    (KeyCode::F11, "f11"),
    (KeyCode::F12, "f12"),
    (KeyCode::F13, "f13"),
    (KeyCode::F14, "f14"),
    (KeyCode::F15, "f15"),
    (KeyCode::F16, "f16"),
    (KeyCode::F17, "f17"),
    (KeyCode::F18, "f18"),
    (KeyCode::F19, "f19"),
    (KeyCode::F20, "f20"),
    (KeyCode::F21, "f21"),
    (KeyCode::F22, "f22"),
    (KeyCode::F23, "f23"),
    (KeyCode::F24, "f24"),
    (KeyCode::F25, "f25"),
    (KeyCode::F26, "f26"),
    (KeyCode::F27, "f27"),
    (KeyCode::F28, "f28"),
    (KeyCode::F29, "f29"),
    (KeyCode::F30, "f30"),
    (KeyCode::F31, "f31"),
    (KeyCode::F32, "f32"),
    (KeyCode::F33, "f33"),
    (KeyCode::F34, "f34"),
    (KeyCode::F35, "f35"),
];
