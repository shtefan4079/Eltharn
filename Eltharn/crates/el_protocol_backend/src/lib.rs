use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use uuid::Uuid;
pub mod diagnostics;
pub const ELTHARN_PROTOCOL_BACKEND_IPC_VERSION: u32 = 37;
pub mod payload;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendMapPatch {
    pub start_x: u8,
    pub start_y: u8,
    pub width: u8,
    pub height: u8,
    pub colors: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendMapDecoration {
    pub kind: String,
    pub x: i8,
    pub y: i8,
    pub rot: u8,
    pub name: Option<String>,
    pub render_on_frame: bool,
}
impl BackendMapDecoration {
    pub fn from_vanilla_type(kind: &str, x: i8, y: i8, rot: u8, name: Option<String>) -> Self {
        let path = kind.strip_prefix("minecraft:").unwrap_or(kind);
        let asset = match path {
            "mansion" => "woodland_mansion".to_owned(),
            "monument" => "ocean_monument".to_owned(),
            name if name.starts_with("banner_") => format!("{}_banner", &name[7..]),
            name if name.starts_with("village_") => format!("{}_village", &name[8..]),
            name => name.to_owned(),
        };
        Self {
            kind: format!("minecraft:{asset}"),
            x,
            y,
            rot: rot & 15,
            name,
            render_on_frame: !matches!(
                path,
                "player" | "red_marker" | "blue_marker" | "player_off_map" | "player_off_limits"
            ),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendStatValue {
    pub category: String,
    pub value_id: String,
    pub value: i32,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BackendAwardStats {
    pub stats: Vec<BackendStatValue>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendAdvancementDefinition {
    pub id: String,
    pub parent_id: Option<String>,
    pub display: Option<BackendAdvancementDisplay>,
    pub requirements: Vec<Vec<String>>,
    pub sends_telemetry_event: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendAdvancementDisplay {
    pub title: serde_json::Value,
    pub description: serde_json::Value,
    pub icon: BackendAdvancementIcon,
    pub frame: String,
    pub show_toast: bool,
    pub hidden: bool,
    pub background: Option<String>,
    pub x: f32,
    pub y: f32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendAdvancementIcon {
    pub item_id: String,
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<serde_json::Value>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendAdvancementCriterionProgress {
    pub criterion: String,
    pub acquired: bool,
    pub acquired_at_epoch_millis: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendAdvancementProgress {
    pub id: String,
    pub criteria: Vec<BackendAdvancementCriterionProgress>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BackendUpdateAdvancements {
    pub reset: bool,
    pub added: Vec<BackendAdvancementDefinition>,
    pub removed: Vec<String>,
    pub progress: Vec<BackendAdvancementProgress>,
    pub show_advancements: Option<bool>,
}
pub const DEFAULT_BACKEND_EXECUTABLE_NAME: &str = "eltharn-protocol";
pub const VANILLA_VERSION_NAME: &str = "1.21.11";
pub const VANILLA_PROTOCOL_VERSION: i32 = 774;
pub fn offline_uuid(username: &str) -> Uuid {
    let digest = md5::compute(format!("OfflinePlayer:{username}").as_bytes());
    uuid::Builder::from_md5_bytes(digest.0).into_uuid()
}
pub const SERVER_PACK_STATUS_PROMPT: u8 = 0;
pub const SERVER_PACK_STATUS_ENABLED: u8 = 1;
pub const SERVER_PACK_STATUS_DISABLED: u8 = 2;
pub mod resource_pack_action {
    pub const SUCCESSFULLY_LOADED: u8 = 0;
    pub const DECLINED: u8 = 1;
    pub const FAILED_DOWNLOAD: u8 = 2;
    pub const ACCEPTED: u8 = 3;
    pub const DOWNLOADED: u8 = 4;
    pub const INVALID_URL: u8 = 5;
    pub const FAILED_RELOAD: u8 = 6;
    pub const DISCARDED: u8 = 7;
    pub const fn canonical(action: u8) -> u8 {
        if action <= DISCARDED {
            action
        } else {
            FAILED_RELOAD
        }
    }
    pub const fn name(action: u8) -> &'static str {
        match canonical(action) {
            SUCCESSFULLY_LOADED => "SuccessfullyLoaded",
            DECLINED => "Declined",
            FAILED_DOWNLOAD => "FailedDownload",
            ACCEPTED => "Accepted",
            DOWNLOADED => "Downloaded",
            INVALID_URL => "InvalidUrl",
            FAILED_RELOAD => "FailedReload",
            DISCARDED => "Discarded",
            _ => unreachable!(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolBackendManifest {
    pub ipc_version: u32,
    pub backend_name: String,
    pub minecraft_version: String,
    pub protocol_version: i32,
    pub protocol_source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets_compatibility: Option<String>,
    #[serde(default)]
    pub supports_game_rule_editor: bool,
}
impl ProtocolBackendManifest {
    pub fn new(
        backend_name: impl Into<String>,
        minecraft_version: impl Into<String>,
        protocol_version: i32,
        protocol_source: impl Into<String>,
    ) -> Self {
        Self {
            ipc_version: ELTHARN_PROTOCOL_BACKEND_IPC_VERSION,
            backend_name: backend_name.into(),
            minecraft_version: minecraft_version.into(),
            protocol_version,
            protocol_source: protocol_source.into(),
            executable: None,
            assets_compatibility: None,
            supports_game_rule_editor: false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendStatus {
    BuiltIn {
        version: String,
        protocol_version: i32,
        ipc_version: u32,
        is_fallback_assumed: bool,
    },
    Ready {
        path: std::path::PathBuf,
        backend_name: String,
        protocol_version: i32,
        protocol_source: String,
    },
    Missing,
    Broken {
        reason: String,
    },
    VersionMismatch {
        expected: String,
        found: String,
    },
    IpcMismatch {
        expected: u32,
        found: u32,
    },
    UnsafeExecutablePath {
        path: String,
    },
    AssetsMismatch {
        expected: String,
        found: String,
    },
}
pub fn validate_manifest(
    manifest_content: &str,
    expected_version: &str,
    expected_assets: &str,
    expected_ipc_version: u32,
    version_dir: &std::path::Path,
) -> BackendStatus {
    let manifest: ProtocolBackendManifest = match serde_json::from_str(manifest_content) {
        Ok(m) => m,
        Err(e) => {
            return BackendStatus::Broken {
                reason: format!("Failed to parse JSON: {e}"),
            };
        }
    };
    if manifest.ipc_version != expected_ipc_version {
        return BackendStatus::IpcMismatch {
            expected: expected_ipc_version,
            found: manifest.ipc_version,
        };
    }
    if manifest.minecraft_version != expected_version {
        return BackendStatus::VersionMismatch {
            expected: expected_version.to_string(),
            found: manifest.minecraft_version,
        };
    }
    if manifest.protocol_version <= 0 {
        return BackendStatus::Broken {
            reason: format!("Invalid protocol version: {}", manifest.protocol_version),
        };
    }
    if manifest.backend_name.trim().is_empty() {
        return BackendStatus::Broken {
            reason: "backend_name field is empty".to_string(),
        };
    }
    if manifest.protocol_source.trim().is_empty() {
        return BackendStatus::Broken {
            reason: "protocol_source field is empty".to_string(),
        };
    }
    if let Some(assets) = &manifest.assets_compatibility {
        if assets != expected_assets {
            return BackendStatus::AssetsMismatch {
                expected: expected_assets.to_string(),
                found: assets.clone(),
            };
        }
    }
    if let Some(exec) = &manifest.executable {
        if exec.trim().is_empty() {
            return BackendStatus::Broken {
                reason: "executable field is empty".to_string(),
            };
        }
        let exec_path = std::path::Path::new(exec);
        if exec_path.is_absolute() {
            return BackendStatus::UnsafeExecutablePath { path: exec.clone() };
        }
        if exec.contains("..") || exec.contains('\\') {
            return BackendStatus::UnsafeExecutablePath { path: exec.clone() };
        }
        let full_path = version_dir.join(exec);
        if !full_path.is_file() {
            return BackendStatus::Broken {
                reason: format!("Executable file not found at {}", exec),
            };
        }
        let canonical_dir = match std::fs::canonicalize(version_dir) {
            Ok(dir) => dir,
            Err(err) => {
                return BackendStatus::Broken {
                    reason: format!("Could not resolve backend version directory: {err}"),
                };
            }
        };
        let canonical_path = match std::fs::canonicalize(&full_path) {
            Ok(path) => path,
            Err(err) => {
                return BackendStatus::Broken {
                    reason: format!("Could not resolve executable at {}: {err}", exec),
                };
            }
        };
        if !canonical_path.starts_with(&canonical_dir) {
            return BackendStatus::UnsafeExecutablePath { path: exec.clone() };
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(&full_path) {
                let mode = metadata.permissions().mode();
                if mode & 0o111 == 0 {
                    return BackendStatus::Broken {
                        reason: format!("Executable file at {} is not executable", exec),
                    };
                }
            } else {
                return BackendStatus::Broken {
                    reason: format!("Could not read executable metadata at {}", exec),
                };
            }
        }
        BackendStatus::Ready {
            path: full_path,
            backend_name: manifest.backend_name,
            protocol_version: manifest.protocol_version,
            protocol_source: manifest.protocol_source,
        }
    } else {
        BackendStatus::Broken {
            reason: "executable field is missing from manifest".to_string(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientToBackendMessage {
    Hello {
        ipc_version: u32,
        requested_minecraft_version: String,
    },
    Connect(BackendConnectArgs),
    Packet(BackendOutboundPacket),
    Disconnect {
        reason: String,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackendToClientMessage {
    Hello {
        manifest: ProtocolBackendManifest,
    },
    Status {
        level: BackendLogLevel,
        message: String,
    },
    Event(BackendNetworkEvent),
    Disconnected {
        reason: String,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendConnectArgs {
    pub server: String,
    pub username: String,
    pub language: String,
    #[serde(default = "default_uuid")]
    pub uuid: Uuid,
    #[serde(default = "default_view_distance")]
    pub view_distance: u8,
    #[serde(default)]
    pub main_hand_left: bool,
    #[serde(default = "default_allows_listing")]
    pub allows_listing: bool,
    #[serde(default = "default_server_pack_status")]
    pub server_pack_status: u8,
    #[serde(default = "default_model_customisation")]
    pub model_customisation: u8,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub server_cookies: HashMap<String, Vec<u8>>,
}
fn default_uuid() -> Uuid {
    Uuid::nil()
}
fn default_view_distance() -> u8 {
    12
}
fn default_allows_listing() -> bool {
    true
}
fn default_server_pack_status() -> u8 {
    0
}
fn default_model_customisation() -> u8 {
    0x7F
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BackendOutboundPacket {
    ResourcePackStatus {
        id: Uuid,
        action: u8,
    },
    Chat {
        message: String,
    },
    Command {
        command: String,
    },
    CustomClickAction {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload: Option<Vec<u8>>,
    },
    CustomPayload {
        channel: String,
        data: Vec<u8>,
    },
    CommandSuggestion {
        id: u32,
        command: String,
    },
    PlayerLoaded,
    ClientTickEnd,
    RequestStats,
    PerformRespawn,
    MovePosRot {
        x: f64,
        y: f64,
        z: f64,
        yaw_degrees: f32,
        pitch_degrees: f32,
        on_ground: bool,
        horizontal_collision: bool,
    },
    MovePos {
        x: f64,
        y: f64,
        z: f64,
        on_ground: bool,
        horizontal_collision: bool,
    },
    MoveRot {
        yaw_degrees: f32,
        pitch_degrees: f32,
        on_ground: bool,
        horizontal_collision: bool,
    },
    MoveStatusOnly {
        on_ground: bool,
        horizontal_collision: bool,
    },
    MoveVehicle {
        x: f64,
        y: f64,
        z: f64,
        yaw_degrees: f32,
        pitch_degrees: f32,
        on_ground: bool,
    },
    PlayerInput {
        forward: bool,
        backward: bool,
        left: bool,
        right: bool,
        jump: bool,
        shift: bool,
        sprint: bool,
    },
    PlayerCommand {
        entity_id: i32,
        action: String,
        data: i32,
    },
    AcceptTeleport {
        teleport_id: u32,
    },
    TeleportConfirm {
        teleport_id: u32,
        x: f64,
        y: f64,
        z: f64,
        yaw_degrees: f32,
        pitch_degrees: f32,
    },
    SetCarriedItem {
        slot: u16,
    },
    PlayerAction {
        action: BackendPlayerActionKind,
        x: i32,
        y: i32,
        z: i32,
        direction: u8,
        seq: u32,
    },
    UseItemOn {
        hand: u8,
        x: i32,
        y: i32,
        z: i32,
        direction: u8,
        cursor_x: f32,
        cursor_y: f32,
        cursor_z: f32,
        inside: bool,
        seq: u32,
    },
    UseItem {
        hand: u8,
        seq: u32,
        yaw_degrees: f32,
        pitch_degrees: f32,
    },
    SwingMainHand,
    SwingOffHand,
    Attack {
        entity_id: i32,
        using_secondary_action: bool,
    },
    Interact {
        entity_id: i32,
        hand: u8,
        using_secondary_action: bool,
    },
    InteractAt {
        entity_id: i32,
        hand: u8,
        relative_x: f32,
        relative_y: f32,
        relative_z: f32,
        using_secondary_action: bool,
    },
    PaddleBoat {
        left: bool,
        right: bool,
    },
    ContainerClick {
        container_id: i32,
        state_id: i32,
        slot_num: i16,
        button_num: i8,
        click_type: i32,
        changed_slots: Vec<(i16, BackendItemStack)>,
        carried_item: BackendItemStack,
    },
    SetCreativeModeSlot {
        slot_num: i16,
        item_stack: BackendItemStack,
    },
    ContainerClose {
        container_id: i32,
    },
    PlayerAbilities {
        is_flying: bool,
    },
    ChangeGameMode {
        mode: String,
    },
    Raw {
        packet: String,
        payload: serde_json::Value,
    },
    Pong {
        id: u32,
    },
    SignUpdate {
        x: i32,
        y: i32,
        z: i32,
        is_front_text: bool,
        lines: [String; 4],
    },
    EditBook {
        slot: u32,
        pages: Vec<String>,
        title: Option<String>,
    },
    RecipeBookSeenRecipe {
        recipe: String,
    },
    RecipeBookChangeSettings {
        book_type: u8,
        is_open: bool,
        is_filtering: bool,
    },
    PlaceRecipe {
        container_id: i32,
        recipe: String,
        shift_down: bool,
    },
    SetStructureBlock {
        x: i32,
        y: i32,
        z: i32,
        update_type: u8,
        mode: u8,
        name: String,
        offset_x: i8,
        offset_y: i8,
        offset_z: i8,
        size_x: u8,
        size_y: u8,
        size_z: u8,
        mirror: u8,
        rotation: u8,
        data: String,
        integrity: f32,
        seed: u64,
        ignore_entities: bool,
        show_air: bool,
        show_bounding_box: bool,
    },
    SetJigsawBlock {
        x: i32,
        y: i32,
        z: i32,
        name: String,
        target: String,
        pool: String,
        final_state: String,
        joint: String,
        selection_priority: i32,
        placement_priority: i32,
    },
    SetCommandBlock {
        x: i32,
        y: i32,
        z: i32,
        command: String,
        mode: u8,
        track_output: bool,
        conditional: bool,
        automatic: bool,
    },
    SetCommandMinecart {
        entity_id: u32,
        command: String,
        track_output: bool,
    },
    SetTestBlock {
        x: i32,
        y: i32,
        z: i32,
        mode: u8,
        message: String,
    },
    SpectatorAction {
        spectate_entity_id: Option<i32>,
    },
    PickItem {
        slot: u32,
    },
    PickItemFromEntity {
        id: i32,
        include_data: bool,
    },
    SelectTrade {
        item: u32,
    },
    SetBeacon {
        primary: Option<u32>,
        secondary: Option<u32>,
    },
    RenameItem {
        name: String,
    },
    CookieResponse {
        key: String,
        payload: Option<Vec<u8>>,
    },
    AcceptCodeOfConduct,
    RequestGameRuleValues,
    SetGameRule {
        key: String,
        value: String,
    },
    LockDifficulty {
        locked: bool,
    },
    SeenAdvancements {
        action: BackendSeenAdvancementsAction,
        tab: Option<String>,
    },
    ChatSessionUpdate {
        session_id: uuid::Uuid,
        expires_at: u64,
        key: Vec<u8>,
        key_signature: Vec<u8>,
    },
    ChatAck {
        offset: u32,
    },
    BundleItemSelected {
        slot_id: i32,
        selected_item_index: u32,
    },
    ContainerButtonClick {
        container_id: i32,
        button_id: u32,
    },
    ContainerSlotStateChanged {
        slot_id: u32,
        container_id: i32,
        new_state: bool,
    },
    BlockEntityTagQuery {
        transaction_id: u32,
        x: i32,
        y: i32,
        z: i32,
    },
    EntityTagQuery {
        transaction_id: u32,
        entity_id: i32,
    },
    DebugSubscription {
        subscriptions: Vec<String>,
    },
    TestInstanceBlockAction {
        x: i32,
        y: i32,
        z: i32,
        action: u8,
        test: Option<String>,
        size_x: i32,
        size_y: i32,
        size_z: i32,
        rotation: u8,
        ignore_entities: bool,
        status: u8,
        error_message: Option<String>,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendSeenAdvancementsAction {
    OpenedTab,
    ClosedScreen,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendPlayerActionKind {
    StartDestroyBlock,
    AbortDestroyBlock,
    StopDestroyBlock,
    DropAllItems,
    DropItem,
    ReleaseUseItem,
    SwapItemWithOffhand,
    Stab,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendBiomeRenderData {
    pub id: String,
    pub has_precipitation: bool,
    pub temperature: f32,
    pub temperature_modifier: String,
    pub downfall: f32,
    pub water_color: u32,
    pub foliage_color: Option<u32>,
    pub dry_foliage_color: Option<u32>,
    pub grass_color: Option<u32>,
    pub grass_color_modifier: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BackendBlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendDebugSubscriptionUpdate {
    pub subscription: String,
    pub value: Option<serde_json::Value>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendDebugSubscriptionEvent {
    pub subscription: String,
    pub value: serde_json::Value,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendRotations {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendEntityPose {
    Standing,
    FallFlying,
    Sleeping,
    Swimming,
    SpinAttack,
    Crouching,
    LongJumping,
    Dying,
    Croaking,
    UsingTongue,
    Sitting,
    Roaring,
    Sniffing,
    Emerging,
    Digging,
    Sliding,
    Shooting,
    Inhaling,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "pos", rename_all = "snake_case")]
pub enum BackendSleepingPosUpdate {
    Clear,
    Set(BackendBlockPos),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "pos", rename_all = "snake_case")]
pub enum BackendEndCrystalBeamTargetUpdate {
    Clear,
    Set(BackendBlockPos),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "text", rename_all = "snake_case")]
pub enum BackendCustomNameUpdate {
    Clear,
    Set(String),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendMinecartStep {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub weight: f32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum BackendNbtTag {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<BackendNbtTag>),
    Compound(Vec<(String, BackendNbtTag)>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum BackendChatClickAction {
    RunCommand {
        command: String,
    },
    SuggestCommand {
        command: String,
    },
    CopyToClipboard {
        value: String,
    },
    OpenUrl {
        url: String,
    },
    OpenFile {
        path: String,
    },
    ChangePage {
        page: i32,
    },
    ShowDialog,
    Custom {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload: Option<Vec<u8>>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum BackendChatHoverAction {
    ShowText {
        value: String,
    },
    ShowItem {
        id: String,
        #[serde(
            default = "default_item_count",
            skip_serializing_if = "is_default_item_count"
        )]
        count: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tag: Option<Vec<u8>>,
    },
    ShowEntity {
        id: String,
        uuid: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
}
fn default_item_count() -> u32 {
    1
}
fn is_default_item_count(count: &u32) -> bool {
    *count == 1
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendChatClickSpan {
    pub start: usize,
    pub end: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<BackendChatClickAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover: Option<BackendChatHoverAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insertion: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendItemStack {
    pub tag: BackendNbtTag,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendBlockEntityData {
    pub kind: String,
    pub nbt: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendBlockEntityEntry {
    pub pos: BackendBlockPos,
    pub data: BackendBlockEntityData,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendPalettedContainer<T> {
    pub palette: Vec<T>,
    pub storage: Vec<u64>,
    pub bits_per_entry: usize,
    pub capacity: usize,
    pub default_value: T,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendChunkSection {
    pub y_index: i32,
    pub non_empty_block_count: u16,
    pub fluid_count: u16,
    pub ticking_block_count: u16,
    pub ticking_fluid_count: u16,
    pub blocks: BackendPalettedContainer<usize>,
    pub biomes: BackendPalettedContainer<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendChunk {
    pub x: i32,
    pub z: i32,
    pub sections: Vec<BackendChunkSection>,
    pub min_y_section: i32,
    pub heightmaps: std::collections::HashMap<String, Vec<u64>>,
    pub block_entities: Vec<BackendBlockEntityEntry>,
    pub sky_light: Vec<Option<Vec<u8>>>,
    pub block_light: Vec<Option<Vec<u8>>>,
    pub is_light_correct: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendLightUpdateEvent {
    pub x: i32,
    pub z: i32,
    pub sky_layers: Vec<(usize, Option<Vec<u8>>)>,
    pub block_layers: Vec<(usize, Option<Vec<u8>>)>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendChunkBiomeUpdate {
    pub x: i32,
    pub z: i32,
    pub sections: Vec<Vec<String>>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendBlockUpdateEvent {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub state_id: usize,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendRemoteEntityEvent {
    pub id: i32,
    pub uuid: Uuid,
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub head_yaw_degrees: f32,
    #[serde(default)]
    pub spawn_data: i32,
    #[serde(default)]
    pub falling_block_state_id: Option<usize>,
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub velocity_z: f64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendNetworkChatSuggestion {
    pub text: String,
    pub tooltip: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendCommandNode {
    pub executable: bool,
    pub children: Vec<u32>,
    pub redirect_node: Option<u32>,
    pub node_type: BackendCommandNodeType,
    pub restricted: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackendCommandParser {
    Bool,
    Number {
        kind: BackendCommandNumberKind,
        min: Option<String>,
        max: Option<String>,
    },
    String {
        kind: BackendCommandStringKind,
    },
    Entity {
        single: bool,
        players_only: bool,
    },
    ScoreHolder {
        allows_multiple: bool,
    },
    Time {
        min: i32,
    },
    Named {
        kind: String,
        registry_key: Option<String>,
    },
}
impl Default for BackendCommandParser {
    fn default() -> Self {
        Self::Named {
            kind: "unknown".to_owned(),
            registry_key: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendCommandNumberKind {
    Float,
    Double,
    Integer,
    Long,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendCommandStringKind {
    SingleWord,
    QuotablePhrase,
    GreedyPhrase,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackendCommandNodeType {
    Root,
    Literal {
        name: String,
    },
    Argument {
        name: String,
        parser: String,
        #[serde(default)]
        parser_info: BackendCommandParser,
        suggestions_type: Option<String>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendChatTrust {
    Secure,
    Modified,
    NotSecure,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendChatCompletionAction {
    Add,
    Remove,
    Set,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendAttributeSnapshot {
    pub attribute_id: String,
    pub base: f64,
    pub modifiers: Vec<(String, f64, u8)>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendRelativeMovementFlags {
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub y_rot: bool,
    pub x_rot: bool,
    pub delta_x: bool,
    pub delta_y: bool,
    pub delta_z: bool,
    pub rotate_delta: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum BackendWaypointIdentifier {
    Uuid(Uuid),
    String(String),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendWaypointIcon {
    pub style: String,
    pub color: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BackendWaypointData {
    Empty,
    Vec3i { x: i32, y: i32, z: i32 },
    Chunk { x: i32, z: i32 },
    Azimuth { angle: f32 },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendWaypoint {
    pub identifier: BackendWaypointIdentifier,
    pub icon: BackendWaypointIcon,
    pub data: BackendWaypointData,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendPlayerInfoAddPlayer {
    pub name: String,
    pub texture_url: Option<String>,
    pub model: Option<String>,
    pub cape_url: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BackendPlayerInfoDisplayNameUpdate {
    Clear,
    Set { text: String },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendPlayerInfoUpdateEntry {
    pub uuid: Uuid,
    pub add_player: Option<BackendPlayerInfoAddPlayer>,
    pub game_mode: Option<u8>,
    pub listed: Option<bool>,
    pub latency: Option<i32>,
    pub display_name: Option<BackendPlayerInfoDisplayNameUpdate>,
    pub list_order: Option<i32>,
    pub show_hat: Option<bool>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BackendScoreNumberFormat {
    Blank,
    Styled {
        color: Option<i32>,
        #[serde(default)]
        style: Option<serde_json::Value>,
    },
    Fixed {
        text: String,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BackendParticle {
    Simple {
        name: String,
    },
    Dust {
        color: [f32; 3],
        scale: f32,
    },
    Block {
        block_state_id: u32,
    },
    BlockMarker {
        block_state_id: u32,
    },
    FallingDust {
        block_state_id: u32,
    },
    DustPillar {
        block_state_id: u32,
    },
    BlockCrumble {
        block_state_id: u32,
    },
    DustColorTransition {
        from: [f32; 3],
        to: [f32; 3],
        scale: f32,
    },
    EntityEffect {
        color: [f32; 3],
    },
    Effect {
        color: [f32; 3],
        power: f32,
    },
    InstantEffect {
        color: [f32; 3],
        power: f32,
    },
    Item {
        item_location: String,
        count: i32,
    },
    SculkCharge {
        roll: f32,
    },
    Shriek {
        delay: i32,
    },
    Vibration {
        x: f64,
        y: f64,
        z: f64,
        entity_id: i32,
        y_offset: f32,
        ticks: u32,
    },
    Geyser {
        water_blocks: i32,
    },
    GeyserBase {
        water_blocks: i32,
        burst_impulse_base: f32,
    },
    DragonBreath {
        power: f32,
    },
    Flash {
        color: [f32; 3],
    },
    Trail {
        target: [f64; 3],
        color: [f32; 3],
        duration: i32,
    },
    TintedLeaves {
        color: [f32; 3],
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendParticleSpawn {
    pub override_limiter: bool,
    pub always_show: bool,
    pub pos: [f64; 3],
    pub x_dist: f32,
    pub y_dist: f32,
    pub z_dist: f32,
    pub max_speed: f32,
    pub count: u32,
    pub particle: BackendParticle,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackendExplosionParticleInfo {
    pub particle: BackendParticle,
    pub scaling: f32,
    pub speed: f32,
    pub weight: i32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BackendNetworkEvent {
    Connected {
        protocol_version: i32,
        version_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        transport_peer: Option<std::net::SocketAddr>,
    },
    Disconnected {
        reason: String,
    },
    Login {
        entity_id: i32,
        player_uuid: Uuid,
        game_mode: u8,
        show_death_screen: bool,
        #[serde(default)]
        online_mode: bool,
        #[serde(default)]
        hardcore: bool,
        dimension: String,
        sea_level: i32,
        min_y: i32,
        height: u32,
        #[serde(default)]
        biome_zoom_seed: i64,
        biomes: Vec<BackendBiomeRenderData>,
    },
    Respawn {
        game_mode: u8,
        dimension: String,
        sea_level: i32,
        min_y: i32,
        height: u32,
        #[serde(default)]
        biome_zoom_seed: i64,
        biomes: Vec<BackendBiomeRenderData>,
        data_to_keep: u8,
    },
    PlayerPosition {
        teleport_id: u32,
        raw_x: f64,
        raw_y: f64,
        raw_z: f64,
        raw_yaw_degrees: f32,
        raw_pitch_degrees: f32,
        raw_delta_x: f64,
        raw_delta_y: f64,
        raw_delta_z: f64,
        relative: BackendRelativeMovementFlags,
    },
    PlayerRotation {
        raw_yaw_degrees: f32,
        raw_pitch_degrees: f32,
        relative_y: bool,
        relative_x: bool,
    },
    ChunkLoaded {
        x: i32,
        z: i32,
        non_air_blocks: usize,
        chunk: BackendChunk,
    },
    ChunkUnloaded {
        x: i32,
        z: i32,
    },
    ChunkCacheCenter {
        x: i32,
        z: i32,
    },
    ChunkCacheRadius {
        radius: u32,
    },
    SimulationDistance {
        distance: u32,
    },
    BlockUpdate {
        update: BackendBlockUpdateEvent,
    },
    SectionBlocksUpdate {
        updates: Vec<BackendBlockUpdateEvent>,
    },
    Sound {
        sound: String,
        source: u8,
        x: f64,
        y: f64,
        z: f64,
        volume: f32,
        pitch: f32,
        seed: u64,
        #[serde(default)]
        fixed_range: Option<f32>,
    },
    LevelEvent {
        event_type: u32,
        pos: BackendBlockPos,
        data: u32,
        global_event: bool,
    },
    SoundEntity {
        sound: String,
        source: u8,
        entity_id: i32,
        volume: f32,
        pitch: f32,
        seed: u64,
        #[serde(default)]
        fixed_range: Option<f32>,
    },
    StopSound {
        #[serde(default)]
        sound: Option<String>,
        #[serde(default)]
        source: Option<u8>,
    },
    BlockEvent {
        pos: BackendBlockPos,
        action_id: u8,
        action_parameter: u8,
    },
    BlockEntityUpdate {
        pos: BackendBlockPos,
        data: BackendBlockEntityData,
    },
    LightUpdate {
        update: BackendLightUpdateEvent,
    },
    ChunkBiomesUpdated {
        updates: Vec<BackendChunkBiomeUpdate>,
    },
    BlockChangedAck {
        sequence: u32,
    },
    InventoryContent {
        items: Vec<BackendItemStack>,
    },
    InventorySlot {
        slot: u32,
        item: BackendItemStack,
        direct_player_slot: bool,
    },
    SetCursorItem {
        item: BackendItemStack,
    },
    OpenScreen {
        container_id: i32,
        menu_type: u32,
        title: String,
    },
    ContainerContent {
        container_id: i32,
        state_id: u32,
        items: Vec<BackendItemStack>,
        carried_item: BackendItemStack,
    },
    ContainerSlot {
        container_id: i32,
        state_id: u32,
        slot: u32,
        item: BackendItemStack,
    },
    ContainerClose {
        container_id: i32,
    },
    TransferRequested {
        host: String,
        port: u32,
    },
    EntityAdded {
        entity: BackendRemoteEntityEvent,
    },
    EntityEvent {
        id: i32,
        event_id: u8,
    },
    EntityAnimation {
        id: i32,
        action: u8,
    },
    EntityMetadata {
        id: i32,
        item_stack: Option<BackendItemStack>,
        #[serde(default)]
        no_gravity: Option<bool>,
        #[serde(default)]
        item_frame_rotation: Option<i32>,
        #[serde(default)]
        item_frame_direction: Option<u8>,
        shared_flags: Option<u8>,
        living_flags: Option<u8>,
        #[serde(default)]
        living_health: Option<f32>,
        custom_name: Option<BackendCustomNameUpdate>,
        custom_name_visible: Option<bool>,
        interaction_width: Option<f32>,
        interaction_height: Option<f32>,
        interaction_response: Option<bool>,
        pose: Option<BackendEntityPose>,
        sleeping_pos: Option<BackendSleepingPosUpdate>,
        player_main_arm_right: Option<bool>,
        player_model_customisation: Option<u8>,
        player_absorption: Option<f32>,
        #[serde(default)]
        ticks_frozen: Option<i32>,
        #[serde(default)]
        mob_size: Option<i32>,
        #[serde(default)]
        primed_tnt_fuse: Option<i32>,
        #[serde(default)]
        primed_tnt_block_state_id: Option<usize>,
        #[serde(default)]
        end_crystal_beam_target: Option<BackendEndCrystalBeamTargetUpdate>,
        #[serde(default)]
        end_crystal_show_bottom: Option<bool>,
        #[serde(default)]
        abstract_arrow_flags: Option<u8>,
        #[serde(default)]
        abstract_arrow_pierce_level: Option<u8>,
        #[serde(default)]
        abstract_arrow_in_ground: Option<bool>,
        #[serde(default)]
        arrow_effect_color: Option<i32>,
        #[serde(default)]
        experience_orb_value: Option<i32>,
        #[serde(default)]
        armor_stand_client_flags: Option<u8>,
        #[serde(default)]
        armor_stand_head_pose: Option<BackendRotations>,
        #[serde(default)]
        armor_stand_body_pose: Option<BackendRotations>,
        #[serde(default)]
        armor_stand_left_arm_pose: Option<BackendRotations>,
        #[serde(default)]
        armor_stand_right_arm_pose: Option<BackendRotations>,
        #[serde(default)]
        armor_stand_left_leg_pose: Option<BackendRotations>,
        #[serde(default)]
        armor_stand_right_leg_pose: Option<BackendRotations>,
        #[serde(default)]
        is_baby: Option<bool>,
        #[serde(default)]
        is_aggressive: Option<bool>,
        #[serde(default)]
        mob_flags: Option<u8>,
        #[serde(default)]
        camel_dashing: Option<bool>,
        #[serde(default)]
        camel_last_pose_change_tick: Option<i64>,
        #[serde(default)]
        mob_boolean_metadata: Vec<(u8, bool)>,
        #[serde(default)]
        piglin_is_baby: Option<bool>,
        #[serde(default)]
        piglin_is_charging_crossbow: Option<bool>,
        #[serde(default)]
        piglin_is_dancing: Option<bool>,
        #[serde(default)]
        tamable_flags: Option<u8>,
        #[serde(default)]
        creaking_can_move: Option<bool>,
        #[serde(default)]
        creaking_is_active: Option<bool>,
        #[serde(default)]
        creaking_is_tearing_down: Option<bool>,
        #[serde(default)]
        display_interpolation_start_delta_ticks: Option<i32>,
        #[serde(default)]
        display_interpolation_duration: Option<i32>,
        #[serde(default)]
        display_teleport_duration: Option<i32>,
        #[serde(default)]
        display_translation: Option<[f32; 3]>,
        #[serde(default)]
        display_scale: Option<[f32; 3]>,
        #[serde(default)]
        display_left_rotation: Option<[f32; 4]>,
        #[serde(default)]
        display_right_rotation: Option<[f32; 4]>,
        #[serde(default)]
        display_billboard: Option<u8>,
        #[serde(default)]
        display_brightness_override: Option<i32>,
        #[serde(default)]
        display_view_range: Option<f32>,
        #[serde(default)]
        display_shadow_radius: Option<f32>,
        #[serde(default)]
        display_shadow_strength: Option<f32>,
        #[serde(default)]
        display_width: Option<f32>,
        #[serde(default)]
        display_height: Option<f32>,
        #[serde(default)]
        display_glow_color_override: Option<i32>,
        #[serde(default)]
        text_display_text: Option<String>,
        #[serde(default)]
        text_display_line_width: Option<i32>,
        #[serde(default)]
        text_display_background: Option<i32>,
        #[serde(default)]
        text_display_text_opacity: Option<i8>,
        #[serde(default)]
        text_display_flags: Option<u8>,
        #[serde(default)]
        block_display_block_state_id: Option<usize>,
        #[serde(default)]
        item_display_item: Option<BackendItemStack>,
        #[serde(default)]
        item_display_context: Option<u8>,
    },
    EntityVelocity {
        id: i32,
        x: f64,
        y: f64,
        z: f64,
    },
    Explosion {
        center_x: f64,
        center_y: f64,
        center_z: f64,
        radius: f32,
        block_count: i32,
        player_knockback: Option<[f64; 3]>,
        #[serde(default)]
        explosion_particle: Option<BackendParticle>,
        #[serde(default)]
        explosion_sound: Option<String>,
        #[serde(default)]
        block_particles: Vec<BackendExplosionParticleInfo>,
    },
    UpdateAttributes {
        entity_id: i32,
        attributes: Vec<BackendAttributeSnapshot>,
    },
    WorldBorder {
        center_x: f64,
        center_z: f64,
        size: f64,
        absolute_max_size: u32,
    },
    WorldBorderInitialize {
        center_x: f64,
        center_z: f64,
        old_size: f64,
        new_size: f64,
        lerp_time: u64,
        absolute_max_size: u32,
        warning_blocks: u32,
        warning_time: u32,
    },
    WorldBorderCenter {
        center_x: f64,
        center_z: f64,
    },
    WorldBorderLerpSize {
        old_size: f64,
        new_size: f64,
        lerp_time: u64,
    },
    WorldBorderSize {
        size: f64,
    },
    WorldBorderWarningDistance {
        warning_blocks: u32,
    },
    WorldBorderWarningDelay {
        warning_time: u32,
    },
    ChangeDifficulty {
        difficulty: u8,
        locked: bool,
    },
    TabList {
        header: String,
        footer: String,
    },
    BossEvent {
        id: uuid::Uuid,
        operation: BackendBossEventOperation,
    },
    SetPassengers {
        vehicle_id: i32,
        passenger_ids: Vec<i32>,
    },
    MoveVehicle {
        x: f64,
        y: f64,
        z: f64,
        yaw_degrees: f32,
        pitch_degrees: f32,
    },
    MoveMinecartAlongTrack {
        entity_id: i32,
        steps: Vec<BackendMinecartStep>,
    },
    EntityPositionSync {
        id: i32,
        x: f64,
        y: f64,
        z: f64,
        yaw_degrees: f32,
        pitch_degrees: f32,
        on_ground: bool,
    },
    EntityTeleport {
        id: i32,
        raw_x: f64,
        raw_y: f64,
        raw_z: f64,
        raw_delta_x: f64,
        raw_delta_y: f64,
        raw_delta_z: f64,
        raw_yaw_degrees: f32,
        raw_pitch_degrees: f32,
        relative: BackendRelativeMovementFlags,
        on_ground: bool,
    },
    EntityDeltaMoved {
        id: i32,
        xa: i16,
        ya: i16,
        za: i16,
        has_position: bool,
        yaw_degrees: Option<f32>,
        pitch_degrees: Option<f32>,
        on_ground: bool,
    },
    EntityHeadLook {
        id: i32,
        y_head_rot_degrees: f32,
    },
    EntityEquipment {
        id: i32,
        main_hand: Option<BackendItemStack>,
        off_hand: Option<BackendItemStack>,
        head: Option<BackendItemStack>,
        chest: Option<BackendItemStack>,
        legs: Option<BackendItemStack>,
        feet: Option<BackendItemStack>,
        body: Option<BackendItemStack>,
        saddle: Option<BackendItemStack>,
    },
    EntitiesRemoved {
        ids: Vec<i32>,
    },
    SystemChat {
        message: String,
        overlay: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        click_spans: Vec<BackendChatClickSpan>,
    },
    ActionBar {
        message: String,
    },
    TitleText {
        message: String,
    },
    SubtitleText {
        message: String,
    },
    TitlesAnimation {
        fade_in: u32,
        stay: u32,
        fade_out: u32,
    },
    ClearTitles {
        reset_times: bool,
    },
    CommandTree {
        root_commands: Vec<String>,
        #[serde(default)]
        root_index: u32,
        #[serde(default)]
        nodes: Vec<BackendCommandNode>,
    },
    CommandSuggestions {
        id: u32,
        range_start: usize,
        range_end: usize,
        suggestions: Vec<BackendNetworkChatSuggestion>,
    },
    OnlinePlayers {
        names: Vec<String>,
    },
    CustomChatCompletions {
        action: BackendChatCompletionAction,
        entries: Vec<String>,
    },
    PlayerChat {
        message: String,
        signed_content: String,
        signature: Option<Vec<u8>>,
        trust: BackendChatTrust,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        click_spans: Vec<BackendChatClickSpan>,
    },
    DisguisedChat {
        message: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        click_spans: Vec<BackendChatClickSpan>,
    },
    DeleteChat {
        signature: Vec<u8>,
    },
    Health {
        health: f32,
        food: u32,
        saturation: f32,
    },
    Experience {
        progress: f32,
        level: u32,
        total: u32,
    },
    Abilities {
        invulnerable: bool,
        flying: bool,
        mayfly: bool,
        instabuild: bool,
        flying_speed: f32,
        walking_speed: f32,
    },
    Time {
        game_time: i64,
        day_time: Option<i64>,
        day_time_rate: Option<f32>,
        day_time_partial_tick: Option<f32>,
    },
    HeldSlot {
        slot: u32,
    },
    GameModeChanged {
        game_mode: u8,
    },
    GameEvent {
        event_id: u8,
        value: f32,
    },
    ChunkBatchStart,
    ChunkBatchFinished {
        batch_size: u32,
    },
    TickingState {
        tick_rate: f32,
        is_frozen: bool,
    },
    TickingStep {
        tick_steps: u32,
    },
    Raw {
        event: String,
        payload: serde_json::Value,
    },
    Ping {
        id: u32,
    },
    PongResponse {
        time: u64,
    },
    ProjectilePower {
        entity_id: i32,
        acceleration_power: f64,
    },
    PlayerLookAt {
        from_anchor: u8,
        x: f64,
        y: f64,
        z: f64,
        at_entity: bool,
        entity_id: Option<i32>,
        to_anchor: Option<u8>,
    },
    BlockDestruction {
        id: i32,
        pos: [i32; 3],
        progress: i32,
    },
    SetEntityLink {
        source_id: i32,
        dest_id: i32,
    },
    LevelParticles(Box<BackendParticleSpawn>),
    DebugBlockValue {
        pos: BackendBlockPos,
        update: BackendDebugSubscriptionUpdate,
    },
    DebugChunkValue {
        chunk_x: i32,
        chunk_z: i32,
        update: BackendDebugSubscriptionUpdate,
    },
    DebugEntityValue {
        entity_id: i32,
        update: BackendDebugSubscriptionUpdate,
    },
    DebugEvent {
        event: BackendDebugSubscriptionEvent,
    },
    GameTestHighlightPos {
        absolute_pos: BackendBlockPos,
        relative_pos: BackendBlockPos,
    },
    MountScreenOpen {
        container_id: i32,
        inventory_columns: u32,
        entity_id: i32,
    },
    SelectAdvancementsTab {
        tab: Option<String>,
    },
    TestInstanceBlockStatus {
        status: String,
        size: Option<[i32; 3]>,
    },
    ShowDialog {
        dialog_kind: Option<String>,
        dialog_nbt: Option<serde_json::Value>,
    },
    ClearDialog,
    UpdateMobEffect {
        entity_id: i32,
        effect_id: u32,
        amplifier: i32,
        duration: i32,
        ambient: bool,
        show_particles: bool,
        show_icon: bool,
        blend: bool,
    },
    RemoveMobEffect {
        entity_id: i32,
        effect_id: u32,
    },
    SetCamera {
        camera_id: i32,
    },
    HurtAnimation {
        entity_id: i32,
        yaw: f32,
    },
    DamageEvent {
        entity_id: i32,
        source_type_id: u32,
        source_cause_id: Option<i32>,
        source_direct_id: Option<i32>,
        source_pos_x: Option<f64>,
        source_pos_y: Option<f64>,
        source_pos_z: Option<f64>,
    },
    TakeItem {
        item_id: i32,
        player_id: i32,
        amount: u32,
    },
    SetDefaultSpawnPosition {
        x: i32,
        y: i32,
        z: i32,
        yaw: f32,
        pitch: f32,
    },
    Cooldown {
        cooldown_group: String,
        duration_ticks: i32,
    },
    ContainerSetData {
        container_id: i32,
        id: u32,
        value: i32,
    },
    AddPlayerInfo {
        uuid: uuid::Uuid,
        name: String,
        texture_url: Option<String>,
        model: Option<String>,
        cape_url: Option<String>,
    },
    RemovePlayerInfo {
        uuid: uuid::Uuid,
    },
    PlayerInfoUpdate {
        entries: Vec<BackendPlayerInfoUpdateEntry>,
    },
    SetPlayerTeam {
        name: String,
        op: u8,
        collision_rule: Option<String>,
        members: Option<Vec<String>>,
        display_name: Option<String>,
        prefix: Option<String>,
        suffix: Option<String>,
        team_color: i32,
        nametag_visibility: Option<String>,
        team_options: Option<u8>,
    },
    RecipeBookSettings {
        gui_open: bool,
        filtering_craftable: bool,
        furnace_gui_open: bool,
        furnace_filtering_craftable: bool,
        blast_furnace_gui_open: bool,
        blast_furnace_filtering_craftable: bool,
        smoker_gui_open: bool,
        smoker_filtering_craftable: bool,
    },
    RecipeBookAdd {
        recipe_ids: Vec<u32>,
        replace: bool,
    },
    RecipeBookRemove {
        recipe_ids: Vec<u32>,
    },
    UpdateRecipes,
    UpdateTags,
    PlaceGhostRecipe {
        container_id: i32,
    },
    PlayerCombatKill {
        player_id: i32,
        message: String,
    },
    PlayerCombatEnter,
    PlayerCombatEnd {
        duration: u32,
    },
    SetObjective {
        objective_name: String,
        title: String,
        display_format: u8,
        method: u8,
        number_format: Option<BackendScoreNumberFormat>,
    },
    SetScore {
        owner: String,
        objective_name: String,
        score: u32,
        display_name: Option<String>,
        display_format: Option<u8>,
        number_format: Option<BackendScoreNumberFormat>,
    },
    ResetScore {
        owner: String,
        objective_name: Option<String>,
    },
    SetDisplayObjective {
        slot: u8,
        objective_name: String,
    },
    CustomPayload {
        channel: String,
        data: Vec<u8>,
    },
    ResourcePackPush {
        id: uuid::Uuid,
        url: String,
        hash: String,
        required: bool,
        prompt: Option<String>,
        server_pack_status: u8,
        initial_status_sent: bool,
    },
    ResourcePackPop {
        id: Option<uuid::Uuid>,
    },
    ResetChat,
    MerchantOffers {
        container_id: i32,
        offers: Vec<String>,
        level: u32,
        xp: u32,
        show_progress: bool,
        can_restock: bool,
    },
    MapItemData {
        map_id: u32,
        scale: u8,
        locked: bool,
        tracking_position: bool,
        color_patch: Option<BackendMapPatch>,
        decorations: Option<Vec<BackendMapDecoration>>,
    },
    OpenBook {
        hand: u8,
    },
    OpenSignEditor {
        pos: [i32; 3],
        is_front_text: bool,
    },
    AwardStats {
        payload: BackendAwardStats,
    },
    UpdateAdvancements {
        payload: BackendUpdateAdvancements,
    },
    ServerData {
        motd: String,
        icon_bytes: Option<Vec<u8>>,
    },
    ServerLinks {
        links: Vec<(String, String)>,
    },
    PauseScreenDialog {
        dialog_kind: String,
        dialog_nbt: Option<serde_json::Value>,
        requires_server_links: bool,
    },
    StoreCookie {
        key: String,
        payload: Vec<u8>,
    },
    SelectKnownPacks {
        known_packs: Vec<(String, String, String)>,
    },
    RegistryData {
        registry_id: String,
        entries_count: u32,
    },
    UpdateEnabledFeatures {
        features: Vec<String>,
    },
    CodeOfConduct {
        text: String,
    },
    CookieRequest {
        key: String,
        payload: Option<Vec<u8>>,
    },
    ChatAck {
        offset: u32,
    },
    CustomReportDetails {
        details: std::collections::HashMap<String, String>,
    },
    DebugSample {
        sample: Vec<u64>,
        sample_type: u8,
    },
    Waypoint {
        operation: u8,
        waypoint: BackendWaypoint,
    },
    LowDiskSpaceWarning,
    GameRuleValues {
        values: std::collections::HashMap<String, String>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BackendBossBarColor {
    Pink = 0,
    Blue = 1,
    Red = 2,
    Green = 3,
    Yellow = 4,
    Purple = 5,
    White = 6,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BackendBossBarOverlay {
    Progress = 0,
    Notched6 = 1,
    Notched10 = 2,
    Notched12 = 3,
    Notched20 = 4,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BackendBossEventStyle {
    pub color: BackendBossBarColor,
    pub overlay: BackendBossBarOverlay,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BackendBossEventProperties {
    pub darken_screen: bool,
    pub play_music: bool,
    pub create_world_fog: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BackendBossEventAdd {
    pub name: String,
    pub progress: f32,
    pub style: BackendBossEventStyle,
    pub properties: BackendBossEventProperties,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BackendBossEventOperation {
    Add(BackendBossEventAdd),
    Remove,
    UpdateProgress(f32),
    UpdateName(String),
    UpdateStyle(BackendBossEventStyle),
    UpdateProperties(BackendBossEventProperties),
}
pub fn encode_json_line<T: Serialize>(message: &T) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = serde_json::to_vec(message)?;
    bytes.push(b'\n');
    Ok(bytes)
}
pub fn decode_json_line<T: DeserializeOwned>(line: &[u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(line.trim_ascii_end())
}
