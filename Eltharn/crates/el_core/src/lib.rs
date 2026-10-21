pub mod biome;
pub mod block;
pub mod block_behaviour;
pub mod block_state;
pub mod chest;
pub mod collision_context;
pub mod entity;
pub mod fluid_state;
pub mod item;
pub mod registry;
pub mod resource_location;
pub use biome::Biome;
pub use block::Block;
pub use block_state::BlockState;
pub use collision_context::{CollisionContext, CollisionEntityKind};
pub use entity::{Entity, EntityDataValue, EntityKind, EntityType, SynchedEntityData};
pub use fluid_state::{FluidKind, FluidState};
pub use item::{ConsumableProperties, DataComponent, FoodProperties, Item, ItemStack};
pub use registry::{
    BlockStateRegistry, BuiltInRegistries, DefaultedRegistry, Holder, HolderSet, MappedRegistry,
    ReferenceHolder, Registry, RegistryAccess, ResourceKey, TagKey, TagLoader,
};
pub use resource_location::ResourceLocation;
use std::cell::RefCell;
#[derive(Clone, Debug)]
pub struct ClientHudState {
    pub width: i32,
    pub height: i32,
    pub health: f32,
    pub food: i32,
    pub armor: i32,
    pub air: i32,
    pub xp_level: i32,
    pub xp_progress: f32,
    pub active_slot: usize,
    pub fps: u32,
    pub show_debug: bool,
    pub chat_messages: Vec<String>,
    pub inventory: Vec<ItemStack>,
    pub is_paused: bool,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub target_block: String,
    pub gui_scale: u32,
}
thread_local! {
    pub static CLIENT_HUD_STATE : RefCell < Option < ClientHudState >> =
    RefCell::new(None);
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UuidStub;
pub type Uuid = uuid::Uuid;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormattedCharSequence {
    pub text: String,
}
pub trait FormattedCharSink {
    fn accept(&mut self, position: i32, style: &Style, codepoint: i32) -> bool;
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Style;
impl Style {
    pub const EMPTY: Style = Style;
    pub fn is_bold(&self) -> bool {
        false
    }
    pub fn is_italic(&self) -> bool {
        false
    }
    pub fn is_underlined(&self) -> bool {
        false
    }
    pub fn is_strikethrough(&self) -> bool {
        false
    }
    pub fn is_obfuscated(&self) -> bool {
        false
    }
    pub fn get_font(&self) -> &FontDescription {
        &FontDescription
    }
    pub fn get_color(&self) -> Option<&TextColor> {
        None
    }
    pub fn get_click_event(&self) -> Option<&click::ClickEvent> {
        None
    }
    pub fn get_insertion(&self) -> Option<&str> {
        None
    }
    pub fn get_shadow_color(&self) -> Option<i32> {
        None
    }
}
pub struct FontDescription;
pub struct TextColor;
impl TextColor {
    pub fn get_value(&self) -> i32 {
        0
    }
}
pub mod click {
    pub struct ClickEvent;
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Component {
    pub text: String,
}
impl Component {
    pub fn literal(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }
    pub fn translatable(key: &str) -> Self {
        Self {
            text: key.to_string(),
        }
    }
    pub fn get_visual_order_text(&self) -> FormattedCharSequence {
        FormattedCharSequence {
            text: self.text.clone(),
        }
    }
    pub fn get_string(&self) -> String {
        self.text.clone()
    }
}
pub struct ARGB;
impl ARGB {
    pub fn white(_opacity: f32) -> i32 {
        0xFFFFFF
    }
    pub fn alpha(_color: i32) -> i32 {
        255
    }
    pub fn alpha_float(_color: i32) -> f32 {
        1.0
    }
    pub fn color(_alpha: i32, _rgb: i32) -> i32 {
        0
    }
    pub fn scale_rgb(_color: i32, _scale: f32) -> i32 {
        0
    }
    pub fn as_8bit_channel(_val: f32) -> i32 {
        0
    }
    pub fn set_brightness(color: i32, _brightness: f32) -> i32 {
        color
    }
}
pub struct Mth;
impl Mth {
    pub fn lerp(_alpha: f64, _start: f64, _end: f64) -> f64 {
        0.0
    }
    pub fn clamp(val: i32, min: i32, max: i32) -> i32 {
        val.clamp(min, max)
    }
    pub fn floor(val: f32) -> i32 {
        val.floor() as i32
    }
    pub fn ceil(val: f32) -> i32 {
        val.ceil() as i32
    }
    pub fn lerp_discrete(pct: f32, start: i32, end: i32) -> i32 {
        start + ((end - start) as f32 * pct).round() as i32
    }
    pub fn sqrt(val: f32) -> f32 {
        val.sqrt()
    }
}
pub struct Util;
impl Util {
    pub fn get_millis() -> u64 {
        0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemTag;
pub mod item_stubs {
    pub struct BundleItem;
    impl BundleItem {
        pub fn get_number_of_items_to_show(_stack: &crate::ItemStack) -> i32 {
            0
        }
        pub fn get_selected_item_index(_stack: &crate::ItemStack) -> i32 {
            0
        }
        pub fn toggle_selected_item(_stack: &crate::ItemStack, _index: i32) {}
    }
}
pub mod inventory {
    pub struct Slot {
        pub index: i32,
    }
    impl Slot {
        pub fn get_item(&self) -> &crate::ItemStack {
            static EMPTY_STACK: std::sync::LazyLock<crate::ItemStack> =
                std::sync::LazyLock::new(|| crate::ItemStack::empty());
            &EMPTY_STACK
        }
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ContainerInput {
        QuickMove,
        Swap,
        Generic,
    }
}
pub struct ItemTags;
impl ItemTags {
    pub const BUNDLES: ItemTag = ItemTag;
}
#[derive(Clone, Copy)]
pub struct Minecraft;
impl Minecraft {
    pub fn get_instance() -> Self {
        Self
    }
    pub fn is_paused(&self) -> bool {
        CLIENT_HUD_STATE.with(|s| s.borrow().as_ref().map(|s| s.is_paused).unwrap_or(false))
    }
    pub fn get_connection(&self) -> Option<&'static ClientPacketListener> {
        static CONN: ClientPacketListener = ClientPacketListener;
        Some(&CONN)
    }
    pub fn level(&self) -> &Level {
        static LEVEL: std::sync::LazyLock<Level> = std::sync::LazyLock::new(|| Level);
        &LEVEL
    }
    pub fn get_window(&self) -> &Window {
        static WINDOW: Window = Window;
        &WINDOW
    }
    pub fn player(&self) -> &LocalPlayer {
        static PLAYER: LocalPlayer = LocalPlayer {
            connection: ClientPacketListener,
            experience_progress: 0.0,
        };
        &PLAYER
    }
    pub fn get_camera_entity(&self) -> Option<&entity::Entity> {
        None
    }
    pub fn game_renderer(&self) -> &GameRenderer {
        static GAME_RENDERER: GameRenderer = GameRenderer {
            main_camera: Camera,
        };
        &GAME_RENDERER
    }
    pub fn gui(&self) -> &GuiStub {
        static GUI: GuiStub = GuiStub { hud: HudStub };
        &GUI
    }
}
pub struct ClientPacketListener;
impl ClientPacketListener {
    pub fn send(&self, _packet: &ServerboundSelectBundleItemPacket) {}
    pub fn get_waypoint_manager(&self) -> &WaypointManager {
        static WAYPOINT_MANAGER: WaypointManager = WaypointManager;
        &WAYPOINT_MANAGER
    }
    pub fn send_teleport(&self, _packet: ServerboundTeleportToEntityPacket) {}
    pub fn get_listed_online_players(&self) -> Vec<PlayerInfo> {
        Vec::new()
    }
}
pub struct ServerboundSelectBundleItemPacket;
impl ServerboundSelectBundleItemPacket {
    pub fn new(_slot_index: i32, _selected_item: i32) -> Self {
        Self
    }
}
pub struct ScrollWheelHandler;
impl ScrollWheelHandler {
    pub fn new() -> Self {
        Self
    }
    pub fn on_mouse_scroll(&self, _scroll_x: f64, _scroll_y: f64) -> el_math::Vector2i {
        el_math::Vector2i::default()
    }
    pub fn get_next_scroll_wheel_selection(_wheel: i32, _selected: i32, _count: i32) -> i32 {
        0
    }
}
pub struct DeltaTracker;
impl DeltaTracker {
    pub fn get_game_time_delta_partial_tick(&self, _frozen: bool) -> f32 {
        0.0
    }
}
pub struct Window;
impl Window {
    pub fn get_gui_scaled_width(&self) -> i32 {
        CLIENT_HUD_STATE.with(|s| s.borrow().as_ref().map(|s| s.width).unwrap_or(800))
    }
    pub fn get_gui_scaled_height(&self) -> i32 {
        CLIENT_HUD_STATE.with(|s| s.borrow().as_ref().map(|s| s.height).unwrap_or(600))
    }
}
pub struct LocalPlayer {
    pub connection: ClientPacketListener,
    pub experience_progress: f32,
}
impl LocalPlayer {
    pub fn get_xp_needed_for_next_level(&self) -> i32 {
        0
    }
    pub fn jumpable_vehicle(&self) -> Option<PlayerRideableJumping> {
        None
    }
    pub fn get_jump_riding_scale(&self) -> f32 {
        0.0
    }
}
pub struct PlayerRideableJumping;
impl PlayerRideableJumping {
    pub fn get_jump_cooldown(&self) -> i32 {
        0
    }
}
pub struct Camera;
pub struct GameRenderer {
    pub main_camera: Camera,
}
pub struct WaypointId;
impl WaypointId {
    pub fn left(&self) -> Option<Uuid> {
        None
    }
    pub fn map<F1, F2, R>(&self, _f1: F1, _f2: F2) -> Option<R>
    where
        F1: FnOnce(Uuid) -> R,
        F2: FnOnce(&str) -> R,
    {
        None
    }
}
pub struct WaypointIcon {
    pub style: String,
    pub color: Option<i32>,
}
pub struct Waypoint {
    pub id: WaypointId,
    pub icon: WaypointIcon,
}
impl Waypoint {
    pub fn id(&self) -> &WaypointId {
        &self.id
    }
    pub fn icon(&self) -> &WaypointIcon {
        &self.icon
    }
    pub fn distance_squared(&self, _entity: &entity::Entity) -> f64 {
        0.0
    }
    pub fn yaw_angle_to_camera(
        &self,
        _level: &Level,
        _camera: &Camera,
        _supplier: &dyn Fn(&entity::Entity) -> f32,
    ) -> f64 {
        0.0
    }
    pub fn pitch_direction_to_camera(
        &self,
        _level: &Level,
        _renderer: &GameRenderer,
        _supplier: &dyn Fn(&entity::Entity) -> f32,
    ) -> TrackedWaypointPitchDirection {
        TrackedWaypointPitchDirection::None
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackedWaypointPitchDirection {
    None,
    Up,
    Down,
}
pub struct WaypointStyle;
impl WaypointStyle {
    pub fn sprite(&self, _distance: f32) -> &Identifier {
        static SPRITE: std::sync::LazyLock<Identifier> =
            std::sync::LazyLock::new(|| Identifier::from_string("minecraft:missing"));
        &SPRITE
    }
}
pub struct WaypointManager;
impl WaypointManager {
    pub fn for_each_waypoint<F>(&self, _camera: &entity::Entity, mut _f: F)
    where
        F: FnMut(&Waypoint),
    {
    }
}
pub struct IdentifierStub;
pub type Identifier = ResourceLocation;
pub struct RenderPipelines;
impl RenderPipelines {
    pub const GUI_TEXTURED: i32 = 0;
}
pub struct GuiStub {
    pub hud: HudStub,
}
pub struct HudStub;
impl HudStub {
    pub fn get_waypoint_styles(&self) -> &std::collections::HashMap<String, WaypointStyle> {
        static STYLES: std::sync::LazyLock<std::collections::HashMap<String, WaypointStyle>> =
            std::sync::LazyLock::new(|| std::collections::HashMap::new());
        &STYLES
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UUID {
    pub most_sig: u64,
    pub least_sig: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameType {
    Survival,
    Creative,
    Adventure,
    Spectator,
}
#[derive(Clone, Debug, Default)]
pub struct PlayerSkin;
#[derive(Clone, Debug)]
pub struct GameProfile {
    id: UUID,
    name: String,
}
impl GameProfile {
    pub fn new(id: UUID, name: String) -> Self {
        Self { id, name }
    }
    pub fn id(&self) -> UUID {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}
#[derive(Clone, Debug)]
pub struct PlayerInfo {
    profile: GameProfile,
    game_mode: GameType,
    skin: PlayerSkin,
}
impl PlayerInfo {
    pub fn new(profile: GameProfile, game_mode: GameType, skin: PlayerSkin) -> Self {
        Self {
            profile,
            game_mode,
            skin,
        }
    }
    pub fn get_profile(&self) -> &GameProfile {
        &self.profile
    }
    pub fn get_game_mode(&self) -> GameType {
        self.game_mode
    }
    pub fn get_skin(&self) -> &PlayerSkin {
        &self.skin
    }
}
pub struct Scoreboard {
    pub teams: Vec<PlayerTeam>,
}
impl Scoreboard {
    pub fn get_player_teams(&self) -> &[PlayerTeam] {
        &self.teams
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TeamColor {
    rgb: i32,
}
impl TeamColor {
    pub fn new(rgb: i32) -> Self {
        Self { rgb }
    }
    pub fn rgb(&self) -> i32 {
        self.rgb
    }
}
#[derive(Clone, Debug)]
pub struct PlayerTeam {
    pub name: String,
    pub display_name: Component,
    pub color: Option<TeamColor>,
    pub players: Vec<String>,
}
impl PlayerTeam {
    pub fn get_players(&self) -> &[String] {
        &self.players
    }
    pub fn get_display_name(&self) -> &Component {
        &self.display_name
    }
    pub fn get_color(&self) -> Option<TeamColor> {
        self.color
    }
}
pub struct RandomSource;
impl RandomSource {
    pub fn create_thread_local_instance() -> Self {
        Self
    }
    pub fn next_int(&self, _bound: i32) -> i32 {
        0
    }
}
pub struct ServerboundTeleportToEntityPacket {
    pub target_uuid: UUID,
}
impl ServerboundTeleportToEntityPacket {
    pub fn new(target_uuid: UUID) -> Self {
        Self { target_uuid }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DyeColor {
    White,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    Black,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BannerPatternLayers;
#[derive(Clone, Debug, Default)]
pub struct ResultField;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorType {
    Default,
    PointingHand,
    Text,
    Crosshair,
    ResizeElW,
    ResizeNs,
    ResizeNesw,
    ResizeNwse,
    ResizeAll,
    NotAllowed,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Quaternionf {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}
pub struct TickRateManager;
impl TickRateManager {
    pub fn is_entity_frozen(&self, _entity: &entity::Entity) -> bool {
        false
    }
}
pub struct Level;
impl Level {
    pub fn tick_rate_manager(&self) -> &TickRateManager {
        static TRM: TickRateManager = TickRateManager;
        &TRM
    }
    pub fn get_scoreboard(&self) -> &Scoreboard {
        static SCOREBOARD: std::sync::LazyLock<Scoreboard> =
            std::sync::LazyLock::new(|| Scoreboard { teams: Vec::new() });
        &SCOREBOARD
    }
}
pub struct LevelChunk;
impl entity::Entity {
    pub fn get_uuid(&self) -> Uuid {
        self.uuid
    }
    pub fn level(&self) -> &Level {
        static LEVEL: Level = Level;
        &LEVEL
    }
}
