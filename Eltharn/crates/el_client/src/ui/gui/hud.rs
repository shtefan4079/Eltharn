use crate::client::waypoint::LocatorBarMarker;
use crate::gameplay::camera::CameraType;
use crate::gameplay::player::local_player::{
    ActiveMobEffect, LocalPlayer, MOB_EFFECT_HUNGER, MOB_EFFECT_POISON, MOB_EFFECT_REGENERATION,
    MOB_EFFECT_WITHER, compare_mob_effects, mob_effect_is_beneficial, mob_effect_path,
};
use crate::resources::manager::{ClientResourceKind, ClientResourceManager};
use crate::ui::chat::chat_component::{
    draw_formatted_chat_text, draw_formatted_chat_text_ex, draw_formatted_chat_text_override_color,
    formatted_chat_width,
};
use crate::ui::gui::font::FontRenderer;
use crate::ui::gui::item_renderer::draw_gui_item;
use crate::ui::gui::renderer::{DrawBatch, GuiRenderer, GuiVertex};
use crate::ui::gui::scoreboard_hud::HudBackgroundOptions;
use el_core::item::{DataComponent, ItemStack};
use el_core::resource_location::ResourceLocation;
use el_math::JavaRandom;
use el_nbt::tag::Tag;
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;
use web_time::Instant;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossBarColor {
    Pink = 0,
    Blue = 1,
    Red = 2,
    Green = 3,
    Yellow = 4,
    Purple = 5,
    White = 6,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossBarOverlay {
    Progress = 0,
    Notched6 = 1,
    Notched10 = 2,
    Notched12 = 3,
    Notched20 = 4,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BossEventStyle {
    pub color: BossBarColor,
    pub overlay: BossBarOverlay,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BossEventProperties {
    pub darken_screen: bool,
    pub play_music: bool,
    pub create_world_fog: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BossEventAdd {
    pub name: String,
    pub progress: f32,
    pub style: BossEventStyle,
    pub properties: BossEventProperties,
}
#[derive(Clone, Debug, PartialEq)]
pub enum BossEventOperation {
    Add(BossEventAdd),
    Remove,
    UpdateProgress(f32),
    UpdateName(String),
    UpdateStyle(BossEventStyle),
    UpdateProperties(BossEventProperties),
}
const HOTBAR_WIDTH: f32 = 182.0;
const HOTBAR_HEIGHT: f32 = 22.0;
const HOTBAR_HALF_WIDTH: f32 = 91.0;
const HOTBAR_SELECTION_WIDTH: f32 = 24.0;
const HOTBAR_SELECTION_HEIGHT: f32 = 23.0;
const HOTBAR_OFFHAND_WIDTH: f32 = 29.0;
const HOTBAR_OFFHAND_HEIGHT: f32 = 24.0;
const HOTBAR_ITEM_Y_OFFSET: f32 = 19.0;
const CONTEXTUAL_BAR_MARGIN_BOTTOM: f32 = 24.0;
const CONTEXTUAL_BAR_HEIGHT: f32 = 5.0;
const PLAYER_BAR_BASELINE_FROM_BOTTOM: f32 = 39.0;
const PLAYER_BAR_ICON_SIZE: f32 = 9.0;
const PLAYER_BAR_ICON_STEP: f32 = 8.0;
const TOOL_HIGHLIGHT_BASE_TICKS: f32 = 40.0;
const OVERLAY_MESSAGE_TICKS: u32 = 60;
const MAX_AIR_SUPPLY: i32 = 300;
const LOCATOR_BAR_WIDTH: f32 = 182.0;
const LOCATOR_BAR_DOT_SIZE: f32 = 9.0;
const LOCATOR_BAR_ARROW_WIDTH: f32 = 7.0;
const LOCATOR_BAR_ARROW_HEIGHT: f32 = 5.0;
const LOCATOR_BAR_VISIBLE_DEGREES: f32 = 60.0;
const LOCATOR_BAR_DOT_RANGE: f32 = 173.0;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HudSpriteRegion {
    source_width: i32,
    source_height: i32,
    source_x: i32,
    source_y: i32,
    width: i32,
    height: i32,
}
#[derive(Clone, Debug)]
struct LocatorWaypointStyle {
    near_distance: i32,
    far_distance: i32,
    sprite_locations: Vec<ResourceLocation>,
}
impl LocatorWaypointStyle {
    fn sprite(&self, distance: f32) -> ResourceLocation {
        if distance < self.near_distance as f32 {
            return self.sprite_locations[0].clone();
        }
        if distance >= self.far_distance as f32 {
            return self
                .sprite_locations
                .last()
                .cloned()
                .unwrap_or_else(missing_sprite);
        }
        if self.sprite_locations.len() == 1 {
            return self.sprite_locations[0].clone();
        }
        if self.sprite_locations.len() == 3 {
            return self.sprite_locations[1].clone();
        }
        let alpha = (distance - self.near_distance as f32)
            / (self.far_distance - self.near_distance) as f32;
        let index = 1 + (alpha * (self.sprite_locations.len() - 2) as f32).floor() as usize;
        self.sprite_locations[index].clone()
    }
}
#[derive(Clone, Debug, Default)]
struct LocatorWaypointStyles {
    styles: HashMap<ResourceLocation, LocatorWaypointStyle>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HudHeartType {
    Normal,
    Poisoned,
    Withered,
    Frozen,
    Absorbing,
}
impl HudHeartType {
    fn for_player(player: &LocalPlayer) -> Self {
        if player.has_effect(MOB_EFFECT_POISON) {
            Self::Poisoned
        } else if player.has_effect(MOB_EFFECT_WITHER) {
            Self::Withered
        } else if player.is_fully_frozen() {
            Self::Frozen
        } else {
            Self::Normal
        }
    }
    fn sprite(self, half: bool, blinking: bool, hardcore: bool) -> ResourceLocation {
        let path = match (self, hardcore, half, blinking) {
            (Self::Normal, false, false, false) => "hud/heart/full",
            (Self::Normal, false, false, true) => "hud/heart/full_blinking",
            (Self::Normal, false, true, false) => "hud/heart/half",
            (Self::Normal, false, true, true) => "hud/heart/half_blinking",
            (Self::Normal, true, false, false) => "hud/heart/hardcore_full",
            (Self::Normal, true, false, true) => "hud/heart/hardcore_full_blinking",
            (Self::Normal, true, true, false) => "hud/heart/hardcore_half",
            (Self::Normal, true, true, true) => "hud/heart/hardcore_half_blinking",
            (Self::Poisoned, false, false, false) => "hud/heart/poisoned_full",
            (Self::Poisoned, false, false, true) => "hud/heart/poisoned_full_blinking",
            (Self::Poisoned, false, true, false) => "hud/heart/poisoned_half",
            (Self::Poisoned, false, true, true) => "hud/heart/poisoned_half_blinking",
            (Self::Poisoned, true, false, false) => "hud/heart/poisoned_hardcore_full",
            (Self::Poisoned, true, false, true) => "hud/heart/poisoned_hardcore_full_blinking",
            (Self::Poisoned, true, true, false) => "hud/heart/poisoned_hardcore_half",
            (Self::Poisoned, true, true, true) => "hud/heart/poisoned_hardcore_half_blinking",
            (Self::Withered, false, false, false) => "hud/heart/withered_full",
            (Self::Withered, false, false, true) => "hud/heart/withered_full_blinking",
            (Self::Withered, false, true, false) => "hud/heart/withered_half",
            (Self::Withered, false, true, true) => "hud/heart/withered_half_blinking",
            (Self::Withered, true, false, false) => "hud/heart/withered_hardcore_full",
            (Self::Withered, true, false, true) => "hud/heart/withered_hardcore_full_blinking",
            (Self::Withered, true, true, false) => "hud/heart/withered_hardcore_half",
            (Self::Withered, true, true, true) => "hud/heart/withered_hardcore_half_blinking",
            (Self::Frozen, false, false, false) => "hud/heart/frozen_full",
            (Self::Frozen, false, false, true) => "hud/heart/frozen_full_blinking",
            (Self::Frozen, false, true, false) => "hud/heart/frozen_half",
            (Self::Frozen, false, true, true) => "hud/heart/frozen_half_blinking",
            (Self::Frozen, true, false, false) => "hud/heart/frozen_hardcore_full",
            (Self::Frozen, true, false, true) => "hud/heart/frozen_hardcore_full_blinking",
            (Self::Frozen, true, true, false) => "hud/heart/frozen_hardcore_half",
            (Self::Frozen, true, true, true) => "hud/heart/frozen_hardcore_half_blinking",
            (Self::Absorbing, false, false, false) => "hud/heart/absorbing_full",
            (Self::Absorbing, false, false, true) => "hud/heart/absorbing_full_blinking",
            (Self::Absorbing, false, true, false) => "hud/heart/absorbing_half",
            (Self::Absorbing, false, true, true) => "hud/heart/absorbing_half_blinking",
            (Self::Absorbing, true, false, false) => "hud/heart/absorbing_hardcore_full",
            (Self::Absorbing, true, false, true) => "hud/heart/absorbing_hardcore_full_blinking",
            (Self::Absorbing, true, true, false) => "hud/heart/absorbing_hardcore_half",
            (Self::Absorbing, true, true, true) => "hud/heart/absorbing_hardcore_half_blinking",
        };
        ResourceLocation::new("minecraft", path)
    }
}
#[derive(serde::Deserialize)]
struct LocatorWaypointStyleJson {
    #[serde(default = "default_locator_near_distance")]
    near_distance: i32,
    #[serde(default = "default_locator_far_distance")]
    far_distance: i32,
    sprites: Vec<String>,
}
const fn default_locator_near_distance() -> i32 {
    128
}
const fn default_locator_far_distance() -> i32 {
    332
}
impl LocatorWaypointStyles {
    fn reload(resource_manager: &ClientResourceManager) -> Self {
        let mut styles = HashMap::new();
        let namespaces = match resource_manager.namespaces() {
            Ok(namespaces) => namespaces,
            Err(error) => {
                log::warn!("[LocatorBar] cannot enumerate waypoint styles: {error}");
                return Self { styles };
            }
        };
        for namespace in namespaces {
            let ids = match resource_manager
                .list_resource_ids(ClientResourceKind::WaypointStyle, &namespace)
            {
                Ok(ids) => ids,
                Err(error) => {
                    log::warn!(
                        "[LocatorBar] cannot list waypoint styles in namespace {namespace}: {error}"
                    );
                    continue;
                }
            };
            for id in ids {
                let source =
                    match resource_manager.read_string(ClientResourceKind::WaypointStyle, &id) {
                        Ok(source) => source,
                        Err(error) => {
                            log::warn!("[LocatorBar] cannot read waypoint style {id}: {error}");
                            continue;
                        }
                    };
                let definition: LocatorWaypointStyleJson = match serde_json::from_str(&source) {
                    Ok(definition) => definition,
                    Err(error) => {
                        log::warn!("[LocatorBar] cannot parse waypoint style {id}: {error}");
                        continue;
                    }
                };
                let Some(style) = parse_locator_waypoint_style(&id, definition) else {
                    continue;
                };
                styles.insert(id, style);
            }
        }
        Self { styles }
    }
    fn sprite(&self, style: &str, distance: f32) -> ResourceLocation {
        ResourceLocation::try_from_string(style)
            .and_then(|id| self.styles.get(&id))
            .map(|style| style.sprite(distance))
            .unwrap_or_else(missing_sprite)
    }
}
fn parse_locator_waypoint_style(
    id: &ResourceLocation,
    definition: LocatorWaypointStyleJson,
) -> Option<LocatorWaypointStyle> {
    if !(0..=60_000_000).contains(&definition.near_distance)
        || !(0..=60_000_000).contains(&definition.far_distance)
        || definition.sprites.is_empty()
        || definition.near_distance <= 0
        || definition.near_distance >= definition.far_distance
    {
        log::warn!("[LocatorBar] rejected invalid Java waypoint style {id}");
        return None;
    }
    let mut sprite_locations = Vec::with_capacity(definition.sprites.len());
    for sprite in definition.sprites {
        let Some(sprite) = ResourceLocation::try_from_string(&sprite) else {
            log::warn!("[LocatorBar] rejected waypoint style {id} with invalid sprite {sprite}");
            return None;
        };
        let path = format!("hud/locator_bar_dot/{}", sprite.path);
        let Some(location) = ResourceLocation::try_new(&sprite.namespace, &path) else {
            log::warn!("[LocatorBar] rejected waypoint style {id} with invalid sprite path");
            return None;
        };
        sprite_locations.push(location);
    }
    Some(LocatorWaypointStyle {
        near_distance: definition.near_distance,
        far_distance: definition.far_distance,
        sprite_locations,
    })
}
fn missing_sprite() -> ResourceLocation {
    ResourceLocation::new("minecraft", "missingno")
}
fn argb_to_rgba(argb: u32) -> [f32; 4] {
    [
        ((argb >> 16) & 0xFF) as f32 / 255.0,
        ((argb >> 8) & 0xFF) as f32 / 255.0,
        (argb & 0xFF) as f32 / 255.0,
        ((argb >> 24) & 0xFF) as f32 / 255.0,
    ]
}
fn lerp_discrete(alpha: f32, p0: i32, p1: i32) -> i32 {
    p0 + (alpha * (p1 - p0 - 1) as f32).floor() as i32 + if alpha > 0.0 { 1 } else { 0 }
}
fn hsv_to_argb(hue: f32, saturation: f32, value: f32, alpha: f32) -> [f32; 4] {
    let h = (hue * 6.0) as i32 % 6;
    let f = hue * 6.0 - h as f32;
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - f * saturation);
    let t = value * (1.0 - (1.0 - f) * saturation);
    let (red, green, blue) = match h {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        _ => (value, p, q),
    };
    let byte = |v: f32| ((v.clamp(0.0, 1.0) * 255.0) as u8) as f32 / 255.0;
    [byte(red), byte(green), byte(blue), alpha.clamp(0.0, 1.0)]
}
#[derive(Clone, Debug, PartialEq)]
pub struct BossEventState {
    pub name: String,
    pub progress: f32,
    pub color: BossBarColor,
    pub overlay: BossBarOverlay,
    pub darken_screen: bool,
    pub play_music: bool,
    pub create_world_fog: bool,
}
#[derive(Clone, Debug, PartialEq)]
struct HudStateKey {
    logical_w_bits: u32,
    logical_h_bits: u32,
    active_hotbar_slot: usize,
    camera_type: CameraType,
    can_hurt_player: bool,
    hardcore: bool,
    has_experience: bool,
    prioritize_experience: bool,
    has_locator_waypoints: bool,
    attack_indicator: i32,
    attack_strength_scale_bits: u32,
    main_hand_left: bool,
    crosshair_target_living_with_delay: bool,
    hud_bg_chat_only: bool,
    hud_bg_opacity_bits: u32,
    health_bits: u32,
    max_health_bits: u32,
    absorption_bits: u32,
    armor: i32,
    food: i32,
    food_saturation_bits: u32,
    air: i32,
    is_under_water: bool,
    xp_level: i32,
    xp_progress_bits: u32,
    active_effects: Vec<(u32, ActiveMobEffect)>,
    hotbar_items: Vec<(usize, ItemStack, u32)>,
    locator_markers: Vec<LocatorBarMarker>,
    tick_count: u32,
    display_health: i32,
    health_blink_until_tick: u32,
    health_blink: bool,
    tool_highlight_timer: u32,
    last_tool_highlight: Option<ItemStack>,
    overlay_message: Option<String>,
    overlay_message_time: u32,
    overlay_message_animate_color: bool,
    boss_events: Vec<(uuid::Uuid, BossEventState)>,
    show_debug: bool,
    debug_fps: u32,
    debug_pos_bits: [u32; 3],
    debug_yaw_bits: u32,
    debug_pitch_bits: u32,
    debug_target_block: Option<el_math::block_pos::BlockPos>,
    custom_textures_len: usize,
}
#[derive(Default)]
struct HudRenderCache {
    state_key: Option<HudStateKey>,
    vertices: Vec<GuiVertex>,
    indices: Vec<u32>,
    batches: Vec<DrawBatch>,
}
pub struct HudRenderer {
    pub show_debug: bool,
    pub chat_messages: Vec<String>,
    overlay_message: Option<String>,
    overlay_message_time: u32,
    overlay_message_animate_color: bool,
    tick_count: u32,
    tool_highlight_timer: u32,
    last_tool_highlight: Option<ItemStack>,
    last_tool_highlight_key: Option<ToolHighlightKey>,
    last_health: i32,
    display_health: i32,
    last_health_time: Option<Instant>,
    health_blink_until_tick: u32,
    locator_waypoint_styles: LocatorWaypointStyles,
    pub boss_events: std::collections::HashMap<uuid::Uuid, BossEventState>,
    cache: RefCell<HudRenderCache>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct ToolHighlightKey {
    item: ResourceLocation,
    hover_name: String,
    custom_name: bool,
}
impl HudRenderer {
    pub fn new() -> Self {
        Self {
            show_debug: false,
            chat_messages: Vec::new(),
            overlay_message: None,
            overlay_message_time: 0,
            overlay_message_animate_color: false,
            tick_count: 0,
            tool_highlight_timer: 0,
            last_tool_highlight: None,
            last_tool_highlight_key: None,
            last_health: 20,
            display_health: 20,
            last_health_time: None,
            health_blink_until_tick: 0,
            locator_waypoint_styles: LocatorWaypointStyles::default(),
            boss_events: std::collections::HashMap::new(),
            cache: RefCell::new(HudRenderCache::default()),
        }
    }
    pub fn invalidate_cache(&self) {
        if let Ok(mut cache) = self.cache.try_borrow_mut() {
            cache.state_key = None;
        }
    }
    pub fn reload_locator_waypoint_styles(&mut self, resource_manager: &ClientResourceManager) {
        self.locator_waypoint_styles = LocatorWaypointStyles::reload(resource_manager);
        self.invalidate_cache();
    }
    pub fn update_boss_event(&mut self, id: uuid::Uuid, operation: BossEventOperation) {
        self.invalidate_cache();
        match operation {
            BossEventOperation::Add(add) => {
                self.boss_events.insert(
                    id,
                    BossEventState {
                        name: add.name,
                        progress: add.progress,
                        color: add.style.color,
                        overlay: add.style.overlay,
                        darken_screen: add.properties.darken_screen,
                        play_music: add.properties.play_music,
                        create_world_fog: add.properties.create_world_fog,
                    },
                );
            }
            BossEventOperation::Remove => {
                self.boss_events.remove(&id);
            }
            BossEventOperation::UpdateProgress(progress) => {
                if let Some(event) = self.boss_events.get_mut(&id) {
                    event.progress = progress;
                }
            }
            BossEventOperation::UpdateName(name) => {
                if let Some(event) = self.boss_events.get_mut(&id) {
                    event.name = name;
                }
            }
            BossEventOperation::UpdateStyle(style) => {
                if let Some(event) = self.boss_events.get_mut(&id) {
                    event.color = style.color;
                    event.overlay = style.overlay;
                }
            }
            BossEventOperation::UpdateProperties(props) => {
                if let Some(event) = self.boss_events.get_mut(&id) {
                    event.darken_screen = props.darken_screen;
                    event.play_music = props.play_music;
                    event.create_world_fog = props.create_world_fog;
                }
            }
        }
    }
    pub fn should_play_boss_music(&self) -> bool {
        self.boss_events.values().any(|event| event.play_music)
    }
    pub fn clear_boss_events(&mut self) {
        self.boss_events.clear();
        self.invalidate_cache();
    }
    pub fn set_overlay_message(&mut self, message: impl Into<String>) {
        self.set_overlay_message_with_animation(message, false);
    }
    pub fn set_overlay_message_with_animation(
        &mut self,
        message: impl Into<String>,
        animate_color: bool,
    ) {
        self.overlay_message = Some(message.into());
        self.overlay_message_time = OVERLAY_MESSAGE_TICKS;
        self.overlay_message_animate_color = animate_color;
        self.invalidate_cache();
    }
    pub fn tick(
        &mut self,
        player: &LocalPlayer,
        active_hotbar_slot: usize,
        notification_display_time: f32,
        client_ticks: u32,
    ) {
        self.tick_count = self.tick_count.wrapping_add(client_ticks);
        self.overlay_message_time = self.overlay_message_time.saturating_sub(client_ticks);
        if self.overlay_message_time == 0 {
            self.overlay_message = None;
        }
        self.tick_selected_item_name(
            player,
            active_hotbar_slot,
            notification_display_time,
            client_ticks,
        );
        self.tick_health_state(player);
    }
    fn tick_selected_item_name(
        &mut self,
        player: &LocalPlayer,
        active_hotbar_slot: usize,
        notification_display_time: f32,
        client_ticks: u32,
    ) {
        let selected = player
            .inventory
            .get(active_hotbar_slot)
            .cloned()
            .unwrap_or_else(ItemStack::empty);
        if selected.is_empty() {
            self.tool_highlight_timer = 0;
            self.last_tool_highlight = None;
            self.last_tool_highlight_key = None;
            return;
        }
        let key = ToolHighlightKey {
            item: selected.item.location.clone(),
            hover_name: selected_item_name(&selected),
            custom_name: has_custom_name(&selected),
        };
        if self.last_tool_highlight_key.as_ref() != Some(&key) {
            self.tool_highlight_timer =
                (TOOL_HIGHLIGHT_BASE_TICKS * notification_display_time.max(0.0)).round() as u32;
        } else if self.tool_highlight_timer > 0 {
            self.tool_highlight_timer = self.tool_highlight_timer.saturating_sub(client_ticks);
        }
        self.last_tool_highlight = Some(selected);
        self.last_tool_highlight_key = Some(key);
    }
    fn tick_health_state(&mut self, player: &LocalPlayer) {
        let current_health = player.health.ceil().max(0.0) as i32;
        let now = Instant::now();
        if current_health < self.last_health && player.invulnerable_time > 0 {
            self.last_health_time = Some(now);
            self.health_blink_until_tick = self.tick_count.saturating_add(20);
        } else if current_health > self.last_health && player.invulnerable_time > 0 {
            self.last_health_time = Some(now);
            self.health_blink_until_tick = self.tick_count.saturating_add(10);
        }
        let should_snap_display_health = self
            .last_health_time
            .map(|time| time.elapsed() > Duration::from_millis(1000))
            .unwrap_or(true);
        if should_snap_display_health {
            self.display_health = current_health;
            self.last_health_time = Some(now);
        }
        self.last_health = current_health;
    }
    pub fn render(
        &self,
        renderer: &mut GuiRenderer,
        font: &FontRenderer,
        player: &LocalPlayer,
        screen_w: f32,
        screen_h: f32,
        active_hotbar_slot: usize,
        fps: u32,
        camera_type: CameraType,
        can_hurt_player: bool,
        hardcore: bool,
        has_experience: bool,
        prioritize_experience: bool,
        has_locator_waypoints: bool,
        locator_markers: &[LocatorBarMarker],
        attack_indicator: i32,
        attack_strength_scale: f32,
        main_hand_left: bool,
        crosshair_target_living: bool,
        hud_background: HudBackgroundOptions,
    ) {
        let mut hotbar_items = Vec::with_capacity(10);
        for slot in 0..9 {
            if let Some(item) = player.inventory.get(slot) {
                if !item.is_empty() {
                    let cooldown = player.item_cooldown_percent(item, 0.0);
                    hotbar_items.push((slot, item.clone(), cooldown.to_bits()));
                }
            }
        }
        if let Some(offhand) = player.inventory.get(40) {
            if !offhand.is_empty() {
                let cooldown = player.item_cooldown_percent(offhand, 0.0);
                hotbar_items.push((40, offhand.clone(), cooldown.to_bits()));
            }
        }
        let mut active_effects: Vec<(u32, ActiveMobEffect)> = player
            .active_effects
            .iter()
            .map(|(&id, &effect)| (id, effect))
            .collect();
        active_effects.sort_unstable_by_key(|&(id, _)| id);
        let mut boss_events: Vec<(uuid::Uuid, BossEventState)> = self
            .boss_events
            .iter()
            .map(|(&id, state)| (id, state.clone()))
            .collect();
        boss_events.sort_unstable_by_key(|&(id, _)| id);
        let health_blink = self.health_blink_until_tick > self.tick_count
            && ((self.health_blink_until_tick - self.tick_count) / 3) % 2 == 1;
        #[cfg(feature = "extensions")]
        let health_blink = crate::modding::hooks::health_blink(health_blink);
        let state_key = HudStateKey {
            logical_w_bits: screen_w.to_bits(),
            logical_h_bits: screen_h.to_bits(),
            active_hotbar_slot,
            camera_type,
            can_hurt_player,
            hardcore,
            has_experience,
            prioritize_experience,
            has_locator_waypoints,
            attack_indicator,
            attack_strength_scale_bits: attack_strength_scale.to_bits(),
            main_hand_left,
            crosshair_target_living_with_delay: crosshair_target_living,
            hud_bg_chat_only: hud_background.background_for_chat_only,
            hud_bg_opacity_bits: hud_background.text_background_opacity.to_bits(),
            health_bits: player.health.to_bits(),
            max_health_bits: player
                .attr_f32(crate::gameplay::attributes::MAX_HEALTH)
                .to_bits(),
            absorption_bits: player.absorption.to_bits(),
            armor: player.attr_f64(crate::gameplay::attributes::ARMOR).floor() as i32,
            food: player.food,
            food_saturation_bits: player.food_saturation.to_bits(),
            air: player.air,
            is_under_water: player.is_under_water,
            xp_level: player.xp_level,
            xp_progress_bits: player.xp_progress.to_bits(),
            active_effects,
            hotbar_items,
            locator_markers: locator_markers.to_vec(),
            tick_count: self.tick_count,
            display_health: self.display_health,
            health_blink_until_tick: self.health_blink_until_tick,
            health_blink,
            tool_highlight_timer: self.tool_highlight_timer,
            last_tool_highlight: self.last_tool_highlight.clone(),
            overlay_message: self.overlay_message.clone(),
            overlay_message_time: self.overlay_message_time,
            overlay_message_animate_color: self.overlay_message_animate_color,
            boss_events,
            show_debug: self.show_debug,
            debug_fps: if self.show_debug { fps } else { 0 },
            debug_pos_bits: if self.show_debug {
                [
                    player.position.x.to_bits(),
                    player.position.y.to_bits(),
                    player.position.z.to_bits(),
                ]
            } else {
                [0, 0, 0]
            },
            debug_yaw_bits: if self.show_debug {
                player.yaw.to_bits()
            } else {
                0
            },
            debug_pitch_bits: if self.show_debug {
                player.pitch.to_bits()
            } else {
                0
            },
            debug_target_block: if self.show_debug {
                player.target_block.clone()
            } else {
                None
            },
            custom_textures_len: renderer.custom_textures.len(),
        };
        if let Ok(cache) = self.cache.try_borrow() {
            if let Some(cached_key) = &cache.state_key {
                if cached_key == &state_key {
                    renderer.append_geometry(&cache.vertices, &cache.indices, &cache.batches);
                    return;
                }
            }
        }
        let v_start = renderer.vertices.len();
        let i_start = renderer.indices.len();
        let b_start = renderer.batches.len();
        let center_x = (screen_w / 2.0).floor();
        let hotbar_x = center_x - HOTBAR_HALF_WIDTH;
        let hotbar_y = screen_h - HOTBAR_HEIGHT;
        if should_render_crosshair(camera_type) {
            self.render_crosshair(renderer, screen_w, screen_h);
            if attack_indicator == 1 {
                self.render_crosshair_attack_indicator(
                    renderer,
                    screen_w,
                    screen_h,
                    attack_strength_scale,
                    crosshair_target_living,
                );
            }
        }
        self.render_hotbar(
            renderer,
            font,
            player,
            center_x,
            hotbar_x,
            hotbar_y,
            active_hotbar_slot,
            main_hand_left,
        );
        if attack_indicator == 2 {
            self.render_hotbar_attack_indicator(
                renderer,
                screen_w,
                screen_h,
                attack_strength_scale,
                main_hand_left,
            );
        }
        self.render_contextual_bar(
            renderer,
            font,
            player,
            screen_w,
            screen_h,
            has_experience,
            prioritize_experience,
            has_locator_waypoints,
            locator_markers,
        );
        if can_hurt_player {
            self.render_player_status_bars(
                renderer,
                player,
                screen_w,
                screen_h,
                hardcore,
                health_blink,
            );
        }
        self.render_active_effects(renderer, player, screen_w);
        self.render_selected_item_name(
            renderer,
            font,
            screen_w,
            screen_h,
            can_hurt_player,
            hud_background,
        );
        self.render_overlay_message(renderer, font, screen_w, screen_h, hud_background);
        self.render_boss_bars(renderer, font, screen_w, screen_h);
        if self.show_debug {
            self.render_debug_overlay(renderer, font, player, fps);
        }
        if v_start == 0 && i_start == 0 && b_start == 0 {
            if let Ok(mut cache) = self.cache.try_borrow_mut() {
                cache.state_key = Some(state_key);
                cache.vertices.clear();
                cache.vertices.extend_from_slice(&renderer.vertices);
                cache.indices.clear();
                cache.indices.extend_from_slice(&renderer.indices);
                cache.batches.clear();
                cache.batches.extend_from_slice(&renderer.batches);
            }
        }
    }
    fn render_active_effects(
        &self,
        renderer: &mut GuiRenderer,
        player: &LocalPlayer,
        screen_w: f32,
    ) {
        let mut effects: Vec<(u32, ActiveMobEffect)> = player
            .active_effects
            .iter()
            .filter_map(|(&effect_id, &effect)| {
                effect
                    .show_icon
                    .then_some((effect_id, effect))
                    .filter(|(id, _)| mob_effect_path(*id).is_some())
            })
            .collect();
        effects.sort_by(|(left_id, left), (right_id, right)| {
            compare_mob_effects(*left_id, *left, *right_id, *right).reverse()
        });
        let mut beneficial_count = 0usize;
        let mut harmful_count = 0usize;
        for (effect_id, effect) in effects {
            let (x, y) = if mob_effect_is_beneficial(effect_id) {
                beneficial_count += 1;
                (screen_w - 25.0 * beneficial_count as f32, 1.0)
            } else {
                harmful_count += 1;
                (screen_w - 25.0 * harmful_count as f32, 27.0)
            };
            let background = if effect.ambient {
                "hud/effect_background_ambient"
            } else {
                "hud/effect_background"
            };
            let background = ResourceLocation::new("minecraft", background);
            renderer.draw_sprite(x, y, 24.0, 24.0, &background, [1.0; 4]);
            let mut alpha = 1.0;
            if !effect.ambient && effect.duration >= 0 && effect.duration <= 200 {
                let used_seconds = 10 - effect.duration / 20;
                alpha = (effect.duration as f32 / 10.0 / 5.0 * 0.5).clamp(0.0, 0.5)
                    + el_math::mth::cos(effect.duration as f32 * std::f32::consts::PI / 5.0)
                        * (used_seconds as f32 / 10.0 * 0.25).clamp(0.0, 0.25);
                alpha = alpha.clamp(0.0, 1.0);
            }
            let icon_path = format!("mob_effect/{}", mob_effect_path(effect_id).unwrap());
            let icon = ResourceLocation::new("minecraft", &icon_path);
            renderer.draw_sprite(x + 3.0, y + 3.0, 18.0, 18.0, &icon, [1.0, 1.0, 1.0, alpha]);
        }
    }
    fn render_crosshair(&self, renderer: &mut GuiRenderer, screen_w: f32, screen_h: f32) {
        let crosshair_loc = ResourceLocation::from_string("minecraft:hud/crosshair");
        let crosshair_x = ((screen_w as i32 - 15) / 2) as f32;
        let crosshair_y = ((screen_h as i32 - 15) / 2) as f32;
        renderer.draw_crosshair_sprite(
            crosshair_x,
            crosshair_y,
            15.0,
            15.0,
            &crosshair_loc,
            [1.0, 1.0, 1.0, 1.0],
        );
    }
    fn render_crosshair_attack_indicator(
        &self,
        renderer: &mut GuiRenderer,
        screen_w: f32,
        screen_h: f32,
        attack_strength_scale: f32,
        crosshair_target_living: bool,
    ) {
        let x = (screen_w / 2.0 - 8.0).floor();
        let y = (screen_h / 2.0 - 7.0 + 16.0).floor();
        if crosshair_target_living && attack_strength_scale >= 1.0 {
            let full =
                ResourceLocation::from_string("minecraft:hud/crosshair_attack_indicator_full");
            renderer.draw_crosshair_sprite(x, y, 16.0, 16.0, &full, [1.0; 4]);
            return;
        }
        if attack_strength_scale >= 1.0 {
            return;
        }
        let background =
            ResourceLocation::from_string("minecraft:hud/crosshair_attack_indicator_background");
        let progress =
            ResourceLocation::from_string("minecraft:hud/crosshair_attack_indicator_progress");
        renderer.draw_crosshair_sprite(x, y, 16.0, 4.0, &background, [1.0; 4]);
        if let Some(region) = crosshair_attack_indicator_progress_region(attack_strength_scale) {
            renderer.draw_crosshair_sprite_region(
                x,
                y,
                region.width as f32,
                region.height as f32,
                &progress,
                [1.0; 4],
                region.source_width as f32,
                region.source_height as f32,
                region.source_x as f32,
                region.source_y as f32,
            );
        }
    }
    fn render_hotbar_attack_indicator(
        &self,
        renderer: &mut GuiRenderer,
        screen_w: f32,
        screen_h: f32,
        attack_strength_scale: f32,
        main_hand_left: bool,
    ) {
        if attack_strength_scale >= 1.0 {
            return;
        }
        let x = if main_hand_left {
            (screen_w / 2.0 - 91.0 - 22.0).floor()
        } else {
            (screen_w / 2.0 + 91.0 + 6.0).floor()
        };
        let y = screen_h - 20.0;
        let background =
            ResourceLocation::from_string("minecraft:hud/hotbar_attack_indicator_background");
        let progress =
            ResourceLocation::from_string("minecraft:hud/hotbar_attack_indicator_progress");
        renderer.draw_sprite(x, y, 18.0, 18.0, &background, [1.0; 4]);
        if let Some(region) = hotbar_attack_indicator_progress_region(attack_strength_scale) {
            renderer.draw_sprite_region(
                x,
                y + region.source_y as f32,
                region.width as f32,
                region.height as f32,
                &progress,
                [1.0; 4],
                region.source_width as f32,
                region.source_height as f32,
                region.source_x as f32,
                region.source_y as f32,
            );
        }
    }
    fn render_hotbar(
        &self,
        renderer: &mut GuiRenderer,
        font: &FontRenderer,
        player: &LocalPlayer,
        center_x: f32,
        hotbar_x: f32,
        hotbar_y: f32,
        active_hotbar_slot: usize,
        main_hand_left: bool,
    ) {
        let hotbar_loc = ResourceLocation::from_string("minecraft:hud/hotbar");
        renderer.draw_sprite(
            hotbar_x,
            hotbar_y,
            HOTBAR_WIDTH,
            HOTBAR_HEIGHT,
            &hotbar_loc,
            [1.0, 1.0, 1.0, 1.0],
        );
        let selection_loc = ResourceLocation::from_string("minecraft:hud/hotbar_selection");
        renderer.draw_sprite(
            hotbar_x - 1.0 + active_hotbar_slot.min(8) as f32 * 20.0,
            hotbar_y - 1.0,
            HOTBAR_SELECTION_WIDTH,
            HOTBAR_SELECTION_HEIGHT,
            &selection_loc,
            [1.0, 1.0, 1.0, 1.0],
        );
        for slot in 0..9 {
            let Some(item) = player.inventory.get(slot) else {
                continue;
            };
            if item.is_empty() {
                continue;
            }
            let item_x = center_x - 90.0 + slot as f32 * 20.0 + 2.0;
            let item_y = hotbar_item_y(screen_h_from_hotbar_y(hotbar_y));
            let cooldown = player.item_cooldown_percent(item, 0.0);
            draw_gui_item(renderer, font, item, item_x, item_y, cooldown, true);
        }
        let Some(offhand) = player.inventory.get(40) else {
            return;
        };
        if offhand.is_empty() {
            return;
        }
        let (offhand_bg_loc, offhand_bg_x, offhand_item_x) = if main_hand_left {
            (
                ResourceLocation::from_string("minecraft:hud/hotbar_offhand_right"),
                center_x + HOTBAR_HALF_WIDTH,
                center_x + HOTBAR_HALF_WIDTH + 10.0,
            )
        } else {
            (
                ResourceLocation::from_string("minecraft:hud/hotbar_offhand_left"),
                center_x - HOTBAR_HALF_WIDTH - HOTBAR_OFFHAND_WIDTH,
                center_x - HOTBAR_HALF_WIDTH - 26.0,
            )
        };
        renderer.draw_sprite(
            offhand_bg_x,
            hotbar_y - 1.0,
            HOTBAR_OFFHAND_WIDTH,
            HOTBAR_OFFHAND_HEIGHT,
            &offhand_bg_loc,
            [1.0, 1.0, 1.0, 1.0],
        );
        let cooldown = player.item_cooldown_percent(offhand, 0.0);
        draw_gui_item(
            renderer,
            font,
            offhand,
            offhand_item_x,
            hotbar_item_y(screen_h_from_hotbar_y(hotbar_y)),
            cooldown,
            true,
        );
    }
    fn render_contextual_bar(
        &self,
        renderer: &mut GuiRenderer,
        font: &FontRenderer,
        player: &LocalPlayer,
        screen_w: f32,
        screen_h: f32,
        has_experience: bool,
        prioritize_experience: bool,
        has_locator_waypoints: bool,
        locator_markers: &[LocatorBarMarker],
    ) {
        let render_locator_bar =
            has_locator_waypoints && (!has_experience || !prioritize_experience);
        if render_locator_bar {
            self.render_locator_bar(renderer, screen_w, screen_h, locator_markers);
        } else if has_experience {
            self.render_experience_progress_bar(renderer, player, screen_w, screen_h);
        }
        if has_experience && player.xp_level > 0 {
            self.render_experience_level(renderer, font, player.xp_level, screen_w, screen_h);
        }
    }
    fn render_experience_progress_bar(
        &self,
        renderer: &mut GuiRenderer,
        player: &LocalPlayer,
        screen_w: f32,
        screen_h: f32,
    ) {
        let xp_needed_for_next_level = xp_needed_for_next_level(player.xp_level);
        if xp_needed_for_next_level <= 0 {
            return;
        }
        let xp_bg_loc = ResourceLocation::from_string("minecraft:hud/experience_bar_background");
        let xp_progress_loc =
            ResourceLocation::from_string("minecraft:hud/experience_bar_progress");
        let xp_x = ((screen_w - HOTBAR_WIDTH) / 2.0).floor();
        let xp_y = contextual_bar_top(screen_h);
        renderer.draw_sprite(
            xp_x,
            xp_y,
            HOTBAR_WIDTH,
            CONTEXTUAL_BAR_HEIGHT,
            &xp_bg_loc,
            [1.0, 1.0, 1.0, 1.0],
        );
        if let Some(region) = experience_bar_progress_region(player.xp_progress) {
            renderer.draw_sprite_region(
                xp_x,
                xp_y,
                region.width as f32,
                region.height as f32,
                &xp_progress_loc,
                [1.0, 1.0, 1.0, 1.0],
                region.source_width as f32,
                region.source_height as f32,
                region.source_x as f32,
                region.source_y as f32,
            );
        }
    }
    fn render_locator_bar(
        &self,
        renderer: &mut GuiRenderer,
        screen_w: f32,
        screen_h: f32,
        markers: &[LocatorBarMarker],
    ) {
        let left = ((screen_w as i32 - LOCATOR_BAR_WIDTH as i32) / 2) as f32;
        let top = contextual_bar_top(screen_h);
        let background = ResourceLocation::from_string("minecraft:hud/locator_bar_background");
        renderer.draw_sprite(
            left,
            top,
            LOCATOR_BAR_WIDTH,
            CONTEXTUAL_BAR_HEIGHT,
            &background,
            [1.0; 4],
        );
        let screen_middle = ((screen_w - LOCATOR_BAR_DOT_SIZE) / 2.0).ceil();
        let arrow_up = ResourceLocation::from_string("minecraft:hud/locator_bar_arrow_up");
        let arrow_down = ResourceLocation::from_string("minecraft:hud/locator_bar_arrow_down");
        for marker in markers {
            if marker.angle_degrees <= -LOCATOR_BAR_VISIBLE_DEGREES
                || marker.angle_degrees > LOCATOR_BAR_VISIBLE_DEGREES
            {
                continue;
            }
            let dot_position =
                (marker.angle_degrees * LOCATOR_BAR_DOT_RANGE / 2.0 / LOCATOR_BAR_VISIBLE_DEGREES)
                    .floor() as i32;
            let x = screen_middle + dot_position as f32;
            let sprite = self
                .locator_waypoint_styles
                .sprite(&marker.style, marker.distance);
            renderer.draw_sprite(
                x,
                top - 2.0,
                LOCATOR_BAR_DOT_SIZE,
                LOCATOR_BAR_DOT_SIZE,
                &sprite,
                argb_to_rgba(marker.color_argb),
            );
            match marker.pitch {
                crate::client::waypoint::LocatorPitchDirection::None => {}
                crate::client::waypoint::LocatorPitchDirection::Down => renderer.draw_sprite(
                    x + 1.0,
                    top + 6.0,
                    LOCATOR_BAR_ARROW_WIDTH,
                    LOCATOR_BAR_ARROW_HEIGHT,
                    &arrow_down,
                    [1.0; 4],
                ),
                crate::client::waypoint::LocatorPitchDirection::Up => renderer.draw_sprite(
                    x + 1.0,
                    top - 6.0,
                    LOCATOR_BAR_ARROW_WIDTH,
                    LOCATOR_BAR_ARROW_HEIGHT,
                    &arrow_up,
                    [1.0; 4],
                ),
            }
        }
    }
    fn render_experience_level(
        &self,
        renderer: &mut GuiRenderer,
        font: &FontRenderer,
        xp_level: i32,
        screen_w: f32,
        screen_h: f32,
    ) {
        let text = xp_level.to_string();
        let text_w = font.measure_width(&text, 1.0).ceil() as i32;
        let x = (((screen_w as i32) - text_w) / 2) as f32;
        let y = (screen_h as i32 - CONTEXTUAL_BAR_MARGIN_BOTTOM as i32 - 9 - 2) as f32;
        for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            renderer.draw_text_top(font, &text, x + dx, y + dy, [0.0, 0.0, 0.0, 1.0], 1.0);
        }
        renderer.draw_text_top(
            font,
            &text,
            x,
            y,
            [128.0 / 255.0, 1.0, 32.0 / 255.0, 1.0],
            1.0,
        );
    }
    fn render_player_status_bars(
        &self,
        renderer: &mut GuiRenderer,
        player: &LocalPlayer,
        screen_w: f32,
        screen_h: f32,
        hardcore: bool,
        blink: bool,
    ) {
        let center_x = (screen_w / 2.0).floor();
        let x_left = center_x - HOTBAR_HALF_WIDTH;
        let x_right = center_x + HOTBAR_HALF_WIDTH;
        let y_line_base = screen_h - PLAYER_BAR_BASELINE_FROM_BOTTOM;
        let current_health = player.health.ceil().max(0.0) as i32;
        let max_health = player
            .attr_f32(crate::gameplay::attributes::MAX_HEALTH)
            .max(current_health as f32)
            .max(self.display_health as f32);
        let absorption = player.absorption.ceil() as i32;
        let health_rows = health_row_count(max_health, absorption);
        let health_row_height = health_row_height(health_rows);
        let y_line_armor = y_line_base - (health_rows - 1) as f32 * health_row_height - 10.0;
        let armor = player.attr_f64(crate::gameplay::attributes::ARMOR).floor() as i32;
        if armor > 0 {
            self.render_armor(renderer, x_left, y_line_armor, armor);
        }
        let mut random = java_hud_random(self.tick_count);
        self.render_hearts(
            renderer,
            player,
            x_left,
            y_line_base,
            health_row_height,
            max_health,
            current_health,
            self.display_health,
            absorption,
            blink,
            hardcore,
            &mut random,
        );
        self.render_food(renderer, player, x_right, y_line_base, &mut random);
        let air_y = y_line_base - 10.0;
        self.render_air_bubbles(renderer, player, x_right, air_y, &mut random);
    }
    fn render_armor(&self, renderer: &mut GuiRenderer, x_left: f32, y: f32, armor: i32) {
        let armor_empty_loc = ResourceLocation::from_string("minecraft:hud/armor_empty");
        let armor_half_loc = ResourceLocation::from_string("minecraft:hud/armor_half");
        let armor_full_loc = ResourceLocation::from_string("minecraft:hud/armor_full");
        let armor = armor.clamp(0, 20);
        for index in 0..10 {
            let x = x_left + index as f32 * PLAYER_BAR_ICON_STEP;
            if index * 2 + 1 < armor {
                renderer.draw_sprite(
                    x,
                    y,
                    PLAYER_BAR_ICON_SIZE,
                    PLAYER_BAR_ICON_SIZE,
                    &armor_full_loc,
                    [1.0, 1.0, 1.0, 1.0],
                );
            } else if index * 2 + 1 == armor {
                renderer.draw_sprite(
                    x,
                    y,
                    PLAYER_BAR_ICON_SIZE,
                    PLAYER_BAR_ICON_SIZE,
                    &armor_half_loc,
                    [1.0, 1.0, 1.0, 1.0],
                );
            } else if index * 2 + 1 > armor {
                renderer.draw_sprite(
                    x,
                    y,
                    PLAYER_BAR_ICON_SIZE,
                    PLAYER_BAR_ICON_SIZE,
                    &armor_empty_loc,
                    [1.0, 1.0, 1.0, 1.0],
                );
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn render_hearts(
        &self,
        renderer: &mut GuiRenderer,
        player: &LocalPlayer,
        x_left: f32,
        y_line_base: f32,
        row_height: f32,
        max_health: f32,
        current_health: i32,
        display_health: i32,
        absorption: i32,
        blink: bool,
        hardcore: bool,
        random: &mut JavaRandom,
    ) {
        let container_loc = if hardcore {
            if blink {
                ResourceLocation::from_string("minecraft:hud/heart/container_hardcore_blinking")
            } else {
                ResourceLocation::from_string("minecraft:hud/heart/container_hardcore")
            }
        } else if blink {
            ResourceLocation::from_string("minecraft:hud/heart/container_blinking")
        } else {
            ResourceLocation::from_string("minecraft:hud/heart/container")
        };
        let health_type = HudHeartType::for_player(player);
        let full_loc = health_type.sprite(false, false, hardcore);
        let half_loc = health_type.sprite(true, false, hardcore);
        let blinking_full_loc = health_type.sprite(false, true, hardcore);
        let blinking_half_loc = health_type.sprite(true, true, hardcore);
        let health_containers = (max_health / 2.0).ceil().max(0.0) as i32;
        let absorption_containers = half_ceil(absorption).max(0);
        let total_containers = health_containers + absorption_containers;
        let heart_offset_index = regeneration_heart_offset_index(
            self.tick_count,
            max_health,
            player.has_effect(MOB_EFFECT_REGENERATION),
        );
        let low_health_shake = current_health + absorption <= 4;
        let absorption_type = if health_type == HudHeartType::Withered {
            HudHeartType::Withered
        } else {
            HudHeartType::Absorbing
        };
        let absorbing_full_loc = absorption_type.sprite(false, false, hardcore);
        let absorbing_half_loc = absorption_type.sprite(true, false, hardcore);
        let max_health_halves = health_containers * 2;
        for container_index in (0..total_containers).rev() {
            let row = container_index / 10;
            let column = container_index % 10;
            let x = x_left + column as f32 * PLAYER_BAR_ICON_STEP;
            let mut y = y_line_base - row as f32 * row_height;
            if low_health_shake {
                y += random.next_int_bound(2) as f32;
            }
            if container_index < health_containers && container_index == heart_offset_index {
                y -= 2.0;
            }
            renderer.draw_sprite(
                x,
                y,
                PLAYER_BAR_ICON_SIZE,
                PLAYER_BAR_ICON_SIZE,
                &container_loc,
                [1.0, 1.0, 1.0, 1.0],
            );
            if container_index >= health_containers {
                let absorption_halves = container_index * 2 - max_health_halves;
                if absorption_halves < absorption {
                    let loc = if absorption_halves + 1 == absorption {
                        &absorbing_half_loc
                    } else {
                        &absorbing_full_loc
                    };
                    renderer.draw_sprite(
                        x,
                        y,
                        PLAYER_BAR_ICON_SIZE,
                        PLAYER_BAR_ICON_SIZE,
                        loc,
                        [1.0, 1.0, 1.0, 1.0],
                    );
                }
            }
            let heart_min = container_index * 2 + 1;
            let heart_max = container_index * 2 + 2;
            if blink && heart_min <= display_health {
                let loc = if heart_max <= display_health {
                    &blinking_full_loc
                } else {
                    &blinking_half_loc
                };
                renderer.draw_sprite(
                    x,
                    y,
                    PLAYER_BAR_ICON_SIZE,
                    PLAYER_BAR_ICON_SIZE,
                    loc,
                    [1.0, 1.0, 1.0, 1.0],
                );
            }
            if heart_min <= current_health {
                let loc = if heart_max <= current_health {
                    &full_loc
                } else {
                    &half_loc
                };
                renderer.draw_sprite(
                    x,
                    y,
                    PLAYER_BAR_ICON_SIZE,
                    PLAYER_BAR_ICON_SIZE,
                    loc,
                    [1.0, 1.0, 1.0, 1.0],
                );
            }
        }
        let _ = player;
    }
    fn render_food(
        &self,
        renderer: &mut GuiRenderer,
        player: &LocalPlayer,
        x_right: f32,
        y_line_base: f32,
        random: &mut JavaRandom,
    ) {
        let (food_empty_loc, food_half_loc, food_full_loc) =
            food_sprite_locations(player.has_effect(MOB_EFFECT_HUNGER));
        let food = player.food.clamp(0, 20);
        let jitter_mod = (food * 3 + 1).max(1) as u32;
        for index in 0..10 {
            let x = food_icon_x(x_right, index);
            let mut y = y_line_base;
            if player.food_saturation <= 0.0 && self.tick_count % jitter_mod == 0 {
                y += random.next_int_bound(3) as f32 - 1.0;
            }
            renderer.draw_sprite(
                x,
                y,
                PLAYER_BAR_ICON_SIZE,
                PLAYER_BAR_ICON_SIZE,
                &food_empty_loc,
                [1.0, 1.0, 1.0, 1.0],
            );
            let food_min = index * 2 + 1;
            let food_max = index * 2 + 2;
            if food_min < food {
                renderer.draw_sprite(
                    x,
                    y,
                    PLAYER_BAR_ICON_SIZE,
                    PLAYER_BAR_ICON_SIZE,
                    &food_full_loc,
                    [1.0, 1.0, 1.0, 1.0],
                );
            } else if food_min == food || food_max == food {
                renderer.draw_sprite(
                    x,
                    y,
                    PLAYER_BAR_ICON_SIZE,
                    PLAYER_BAR_ICON_SIZE,
                    &food_half_loc,
                    [1.0, 1.0, 1.0, 1.0],
                );
            }
        }
    }
    fn render_air_bubbles(
        &self,
        renderer: &mut GuiRenderer,
        player: &LocalPlayer,
        x_right: f32,
        y: f32,
        random: &mut JavaRandom,
    ) {
        let current_air = player.air.clamp(0, MAX_AIR_SUPPLY);
        let is_underwater = player.is_under_water;
        if !is_underwater && current_air >= MAX_AIR_SUPPLY {
            return;
        }
        let air_loc = ResourceLocation::from_string("minecraft:hud/air");
        let bursting_loc = ResourceLocation::from_string("minecraft:hud/air_bursting");
        let air_empty_loc = ResourceLocation::from_string("minecraft:hud/air_empty");
        let full_bubbles = ((current_air - 2).max(0) * 10 + MAX_AIR_SUPPLY - 1) / MAX_AIR_SUPPLY;
        let popping_bubble = (current_air * 10 + MAX_AIR_SUPPLY - 1) / MAX_AIR_SUPPLY;
        let delay = if current_air != 0 && is_underwater {
            1
        } else {
            0
        };
        let empty_bubbles = 10 - ((current_air + delay) * 10 + MAX_AIR_SUPPLY - 1) / MAX_AIR_SUPPLY;
        for bubble in 1..=10 {
            let x = air_icon_x(x_right, bubble - 1);
            if bubble <= full_bubbles {
                renderer.draw_sprite(
                    x,
                    y,
                    PLAYER_BAR_ICON_SIZE,
                    PLAYER_BAR_ICON_SIZE,
                    &air_loc,
                    [1.0, 1.0, 1.0, 1.0],
                );
            } else if is_underwater && bubble == popping_bubble {
                renderer.draw_sprite(
                    x,
                    y,
                    PLAYER_BAR_ICON_SIZE,
                    PLAYER_BAR_ICON_SIZE,
                    &bursting_loc,
                    [1.0, 1.0, 1.0, 1.0],
                );
            } else if bubble > 10 - empty_bubbles {
                let mut bubble_y = y;
                if empty_bubbles == 10 && self.tick_count % 2 == 0 {
                    bubble_y += random.next_int_bound(2) as f32;
                }
                renderer.draw_sprite(
                    x,
                    bubble_y,
                    PLAYER_BAR_ICON_SIZE,
                    PLAYER_BAR_ICON_SIZE,
                    &air_empty_loc,
                    [1.0, 1.0, 1.0, 1.0],
                );
            }
        }
    }
    fn render_selected_item_name(
        &self,
        renderer: &mut GuiRenderer,
        font: &FontRenderer,
        screen_w: f32,
        screen_h: f32,
        can_hurt_player: bool,
        hud_background: HudBackgroundOptions,
    ) {
        if self.tool_highlight_timer == 0 {
            return;
        }
        let Some(item) = self
            .last_tool_highlight
            .as_ref()
            .filter(|item| !item.is_empty())
        else {
            return;
        };
        let name = selected_item_display_name(item);
        let alpha = (self.tool_highlight_timer as f32 * 256.0 / 10.0)
            .floor()
            .min(255.0)
            / 255.0;
        if alpha <= 0.0 {
            return;
        }
        let text_w = formatted_chat_width(font, &name, 1.0).ceil();
        let x = ((screen_w as i32 - text_w as i32) / 2) as f32;
        let y = selected_item_name_y(screen_h, can_hurt_player);
        if let Some(backdrop) = hud_background.text_with_backdrop_color(0.0, alpha) {
            renderer.draw_rect(x - 2.0, y - 2.0, text_w + 4.0, 13.0, backdrop);
        }
        draw_formatted_chat_text_ex(renderer, font, &name, x, y, alpha, 1.0, true);
    }
    fn render_overlay_message(
        &self,
        renderer: &mut GuiRenderer,
        font: &FontRenderer,
        screen_w: f32,
        screen_h: f32,
        hud_background: HudBackgroundOptions,
    ) {
        let Some(message) = self
            .overlay_message
            .as_ref()
            .filter(|_| self.overlay_message_time > 0)
        else {
            return;
        };
        let t = self.overlay_message_time as f32;
        let alpha = (t * 255.0 / 20.0).floor().min(255.0) / 255.0;
        if alpha <= 0.0 {
            return;
        }
        let width = formatted_chat_width(font, message, 1.0).ceil();
        let x = ((screen_w as i32 / 2) - (width as i32 / 2)) as f32;
        let y = screen_h - 72.0;
        if let Some(backdrop) = hud_background.text_with_backdrop_color(0.0, alpha) {
            renderer.draw_rect(x - 2.0, y - 2.0, width + 4.0, 13.0, backdrop);
        }
        if self.overlay_message_animate_color {
            let color = hsv_to_argb(t / 50.0, 0.7, 0.6, alpha);
            draw_formatted_chat_text_override_color(
                renderer, font, message, x, y, 1.0, color, true,
            );
        } else {
            draw_formatted_chat_text(renderer, font, message, x, y, alpha, 1.0);
        }
    }
    fn render_debug_overlay(
        &self,
        renderer: &mut GuiRenderer,
        font: &FontRenderer,
        player: &LocalPlayer,
        fps: u32,
    ) {
        renderer.draw_rect(2.0, 2.0, 250.0, 110.0, [0.0, 0.0, 0.0, 0.5]);
        let lines = vec![
            format!(
                "Eltharn ({})",
                el_launcher_core::ELTHARN_CLIENT_ASSETS_COMPATIBILITY
            ),
            format!("FPS: {}", fps),
            format!(
                "Pos: {:.3} / {:.3} / {:.3}",
                player.position.x, player.position.y, player.position.z
            ),
            format!(
                "Chunk: {} / {} in [0..0]",
                (player.position.x / 16.0).floor() as i32,
                (player.position.z / 16.0).floor() as i32
            ),
            format!(
                "Yaw: {:.1} / Pitch: {:.1}",
                player.yaw.to_degrees(),
                player.pitch.to_degrees()
            ),
            format!("Target Block: {:?}", player.target_block),
        ];
        for (idx, line) in lines.iter().enumerate() {
            renderer.draw_text_top(
                font,
                line,
                5.0,
                12.0 + (idx as f32 * 11.0),
                [1.0, 1.0, 1.0, 1.0],
                1.0,
            );
        }
    }
}
pub fn should_render_crosshair(camera_type: CameraType) -> bool {
    camera_type.is_first_person()
}
pub fn hotbar_item_y(screen_h: f32) -> f32 {
    screen_h - HOTBAR_ITEM_Y_OFFSET
}
pub fn food_icon_x(x_right: f32, index: i32) -> f32 {
    x_right - index as f32 * PLAYER_BAR_ICON_STEP - PLAYER_BAR_ICON_SIZE
}
pub fn air_icon_x(x_right: f32, bubble_index: i32) -> f32 {
    x_right - bubble_index as f32 * PLAYER_BAR_ICON_STEP - PLAYER_BAR_ICON_SIZE
}
pub fn selected_item_name_y(screen_h: f32, can_hurt_player: bool) -> f32 {
    let mut y = screen_h - 59.0;
    if !can_hurt_player {
        y += 14.0;
    }
    y
}
pub fn contextual_bar_top(screen_h: f32) -> f32 {
    screen_h - CONTEXTUAL_BAR_MARGIN_BOTTOM - CONTEXTUAL_BAR_HEIGHT
}
fn experience_bar_progress_region(experience_progress: f32) -> Option<HudSpriteRegion> {
    let progress = (experience_progress * 183.0) as i32;
    (progress > 0).then_some(HudSpriteRegion {
        source_width: 182,
        source_height: 5,
        source_x: 0,
        source_y: 0,
        width: progress,
        height: 5,
    })
}
fn crosshair_attack_indicator_progress_region(
    attack_strength_scale: f32,
) -> Option<HudSpriteRegion> {
    let progress = (attack_strength_scale * 17.0) as i32;
    (progress > 0).then_some(HudSpriteRegion {
        source_width: 16,
        source_height: 4,
        source_x: 0,
        source_y: 0,
        width: progress,
        height: 4,
    })
}
fn hotbar_attack_indicator_progress_region(attack_strength_scale: f32) -> Option<HudSpriteRegion> {
    let progress = (attack_strength_scale * 19.0) as i32;
    (progress > 0).then_some(HudSpriteRegion {
        source_width: 18,
        source_height: 18,
        source_x: 0,
        source_y: 18 - progress,
        width: 18,
        height: progress,
    })
}
pub fn selected_item_name(item: &ItemStack) -> String {
    if let Some(DataComponent::CustomName(name)) = item.components.get("minecraft:custom_name") {
        return strip_json_text_component_quotes(name);
    }
    if let Some(DataComponent::Generic { value, .. }) = item.components.get("minecraft:custom_name")
        && let Some(name) = formatted_text_from_tag(value)
        && !name.is_empty()
    {
        return name;
    }
    if let Some(DataComponent::Generic {
        value: Tag::String(name),
        ..
    }) = item.components.get("minecraft:item_name")
    {
        let name = strip_json_text_component_quotes(name);
        if !name.is_empty() {
            return name;
        }
    }
    if let Some(DataComponent::Generic { value, .. }) = item.components.get("minecraft:item_name")
        && let Some(json) = tag_to_json(value)
    {
        let name = flatten_text_component(&json);
        if !name.is_empty() {
            return name;
        }
    }
    let key = format!(
        "item.{}.{}",
        item.item.location.namespace, item.item.location.path
    );
    let translated = crate::ui::i18n::translate(&key);
    if translated != key {
        return translated.to_string();
    }
    let block_key = format!(
        "block.{}.{}",
        item.item.location.namespace, item.item.location.path
    );
    let translated_block = crate::ui::i18n::translate(&block_key);
    if translated_block != block_key {
        return translated_block.to_string();
    }
    item.item.location.path.replace('_', " ")
}
fn selected_item_display_name(item: &ItemStack) -> String {
    if let Some(styled) = crate::ui::gui::screen::container::item_name_styled_ansi(item) {
        return styled;
    }
    let name = selected_item_name(item);
    let color = item_name_color(item);
    let mut styled = format!(
        "\u{1b}[38;2;{};{};{}m",
        (color >> 16) & 0xff,
        (color >> 8) & 0xff,
        color & 0xff
    );
    if has_custom_name(item) {
        styled.push_str("\u{1b}[3m");
    }
    styled.push_str(&name);
    styled.push_str("\u{1b}[m");
    styled
}
pub(crate) fn item_name_color(item: &ItemStack) -> u32 {
    if let Some(component) = item
        .components
        .get("minecraft:custom_name")
        .or_else(|| item.components.get("minecraft:item_name"))
    {
        let value = match component {
            DataComponent::CustomName(raw) => serde_json::from_str::<serde_json::Value>(raw).ok(),
            DataComponent::Generic { value, .. } => tag_to_json(value),
            _ => None,
        };
        if let Some(value) = value.and_then(|value| root_text_color(&value)) {
            return value;
        }
    }
    if let Some(DataComponent::Generic { value, .. }) = item.components.get("minecraft:rarity") {
        if let Tag::String(rarity) = value {
            return match rarity.strip_prefix("minecraft:").unwrap_or(rarity) {
                "uncommon" => 0xFFFF55,
                "rare" => 0x55FFFF,
                "epic" => 0xFF55FF,
                _ => 0xFFFFFF,
            };
        }
    }
    0xFFFFFF
}
fn root_text_color(value: &serde_json::Value) -> Option<u32> {
    let object = match value {
        serde_json::Value::String(raw) => serde_json::from_str(raw).ok()?,
        serde_json::Value::Object(object) => serde_json::Value::Object(object.clone()),
        _ => return None,
    };
    let color = object.get("color")?;
    if let Some(hex) = color.as_str().and_then(|s| s.strip_prefix('#')) {
        return u32::from_str_radix(hex, 16).ok();
    }
    let id = match color.as_str()? {
        "black" => 0,
        "dark_blue" => 1,
        "dark_green" => 2,
        "dark_aqua" => 3,
        "dark_red" => 4,
        "dark_purple" => 5,
        "gold" => 6,
        "gray" => 7,
        "dark_gray" => 8,
        "blue" => 9,
        "green" => 10,
        "aqua" => 11,
        "red" => 12,
        "light_purple" => 13,
        "yellow" => 14,
        "white" => 15,
        _ => return None,
    };
    crate::ui::scoreboard::chat_color_rgb(id)
}
fn screen_h_from_hotbar_y(hotbar_y: f32) -> f32 {
    hotbar_y + HOTBAR_HEIGHT
}
fn half_ceil(value: i32) -> i32 {
    if value >= 0 {
        ((i64::from(value) + 1) / 2) as i32
    } else {
        (i64::from(value) / 2) as i32
    }
}
fn food_sprite_locations(
    hunger_effect: bool,
) -> (ResourceLocation, ResourceLocation, ResourceLocation) {
    let suffix = if hunger_effect { "_hunger" } else { "" };
    (
        ResourceLocation::from_string(&format!("minecraft:hud/food_empty{suffix}")),
        ResourceLocation::from_string(&format!("minecraft:hud/food_half{suffix}")),
        ResourceLocation::from_string(&format!("minecraft:hud/food_full{suffix}")),
    )
}
fn health_row_count(max_health: f32, absorption: i32) -> i32 {
    ((max_health + absorption as f32) / 20.0).ceil().max(1.0) as i32
}
fn regeneration_heart_offset_index(tick: u32, max_health: f32, active: bool) -> i32 {
    if active {
        (tick % (max_health + 5.0).ceil().max(1.0) as u32) as i32
    } else {
        -1
    }
}
fn xp_needed_for_next_level(level: i32) -> i32 {
    if level >= 30 {
        112 + (level - 30) * 9
    } else if level >= 15 {
        37 + (level - 15) * 5
    } else {
        7 + level * 2
    }
}
fn health_row_height(rows: i32) -> f32 {
    (10 - (rows - 2)).max(3) as f32
}
fn has_custom_name(item: &ItemStack) -> bool {
    item.components.get("minecraft:custom_name").is_some()
}
fn java_hud_random(tick_count: u32) -> JavaRandom {
    let seed = (tick_count as i32).wrapping_mul(312_871) as i64;
    JavaRandom::new(seed as u64)
}
fn strip_json_text_component_quotes(name: &str) -> String {
    let trimmed = name.trim();
    let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return trimmed.trim_matches('"').to_string();
    };
    flatten_text_component(&value)
}
fn flatten_text_component(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if (trimmed.starts_with('{') || trimmed.starts_with('['))
                && let Ok(nested) = serde_json::from_str::<serde_json::Value>(trimmed)
                && matches!(
                    nested,
                    serde_json::Value::Object(_) | serde_json::Value::Array(_)
                )
            {
                return flatten_text_component(&nested);
            }
            text.clone()
        }
        serde_json::Value::Array(values) => values
            .iter()
            .map(flatten_text_component)
            .collect::<Vec<_>>()
            .join(""),
        serde_json::Value::Object(object) => {
            let mut text = object
                .get("text")
                .map(flatten_text_component)
                .unwrap_or_default();
            if let Some(translate) = object.get("translate").and_then(|v| v.as_str()) {
                text = crate::ui::i18n::translate(translate).to_string();
            }
            if let Some(extra) = object.get("extra") {
                text.push_str(&flatten_text_component(extra));
            }
            if text.is_empty() {
                if let Some(with) = object.get("with") {
                    text.push_str(&flatten_text_component(with));
                }
            }
            text
        }
        _ => String::new(),
    }
}
fn tag_to_json(tag: &Tag) -> Option<serde_json::Value> {
    match tag {
        Tag::End => Some(serde_json::Value::Null),
        Tag::Byte(v) => Some(serde_json::json!(*v)),
        Tag::Short(v) => Some(serde_json::json!(*v)),
        Tag::Int(v) => Some(serde_json::json!(*v)),
        Tag::Long(v) => Some(serde_json::json!(*v)),
        Tag::Float(v) => Some(serde_json::json!(*v)),
        Tag::Double(v) => Some(serde_json::json!(*v)),
        Tag::String(v) => Some(serde_json::Value::String(v.clone())),
        Tag::ByteArray(values) => Some(serde_json::Value::Array(
            values.iter().map(|v| serde_json::json!(*v)).collect(),
        )),
        Tag::IntArray(values) => Some(serde_json::Value::Array(
            values.iter().map(|v| serde_json::json!(*v)).collect(),
        )),
        Tag::LongArray(values) => Some(serde_json::Value::Array(
            values.iter().map(|v| serde_json::json!(*v)).collect(),
        )),
        Tag::List(values) => Some(serde_json::Value::Array(
            values.iter().filter_map(tag_to_json).collect(),
        )),
        Tag::Compound(fields) => Some(serde_json::Value::Object(
            fields
                .iter()
                .filter_map(|(key, value)| Some((key.clone(), tag_to_json(value)?)))
                .collect(),
        )),
    }
}
fn formatted_text_from_tag(tag: &Tag) -> Option<String> {
    match tag {
        Tag::String(raw) => Some(strip_json_text_component_quotes(raw)),
        Tag::Compound(_) | Tag::List(_) => {
            let json = tag_to_json(tag)?;
            Some(flatten_text_component(&json))
        }
        _ => None,
    }
}
fn boss_bar_color_sprite(color: BossBarColor) -> (&'static str, &'static str) {
    use BossBarColor::*;
    match color {
        Pink => (
            "minecraft:boss_bar/pink_background",
            "minecraft:boss_bar/pink_progress",
        ),
        Blue => (
            "minecraft:boss_bar/blue_background",
            "minecraft:boss_bar/blue_progress",
        ),
        Red => (
            "minecraft:boss_bar/red_background",
            "minecraft:boss_bar/red_progress",
        ),
        Green => (
            "minecraft:boss_bar/green_background",
            "minecraft:boss_bar/green_progress",
        ),
        Yellow => (
            "minecraft:boss_bar/yellow_background",
            "minecraft:boss_bar/yellow_progress",
        ),
        Purple => (
            "minecraft:boss_bar/purple_background",
            "minecraft:boss_bar/purple_progress",
        ),
        White => (
            "minecraft:boss_bar/white_background",
            "minecraft:boss_bar/white_progress",
        ),
    }
}
fn boss_bar_overlay_sprite(overlay: BossBarOverlay) -> Option<(&'static str, &'static str)> {
    use BossBarOverlay::*;
    match overlay {
        Progress => None,
        Notched6 => Some((
            "minecraft:boss_bar/notched_6_background",
            "minecraft:boss_bar/notched_6_progress",
        )),
        Notched10 => Some((
            "minecraft:boss_bar/notched_10_background",
            "minecraft:boss_bar/notched_10_progress",
        )),
        Notched12 => Some((
            "minecraft:boss_bar/notched_12_background",
            "minecraft:boss_bar/notched_12_progress",
        )),
        Notched20 => Some((
            "minecraft:boss_bar/notched_20_background",
            "minecraft:boss_bar/notched_20_progress",
        )),
    }
}
impl HudRenderer {
    fn render_boss_bars(
        &self,
        renderer: &mut GuiRenderer,
        font: &FontRenderer,
        screen_w: f32,
        screen_h: f32,
    ) {
        if self.boss_events.is_empty() {
            return;
        }
        let mut y = 12.0;
        for event in self.boss_events.values() {
            let x = ((screen_w - 182.0) / 2.0).floor();
            let (bg_str, fg_str) = boss_bar_color_sprite(event.color.clone());
            let bg_loc = ResourceLocation::from_string(bg_str);
            let fg_loc = ResourceLocation::from_string(fg_str);
            renderer.draw_sprite(x, y, 182.0, 5.0, &bg_loc, [1.0, 1.0, 1.0, 1.0]);
            let width = lerp_discrete(event.progress.clamp(0.0, 1.0), 0, 182);
            if width > 0 {
                renderer.draw_sprite_region(
                    x,
                    y,
                    width as f32,
                    5.0,
                    &fg_loc,
                    [1.0, 1.0, 1.0, 1.0],
                    182.0,
                    5.0,
                    0.0,
                    0.0,
                );
            }
            if let Some((bg_ol_str, fg_ol_str)) = boss_bar_overlay_sprite(event.overlay.clone()) {
                let bg_ol_loc = ResourceLocation::from_string(bg_ol_str);
                let fg_ol_loc = ResourceLocation::from_string(fg_ol_str);
                renderer.draw_sprite(x, y, 182.0, 5.0, &bg_ol_loc, [1.0, 1.0, 1.0, 1.0]);
                if width > 0 {
                    renderer.draw_sprite_region(
                        x,
                        y,
                        width as f32,
                        5.0,
                        &fg_ol_loc,
                        [1.0, 1.0, 1.0, 1.0],
                        182.0,
                        5.0,
                        0.0,
                        0.0,
                    );
                }
            }
            let name_w = formatted_chat_width(font, &event.name, 1.0);
            let name_x = ((screen_w - name_w) / 2.0).floor();
            let name_y = y - 9.0;
            draw_formatted_chat_text(renderer, font, &event.name, name_x, name_y, 1.0, 1.0);
            y += 10.0 + 9.0;
            if y >= screen_h / 3.0 {
                break;
            }
        }
    }
}
