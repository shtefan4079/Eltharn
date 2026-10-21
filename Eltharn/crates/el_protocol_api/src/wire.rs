use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtocolPayload(pub Value);
impl ProtocolPayload {
    pub fn empty_object() -> Self {
        Self(Value::Object(Default::default()))
    }
    pub fn from_json(value: Value) -> Self {
        Self(value)
    }
    pub fn as_json(&self) -> &Value {
        &self.0
    }
    pub fn into_json(self) -> Value {
        self.0
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub enum KnownInboundEventKind {
    Connected,
    Disconnected,
    Login,
    Respawn,
    PlayerPosition,
    PlayerRotation,
    ChunkLoaded,
    ChunkUnloaded,
    ChunkCacheCenter,
    ChunkCacheRadius,
    SimulationDistance,
    BlockUpdate,
    SectionBlocksUpdate,
    BlockEvent,
    Sound,
    SoundEntity,
    LevelEvent,
    StopSound,
    BlockEntityUpdate,
    LightUpdate,
    ChunkBiomesUpdated,
    BlockChangedAck,
    InventoryContent,
    InventorySlot,
    SetCursorItem,
    OpenScreen,
    ContainerContent,
    ContainerSlot,
    ContainerClose,
    TransferRequested,
    EntityAdded,
    EntityEvent,
    EntityAnimation,
    EntityMetadata,
    EntityVelocity,
    Explosion,
    UpdateAttributes,
    WorldBorder,
    WorldBorderInitialize,
    WorldBorderCenter,
    WorldBorderLerpSize,
    WorldBorderSize,
    WorldBorderWarningDistance,
    WorldBorderWarningDelay,
    ChangeDifficulty,
    TabList,
    BossEvent,
    SetPassengers,
    MoveVehicle,
    MoveMinecartAlongTrack,
    EntityPositionSync,
    EntityTeleport,
    EntityDeltaMoved,
    EntityHeadLook,
    EntityEquipment,
    EntitiesRemoved,
    SystemChat,
    ActionBar,
    TitleText,
    SubtitleText,
    TitlesAnimation,
    ClearTitles,
    CommandTree,
    CommandSuggestions,
    OnlinePlayers,
    CustomChatCompletions,
    PlayerChat,
    DisguisedChat,
    DeleteChat,
    Health,
    Experience,
    Abilities,
    Time,
    HeldSlot,
    GameModeChanged,
    GameEvent,
    ChunkBatchStart,
    ChunkBatchFinished,
    TickingState,
    TickingStep,
    Ping,
    UpdateMobEffect,
    RemoveMobEffect,
    SetCamera,
    HurtAnimation,
    DamageEvent,
    TakeItem,
    SetDefaultSpawnPosition,
    Cooldown,
    ContainerSetData,
    AddPlayerInfo,
    RemovePlayerInfo,
    PlayerInfoUpdate,
    SetPlayerTeam,
    RecipeBookSettings,
    RecipeBookAdd,
    RecipeBookRemove,
    UpdateRecipes,
    UpdateTags,
    PlaceGhostRecipe,
    PlayerCombatKill,
    PlayerCombatEnter,
    PlayerCombatEnd,
    SetObjective,
    SetScore,
    ResetScore,
    SetDisplayObjective,
    CustomPayload,
    ResourcePackPush,
    ResourcePackPop,
    MerchantOffers,
    MapItemData,
    OpenBook,
    OpenSignEditor,
    AwardStats,
    UpdateAdvancements,
    ServerData,
    ServerLinks,
    StoreCookie,
    SelectKnownPacks,
    RegistryData,
    UpdateEnabledFeatures,
    CodeOfConduct,
    CookieRequest,
    ChatAck,
    CustomReportDetails,
    DebugSample,
    Waypoint,
    LowDiskSpaceWarning,
    GameRuleValues,
    LevelParticles,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnownOutboundActionKind {
    ResourcePackStatus,
    Chat,
    Command,
    CustomClickAction,
    CustomPayload,
    CommandSuggestion,
    PlayerLoaded,
    ClientTickEnd,
    PerformRespawn,
    MovePosRot,
    MovePos,
    MoveRot,
    MoveStatusOnly,
    MoveVehicle,
    PlayerInput,
    PlayerCommand,
    AcceptTeleport,
    TeleportConfirm,
    SetCarriedItem,
    PlayerAction,
    UseItemOn,
    UseItem,
    SwingMainHand,
    SwingOffHand,
    Attack,
    Interact,
    InteractAt,
    PaddleBoat,
    ContainerClick,
    SetCreativeModeSlot,
    ContainerClose,
    PlayerAbilities,
    ChangeGameMode,
    Pong,
    SignUpdate,
    EditBook,
    RecipeBookSeenRecipe,
    RecipeBookChangeSettings,
    PlaceRecipe,
    SetStructureBlock,
    SetJigsawBlock,
    SetCommandBlock,
    SetCommandMinecart,
    SetTestBlock,
    SpectatorAction,
    PickItem,
    PickItemFromEntity,
    SelectTrade,
    SetBeacon,
    RenameItem,
    CookieResponse,
    AcceptCodeOfConduct,
    SetGameRule,
    LockDifficulty,
    SeenAdvancements,
    ChatSessionUpdate,
    ChatAck,
    BundleItemSelected,
    ContainerButtonClick,
    ContainerSlotStateChanged,
    BlockEntityTagQuery,
    EntityTagQuery,
    DebugSubscription,
    TestInstanceBlockAction,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "direction", content = "kind", rename_all = "snake_case")]
pub enum ProtocolContractItem {
    Inbound(KnownInboundEventKind),
    Outbound(KnownOutboundActionKind),
    LegacyRawOutbound,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProtocolInboundEvent {
    Event {
        kind: KnownInboundEventKind,
        payload: ProtocolPayload,
    },
    Unknown {
        wire_kind: String,
        payload: ProtocolPayload,
    },
    Missing {
        item: ProtocolContractItem,
        detail: Option<String>,
    },
    Status {
        level: ProtocolLogLevel,
        message: String,
    },
    Disconnected {
        reason: String,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProtocolOutboundAction {
    Action {
        kind: KnownOutboundActionKind,
        payload: ProtocolPayload,
    },
    LegacyRaw {
        packet: String,
        payload: ProtocolPayload,
    },
}
pub type ClientNetworkEvent = ProtocolInboundEvent;
pub type ClientOutboundPacket = ProtocolOutboundAction;
impl ProtocolOutboundAction {
    pub fn known(kind: KnownOutboundActionKind, payload: ProtocolPayload) -> Self {
        Self::Action { kind, payload }
    }
    pub fn contract_item(&self) -> ProtocolContractItem {
        match self {
            Self::Action { kind, .. } => ProtocolContractItem::Outbound(*kind),
            Self::LegacyRaw { .. } => ProtocolContractItem::LegacyRawOutbound,
        }
    }
}
