mod stdio;
pub use stdio::run_stdio_backend;
pub fn backend_manifest() -> el_protocol_backend::ProtocolBackendManifest {
    let mut manifest = el_protocol_backend::ProtocolBackendManifest::new(
        "eltharn-protocol-1.21.11",
        VANILLA_VERSION_NAME,
        VANILLA_PROTOCOL_VERSION,
        "git+https://github.com/azalea-rs/azalea?rev=41a9ae6aaff77646c08c64ac1334a8cc6081c24f",
    );
    manifest.assets_compatibility = Some("26.2".to_owned());
    manifest
}
use azalea_block::{BlockState as AzaleaBlockState, BlockTrait};
use azalea_buf::{AzBuf, AzBufVar, BufReadError};
use azalea_chat::{FormattedText, numbers::NumberFormat};
use azalea_core::{
    bitset::BitSet,
    direction::Direction as AzDirection,
    objectives::ObjectiveCriteria,
    position::{BlockPos as AzBlockPos, Vec3 as AzVec3},
    registry_holder::{RegistryHolder, dimension_type::DimensionKindElement},
};
use azalea_crypto::offline;
use azalea_entity::{HumanoidArm, LookDirection};
use azalea_protocol::packets::game::c_animate::AnimationAction;
use azalea_protocol::packets::game::c_light_update::ClientboundLightUpdatePacketData;
pub use azalea_protocol::packets::game::c_move_minecart_along_track::MinecartStep;
use azalea_protocol::{
    address::ResolvableAddr,
    common::{
        client_information::ClientInformation,
        debug_subscription::{
            DebugSubscriptionEvent, DebugSubscriptionUpdate, MinecraftPath, MinecraftPathNode,
        },
        movements::MoveFlags,
    },
    connect::{Connection, ConnectionError, WriteConnection},
    packets::{
        ClientIntention, PROTOCOL_VERSION, ProtocolPacket, VERSION_NAME,
        config::{self, ClientboundConfigPacket, ServerboundConfigPacket, *},
        game::{self, ClientboundGamePacket, ServerboundGamePacket, *},
        handshake::{ClientboundHandshakePacket, ServerboundHandshakePacket, ServerboundIntention},
        login::{self, ClientboundLoginPacket, ServerboundLoginPacket, *},
    },
    read::ReadPacketError,
};
use azalea_registry::{Registry, builtin::ParticleKind, identifier::Identifier};
use azalea_world::{Chunk as AzaleaChunk, palette::Palette as AzaleaPalette};
use base64::Engine;
use crossbeam_channel::{Receiver, Sender};
use el_core::resource_location::ResourceLocation;
use el_core::{block_state::BlockState as LocalBlockState, registry::BuiltInRegistries};
use el_math::block_pos::BlockPos;
use el_nbt::Tag;
use el_protocol_backend::{
    BackendAdvancementCriterionProgress, BackendAdvancementDefinition, BackendAdvancementDisplay,
    BackendAdvancementIcon, BackendAdvancementProgress, BackendAwardStats, BackendBlockPos,
    BackendChatClickAction, BackendChatClickSpan, BackendChatHoverAction, BackendCommandNode,
    BackendCommandNodeType, BackendCommandNumberKind, BackendCommandParser,
    BackendCommandStringKind, BackendCustomNameUpdate, BackendDebugSubscriptionEvent,
    BackendDebugSubscriptionUpdate, BackendEndCrystalBeamTargetUpdate, BackendEntityPose,
    BackendParticle, BackendParticleSpawn, BackendPlayerInfoAddPlayer,
    BackendPlayerInfoDisplayNameUpdate, BackendPlayerInfoUpdateEntry, BackendRotations,
    BackendScoreNumberFormat, BackendSeenAdvancementsAction, BackendSleepingPosUpdate,
    BackendStatValue, BackendUpdateAdvancements, BackendWaypoint, BackendWaypointData,
    BackendWaypointIcon, BackendWaypointIdentifier, diagnostics, resource_pack_action,
};
use el_world::{
    PalettedContainer,
    chunk::{BlockEntityData, Chunk, DataLayer, HeightmapType, SECTION_VOLUME},
};
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    io::{self, Cursor},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
const VERBOSE_PROTOCOL_LOGS: bool = false;
const EVENT_QUEUE_CAPACITY: usize = 8192;
const PACKET_QUEUE_CAPACITY: usize = 8192;
const ELTHARN_NOISY_DEBUG_ENV: &str = "ELTHARN_NOISY_DEBUG";
const ELTHARN_CHEST_DEBUG_ENV: &str = "ELTHARN_CHEST_DEBUG";
const MENU_CLICK_PACKET_PROBE_MS: u64 = 12_000;
const PLAY_BUNDLE_SIZE_LIMIT: usize = 4096;
type SharedGameWriter = Arc<tokio::sync::Mutex<Option<WriteConnection<ServerboundGamePacket>>>>;
#[derive(Clone, Debug, Default)]
struct EnchantmentTooltipRegistry {
    definitions: BTreeMap<String, EnchantmentTooltipDefinition>,
    tooltip_order_protocol_ids: Vec<i32>,
    curse_protocol_ids: HashSet<i32>,
    tooltip_order: Vec<String>,
    curses: HashSet<String>,
    has_synced_tooltip_order: bool,
    has_synced_curse_tag: bool,
}
#[derive(Clone, Debug)]
struct EnchantmentTooltipDefinition {
    description: serde_json::Value,
    max_level: i32,
    slots: serde_json::Value,
    attribute_effects: serde_json::Value,
}
impl EnchantmentTooltipRegistry {
    fn capture_registry_entries(
        &mut self,
        entries: &[(
            Identifier,
            Option<azalea_protocol::simdnbt::owned::NbtCompound>,
        )],
    ) {
        for (id, entry) in entries {
            let key = id.to_string();
            let Some(entry) = entry else {
                self.definitions.remove(&key);
                continue;
            };
            let Some(description) = enchantment_description_json(entry) else {
                continue;
            };
            let Some(max_level) = entry.int("max_level") else {
                continue;
            };
            let presentation = map_dialog_compound(entry);
            self.definitions.insert(
                key,
                EnchantmentTooltipDefinition {
                    description,
                    max_level,
                    slots: presentation
                        .as_ref()
                        .and_then(|entry| entry.get("slots").cloned())
                        .unwrap_or_else(|| serde_json::json!([])),
                    attribute_effects: presentation
                        .as_ref()
                        .and_then(|entry| {
                            entry
                                .get("effects")
                                .and_then(|effects| effects.get("minecraft:attributes"))
                                .cloned()
                        })
                        .unwrap_or_else(|| serde_json::json!([])),
                },
            );
        }
    }
    fn capture_tags(&mut self, tags: &azalea_protocol::common::tags::TagMap) {
        let registry = Identifier::new("minecraft:enchantment");
        let Some(tags) = tags.get(&registry) else {
            return;
        };
        self.tooltip_order_protocol_ids.clear();
        self.curse_protocol_ids.clear();
        self.tooltip_order.clear();
        self.curses.clear();
        self.has_synced_tooltip_order = false;
        self.has_synced_curse_tag = false;
        for tag in tags {
            match tag.name.to_string().as_str() {
                "minecraft:tooltip_order" => {
                    self.tooltip_order_protocol_ids = tag.elements.clone();
                    self.has_synced_tooltip_order = true;
                }
                "minecraft:curse" => {
                    self.curse_protocol_ids = tag.elements.iter().copied().collect();
                    self.has_synced_curse_tag = true;
                }
                _ => {}
            }
        }
    }
    fn resolve_tag_ids(&mut self, registries: &RegistryHolder) {
        let enchantment_registry = Identifier::new("minecraft:enchantment");
        if self.has_synced_tooltip_order {
            self.tooltip_order = self
                .tooltip_order_protocol_ids
                .iter()
                .filter_map(|id| u32::try_from(*id).ok())
                .filter_map(|id| {
                    registries
                        .protocol_id_to_identifier(enchantment_registry.clone(), id)
                        .map(ToString::to_string)
                })
                .collect();
        }
        if self.has_synced_curse_tag {
            self.curses = self
                .curse_protocol_ids
                .iter()
                .filter_map(|id| u32::try_from(*id).ok())
                .filter_map(|id| {
                    registries
                        .protocol_id_to_identifier(enchantment_registry.clone(), id)
                        .map(ToString::to_string)
                })
                .collect();
        }
    }
    fn item_component_json(&self, enchantment_ids: &HashSet<String>) -> Option<serde_json::Value> {
        if enchantment_ids.is_empty() {
            return None;
        }
        let definitions = self
            .definitions
            .iter()
            .filter(|(id, _)| enchantment_ids.contains(*id))
            .map(|(id, definition)| {
                (
                    id.clone(),
                    serde_json::json!(
                        { "description" : definition.description.clone(), "max_level" :
                        definition.max_level, "slots" : definition.slots.clone(),
                        "attribute_effects" : definition.attribute_effects.clone(), }
                    ),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        let tooltip_order = self
            .tooltip_order
            .iter()
            .filter(|id| enchantment_ids.contains(*id))
            .cloned()
            .collect::<Vec<_>>();
        let curses = self
            .curses
            .iter()
            .filter(|id| enchantment_ids.contains(*id))
            .cloned()
            .collect::<Vec<_>>();
        if definitions.is_empty() && !self.has_synced_tooltip_order && !self.has_synced_curse_tag {
            return None;
        }
        Some(serde_json::json!(
            { "definitions" : definitions, "tooltip_order" : tooltip_order, "curses"
            : curses, "has_tooltip_order" : self.has_synced_tooltip_order,
            "has_curse_tag" : self.has_synced_curse_tag, }
        ))
    }
}
fn enchantment_description_json(
    entry: &azalea_protocol::simdnbt::owned::NbtCompound,
) -> Option<serde_json::Value> {
    let mut encoded = Vec::new();
    entry.write(&mut encoded);
    let base_compound =
        azalea_protocol::simdnbt::borrow::read_compound(&mut Cursor::new(encoded.as_slice()))
            .ok()?;
    let compound = azalea_protocol::simdnbt::borrow::NbtCompound::from(&base_compound);
    let description = compound.get("description")?;
    let component =
        <FormattedText as azalea_protocol::simdnbt::FromNbtTag>::from_nbt_tag(description)?;
    serde_json::to_value(component).ok()
}
const PINNED_AZALEA_SERVERBOUND_GAME_PACKET_NAMES: [&str; 66] = [
    "accept_teleportation",
    "block_entity_tag_query",
    "bundle_item_selected",
    "change_difficulty",
    "change_game_mode",
    "chat_ack",
    "chat_command",
    "chat_command_signed",
    "chat",
    "chat_session_update",
    "chunk_batch_received",
    "client_command",
    "client_tick_end",
    "client_information",
    "command_suggestion",
    "configuration_acknowledged",
    "container_button_click",
    "container_click",
    "container_close",
    "container_slot_state_changed",
    "cookie_response",
    "custom_payload",
    "debug_subscription_request",
    "edit_book",
    "entity_tag_query",
    "interact",
    "jigsaw_generate",
    "keep_alive",
    "lock_difficulty",
    "move_player_pos",
    "move_player_pos_rot",
    "move_player_rot",
    "move_player_status_only",
    "move_vehicle",
    "paddle_boat",
    "pick_item_from_block",
    "pick_item_from_entity",
    "ping_request",
    "place_recipe",
    "player_abilities",
    "player_action",
    "player_command",
    "player_input",
    "player_loaded",
    "pong",
    "recipe_book_change_settings",
    "recipe_book_seen_recipe",
    "rename_item",
    "resource_pack",
    "seen_advancements",
    "select_trade",
    "set_beacon",
    "set_carried_item",
    "set_command_block",
    "set_command_minecart",
    "set_creative_mode_slot",
    "set_jigsaw_block",
    "set_structure_block",
    "set_test_block",
    "sign_update",
    "swing",
    "teleport_to_entity",
    "test_instance_block_action",
    "use_item_on",
    "use_item",
    "custom_click_action",
];
fn java_26_2_serverbound_game_packet_id(packet: &ServerboundGamePacket) -> io::Result<u32> {
    let generated_id = packet.id();
    let expected_name = PINNED_AZALEA_SERVERBOUND_GAME_PACKET_NAMES
        .get(generated_id as usize)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "unsupported pinned Azalea serverbound game packet id {generated_id} ({})",
                    packet.name()
                ),
            )
        })?;
    if packet.name() != *expected_name {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "pinned Azalea serverbound registry drift at id {generated_id}: expected {expected_name}, got {}",
                packet.name()
            ),
        ));
    }
    Ok(generated_id)
}
fn encode_serverbound_game_packet_java_26_2(packet: &ServerboundGamePacket) -> io::Result<Vec<u8>> {
    let generated_id = packet.id();
    let java_id = java_26_2_serverbound_game_packet_id(packet)?;
    let payload = azalea_protocol::write::serialize_packet(packet)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?
        .into_vec();
    let mut generated_prefix = Vec::with_capacity(5);
    generated_id.azalea_write_var(&mut generated_prefix)?;
    if !payload.starts_with(&generated_prefix) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Azalea serialized {} without its generated packet id {generated_id}",
                packet.name()
            ),
        ));
    }
    debug_assert_eq!(generated_id, java_id);
    Ok(payload)
}
async fn write_serverbound_game_packet_java_26_2(
    writer: &mut WriteConnection<ServerboundGamePacket>,
    packet: &ServerboundGamePacket,
) -> io::Result<()> {
    let payload = encode_serverbound_game_packet_java_26_2(packet)?;
    writer.raw.write(&payload).await
}
fn encode_serverbound_attack(entity_id: i32, using_secondary_action: bool) -> io::Result<Vec<u8>> {
    let mut payload = Vec::with_capacity(8);
    25_u32.azalea_write_var(&mut payload)?;
    entity_id.azalea_write_var(&mut payload)?;
    1_u32.azalea_write_var(&mut payload)?;
    using_secondary_action.azalea_write(&mut payload)?;
    Ok(payload)
}
fn encode_serverbound_teleport_to_entity(target_uuid: uuid::Uuid) -> io::Result<Vec<u8>> {
    let mut payload = Vec::with_capacity(19);
    61_u32.azalea_write_var(&mut payload)?;
    target_uuid.azalea_write(&mut payload)?;
    Ok(payload)
}
fn encode_serverbound_interact(
    entity_id: i32,
    hand: u8,
    relative_x: f32,
    relative_y: f32,
    relative_z: f32,
    using_secondary_action: bool,
) -> io::Result<Vec<u8>> {
    let mut payload = Vec::with_capacity(16);
    25_u32.azalea_write_var(&mut payload)?;
    entity_id.azalea_write_var(&mut payload)?;
    2_u32.azalea_write_var(&mut payload)?;
    relative_x.azalea_write(&mut payload)?;
    relative_y.azalea_write(&mut payload)?;
    relative_z.azalea_write(&mut payload)?;
    let hand_id = if hand == 1 { 1_u32 } else { 0_u32 };
    hand_id.azalea_write_var(&mut payload)?;
    using_secondary_action.azalea_write(&mut payload)?;
    Ok(payload)
}
fn encode_serverbound_interact_no_location(
    entity_id: i32,
    hand: u8,
    using_secondary_action: bool,
) -> io::Result<Vec<u8>> {
    let mut payload = Vec::with_capacity(8);
    25_u32.azalea_write_var(&mut payload)?;
    entity_id.azalea_write_var(&mut payload)?;
    0_u32.azalea_write_var(&mut payload)?;
    let hand_id = if hand == 1 { 1_u32 } else { 0_u32 };
    hand_id.azalea_write_var(&mut payload)?;
    using_secondary_action.azalea_write(&mut payload)?;
    Ok(payload)
}
fn is_supported_resource_pack_url(value: &str) -> bool {
    url::Url::parse(value)
        .map(|url| matches!(url.scheme(), "http" | "https"))
        .unwrap_or(false)
}
fn encode_serverbound_resource_pack_status(
    packet_id: u32,
    id: uuid::Uuid,
    action: u8,
) -> io::Result<Vec<u8>> {
    let mut payload = Vec::with_capacity(24);
    packet_id.azalea_write_var(&mut payload)?;
    id.azalea_write(&mut payload)?;
    (resource_pack_action::canonical(action) as i32).azalea_write_var(&mut payload)?;
    Ok(payload)
}
fn serverbound_game_resource_pack_packet_id(id: uuid::Uuid) -> io::Result<u32> {
    let packet = ServerboundGamePacket::ResourcePack(game::ServerboundResourcePack {
        id,
        action: game::s_resource_pack::Action::SuccessfullyLoaded,
    });
    java_26_2_serverbound_game_packet_id(&packet)
}
fn generated_serverbound_move_vehicle_packet(
    x: f64,
    y: f64,
    z: f64,
    yaw_degrees: f32,
    pitch_degrees: f32,
) -> ServerboundGamePacket {
    ServerboundGamePacket::MoveVehicle(
        azalea_protocol::packets::game::s_move_vehicle::ServerboundMoveVehicle {
            pos: AzVec3::new(x, y, z),
            look_direction: LookDirection::new(yaw_degrees, pitch_degrees),
        },
    )
}
fn encode_serverbound_move_vehicle(
    x: f64,
    y: f64,
    z: f64,
    yaw_degrees: f32,
    pitch_degrees: f32,
    on_ground: bool,
) -> io::Result<Vec<u8>> {
    let packet = generated_serverbound_move_vehicle_packet(x, y, z, yaw_degrees, pitch_degrees);
    let mut payload = encode_serverbound_game_packet_java_26_2(&packet)?;
    on_ground.azalea_write(&mut payload)?;
    Ok(payload)
}
fn generated_serverbound_player_action_packet(
    action: PlayerActionKind,
    x: i32,
    y: i32,
    z: i32,
    direction: u8,
    seq: u32,
) -> ServerboundGamePacket {
    ServerboundGamePacket::PlayerAction(ServerboundPlayerAction {
        action: match action {
            PlayerActionKind::StartDestroyBlock => s_player_action::Action::StartDestroyBlock,
            PlayerActionKind::AbortDestroyBlock => s_player_action::Action::AbortDestroyBlock,
            PlayerActionKind::StopDestroyBlock => s_player_action::Action::StopDestroyBlock,
            PlayerActionKind::DropAllItems => s_player_action::Action::DropAllItems,
            PlayerActionKind::DropItem => s_player_action::Action::DropItem,
            PlayerActionKind::ReleaseUseItem => s_player_action::Action::ReleaseUseItem,
            PlayerActionKind::SwapItemWithOffhand => s_player_action::Action::SwapItemWithOffhand,
            PlayerActionKind::Stab => {
                unreachable!("STAB is written from Java's raw packet body")
            }
        },
        pos: AzBlockPos::new(x, y, z),
        direction: az_direction(direction),
        seq,
    })
}
const SERVERBOUND_PLAYER_ACTION_STAB_ORDINAL: u8 = 7;
fn serverbound_game_player_action_packet_id() -> io::Result<u32> {
    let packet =
        generated_serverbound_player_action_packet(PlayerActionKind::ReleaseUseItem, 0, 0, 0, 0, 0);
    java_26_2_serverbound_game_packet_id(&packet)
}
fn encode_serverbound_player_action_stab(packet_id: u32) -> io::Result<Vec<u8>> {
    let mut payload = Vec::with_capacity(16);
    packet_id.azalea_write_var(&mut payload)?;
    (SERVERBOUND_PLAYER_ACTION_STAB_ORDINAL as i32).azalea_write_var(&mut payload)?;
    payload.extend_from_slice(&0_i64.to_be_bytes());
    payload.push(0);
    0_i32.azalea_write_var(&mut payload)?;
    Ok(payload)
}
const CUSTOM_CLICK_ACTION_MAX_NBT_BYTES: usize = 65_536;
fn custom_click_payload_metadata(payload: Option<&[u8]>) -> (&'static str, usize, usize) {
    match payload {
        Some(payload) => ("opaque_nbt", payload.len(), payload.len()),
        None => ("absent_tag_end", 0, 1),
    }
}
fn encode_serverbound_custom_click_action(
    packet_id: u32,
    id: &str,
    payload: Option<&[u8]>,
) -> io::Result<Vec<u8>> {
    let nbt = payload.unwrap_or(&[0]);
    if nbt.is_empty() || nbt.len() > CUSTOM_CLICK_ACTION_MAX_NBT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "custom click action NBT body must contain 1..={CUSTOM_CLICK_ACTION_MAX_NBT_BYTES} bytes"
            ),
        ));
    }
    let mut encoded = Vec::with_capacity(id.len() + nbt.len() + 8);
    packet_id.azalea_write_var(&mut encoded)?;
    id.to_owned().azalea_write(&mut encoded)?;
    (nbt.len() as i32).azalea_write_var(&mut encoded)?;
    encoded.extend_from_slice(nbt);
    Ok(encoded)
}
fn serverbound_game_custom_click_action_packet_id() -> io::Result<u32> {
    let packet = ServerboundGamePacket::CustomClickAction(game::ServerboundCustomClickAction {
        id: Identifier::new("minecraft:eltharn"),
        payload: azalea_protocol::simdnbt::owned::Nbt::None,
    });
    java_26_2_serverbound_game_packet_id(&packet)
}
fn serverbound_config_resource_pack_packet_id(id: uuid::Uuid) -> u32 {
    ServerboundConfigPacket::ResourcePack(config::ServerboundResourcePack {
        id,
        action: config::s_resource_pack::Action::SuccessfullyLoaded,
    })
    .id()
}
async fn write_config_resource_pack_status(
    conn: &mut Connection<ClientboundConfigPacket, ServerboundConfigPacket>,
    id: uuid::Uuid,
    action: u8,
) -> Result<(), NetworkError> {
    let payload = encode_serverbound_resource_pack_status(
        serverbound_config_resource_pack_packet_id(id),
        id,
        action,
    )?;
    conn.writer.raw.write(&payload).await?;
    Ok(())
}
fn noisy_debug_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var(ELTHARN_NOISY_DEBUG_ENV)
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
            .unwrap_or(false)
    })
}
fn skin_debug_enabled() -> bool {
    std::env::var("ELTHARN_SKIN_DEBUG")
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false)
        || noisy_debug_enabled()
}
fn skin_debug(detail: impl std::fmt::Display) {
    if skin_debug_enabled() {
        diagnostics::write_line(&format!("[SkinDebug][backend] {detail}"));
    }
}
fn skin_url_diagnostic(url: &str) -> String {
    match url::Url::parse(url) {
        Ok(parsed) => {
            format!(
                "url_valid=true scheme={} query_present={} fragment_present={} url_len={}",
                parsed.scheme(),
                parsed.query().is_some(),
                parsed.fragment().is_some(),
                url.len()
            )
        }
        Err(_) => format!("url_valid=false url_len={}", url.len()),
    }
}
fn should_print_console_line(line: &str) -> bool {
    if noisy_debug_enabled()
        || std::env::var("ELTHARN_VERBOSE_LOGS")
            .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
            .unwrap_or(false)
        || matches!(
            std::env::var("ELTHARN_LOGS").as_deref(),
            Ok("all" | "trace" | "debug" | "info")
        )
    {
        return true;
    }
    if std::env::var("ELTHARN_DIAGNOSTICS")
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false)
        && (line.starts_with("[MovementDebug]")
            || line.starts_with("[MoveDebug]")
            || line.starts_with("[SlideDebug]")
            || line.starts_with("[NetDebug]"))
    {
        return true;
    }
    if !matches!(
        std::env::var("ELTHARN_LOGS").as_deref(),
        Ok("error" | "warn")
    ) {
        return false;
    }
    let lower = line.to_ascii_lowercase();
    line.starts_with("[ERROR]")
        || lower.contains("error")
        || lower.contains("failed")
        || lower.contains("fatal")
        || lower.contains("disconnect")
        || lower.contains("kicked")
        || lower.contains("kick reason")
}
macro_rules! eprintln {
    ($($arg:tt)*) => {
        { let line = format!($($arg)*); if should_print_console_line(& line) {
        std::eprintln!("{}", line); } }
    };
}
fn game_event_id(event: c_game_event::EventType) -> u8 {
    match event {
        c_game_event::EventType::NoRespawnBlockAvailable => 0,
        c_game_event::EventType::StartRaining => 1,
        c_game_event::EventType::StopRaining => 2,
        c_game_event::EventType::ChangeGameMode => 3,
        c_game_event::EventType::WinGame => 4,
        c_game_event::EventType::DemoEvent => 5,
        c_game_event::EventType::ArrowHitPlayer => 6,
        c_game_event::EventType::RainLevelChange => 7,
        c_game_event::EventType::ThunderLevelChange => 8,
        c_game_event::EventType::PufferFishSting => 9,
        c_game_event::EventType::GuardianElderEffect => 10,
        c_game_event::EventType::ImmediateRespawn => 11,
        c_game_event::EventType::LimitedCrafting => 12,
        c_game_event::EventType::WaitForLevelChunks => 13,
    }
}
fn describe_local_stack(stack: &el_core::ItemStack) -> String {
    if stack.is_empty() {
        "minecraft:airx0".to_string()
    } else {
        format!("{}x{}", stack.item.location, stack.count)
    }
}
fn describe_local_changed_slots(
    changed_slots: &std::collections::BTreeMap<i16, el_core::ItemStack>,
) -> String {
    if changed_slots.is_empty() {
        return "[]".to_string();
    }
    let entries = changed_slots
        .iter()
        .map(|(slot, stack)| format!("{slot}={}", describe_local_stack(stack)))
        .collect::<Vec<_>>();
    format!("[{}]", entries.join(","))
}
fn describe_azalea_stack(stack: &azalea_inventory::ItemStack) -> String {
    match stack {
        azalea_inventory::ItemStack::Empty => "Empty".to_string(),
        azalea_inventory::ItemStack::Present(data) => {
            let components = data
                .component_patch
                .iter()
                .map(|(kind, value)| {
                    if value.is_some() {
                        format!("{kind:?}")
                    } else {
                        format!("-{kind:?}")
                    }
                })
                .collect::<Vec<_>>();
            format!(
                "{:?}x{} comps={} [{}]",
                data.kind,
                data.count,
                components.len(),
                components.join(",")
            )
        }
    }
}
fn describe_raw_changed_slots(
    changed_slots: &[(u16, azalea_inventory::ItemStack, &'static str)],
) -> String {
    if changed_slots.is_empty() {
        return "[]".to_string();
    }
    let entries = changed_slots
        .iter()
        .map(|(slot, stack, source)| format!("{slot}={source}:{}", describe_azalea_stack(stack)))
        .collect::<Vec<_>>();
    format!("[{}]", entries.join(","))
}
fn describe_hashed_stack(
    stack: &azalea_protocol::packets::game::s_container_click::HashedStack,
) -> String {
    match &stack.0 {
        None => "EmptyHash".to_string(),
        Some(actual) => {
            format!(
                "{:?}x{} hash_added={} hash_removed={}",
                actual.kind,
                actual.count,
                actual.components.added_components.len(),
                actual.components.removed_components.len()
            )
        }
    }
}
fn describe_hashed_changed_slots(
    changed_slots: &indexmap::IndexMap<
        u16,
        azalea_protocol::packets::game::s_container_click::HashedStack,
    >,
) -> String {
    if changed_slots.is_empty() {
        return "[]".to_string();
    }
    let entries = changed_slots
        .iter()
        .map(|(slot, stack)| format!("{slot}={}", describe_hashed_stack(stack)))
        .collect::<Vec<_>>();
    format!("[{}]", entries.join(","))
}
#[derive(Default)]
struct MenuClickPacketProbe {
    active_until: Option<Instant>,
    generation: u64,
}
impl MenuClickPacketProbe {
    fn begin(&mut self, packet: &el_protocol::packets::game::ServerboundContainerClickPacket) {
        self.generation = self.generation.wrapping_add(1);
        self.active_until =
            Some(Instant::now() + Duration::from_millis(MENU_CLICK_PACKET_PROBE_MS));
        eprintln!(
            "[MenuTrace] PROBE_START gen={} duration_ms={} container={} state={} slot={} button={} input={:?} changed={} changed_slots={} carried={}",
            self.generation,
            MENU_CLICK_PACKET_PROBE_MS,
            packet.container_id,
            packet.state_id,
            packet.slot_num,
            packet.button_num,
            packet.container_input,
            packet.changed_slots.len(),
            describe_local_changed_slots(&packet.changed_slots),
            describe_local_stack(&packet.carried_item)
        );
    }
    fn active_generation(&self) -> Option<u64> {
        match self.active_until {
            Some(deadline) if Instant::now() <= deadline => Some(self.generation),
            _ => None,
        }
    }
}
struct ChunkBatchSizeCalculator {
    aggregated_nanos_per_chunk: f64,
    old_samples_weight: i32,
    chunk_batch_start_time: Instant,
}
impl ChunkBatchSizeCalculator {
    fn new() -> Self {
        Self {
            aggregated_nanos_per_chunk: 2_000_000.0,
            old_samples_weight: 1,
            chunk_batch_start_time: Instant::now(),
        }
    }
    fn on_batch_start(&mut self) {
        self.chunk_batch_start_time = Instant::now();
    }
    fn on_batch_finished(&mut self, batch_size: u32) {
        if batch_size == 0 {
            return;
        }
        let batch_duration = self.chunk_batch_start_time.elapsed().as_nanos() as f64;
        let nanos_per_chunk = batch_duration / f64::from(batch_size);
        let clamped_nanos_per_chunk = nanos_per_chunk.clamp(
            self.aggregated_nanos_per_chunk / 3.0,
            self.aggregated_nanos_per_chunk * 3.0,
        );
        self.aggregated_nanos_per_chunk = (self.aggregated_nanos_per_chunk
            * f64::from(self.old_samples_weight)
            + clamped_nanos_per_chunk)
            / f64::from(self.old_samples_weight + 1);
        self.old_samples_weight = (self.old_samples_weight + 1).min(49);
    }
    fn desired_chunks_per_tick(&self) -> f32 {
        (7_000_000.0 / self.aggregated_nanos_per_chunk) as f32
    }
}
pub const VANILLA_PROTOCOL_VERSION: i32 = PROTOCOL_VERSION;
pub const VANILLA_VERSION_NAME: &str = VERSION_NAME;
pub const SERVER_PACK_STATUS_PROMPT: u8 = 0;
pub const SERVER_PACK_STATUS_ENABLED: u8 = 1;
pub const SERVER_PACK_STATUS_DISABLED: u8 = 2;
pub use azalea_core::game_type::GameMode;
static BLOCK_STATE_FALLBACK_LOG: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static BIOME_FALLBACK_LOG: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
fn log_biome_fallback_once(protocol_id: u32, context: &str) {
    let key = format!("{context}:{protocol_id}");
    let log = BIOME_FALLBACK_LOG.get_or_init(|| Mutex::new(HashSet::new()));
    let Ok(mut seen) = log.lock() else {
        return;
    };
    if !seen.insert(key) {
        return;
    }
    if VERBOSE_PROTOCOL_LOGS {
        eprintln!(
            "[AzaleaProtocol] Unknown biome protocol id {protocol_id} ({context}), falling back to minecraft:plains"
        );
    }
    log::warn!(
        "[AzaleaProtocol] Unknown biome protocol id {protocol_id} ({context}), falling back to minecraft:plains"
    );
}
#[derive(Debug, Clone)]
pub struct ConnectArgs {
    pub server: String,
    pub username: String,
    pub language: String,
    pub uuid: uuid::Uuid,
    pub view_distance: u8,
    pub main_hand_left: bool,
    pub allows_listing: bool,
    pub server_pack_status: u8,
    pub model_customisation: u8,
    pub server_cookies: HashMap<String, Vec<u8>>,
}
impl ConnectArgs {
    pub fn offline(server: impl Into<String>, username: impl Into<String>) -> Self {
        let username = username.into();
        Self {
            server: server.into(),
            uuid: offline_uuid(&username),
            username,
            language: "en_us".to_owned(),
            view_distance: 12,
            main_hand_left: false,
            allows_listing: true,
            server_pack_status: SERVER_PACK_STATUS_PROMPT,
            model_customisation: 0x7F,
            server_cookies: HashMap::new(),
        }
    }
}
#[derive(Debug)]
pub struct ConnectionHandle {
    pub event_rx: Receiver<NetworkEvent>,
    pub packet_tx: PacketSender,
    task: Option<thread::JoinHandle<()>>,
}
impl Drop for ConnectionHandle {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            let _ = task.thread().id();
        }
    }
}
#[derive(Debug, Clone)]
pub struct PacketSender {
    tx: tokio::sync::mpsc::Sender<OutboundPacket>,
}
#[derive(Debug, Clone)]
struct EventSender {
    tx: Sender<NetworkEvent>,
    queue_warned: Arc<AtomicBool>,
}
impl EventSender {
    fn new(tx: Sender<NetworkEvent>) -> Self {
        Self {
            tx,
            queue_warned: Arc::new(AtomicBool::new(false)),
        }
    }
    fn send(
        &self,
        event: NetworkEvent,
    ) -> Result<(), crossbeam_channel::TrySendError<NetworkEvent>> {
        match self.tx.try_send(event) {
            Ok(()) => Ok(()),
            Err(err) => {
                if !self.queue_warned.swap(true, Ordering::Relaxed) {
                    match &err {
                        crossbeam_channel::TrySendError::Full(_) => {
                            eprintln!(
                                "[AzaleaProtocol][EVENTS] event queue full at {EVENT_QUEUE_CAPACITY} entries; dropping events until it drains"
                            )
                        }
                        crossbeam_channel::TrySendError::Disconnected(_) => {
                            eprintln!(
                                "[AzaleaProtocol][EVENTS] event receiver disconnected; dropping events"
                            )
                        }
                    }
                }
                Err(err)
            }
        }
    }
}
#[derive(Default)]
struct RemoteContainerMirror {
    slots: HashMap<i32, Vec<azalea_inventory::ItemStack>>,
    carried: HashMap<i32, azalea_inventory::ItemStack>,
    active_container_id: Option<i32>,
}
impl RemoteContainerMirror {
    fn open(&mut self, container_id: i32) {
        if let Some(previous) = self.active_container_id.replace(container_id) {
            if previous != container_id {
                self.slots.remove(&previous);
                self.carried.remove(&previous);
            }
        }
        self.slots.remove(&container_id);
        self.carried.remove(&container_id);
    }
    fn is_current_container(&self, container_id: i32) -> bool {
        container_id == 0 || self.active_container_id == Some(container_id)
    }
    fn set_content(
        &mut self,
        container_id: i32,
        items: &[azalea_inventory::ItemStack],
        carried_item: &azalea_inventory::ItemStack,
    ) {
        self.slots.insert(container_id, items.to_vec());
        self.carried.insert(container_id, carried_item.clone());
    }
    fn set_slot(&mut self, container_id: i32, slot: u16, item_stack: &azalea_inventory::ItemStack) {
        let slots = self.slots.entry(container_id).or_default();
        let slot = usize::from(slot);
        if slot >= slots.len() {
            slots.resize(slot + 1, azalea_inventory::ItemStack::Empty);
        }
        slots[slot] = item_stack.clone();
    }
    fn set_player_inventory_slot(
        &mut self,
        player_slot: u16,
        item_stack: &azalea_inventory::ItemStack,
    ) {
        if let Some(menu_slot) = inventory_menu_slot_for_player_slot(usize::from(player_slot)) {
            self.set_slot(0, menu_slot, item_stack);
        }
    }
    fn close(&mut self, container_id: i32) {
        self.slots.remove(&container_id);
        self.carried.remove(&container_id);
        if self.active_container_id == Some(container_id) {
            self.active_container_id = None;
        }
    }
    fn set_cursor_item(&mut self, item_stack: &azalea_inventory::ItemStack) {
        let container_id = self.active_container_id.unwrap_or(0);
        self.carried.insert(container_id, item_stack.clone());
    }
    fn raw_stack_for_outgoing_click_with_source(
        &self,
        container_id: i32,
        slot: Option<i16>,
        local: &el_core::ItemStack,
        registries: &RegistryHolder,
    ) -> (azalea_inventory::ItemStack, &'static str) {
        if local.is_empty() {
            return (azalea_inventory::ItemStack::Empty, "local_empty");
        }
        if let Some(raw_carried) = self.carried.get(&container_id)
            && azalea_stack_matches_local_ignoring_count(raw_carried, local, registries)
        {
            return (
                azalea_stack_with_local_count(raw_carried, local),
                "mirror_carried_exact",
            );
        }
        if let Some(slot) = slot.and_then(|slot| u16::try_from(slot).ok())
            && let Some(raw_slot) = self
                .slots
                .get(&container_id)
                .and_then(|slots| slots.get(usize::from(slot)))
            && azalea_stack_matches_local_ignoring_count(raw_slot, local, registries)
        {
            return (
                azalea_stack_with_local_count(raw_slot, local),
                "mirror_slot_exact",
            );
        }
        if let Some(raw_slot) = self
            .slots
            .get(&container_id)
            .into_iter()
            .flat_map(|slots| slots.iter())
            .find(|raw| azalea_stack_matches_local_ignoring_count(raw, local, registries))
        {
            return (raw_slot.clone(), "mirror_any_exact");
        }
        if container_id != 0
            && let Some(raw_slot) = self
                .slots
                .get(&0)
                .into_iter()
                .flat_map(|slots| slots.iter())
                .find(|raw| azalea_stack_matches_local_ignoring_count(raw, local, registries))
        {
            return (raw_slot.clone(), "mirror_inventory_exact");
        }
        if let Some(raw_carried) = self.carried.get(&container_id) {
            if azalea_stack_same_item(raw_carried, local) {
                return (
                    azalea_stack_with_local_count(raw_carried, local),
                    "mirror_carried_same_item",
                );
            }
        }
        if let Some(slot) = slot.and_then(|slot| u16::try_from(slot).ok())
            && let Some(raw_slot) = self
                .slots
                .get(&container_id)
                .and_then(|slots| slots.get(usize::from(slot)))
            && azalea_stack_same_item(raw_slot, local)
        {
            return (
                azalea_stack_with_local_count(raw_slot, local),
                "mirror_slot_same_item",
            );
        }
        if let Some(raw_slot) = self
            .slots
            .get(&container_id)
            .into_iter()
            .flat_map(|slots| slots.iter())
            .find(|raw| azalea_stack_same_item(raw, local))
        {
            return (
                azalea_stack_with_local_count(raw_slot, local),
                "mirror_any_same_item",
            );
        }
        if container_id != 0
            && let Some(raw_slot) = self
                .slots
                .get(&0)
                .into_iter()
                .flat_map(|slots| slots.iter())
                .find(|raw| azalea_stack_same_item(raw, local))
        {
            return (
                azalea_stack_with_local_count(raw_slot, local),
                "mirror_inventory_same_item",
            );
        }
        (azalea_item_stack_from_local(local), "local_fallback")
    }
    fn apply_outgoing_click_prediction(
        &mut self,
        container_id: i32,
        changed_slots: &[(u16, azalea_inventory::ItemStack)],
        carried_item: &azalea_inventory::ItemStack,
    ) {
        for (slot, item_stack) in changed_slots {
            self.set_slot(container_id, *slot, item_stack);
        }
        self.carried.insert(container_id, carried_item.clone());
    }
}
fn inventory_menu_slot_for_player_slot(player_slot: usize) -> Option<u16> {
    match player_slot {
        45 => Some(0),
        41..=44 => u16::try_from(player_slot - 40).ok(),
        36..=39 => u16::try_from(44 - player_slot).ok(),
        9..=35 => u16::try_from(player_slot).ok(),
        0..=8 => u16::try_from(player_slot + 36).ok(),
        40 => Some(45),
        _ => None,
    }
}
impl PacketSender {
    pub fn send_raw_outbound(&self, packet: OutboundPacket) {
        let _ = self.tx.blocking_send(packet);
    }
    pub fn send_resource_pack_status(&self, id: uuid::Uuid, action: u8) {
        let _ = self
            .tx
            .blocking_send(OutboundPacket::ResourcePackStatus { id, action });
    }
    pub fn send_chat(&self, message: impl Into<String>) {
        let _ = self.tx.blocking_send(OutboundPacket::Chat(message.into()));
    }
    pub fn send_command(&self, command: impl Into<String>) {
        let _ = self
            .tx
            .blocking_send(OutboundPacket::Command(command.into()));
    }
    pub fn send_custom_click_action(&self, id: String, payload: Option<Vec<u8>>) {
        let _ = self
            .tx
            .blocking_send(OutboundPacket::CustomClickAction { id, payload });
    }
    pub fn send_custom_payload(&self, channel: String, data: Vec<u8>) -> Result<(), String> {
        el_protocol_backend::payload::validate_custom_payload(&channel, &data, true)?;
        self.tx
            .blocking_send(OutboundPacket::CustomPayload { channel, data })
            .map_err(|_| "outbound connection closed".to_owned())
    }
    pub fn send_command_suggestion(&self, id: u32, command: impl Into<String>) {
        let _ = self.tx.blocking_send(OutboundPacket::CommandSuggestion {
            id,
            command: command.into(),
        });
    }
    pub fn send_player_loaded(&self) {
        let _ = self.tx.blocking_send(OutboundPacket::PlayerLoaded);
    }
    pub fn send_client_tick_end(&self) {
        let _ = self.tx.blocking_send(OutboundPacket::ClientTickEnd);
    }
    pub fn send_pong(&self, id: u32) {
        let _ = self.tx.blocking_send(OutboundPacket::Pong(id));
    }
    pub fn send_perform_respawn(&self) {
        let _ = self.tx.blocking_send(OutboundPacket::PerformRespawn);
    }
    pub fn send_move_pos_rot(
        &self,
        x: f64,
        y: f64,
        z: f64,
        yaw_degrees: f32,
        pitch_degrees: f32,
        on_ground: bool,
        horizontal_collision: bool,
    ) {
        let _ = self.tx.blocking_send(OutboundPacket::MovePosRot {
            x,
            y,
            z,
            yaw_degrees,
            pitch_degrees,
            on_ground,
            horizontal_collision,
        });
    }
    pub fn send_move_pos(
        &self,
        x: f64,
        y: f64,
        z: f64,
        on_ground: bool,
        horizontal_collision: bool,
    ) {
        let _ = self.tx.blocking_send(OutboundPacket::MovePos {
            x,
            y,
            z,
            on_ground,
            horizontal_collision,
        });
    }
    pub fn send_move_rot(
        &self,
        yaw_degrees: f32,
        pitch_degrees: f32,
        on_ground: bool,
        horizontal_collision: bool,
    ) {
        let _ = self.tx.blocking_send(OutboundPacket::MoveRot {
            yaw_degrees,
            pitch_degrees,
            on_ground,
            horizontal_collision,
        });
    }
    pub fn send_move_status_only(&self, on_ground: bool, horizontal_collision: bool) {
        let _ = self.tx.blocking_send(OutboundPacket::MoveStatusOnly {
            on_ground,
            horizontal_collision,
        });
    }
    pub fn send_move_vehicle(
        &self,
        x: f64,
        y: f64,
        z: f64,
        yaw_degrees: f32,
        pitch_degrees: f32,
        on_ground: bool,
    ) {
        let _ = self.tx.blocking_send(OutboundPacket::MoveVehicle {
            x,
            y,
            z,
            yaw_degrees,
            pitch_degrees,
            on_ground,
        });
    }
    pub fn send_paddle_boat(&self, left: bool, right: bool) {
        let _ = self
            .tx
            .blocking_send(OutboundPacket::PaddleBoat { left, right });
    }
    pub fn send_player_input(
        &self,
        forward: bool,
        backward: bool,
        left: bool,
        right: bool,
        jump: bool,
        shift: bool,
        sprint: bool,
    ) {
        let _ = self.tx.blocking_send(OutboundPacket::PlayerInput {
            forward,
            backward,
            left,
            right,
            jump,
            shift,
            sprint,
        });
    }
    pub fn send_player_command(&self, entity_id: i32, action: &str, data: i32) {
        let _ = self.tx.blocking_send(OutboundPacket::PlayerCommand {
            entity_id,
            action: action.to_string(),
            data,
        });
    }
    pub fn send_accept_teleportation(&self, id: u32) {
        let _ = self
            .tx
            .blocking_send(OutboundPacket::AcceptTeleportation(id));
    }
    pub fn send_teleport_confirm(
        &self,
        teleport_id: u32,
        x: f64,
        y: f64,
        z: f64,
        yaw_degrees: f32,
        pitch_degrees: f32,
    ) {
        let _ = self.tx.blocking_send(OutboundPacket::TeleportConfirm {
            teleport_id,
            x,
            y,
            z,
            yaw_degrees,
            pitch_degrees,
        });
    }
    pub fn send_set_carried_item(&self, slot: u16) {
        let _ = self.tx.blocking_send(OutboundPacket::SetCarriedItem(slot));
    }
    pub fn send_start_destroy_block(&self, x: i32, y: i32, z: i32, direction: u8, seq: u32) {
        let _ = self.tx.blocking_send(OutboundPacket::PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            x,
            y,
            z,
            direction,
            seq,
        });
    }
    pub fn send_stop_destroy_block(&self, x: i32, y: i32, z: i32, direction: u8, seq: u32) {
        let _ = self.tx.blocking_send(OutboundPacket::PlayerAction {
            action: PlayerActionKind::StopDestroyBlock,
            x,
            y,
            z,
            direction,
            seq,
        });
    }
    pub fn send_abort_destroy_block(&self, x: i32, y: i32, z: i32, direction: u8, seq: u32) {
        let _ = self.tx.blocking_send(OutboundPacket::PlayerAction {
            action: PlayerActionKind::AbortDestroyBlock,
            x,
            y,
            z,
            direction,
            seq,
        });
    }
    pub fn send_drop_item(&self) {
        let _ = self.tx.blocking_send(OutboundPacket::PlayerAction {
            action: PlayerActionKind::DropItem,
            x: 0,
            y: 0,
            z: 0,
            direction: 0,
            seq: 0,
        });
    }
    pub fn send_drop_all_items(&self) {
        let _ = self.tx.blocking_send(OutboundPacket::PlayerAction {
            action: PlayerActionKind::DropAllItems,
            x: 0,
            y: 0,
            z: 0,
            direction: 0,
            seq: 0,
        });
    }
    pub fn send_release_using_item(&self) {
        let _ = self.tx.blocking_send(OutboundPacket::PlayerAction {
            action: PlayerActionKind::ReleaseUseItem,
            x: 0,
            y: 0,
            z: 0,
            direction: 0,
            seq: 0,
        });
    }
    pub fn send_swap_item_with_offhand(&self) {
        let _ = self.tx.blocking_send(OutboundPacket::PlayerAction {
            action: PlayerActionKind::SwapItemWithOffhand,
            x: 0,
            y: 0,
            z: 0,
            direction: 0,
            seq: 0,
        });
    }
    pub fn send_stab(&self) {
        let _ = self.tx.blocking_send(OutboundPacket::PlayerAction {
            action: PlayerActionKind::Stab,
            x: 0,
            y: 0,
            z: 0,
            direction: 0,
            seq: 0,
        });
    }
    pub fn send_use_item_on(
        &self,
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
    ) {
        eprintln!(
            "[MenuDebug] BACKEND_QUEUE_USE_ITEM_ON hand={} seq={} pos=[{},{},{}] face={} cursor=[{:.3},{:.3},{:.3}] inside={}",
            hand, seq, x, y, z, direction, cursor_x, cursor_y, cursor_z, inside
        );
        let _ = self.tx.blocking_send(OutboundPacket::UseItemOn {
            hand,
            x,
            y,
            z,
            direction,
            cursor_x,
            cursor_y,
            cursor_z,
            inside,
            seq,
        });
    }
    pub fn send_use_item(&self, hand: u8, seq: u32, yaw_degrees: f32, pitch_degrees: f32) {
        eprintln!(
            "[MenuDebug] BACKEND_QUEUE_USE_ITEM hand={} seq={} yaw={:.2} pitch={:.2}",
            hand, seq, yaw_degrees, pitch_degrees
        );
        let _ = self.tx.blocking_send(OutboundPacket::UseItem {
            hand,
            seq,
            yaw_degrees,
            pitch_degrees,
        });
    }
    pub fn send_swing_main_hand(&self) {
        let _ = self.tx.blocking_send(OutboundPacket::SwingMainHand);
    }
    pub fn send_swing_off_hand(&self) {
        let _ = self.tx.blocking_send(OutboundPacket::SwingOffHand);
    }
    pub fn send_attack(&self, entity_id: i32, using_secondary_action: bool) {
        let _ = self.tx.blocking_send(OutboundPacket::Attack {
            entity_id,
            using_secondary_action,
        });
    }
    pub fn send_interact_entity(
        &self,
        entity_id: i32,
        hand: u8,
        relative_x: f32,
        relative_y: f32,
        relative_z: f32,
        using_secondary_action: bool,
    ) {
        let _ = self.tx.blocking_send(OutboundPacket::Interact {
            entity_id,
            hand,
            relative_x,
            relative_y,
            relative_z,
            using_secondary_action,
        });
    }
    pub fn send_interact_no_location(
        &self,
        entity_id: i32,
        hand: u8,
        using_secondary_action: bool,
    ) {
        let _ = self.tx.blocking_send(OutboundPacket::InteractNoLocation {
            entity_id,
            hand,
            using_secondary_action,
        });
    }
    pub fn send_container_click(
        &self,
        packet: el_protocol::packets::game::ServerboundContainerClickPacket,
    ) {
        eprintln!(
            "[MenuDebug] BACKEND_QUEUE_CONTAINER_CLICK container={} state={} slot={} button={} input={:?} changed={} carried={}x{}",
            packet.container_id,
            packet.state_id,
            packet.slot_num,
            packet.button_num,
            packet.container_input,
            packet.changed_slots.len(),
            packet.carried_item.item.location,
            packet.carried_item.count
        );
        let _ = self
            .tx
            .blocking_send(OutboundPacket::ContainerClick(packet));
    }
    pub fn send_set_creative_mode_slot(
        &self,
        packet: el_protocol::packets::game::ServerboundSetCreativeModeSlotPacket,
    ) {
        let _ = self
            .tx
            .blocking_send(OutboundPacket::SetCreativeModeSlot(packet));
    }
    pub fn send_container_close(&self, container_id: i32) {
        let _ = self
            .tx
            .blocking_send(OutboundPacket::ContainerClose(container_id));
    }
    pub fn send_player_abilities(&self, is_flying: bool) {
        let _ = self
            .tx
            .blocking_send(OutboundPacket::PlayerAbilities { is_flying });
    }
    pub fn send_change_game_mode(&self, mode: GameMode) {
        let _ = self.tx.blocking_send(OutboundPacket::ChangeGameMode(mode));
    }
}
#[derive(Debug)]
pub enum OutboundPacket {
    ResourcePackStatus {
        id: uuid::Uuid,
        action: u8,
    },
    Chat(String),
    Command(String),
    CustomClickAction {
        id: String,
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
    Pong(u32),
    PerformRespawn,
    AcceptTeleportation(u32),
    TeleportConfirm {
        teleport_id: u32,
        x: f64,
        y: f64,
        z: f64,
        yaw_degrees: f32,
        pitch_degrees: f32,
    },
    SetCarriedItem(u16),
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
    PaddleBoat {
        left: bool,
        right: bool,
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
    PlayerAction {
        action: PlayerActionKind,
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
        relative_x: f32,
        relative_y: f32,
        relative_z: f32,
        using_secondary_action: bool,
    },
    InteractNoLocation {
        entity_id: i32,
        hand: u8,
        using_secondary_action: bool,
    },
    ContainerClick(el_protocol::packets::game::ServerboundContainerClickPacket),
    SetCreativeModeSlot(el_protocol::packets::game::ServerboundSetCreativeModeSlotPacket),
    ContainerClose(i32),
    PlayerAbilities {
        is_flying: bool,
    },
    ChangeGameMode(GameMode),
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
#[derive(Debug, Clone, Copy)]
pub enum PlayerActionKind {
    StartDestroyBlock,
    AbortDestroyBlock,
    StopDestroyBlock,
    DropAllItems,
    DropItem,
    ReleaseUseItem,
    SwapItemWithOffhand,
    Stab,
}
#[derive(Debug, Clone)]
pub struct BlockUpdateEvent {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub state_id: usize,
}
#[derive(Debug, Clone)]
pub struct LightUpdateEvent {
    pub x: i32,
    pub z: i32,
    pub sky_layers: Vec<(usize, Option<DataLayer>)>,
    pub block_layers: Vec<(usize, Option<DataLayer>)>,
}
#[derive(Debug, Clone)]
pub struct BiomeRenderData {
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
#[derive(Debug, Clone)]
pub struct ChunkBiomeUpdate {
    pub x: i32,
    pub z: i32,
    pub sections: Vec<Vec<ResourceLocation>>,
}
#[derive(Debug, Clone)]
pub struct RemoteEntityEvent {
    pub id: i32,
    pub uuid: uuid::Uuid,
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub head_yaw_degrees: f32,
    pub spawn_data: i32,
    pub falling_block_state_id: Option<usize>,
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub velocity_z: f64,
}
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeSnapshotEvent {
    pub attribute_id: String,
    pub base: f64,
    pub modifiers: Vec<(String, f64, u8)>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RelativeMovementFlags {
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
fn backend_command_parser(parser: &game::c_commands::BrigadierParser) -> BackendCommandParser {
    use game::c_commands::{BrigadierString, EntityParser};
    match parser {
        game::c_commands::BrigadierParser::Bool => BackendCommandParser::Bool,
        game::c_commands::BrigadierParser::Float(bounds) => BackendCommandParser::Number {
            kind: BackendCommandNumberKind::Float,
            min: bounds.min.map(|value| value.to_string()),
            max: bounds.max.map(|value| value.to_string()),
        },
        game::c_commands::BrigadierParser::Double(bounds) => BackendCommandParser::Number {
            kind: BackendCommandNumberKind::Double,
            min: bounds.min.map(|value| value.to_string()),
            max: bounds.max.map(|value| value.to_string()),
        },
        game::c_commands::BrigadierParser::Integer(bounds) => BackendCommandParser::Number {
            kind: BackendCommandNumberKind::Integer,
            min: bounds.min.map(|value| value.to_string()),
            max: bounds.max.map(|value| value.to_string()),
        },
        game::c_commands::BrigadierParser::Long(bounds) => BackendCommandParser::Number {
            kind: BackendCommandNumberKind::Long,
            min: bounds.min.map(|value| value.to_string()),
            max: bounds.max.map(|value| value.to_string()),
        },
        game::c_commands::BrigadierParser::String(kind) => BackendCommandParser::String {
            kind: match kind {
                BrigadierString::SingleWord => BackendCommandStringKind::SingleWord,
                BrigadierString::QuotablePhrase => BackendCommandStringKind::QuotablePhrase,
                BrigadierString::GreedyPhrase => BackendCommandStringKind::GreedyPhrase,
            },
        },
        game::c_commands::BrigadierParser::Entity(EntityParser {
            single,
            players_only,
        }) => BackendCommandParser::Entity {
            single: *single,
            players_only: *players_only,
        },
        game::c_commands::BrigadierParser::ScoreHolder { allows_multiple } => {
            BackendCommandParser::ScoreHolder {
                allows_multiple: *allows_multiple,
            }
        }
        game::c_commands::BrigadierParser::Time { min } => BackendCommandParser::Time { min: *min },
        game::c_commands::BrigadierParser::ResourceOrTag { registry_key } => {
            BackendCommandParser::Named {
                kind: "resource_or_tag".to_owned(),
                registry_key: Some(registry_key.to_string()),
            }
        }
        game::c_commands::BrigadierParser::ResourceOrTagKey { registry_key } => {
            BackendCommandParser::Named {
                kind: "resource_or_tag_key".to_owned(),
                registry_key: Some(registry_key.to_string()),
            }
        }
        game::c_commands::BrigadierParser::Resource { registry_key } => {
            BackendCommandParser::Named {
                kind: "resource".to_owned(),
                registry_key: Some(registry_key.to_string()),
            }
        }
        game::c_commands::BrigadierParser::ResourceKey { registry_key } => {
            BackendCommandParser::Named {
                kind: "resource_key".to_owned(),
                registry_key: Some(registry_key.to_string()),
            }
        }
        game::c_commands::BrigadierParser::ResourceSelector { registry_key } => {
            BackendCommandParser::Named {
                kind: "resource_selector".to_owned(),
                registry_key: Some(registry_key.to_string()),
            }
        }
        other => BackendCommandParser::Named {
            kind: format!("{other:?}"),
            registry_key: None,
        },
    }
}
fn backend_command_node(node: &game::c_commands::BrigadierNodeStub) -> BackendCommandNode {
    let node_type = match &node.node_type {
        game::c_commands::NodeType::Root => BackendCommandNodeType::Root,
        game::c_commands::NodeType::Literal { name } => {
            BackendCommandNodeType::Literal { name: name.clone() }
        }
        game::c_commands::NodeType::Argument {
            name,
            parser,
            suggestions_type,
        } => BackendCommandNodeType::Argument {
            name: name.clone(),
            parser: format!("{parser:?}"),
            parser_info: backend_command_parser(parser),
            suggestions_type: suggestions_type.as_ref().map(ToString::to_string),
        },
    };
    BackendCommandNode {
        executable: node.is_executable,
        children: node.children.clone(),
        redirect_node: node.redirect_node,
        node_type,
        restricted: node.is_restricted,
    }
}
fn backend_award_stats(packet: &game::c_award_stats::ClientboundAwardStats) -> BackendAwardStats {
    use game::c_award_stats::Stat;
    let mut stats = packet
        .stats
        .iter()
        .map(|(stat, value)| {
            let (category, value_id) = match stat {
                Stat::Mined(value) => ("mined", value.to_string()),
                Stat::Crafted(value) => ("crafted", value.to_string()),
                Stat::Used(value) => ("used", value.to_string()),
                Stat::Broken(value) => ("broken", value.to_string()),
                Stat::PickedUp(value) => ("picked_up", value.to_string()),
                Stat::Dropped(value) => ("dropped", value.to_string()),
                Stat::Killed(value) => ("killed", value.to_string()),
                Stat::KilledBy(value) => ("killed_by", value.to_string()),
                Stat::Custom(value) => ("custom", value.to_string()),
            };
            BackendStatValue {
                category: format!("minecraft:{category}"),
                value_id,
                value: *value,
            }
        })
        .collect::<Vec<_>>();
    stats.sort_by(|left, right| {
        (&left.category, &left.value_id).cmp(&(&right.category, &right.value_id))
    });
    BackendAwardStats { stats }
}
fn backend_update_advancements(
    packet: &game::c_update_advancements::ClientboundUpdateAdvancements,
) -> BackendUpdateAdvancements {
    let added = packet
        .added
        .iter()
        .map(|advancement| {
            let display = advancement.value.display.as_deref().map(|display| {
                let (item_id, count, components) = match display.icon.as_present() {
                    Some(item) => (
                        item.kind.to_string(),
                        item.count,
                        serde_json::to_value(&item.component_patch).ok(),
                    ),
                    None => ("minecraft:air".to_owned(), 0, None),
                };
                let frame = match display.frame {
                    game::c_update_advancements::FrameType::Task => "task",
                    game::c_update_advancements::FrameType::Challenge => "challenge",
                    game::c_update_advancements::FrameType::Goal => "goal",
                };
                BackendAdvancementDisplay {
                    title: serde_json::to_value(&display.title)
                        .unwrap_or_else(|_| serde_json::Value::String(display.title.to_string())),
                    description: serde_json::to_value(&display.description).unwrap_or_else(|_| {
                        serde_json::Value::String(display.description.to_string())
                    }),
                    icon: BackendAdvancementIcon {
                        item_id,
                        count,
                        components,
                    },
                    frame: frame.to_owned(),
                    show_toast: display.show_toast,
                    hidden: display.hidden,
                    background: display.background.as_ref().map(ToString::to_string),
                    x: display.x,
                    y: display.y,
                }
            });
            BackendAdvancementDefinition {
                id: advancement.id.to_string(),
                parent_id: advancement
                    .value
                    .parent_id
                    .as_ref()
                    .map(ToString::to_string),
                display,
                requirements: advancement.value.requirements.clone(),
                sends_telemetry_event: advancement.value.sends_telemetry_event,
            }
        })
        .collect();
    let progress = packet
        .progress
        .iter()
        .map(|(id, criteria)| {
            let mut criteria = criteria
                .iter()
                .map(
                    |(criterion, progress)| BackendAdvancementCriterionProgress {
                        criterion: criterion.clone(),
                        acquired: progress.date.is_some(),
                        acquired_at_epoch_millis: progress.date,
                    },
                )
                .collect::<Vec<_>>();
            criteria.sort_by(|left, right| left.criterion.cmp(&right.criterion));
            BackendAdvancementProgress {
                id: id.to_string(),
                criteria,
            }
        })
        .collect();
    BackendUpdateAdvancements {
        reset: packet.reset,
        added,
        removed: packet.removed.iter().map(ToString::to_string).collect(),
        progress,
        show_advancements: Some(packet.show_advancements),
    }
}
pub enum NetworkEvent {
    Connected {
        protocol_version: i32,
        version_name: &'static str,
        transport_peer: Option<std::net::SocketAddr>,
    },
    Disconnected {
        reason: String,
    },
    Login {
        entity_id: i32,
        player_uuid: uuid::Uuid,
        game_mode: u8,
        show_death_screen: bool,
        online_mode: bool,
        hardcore: bool,
        dimension: String,
        sea_level: i32,
        min_y: i32,
        height: u32,
        biome_zoom_seed: i64,
        biomes: Vec<BiomeRenderData>,
    },
    Respawn {
        game_mode: u8,
        dimension: String,
        sea_level: i32,
        min_y: i32,
        height: u32,
        biome_zoom_seed: i64,
        biomes: Vec<BiomeRenderData>,
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
        relative: RelativeMovementFlags,
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
        chunk: Chunk,
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
    BlockUpdate(BlockUpdateEvent),
    Sound {
        sound: String,
        source: u8,
        x: f64,
        y: f64,
        z: f64,
        volume: f32,
        pitch: f32,
        seed: u64,
        fixed_range: Option<f32>,
    },
    LevelEvent {
        event_type: u32,
        pos: BlockPos,
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
        fixed_range: Option<f32>,
    },
    StopSound {
        sound: Option<String>,
        source: Option<u8>,
    },
    BlockEvent {
        pos: BlockPos,
        action_id: u8,
        action_parameter: u8,
    },
    BlockEntityUpdate {
        pos: BlockPos,
        data: BlockEntityData,
    },
    SectionBlocksUpdate {
        updates: Vec<BlockUpdateEvent>,
    },
    LightUpdate(LightUpdateEvent),
    ChunkBiomesUpdated(Vec<ChunkBiomeUpdate>),
    BlockChangedAck {
        sequence: u32,
    },
    InventoryContent {
        items: Vec<el_core::ItemStack>,
    },
    InventorySlot {
        slot: u32,
        item: el_core::ItemStack,
        direct_player_slot: bool,
    },
    SetCursorItem {
        item: el_core::ItemStack,
    },
    OpenScreen {
        container_id: i32,
        menu_type: u32,
        title: String,
    },
    ContainerContent {
        container_id: i32,
        state_id: u32,
        items: Vec<el_core::ItemStack>,
        carried_item: el_core::ItemStack,
    },
    ContainerSlot {
        container_id: i32,
        state_id: u32,
        slot: u32,
        item: el_core::ItemStack,
    },
    ContainerClose {
        container_id: i32,
    },
    TransferRequested {
        host: String,
        port: u32,
    },
    EntityAdded(RemoteEntityEvent),
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
        item_stack: Option<el_core::ItemStack>,
        no_gravity: Option<bool>,
        item_frame_rotation: Option<i32>,
        item_frame_direction: Option<u8>,
        shared_flags: Option<u8>,
        living_flags: Option<u8>,
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
        ticks_frozen: Option<i32>,
        mob_size: Option<i32>,
        primed_tnt_fuse: Option<i32>,
        primed_tnt_block_state: Option<LocalBlockState>,
        end_crystal_beam_target: Option<BackendEndCrystalBeamTargetUpdate>,
        end_crystal_show_bottom: Option<bool>,
        abstract_arrow_flags: Option<u8>,
        abstract_arrow_pierce_level: Option<u8>,
        abstract_arrow_in_ground: Option<bool>,
        arrow_effect_color: Option<i32>,
        experience_orb_value: Option<i32>,
        armor_stand_client_flags: Option<u8>,
        armor_stand_head_pose: Option<BackendRotations>,
        armor_stand_body_pose: Option<BackendRotations>,
        armor_stand_left_arm_pose: Option<BackendRotations>,
        armor_stand_right_arm_pose: Option<BackendRotations>,
        armor_stand_left_leg_pose: Option<BackendRotations>,
        armor_stand_right_leg_pose: Option<BackendRotations>,
        is_baby: Option<bool>,
        is_aggressive: Option<bool>,
        mob_flags: Option<u8>,
        camel_dashing: Option<bool>,
        camel_last_pose_change_tick: Option<i64>,
        mob_boolean_metadata: Vec<(u8, bool)>,
        piglin_is_baby: Option<bool>,
        piglin_is_charging_crossbow: Option<bool>,
        piglin_is_dancing: Option<bool>,
        tamable_flags: Option<u8>,
        creaking_can_move: Option<bool>,
        creaking_is_active: Option<bool>,
        creaking_is_tearing_down: Option<bool>,
        block_display_block_state: Option<LocalBlockState>,
        item_display_item: Option<el_core::ItemStack>,
        item_display_context: Option<u8>,
        display_interpolation_start_delta_ticks: Option<i32>,
        display_interpolation_duration: Option<i32>,
        display_teleport_duration: Option<i32>,
        display_translation: Option<[f32; 3]>,
        display_scale: Option<[f32; 3]>,
        display_left_rotation: Option<[f32; 4]>,
        display_right_rotation: Option<[f32; 4]>,
        display_billboard: Option<u8>,
        display_brightness_override: Option<i32>,
        display_view_range: Option<f32>,
        display_shadow_radius: Option<f32>,
        display_shadow_strength: Option<f32>,
        display_width: Option<f32>,
        display_height: Option<f32>,
        display_glow_color_override: Option<i32>,
        text_display_text: Option<String>,
        text_display_line_width: Option<i32>,
        text_display_background: Option<i32>,
        text_display_text_opacity: Option<i8>,
        text_display_flags: Option<u8>,
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
        explosion_particle: Option<BackendParticle>,
        explosion_sound: Option<String>,
        block_particles: Vec<el_protocol_backend::BackendExplosionParticleInfo>,
    },
    UpdateAttributes {
        entity_id: i32,
        attributes: Vec<AttributeSnapshotEvent>,
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
        operation: BossEventOperation,
    },
    SetPassengers {
        vehicle_id: i32,
        passenger_ids: Vec<i32>,
    },
    SetCamera {
        camera_id: i32,
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
        steps: Vec<MinecartStep>,
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
        relative: RelativeMovementFlags,
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
        main_hand: Option<el_core::ItemStack>,
        off_hand: Option<el_core::ItemStack>,
        head: Option<el_core::ItemStack>,
        chest: Option<el_core::ItemStack>,
        legs: Option<el_core::ItemStack>,
        feet: Option<el_core::ItemStack>,
        body: Option<el_core::ItemStack>,
        saddle: Option<el_core::ItemStack>,
    },
    EntitiesRemoved {
        ids: Vec<i32>,
    },
    SystemChat {
        message: String,
        overlay: bool,
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
        root_index: u32,
        nodes: Vec<BackendCommandNode>,
    },
    CommandSuggestions {
        id: u32,
        range_start: usize,
        range_end: usize,
        suggestions: Vec<NetworkChatSuggestion>,
    },
    OnlinePlayers {
        names: Vec<String>,
    },
    CustomChatCompletions {
        action: ChatCompletionAction,
        entries: Vec<String>,
    },
    PlayerChat {
        message: String,
        signed_content: String,
        signature: Option<Vec<u8>>,
        trust: ChatTrust,
        click_spans: Vec<BackendChatClickSpan>,
    },
    DisguisedChat {
        message: String,
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
        pos: [i32; 3],
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
        absolute_pos: [i32; 3],
        relative_pos: [i32; 3],
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
    TickingState {
        tick_rate: f32,
        is_frozen: bool,
    },
    TickingStep {
        tick_steps: u32,
    },
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
        cape_url: Option<String>,
        model: Option<String>,
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
        color_patch: Option<el_protocol_backend::BackendMapPatch>,
        decorations: Option<Vec<el_protocol_backend::BackendMapDecoration>>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatTrust {
    Secure,
    Modified,
    NotSecure,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkChatSuggestion {
    pub text: String,
    pub tooltip: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatCompletionAction {
    Add,
    Remove,
    Set,
}
struct MessageSignatureCache {
    entries: Vec<Option<Vec<u8>>>,
}
impl MessageSignatureCache {
    fn new() -> Self {
        Self {
            entries: vec![None; 128],
        }
    }
    fn unpack(&self, signature: &game::c_player_chat::PackedMessageSignature) -> Option<Vec<u8>> {
        match signature {
            game::c_player_chat::PackedMessageSignature::Signature(signature) => {
                Some(signature.bytes.to_vec())
            }
            game::c_player_chat::PackedMessageSignature::Id(id) => {
                self.entries.get(*id as usize).cloned().flatten()
            }
        }
    }
    fn push(&mut self, last_seen: Vec<Vec<u8>>, signature: Option<Vec<u8>>) {
        let mut queue = VecDeque::from(last_seen);
        if let Some(signature) = signature {
            queue.push_back(signature);
        }
        let new_entries = queue.iter().cloned().collect::<HashSet<_>>();
        for index in 0..self.entries.len() {
            let Some(next) = queue.pop_back() else {
                break;
            };
            let previous = self.entries[index].replace(next);
            if let Some(previous) = previous {
                if !new_entries.contains(&previous) {
                    queue.push_front(previous);
                }
            }
        }
    }
}
#[derive(Clone)]
struct LastSeenTrackedEntry {
    signature: Vec<u8>,
    pending: bool,
}
struct LastSeenMessagesTracker {
    tracked_messages: Vec<Option<LastSeenTrackedEntry>>,
    tail: usize,
    offset: u32,
    last_tracked_message: Option<Vec<u8>>,
}
impl LastSeenMessagesTracker {
    fn new() -> Self {
        Self {
            tracked_messages: vec![None; 20],
            tail: 0,
            offset: 0,
            last_tracked_message: None,
        }
    }
    fn add_pending(&mut self, signature: Vec<u8>, was_shown: bool) -> bool {
        if self.last_tracked_message.as_ref() == Some(&signature) {
            return false;
        }
        self.last_tracked_message = Some(signature.clone());
        let entry = was_shown.then_some(LastSeenTrackedEntry {
            signature,
            pending: true,
        });
        self.tracked_messages[self.tail] = entry;
        self.tail = (self.tail + 1) % self.tracked_messages.len();
        self.offset = self.offset.saturating_add(1);
        true
    }
    fn take_ack_offset_if_ready(&mut self) -> Option<u32> {
        if self.offset <= 64 {
            return None;
        }
        Some(self.take_offset())
    }
    fn take_offset(&mut self) -> u32 {
        let offset = self.offset;
        self.offset = 0;
        offset
    }
    fn generate_and_apply_update(&mut self) -> game::s_chat::LastSeenMessagesUpdate {
        let offset = self.take_offset();
        let mut acknowledged = azalea_core::bitset::FixedBitSet::<20>::new();
        let mut entries = Vec::with_capacity(20);
        for i in 0..self.tracked_messages.len() {
            let index = (self.tail + i) % self.tracked_messages.len();
            if let Some(entry) = self.tracked_messages[index].as_mut() {
                acknowledged.set(i);
                entries.push(entry.signature.clone());
                entry.pending = false;
            }
        }
        game::s_chat::LastSeenMessagesUpdate {
            offset,
            acknowledged,
            checksum: last_seen_checksum(&entries),
        }
    }
}
fn java_byte_array_hash(bytes: &[u8]) -> i32 {
    bytes.iter().fold(1_i32, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(*byte as i8 as i32)
    })
}
fn last_seen_checksum(entries: &[Vec<u8>]) -> u8 {
    let checksum = entries.iter().fold(1_i32, |checksum, signature| {
        checksum
            .wrapping_mul(31)
            .wrapping_add(java_byte_array_hash(signature))
    }) as u8;
    if checksum == 0 { 1 } else { checksum }
}
fn contains_non_default_chat_font(component: &azalea_chat::FormattedText) -> bool {
    component
        .get_base()
        .style
        .font
        .as_deref()
        .is_some_and(|font| font != "minecraft:default" && font != "default")
        || component
            .get_base()
            .siblings
            .iter()
            .any(contains_non_default_chat_font)
}
#[derive(Default)]
struct SystemChatInteractionProbe {
    raw_probe_present: bool,
    raw_component_nodes: usize,
    raw_canonical_click_event_fields: usize,
    raw_legacy_click_event_fields: usize,
    raw_insertion_fields: usize,
    ast_component_nodes: usize,
    ast_click_event_nodes: usize,
    ast_insertion_nodes: usize,
    ast_action_counts: BTreeMap<&'static str, usize>,
}
fn system_chat_interaction_probe(component: &FormattedText) -> SystemChatInteractionProbe {
    let mut probe = SystemChatInteractionProbe::default();
    if let Some(raw) = component.get_base().raw_interaction_probe {
        probe.raw_probe_present = true;
        probe.raw_component_nodes = raw.component_nodes;
        probe.raw_canonical_click_event_fields = raw.canonical_click_event_fields;
        probe.raw_legacy_click_event_fields = raw.legacy_click_event_fields;
        probe.raw_insertion_fields = raw.insertion_fields;
    }
    collect_system_chat_ast_interaction_probe(component, &mut probe);
    probe
}
fn collect_system_chat_ast_interaction_probe(
    component: &FormattedText,
    probe: &mut SystemChatInteractionProbe,
) {
    probe.ast_component_nodes += 1;
    let style = &component.get_base().style;
    if let Some(action) = style.click_event.as_ref() {
        probe.ast_click_event_nodes += 1;
        *probe
            .ast_action_counts
            .entry(azalea_click_event_name(action))
            .or_default() += 1;
    }
    if style.insertion.is_some() {
        probe.ast_insertion_nodes += 1;
    }
    if let FormattedText::Translatable(translatable) = component {
        for argument in &translatable.args {
            if let azalea_chat::translatable_component::PrimitiveOrComponent::FormattedText(
                argument_component,
            ) = argument
            {
                collect_system_chat_ast_interaction_probe(argument_component, probe);
            }
        }
    }
    for sibling in &component.get_base().siblings {
        collect_system_chat_ast_interaction_probe(sibling, probe);
    }
}
fn azalea_click_event_name(event: &azalea_chat::click_event::ClickEvent) -> &'static str {
    use azalea_chat::click_event::ClickEvent;
    match event {
        ClickEvent::RunCommand { .. } => "run_command",
        ClickEvent::SuggestCommand { .. } => "suggest_command",
        ClickEvent::CopyToClipboard { .. } => "copy_to_clipboard",
        ClickEvent::OpenUrl { .. } => "open_url",
        ClickEvent::OpenFile { .. } => "open_file",
        ClickEvent::ChangePage { .. } => "change_page",
        ClickEvent::ShowDialog {} => "show_dialog",
        ClickEvent::Custom { .. } => "custom",
    }
}
fn log_system_chat_interaction_probe(probe: &SystemChatInteractionProbe, overlay: bool) {
    let raw_click_event_fields =
        probe.raw_canonical_click_event_fields + probe.raw_legacy_click_event_fields;
    let click_reconciliation = match (raw_click_event_fields, probe.ast_click_event_nodes) {
        (0, 0) => "raw_absent_ast_absent",
        (0, _) => "raw_absent_ast_present",
        (_, 0) => "raw_present_ast_absent",
        _ => "raw_present_ast_present",
    };
    let ast_actions = probe
        .ast_action_counts
        .iter()
        .map(|(kind, count)| format!("{kind}:{count}"))
        .collect::<Vec<_>>()
        .join(",");
    let ast_actions = if ast_actions.is_empty() {
        "none"
    } else {
        ast_actions.as_str()
    };
    diagnostics::write_line(&format!(
        "[ParityProbe] chat phase=backend_component_ast overlay={overlay} raw_probe={} raw_nodes={} raw_canonical_click_fields={} raw_legacy_click_fields={} raw_insertion_fields={} ast_nodes={} ast_click_nodes={} ast_insertion_nodes={} ast_actions={ast_actions} click_reconciliation={click_reconciliation}",
        if probe.raw_probe_present {
            "present"
        } else {
            "absent"
        },
        probe.raw_component_nodes,
        probe.raw_canonical_click_event_fields,
        probe.raw_legacy_click_event_fields,
        probe.raw_insertion_fields,
        probe.ast_component_nodes,
        probe.ast_click_event_nodes,
        probe.ast_insertion_nodes,
    ));
}
fn system_chat_text_and_click_spans(
    component: &FormattedText,
) -> (String, Vec<BackendChatClickSpan>) {
    #[derive(Clone, Default)]
    struct ActiveChatStyle {
        action: Option<BackendChatClickAction>,
        hover: Option<BackendChatHoverAction>,
        insertion: Option<String>,
    }
    let active_style = std::cell::RefCell::new(ActiveChatStyle::default());
    let spans = std::cell::RefCell::new(Vec::new());
    let visible_char_offset = std::cell::RefCell::new(0usize);
    let message = component.to_custom_format(
        |running_style, merged_style| {
            *active_style.borrow_mut() = ActiveChatStyle {
                action: merged_style
                    .click_event
                    .as_ref()
                    .map(backend_chat_click_action),
                hover: merged_style
                    .hover_event
                    .as_ref()
                    .map(backend_chat_hover_action),
                insertion: merged_style.insertion.clone(),
            };
            (running_style.compare_ansi(merged_style), String::new())
        },
        |text| {
            let active = active_style.borrow().clone();
            let mut offset = visible_char_offset.borrow_mut();
            let start = *offset;
            *offset += visible_chat_char_count_like_java(text);
            if start < *offset
                && (active.action.is_some() || active.hover.is_some() || active.insertion.is_some())
            {
                spans.borrow_mut().push(BackendChatClickSpan {
                    start,
                    end: *offset,
                    action: active.action,
                    hover: active.hover,
                    insertion: active.insertion,
                });
            }
            text.to_owned()
        },
        |style| {
            if !style.is_empty() {
                "\u{1b}[m".to_owned()
            } else {
                String::new()
            }
        },
        &azalea_chat::DEFAULT_STYLE,
    );
    (message, spans.into_inner())
}
fn enrich_hover_item_tooltips(
    spans: &mut [BackendChatClickSpan],
    metadata: &EnchantmentTooltipRegistry,
) {
    for span in spans {
        let Some(BackendChatHoverAction::ShowItem { id, tag, .. }) = span.hover.as_mut() else {
            continue;
        };
        let mut components = match tag.as_deref() {
            Some(bytes) => match el_nbt::io::read_any_compound(bytes) {
                Ok(Tag::Compound(components)) => components,
                _ => continue,
            },
            None => Vec::new(),
        };
        if let Ok(kind) = id.parse::<azalea_registry::builtin::ItemKind>() {
            use azalea_inventory::components::{Damage, MaxDamage, MaxStackSize, Rarity};
            use azalea_inventory::default_components::get_default_component;
            let mut local =
                local_item_stack_from_azalea(&azalea_inventory::ItemStack::new(kind, 1));
            for (name, value) in [
                (
                    "minecraft:max_damage",
                    get_default_component::<MaxDamage>(kind)
                        .and_then(|value| serde_json::to_value(value).ok()),
                ),
                (
                    "minecraft:max_stack_size",
                    get_default_component::<MaxStackSize>(kind)
                        .and_then(|value| serde_json::to_value(value).ok()),
                ),
                (
                    "minecraft:damage",
                    get_default_component::<Damage>(kind)
                        .and_then(|value| serde_json::to_value(value).ok()),
                ),
                (
                    "minecraft:rarity",
                    get_default_component::<Rarity>(kind)
                        .and_then(|value| serde_json::to_value(value).ok()),
                ),
            ] {
                if let Some(value) = value.as_ref().and_then(json_value_to_eltharn_nbt) {
                    local.components.set(
                        name,
                        el_core::item::DataComponent::Generic {
                            type_name: name.to_string(),
                            value,
                        },
                    );
                }
            }
            if let Tag::Compound(fields) = local.to_nbt()
                && let Some(Tag::Compound(defaults)) = fields
                    .into_iter()
                    .find(|(name, _)| name == "components")
                    .map(|(_, value)| value)
            {
                for (name, value) in defaults {
                    let removal = format!("!{name}");
                    if !components
                        .iter()
                        .any(|(key, _)| key == &name || key == &removal)
                    {
                        components.push((name, value));
                    }
                }
            }
        }
        let mut enchantment_ids = HashSet::new();
        for (key, value) in &components {
            if !matches!(
                key.as_str(),
                "minecraft:enchantments" | "minecraft:stored_enchantments"
            ) {
                continue;
            }
            let Tag::Compound(fields) = value else {
                continue;
            };
            let levels = fields
                .iter()
                .find(|(key, _)| key == "levels")
                .and_then(|(_, value)| {
                    if let Tag::Compound(fields) = value {
                        Some(fields)
                    } else {
                        None
                    }
                })
                .unwrap_or(fields);
            enchantment_ids.extend(
                levels
                    .iter()
                    .filter(|(_, value)| {
                        matches!(
                            value,
                            Tag::Byte(_) | Tag::Short(_) | Tag::Int(_) | Tag::Long(_)
                        )
                    })
                    .map(|(key, _)| key.clone()),
            );
        }
        if let Some(value) = metadata
            .item_component_json(&enchantment_ids)
            .as_ref()
            .and_then(json_value_to_eltharn_nbt)
        {
            components.retain(|(key, _)| key != "rustore:enchantment_tooltip_metadata");
            components.push(("rustore:enchantment_tooltip_metadata".into(), value));
        }
        let mut encoded = Vec::new();
        if el_nbt::io::write_named_tag(&mut encoded, "", &Tag::Compound(components)).is_ok() {
            *tag = Some(encoded);
        }
    }
}
fn backend_chat_hover_action(
    event: &azalea_chat::hover_event::HoverEvent,
) -> BackendChatHoverAction {
    use azalea_chat::hover_event::HoverEvent;
    match event {
        HoverEvent::ShowText { value } => BackendChatHoverAction::ShowText {
            value: value.to_ansi(),
        },
        HoverEvent::ShowItem {
            id,
            count,
            components,
        } => BackendChatHoverAction::ShowItem {
            id: id.clone(),
            count: *count,
            tag: components.as_ref().map(|components| {
                let mut bytes = Vec::new();
                azalea_protocol::simdnbt::owned::BaseNbt::new("", components.clone())
                    .write_unnamed(&mut bytes);
                bytes
            }),
        },
        HoverEvent::ShowEntity { id, uuid, name } => BackendChatHoverAction::ShowEntity {
            id: id.clone(),
            uuid: uuid.clone(),
            name: name.as_ref().map(|name| name.to_ansi()),
        },
    }
}
fn visible_chat_char_count_like_java(text: &str) -> usize {
    let mut chars = text.chars();
    let mut count = 0usize;
    while let Some(character) = chars.next() {
        if character == '\u{00a7}' {
            if chars.next().is_none() {
                break;
            }
            continue;
        }
        count += 1;
    }
    count
}
fn backend_chat_click_action(
    event: &azalea_chat::click_event::ClickEvent,
) -> BackendChatClickAction {
    use azalea_chat::click_event::ClickEvent;
    match event {
        ClickEvent::RunCommand { command } => BackendChatClickAction::RunCommand {
            command: command.clone(),
        },
        ClickEvent::SuggestCommand { command } => BackendChatClickAction::SuggestCommand {
            command: command.clone(),
        },
        ClickEvent::CopyToClipboard { value } => BackendChatClickAction::CopyToClipboard {
            value: value.clone(),
        },
        ClickEvent::OpenUrl { url } => BackendChatClickAction::OpenUrl { url: url.clone() },
        ClickEvent::OpenFile { path } => BackendChatClickAction::OpenFile { path: path.clone() },
        ClickEvent::ChangePage { page } => BackendChatClickAction::ChangePage { page: *page },
        ClickEvent::ShowDialog {} => BackendChatClickAction::ShowDialog,
        ClickEvent::Custom { id, payload } => BackendChatClickAction::Custom {
            id: id.clone(),
            payload: custom_click_action_payload_wire(payload.as_ref()),
        },
    }
}
fn custom_click_action_payload_wire(
    payload: Option<&azalea_protocol::simdnbt::owned::NbtTag>,
) -> Option<Vec<u8>> {
    let mut encoded = Vec::new();
    payload?.write(&mut encoded);
    Some(encoded)
}
fn backend_chat_click_action_name(action: &BackendChatClickAction) -> &'static str {
    match action {
        BackendChatClickAction::RunCommand { .. } => "run_command",
        BackendChatClickAction::SuggestCommand { .. } => "suggest_command",
        BackendChatClickAction::CopyToClipboard { .. } => "copy_to_clipboard",
        BackendChatClickAction::OpenUrl { .. } => "open_url",
        BackendChatClickAction::OpenFile { .. } => "open_file",
        BackendChatClickAction::ChangePage { .. } => "change_page",
        BackendChatClickAction::ShowDialog => "show_dialog",
        BackendChatClickAction::Custom { .. } => "custom",
    }
}
fn log_system_chat_click_metadata(message: &str, overlay: bool, spans: &[BackendChatClickSpan]) {
    let mut action_counts = BTreeMap::<&'static str, usize>::new();
    let mut insertion_spans = 0usize;
    let mut action_spans = 0usize;
    let mut custom_spans = 0usize;
    for span in spans {
        if let Some(action) = span.action.as_ref() {
            action_spans += 1;
            if matches!(action, BackendChatClickAction::Custom { .. }) {
                custom_spans += 1;
            }
            *action_counts
                .entry(backend_chat_click_action_name(action))
                .or_default() += 1;
        }
        if span.insertion.is_some() {
            insertion_spans += 1;
        }
    }
    let actions = action_counts
        .into_iter()
        .map(|(kind, count)| format!("{kind}:{count}"))
        .collect::<Vec<_>>()
        .join(",");
    let actions = if actions.is_empty() { "none" } else { &actions };
    diagnostics::write_line(&format!(
        "[ProtocolDebug][backend] system_chat click_spans={} insertion_spans={} actions={actions}",
        spans.len(),
        insertion_spans,
    ));
    diagnostics::write_line(&format!(
        "[ParityProbe] chat phase=backend_extract overlay={overlay} visible_chars={} click_spans={} action_spans={action_spans} custom_spans={custom_spans} insertion_spans={insertion_spans} actions={actions}",
        message.chars().count(),
        spans.len(),
    ));
}
#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("failed to build tokio runtime: {0}")]
    Runtime(std::io::Error),
    #[error("failed to resolve server address: {0}")]
    Resolve(#[from] azalea_protocol::resolve::ResolveError),
    #[error("connection error: {0}")]
    Connection(#[from] ConnectionError),
    #[error("packet read error: {0}")]
    Read(#[from] Box<azalea_protocol::read::ReadPacketError>),
    #[error("packet write error: {0}")]
    Write(#[from] std::io::Error),
    #[error("encryption request could not be answered")]
    Encryption,
    #[error("invalid client information: {0}")]
    InvalidClientInformation(String),
    #[error("server disconnected during login: {0}")]
    LoginDisconnect(String),
    #[error("server disconnected during configuration: {0}")]
    ConfigDisconnect(String),
    #[error("server disconnected during play: {0}")]
    PlayDisconnect(String),
    #[error("configuration finished before dimension registry was received")]
    MissingDimensionRegistry,
    #[error("failed to decode chunk [{x}, {z}]: {source}")]
    ChunkDecode {
        x: i32,
        z: i32,
        source: azalea_buf::BufReadError,
    },
}
pub fn offline_uuid(username: &str) -> uuid::Uuid {
    offline::generate_uuid(username)
}
fn build_network_runtime() -> io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("eltharn-protocol-io")
        .enable_all()
        .build()
}
pub fn spawn_connection(args: ConnectArgs) -> ConnectionHandle {
    let (event_tx, event_rx) = crossbeam_channel::bounded(EVENT_QUEUE_CAPACITY);
    let (packet_tx, packet_rx) = tokio::sync::mpsc::channel(PACKET_QUEUE_CAPACITY);
    let event_tx = EventSender::new(event_tx);
    let task_event_tx = event_tx.clone();
    let task = thread::Builder::new()
        .name("eltharn-azalea-network".to_string())
        .spawn(move || {
            let runtime = match build_network_runtime() {
                Ok(runtime) => runtime,
                Err(err) => {
                    send_disconnect(&task_event_tx, NetworkError::Runtime(err).to_string());
                    return;
                }
            };
            if let Err(err) =
                runtime.block_on(connect_and_run(args, task_event_tx.clone(), packet_rx))
            {
                send_disconnect(&task_event_tx, err.to_string());
            }
        })
        .expect("failed to spawn eltharn-azalea-network thread");
    ConnectionHandle {
        event_rx,
        packet_tx: PacketSender { tx: packet_tx },
        task: Some(task),
    }
}
fn send_disconnect(event_tx: &EventSender, reason: String) {
    eprintln!("[AzaleaProtocol][DISCONNECT] {reason}");
    let _ = event_tx.send(NetworkEvent::Disconnected { reason });
}
async fn connect_and_run(
    args: ConnectArgs,
    event_tx: EventSender,
    packet_rx: tokio::sync::mpsc::Receiver<OutboundPacket>,
) -> Result<(), NetworkError> {
    diagnostics::write_line(&format!(
        "[ConnectionDebug][backend] requested_target={:?} protocol={} version={}",
        args.server, PROTOCOL_VERSION, VERSION_NAME
    ));
    if VERBOSE_PROTOCOL_LOGS {
        eprintln!(
            "[AzaleaProtocol] Connecting to {} as {} with protocol {} ({})",
            args.server, args.username, PROTOCOL_VERSION, VERSION_NAME
        );
    }
    log::info!(
        "Connecting to {} as {} with protocol {} ({})",
        args.server,
        args.username,
        PROTOCOL_VERSION,
        VERSION_NAME
    );
    let resolved = args.server.clone().resolve().await?;
    diagnostics::write_line(&format!(
        "[ConnectionDebug][backend] resolved_target={} socket={}",
        resolved.server, resolved.socket
    ));
    let mut conn =
        Connection::<ClientboundHandshakePacket, ServerboundHandshakePacket>::new(&resolved.socket)
            .await?;
    conn.write(ServerboundIntention {
        protocol_version: PROTOCOL_VERSION,
        hostname: resolved.server.host.clone(),
        port: resolved.server.port,
        intention: ClientIntention::Login,
    })
    .await?;
    let conn = conn.login();
    let (conn, player_uuid) = login_sequence(conn, &args).await?;
    let (conn, registries, enchantment_tooltips, server_cookies) =
        config_sequence(conn, &args, &event_tx, args.server_cookies.clone()).await?;
    let conn = conn.game();
    let _ = event_tx.send(NetworkEvent::Connected {
        protocol_version: PROTOCOL_VERSION,
        version_name: VERSION_NAME,
        transport_peer: Some(resolved.socket),
    });
    game_loop(
        conn,
        event_tx,
        packet_rx,
        registries,
        enchantment_tooltips,
        player_uuid,
        args,
        server_cookies,
    )
    .await
}
async fn login_sequence(
    mut conn: Connection<ClientboundLoginPacket, ServerboundLoginPacket>,
    args: &ConnectArgs,
) -> Result<
    (
        Connection<ClientboundConfigPacket, ServerboundConfigPacket>,
        uuid::Uuid,
    ),
    NetworkError,
> {
    skin_debug(format!(
        "phase=serverbound_hello profile_uuid={} identity_source={}",
        args.uuid,
        if args.uuid == offline_uuid(&args.username) {
            "offline_fallback"
        } else {
            "launcher_profile"
        }
    ));
    conn.write(ServerboundHello {
        name: args.username.clone(),
        profile_id: args.uuid,
    })
    .await?;
    loop {
        let packet = conn.read().await?;
        if VERBOSE_PROTOCOL_LOGS {
            eprintln!(
                "[AzaleaProtocol] Login packet {} / {}",
                packet.id(),
                packet.name()
            );
        }
        match packet {
            ClientboundLoginPacket::Hello(p) => {
                let encrypted = azalea_crypto::encrypt(&p.public_key, &p.challenge)
                    .map_err(|_| NetworkError::Encryption)?;
                conn.write(ServerboundKey {
                    key_bytes: encrypted.encrypted_public_key,
                    encrypted_challenge: encrypted.encrypted_challenge,
                })
                .await?;
                conn.set_encryption_key(encrypted.secret_key);
            }
            ClientboundLoginPacket::LoginCompression(p) => {
                conn.set_compression_threshold(p.compression_threshold);
            }
            ClientboundLoginPacket::LoginFinished(p) => {
                conn.write(ServerboundLoginAcknowledged).await?;
                return Ok((conn.config(), p.game_profile.uuid));
            }
            ClientboundLoginPacket::LoginDisconnect(p) => {
                eprintln!(
                    "[AzaleaProtocol][DISCONNECT] Login disconnect: {}",
                    p.reason
                );
                return Err(NetworkError::LoginDisconnect(p.reason.to_string()));
            }
            ClientboundLoginPacket::CustomQuery(p) => {
                conn.write(ServerboundCustomQueryAnswer {
                    transaction_id: p.transaction_id,
                    data: None,
                })
                .await?;
            }
            ClientboundLoginPacket::CookieRequest(p) => {
                let payload = args.server_cookies.get(&p.key.to_string()).cloned();
                eprintln!(
                    "[CookieDebug] LOGIN_RECV_COOKIE_REQUEST key={} has_payload={} payload_len={}",
                    p.key,
                    payload.is_some(),
                    payload.as_ref().map_or(0, Vec::len)
                );
                conn.write(login::ServerboundCookieResponse {
                    key: p.key,
                    payload,
                })
                .await?;
            }
        }
    }
}
async fn config_sequence(
    mut conn: Connection<ClientboundConfigPacket, ServerboundConfigPacket>,
    args: &ConnectArgs,
    event_tx: &EventSender,
    mut server_cookies: HashMap<String, Vec<u8>>,
) -> Result<
    (
        Connection<ClientboundConfigPacket, ServerboundConfigPacket>,
        RegistryHolder,
        EnchantmentTooltipRegistry,
        HashMap<String, Vec<u8>>,
    ),
    NetworkError,
> {
    let mut registries = RegistryHolder::default();
    let mut enchantment_tooltips = EnchantmentTooltipRegistry::default();
    let mut pause_screen_additions = Vec::new();
    let mut packet_receive_diagnostics = ConfigPacketReceiveDiagnostics::default();
    let mut custom_payload_trace = CustomPayloadTrace::default();
    let mut brand_data = Vec::new();
    "vanilla".to_owned().azalea_write(&mut brand_data)?;
    conn.write(config::ServerboundCustomPayload {
        identifier: "brand".into(),
        data: brand_data.into(),
    })
    .await?;
    diagnostics::write_line(
        "[PayloadTrace][backend] direction=serverbound phase=configuration channel=minecraft:brand data_bytes=8 brand=vanilla result=write_ok",
    );
    let mut info = ClientInformation::default();
    if args.language.is_empty() || args.language.encode_utf16().count() > 16 {
        return Err(NetworkError::InvalidClientInformation(
            "language must contain 1-16 UTF-16 units".into(),
        ));
    }
    info.language = args.language.clone();
    info.view_distance = args.view_distance;
    info.main_hand = if args.main_hand_left {
        HumanoidArm::Left
    } else {
        HumanoidArm::Right
    };
    info.allows_listing = args.allows_listing;
    info.model_customization = azalea_protocol::common::client_information::ModelCustomization {
        cape: (args.model_customisation & (1 << 0)) != 0,
        jacket: (args.model_customisation & (1 << 1)) != 0,
        left_sleeve: (args.model_customisation & (1 << 2)) != 0,
        right_sleeve: (args.model_customisation & (1 << 3)) != 0,
        left_pants: (args.model_customisation & (1 << 4)) != 0,
        right_pants: (args.model_customisation & (1 << 5)) != 0,
        hat: (args.model_customisation & (1 << 6)) != 0,
    };
    eprintln!(
        "[ClientInformationDebug] CONFIG_SEND_CLIENT_INFORMATION language={} view_distance={} main_hand={} allows_listing={}",
        info.language,
        info.view_distance,
        if args.main_hand_left { "left" } else { "right" },
        info.allows_listing
    );
    conn.write(config::ServerboundClientInformation { information: info })
        .await?;
    loop {
        let packet = conn.read().await?;
        packet_receive_diagnostics.received_once(packet.id() as u32, packet.name());
        if VERBOSE_PROTOCOL_LOGS {
            eprintln!(
                "[AzaleaProtocol] Config packet {} / {}",
                packet.id(),
                packet.name()
            );
        }
        match packet {
            ClientboundConfigPacket::FinishConfiguration(_) => {
                if registries.dimension_type.map.is_empty() {
                    return Err(NetworkError::MissingDimensionRegistry);
                }
                enchantment_tooltips.resolve_tag_ids(&registries);
                emit_pause_screen_dialog(&event_tx, &registries, &pause_screen_additions);
                conn.write(ServerboundFinishConfiguration).await?;
                return Ok((conn, registries, enchantment_tooltips, server_cookies));
            }
            ClientboundConfigPacket::RegistryData(p) => {
                let _ = event_tx.send(NetworkEvent::RegistryData {
                    registry_id: p.registry_id.to_string(),
                    entries_count: p.entries.len() as u32,
                });
                if p.registry_id == Identifier::new("minecraft:enchantment") {
                    enchantment_tooltips.capture_registry_entries(&p.entries);
                }
                registries.append(p.registry_id, p.entries);
            }
            ClientboundConfigPacket::UpdateTags(p) => {
                enchantment_tooltips.capture_tags(&p.tags);
                if p.tags.0.contains_key(&Identifier::new("minecraft:dialog")) {
                    pause_screen_additions = pause_screen_dialog_tag_ids(&p.tags);
                }
            }
            ClientboundConfigPacket::SelectKnownPacks(p) => {
                let known_packs = p
                    .known_packs
                    .iter()
                    .map(|kp| (kp.namespace.clone(), kp.id.clone(), kp.version.clone()))
                    .collect::<Vec<_>>();
                let _ = event_tx.send(NetworkEvent::SelectKnownPacks { known_packs });
                conn.write(ServerboundSelectKnownPacks {
                    known_packs: Vec::new(),
                })
                .await?;
            }
            ClientboundConfigPacket::KeepAlive(p) => {
                conn.write(config::ServerboundKeepAlive { id: p.id })
                    .await?;
            }
            ClientboundConfigPacket::Ping(p) => {
                conn.write(config::ServerboundPong { id: p.id }).await?;
            }
            ClientboundConfigPacket::Disconnect(p) => {
                eprintln!(
                    "[AzaleaProtocol][DISCONNECT] Config disconnect: {}",
                    p.reason
                );
                return Err(NetworkError::ConfigDisconnect(p.reason.to_string()));
            }
            ClientboundConfigPacket::CookieRequest(p) => {
                let payload = server_cookies.get(&p.key.to_string()).cloned();
                eprintln!(
                    "[CookieDebug] CONFIG_RECV_COOKIE_REQUEST key={} has_payload={} payload_len={}",
                    p.key,
                    payload.is_some(),
                    payload.as_ref().map_or(0, Vec::len)
                );
                conn.write(config::ServerboundCookieResponse {
                    key: p.key,
                    payload,
                })
                .await?;
            }
            ClientboundConfigPacket::ResourcePackPush(p) => {
                eprintln!(
                    "[ResourcePackDebug] CONFIG_RECV_RESOURCE_PACK_PUSH id={} required={} url={:?} hash={:?} prompt_present={}",
                    p.id,
                    p.required,
                    p.url,
                    p.hash,
                    p.prompt.is_some()
                );
                if !is_supported_resource_pack_url(&p.url) {
                    write_config_resource_pack_status(
                        &mut conn,
                        p.id,
                        resource_pack_action::INVALID_URL,
                    )
                    .await?;
                    eprintln!(
                        "[ResourcePackDebug] CONFIG_SEND_RESOURCE_PACK_STATUS id={} action=InvalidUrl invalid_url={:?}",
                        p.id, p.url
                    );
                    continue;
                }
                let initial_status_sent =
                    matches!(args.server_pack_status, SERVER_PACK_STATUS_ENABLED)
                        || (args.server_pack_status == SERVER_PACK_STATUS_DISABLED && !p.required);
                let _ = event_tx.send(NetworkEvent::ResourcePackPush {
                    id: p.id,
                    url: p.url.clone(),
                    hash: p.hash.clone(),
                    required: p.required,
                    prompt: p.prompt.as_ref().map(|prompt| prompt.to_ansi()),
                    server_pack_status: args.server_pack_status,
                    initial_status_sent,
                });
                match args.server_pack_status {
                    SERVER_PACK_STATUS_ENABLED => {
                        conn.write(config::ServerboundResourcePack {
                            id: p.id,
                            action: config::s_resource_pack::Action::Accepted,
                        })
                        .await?;
                        eprintln!(
                            "[ResourcePackDebug] CONFIG_SEND_RESOURCE_PACK_STATUS id={} preference=ENABLED action=Accepted (awaiting client download)",
                            p.id
                        );
                    }
                    SERVER_PACK_STATUS_DISABLED if !p.required => {
                        conn.write(config::ServerboundResourcePack {
                            id: p.id,
                            action: config::s_resource_pack::Action::Declined,
                        })
                        .await?;
                        eprintln!(
                            "[ResourcePackDebug] CONFIG_SEND_RESOURCE_PACK_STATUS id={} preference=DISABLED action=Declined",
                            p.id
                        );
                    }
                    _ => {
                        eprintln!(
                            "[ResourcePackDebug] CONFIG_DEFER_RESOURCE_PACK_STATUS id={} preference={} required={}",
                            p.id, args.server_pack_status, p.required
                        )
                    }
                }
            }
            ClientboundConfigPacket::Transfer(p) => {
                eprintln!(
                    "[TransferDebug] BACKEND_RECV_CONFIG_TRANSFER host={} port={}",
                    p.host, p.port
                );
                let _ = event_tx.send(NetworkEvent::TransferRequested {
                    host: p.host.clone(),
                    port: p.port,
                });
                return Err(NetworkError::ConfigDisconnect(format!(
                    "server requested transfer to {}:{}",
                    p.host, p.port
                )));
            }
            ClientboundConfigPacket::UpdateEnabledFeatures(p) => {
                let features = p.features.iter().map(|f| f.to_string()).collect::<Vec<_>>();
                let _ = event_tx.send(NetworkEvent::UpdateEnabledFeatures { features });
            }
            ClientboundConfigPacket::CodeOfConduct(p) => {
                let _ = event_tx.send(NetworkEvent::CodeOfConduct {
                    text: p.code_of_conduct.clone(),
                });
                conn.write(config::ServerboundAcceptCodeOfConduct).await?;
            }
            ClientboundConfigPacket::CustomReportDetails(p) => {
                let details = p
                    .details
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                let _ = event_tx.send(NetworkEvent::CustomReportDetails { details });
            }
            ClientboundConfigPacket::ServerLinks(p) => {
                let links = p
                    .links
                    .iter()
                    .map(|l| {
                        let label = match &l.kind {
                            azalea_protocol::common::server_links::ServerLinkKind::Known(kind) => {
                                format!("{:?}", kind)
                            }
                            azalea_protocol::common::server_links::ServerLinkKind::Component(
                                text,
                            ) => text.to_ansi(),
                        };
                        (label, l.link.clone())
                    })
                    .collect::<Vec<_>>();
                let _ = event_tx.send(NetworkEvent::ServerLinks { links });
            }
            ClientboundConfigPacket::StoreCookie(p) => {
                let key = p.key.to_string();
                eprintln!(
                    "[CookieDebug] CONFIG_RECV_STORE_COOKIE key={} payload_len={}",
                    key,
                    p.payload.len()
                );
                server_cookies.insert(key.clone(), p.payload.clone());
                let _ = event_tx.send(NetworkEvent::StoreCookie {
                    key,
                    payload: p.payload.clone(),
                });
            }
            ClientboundConfigPacket::ResetChat(_) => {
                let _ = event_tx.send(NetworkEvent::ResetChat);
            }
            ClientboundConfigPacket::ResourcePackPop(p) => {
                let _ = event_tx.send(NetworkEvent::ResourcePackPop { id: p.id });
            }
            ClientboundConfigPacket::CustomPayload(p) => {
                custom_payload_trace.record(
                    "clientbound",
                    "configuration",
                    &p.identifier.to_string(),
                    p.data.0.len(),
                );
                let _ = event_tx.send(NetworkEvent::CustomPayload {
                    channel: p.identifier.to_string(),
                    data: p.data.0.clone(),
                });
            }
            ClientboundConfigPacket::ClearDialog(_) => {
                let _ = event_tx.send(NetworkEvent::ClearDialog);
            }
            ClientboundConfigPacket::ShowDialog(p) => {
                let dialog_nbt = map_dialog_nbt(&p.dialog);
                let _ = event_tx.send(NetworkEvent::ShowDialog {
                    dialog_kind: None,
                    dialog_nbt,
                });
            }
        }
    }
}
async fn write_shared_game_packet(
    writer: &SharedGameWriter,
    packet: ServerboundGamePacket,
    keep_alive_trace_order: Option<&AtomicU64>,
) -> Result<(), NetworkError> {
    let trace_started = keep_alive_trace_order.map(|_| Instant::now());
    let mut guard = writer.lock().await;
    let Some(writer) = guard.as_mut() else {
        return Err(NetworkError::PlayDisconnect(
            "play packet writer unavailable".to_string(),
        ));
    };
    let trace = keep_alive_trace_order
        .zip(trace_started)
        .map(|(counter, started)| {
            let order = counter.fetch_add(1, Ordering::Relaxed) + 1;
            wire_trace_begin(
                order,
                "kind=KeepAlive source=network_thread",
                started,
                started.elapsed(),
            );
            (order, started)
        });
    let write_started = trace.map(|_| Instant::now());
    let result = write_serverbound_game_packet_java_26_2(writer, &packet).await;
    if let (Some((order, started)), Some(write_started)) = (trace, write_started) {
        match &result {
            Ok(()) => wire_trace_ok(
                order,
                "kind=KeepAlive source=network_thread",
                started,
                write_started,
            ),
            Err(error) => wire_trace_error(
                order,
                "kind=KeepAlive source=network_thread",
                started,
                write_started,
                error,
            ),
        }
    }
    result?;
    Ok(())
}
fn wire_trace_summary(packet: &OutboundPacket) -> Option<String> {
    match packet {
        OutboundPacket::ResourcePackStatus { id, action } => {
            let action = resource_pack_action::canonical(*action);
            Some(format!(
                "kind=ResourcePackStatus id={id} action={action} name={}",
                resource_pack_action::name(action)
            ))
        }
        OutboundPacket::CustomClickAction { payload, .. } => {
            let (payload_kind, payload_bytes, nbt_wire_bytes) =
                custom_click_payload_metadata(payload.as_deref());
            Some(format!(
                "kind=CustomClickAction payload_kind={payload_kind} payload_bytes={payload_bytes} nbt_wire_bytes={nbt_wire_bytes}"
            ))
        }
        OutboundPacket::CustomPayload { channel, data } => Some(format!(
            "kind=CustomPayload channel={channel} data_bytes={}",
            data.len()
        )),
        OutboundPacket::PlayerLoaded => Some("kind=PlayerLoaded".to_string()),
        OutboundPacket::SetCarriedItem(slot) => Some(format!("kind=SetCarriedItem slot={slot}")),
        OutboundPacket::ClientTickEnd if !noisy_debug_enabled() => None,
        OutboundPacket::ClientTickEnd => Some("kind=ClientTickEnd".to_string()),
        OutboundPacket::Pong(_) if !noisy_debug_enabled() => None,
        OutboundPacket::Pong(id) => Some(format!("kind=Pong id={id}")),
        OutboundPacket::AcceptTeleportation(id) => Some(format!(
            "kind=AcceptTeleportation source=ipc teleport_id={id}"
        )),
        OutboundPacket::MovePosRot {
            x,
            y,
            z,
            yaw_degrees,
            pitch_degrees,
            on_ground,
            horizontal_collision,
        } => Some(format!(
            "kind=MovePosRot pos=[{x:.12},{y:.12},{z:.12}] yaw={yaw_degrees:.6} pitch={pitch_degrees:.6} on_ground={on_ground} horiz_collision={horizontal_collision}"
        )),
        OutboundPacket::MovePos {
            x,
            y,
            z,
            on_ground,
            horizontal_collision,
        } => Some(format!(
            "kind=MovePos pos=[{x:.12},{y:.12},{z:.12}] on_ground={on_ground} horiz_collision={horizontal_collision}"
        )),
        OutboundPacket::MoveRot {
            yaw_degrees,
            pitch_degrees,
            on_ground,
            horizontal_collision,
        } => Some(format!(
            "kind=MoveRot yaw={yaw_degrees:.6} pitch={pitch_degrees:.6} on_ground={on_ground} horiz_collision={horizontal_collision}"
        )),
        OutboundPacket::MoveStatusOnly {
            on_ground,
            horizontal_collision,
        } => Some(format!(
            "kind=MoveStatusOnly on_ground={on_ground} horiz_collision={horizontal_collision}"
        )),
        OutboundPacket::MoveVehicle {
            x,
            y,
            z,
            yaw_degrees,
            pitch_degrees,
            on_ground,
        } => Some(format!(
            "kind=MoveVehicle pos=[{x:.12},{y:.12},{z:.12}] yaw={yaw_degrees:.6} pitch={pitch_degrees:.6} on_ground={on_ground}"
        )),
        OutboundPacket::PaddleBoat { left, right } => {
            Some(format!("kind=PaddleBoat left={left} right={right}"))
        }
        OutboundPacket::PlayerInput {
            forward,
            backward,
            left,
            right,
            jump,
            shift,
            sprint,
        } => Some(format!(
            "kind=PlayerInput f={forward} b={backward} l={left} r={right} jump={jump} shift={shift} sprint={sprint}"
        )),
        OutboundPacket::PlayerCommand {
            entity_id,
            action,
            data,
        } => Some(format!(
            "kind=PlayerCommand entity_id={entity_id} action={action} data={data}"
        )),
        OutboundPacket::PlayerAction {
            action,
            x,
            y,
            z,
            direction,
            seq,
        } => Some(format!(
            "kind=PlayerAction action={action:?} pos=[{x},{y},{z}] direction={direction} seq={seq}"
        )),
        OutboundPacket::UseItemOn {
            hand,
            x,
            y,
            z,
            direction,
            cursor_x,
            cursor_y,
            cursor_z,
            inside,
            seq,
        } => Some(format!(
            "kind=UseItemOn hand={hand} pos=[{x},{y},{z}] direction={direction} cursor=[{cursor_x:.3},{cursor_y:.3},{cursor_z:.3}] inside={inside} seq={seq}"
        )),
        OutboundPacket::UseItem {
            hand,
            seq,
            yaw_degrees,
            pitch_degrees,
        } => Some(format!(
            "kind=UseItem hand={hand} seq={seq} yaw={yaw_degrees:.6} pitch={pitch_degrees:.6}"
        )),
        OutboundPacket::SwingMainHand => Some("kind=SwingMainHand".to_string()),
        OutboundPacket::Attack {
            entity_id,
            using_secondary_action,
        } => Some(format!(
            "kind=Attack entity_id={entity_id} secondary={using_secondary_action}"
        )),
        OutboundPacket::Interact {
            entity_id,
            hand,
            relative_x,
            relative_y,
            relative_z,
            using_secondary_action,
        } => Some(format!(
            "kind=Interact entity_id={entity_id} hand={hand} relative=[{relative_x:.6},{relative_y:.6},{relative_z:.6}] secondary={using_secondary_action}"
        )),
        OutboundPacket::InteractNoLocation {
            entity_id,
            hand,
            using_secondary_action,
        } => Some(format!(
            "kind=InteractNoLocation entity_id={entity_id} hand={hand} secondary={using_secondary_action}"
        )),
        _ => None,
    }
}
fn wire_trace_begin(order: u64, detail: &str, dequeued_at: Instant, lock_wait: Duration) {
    diagnostics::write_line(&format!(
        "[WireDebug][backend] order={order} phase=write_begin {detail} dequeue_to_begin_us={} writer_lock_us={}",
        dequeued_at.elapsed().as_micros(),
        lock_wait.as_micros(),
    ));
}
fn wire_trace_ok(order: u64, detail: &str, dequeued_at: Instant, write_started_at: Instant) {
    diagnostics::write_line(&format!(
        "[WireDebug][backend] order={order} phase=write_ok {detail} dequeue_to_ok_us={} write_us={}",
        dequeued_at.elapsed().as_micros(),
        write_started_at.elapsed().as_micros(),
    ));
}
fn wire_trace_error(
    order: u64,
    detail: &str,
    dequeued_at: Instant,
    write_started_at: Instant,
    error: &impl std::fmt::Display,
) {
    diagnostics::write_line(&format!(
        "[WireDebug][backend] order={order} phase=write_error {detail} dequeue_to_error_us={} write_us={} error={error}",
        dequeued_at.elapsed().as_micros(),
        write_started_at.elapsed().as_micros(),
    ));
}
async fn game_loop(
    conn: Connection<ClientboundGamePacket, ServerboundGamePacket>,
    event_tx: EventSender,
    packet_rx: tokio::sync::mpsc::Receiver<OutboundPacket>,
    registries: RegistryHolder,
    enchantment_tooltips: EnchantmentTooltipRegistry,
    player_uuid: uuid::Uuid,
    args: ConnectArgs,
    server_cookies: HashMap<String, Vec<u8>>,
) -> Result<(), NetworkError> {
    game_loop_with_rx(
        conn,
        event_tx,
        Arc::new(tokio::sync::Mutex::new(packet_rx)),
        registries,
        enchantment_tooltips,
        player_uuid,
        args,
        server_cookies,
    )
    .await
}
async fn game_loop_with_rx(
    conn: Connection<ClientboundGamePacket, ServerboundGamePacket>,
    event_tx: EventSender,
    packet_rx: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<OutboundPacket>>>,
    registries: RegistryHolder,
    mut enchantment_tooltips: EnchantmentTooltipRegistry,
    player_uuid: uuid::Uuid,
    args: ConnectArgs,
    mut server_cookies: HashMap<String, Vec<u8>>,
) -> Result<(), NetworkError> {
    let (mut reader, writer) = conn.into_split();
    let writer = Arc::new(tokio::sync::Mutex::new(Some(writer)));
    let writer_for_task = Arc::clone(&writer);
    let packet_rx_for_task = Arc::clone(&packet_rx);
    let last_seen_messages = Arc::new(Mutex::new(LastSeenMessagesTracker::new()));
    let last_seen_messages_for_task = Arc::clone(&last_seen_messages);
    let registries_for_writer = registries.clone();
    let container_mirror = Arc::new(Mutex::new(RemoteContainerMirror::default()));
    let container_mirror_for_writer = Arc::clone(&container_mirror);
    let container_mirror_for_reader = Arc::clone(&container_mirror);
    let menu_click_probe = Arc::new(Mutex::new(MenuClickPacketProbe::default()));
    let menu_click_probe_for_writer = Arc::clone(&menu_click_probe);
    let menu_click_probe_for_reader = Arc::clone(&menu_click_probe);
    let shared_pose = Arc::new(Mutex::new((
        AzVec3::new(0.0, 0.0, 0.0),
        LookDirection::new(0.0, 0.0),
    )));
    let shared_pose_for_writer = Arc::clone(&shared_pose);
    let wire_trace_order = Arc::new(AtomicU64::new(0));
    let wire_trace_order_for_writer = Arc::clone(&wire_trace_order);
    let entity_uuid_by_id: Arc<Mutex<HashMap<i32, uuid::Uuid>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let entity_uuid_by_id_for_reader = Arc::clone(&entity_uuid_by_id);
    let entity_uuid_by_id_for_writer = Arc::clone(&entity_uuid_by_id);
    let (writer_error_tx, mut writer_error_rx) = tokio::sync::oneshot::channel();
    let writer_task = tokio::spawn(async move {
        let mut writer_error_tx = Some(writer_error_tx);
        loop {
            let packet = {
                let mut packet_rx = packet_rx_for_task.lock().await;
                packet_rx.recv().await
            };
            let Some(packet) = packet else {
                break;
            };
            let wire_trace = wire_trace_summary(&packet);
            let wire_trace_dequeued_at = Instant::now();
            if let OutboundPacket::ResourcePackStatus { id, action } = &packet {
                let payload =
                    match serverbound_game_resource_pack_packet_id(*id).and_then(|packet_id| {
                        encode_serverbound_resource_pack_status(packet_id, *id, *action)
                    }) {
                        Ok(payload) => payload,
                        Err(err) => {
                            if let Some(writer_error_tx) = writer_error_tx.take() {
                                let _ = writer_error_tx.send(err);
                            }
                            break;
                        }
                    };
                let trace_order = wire_trace
                    .as_ref()
                    .map(|_| wire_trace_order_for_writer.fetch_add(1, Ordering::Relaxed) + 1);
                let lock_started_at = Instant::now();
                let write_result = {
                    let mut guard = writer_for_task.lock().await;
                    let lock_wait = lock_started_at.elapsed();
                    if let Some(writer) = guard.as_mut() {
                        let write_started_at = Instant::now();
                        if let (Some(order), Some(detail)) = (trace_order, wire_trace.as_deref()) {
                            wire_trace_begin(order, detail, wire_trace_dequeued_at, lock_wait);
                        }
                        let result = writer.raw.write(&payload).await;
                        if let (Some(order), Some(detail)) = (trace_order, wire_trace.as_deref()) {
                            match &result {
                                Ok(()) => wire_trace_ok(
                                    order,
                                    detail,
                                    wire_trace_dequeued_at,
                                    write_started_at,
                                ),
                                Err(err) => wire_trace_error(
                                    order,
                                    detail,
                                    wire_trace_dequeued_at,
                                    write_started_at,
                                    err,
                                ),
                            }
                        }
                        result
                    } else {
                        diagnostics::write_line(
                            "[WireDebug][backend] phase=writer_missing source=resource_pack_status",
                        );
                        break;
                    }
                };
                if let Err(err) = write_result {
                    if let Some(writer_error_tx) = writer_error_tx.take() {
                        let _ = writer_error_tx.send(err);
                    }
                    break;
                }
                continue;
            }
            let raw_java_packet = match &packet {
                OutboundPacket::Attack {
                    entity_id,
                    using_secondary_action,
                } => Some((
                    "attack",
                    encode_serverbound_attack(*entity_id, *using_secondary_action),
                )),
                OutboundPacket::SpectatorAction { spectate_entity_id } => {
                    let target_uuid = spectate_entity_id.and_then(|id| {
                        entity_uuid_by_id_for_writer
                            .lock()
                            .ok()
                            .and_then(|uuid_by_id| uuid_by_id.get(&id).copied())
                    });
                    match target_uuid {
                        Some(target_uuid) => Some((
                            "teleport_to_entity",
                            encode_serverbound_teleport_to_entity(target_uuid),
                        )),
                        None => {
                            diagnostics::write_line(
                                "[ProtocolDebug][backend] direction=serverbound phase=play disposition=dropped_unresolvable_spectate_target",
                            );
                            None
                        }
                    }
                }
                OutboundPacket::Interact {
                    entity_id,
                    hand,
                    relative_x,
                    relative_y,
                    relative_z,
                    using_secondary_action,
                } => Some((
                    "interact",
                    encode_serverbound_interact(
                        *entity_id,
                        *hand,
                        *relative_x,
                        *relative_y,
                        *relative_z,
                        *using_secondary_action,
                    ),
                )),
                OutboundPacket::InteractNoLocation {
                    entity_id,
                    hand,
                    using_secondary_action,
                } => Some((
                    "interact_no_location",
                    encode_serverbound_interact_no_location(
                        *entity_id,
                        *hand,
                        *using_secondary_action,
                    ),
                )),
                _ => None,
            };
            if let Some((source, payload)) = raw_java_packet {
                let payload = match payload {
                    Ok(payload) => payload,
                    Err(err) => {
                        if let Some(writer_error_tx) = writer_error_tx.take() {
                            let _ = writer_error_tx.send(err);
                        }
                        break;
                    }
                };
                let trace_order = wire_trace
                    .as_ref()
                    .map(|_| wire_trace_order_for_writer.fetch_add(1, Ordering::Relaxed) + 1);
                let lock_started_at = Instant::now();
                let write_result = {
                    let mut guard = writer_for_task.lock().await;
                    let lock_wait = lock_started_at.elapsed();
                    if let Some(writer) = guard.as_mut() {
                        let write_started_at = Instant::now();
                        if let (Some(order), Some(detail)) = (trace_order, wire_trace.as_deref()) {
                            wire_trace_begin(order, detail, wire_trace_dequeued_at, lock_wait);
                        }
                        let result = writer.raw.write(&payload).await;
                        if let (Some(order), Some(detail)) = (trace_order, wire_trace.as_deref()) {
                            match &result {
                                Ok(()) => wire_trace_ok(
                                    order,
                                    detail,
                                    wire_trace_dequeued_at,
                                    write_started_at,
                                ),
                                Err(err) => wire_trace_error(
                                    order,
                                    detail,
                                    wire_trace_dequeued_at,
                                    write_started_at,
                                    err,
                                ),
                            }
                        }
                        result
                    } else {
                        diagnostics::write_line(&format!(
                            "[WireDebug][backend] phase=writer_missing source={source}",
                        ));
                        break;
                    }
                };
                if let Err(err) = write_result {
                    if let Some(writer_error_tx) = writer_error_tx.take() {
                        let _ = writer_error_tx.send(err);
                    }
                    break;
                }
                continue;
            }
            if matches!(
                &packet,
                OutboundPacket::PlayerAction {
                    action: PlayerActionKind::Stab,
                    x: 0,
                    y: 0,
                    z: 0,
                    direction: 0,
                    seq: 0,
                }
            ) {
                let payload = match serverbound_game_player_action_packet_id()
                    .and_then(encode_serverbound_player_action_stab)
                {
                    Ok(payload) => payload,
                    Err(err) => {
                        if let Some(writer_error_tx) = writer_error_tx.take() {
                            let _ = writer_error_tx.send(err);
                        }
                        break;
                    }
                };
                let trace_order = wire_trace
                    .as_ref()
                    .map(|_| wire_trace_order_for_writer.fetch_add(1, Ordering::Relaxed) + 1);
                let lock_started_at = Instant::now();
                let write_result = {
                    let mut guard = writer_for_task.lock().await;
                    let lock_wait = lock_started_at.elapsed();
                    if let Some(writer) = guard.as_mut() {
                        let write_started_at = Instant::now();
                        if let (Some(order), Some(detail)) = (trace_order, wire_trace.as_deref()) {
                            wire_trace_begin(order, detail, wire_trace_dequeued_at, lock_wait);
                        }
                        let result = writer.raw.write(&payload).await;
                        if let (Some(order), Some(detail)) = (trace_order, wire_trace.as_deref()) {
                            match &result {
                                Ok(()) => wire_trace_ok(
                                    order,
                                    detail,
                                    wire_trace_dequeued_at,
                                    write_started_at,
                                ),
                                Err(err) => wire_trace_error(
                                    order,
                                    detail,
                                    wire_trace_dequeued_at,
                                    write_started_at,
                                    err,
                                ),
                            }
                        }
                        result
                    } else {
                        diagnostics::write_line(
                            "[WireDebug][backend] phase=writer_missing source=player_action_stab",
                        );
                        break;
                    }
                };
                if let Err(err) = write_result {
                    if let Some(writer_error_tx) = writer_error_tx.take() {
                        let _ = writer_error_tx.send(err);
                    }
                    break;
                }
                continue;
            }
            if let OutboundPacket::MoveVehicle {
                x,
                y,
                z,
                yaw_degrees,
                pitch_degrees,
                on_ground,
            } = &packet
            {
                let payload = match encode_serverbound_move_vehicle(
                    *x,
                    *y,
                    *z,
                    *yaw_degrees,
                    *pitch_degrees,
                    *on_ground,
                ) {
                    Ok(payload) => payload,
                    Err(err) => {
                        if let Some(writer_error_tx) = writer_error_tx.take() {
                            let _ = writer_error_tx.send(err);
                        }
                        break;
                    }
                };
                let trace_order = wire_trace
                    .as_ref()
                    .map(|_| wire_trace_order_for_writer.fetch_add(1, Ordering::Relaxed) + 1);
                let lock_started_at = Instant::now();
                let write_result = {
                    let mut guard = writer_for_task.lock().await;
                    let lock_wait = lock_started_at.elapsed();
                    if let Some(writer) = guard.as_mut() {
                        let write_started_at = Instant::now();
                        if let (Some(order), Some(detail)) = (trace_order, wire_trace.as_deref()) {
                            wire_trace_begin(order, detail, wire_trace_dequeued_at, lock_wait);
                        }
                        let result = writer.raw.write(&payload).await;
                        if let (Some(order), Some(detail)) = (trace_order, wire_trace.as_deref()) {
                            match &result {
                                Ok(()) => wire_trace_ok(
                                    order,
                                    detail,
                                    wire_trace_dequeued_at,
                                    write_started_at,
                                ),
                                Err(err) => wire_trace_error(
                                    order,
                                    detail,
                                    wire_trace_dequeued_at,
                                    write_started_at,
                                    err,
                                ),
                            }
                        }
                        result
                    } else {
                        diagnostics::write_line(
                            "[WireDebug][backend] phase=writer_missing source=move_vehicle",
                        );
                        break;
                    }
                };
                if let Err(err) = write_result {
                    if let Some(writer_error_tx) = writer_error_tx.take() {
                        let _ = writer_error_tx.send(err);
                    }
                    break;
                }
                continue;
            }
            if let OutboundPacket::CustomClickAction { id, payload } = &packet {
                let (payload_kind, payload_bytes, nbt_wire_bytes) =
                    custom_click_payload_metadata(payload.as_deref());
                let payload = match serverbound_game_custom_click_action_packet_id().and_then(
                    |packet_id| {
                        encode_serverbound_custom_click_action(packet_id, id, payload.as_deref())
                    },
                ) {
                    Ok(payload) => payload,
                    Err(err) => {
                        diagnostics::write_line(&format!(
                            "[ParityProbe] custom_click phase=writer_encode result=invalid_payload payload_kind={payload_kind} payload_bytes={payload_bytes} nbt_wire_bytes={nbt_wire_bytes}"
                        ));
                        if let Some(writer_error_tx) = writer_error_tx.take() {
                            let _ = writer_error_tx.send(err);
                        }
                        break;
                    }
                };
                let custom_wire_detail = format!(
                    "kind=CustomClickAction payload_kind={payload_kind} payload_bytes={payload_bytes} nbt_wire_bytes={nbt_wire_bytes} packet_body_bytes={}",
                    payload.len(),
                );
                diagnostics::write_line(&format!(
                    "[ParityProbe] custom_click phase=writer_encode result=ready payload_kind={payload_kind} payload_bytes={payload_bytes} nbt_wire_bytes={nbt_wire_bytes} packet_body_bytes={}",
                    payload.len(),
                ));
                let trace_order = wire_trace_order_for_writer.fetch_add(1, Ordering::Relaxed) + 1;
                let lock_started_at = Instant::now();
                let write_result = {
                    let mut guard = writer_for_task.lock().await;
                    let lock_wait = lock_started_at.elapsed();
                    if let Some(writer) = guard.as_mut() {
                        let write_started_at = Instant::now();
                        wire_trace_begin(
                            trace_order,
                            &custom_wire_detail,
                            wire_trace_dequeued_at,
                            lock_wait,
                        );
                        let result = writer.raw.write(&payload).await;
                        match &result {
                            Ok(()) => wire_trace_ok(
                                trace_order,
                                &custom_wire_detail,
                                wire_trace_dequeued_at,
                                write_started_at,
                            ),
                            Err(err) => wire_trace_error(
                                trace_order,
                                &custom_wire_detail,
                                wire_trace_dequeued_at,
                                write_started_at,
                                err,
                            ),
                        }
                        result
                    } else {
                        diagnostics::write_line(
                            "[WireDebug][backend] phase=writer_missing source=custom_click_action",
                        );
                        diagnostics::write_line(
                            "[ParityProbe] custom_click phase=raw_write result=writer_missing",
                        );
                        break;
                    }
                };
                match &write_result {
                    Ok(()) => diagnostics::write_line(&format!(
                        "[ParityProbe] custom_click phase=raw_write result=ok order={trace_order} payload_kind={payload_kind} payload_bytes={payload_bytes} packet_body_bytes={}",
                        payload.len(),
                    )),
                    Err(_) => diagnostics::write_line(&format!(
                        "[ParityProbe] custom_click phase=raw_write result=error order={trace_order} payload_kind={payload_kind} payload_bytes={payload_bytes} packet_body_bytes={}",
                        payload.len(),
                    )),
                }
                if let Err(err) = write_result {
                    if let Some(writer_error_tx) = writer_error_tx.take() {
                        let _ = writer_error_tx.send(err);
                    }
                    break;
                }
                continue;
            }
            let packet = match packet {
                OutboundPacket::ResourcePackStatus { .. } => {
                    unreachable!(
                        "resource-pack status is serialized before typed packets"
                    )
                }
                OutboundPacket::CustomClickAction { .. } => {
                    unreachable!(
                        "custom click action is serialized before typed packets"
                    )
                }
                OutboundPacket::CustomPayload { channel, data } => {
                    if let Err(error) = el_protocol_backend::payload::validate_custom_payload(
                        &channel,
                        &data,
                        true,
                    ) {
                        eprintln!("[CustomPayload] rejected outbound: {error}");
                        continue;
                    }
                    ServerboundGamePacket::CustomPayload(game::ServerboundCustomPayload {
                        identifier: Identifier::new(&channel),
                        data: azalea_buf::UnsizedByteArray::from(data),
                    })
                }
                OutboundPacket::Chat(message) => {
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let last_seen_messages = last_seen_messages_for_task
                        .lock()
                        .expect("last-seen chat tracker poisoned")
                        .generate_and_apply_update();
                    ServerboundGamePacket::Chat(ServerboundChat {
                        message,
                        timestamp,
                        salt: azalea_crypto::signing::make_salt(),
                        signature: None,
                        last_seen_messages,
                    })
                }
                OutboundPacket::Command(command) => {
                    ServerboundGamePacket::ChatCommand(ServerboundChatCommand {
                        command,
                    })
                }
                OutboundPacket::CommandSuggestion { id, command } => {
                    ServerboundGamePacket::CommandSuggestion(ServerboundCommandSuggestion {
                        id,
                        command,
                    })
                }
                OutboundPacket::PlayerLoaded => {
                    eprintln!("[Network] BACKEND_SEND_PLAYER_LOADED");
                    ServerboundGamePacket::PlayerLoaded(ServerboundPlayerLoaded)
                }
                OutboundPacket::ClientTickEnd => {
                    ServerboundGamePacket::ClientTickEnd(ServerboundClientTickEnd)
                }
                OutboundPacket::RequestStats => {
                    ServerboundGamePacket::ClientCommand(ServerboundClientCommand {
                        action: s_client_command::Action::RequestStats,
                    })
                }
                OutboundPacket::Pong(id) => {
                    ServerboundGamePacket::Pong(game::ServerboundPong { id })
                }
                OutboundPacket::PerformRespawn => {
                    eprintln!("[Network] BACKEND_SEND_PERFORM_RESPAWN");
                    ServerboundGamePacket::ClientCommand(ServerboundClientCommand {
                        action: s_client_command::Action::PerformRespawn,
                    })
                }
                OutboundPacket::AcceptTeleportation(id) => {
                    ServerboundGamePacket::AcceptTeleportation(ServerboundAcceptTeleportation {
                        id,
                    })
                }
                OutboundPacket::TeleportConfirm {
                    teleport_id,
                    x,
                    y,
                    z,
                    yaw_degrees,
                    pitch_degrees,
                } => {
                    let pos = AzVec3::new(x, y, z);
                    let look = LookDirection::new(yaw_degrees, pitch_degrees);
                    if let Ok(mut lock) = shared_pose_for_writer.lock() {
                        *lock = (pos, look);
                    }
                    let confirm_packet = ServerboundGamePacket::AcceptTeleportation(ServerboundAcceptTeleportation {
                        id: teleport_id,
                    });
                    let move_packet = ServerboundGamePacket::MovePlayerPosRot(ServerboundMovePlayerPosRot {
                        pos,
                        look_direction: look,
                        flags: MoveFlags {
                            on_ground: false,
                            horizontal_collision: false,
                        },
                    });
                    let confirm_detail = format!(
                        "kind=AcceptTeleportation source=teleport_confirm teleport_id={teleport_id}"
                    );
                    let move_detail = format!(
                        "kind=MovePosRot source=teleport_confirm teleport_id={teleport_id} pos=[{x:.12},{y:.12},{z:.12}] yaw={yaw_degrees:.6} pitch={pitch_degrees:.6} on_ground=false horiz_collision=false"
                    );
                    let lock_started_at = Instant::now();
                    let mut guard = writer_for_task.lock().await;
                    let lock_wait = lock_started_at.elapsed();
                    let Some(writer) = guard.as_mut() else {
                        diagnostics::write_line(
                            "[WireDebug][backend] phase=writer_missing source=teleport_confirm",
                        );
                        break;
                    };
                    let confirm_order = wire_trace_order_for_writer
                        .fetch_add(1, Ordering::Relaxed) + 1;
                    wire_trace_begin(
                        confirm_order,
                        &confirm_detail,
                        wire_trace_dequeued_at,
                        lock_wait,
                    );
                    let confirm_started_at = Instant::now();
                    if let Err(err) = write_serverbound_game_packet_java_26_2(
                            writer,
                            &confirm_packet,
                        )
                        .await
                    {
                        wire_trace_error(
                            confirm_order,
                            &confirm_detail,
                            wire_trace_dequeued_at,
                            confirm_started_at,
                            &err,
                        );
                        if let Some(writer_error_tx) = writer_error_tx.take() {
                            let _ = writer_error_tx.send(err);
                        }
                        break;
                    }
                    wire_trace_ok(
                        confirm_order,
                        &confirm_detail,
                        wire_trace_dequeued_at,
                        confirm_started_at,
                    );
                    let move_order = wire_trace_order_for_writer
                        .fetch_add(1, Ordering::Relaxed) + 1;
                    wire_trace_begin(
                        move_order,
                        &move_detail,
                        wire_trace_dequeued_at,
                        Duration::ZERO,
                    );
                    let move_started_at = Instant::now();
                    if let Err(err) = write_serverbound_game_packet_java_26_2(
                            writer,
                            &move_packet,
                        )
                        .await
                    {
                        wire_trace_error(
                            move_order,
                            &move_detail,
                            wire_trace_dequeued_at,
                            move_started_at,
                            &err,
                        );
                        if let Some(writer_error_tx) = writer_error_tx.take() {
                            let _ = writer_error_tx.send(err);
                        }
                        break;
                    }
                    wire_trace_ok(
                        move_order,
                        &move_detail,
                        wire_trace_dequeued_at,
                        move_started_at,
                    );
                    continue;
                }
                OutboundPacket::SetCarriedItem(slot) => {
                    ServerboundGamePacket::SetCarriedItem(ServerboundSetCarriedItem {
                        slot,
                    })
                }
                OutboundPacket::MovePosRot {
                    x,
                    y,
                    z,
                    yaw_degrees,
                    pitch_degrees,
                    on_ground,
                    horizontal_collision,
                } => {
                    let pos = AzVec3::new(x, y, z);
                    let look = LookDirection::new(yaw_degrees, pitch_degrees);
                    if let Ok(mut lock) = shared_pose_for_writer.lock() {
                        *lock = (pos, look);
                    }
                    ServerboundGamePacket::MovePlayerPosRot(ServerboundMovePlayerPosRot {
                        pos,
                        look_direction: look,
                        flags: MoveFlags {
                            on_ground,
                            horizontal_collision,
                        },
                    })
                }
                OutboundPacket::MovePos { x, y, z, on_ground, horizontal_collision } => {
                    let pos = AzVec3::new(x, y, z);
                    if let Ok(mut lock) = shared_pose_for_writer.lock() {
                        lock.0 = pos;
                    }
                    ServerboundGamePacket::MovePlayerPos(ServerboundMovePlayerPos {
                        pos,
                        flags: MoveFlags {
                            on_ground,
                            horizontal_collision,
                        },
                    })
                }
                OutboundPacket::MoveRot {
                    yaw_degrees,
                    pitch_degrees,
                    on_ground,
                    horizontal_collision,
                } => {
                    let look = LookDirection::new(yaw_degrees, pitch_degrees);
                    if let Ok(mut lock) = shared_pose_for_writer.lock() {
                        lock.1 = look;
                    }
                    ServerboundGamePacket::MovePlayerRot(ServerboundMovePlayerRot {
                        look_direction: look,
                        flags: MoveFlags {
                            on_ground,
                            horizontal_collision,
                        },
                    })
                }
                OutboundPacket::MoveStatusOnly { on_ground, horizontal_collision } => {
                    ServerboundGamePacket::MovePlayerStatusOnly(ServerboundMovePlayerStatusOnly {
                        flags: MoveFlags {
                            on_ground,
                            horizontal_collision,
                        },
                    })
                }
                OutboundPacket::MoveVehicle { .. } => {
                    unreachable!(
                        "MoveVehicle is written above with Java's required onGround boolean"
                    )
                }
                OutboundPacket::PaddleBoat { left, right } => {
                    ServerboundGamePacket::PaddleBoat(azalea_protocol::packets::game::s_paddle_boat::ServerboundPaddleBoat {
                        left,
                        right,
                    })
                }
                OutboundPacket::PlayerInput {
                    forward,
                    backward,
                    left,
                    right,
                    jump,
                    shift,
                    sprint,
                } => {
                    ServerboundGamePacket::PlayerInput(ServerboundPlayerInput {
                        forward,
                        backward,
                        left,
                        right,
                        jump,
                        shift,
                        sprint,
                    })
                }
                OutboundPacket::PlayerCommand { entity_id, action, data } => {
                    let parsed_action = match action.to_ascii_lowercase().as_str() {
                        "stop_sleeping" => {
                            Some(
                                azalea_protocol::packets::game::s_player_command::Action::StopSleeping,
                            )
                        }
                        "start_sprinting" => {
                            Some(
                                azalea_protocol::packets::game::s_player_command::Action::StartSprinting,
                            )
                        }
                        "stop_sprinting" => {
                            Some(
                                azalea_protocol::packets::game::s_player_command::Action::StopSprinting,
                            )
                        }
                        "start_riding_jump" => {
                            Some(
                                azalea_protocol::packets::game::s_player_command::Action::StartRidingJump,
                            )
                        }
                        "stop_riding_jump" => {
                            Some(
                                azalea_protocol::packets::game::s_player_command::Action::StopRidingJump,
                            )
                        }
                        "open_inventory" => {
                            Some(
                                azalea_protocol::packets::game::s_player_command::Action::OpenInventory,
                            )
                        }
                        "start_fall_flying" => {
                            Some(
                                azalea_protocol::packets::game::s_player_command::Action::StartFallFlying,
                            )
                        }
                        _ => None,
                    };
                    if let Some(act) = parsed_action {
                        ServerboundGamePacket::PlayerCommand(azalea_protocol::packets::game::s_player_command::ServerboundPlayerCommand {
                            id: azalea_core::entity_id::MinecraftEntityId(entity_id),
                            action: act,
                            data: data as u32,
                        })
                    } else {
                        continue;
                    }
                }
                OutboundPacket::PlayerAction { action, x, y, z, direction, seq } => {
                    generated_serverbound_player_action_packet(
                        action,
                        x,
                        y,
                        z,
                        direction,
                        seq,
                    )
                }
                OutboundPacket::UseItemOn {
                    hand,
                    x,
                    y,
                    z,
                    direction,
                    cursor_x,
                    cursor_y,
                    cursor_z,
                    inside,
                    seq,
                } => {
                    eprintln!(
                        "[MenuDebug] BACKEND_SEND_USE_ITEM_ON hand={} seq={} pos=[{},{},{}] face={} cursor=[{:.3},{:.3},{:.3}] inside={}",
                        hand, seq, x, y, z, direction, cursor_x, cursor_y, cursor_z,
                        inside
                    );
                    ServerboundGamePacket::UseItemOn(ServerboundUseItemOn {
                        hand: az_hand(hand),
                        block_hit: s_use_item_on::BlockHit {
                            block_pos: AzBlockPos::new(x, y, z),
                            direction: az_direction(direction),
                            location: AzVec3::new(
                                x as f64 + cursor_x as f64,
                                y as f64 + cursor_y as f64,
                                z as f64 + cursor_z as f64,
                            ),
                            inside,
                            world_border: false,
                        },
                        seq,
                    })
                }
                OutboundPacket::UseItem { hand, seq, yaw_degrees, pitch_degrees } => {
                    eprintln!(
                        "[MenuDebug] BACKEND_SEND_USE_ITEM hand={} seq={} yaw={:.2} pitch={:.2}",
                        hand, seq, yaw_degrees, pitch_degrees
                    );
                    ServerboundGamePacket::UseItem(ServerboundUseItem {
                        hand: az_hand(hand),
                        seq,
                        y_rot: yaw_degrees,
                        x_rot: pitch_degrees,
                    })
                }
                OutboundPacket::SwingMainHand => {
                    ServerboundGamePacket::Swing(ServerboundSwing {
                        hand: s_interact::InteractionHand::MainHand,
                    })
                }
                OutboundPacket::SwingOffHand => {
                    ServerboundGamePacket::Swing(ServerboundSwing {
                        hand: s_interact::InteractionHand::OffHand,
                    })
                }
                OutboundPacket::Attack { .. } => {
                    unreachable!("attack is written above as ServerboundInteractPacket")
                }
                OutboundPacket::Interact { .. } => {
                    unreachable!(
                        "interact is serialized above as ServerboundInteractPacket"
                    )
                }
                OutboundPacket::InteractNoLocation { .. } => {
                    unreachable!(
                        "interact_no_location is serialized above as ServerboundInteractPacket"
                    )
                }
                OutboundPacket::ContainerClick(packet) => {
                    eprintln!(
                        "[MenuDebug] BACKEND_SEND_CONTAINER_CLICK container={} state={} slot={} button={} input={:?} changed={} carried={}x{}",
                        packet.container_id, packet.state_id, packet.slot_num, packet
                        .button_num, packet.container_input, packet.changed_slots.len(),
                        packet.carried_item.item.location, packet.carried_item.count
                    );
                    if let Ok(mut probe) = menu_click_probe_for_writer.lock() {
                        probe.begin(&packet);
                    }
                    let (
                        raw_changed_slots_with_sources,
                        raw_carried_item,
                        raw_carried_source,
                    ) = if let Ok(mirror) = container_mirror_for_writer.lock() {
                        let raw_changed_slots = packet
                            .changed_slots
                            .iter()
                            .filter_map(|(slot_num, item_stack)| {
                                let slot = u16::try_from(*slot_num).ok()?;
                                let (raw_stack, source) = mirror
                                    .raw_stack_for_outgoing_click_with_source(
                                        packet.container_id,
                                        Some(*slot_num),
                                        item_stack,
                                        &registries_for_writer,
                                    );
                                Some((slot, raw_stack, source))
                            })
                            .collect::<Vec<_>>();
                        let (raw_carried_item, raw_carried_source) = mirror
                            .raw_stack_for_outgoing_click_with_source(
                                packet.container_id,
                                Some(packet.slot_num),
                                &packet.carried_item,
                                &registries_for_writer,
                            );
                        (raw_changed_slots, raw_carried_item, raw_carried_source)
                    } else {
                        let raw_changed_slots = packet
                            .changed_slots
                            .iter()
                            .filter_map(|(slot_num, item_stack)| {
                                Some((
                                    u16::try_from(*slot_num).ok()?,
                                    azalea_item_stack_from_local(item_stack),
                                    "mirror_lock_failed",
                                ))
                            })
                            .collect::<Vec<_>>();
                        (
                            raw_changed_slots,
                            azalea_item_stack_from_local(&packet.carried_item),
                            "mirror_lock_failed",
                        )
                    };
                    eprintln!(
                        "[MenuDebug] BACKEND_CONTAINER_CLICK_RAW container={} state={} slot={} changed_raw={} carried_raw={}:{}",
                        packet.container_id, packet.state_id, packet.slot_num,
                        describe_raw_changed_slots(& raw_changed_slots_with_sources),
                        raw_carried_source, describe_azalea_stack(& raw_carried_item)
                    );
                    let raw_changed_slots = raw_changed_slots_with_sources
                        .iter()
                        .map(|(slot, stack, _source)| (*slot, stack.clone()))
                        .collect::<Vec<_>>();
                    if let Ok(mut mirror) = container_mirror_for_writer.lock() {
                        mirror
                            .apply_outgoing_click_prediction(
                                packet.container_id,
                                &raw_changed_slots,
                                &raw_carried_item,
                            );
                    }
                    let mut changed_slots = indexmap::IndexMap::new();
                    for (slot_num, az_stack) in raw_changed_slots {
                        let hashed = azalea_protocol::packets::game::s_container_click::HashedStack::from_item_stack(
                            &az_stack,
                            &registries_for_writer,
                        );
                        changed_slots.insert(slot_num as u16, hashed);
                    }
                    let carried_item = azalea_protocol::packets::game::s_container_click::HashedStack::from_item_stack(
                        &raw_carried_item,
                        &registries_for_writer,
                    );
                    eprintln!(
                        "[MenuDebug] BACKEND_CONTAINER_CLICK_HASHED container={} state={} slot={} changed_hash={} carried_hash={}",
                        packet.container_id, packet.state_id, packet.slot_num,
                        describe_hashed_changed_slots(& changed_slots),
                        describe_hashed_stack(& carried_item)
                    );
                    let click_type = match packet.container_input {
                        el_protocol::packets::game::ContainerInput::Pickup => {
                            azalea_inventory::operations::ClickType::Pickup
                        }
                        el_protocol::packets::game::ContainerInput::QuickMove => {
                            azalea_inventory::operations::ClickType::QuickMove
                        }
                        el_protocol::packets::game::ContainerInput::Swap => {
                            azalea_inventory::operations::ClickType::Swap
                        }
                        el_protocol::packets::game::ContainerInput::Clone => {
                            azalea_inventory::operations::ClickType::Clone
                        }
                        el_protocol::packets::game::ContainerInput::Throw => {
                            azalea_inventory::operations::ClickType::Throw
                        }
                        el_protocol::packets::game::ContainerInput::QuickCraft => {
                            azalea_inventory::operations::ClickType::QuickCraft
                        }
                        el_protocol::packets::game::ContainerInput::PickupAll => {
                            azalea_inventory::operations::ClickType::PickupAll
                        }
                    };
                    ServerboundGamePacket::ContainerClick(ServerboundContainerClick {
                        container_id: packet.container_id,
                        state_id: packet.state_id as u32,
                        slot_num: packet.slot_num,
                        button_num: packet.button_num as u8,
                        click_type,
                        changed_slots,
                        carried_item,
                    })
                }
                OutboundPacket::SetCreativeModeSlot(packet) => {
                    let az_stack = azalea_item_stack_from_local(&packet.item_stack);
                    ServerboundGamePacket::SetCreativeModeSlot(ServerboundSetCreativeModeSlot {
                        slot_num: packet.slot_num as u16,
                        item_stack: az_stack,
                    })
                }
                OutboundPacket::ContainerClose(container_id) => {
                    if let Ok(mut mirror) = container_mirror_for_writer.lock() {
                        mirror.close(container_id);
                    }
                    ServerboundGamePacket::ContainerClose(ServerboundContainerClose {
                        container_id,
                    })
                }
                OutboundPacket::PlayerAbilities { is_flying } => {
                    ServerboundGamePacket::PlayerAbilities(ServerboundPlayerAbilities {
                        is_flying,
                    })
                }
                OutboundPacket::ChangeGameMode(mode) => {
                    ServerboundGamePacket::ChangeGameMode(ServerboundChangeGameMode {
                        mode,
                    })
                }
                OutboundPacket::SignUpdate { x, y, z, is_front_text, lines } => {
                    ServerboundGamePacket::SignUpdate(azalea_protocol::packets::game::s_sign_update::ServerboundSignUpdate {
                        pos: AzBlockPos::new(x, y, z),
                        is_front_text,
                        lines,
                    })
                }
                OutboundPacket::EditBook { slot, pages, title } => {
                    ServerboundGamePacket::EditBook(azalea_protocol::packets::game::s_edit_book::ServerboundEditBook {
                        slot,
                        pages,
                        title,
                    })
                }
                OutboundPacket::RecipeBookSeenRecipe { recipe } => {
                    ServerboundGamePacket::RecipeBookSeenRecipe(azalea_protocol::packets::game::s_recipe_book_seen_recipe::ServerboundRecipeBookSeenRecipe {
                        recipe: azalea_registry::identifier::Identifier::new(&recipe),
                    })
                }
                OutboundPacket::RecipeBookChangeSettings {
                    book_type,
                    is_open,
                    is_filtering,
                } => {
                    let book_type = match book_type {
                        0 => {
                            azalea_protocol::packets::game::s_recipe_book_change_settings::RecipeBookType::Crafting
                        }
                        1 => {
                            azalea_protocol::packets::game::s_recipe_book_change_settings::RecipeBookType::Furnace
                        }
                        2 => {
                            azalea_protocol::packets::game::s_recipe_book_change_settings::RecipeBookType::BlastFurnace
                        }
                        _ => {
                            azalea_protocol::packets::game::s_recipe_book_change_settings::RecipeBookType::Smoker
                        }
                    };
                    ServerboundGamePacket::RecipeBookChangeSettings(azalea_protocol::packets::game::s_recipe_book_change_settings::ServerboundRecipeBookChangeSettings {
                        book_type,
                        is_open,
                        is_filtering,
                    })
                }
                OutboundPacket::PlaceRecipe { container_id, recipe, shift_down } => {
                    ServerboundGamePacket::PlaceRecipe(azalea_protocol::packets::game::s_place_recipe::ServerboundPlaceRecipe {
                        container_id,
                        recipe: azalea_registry::identifier::Identifier::new(&recipe),
                        shift_down,
                    })
                }
                OutboundPacket::SetStructureBlock {
                    x,
                    y,
                    z,
                    update_type,
                    mode,
                    name,
                    offset_x,
                    offset_y,
                    offset_z,
                    size_x,
                    size_y,
                    size_z,
                    mirror,
                    rotation,
                    data,
                    integrity,
                    seed,
                    ignore_entities,
                    show_air,
                    show_bounding_box,
                } => {
                    let update_type = match update_type {
                        0 => {
                            azalea_protocol::packets::game::s_set_structure_block::UpdateType::UpdateData
                        }
                        1 => {
                            azalea_protocol::packets::game::s_set_structure_block::UpdateType::SaveArea
                        }
                        2 => {
                            azalea_protocol::packets::game::s_set_structure_block::UpdateType::LoadArea
                        }
                        _ => {
                            azalea_protocol::packets::game::s_set_structure_block::UpdateType::ScanArea
                        }
                    };
                    let mode = match mode {
                        0 => {
                            azalea_protocol::packets::game::s_set_structure_block::StructureMode::Save
                        }
                        1 => {
                            azalea_protocol::packets::game::s_set_structure_block::StructureMode::Load
                        }
                        2 => {
                            azalea_protocol::packets::game::s_set_structure_block::StructureMode::Corner
                        }
                        _ => {
                            azalea_protocol::packets::game::s_set_structure_block::StructureMode::Data
                        }
                    };
                    let mirror = match mirror {
                        0 => {
                            azalea_protocol::packets::game::s_set_structure_block::Mirror::None
                        }
                        1 => {
                            azalea_protocol::packets::game::s_set_structure_block::Mirror::LeftRight
                        }
                        _ => {
                            azalea_protocol::packets::game::s_set_structure_block::Mirror::FrontBack
                        }
                    };
                    let rotation = match rotation {
                        0 => {
                            azalea_protocol::packets::game::s_set_structure_block::Rotation::None
                        }
                        1 => {
                            azalea_protocol::packets::game::s_set_structure_block::Rotation::Clockwise90
                        }
                        2 => {
                            azalea_protocol::packets::game::s_set_structure_block::Rotation::Clockwise180
                        }
                        _ => {
                            azalea_protocol::packets::game::s_set_structure_block::Rotation::Counterclockwise90
                        }
                    };
                    ServerboundGamePacket::SetStructureBlock(azalea_protocol::packets::game::s_set_structure_block::ServerboundSetStructureBlock {
                        pos: AzBlockPos::new(x, y, z),
                        update_type,
                        mode,
                        name,
                        offset: azalea_protocol::packets::game::s_set_structure_block::BytePosition {
                            x: offset_x as u8,
                            y: offset_y as u8,
                            z: offset_z as u8,
                        },
                        size: azalea_protocol::packets::game::s_set_structure_block::BytePosition {
                            x: size_x,
                            y: size_y,
                            z: size_z,
                        },
                        mirror,
                        rotation,
                        data,
                        integrity,
                        seed,
                        flags: azalea_protocol::packets::game::s_set_structure_block::Flags {
                            ignore_entities,
                            show_air,
                            show_bounding_box,
                        },
                    })
                }
                OutboundPacket::SetJigsawBlock {
                    x,
                    y,
                    z,
                    name,
                    target,
                    pool,
                    final_state,
                    joint,
                    selection_priority,
                    placement_priority,
                } => {
                    ServerboundGamePacket::SetJigsawBlock(azalea_protocol::packets::game::s_set_jigsaw_block::ServerboundSetJigsawBlock {
                        pos: AzBlockPos::new(x, y, z),
                        name: azalea_registry::identifier::Identifier::new(&name),
                        target: azalea_registry::identifier::Identifier::new(&target),
                        pool: azalea_registry::identifier::Identifier::new(&pool),
                        final_state,
                        joint,
                        selection_priority,
                        placement_priority,
                    })
                }
                OutboundPacket::SetCommandBlock {
                    x,
                    y,
                    z,
                    command,
                    mode,
                    track_output,
                    conditional,
                    automatic,
                } => {
                    let mode = match mode {
                        0 => {
                            azalea_protocol::packets::game::s_set_command_block::Mode::Sequence
                        }
                        1 => {
                            azalea_protocol::packets::game::s_set_command_block::Mode::Auto
                        }
                        _ => {
                            azalea_protocol::packets::game::s_set_command_block::Mode::Redstone
                        }
                    };
                    ServerboundGamePacket::SetCommandBlock(azalea_protocol::packets::game::s_set_command_block::ServerboundSetCommandBlock {
                        pos: AzBlockPos::new(x, y, z),
                        command,
                        mode,
                        track_output,
                        conditional,
                        automatic,
                    })
                }
                OutboundPacket::SetCommandMinecart {
                    entity_id,
                    command,
                    track_output,
                } => {
                    ServerboundGamePacket::SetCommandMinecart(azalea_protocol::packets::game::s_set_command_minecart::ServerboundSetCommandMinecart {
                        entity: entity_id,
                        command,
                        track_output,
                    })
                }
                OutboundPacket::SetTestBlock { x, y, z, mode, message } => {
                    let mode = match mode {
                        0 => {
                            azalea_protocol::packets::game::s_set_test_block::TestBlockMode::Start
                        }
                        1 => {
                            azalea_protocol::packets::game::s_set_test_block::TestBlockMode::Log
                        }
                        2 => {
                            azalea_protocol::packets::game::s_set_test_block::TestBlockMode::Fail
                        }
                        _ => {
                            azalea_protocol::packets::game::s_set_test_block::TestBlockMode::Accept
                        }
                    };
                    ServerboundGamePacket::SetTestBlock(azalea_protocol::packets::game::s_set_test_block::ServerboundSetTestBlock {
                        position: AzBlockPos::new(x, y, z),
                        mode,
                        message,
                    })
                }
                OutboundPacket::SpectatorAction { .. } => {
                    continue;
                }
                OutboundPacket::PickItem { slot } => {
                    ServerboundGamePacket::PickItemFromBlock(azalea_protocol::packets::game::s_pick_item_from_block::ServerboundPickItemFromBlock {
                        slot,
                    })
                }
                OutboundPacket::PickItemFromEntity { id, include_data } => {
                    ServerboundGamePacket::PickItemFromEntity(azalea_protocol::packets::game::s_pick_item_from_entity::ServerboundPickItemFromEntity {
                        id: azalea_core::entity_id::MinecraftEntityId(id),
                        include_data,
                    })
                }
                OutboundPacket::SelectTrade { item } => {
                    ServerboundGamePacket::SelectTrade(azalea_protocol::packets::game::s_select_trade::ServerboundSelectTrade {
                        item,
                    })
                }
                OutboundPacket::SetBeacon { primary, secondary } => {
                    ServerboundGamePacket::SetBeacon(azalea_protocol::packets::game::s_set_beacon::ServerboundSetBeacon {
                        primary,
                        secondary,
                    })
                }
                OutboundPacket::RenameItem { name } => {
                    ServerboundGamePacket::RenameItem(azalea_protocol::packets::game::s_rename_item::ServerboundRenameItem {
                        name,
                    })
                }
                OutboundPacket::CookieResponse { key, payload } => {
                    ServerboundGamePacket::CookieResponse(azalea_protocol::packets::game::s_cookie_response::ServerboundCookieResponse {
                        key: azalea_registry::identifier::Identifier::from(key.as_str()),
                        payload,
                    })
                }
                OutboundPacket::AcceptCodeOfConduct => {
                    diagnostics::write_line(
                        "[ProtocolDebug][backend] direction=serverbound phase=play disposition=dropped_configuration_only packet=accept_code_of_conduct",
                    );
                    continue;
                }
                OutboundPacket::RequestGameRuleValues => {
                    diagnostics::write_line(
                        "[ProtocolDebug][backend] direction=serverbound phase=play disposition=dropped_no_1_21_11_packet packet=request_game_rule_values",
                    );
                    continue;
                }
                OutboundPacket::SetGameRule { key, value } => {
                    diagnostics::write_line(
                        &format!(
                            "[ProtocolDebug][backend] direction=serverbound phase=play disposition=dropped_no_1_21_11_packet packet=set_game_rule key={key} value={value}"
                        ),
                    );
                    continue;
                }
                OutboundPacket::LockDifficulty { locked } => {
                    ServerboundGamePacket::LockDifficulty(azalea_protocol::packets::game::s_lock_difficulty::ServerboundLockDifficulty {
                        locked,
                    })
                }
                OutboundPacket::SeenAdvancements { action, tab } => {
                    let action = match action {
                        BackendSeenAdvancementsAction::OpenedTab => {
                            azalea_protocol::packets::game::s_seen_advancements::Action::OpenedTab
                        }
                        BackendSeenAdvancementsAction::ClosedScreen => {
                            azalea_protocol::packets::game::s_seen_advancements::Action::ClosedScreen
                        }
                    };
                    ServerboundGamePacket::SeenAdvancements(azalea_protocol::packets::game::s_seen_advancements::ServerboundSeenAdvancements {
                        action,
                        tab: tab
                            .map(|t| azalea_registry::identifier::Identifier::from(
                                t.as_str(),
                            )),
                    })
                }
                OutboundPacket::ChatSessionUpdate {
                    session_id,
                    expires_at,
                    key,
                    key_signature,
                } => {
                    ServerboundGamePacket::ChatSessionUpdate(azalea_protocol::packets::game::s_chat_session_update::ServerboundChatSessionUpdate {
                        chat_session: azalea_protocol::packets::game::s_chat_session_update::RemoteChatSessionData {
                            session_id,
                            profile_public_key: azalea_protocol::packets::game::s_chat_session_update::ProfilePublicKeyData {
                                expires_at,
                                key,
                                key_signature,
                            },
                        },
                    })
                }
                OutboundPacket::ChatAck { offset } => {
                    ServerboundGamePacket::ChatAck(azalea_protocol::packets::game::s_chat_ack::ServerboundChatAck {
                        messages: offset,
                    })
                }
                OutboundPacket::BundleItemSelected { slot_id, selected_item_index } => {
                    ServerboundGamePacket::BundleItemSelected(azalea_protocol::packets::game::s_bundle_item_selected::ServerboundBundleItemSelected {
                        slot_id,
                        selected_item_index,
                    })
                }
                OutboundPacket::ContainerButtonClick { container_id, button_id } => {
                    ServerboundGamePacket::ContainerButtonClick(azalea_protocol::packets::game::s_container_button_click::ServerboundContainerButtonClick {
                        container_id,
                        button_id,
                    })
                }
                OutboundPacket::ContainerSlotStateChanged {
                    slot_id,
                    container_id,
                    new_state,
                } => {
                    ServerboundGamePacket::ContainerSlotStateChanged(azalea_protocol::packets::game::s_container_slot_state_changed::ServerboundContainerSlotStateChanged {
                        slot_id,
                        container_id,
                        new_state,
                    })
                }
                OutboundPacket::BlockEntityTagQuery { transaction_id, x, y, z } => {
                    ServerboundGamePacket::BlockEntityTagQuery(azalea_protocol::packets::game::s_block_entity_tag_query::ServerboundBlockEntityTagQuery {
                        transaction_id,
                        pos: azalea_core::position::BlockPos::new(x, y, z),
                    })
                }
                OutboundPacket::EntityTagQuery { transaction_id, entity_id } => {
                    ServerboundGamePacket::EntityTagQuery(azalea_protocol::packets::game::s_entity_tag_query::ServerboundEntityTagQuery {
                        transaction_id,
                        entity_id: azalea_core::entity_id::MinecraftEntityId(entity_id),
                    })
                }
                OutboundPacket::DebugSubscription { subscriptions } => {
                    let subs = subscriptions
                        .iter()
                        .filter_map(|s| std::str::FromStr::from_str(s).ok())
                        .collect::<Vec<_>>();
                    ServerboundGamePacket::DebugSubscriptionRequest(azalea_protocol::packets::game::s_debug_subscription_request::ServerboundDebugSubscriptionRequest {
                        subscriptions: subs,
                    })
                }
                OutboundPacket::TestInstanceBlockAction {
                    x,
                    y,
                    z,
                    action,
                    test,
                    size_x,
                    size_y,
                    size_z,
                    rotation,
                    ignore_entities,
                    status,
                    error_message,
                } => {
                    let act = match action {
                        0 => {
                            azalea_protocol::packets::game::s_test_instance_block_action::Action::Init
                        }
                        1 => {
                            azalea_protocol::packets::game::s_test_instance_block_action::Action::Query
                        }
                        2 => {
                            azalea_protocol::packets::game::s_test_instance_block_action::Action::Set
                        }
                        3 => {
                            azalea_protocol::packets::game::s_test_instance_block_action::Action::Reset
                        }
                        4 => {
                            azalea_protocol::packets::game::s_test_instance_block_action::Action::Save
                        }
                        5 => {
                            azalea_protocol::packets::game::s_test_instance_block_action::Action::Export
                        }
                        _ => {
                            azalea_protocol::packets::game::s_test_instance_block_action::Action::Run
                        }
                    };
                    let rot = match rotation {
                        0 => {
                            azalea_protocol::packets::game::s_set_structure_block::Rotation::None
                        }
                        1 => {
                            azalea_protocol::packets::game::s_set_structure_block::Rotation::Clockwise90
                        }
                        2 => {
                            azalea_protocol::packets::game::s_set_structure_block::Rotation::Clockwise180
                        }
                        _ => {
                            azalea_protocol::packets::game::s_set_structure_block::Rotation::Counterclockwise90
                        }
                    };
                    let stat = match status {
                        0 => {
                            azalea_protocol::packets::game::s_test_instance_block_action::TestInstanceBlockEntityStatus::Cleared
                        }
                        1 => {
                            azalea_protocol::packets::game::s_test_instance_block_action::TestInstanceBlockEntityStatus::Running
                        }
                        _ => {
                            azalea_protocol::packets::game::s_test_instance_block_action::TestInstanceBlockEntityStatus::Finished
                        }
                    };
                    let parsed_test = test
                        .and_then(|t| std::str::FromStr::from_str(&t).ok());
                    let err_msg = error_message
                        .map(|msg| azalea_chat::FormattedText::from(msg));
                    ServerboundGamePacket::TestInstanceBlockAction(azalea_protocol::packets::game::s_test_instance_block_action::ServerboundTestInstanceBlockAction {
                        pos: azalea_core::position::BlockPos::new(x, y, z),
                        action: act,
                        data: azalea_protocol::packets::game::s_test_instance_block_action::TestInstanceBlockEntityData {
                            test: parsed_test,
                            size: azalea_core::position::Vec3i::new(
                                size_x,
                                size_y,
                                size_z,
                            ),
                            rotation: rot,
                            ignore_entities,
                            status: stat,
                            error_message: err_msg,
                        },
                    })
                }
            };
            let trace_order = wire_trace
                .as_ref()
                .map(|_| wire_trace_order_for_writer.fetch_add(1, Ordering::Relaxed) + 1);
            let lock_started_at = Instant::now();
            let write_result = {
                let mut guard = writer_for_task.lock().await;
                let lock_wait = lock_started_at.elapsed();
                if let Some(writer) = guard.as_mut() {
                    let write_started_at = Instant::now();
                    if let (Some(order), Some(detail)) = (trace_order, wire_trace.as_deref()) {
                        wire_trace_begin(order, detail, wire_trace_dequeued_at, lock_wait);
                    }
                    let result = write_serverbound_game_packet_java_26_2(writer, &packet).await;
                    if let (Some(order), Some(detail)) = (trace_order, wire_trace.as_deref()) {
                        match &result {
                            Ok(()) => wire_trace_ok(
                                order,
                                detail,
                                wire_trace_dequeued_at,
                                write_started_at,
                            ),
                            Err(err) => wire_trace_error(
                                order,
                                detail,
                                wire_trace_dequeued_at,
                                write_started_at,
                                err,
                            ),
                        }
                    }
                    result
                } else {
                    diagnostics::write_line(
                        "[WireDebug][backend] phase=writer_missing source=normal",
                    );
                    break;
                }
            };
            if let Err(err) = write_result {
                if let Some(writer_error_tx) = writer_error_tx.take() {
                    let _ = writer_error_tx.send(err);
                }
                break;
            }
        }
    });
    let mut dimension_min_y = -64;
    let mut dimension_height = 384;
    let mut message_signature_cache = MessageSignatureCache::new();
    let mut online_players: HashMap<uuid::Uuid, String> = HashMap::new();
    let mut chunk_batch_size_calculator = ChunkBatchSizeCalculator::new();
    let mut pending_bundle_packets: Option<Vec<IncomingPlayPacket>> = None;
    let mut packet_queue = VecDeque::new();
    let mut packet_receive_diagnostics = GamePacketReceiveDiagnostics::default();
    let mut custom_payload_trace = CustomPayloadTrace::default();
    let mut keep_alive_replies_traced = 0_u8;
    macro_rules! local_server_stack {
        ($stack:expr) => {
            local_item_stack_from_azalea_with_enchantment_tooltips(
                $stack,
                &registries,
                &enchantment_tooltips,
            )
        };
    }
    loop {
        let incoming_packet = if let Some(packet) = packet_queue.pop_front() {
            packet
        } else {
            let raw_packet = tokio::select! {
                biased; write_error = & mut writer_error_rx => { writer_task.abort();
                return match write_error { Ok(err) => Err(NetworkError::Write(err)),
                Err(_) => Err(NetworkError::PlayDisconnect("play packet writer stopped"
                .to_string(),)), }; } read_result = reader.raw.read() => read_result ?,
            };
            if let Some(keep_alive_id) = raw_keep_alive_game_id(&raw_packet) {
                let trace_order = (diagnostics::file_enabled() && keep_alive_replies_traced < 16)
                    .then_some(wire_trace_order.as_ref());
                keep_alive_replies_traced = keep_alive_replies_traced.saturating_add(1);
                write_shared_game_packet(
                    &writer,
                    ServerboundGamePacket::KeepAlive(game::ServerboundKeepAlive {
                        id: keep_alive_id as u64,
                    }),
                    trace_order,
                )
                .await?;
                continue;
            }
            if let Some(packet_id) = raw_game_packet_id(&raw_packet) {
                let active_generation = menu_click_probe_for_reader
                    .lock()
                    .ok()
                    .and_then(|probe| probe.active_generation());
                if let Some(probe_generation) = active_generation {
                    if should_trace_game_packet_after_menu_click(packet_id) {
                        eprintln!(
                            "[MenuTrace] BACKEND_RECV_AFTER_CLICK gen={} packet_id={} name={} supported={} bytes={}",
                            probe_generation,
                            packet_id,
                            game_packet_name(packet_id),
                            is_supported_game_packet(packet_id),
                            raw_packet.len()
                        );
                    }
                }
            }
            let Some(packet) =
                read_supported_game_packet(&raw_packet, &mut packet_receive_diagnostics)?
            else {
                continue;
            };
            queue_play_bundle_packet(packet, &mut pending_bundle_packets, &mut packet_queue)?;
            let Some(packet) = packet_queue.pop_front() else {
                continue;
            };
            packet
        };
        let packet = match incoming_packet {
            IncomingPlayPacket::Azalea(packet) => packet,
            IncomingPlayPacket::LightUpdate(update) => {
                let _ = event_tx.send(NetworkEvent::LightUpdate(update));
                continue;
            }
            IncomingPlayPacket::ManualParticles(spawn) => {
                let _ = event_tx.send(NetworkEvent::LevelParticles(spawn));
                continue;
            }
            IncomingPlayPacket::MapItemData(event) | IncomingPlayPacket::Cooldown(event) => {
                let _ = event_tx.send(*event);
                continue;
            }
        };
        match packet {
            ClientboundGamePacket::Login(p) => {
                if VERBOSE_PROTOCOL_LOGS {
                    eprintln!("[AzaleaProtocol] Play login");
                }
                let (min_y, height) = dimension_bounds(&p.common, &registries)
                    .unwrap_or((dimension_min_y, dimension_height));
                dimension_min_y = min_y;
                dimension_height = height;
                let login_event = NetworkEvent::Login {
                    entity_id: p.player_id.0,
                    player_uuid,
                    game_mode: game_mode_id(p.common.game_type),
                    show_death_screen: p.show_death_screen,
                    online_mode: false,
                    hardcore: p.hardcore,
                    dimension: p.common.dimension.to_string(),
                    sea_level: p.common.sea_level,
                    min_y,
                    height,
                    biome_zoom_seed: p.common.seed,
                    biomes: extract_biome_render_data(&registries),
                };
                eprintln!(
                    "[AzaleaProtocol] PLAY login entity_id={} uuid={} game_mode={} dim={} sea_level={} min_y={} height={}",
                    p.player_id.0,
                    player_uuid,
                    game_mode_id(p.common.game_type),
                    p.common.dimension,
                    p.common.sea_level,
                    min_y,
                    height,
                );
                let _ = event_tx.send(login_event);
            }
            ClientboundGamePacket::Respawn(p) => {
                if VERBOSE_PROTOCOL_LOGS {
                    eprintln!("[AzaleaProtocol] Play respawn");
                }
                let (min_y, height) = dimension_bounds(&p.common, &registries)
                    .unwrap_or((dimension_min_y, dimension_height));
                dimension_min_y = min_y;
                dimension_height = height;
                let _ = event_tx.send(NetworkEvent::Respawn {
                    game_mode: game_mode_id(p.common.game_type),
                    dimension: p.common.dimension.to_string(),
                    sea_level: p.common.sea_level,
                    min_y,
                    height,
                    biome_zoom_seed: p.common.seed,
                    biomes: extract_biome_render_data(&registries),
                    data_to_keep: p.data_to_keep,
                });
            }
            ClientboundGamePacket::PlayerPosition(p) => {
                let raw_x = p.change.pos.x;
                let raw_y = p.change.pos.y;
                let raw_z = p.change.pos.z;
                let raw_yaw_degrees = p.change.look_direction.y_rot();
                let raw_pitch_degrees = p.change.look_direction.x_rot();
                let raw_delta_x = p.change.delta.x;
                let raw_delta_y = p.change.delta.y;
                let raw_delta_z = p.change.delta.z;
                let relative = RelativeMovementFlags {
                    x: p.relative.x,
                    y: p.relative.y,
                    z: p.relative.z,
                    y_rot: p.relative.y_rot,
                    x_rot: p.relative.x_rot,
                    delta_x: p.relative.delta_x,
                    delta_y: p.relative.delta_y,
                    delta_z: p.relative.delta_z,
                    rotate_delta: p.relative.rotate_delta,
                };
                let teleport_id = p.id;
                let _ = event_tx.send(NetworkEvent::PlayerPosition {
                    teleport_id,
                    raw_x,
                    raw_y,
                    raw_z,
                    raw_yaw_degrees,
                    raw_pitch_degrees,
                    raw_delta_x,
                    raw_delta_y,
                    raw_delta_z,
                    relative,
                });
            }
            ClientboundGamePacket::PlayerRotation(p) => {
                let _ = event_tx.send(NetworkEvent::PlayerRotation {
                    raw_yaw_degrees: p.y_rot,
                    raw_pitch_degrees: p.x_rot,
                    relative_y: p.relative_y,
                    relative_x: p.relative_x,
                });
            }
            ClientboundGamePacket::LevelChunkWithLight(p) => {
                match decode_chunk_loaded_event(
                    p,
                    dimension_min_y,
                    dimension_height,
                    registries.clone(),
                ) {
                    Ok(event) => {
                        let _ = event_tx.send(event);
                    }
                    Err(err) => send_disconnect(&event_tx, err.to_string()),
                }
            }
            ClientboundGamePacket::LightUpdate(p) => {
                let _ = event_tx.send(NetworkEvent::LightUpdate(LightUpdateEvent {
                    x: p.x,
                    z: p.z,
                    sky_layers: decoded_light_layers(
                        &p.light_data.sky_y_mask,
                        &p.light_data.empty_sky_y_mask,
                        &p.light_data.sky_updates,
                    ),
                    block_layers: decoded_light_layers(
                        &p.light_data.block_y_mask,
                        &p.light_data.empty_block_y_mask,
                        &p.light_data.block_updates,
                    ),
                }));
            }
            ClientboundGamePacket::ChunksBiomes(p) => {
                let section_count = (dimension_height / 16) as usize;
                let mut chunks = Vec::with_capacity(p.chunk_biome_data.len());
                for update in p.chunk_biome_data {
                    let mut cursor = Cursor::new(update.buffer.as_slice());
                    let mut sections = Vec::with_capacity(section_count);
                    for _ in 0..section_count {
                        let Ok(container) = azalea_world::palette::PalettedContainer::<
                            azalea_registry::data::Biome,
                        >::read(&mut cursor) else {
                            break;
                        };
                        let mut biomes = Vec::with_capacity(64);
                        for index in 0..64 {
                            let protocol_id = u32::from(container.get_at_index(index));
                            let registry = azalea_registry::identifier::Identifier::new(
                                "minecraft:worldgen/biome",
                            );
                            biomes.push(
                                registries
                                    .protocol_id_to_identifier(registry, protocol_id)
                                    .map(|id| ResourceLocation::from_string(&id.to_string()))
                                    .unwrap_or_else(|| {
                                        log_biome_fallback_once(protocol_id, "chunks_biomes");
                                        ResourceLocation::new("minecraft", "plains")
                                    }),
                            );
                        }
                        sections.push(biomes);
                    }
                    chunks.push(ChunkBiomeUpdate {
                        x: update.pos.x,
                        z: update.pos.z,
                        sections,
                    });
                }
                let _ = event_tx.send(NetworkEvent::ChunkBiomesUpdated(chunks));
            }
            ClientboundGamePacket::BlockChangedAck(p) => {
                let _ = event_tx.send(NetworkEvent::BlockChangedAck { sequence: p.seq });
            }
            ClientboundGamePacket::OpenScreen(p) => {
                if let Ok(mut mirror) = container_mirror_for_reader.lock() {
                    mirror.open(p.container_id);
                }
                eprintln!(
                    "[MenuDebug] BACKEND_RECV_OPEN_SCREEN container={} menu_type={} title={:?}",
                    p.container_id,
                    p.menu_type.to_u32(),
                    p.title
                );
                let _ = event_tx.send(NetworkEvent::OpenScreen {
                    container_id: p.container_id,
                    menu_type: p.menu_type.to_u32(),
                    title: p.title.to_string(),
                });
            }
            ClientboundGamePacket::ContainerSetContent(p) => {
                let is_current = match container_mirror_for_reader.lock() {
                    Ok(mut mirror) => {
                        let is_current = mirror.is_current_container(p.container_id);
                        if is_current {
                            mirror.set_content(p.container_id, &p.items, &p.carried_item);
                        }
                        is_current
                    }
                    Err(_) => true,
                };
                if is_current {
                    let items = p
                        .items
                        .iter()
                        .map(|item| local_server_stack!(item))
                        .collect::<Vec<_>>();
                    eprintln!(
                        "[MenuDebug] BACKEND_RECV_CONTAINER_CONTENT container={} state_id={} items={} carried={:?}",
                        p.container_id,
                        p.state_id,
                        items.len(),
                        p.carried_item
                    );
                    let _ = event_tx.send(NetworkEvent::ContainerContent {
                        container_id: p.container_id,
                        state_id: p.state_id,
                        items,
                        carried_item: local_server_stack!(&p.carried_item),
                    });
                } else {
                    eprintln!(
                        "[MenuDebug] BACKEND_IGNORE_STALE_CONTAINER_CONTENT container={} state_id={}",
                        p.container_id, p.state_id
                    );
                }
            }
            ClientboundGamePacket::ContainerSetSlot(p) => {
                if p.container_id == -2 {
                    if let Ok(mut mirror) = container_mirror_for_reader.lock() {
                        mirror.set_player_inventory_slot(p.slot, &p.item_stack);
                    }
                    eprintln!(
                        "[MenuDebug] BACKEND_RECV_DIRECT_PLAYER_SLOT container=-2 state_id={} slot={} item={:?}",
                        p.state_id, p.slot, p.item_stack
                    );
                    let _ = event_tx.send(NetworkEvent::InventorySlot {
                        slot: u32::from(p.slot),
                        item: local_server_stack!(&p.item_stack),
                        direct_player_slot: true,
                    });
                } else {
                    let is_current = match container_mirror_for_reader.lock() {
                        Ok(mut mirror) => {
                            let is_current = mirror.is_current_container(p.container_id);
                            if is_current {
                                mirror.set_slot(p.container_id, p.slot, &p.item_stack);
                            }
                            is_current
                        }
                        Err(_) => true,
                    };
                    if is_current {
                        eprintln!(
                            "[MenuDebug] BACKEND_RECV_CONTAINER_SLOT container={} state_id={} slot={} item={:?}",
                            p.container_id, p.state_id, p.slot, p.item_stack
                        );
                        let _ = event_tx.send(NetworkEvent::ContainerSlot {
                            container_id: p.container_id,
                            state_id: p.state_id,
                            slot: u32::from(p.slot),
                            item: local_server_stack!(&p.item_stack),
                        });
                    } else {
                        eprintln!(
                            "[MenuDebug] BACKEND_IGNORE_STALE_CONTAINER_SLOT container={} state_id={} slot={}",
                            p.container_id, p.state_id, p.slot
                        );
                    }
                }
            }
            ClientboundGamePacket::SetCursorItem(p) => {
                if let Ok(mut mirror) = container_mirror_for_reader.lock() {
                    mirror.set_cursor_item(&p.contents);
                }
                eprintln!(
                    "[MenuDebug] BACKEND_RECV_SET_CURSOR_ITEM item={:?}",
                    p.contents
                );
                let _ = event_tx.send(NetworkEvent::SetCursorItem {
                    item: local_server_stack!(&p.contents),
                });
            }
            ClientboundGamePacket::ContainerClose(p) => {
                if let Ok(mut mirror) = container_mirror_for_reader.lock() {
                    mirror.close(p.container_id);
                }
                eprintln!(
                    "[MenuDebug] BACKEND_RECV_CONTAINER_CLOSE container={}",
                    p.container_id
                );
                let _ = event_tx.send(NetworkEvent::ContainerClose {
                    container_id: p.container_id,
                });
            }
            ClientboundGamePacket::ContainerSetData(p) => {
                let _ = event_tx.send(NetworkEvent::ContainerSetData {
                    container_id: p.container_id,
                    id: p.id as u32,
                    value: p.value as i32,
                });
            }
            ClientboundGamePacket::Transfer(p) => {
                eprintln!(
                    "[TransferDebug] BACKEND_RECV_TRANSFER host={} port={}",
                    p.host, p.port
                );
                let _ = event_tx.send(NetworkEvent::TransferRequested {
                    host: p.host,
                    port: p.port,
                });
            }
            ClientboundGamePacket::SetPlayerInventory(p) => {
                if let Ok(player_slot) = u16::try_from(p.slot)
                    && let Ok(mut mirror) = container_mirror_for_reader.lock()
                {
                    mirror.set_player_inventory_slot(player_slot, &p.contents);
                }
                let _ = event_tx.send(NetworkEvent::InventorySlot {
                    slot: p.slot,
                    item: local_server_stack!(&p.contents),
                    direct_player_slot: true,
                });
            }
            ClientboundGamePacket::ForgetLevelChunk(p) => {
                let _ = event_tx.send(NetworkEvent::ChunkUnloaded {
                    x: p.pos.x,
                    z: p.pos.z,
                });
            }
            ClientboundGamePacket::SetChunkCacheCenter(p) => {
                let _ = event_tx.send(NetworkEvent::ChunkCacheCenter { x: p.x, z: p.z });
            }
            ClientboundGamePacket::SetChunkCacheRadius(p) => {
                let _ = event_tx.send(NetworkEvent::ChunkCacheRadius { radius: p.radius });
            }
            ClientboundGamePacket::SetSimulationDistance(p) => {
                eprintln!(
                    "[VanillaPacketDebug] BACKEND_RECV_SET_SIMULATION_DISTANCE distance={}",
                    p.simulation_distance
                );
                let _ = event_tx.send(NetworkEvent::SimulationDistance {
                    distance: p.simulation_distance,
                });
            }
            ClientboundGamePacket::SetDefaultSpawnPosition(p) => {
                let _ = event_tx.send(NetworkEvent::SetDefaultSpawnPosition {
                    x: p.global_pos.pos.x,
                    y: p.global_pos.pos.y,
                    z: p.global_pos.pos.z,
                    yaw: p.yaw,
                    pitch: p.pitch,
                });
            }
            ClientboundGamePacket::TagQuery(p) => {
                eprintln!(
                    "[VanillaPacketDebug] BACKEND_RECV_TAG_QUERY transaction_id={}",
                    p.transaction_id
                );
            }
            ClientboundGamePacket::SetPlayerTeam(p) => {
                use azalea_protocol::packets::game::c_set_player_team::Method;
                let op;
                let mut collision_rule = None;
                let mut members = None;
                let mut display_name = None;
                let mut prefix = None;
                let mut suffix = None;
                let mut team_color = -1;
                let mut nametag_visibility = None;
                let mut team_options = None;
                match p.method {
                    Method::Add((params, players)) => {
                        op = 0;
                        let fields = team_parameters(&params);
                        collision_rule = Some(fields.0);
                        display_name = Some(fields.1);
                        prefix = Some(fields.2);
                        suffix = Some(fields.3);
                        team_color = fields.4;
                        nametag_visibility = Some(fields.5);
                        team_options = Some(fields.6);
                        members = Some(players);
                    }
                    Method::Remove => {
                        op = 1;
                    }
                    Method::Change(params) => {
                        op = 2;
                        let fields = team_parameters(&params);
                        collision_rule = Some(fields.0);
                        display_name = Some(fields.1);
                        prefix = Some(fields.2);
                        suffix = Some(fields.3);
                        team_color = fields.4;
                        nametag_visibility = Some(fields.5);
                        team_options = Some(fields.6);
                    }
                    Method::Join(players) => {
                        op = 3;
                        members = Some(players);
                    }
                    Method::Leave(players) => {
                        op = 4;
                        members = Some(players);
                    }
                }
                let _ = event_tx.send(NetworkEvent::SetPlayerTeam {
                    name: p.name,
                    op,
                    collision_rule,
                    members,
                    display_name,
                    prefix,
                    suffix,
                    team_color,
                    nametag_visibility,
                    team_options,
                });
            }
            ClientboundGamePacket::InitializeBorder(p) => {
                let _ = event_tx.send(NetworkEvent::WorldBorderInitialize {
                    center_x: p.new_center_x,
                    center_z: p.new_center_z,
                    old_size: p.old_size,
                    new_size: p.new_size,
                    lerp_time: p.lerp_time,
                    absolute_max_size: p.new_absolute_max_size,
                    warning_blocks: p.warning_blocks,
                    warning_time: p.warning_time,
                });
            }
            ClientboundGamePacket::SetBorderCenter(p) => {
                let _ = event_tx.send(NetworkEvent::WorldBorderCenter {
                    center_x: p.new_center_x,
                    center_z: p.new_center_z,
                });
            }
            ClientboundGamePacket::SetBorderLerpSize(p) => {
                let _ = event_tx.send(NetworkEvent::WorldBorderLerpSize {
                    old_size: p.old_size,
                    new_size: p.new_size,
                    lerp_time: p.lerp_time,
                });
            }
            ClientboundGamePacket::SetBorderSize(p) => {
                let _ = event_tx.send(NetworkEvent::WorldBorderSize { size: p.size });
            }
            ClientboundGamePacket::SetBorderWarningDistance(p) => {
                let _ = event_tx.send(NetworkEvent::WorldBorderWarningDistance {
                    warning_blocks: p.warning_blocks,
                });
            }
            ClientboundGamePacket::SetBorderWarningDelay(p) => {
                let _ = event_tx.send(NetworkEvent::WorldBorderWarningDelay {
                    warning_time: p.warning_delay,
                });
            }
            ClientboundGamePacket::BossEvent(p) => {
                eprintln!(
                    "[VanillaPacketDebug] BACKEND_RECV_BOSS_EVENT id={} operation={:?}",
                    p.id, p.operation
                );
                let _ = event_tx.send(NetworkEvent::BossEvent {
                    id: p.id,
                    operation: map_boss_event_operation(&p.operation),
                });
            }
            ClientboundGamePacket::Animate(p) => {
                eprintln!(
                    "[VanillaPacketDebug] BACKEND_RECV_ANIMATE entity={} action={:?}",
                    p.id.0, p.action
                );
                let action = match p.action {
                    AnimationAction::SwingMainHand => 0,
                    AnimationAction::Hurt => 1,
                    AnimationAction::WakeUp => 2,
                    AnimationAction::SwingOffHand => 3,
                    AnimationAction::CriticalHit => 4,
                    AnimationAction::MagicCriticalHit => 5,
                };
                let _ = event_tx.send(NetworkEvent::EntityAnimation { id: p.id.0, action });
            }
            ClientboundGamePacket::BlockUpdate(p) => {
                let _ = event_tx.send(NetworkEvent::BlockUpdate(BlockUpdateEvent {
                    x: p.pos.x,
                    y: p.pos.y,
                    z: p.pos.z,
                    state_id: local_block_state_id_from_azalea(p.block_state).unwrap_or(0),
                }));
            }
            ClientboundGamePacket::LevelEvent(p) => {
                let _ = event_tx.send(NetworkEvent::LevelEvent {
                    event_type: p.event_type,
                    pos: BlockPos::new(p.pos.x, p.pos.y, p.pos.z),
                    data: p.data,
                    global_event: p.global_event,
                });
            }
            ClientboundGamePacket::Sound(p) => {
                let (sound, fixed_range) = match &p.sound {
                    azalea_registry::Holder::Reference(event) => (event.to_string(), None),
                    azalea_registry::Holder::Direct(custom) => {
                        (custom.sound_id.to_string(), custom.range)
                    }
                };
                eprintln!(
                    "[SoundTrace][backend] stage=packet event={sound} source={} pos=({:.2},{:.2},{:.2}) volume={:.3} pitch={:.3} fixed_range={fixed_range:?}",
                    p.source as u8,
                    f64::from(p.x) / 8.0,
                    f64::from(p.y) / 8.0,
                    f64::from(p.z) / 8.0,
                    p.volume,
                    p.pitch,
                );
                let _ = event_tx.send(NetworkEvent::Sound {
                    sound,
                    source: p.source as u8,
                    x: f64::from(p.x) / 8.0,
                    y: f64::from(p.y) / 8.0,
                    z: f64::from(p.z) / 8.0,
                    volume: p.volume,
                    pitch: p.pitch,
                    seed: p.seed,
                    fixed_range,
                });
            }
            ClientboundGamePacket::SoundEntity(p) => {
                eprintln!(
                    "[VanillaPacketDebug] BACKEND_RECV_SOUND_ENTITY entity={} source={:?} volume={:.2} pitch={:.2} seed={}",
                    p.id.0, p.source, p.volume, p.pitch, p.seed
                );
                let (sound, fixed_range) = match &p.sound {
                    azalea_registry::Holder::Reference(event) => (event.to_string(), None),
                    azalea_registry::Holder::Direct(custom) => {
                        (custom.sound_id.to_string(), custom.range)
                    }
                };
                let _ = event_tx.send(NetworkEvent::SoundEntity {
                    sound,
                    source: p.source as u8,
                    entity_id: p.id.0,
                    volume: p.volume,
                    pitch: p.pitch,
                    seed: p.seed,
                    fixed_range,
                });
            }
            ClientboundGamePacket::StopSound(p) => {
                let _ = event_tx.send(NetworkEvent::StopSound {
                    sound: p.name.as_ref().map(ToString::to_string),
                    source: p.source.map(|source| source as u8),
                });
            }
            ClientboundGamePacket::BlockEvent(p) => {
                let _ = event_tx.send(NetworkEvent::BlockEvent {
                    pos: BlockPos::new(p.pos.x, p.pos.y, p.pos.z),
                    action_id: p.action_id,
                    action_parameter: p.action_parameter,
                });
            }
            ClientboundGamePacket::BlockEntityData(p) => {
                let pos = BlockPos::new(p.pos.x, p.pos.y, p.pos.z);
                let mut nbt = Vec::new();
                p.tag.write(&mut nbt);
                let _ = event_tx.send(NetworkEvent::BlockEntityUpdate {
                    pos,
                    data: BlockEntityData {
                        kind: p.block_entity_type.to_str().to_string(),
                        nbt,
                    },
                });
            }
            ClientboundGamePacket::EntityEvent(p) => {
                eprintln!(
                    "[EntityDebug] BACKEND_RECV_ENTITY_EVENT id={} event_id={}",
                    p.entity_id.0, p.event_id
                );
                let _ = event_tx.send(NetworkEvent::EntityEvent {
                    id: p.entity_id.0,
                    event_id: p.event_id,
                });
            }
            ClientboundGamePacket::SectionBlocksUpdate(p) => {
                let base_x = p.section_pos.x << 4;
                let base_y = p.section_pos.y << 4;
                let base_z = p.section_pos.z << 4;
                let updates = p
                    .states
                    .into_iter()
                    .map(|entry| BlockUpdateEvent {
                        x: base_x + i32::from(entry.pos.x),
                        y: base_y + i32::from(entry.pos.y),
                        z: base_z + i32::from(entry.pos.z),
                        state_id: local_block_state_id_from_azalea(entry.state).unwrap_or(0),
                    })
                    .collect::<Vec<_>>();
                let _ = event_tx.send(NetworkEvent::SectionBlocksUpdate { updates });
            }
            ClientboundGamePacket::SetEntityData(p) => {
                let shared_flags = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 0 {
                        if let azalea_entity::EntityDataValue::Byte(flags) = &item.value {
                            return Some(*flags);
                        }
                    }
                    None
                });
                let living_flags = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 8 {
                        if let azalea_entity::EntityDataValue::Byte(flags) = &item.value {
                            return Some(*flags);
                        }
                    }
                    None
                });
                let custom_name = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 2 {
                        if let azalea_entity::EntityDataValue::OptionalFormattedText(value) =
                            &item.value
                        {
                            return Some(match value {
                                Some(component) => {
                                    BackendCustomNameUpdate::Set(component.to_ansi())
                                }
                                None => BackendCustomNameUpdate::Clear,
                            });
                        }
                    }
                    None
                });
                let custom_name_visible = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 3 {
                        if let azalea_entity::EntityDataValue::Boolean(visible) = &item.value {
                            return Some(*visible);
                        }
                    }
                    None
                });
                let display_interpolation_start_delta_ticks =
                    p.packed_items.0.iter().find_map(|item| {
                        if item.index == 8 {
                            if let azalea_entity::EntityDataValue::Int(v) = &item.value {
                                return Some(*v);
                            }
                        }
                        None
                    });
                let display_interpolation_duration = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 9 {
                        if let azalea_entity::EntityDataValue::Int(v) = &item.value {
                            return Some(*v);
                        }
                    }
                    None
                });
                let display_teleport_duration = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 10 {
                        if let azalea_entity::EntityDataValue::Int(v) = &item.value {
                            return Some(*v);
                        }
                    }
                    None
                });
                let display_translation = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 11 {
                        if let azalea_entity::EntityDataValue::Vector3(v) = &item.value {
                            return Some([v.x, v.y, v.z]);
                        }
                    }
                    None
                });
                let display_scale = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 12 {
                        if let azalea_entity::EntityDataValue::Vector3(v) = &item.value {
                            return Some([v.x, v.y, v.z]);
                        }
                    }
                    None
                });
                let display_left_rotation = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 13 {
                        if let azalea_entity::EntityDataValue::Quaternion(q) = &item.value {
                            return Some([q.x, q.y, q.z, q.w]);
                        }
                    }
                    None
                });
                let display_right_rotation = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 14 {
                        if let azalea_entity::EntityDataValue::Quaternion(q) = &item.value {
                            return Some([q.x, q.y, q.z, q.w]);
                        }
                    }
                    None
                });
                let display_billboard = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 15 {
                        if let azalea_entity::EntityDataValue::Byte(v) = &item.value {
                            return Some(*v as u8);
                        }
                    }
                    None
                });
                let display_brightness_override = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 16 {
                        if let azalea_entity::EntityDataValue::Int(v) = &item.value {
                            return Some(*v);
                        }
                    }
                    None
                });
                let display_view_range = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 17 {
                        if let azalea_entity::EntityDataValue::Float(v) = &item.value {
                            return Some(*v);
                        }
                    }
                    None
                });
                let display_shadow_radius = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 18 {
                        if let azalea_entity::EntityDataValue::Float(v) = &item.value {
                            return Some(*v);
                        }
                    }
                    None
                });
                let display_shadow_strength = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 19 {
                        if let azalea_entity::EntityDataValue::Float(v) = &item.value {
                            return Some(*v);
                        }
                    }
                    None
                });
                let display_width = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 20 {
                        if let azalea_entity::EntityDataValue::Float(v) = &item.value {
                            return Some(*v);
                        }
                    }
                    None
                });
                let display_height = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 21 {
                        if let azalea_entity::EntityDataValue::Float(v) = &item.value {
                            return Some(*v);
                        }
                    }
                    None
                });
                let display_glow_color_override = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 22 {
                        if let azalea_entity::EntityDataValue::Int(v) = &item.value {
                            return Some(*v);
                        }
                    }
                    None
                });
                let text_display_text = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 23 {
                        match &item.value {
                            azalea_entity::EntityDataValue::FormattedText(value) => {
                                Some(value.to_ansi())
                            }
                            azalea_entity::EntityDataValue::OptionalFormattedText(Some(value)) => {
                                Some(value.to_ansi())
                            }
                            azalea_entity::EntityDataValue::String(value) => {
                                Some(value.to_string())
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                });
                let text_display_line_width = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 24 {
                        if let azalea_entity::EntityDataValue::Int(v) = &item.value {
                            return Some(*v);
                        }
                    }
                    None
                });
                let text_display_background = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 25 {
                        if let azalea_entity::EntityDataValue::Int(v) = &item.value {
                            return Some(*v);
                        }
                    }
                    None
                });
                let text_display_text_opacity = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 26 {
                        if let azalea_entity::EntityDataValue::Byte(v) = &item.value {
                            return Some(*v as i8);
                        }
                    }
                    None
                });
                let text_display_flags = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 27 {
                        if let azalea_entity::EntityDataValue::Byte(v) = &item.value {
                            return Some(*v as u8);
                        }
                    }
                    None
                });
                let interaction_width = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 8 {
                        if let azalea_entity::EntityDataValue::Float(width) = &item.value {
                            return Some(*width);
                        }
                    }
                    None
                });
                let interaction_height = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 9 {
                        if let azalea_entity::EntityDataValue::Float(height) = &item.value {
                            return Some(*height);
                        }
                    }
                    None
                });
                let living_health = interaction_height;
                let interaction_response = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 10 {
                        if let azalea_entity::EntityDataValue::Boolean(response) = &item.value {
                            return Some(*response);
                        }
                    }
                    None
                });
                let pose = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 6 {
                        if let azalea_entity::EntityDataValue::Pose(pose) = &item.value {
                            return Some(backend_entity_pose_from_azalea(*pose));
                        }
                    }
                    None
                });
                let ticks_frozen = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 7 {
                        if let azalea_entity::EntityDataValue::Int(ticks) = &item.value {
                            return Some(*ticks);
                        }
                    }
                    None
                });
                let sleeping_pos = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 14 {
                        if let azalea_entity::EntityDataValue::OptionalBlockPos(pos) = &item.value {
                            return Some(backend_sleeping_pos_update_from_azalea(*pos));
                        }
                    }
                    None
                });
                let player_main_arm_right = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 15 {
                        if let azalea_entity::EntityDataValue::HumanoidArm(arm) = &item.value {
                            return Some(*arm == azalea_entity::HumanoidArm::Right);
                        }
                    }
                    None
                });
                let player_model_customisation = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 16 {
                        if let azalea_entity::EntityDataValue::Byte(flags) = &item.value {
                            return Some(*flags);
                        }
                    }
                    None
                });
                let armor_stand_client_flags = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 15 {
                        if let azalea_entity::EntityDataValue::Byte(flags) = &item.value {
                            return Some(*flags);
                        }
                    }
                    None
                });
                let armor_stand_pose = |index: u8| {
                    p.packed_items.0.iter().find_map(|item| {
                        if item.index == index {
                            if let azalea_entity::EntityDataValue::Rotations(rotations) =
                                &item.value
                            {
                                return Some(backend_rotations_from_azalea(rotations));
                            }
                        }
                        None
                    })
                };
                let armor_stand_head_pose = armor_stand_pose(16);
                let armor_stand_body_pose = armor_stand_pose(17);
                let armor_stand_left_arm_pose = armor_stand_pose(18);
                let armor_stand_right_arm_pose = armor_stand_pose(19);
                let armor_stand_left_leg_pose = armor_stand_pose(20);
                let armor_stand_right_leg_pose = armor_stand_pose(21);
                let player_absorption = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 17 {
                        if let azalea_entity::EntityDataValue::Float(absorption) = &item.value {
                            return Some(*absorption);
                        }
                    }
                    None
                });
                let item_stack = entity_item_stack_metadata(&p.packed_items.0)
                    .map(|stack| local_server_stack!(stack));
                let mob_size = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 16 {
                        if let azalea_entity::EntityDataValue::Int(size) = &item.value {
                            return Some(*size);
                        }
                    }
                    None
                });
                let special_metadata = decode_entity_special_metadata(&p.packed_items.0);
                let is_baby = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 16 {
                        if let azalea_entity::EntityDataValue::Boolean(b) = &item.value {
                            return Some(*b);
                        }
                    }
                    None
                });
                let is_aggressive = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 15 {
                        if let azalea_entity::EntityDataValue::Byte(flags) = &item.value {
                            return Some(flags & 0x4 != 0);
                        }
                    }
                    None
                });
                let camel_dashing = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 18 {
                        if let azalea_entity::EntityDataValue::Boolean(dash) = &item.value {
                            return Some(*dash);
                        }
                    }
                    None
                });
                let camel_last_pose_change_tick = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 19 {
                        if let azalea_entity::EntityDataValue::Long(v) = &item.value {
                            return Some(*v);
                        }
                    }
                    None
                });
                let piglin_is_baby = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 17 {
                        if let azalea_entity::EntityDataValue::Boolean(b) = &item.value {
                            return Some(*b);
                        }
                    }
                    None
                });
                let piglin_is_charging_crossbow = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 18 {
                        if let azalea_entity::EntityDataValue::Boolean(b) = &item.value {
                            return Some(*b);
                        }
                    }
                    None
                });
                let piglin_is_dancing = p.packed_items.0.iter().find_map(|item| {
                    if item.index == 19 {
                        if let azalea_entity::EntityDataValue::Boolean(b) = &item.value {
                            return Some(*b);
                        }
                    }
                    None
                });
                let _ = event_tx.send(NetworkEvent::EntityMetadata {
                    id: p.id.0,
                    item_stack,
                    no_gravity: entity_no_gravity_metadata(&p.packed_items.0),
                    item_frame_rotation: item_frame_rotation_metadata(&p.packed_items.0),
                    item_frame_direction: item_frame_direction_metadata(&p.packed_items.0),
                    shared_flags,
                    living_flags,
                    living_health,
                    custom_name,
                    custom_name_visible,
                    interaction_width,
                    interaction_height,
                    interaction_response,
                    pose,
                    sleeping_pos,
                    player_main_arm_right,
                    player_model_customisation,
                    player_absorption,
                    ticks_frozen,
                    mob_size,
                    primed_tnt_fuse: special_metadata.primed_tnt_fuse,
                    primed_tnt_block_state: special_metadata.primed_tnt_block_state,
                    end_crystal_beam_target: special_metadata.end_crystal_beam_target,
                    end_crystal_show_bottom: special_metadata.end_crystal_show_bottom,
                    abstract_arrow_flags: special_metadata.abstract_arrow_flags,
                    abstract_arrow_pierce_level: special_metadata.abstract_arrow_pierce_level,
                    abstract_arrow_in_ground: special_metadata.abstract_arrow_in_ground,
                    arrow_effect_color: special_metadata.arrow_effect_color,
                    experience_orb_value: special_metadata.experience_orb_value,
                    armor_stand_client_flags,
                    armor_stand_head_pose,
                    armor_stand_body_pose,
                    armor_stand_left_arm_pose,
                    armor_stand_right_arm_pose,
                    armor_stand_left_leg_pose,
                    armor_stand_right_leg_pose,
                    is_baby,
                    is_aggressive,
                    mob_flags: mob_flags_metadata(&p.packed_items.0),
                    camel_dashing,
                    camel_last_pose_change_tick,
                    mob_boolean_metadata: mob_boolean_metadata(&p.packed_items.0),
                    piglin_is_baby,
                    piglin_is_charging_crossbow,
                    piglin_is_dancing,
                    tamable_flags: special_metadata.tamable_flags,
                    creaking_can_move: special_metadata.creaking_can_move,
                    creaking_is_active: special_metadata.creaking_is_active,
                    creaking_is_tearing_down: special_metadata.creaking_is_tearing_down,
                    block_display_block_state: special_metadata.block_display_block_state,
                    item_display_item: special_metadata
                        .item_display_item
                        .as_ref()
                        .map(|stack| local_server_stack!(stack)),
                    item_display_context: special_metadata.item_display_context,
                    display_interpolation_start_delta_ticks,
                    display_interpolation_duration,
                    display_teleport_duration,
                    display_translation,
                    display_scale,
                    display_left_rotation,
                    display_right_rotation,
                    display_billboard,
                    display_brightness_override,
                    display_view_range,
                    display_shadow_radius,
                    display_shadow_strength,
                    display_width,
                    display_height,
                    display_glow_color_override,
                    text_display_text,
                    text_display_line_width,
                    text_display_background,
                    text_display_text_opacity,
                    text_display_flags,
                });
            }
            ClientboundGamePacket::SetEntityMotion(p) => {
                let _ = event_tx.send(entity_motion_event(&p));
            }
            ClientboundGamePacket::Explode(p) => {
                let _ = event_tx.send(NetworkEvent::Explosion {
                    center_x: p.center.x,
                    center_y: p.center.y,
                    center_z: p.center.z,
                    radius: p.radius,
                    block_count: p.block_count,
                    player_knockback: p
                        .player_knockback
                        .map(|knockback| [knockback.x, knockback.y, knockback.z]),
                    explosion_particle: map_azalea_particle_to_backend(
                        p.explosion_particle.clone(),
                    ),
                    explosion_sound: Some(p.explosion_sound.to_string()),
                    block_particles: p
                        .block_particles
                        .iter()
                        .filter_map(|weighted| {
                            Some(el_protocol_backend::BackendExplosionParticleInfo {
                                particle: map_azalea_particle_to_backend(
                                    weighted.value.particle.clone(),
                                )?,
                                scaling: weighted.value.scaling,
                                speed: weighted.value.speed,
                                weight: weighted.weight,
                            })
                        })
                        .collect(),
                });
            }
            ClientboundGamePacket::ChangeDifficulty(p) => {
                let _ = event_tx.send(NetworkEvent::ChangeDifficulty {
                    difficulty: p.difficulty.id(),
                    locked: p.locked,
                });
            }
            ClientboundGamePacket::TabList(p) => {
                let _ = event_tx.send(NetworkEvent::TabList {
                    header: p.header.to_ansi(),
                    footer: p.footer.to_ansi(),
                });
            }
            ClientboundGamePacket::HurtAnimation(p) => {
                let _ = event_tx.send(NetworkEvent::HurtAnimation {
                    entity_id: p.id.0,
                    yaw: p.yaw,
                });
            }
            ClientboundGamePacket::DamageEvent(p) => {
                let _ = event_tx.send(NetworkEvent::DamageEvent {
                    entity_id: p.entity_id.0,
                    source_type_id: p.source_type_id,
                    source_cause_id: p.source_cause_id.0.map(|id| id as i32),
                    source_direct_id: p.source_direct_id.0.map(|id| id as i32),
                    source_pos_x: p.source_position.as_ref().map(|pos| pos.x),
                    source_pos_y: p.source_position.as_ref().map(|pos| pos.y),
                    source_pos_z: p.source_position.as_ref().map(|pos| pos.z),
                });
            }
            ClientboundGamePacket::RecipeBookSettings(p) => {
                let _ = event_tx.send(NetworkEvent::RecipeBookSettings {
                    gui_open: p.book_settings.gui_open,
                    filtering_craftable: p.book_settings.filtering_craftable,
                    furnace_gui_open: p.book_settings.furnace_gui_open,
                    furnace_filtering_craftable: p.book_settings.furnace_filtering_craftable,
                    blast_furnace_gui_open: p.book_settings.blast_furnace_gui_open,
                    blast_furnace_filtering_craftable: p
                        .book_settings
                        .blast_furnace_filtering_craftable,
                    smoker_gui_open: p.book_settings.smoker_gui_open,
                    smoker_filtering_craftable: p.book_settings.smoker_filtering_craftable,
                });
            }
            ClientboundGamePacket::RecipeBookAdd(p) => {
                let _ = event_tx.send(NetworkEvent::RecipeBookAdd {
                    recipe_ids: p.entries.iter().map(|entry| entry.contents.id).collect(),
                    replace: p.replace,
                });
            }
            ClientboundGamePacket::RecipeBookRemove(p) => {
                let _ = event_tx.send(NetworkEvent::RecipeBookRemove {
                    recipe_ids: p.recipes.clone(),
                });
            }
            ClientboundGamePacket::UpdateRecipes(_p) => {
                let _ = event_tx.send(NetworkEvent::UpdateRecipes);
            }
            ClientboundGamePacket::UpdateTags(p) => {
                enchantment_tooltips.capture_tags(&p.tags);
                enchantment_tooltips.resolve_tag_ids(&registries);
                if p.tags.0.contains_key(&Identifier::new("minecraft:dialog")) {
                    emit_pause_screen_dialog(
                        &event_tx,
                        &registries,
                        &pause_screen_dialog_tag_ids(&p.tags),
                    );
                }
                let _ = event_tx.send(NetworkEvent::UpdateTags);
            }
            ClientboundGamePacket::PlaceGhostRecipe(p) => {
                let _ = event_tx.send(NetworkEvent::PlaceGhostRecipe {
                    container_id: p.container_id,
                });
            }
            ClientboundGamePacket::SetPassengers(p) => {
                let _ = event_tx.send(NetworkEvent::SetPassengers {
                    vehicle_id: p.vehicle.0,
                    passenger_ids: p.passengers.iter().map(|id| id.0).collect(),
                });
            }
            ClientboundGamePacket::PlayerCombatKill(p) => {
                let _ = event_tx.send(NetworkEvent::PlayerCombatKill {
                    player_id: p.player_id.0,
                    message: p.message.to_string(),
                });
            }
            ClientboundGamePacket::PlayerCombatEnter(_) => {
                let _ = event_tx.send(NetworkEvent::PlayerCombatEnter);
            }
            ClientboundGamePacket::PlayerCombatEnd(p) => {
                let _ = event_tx.send(NetworkEvent::PlayerCombatEnd {
                    duration: p.duration,
                });
            }
            ClientboundGamePacket::SetObjective(p) => {
                let (title, render_type_val, method_val, number_format) = match &p.method {
                    azalea_protocol::packets::game::c_set_objective::Method::Add {
                        display_name,
                        render_type,
                        number_format,
                    } => (
                        display_name.to_ansi(),
                        *render_type as u8,
                        0u8,
                        map_objective_number_format(number_format),
                    ),
                    azalea_protocol::packets::game::c_set_objective::Method::Remove => {
                        (String::new(), 0u8, 1u8, None)
                    }
                    azalea_protocol::packets::game::c_set_objective::Method::Change {
                        display_name,
                        render_type,
                        number_format,
                    } => (
                        display_name.to_ansi(),
                        *render_type as u8,
                        2u8,
                        map_objective_number_format(number_format),
                    ),
                };
                let _ = event_tx.send(NetworkEvent::SetObjective {
                    objective_name: p.objective_name.clone(),
                    title,
                    display_format: render_type_val,
                    method: method_val,
                    number_format,
                });
            }
            ClientboundGamePacket::SetScore(p) => {
                let display_name = p.display.as_ref().map(|d| d.to_ansi());
                let _ = event_tx.send(NetworkEvent::SetScore {
                    owner: p.owner.clone(),
                    objective_name: p.objective_name.clone(),
                    score: p.score,
                    display_name,
                    display_format: None,
                    number_format: p.number_format.as_ref().map(map_number_format),
                });
            }
            ClientboundGamePacket::ResetScore(p) => {
                let _ = event_tx.send(NetworkEvent::ResetScore {
                    owner: p.owner,
                    objective_name: p.objective_name,
                });
            }
            ClientboundGamePacket::SetDisplayObjective(p) => {
                let _ = event_tx.send(NetworkEvent::SetDisplayObjective {
                    slot: p.slot as u8,
                    objective_name: p.objective_name.clone(),
                });
            }
            ClientboundGamePacket::MerchantOffers(p) => {
                let offers = p
                    .offers
                    .iter()
                    .map(|o| format!("{:?} -> {:?}", o.base_cost_a.item, o.result.kind()))
                    .collect();
                let _ = event_tx.send(NetworkEvent::MerchantOffers {
                    container_id: p.container_id,
                    offers,
                    level: p.villager_level,
                    xp: p.villager_xp,
                    show_progress: p.show_progress,
                    can_restock: p.can_restock,
                });
            }
            ClientboundGamePacket::MapItemData(p) => {
                let _ = event_tx.send(NetworkEvent::MapItemData {
                    map_id: p.map_id,
                    scale: p.scale,
                    locked: p.locked,
                    tracking_position: p.decorations.is_some(),
                    color_patch: p.color_patch.0.map(|patch| {
                        el_protocol_backend::BackendMapPatch {
                            start_x: patch.start_x,
                            start_y: patch.start_y,
                            width: patch.width,
                            height: patch.height,
                            colors: patch.map_colors,
                        }
                    }),
                    decorations: p.decorations.map(|decorations| {
                        decorations
                            .into_iter()
                            .map(|decoration| {
                                let kind = azalea_registry::builtin::MapDecorationKind::from_u32(
                                    decoration.decoration_type as u32,
                                )
                                .unwrap_or(azalea_registry::builtin::MapDecorationKind::Player);
                                el_protocol_backend::BackendMapDecoration::from_vanilla_type(
                                    kind.to_str(),
                                    decoration.x,
                                    decoration.y,
                                    decoration.rot as u8,
                                    decoration.name.map(|name| name.to_string()),
                                )
                            })
                            .collect()
                    }),
                });
            }
            ClientboundGamePacket::OpenBook(p) => {
                let hand = match p.hand {
                    azalea_protocol::packets::game::s_interact::InteractionHand::MainHand => 0,
                    azalea_protocol::packets::game::s_interact::InteractionHand::OffHand => 1,
                };
                let _ = event_tx.send(NetworkEvent::OpenBook { hand });
            }
            ClientboundGamePacket::OpenSignEditor(p) => {
                let _ = event_tx.send(NetworkEvent::OpenSignEditor {
                    pos: [p.pos.x, p.pos.y, p.pos.z],
                    is_front_text: p.is_front_text,
                });
            }
            ClientboundGamePacket::AwardStats(packet) => {
                let _ = event_tx.send(NetworkEvent::AwardStats {
                    payload: backend_award_stats(&packet),
                });
            }
            ClientboundGamePacket::UpdateAdvancements(packet) => {
                let _ = event_tx.send(NetworkEvent::UpdateAdvancements {
                    payload: backend_update_advancements(&packet),
                });
            }
            ClientboundGamePacket::SetCamera(p) => {
                let _ = event_tx.send(NetworkEvent::SetCamera {
                    camera_id: p.camera_id.0,
                });
            }
            ClientboundGamePacket::ServerData(p) => {
                let _ = event_tx.send(NetworkEvent::ServerData {
                    motd: p.motd.to_string(),
                    icon_bytes: p.icon_bytes.clone(),
                });
            }
            ClientboundGamePacket::ServerLinks(p) => {
                let links = p
                    .links
                    .iter()
                    .map(|l| {
                        let label = match &l.kind {
                            azalea_protocol::common::server_links::ServerLinkKind::Known(kind) => {
                                format!("{:?}", kind)
                            }
                            azalea_protocol::common::server_links::ServerLinkKind::Component(
                                text,
                            ) => text.to_ansi(),
                        };
                        (label, l.link.clone())
                    })
                    .collect::<Vec<_>>();
                let _ = event_tx.send(NetworkEvent::ServerLinks { links });
            }
            ClientboundGamePacket::CustomReportDetails(p) => {
                let details = p
                    .details
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                let _ = event_tx.send(NetworkEvent::CustomReportDetails { details });
            }
            ClientboundGamePacket::DebugSample(p) => {
                let _ = event_tx.send(NetworkEvent::DebugSample {
                    sample: p.sample.clone(),
                    sample_type: p.debug_sample_type as u8,
                });
            }
            ClientboundGamePacket::Waypoint(p) => {
                let identifier = match &p.waypoint.identifier {
                    azalea_protocol::packets::game::c_waypoint::WaypointIdentifier::String(s) => {
                        BackendWaypointIdentifier::String(s.clone())
                    }
                    azalea_protocol::packets::game::c_waypoint::WaypointIdentifier::Uuid(u) => {
                        BackendWaypointIdentifier::Uuid(*u)
                    }
                };
                let color = p.waypoint.icon.color.map(|color| {
                    (u32::from(color.red()) << 16)
                        | (u32::from(color.green()) << 8)
                        | u32::from(color.blue())
                });
                let data = match &p.waypoint.data {
                    azalea_protocol::packets::game::c_waypoint::WaypointData::Vec3i(v) => {
                        BackendWaypointData::Vec3i {
                            x: v.x,
                            y: v.y,
                            z: v.z,
                        }
                    }
                    azalea_protocol::packets::game::c_waypoint::WaypointData::Chunk { x, z } => {
                        BackendWaypointData::Chunk { x: *x, z: *z }
                    }
                    azalea_protocol::packets::game::c_waypoint::WaypointData::Azimuth { angle } => {
                        BackendWaypointData::Azimuth { angle: *angle }
                    }
                    azalea_protocol::packets::game::c_waypoint::WaypointData::Empty => {
                        BackendWaypointData::Empty
                    }
                };
                let _ = event_tx.send(NetworkEvent::Waypoint {
                    operation: p.operation as u8,
                    waypoint: BackendWaypoint {
                        identifier,
                        icon: BackendWaypointIcon {
                            style: p.waypoint.icon.style.to_string(),
                            color,
                        },
                        data,
                    },
                });
            }
            ClientboundGamePacket::MoveVehicle(p) => {
                let _ = event_tx.send(NetworkEvent::MoveVehicle {
                    x: p.pos.x,
                    y: p.pos.y,
                    z: p.pos.z,
                    yaw_degrees: p.look_direction.y_rot(),
                    pitch_degrees: p.look_direction.x_rot(),
                });
            }
            ClientboundGamePacket::MoveMinecartAlongTrack(p) => {
                let _ = event_tx.send(NetworkEvent::MoveMinecartAlongTrack {
                    entity_id: p.entity_id.0,
                    steps: p.lerp_steps.clone(),
                });
            }
            ClientboundGamePacket::AddEntity(p) => {
                let kind = p.entity_type.to_str().to_string();
                let falling_block_state_id =
                    canonical_falling_block_state_id_from_add_entity_data(&kind, p.data);
                let initial_velocity = p.movement.to_vec3();
                if let Ok(mut uuid_by_id) = entity_uuid_by_id_for_reader.lock() {
                    uuid_by_id.insert(p.id.0, p.uuid);
                }
                log::debug!(
                    "[Entity] AddEntity id={} kind={:?} pos=({:.2},{:.2},{:.2})",
                    p.id.0,
                    kind,
                    p.position.x,
                    p.position.y,
                    p.position.z
                );
                let _ = event_tx.send(NetworkEvent::EntityAdded(RemoteEntityEvent {
                    id: p.id.0,
                    uuid: p.uuid,
                    kind,
                    x: p.position.x,
                    y: p.position.y,
                    z: p.position.z,
                    yaw_degrees: packed_rotation_degrees(p.y_rot),
                    pitch_degrees: packed_rotation_degrees(p.x_rot),
                    head_yaw_degrees: packed_rotation_degrees(p.y_head_rot),
                    spawn_data: p.data,
                    falling_block_state_id,
                    velocity_x: initial_velocity.x,
                    velocity_y: initial_velocity.y,
                    velocity_z: initial_velocity.z,
                }));
            }
            ClientboundGamePacket::EntityPositionSync(p) => {
                let _ = event_tx.send(NetworkEvent::EntityPositionSync {
                    id: p.id.0,
                    x: p.values.pos.x,
                    y: p.values.pos.y,
                    z: p.values.pos.z,
                    yaw_degrees: p.values.look_direction.y_rot(),
                    pitch_degrees: p.values.look_direction.x_rot(),
                    on_ground: p.on_ground,
                });
            }
            ClientboundGamePacket::MoveEntityPos(p) => {
                let _ = event_tx.send(NetworkEvent::EntityDeltaMoved {
                    id: p.entity_id.0,
                    xa: p.delta.xa,
                    ya: p.delta.ya,
                    za: p.delta.za,
                    has_position: true,
                    yaw_degrees: None,
                    pitch_degrees: None,
                    on_ground: p.on_ground,
                });
            }
            ClientboundGamePacket::MoveEntityPosRot(p) => {
                let _ = event_tx.send(NetworkEvent::EntityDeltaMoved {
                    id: p.entity_id.0,
                    xa: p.delta.xa,
                    ya: p.delta.ya,
                    za: p.delta.za,
                    has_position: true,
                    yaw_degrees: Some(packed_rotation_degrees(p.look_direction.y_rot)),
                    pitch_degrees: Some(packed_rotation_degrees(p.look_direction.x_rot)),
                    on_ground: p.on_ground,
                });
            }
            ClientboundGamePacket::MoveEntityRot(p) => {
                let _ = event_tx.send(NetworkEvent::EntityDeltaMoved {
                    id: p.entity_id.0,
                    xa: 0,
                    ya: 0,
                    za: 0,
                    has_position: false,
                    yaw_degrees: Some(packed_rotation_degrees(p.look_direction.y_rot)),
                    pitch_degrees: Some(packed_rotation_degrees(p.look_direction.x_rot)),
                    on_ground: p.on_ground,
                });
            }
            ClientboundGamePacket::RotateHead(p) => {
                let _ = event_tx.send(NetworkEvent::EntityHeadLook {
                    id: p.entity_id.0,
                    y_head_rot_degrees: packed_rotation_degrees(p.y_head_rot),
                });
            }
            ClientboundGamePacket::SetEquipment(p) => {
                let mut main_hand = None;
                let mut off_hand = None;
                let mut head = None;
                let mut chest = None;
                let mut legs = None;
                let mut feet = None;
                let mut body = None;
                let mut saddle = None;
                for (slot, item) in p.slots.slots {
                    match slot {
                        azalea_inventory::components::EquipmentSlot::Mainhand => {
                            main_hand = Some(local_server_stack!(&item));
                        }
                        azalea_inventory::components::EquipmentSlot::Offhand => {
                            off_hand = Some(local_server_stack!(&item));
                        }
                        azalea_inventory::components::EquipmentSlot::Head => {
                            head = Some(local_server_stack!(&item));
                        }
                        azalea_inventory::components::EquipmentSlot::Chest => {
                            chest = Some(local_server_stack!(&item));
                        }
                        azalea_inventory::components::EquipmentSlot::Legs => {
                            legs = Some(local_server_stack!(&item));
                        }
                        azalea_inventory::components::EquipmentSlot::Feet => {
                            feet = Some(local_server_stack!(&item));
                        }
                        azalea_inventory::components::EquipmentSlot::Body => {
                            body = Some(local_server_stack!(&item));
                        }
                        azalea_inventory::components::EquipmentSlot::Saddle => {
                            saddle = Some(local_server_stack!(&item));
                        }
                    }
                }
                if main_hand.is_some()
                    || off_hand.is_some()
                    || head.is_some()
                    || chest.is_some()
                    || legs.is_some()
                    || feet.is_some()
                    || body.is_some()
                    || saddle.is_some()
                {
                    let _ = event_tx.send(NetworkEvent::EntityEquipment {
                        id: p.entity_id.0,
                        main_hand,
                        off_hand,
                        head,
                        chest,
                        legs,
                        feet,
                        body,
                        saddle,
                    });
                }
            }
            ClientboundGamePacket::TeleportEntity(p) => {
                let _ = event_tx.send(NetworkEvent::EntityTeleport {
                    id: p.id.0,
                    raw_x: p.change.pos.x,
                    raw_y: p.change.pos.y,
                    raw_z: p.change.pos.z,
                    raw_delta_x: p.change.delta.x,
                    raw_delta_y: p.change.delta.y,
                    raw_delta_z: p.change.delta.z,
                    raw_yaw_degrees: p.change.look_direction.y_rot(),
                    raw_pitch_degrees: p.change.look_direction.x_rot(),
                    relative: RelativeMovementFlags {
                        x: p.relative.x,
                        y: p.relative.y,
                        z: p.relative.z,
                        y_rot: p.relative.y_rot,
                        x_rot: p.relative.x_rot,
                        delta_x: p.relative.delta_x,
                        delta_y: p.relative.delta_y,
                        delta_z: p.relative.delta_z,
                        rotate_delta: p.relative.rotate_delta,
                    },
                    on_ground: p.on_ground,
                });
            }
            ClientboundGamePacket::RemoveEntities(p) => {
                if let Ok(mut uuid_by_id) = entity_uuid_by_id_for_reader.lock() {
                    for id in &p.entity_ids {
                        uuid_by_id.remove(&id.0);
                    }
                }
                let _ = event_tx.send(NetworkEvent::EntitiesRemoved {
                    ids: p.entity_ids.into_iter().map(|id| id.0).collect(),
                });
            }
            ClientboundGamePacket::TakeItemEntity(p) => {
                let _ = event_tx.send(NetworkEvent::TakeItem {
                    item_id: p.item_id as i32,
                    player_id: p.player_id.0,
                    amount: p.amount,
                });
            }
            ClientboundGamePacket::KeepAlive(p) => {
                let trace_order = (diagnostics::file_enabled() && keep_alive_replies_traced < 16)
                    .then_some(wire_trace_order.as_ref());
                keep_alive_replies_traced = keep_alive_replies_traced.saturating_add(1);
                write_shared_game_packet(
                    &writer,
                    ServerboundGamePacket::KeepAlive(game::ServerboundKeepAlive { id: p.id }),
                    trace_order,
                )
                .await?;
            }
            ClientboundGamePacket::TickingState(p) => {
                let _ = event_tx.send(NetworkEvent::TickingState {
                    tick_rate: p.tick_rate,
                    is_frozen: p.is_frozen,
                });
            }
            ClientboundGamePacket::TickingStep(p) => {
                let _ = event_tx.send(NetworkEvent::TickingStep {
                    tick_steps: p.tick_steps,
                });
            }
            ClientboundGamePacket::Ping(p) => {
                let _ = event_tx.send(NetworkEvent::Ping { id: p.id });
            }
            ClientboundGamePacket::PongResponse(p) => {
                let _ = event_tx.send(NetworkEvent::PongResponse { time: p.time });
            }
            ClientboundGamePacket::ProjectilePower(p) => {
                let _ = event_tx.send(NetworkEvent::ProjectilePower {
                    entity_id: p.id.0,
                    acceleration_power: p.acceleration_power,
                });
            }
            ClientboundGamePacket::PlayerLookAt(p) => {
                let at_entity = p.entity.is_some();
                let (entity_id, to_anchor) = match p.entity {
                    Some(at_entity_data) => (
                        Some(at_entity_data.entity as i32),
                        Some(at_entity_data.to_anchor as u8),
                    ),
                    None => (None, None),
                };
                let _ = event_tx.send(NetworkEvent::PlayerLookAt {
                    from_anchor: p.from_anchor as u8,
                    x: p.pos.x,
                    y: p.pos.y,
                    z: p.pos.z,
                    at_entity,
                    entity_id,
                    to_anchor,
                });
            }
            ClientboundGamePacket::BlockDestruction(p) => {
                let _ = event_tx.send(NetworkEvent::BlockDestruction {
                    id: p.id.0,
                    pos: [p.pos.x, p.pos.y, p.pos.z],
                    progress: p.progress as i32,
                });
            }
            ClientboundGamePacket::SetEntityLink(p) => {
                let _ = event_tx.send(NetworkEvent::SetEntityLink {
                    source_id: p.source_id.0,
                    dest_id: p.dest_id.0,
                });
            }
            ClientboundGamePacket::LevelParticles(p) => {
                let Some(particle) = map_azalea_particle_to_backend(p.particle.clone()) else {
                    eprintln!(
                        "[ParticleDebug] disposition=dropped_unsupported position=({},{},{}) count={}",
                        p.pos.x, p.pos.y, p.pos.z, p.count
                    );
                    return Ok(());
                };
                let _ = event_tx.send(NetworkEvent::LevelParticles(Box::new(
                    BackendParticleSpawn {
                        override_limiter: p.override_limiter,
                        always_show: p.always_show,
                        pos: [p.pos.x, p.pos.y, p.pos.z],
                        x_dist: p.x_dist,
                        y_dist: p.y_dist,
                        z_dist: p.z_dist,
                        max_speed: p.max_speed,
                        count: p.count,
                        particle,
                    },
                )));
            }
            ClientboundGamePacket::DebugBlockValue(p) => {
                let _ = event_tx.send(NetworkEvent::DebugBlockValue {
                    pos: [p.block_pos.x, p.block_pos.y, p.block_pos.z],
                    update: map_debug_subscription_update(p.update),
                });
            }
            ClientboundGamePacket::DebugChunkValue(p) => {
                let _ = event_tx.send(NetworkEvent::DebugChunkValue {
                    chunk_x: p.chunk_pos.x,
                    chunk_z: p.chunk_pos.z,
                    update: map_debug_subscription_update(p.update),
                });
            }
            ClientboundGamePacket::DebugEntityValue(p) => {
                let _ = event_tx.send(NetworkEvent::DebugEntityValue {
                    entity_id: p.entity_id.0,
                    update: map_debug_subscription_update(p.update),
                });
            }
            ClientboundGamePacket::DebugEvent(p) => {
                let _ = event_tx.send(NetworkEvent::DebugEvent {
                    event: map_debug_subscription_event(p.event),
                });
            }
            ClientboundGamePacket::GameTestHighlightPos(p) => {
                let _ = event_tx.send(NetworkEvent::GameTestHighlightPos {
                    absolute_pos: [p.absolute_pos.x, p.absolute_pos.y, p.absolute_pos.z],
                    relative_pos: [p.relative_pos.x, p.relative_pos.y, p.relative_pos.z],
                });
            }
            ClientboundGamePacket::MountScreenOpen(p) => {
                let _ = event_tx.send(NetworkEvent::MountScreenOpen {
                    container_id: p.container_id,
                    inventory_columns: p.inventory_columns,
                    entity_id: p.entity_id.0,
                });
            }
            ClientboundGamePacket::SelectAdvancementsTab(p) => {
                let _ = event_tx.send(NetworkEvent::SelectAdvancementsTab {
                    tab: p.tab.map(|id| id.to_string()),
                });
            }
            ClientboundGamePacket::TestInstanceBlockStatus(p) => {
                let _ = event_tx.send(NetworkEvent::TestInstanceBlockStatus {
                    status: p.status.to_string(),
                    size: p
                        .size
                        .map(|size| [size.x as i32, size.y as i32, size.z as i32]),
                });
            }
            ClientboundGamePacket::ShowDialog(p) => {
                let (dialog_kind, dialog_nbt) = map_dialog_holder(&p.dialog, &registries);
                let _ = event_tx.send(NetworkEvent::ShowDialog {
                    dialog_kind,
                    dialog_nbt,
                });
            }
            ClientboundGamePacket::ClearDialog(_) => {
                let _ = event_tx.send(NetworkEvent::ClearDialog);
            }
            ClientboundGamePacket::CookieRequest(p) => {
                let payload = server_cookies.get(&p.key.to_string()).cloned();
                eprintln!(
                    "[CookieDebug] PLAY_RECV_COOKIE_REQUEST key={} has_payload={} payload_len={}",
                    p.key,
                    payload.is_some(),
                    payload.as_ref().map_or(0, |bytes: &Vec<u8>| bytes.len())
                );
                let _ = event_tx.send(NetworkEvent::CookieRequest {
                    key: p.key.to_string(),
                    payload,
                });
            }
            ClientboundGamePacket::StoreCookie(p) => {
                eprintln!(
                    "[CookieDebug] PLAY_RECV_STORE_COOKIE key={} payload_len={}",
                    p.key,
                    p.payload.len()
                );
                let _ = event_tx.send(NetworkEvent::StoreCookie {
                    key: p.key.to_string(),
                    payload: p.payload.clone(),
                });
                server_cookies.insert(p.key.to_string(), p.payload);
            }
            ClientboundGamePacket::CustomPayload(p) => {
                let channel = p.identifier.to_string();
                custom_payload_trace.record("clientbound", "play", &channel, p.data.0.len());
                el_protocol_backend::payload::validate_custom_payload(&channel, &p.data.0, false)
                    .map_err(NetworkError::PlayDisconnect)?;
                let _ = event_tx.send(NetworkEvent::CustomPayload {
                    channel,
                    data: p.data.0,
                });
            }
            ClientboundGamePacket::ResourcePackPop(p) => {
                eprintln!(
                    "[ResourcePackDebug] PLAY_RECV_RESOURCE_PACK_POP id={:?}",
                    p.id
                );
                let _ = event_tx.send(NetworkEvent::ResourcePackPop { id: p.id });
            }
            ClientboundGamePacket::ResourcePackPush(p) => {
                eprintln!(
                    "[ResourcePackDebug] PLAY_RECV_RESOURCE_PACK_PUSH id={} required={} url={:?} hash={:?} prompt_present={}",
                    p.id,
                    p.required,
                    p.url,
                    p.hash,
                    p.prompt.is_some()
                );
                let prompt = p.prompt.as_ref().map(|prompt| prompt.to_ansi());
                let _ = event_tx.send(NetworkEvent::ResourcePackPush {
                    id: p.id,
                    url: p.url.clone(),
                    hash: p.hash.clone(),
                    required: p.required,
                    prompt,
                    server_pack_status: args.server_pack_status,
                    initial_status_sent: false,
                });
                eprintln!(
                    "[ResourcePackDebug] PLAY_QUEUE_RESOURCE_PACK_STATUS id={} preference={} required={}",
                    p.id, args.server_pack_status, p.required
                );
            }
            ClientboundGamePacket::ChunkBatchStart(_) => {
                if VERBOSE_PROTOCOL_LOGS {
                    eprintln!("[AzaleaProtocol] Chunk batch start");
                }
                chunk_batch_size_calculator.on_batch_start();
                let _ = event_tx.send(NetworkEvent::ChunkBatchStart);
            }
            ClientboundGamePacket::ChunkBatchFinished(p) => {
                if VERBOSE_PROTOCOL_LOGS {
                    eprintln!(
                        "[AzaleaProtocol] Chunk batch finished: batch_size={}",
                        p.batch_size
                    );
                }
                chunk_batch_size_calculator.on_batch_finished(p.batch_size);
                let desired_chunks_per_tick = chunk_batch_size_calculator.desired_chunks_per_tick();
                write_shared_game_packet(
                    &writer,
                    ServerboundGamePacket::ChunkBatchReceived(ServerboundChunkBatchReceived {
                        desired_chunks_per_tick,
                    }),
                    None,
                )
                .await?;
                let _ = event_tx.send(NetworkEvent::ChunkBatchFinished {
                    batch_size: p.batch_size,
                });
            }
            ClientboundGamePacket::Disconnect(p) => {
                eprintln!("[AzaleaProtocol][DISCONNECT] Play disconnect: {}", p.reason);
                return Err(NetworkError::PlayDisconnect(p.reason.to_string()));
            }
            ClientboundGamePacket::SystemChat(p) => {
                let interaction_probe = system_chat_interaction_probe(&p.content);
                log_system_chat_interaction_probe(&interaction_probe, p.overlay);
                let (message, mut click_spans) = system_chat_text_and_click_spans(&p.content);
                enrich_hover_item_tooltips(&mut click_spans, &enchantment_tooltips);
                log_system_chat_click_metadata(&message, p.overlay, &click_spans);
                let _ = event_tx.send(NetworkEvent::SystemChat {
                    message,
                    overlay: p.overlay,
                    click_spans,
                });
            }
            ClientboundGamePacket::Commands(p) => {
                let root_commands = p
                    .entries
                    .get(p.root_index as usize)
                    .into_iter()
                    .flat_map(|root| root.children.iter())
                    .filter_map(|child| p.entries.get(*child as usize))
                    .filter_map(|node| match &node.node_type {
                        game::c_commands::NodeType::Literal { name } => Some(name.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let nodes = p.entries.iter().map(backend_command_node).collect();
                let _ = event_tx.send(NetworkEvent::CommandTree {
                    root_commands,
                    root_index: p.root_index,
                    nodes,
                });
            }
            ClientboundGamePacket::CommandSuggestions(p) => {
                let range = p.suggestions.range();
                let suggestions = p
                    .suggestions
                    .list()
                    .iter()
                    .map(|suggestion| NetworkChatSuggestion {
                        text: suggestion.text(),
                        tooltip: suggestion.tooltip.clone(),
                    })
                    .collect();
                let _ = event_tx.send(NetworkEvent::CommandSuggestions {
                    id: p.id,
                    range_start: range.start(),
                    range_end: range.end(),
                    suggestions,
                });
            }
            ClientboundGamePacket::CustomChatCompletions(p) => {
                let action = match p.action {
                    game::c_custom_chat_completions::Action::Add => ChatCompletionAction::Add,
                    game::c_custom_chat_completions::Action::Remove => ChatCompletionAction::Remove,
                    game::c_custom_chat_completions::Action::Set => ChatCompletionAction::Set,
                };
                let _ = event_tx.send(NetworkEvent::CustomChatCompletions {
                    action,
                    entries: p.entries,
                });
            }
            ClientboundGamePacket::SetActionBarText(p) => {
                let _ = event_tx.send(NetworkEvent::ActionBar {
                    message: p.text.to_ansi(),
                });
            }
            ClientboundGamePacket::ClearTitles(p) => {
                let _ = event_tx.send(NetworkEvent::ClearTitles {
                    reset_times: p.reset_times,
                });
            }
            ClientboundGamePacket::SetSubtitleText(p) => {
                let _ = event_tx.send(NetworkEvent::SubtitleText {
                    message: p.text.to_ansi(),
                });
            }
            ClientboundGamePacket::SetTitleText(p) => {
                let _ = event_tx.send(NetworkEvent::TitleText {
                    message: p.text.to_ansi(),
                });
            }
            ClientboundGamePacket::SetTitlesAnimation(p) => {
                let _ = event_tx.send(NetworkEvent::TitlesAnimation {
                    fade_in: p.fade_in,
                    stay: p.stay,
                    fade_out: p.fade_out,
                });
            }
            ClientboundGamePacket::PlayerInfoUpdate(p) => {
                let actions = &p.actions;
                let update_entries = p
                    .entries
                    .iter()
                    .map(|entry| {
                        let mut out = BackendPlayerInfoUpdateEntry {
                            uuid: entry.profile.uuid,
                            add_player: None,
                            game_mode: None,
                            listed: None,
                            latency: None,
                            display_name: None,
                            list_order: None,
                            show_hat: None,
                        };
                        if actions.add_player {
                            let (texture_url, model, cape_url) = skin_from_profile_properties(
                                entry.profile.uuid,
                                &entry.profile.properties,
                            );
                            out.add_player = Some(BackendPlayerInfoAddPlayer {
                                name: entry.profile.name.clone(),
                                texture_url,
                                cape_url,
                                model,
                            });
                        }
                        if actions.update_game_mode {
                            out.game_mode = Some(entry.game_mode.to_id());
                        }
                        if actions.update_listed {
                            out.listed = Some(entry.listed);
                        }
                        if actions.update_latency {
                            out.latency = Some(entry.latency);
                        }
                        if actions.update_display_name {
                            out.display_name = Some(match entry.display_name.as_ref() {
                                Some(name) => BackendPlayerInfoDisplayNameUpdate::Set {
                                    text: name.to_ansi(),
                                },
                                None => BackendPlayerInfoDisplayNameUpdate::Clear,
                            });
                        }
                        if actions.update_list_order {
                            out.list_order = Some(entry.list_order);
                        }
                        if actions.update_hat {
                            out.show_hat = Some(entry.update_hat);
                        }
                        out
                    })
                    .collect();
                let _ = event_tx.send(NetworkEvent::PlayerInfoUpdate {
                    entries: update_entries,
                });
                if p.actions.add_player {
                    for entry in p.entries {
                        if !entry.profile.name.is_empty() {
                            online_players.insert(entry.profile.uuid, entry.profile.name.clone());
                            let (texture_url, model, cape_url) = skin_from_profile_properties(
                                entry.profile.uuid,
                                &entry.profile.properties,
                            );
                            let _ = event_tx.send(NetworkEvent::AddPlayerInfo {
                                uuid: entry.profile.uuid,
                                name: entry.profile.name,
                                texture_url,
                                cape_url,
                                model,
                            });
                        }
                    }
                    let mut names = online_players.values().cloned().collect::<Vec<_>>();
                    names.sort_unstable();
                    names.dedup();
                    let _ = event_tx.send(NetworkEvent::OnlinePlayers { names });
                }
            }
            ClientboundGamePacket::PlayerInfoRemove(p) => {
                let mut changed = false;
                for profile_id in p.profile_ids {
                    if online_players.remove(&profile_id).is_some() {
                        changed = true;
                        let _ = event_tx.send(NetworkEvent::RemovePlayerInfo { uuid: profile_id });
                    }
                }
                if changed {
                    let mut names = online_players.values().cloned().collect::<Vec<_>>();
                    names.sort_unstable();
                    names.dedup();
                    let _ = event_tx.send(NetworkEvent::OnlinePlayers { names });
                }
            }
            ClientboundGamePacket::PlayerChat(mut p) => {
                let signed_content = p.body.content.clone();
                let decorated_before_filter = p.message_using_registries(&registries);
                let decorated_modified = !decorated_before_filter
                    .to_string()
                    .contains(&signed_content)
                    || p.unsigned_content
                        .as_ref()
                        .is_some_and(contains_non_default_chat_font);
                let signature = p
                    .signature
                    .as_ref()
                    .map(|signature| signature.bytes.to_vec());
                let last_seen = p
                    .body
                    .last_seen
                    .entries
                    .iter()
                    .filter_map(|entry| message_signature_cache.unpack(entry))
                    .collect::<Vec<_>>();
                message_signature_cache.push(last_seen, signature.clone());
                let was_shown = !matches!(
                    p.filter_mask,
                    game::c_player_chat::FilterMask::FullyFiltered
                );
                let ack_offset = if let Some(signature) = signature.clone() {
                    let ack_offset = {
                        let mut tracker = last_seen_messages
                            .lock()
                            .expect("last-seen chat tracker poisoned");
                        if tracker.add_pending(signature, was_shown) {
                            tracker.take_ack_offset_if_ready()
                        } else {
                            None
                        }
                    };
                    ack_offset
                } else {
                    None
                };
                if was_shown {
                    if let game::c_player_chat::FilterMask::PartiallyFiltered(mask) = &p.filter_mask
                    {
                        let filtered = p
                            .body
                            .content
                            .chars()
                            .enumerate()
                            .map(|(index, ch)| {
                                if mask.get(index).unwrap_or(false) {
                                    '#'
                                } else {
                                    ch
                                }
                            })
                            .collect::<String>();
                        p.unsigned_content = Some(azalea_chat::FormattedText::from(filtered));
                    }
                    let now_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let expired = now_ms.saturating_sub(p.body.timestamp) > 5 * 60 * 1000;
                    let trust = if signature.is_none() || expired {
                        ChatTrust::NotSecure
                    } else if decorated_modified {
                        ChatTrust::Modified
                    } else {
                        ChatTrust::Secure
                    };
                    let decorated = p.message_using_registries(&registries);
                    let (message, mut click_spans) = system_chat_text_and_click_spans(&decorated);
                    enrich_hover_item_tooltips(&mut click_spans, &enchantment_tooltips);
                    let _ = event_tx.send(NetworkEvent::PlayerChat {
                        message,
                        signed_content,
                        signature,
                        trust,
                        click_spans,
                    });
                }
                if let Some(offset) = ack_offset {
                    let _ = event_tx.send(NetworkEvent::ChatAck { offset });
                }
            }
            ClientboundGamePacket::DisguisedChat(p) => {
                let decorated = p.message_using_registries(&registries);
                let (message, mut click_spans) = system_chat_text_and_click_spans(&decorated);
                enrich_hover_item_tooltips(&mut click_spans, &enchantment_tooltips);
                let _ = event_tx.send(NetworkEvent::DisguisedChat {
                    message,
                    click_spans,
                });
            }
            ClientboundGamePacket::DeleteChat(p) => {
                if let Some(signature) = message_signature_cache.unpack(&p.signature) {
                    let _ = event_tx.send(NetworkEvent::DeleteChat { signature });
                }
            }
            ClientboundGamePacket::SetHealth(p) => {
                let _ = event_tx.send(NetworkEvent::Health {
                    health: p.health,
                    food: p.food,
                    saturation: p.saturation,
                });
            }
            ClientboundGamePacket::SetExperience(p) => {
                let _ = event_tx.send(NetworkEvent::Experience {
                    progress: p.experience_progress,
                    level: p.experience_level,
                    total: p.total_experience,
                });
            }
            ClientboundGamePacket::PlayerAbilities(p) => {
                let _ = event_tx.send(NetworkEvent::Abilities {
                    invulnerable: p.flags.invulnerable,
                    flying: p.flags.flying,
                    mayfly: p.flags.can_fly,
                    instabuild: p.flags.instant_break,
                    flying_speed: p.flying_speed,
                    walking_speed: p.walking_speed,
                });
            }
            ClientboundGamePacket::UpdateMobEffect(p) => {
                let effect_id = p.mob_effect as u32;
                let _ = event_tx.send(NetworkEvent::UpdateMobEffect {
                    entity_id: p.entity_id.0,
                    effect_id,
                    amplifier: p.data.amplifier,
                    duration: p.data.duration,
                    ambient: p.data.flags.ambient,
                    show_particles: p.data.flags.show_particles,
                    show_icon: p.data.flags.show_icon,
                    blend: p.data.flags.blend,
                });
            }
            ClientboundGamePacket::RemoveMobEffect(p) => {
                let effect_id = p.effect as u32;
                let _ = event_tx.send(NetworkEvent::RemoveMobEffect {
                    entity_id: p.entity_id.0,
                    effect_id,
                });
            }
            ClientboundGamePacket::UpdateAttributes(p) => {
                use azalea_registry::builtin::Attribute;
                let attributes = p
                    .values
                    .iter()
                    .map(|snap| {
                        let attribute_id = match snap.attribute {
                            Attribute::MovementSpeed => "minecraft:movement_speed",
                            Attribute::Gravity => "minecraft:gravity",
                            Attribute::JumpStrength => "minecraft:jump_strength",
                            Attribute::StepHeight => "minecraft:step_height",
                            Attribute::BlockInteractionRange => {
                                "minecraft:block_interaction_range"
                            }
                            Attribute::EntityInteractionRange => {
                                "minecraft:entity_interaction_range"
                            }
                            Attribute::SneakingSpeed => "minecraft:sneaking_speed",
                            Attribute::KnockbackResistance => {
                                "minecraft:knockback_resistance"
                            }
                            Attribute::FlyingSpeed => "minecraft:flying_speed",
                            Attribute::WaterMovementEfficiency => {
                                "minecraft:water_movement_efficiency"
                            }
                            Attribute::MovementEfficiency => {
                                "minecraft:movement_efficiency"
                            }
                            Attribute::CameraDistance => "minecraft:camera_distance",
                            Attribute::Armor => "minecraft:armor",
                            Attribute::ArmorToughness => "minecraft:armor_toughness",
                            Attribute::AttackDamage => "minecraft:attack_damage",
                            Attribute::AttackKnockback => "minecraft:attack_knockback",
                            Attribute::AttackSpeed => "minecraft:attack_speed",
                            Attribute::BlockBreakSpeed => "minecraft:block_break_speed",
                            Attribute::BurningTime => "minecraft:burning_time",
                            Attribute::ExplosionKnockbackResistance => {
                                "minecraft:explosion_knockback_resistance"
                            }
                            Attribute::FallDamageMultiplier => {
                                "minecraft:fall_damage_multiplier"
                            }
                            Attribute::FollowRange => "minecraft:follow_range",
                            Attribute::Luck => "minecraft:luck",
                            Attribute::MaxAbsorption => "minecraft:max_absorption",
                            Attribute::MaxHealth => "minecraft:max_health",
                            Attribute::MiningEfficiency => "minecraft:mining_efficiency",
                            Attribute::OxygenBonus => "minecraft:oxygen_bonus",
                            Attribute::SafeFallDistance => "minecraft:safe_fall_distance",
                            Attribute::Scale => "minecraft:scale",
                            Attribute::SpawnReinforcements => {
                                "minecraft:spawn_reinforcements"
                            }
                            Attribute::SubmergedMiningSpeed => {
                                "minecraft:submerged_mining_speed"
                            }
                            Attribute::SweepingDamageRatio => {
                                "minecraft:sweeping_damage_ratio"
                            }
                            Attribute::TemptRange => "minecraft:tempt_range",
                            Attribute::WaypointTransmitRange => {
                                "minecraft:waypoint_transmit_range"
                            }
                            Attribute::WaypointReceiveRange => {
                                "minecraft:waypoint_receive_range"
                            }
                        }
                            .to_string();
                        let modifiers = snap
                            .modifiers
                            .iter()
                            .map(|m| {
                                let op = match m.operation {
                                    azalea_core::attribute_modifier_operation::AttributeModifierOperation::AddValue => {
                                        0u8
                                    }
                                    azalea_core::attribute_modifier_operation::AttributeModifierOperation::AddMultipliedBase => {
                                        1u8
                                    }
                                    azalea_core::attribute_modifier_operation::AttributeModifierOperation::AddMultipliedTotal => {
                                        2u8
                                    }
                                };
                                (m.id.to_string(), m.amount, op)
                            })
                            .collect();
                        AttributeSnapshotEvent {
                            attribute_id,
                            base: snap.base,
                            modifiers,
                        }
                    })
                    .collect();
                let _ = event_tx.send(NetworkEvent::UpdateAttributes {
                    entity_id: p.entity_id.0,
                    attributes,
                });
            }
            ClientboundGamePacket::SetHeldSlot(p) => {
                let _ = event_tx.send(NetworkEvent::HeldSlot { slot: p.slot });
            }
            ClientboundGamePacket::SetTime(p) => {
                let game_time = i64::try_from(p.game_time).unwrap_or(i64::MAX);
                let day_time = Some(i64::try_from(p.day_time).unwrap_or(i64::MAX));
                let day_time_rate = Some(if p.tick_day_time { 1.0 } else { 0.0 });
                let day_time_partial_tick = Some(0.0);
                let _ = event_tx.send(NetworkEvent::Time {
                    game_time,
                    day_time,
                    day_time_rate,
                    day_time_partial_tick,
                });
            }
            ClientboundGamePacket::GameEvent(p) => {
                let event_id = game_event_id(p.event);
                let _ = event_tx.send(NetworkEvent::GameEvent {
                    event_id,
                    value: p.param,
                });
                if matches!(p.event, c_game_event::EventType::ChangeGameMode) {
                    let _ = event_tx.send(NetworkEvent::GameModeChanged {
                        game_mode: p.param as u8,
                    });
                }
            }
            ClientboundGamePacket::StartConfiguration(_) => {
                eprintln!(
                    "[ReconfigDebug] PLAY_RECV_START_CONFIGURATION; sending acknowledged packet and switching to configuration"
                );
                writer_task.abort();
                let _ = writer_task.await;
                let writer = {
                    let mut guard = writer.lock().await;
                    guard.take().ok_or_else(|| {
                        NetworkError::PlayDisconnect(
                            "play packet writer unavailable during reconfiguration".to_string(),
                        )
                    })?
                };
                let mut conn = Connection { reader, writer };
                conn.write(ServerboundConfigurationAcknowledged).await?;
                eprintln!("[ReconfigDebug] PLAY_SEND_CONFIGURATION_ACKNOWLEDGED");
                let config_conn = conn.config();
                let (config_conn, registries, enchantment_tooltips, server_cookies) =
                    config_sequence(config_conn, &args, &event_tx, server_cookies).await?;
                eprintln!(
                    "[ReconfigDebug] CONFIG_FINISHED_AFTER_PLAY_START; switching back to play"
                );
                let conn = config_conn.game();
                return Box::pin(game_loop_with_rx(
                    conn,
                    event_tx,
                    packet_rx,
                    registries,
                    enchantment_tooltips,
                    player_uuid,
                    args,
                    server_cookies,
                ))
                .await;
            }
            _ => {}
        }
    }
}
#[derive(Default)]
struct GamePacketReceiveDiagnostics {
    received_packet_ids: HashSet<u32>,
    unsupported_packet_ids: HashSet<u32>,
    ignored_decode_error_packet_ids: HashSet<u32>,
    ignored_decode_error_total: u64,
    ignored_decode_error_counts: HashMap<u32, u64>,
}
#[derive(Default)]
struct CustomPayloadTrace {
    emitted: usize,
}
impl CustomPayloadTrace {
    const LIMIT: usize = 32;
    fn record(&mut self, direction: &str, phase: &str, channel: &str, data_bytes: usize) {
        if self.emitted >= Self::LIMIT {
            return;
        }
        self.emitted += 1;
        diagnostics::write_line(&format!(
            "[PayloadTrace][backend] direction={direction} phase={phase} channel={channel} data_bytes={data_bytes}"
        ));
    }
}
impl GamePacketReceiveDiagnostics {
    fn received_once(&mut self, packet_id: u32, bytes: usize) {
        if self.received_packet_ids.insert(packet_id) {
            diagnostics::write_line(&format!(
                "[ProtocolDebug][backend] direction=clientbound phase=play disposition=received packet_id={packet_id} name={} supported={} bytes={bytes}",
                game_packet_name(packet_id),
                is_supported_game_packet(packet_id),
            ));
        }
    }
    fn unsupported_packet_once(&mut self, packet_id: u32, bytes: usize) {
        if self.unsupported_packet_ids.insert(packet_id) {
            diagnostics::write_line(&format!(
                "[ProtocolDebug][backend] direction=clientbound phase=play disposition=dropped_unsupported packet_id={packet_id} name={} bytes={bytes}",
                game_packet_name(packet_id),
            ));
        }
    }
    fn ignored_decode_error_once(
        &mut self,
        packet_id: u32,
        bytes: usize,
        error: &impl std::fmt::Display,
    ) {
        let count = self
            .ignored_decode_error_counts
            .entry(packet_id)
            .and_modify(|count| *count += 1)
            .or_insert(1);
        self.ignored_decode_error_total += 1;
        let total = self.ignored_decode_error_total;
        if self.ignored_decode_error_packet_ids.insert(packet_id) {
            diagnostics::write_line(&format!(
                "[ProtocolDebug][backend] direction=clientbound phase=play disposition=dropped_decode_error packet_id={packet_id} name={} bytes={bytes} error={error}",
                game_packet_name(packet_id),
            ));
            log::warn!(
                "[AzaleaProtocol] Ignoring failed decode for play packet {packet_id} / {} (occurrence {count}, ignored total {total}): {error}",
                game_packet_name(packet_id),
            );
        } else if *count % 100 == 0 {
            log::warn!(
                "[AzaleaProtocol] Still ignoring failed decodes for play packet {packet_id} / {} (occurrences {count}, ignored total {total}): {error}",
                game_packet_name(packet_id),
            );
        }
    }
}
#[derive(Default)]
struct ConfigPacketReceiveDiagnostics {
    received_packet_ids: HashSet<u32>,
}
impl ConfigPacketReceiveDiagnostics {
    fn received_once(&mut self, packet_id: u32, name: &str) {
        if self.received_packet_ids.insert(packet_id) {
            diagnostics::write_line(&format!(
                "[ProtocolDebug][backend] direction=clientbound phase=configuration disposition=received packet_id={packet_id} name={name}",
            ));
        }
    }
}
enum IncomingPlayPacket {
    Azalea(ClientboundGamePacket),
    Cooldown(Box<NetworkEvent>),
    LightUpdate(LightUpdateEvent),
    ManualParticles(Box<BackendParticleSpawn>),
    MapItemData(Box<NetworkEvent>),
}
fn generated_clientbound_game_packet_id_from_java_26_2(java_id: u32) -> Option<u32> {
    (java_id <= 138).then_some(java_id)
}
fn is_supported_game_packet(java_id: u32) -> bool {
    generated_clientbound_game_packet_id_from_java_26_2(java_id)
        .is_some_and(is_supported_generated_game_packet)
}
fn game_packet_name(java_id: u32) -> &'static str {
    generated_clientbound_game_packet_id_from_java_26_2(java_id)
        .map(generated_game_packet_name)
        .unwrap_or("unknown")
}
fn should_trace_game_packet_after_menu_click(java_id: u32) -> bool {
    generated_clientbound_game_packet_id_from_java_26_2(java_id)
        .map(should_trace_generated_game_packet_after_menu_click)
        .unwrap_or(true)
}
fn read_map_item_data_java_layout(
    cursor: &mut Cursor<&[u8]>,
) -> Result<NetworkEvent, azalea_buf::BufReadError> {
    use el_protocol_backend::{BackendMapDecoration, BackendMapPatch};
    let map_id = u32::azalea_read_var(cursor)?;
    let scale = u8::azalea_read(cursor)?;
    let locked = bool::azalea_read(cursor)?;
    let decorations = if bool::azalea_read(cursor)? {
        let count = u32::azalea_read_var(cursor)?;
        let mut decorations = Vec::with_capacity(count.min(1024) as usize);
        for _ in 0..count {
            let kind = azalea_registry::builtin::MapDecorationKind::azalea_read(cursor)?;
            let x = i8::azalea_read(cursor)?;
            let y = i8::azalea_read(cursor)?;
            let rot = u8::azalea_read(cursor)?;
            let name = Option::<FormattedText>::azalea_read(cursor)?.map(|name| name.to_string());
            decorations.push(BackendMapDecoration::from_vanilla_type(
                kind.to_str(),
                x,
                y,
                rot,
                name,
            ));
        }
        Some(decorations)
    } else {
        None
    };
    let width = u8::azalea_read(cursor)?;
    let color_patch = if width == 0 {
        None
    } else {
        Some(BackendMapPatch {
            width,
            height: u8::azalea_read(cursor)?,
            start_x: u8::azalea_read(cursor)?,
            start_y: u8::azalea_read(cursor)?,
            colors: Vec::<u8>::azalea_read(cursor)?,
        })
    };
    Ok(NetworkEvent::MapItemData {
        map_id,
        scale,
        locked,
        tracking_position: decorations.is_some(),
        color_patch,
        decorations,
    })
}
fn map_item_data_packet_id() -> u32 {
    static WIRE_ID: OnceLock<u32> = OnceLock::new();
    *WIRE_ID.get_or_init(|| {
        let packet = ClientboundGamePacket::MapItemData(game::ClientboundMapItemData {
            map_id: 0,
            scale: 0,
            locked: false,
            decorations: None,
            color_patch: game::c_map_item_data::OptionalMapPatch(None),
        });
        let encoded =
            azalea_protocol::write::serialize_packet(&packet).expect("map packet serialization");
        u32::azalea_read_var(&mut Cursor::new(&encoded[..])).expect("map packet id varint")
    })
}
fn read_supported_game_packet(
    raw_packet: &[u8],
    receive_diagnostics: &mut GamePacketReceiveDiagnostics,
) -> Result<Option<IncomingPlayPacket>, NetworkError> {
    let mut cursor = Cursor::new(raw_packet);
    let packet_id = u32::azalea_read_var(&mut cursor)
        .map_err(|source| NetworkError::Read(Box::new(ReadPacketError::ReadPacketId { source })))?;
    receive_diagnostics.received_once(packet_id, raw_packet.len());
    if packet_id == 22 {
        let event = (|| {
            Ok::<_, azalea_buf::BufReadError>(NetworkEvent::Cooldown {
                cooldown_group: Identifier::azalea_read(&mut cursor)?.to_string(),
                duration_ticks: i32::azalea_read_var(&mut cursor)?,
            })
        })()
        .map_err(|source| {
            NetworkError::Read(Box::new(ReadPacketError::Parse {
                packet_id,
                packet_name: "cooldown".to_string(),
                backtrace: Box::new(std::backtrace::Backtrace::capture()),
                source,
            }))
        })?;
        return Ok(Some(IncomingPlayPacket::Cooldown(Box::new(event))));
    }
    if packet_id == map_item_data_packet_id() {
        let event = read_map_item_data_java_layout(&mut cursor).map_err(|source| {
            NetworkError::Read(Box::new(ReadPacketError::Parse {
                packet_id,
                packet_name: "map_item_data".to_owned(),
                backtrace: Box::new(std::backtrace::Backtrace::capture()),
                source,
            }))
        })?;
        return Ok(Some(IncomingPlayPacket::MapItemData(Box::new(event))));
    }
    if !is_supported_game_packet(packet_id) {
        receive_diagnostics.unsupported_packet_once(packet_id, raw_packet.len());
        return Ok(None);
    }
    if packet_id == 47 {
        let update = read_light_update_packet_java_layout(&mut cursor).map_err(|source| {
            NetworkError::Read(Box::new(ReadPacketError::Parse {
                packet_id,
                packet_name: game_packet_name(packet_id).to_string(),
                backtrace: Box::new(std::backtrace::Backtrace::capture()),
                source,
            }))
        })?;
        return Ok(Some(IncomingPlayPacket::LightUpdate(update)));
    }
    if packet_id == 46 {
        let body_pos = cursor.position();
        if let Some(spawn) = try_read_java_option_particle_spawn(&mut cursor) {
            return Ok(Some(IncomingPlayPacket::ManualParticles(Box::new(spawn))));
        }
        cursor.set_position(body_pos);
    }
    let generated_id = generated_clientbound_game_packet_id_from_java_26_2(packet_id)
        .expect("the support gate admits only generated or explicitly handled Java packet ids");
    if packet_id == 104 {
        let method_packet =
            read_set_objective_packet_java_layout(&mut cursor).map_err(|source| {
                NetworkError::Read(Box::new(ReadPacketError::Parse {
                    packet_id,
                    packet_name: game_packet_name(packet_id).to_string(),
                    backtrace: Box::new(std::backtrace::Backtrace::capture()),
                    source,
                }))
            })?;
        return Ok(Some(IncomingPlayPacket::Azalea(method_packet)));
    }
    match ClientboundGamePacket::read(generated_id, &mut cursor) {
        Ok(packet) => Ok(Some(IncomingPlayPacket::Azalea(packet))),
        Err(err) if can_ignore_generated_game_packet_decode_error(generated_id) => {
            receive_diagnostics.ignored_decode_error_once(packet_id, raw_packet.len(), &err);
            if VERBOSE_PROTOCOL_LOGS {
                eprintln!(
                    "[AzaleaProtocol] Ignoring failed decode for optional play packet {} / {}: {}",
                    packet_id,
                    game_packet_name(packet_id),
                    err
                );
            }
            Ok(None)
        }
        Err(err) => Err(NetworkError::Read(err)),
    }
}
fn read_light_update_packet_java_layout(
    cursor: &mut Cursor<&[u8]>,
) -> Result<LightUpdateEvent, BufReadError> {
    let x = i32::azalea_read_var(cursor)?;
    let z = i32::azalea_read_var(cursor)?;
    let sky_y_mask = read_java_bit_set(cursor)?;
    let block_y_mask = read_java_bit_set(cursor)?;
    let empty_sky_y_mask = read_java_bit_set(cursor)?;
    let empty_block_y_mask = read_java_bit_set(cursor)?;
    let sky_updates = read_java_light_arrays(cursor)?;
    let block_updates = read_java_light_arrays(cursor)?;
    Ok(LightUpdateEvent {
        x,
        z,
        sky_layers: decoded_light_layers(&sky_y_mask, &empty_sky_y_mask, &sky_updates),
        block_layers: decoded_light_layers(&block_y_mask, &empty_block_y_mask, &block_updates),
    })
}
fn read_java_bit_set(cursor: &mut Cursor<&[u8]>) -> Result<BitSet, BufReadError> {
    let count = i32::azalea_read_var(cursor)?;
    if count < 0 {
        return Err(BufReadError::Custom(format!(
            "negative light BitSet long count {count}"
        )));
    }
    let count = count as usize;
    let remaining = cursor
        .get_ref()
        .len()
        .saturating_sub(cursor.position() as usize);
    if count > remaining / std::mem::size_of::<u64>() {
        return Err(BufReadError::UnexpectedEof {
            attempted_read: count.saturating_mul(std::mem::size_of::<u64>()),
            actual_read: remaining,
            backtrace: std::backtrace::Backtrace::capture(),
        });
    }
    let mut words = Vec::with_capacity(count);
    for _ in 0..count {
        words.push(u64::azalea_read(cursor)?);
    }
    Ok(BitSet::from(words))
}
fn read_java_light_arrays(cursor: &mut Cursor<&[u8]>) -> Result<Vec<Box<[u8]>>, BufReadError> {
    let count = i32::azalea_read_var(cursor)?;
    if count < 0 {
        return Err(BufReadError::Custom(format!(
            "negative light-array list size {count}"
        )));
    }
    let count = count as usize;
    let remaining = cursor
        .get_ref()
        .len()
        .saturating_sub(cursor.position() as usize);
    if count > remaining {
        return Err(BufReadError::UnexpectedEof {
            attempted_read: count,
            actual_read: remaining,
            backtrace: std::backtrace::Backtrace::capture(),
        });
    }
    let mut arrays = Vec::with_capacity(count);
    for _ in 0..count {
        let length = i32::azalea_read_var(cursor)?;
        if !(0..=2048).contains(&length) {
            return Err(BufReadError::Custom(format!(
                "light array length {length} is outside Java's 0..=2048 limit"
            )));
        }
        let mut bytes = Vec::with_capacity(length as usize);
        for _ in 0..length {
            bytes.push(u8::azalea_read(cursor)?);
        }
        arrays.push(bytes.into_boxed_slice());
    }
    Ok(arrays)
}
fn read_set_objective_packet_java_layout(
    cursor: &mut Cursor<&[u8]>,
) -> Result<ClientboundGamePacket, azalea_buf::BufReadError> {
    let objective_name = String::azalea_read(cursor)?;
    let method_id = u8::azalea_read(cursor)?;
    let method = match method_id {
        0 | 2 => {
            let display_name = FormattedText::azalea_read(cursor)?;
            let render_type = ObjectiveCriteria::azalea_read(cursor)?;
            let number_format =
                Option::<NumberFormat>::azalea_read(cursor)?.unwrap_or(NumberFormat::Blank);
            if method_id == 0 {
                game::c_set_objective::Method::Add {
                    display_name,
                    render_type,
                    number_format,
                }
            } else {
                game::c_set_objective::Method::Change {
                    display_name,
                    render_type,
                    number_format,
                }
            }
        }
        1 => game::c_set_objective::Method::Remove,
        _ => {
            return Err(azalea_buf::BufReadError::UnexpectedEnumVariant {
                id: method_id as i32,
            });
        }
    };
    Ok(ClientboundGamePacket::SetObjective(
        game::ClientboundSetObjective {
            objective_name,
            method,
        },
    ))
}
fn try_read_java_option_particle_spawn(cursor: &mut Cursor<&[u8]>) -> Option<BackendParticleSpawn> {
    let override_limiter = bool::azalea_read(cursor).ok()?;
    let always_show = bool::azalea_read(cursor).ok()?;
    let x = f64::azalea_read(cursor).ok()?;
    let y = f64::azalea_read(cursor).ok()?;
    let z = f64::azalea_read(cursor).ok()?;
    let x_dist = f32::azalea_read(cursor).ok()?;
    let y_dist = f32::azalea_read(cursor).ok()?;
    let z_dist = f32::azalea_read(cursor).ok()?;
    let max_speed = f32::azalea_read(cursor).ok()?;
    let count = i32::azalea_read(cursor).ok()?;
    let kind_id = u32::azalea_read_var(cursor).ok()?;
    let kind = ParticleKind::try_from(kind_id).ok()?;
    let rgb = |packed: i32| {
        let packed = packed as u32;
        [
            ((packed >> 16) & 0xFF) as f32 / 255.0,
            ((packed >> 8) & 0xFF) as f32 / 255.0,
            (packed & 0xFF) as f32 / 255.0,
        ]
    };
    let particle = match kind {
        ParticleKind::DragonBreath => {
            let power = f32::azalea_read(cursor).ok()?;
            BackendParticle::DragonBreath { power }
        }
        ParticleKind::Effect => {
            let color = rgb(i32::azalea_read(cursor).ok()?);
            let power = f32::azalea_read(cursor).ok()?;
            BackendParticle::Effect { color, power }
        }
        ParticleKind::InstantEffect => {
            let color = rgb(i32::azalea_read(cursor).ok()?);
            let power = f32::azalea_read(cursor).ok()?;
            BackendParticle::InstantEffect { color, power }
        }
        ParticleKind::TintedLeaves => {
            let color = rgb(i32::azalea_read(cursor).ok()?);
            BackendParticle::TintedLeaves { color }
        }
        ParticleKind::Flash => {
            let color = rgb(i32::azalea_read(cursor).ok()?);
            BackendParticle::Flash { color }
        }
        ParticleKind::Trail => {
            let target = [
                f64::azalea_read(cursor).ok()?,
                f64::azalea_read(cursor).ok()?,
                f64::azalea_read(cursor).ok()?,
            ];
            let color = rgb(i32::azalea_read(cursor).ok()?);
            let duration = i32::azalea_read_var(cursor).ok()?;
            BackendParticle::Trail {
                target,
                color,
                duration,
            }
        }
        ParticleKind::DustPillar => {
            let state_id = u32::azalea_read_var(cursor).ok()?;
            let block_state_id =
                local_block_state_id_from_azalea(AzaleaBlockState::try_from(state_id).ok()?)?
                    as u32;
            BackendParticle::DustPillar { block_state_id }
        }
        ParticleKind::BlockCrumble => {
            let state_id = u32::azalea_read_var(cursor).ok()?;
            let block_state_id =
                local_block_state_id_from_azalea(AzaleaBlockState::try_from(state_id).ok()?)?
                    as u32;
            BackendParticle::BlockCrumble { block_state_id }
        }
        _ => return None,
    };
    Some(BackendParticleSpawn {
        override_limiter,
        always_show,
        pos: [x, y, z],
        x_dist,
        y_dist,
        z_dist,
        max_speed,
        count: count.max(0) as u32,
        particle,
    })
}
fn queue_play_bundle_packet(
    packet: IncomingPlayPacket,
    pending_bundle_packets: &mut Option<Vec<IncomingPlayPacket>>,
    packet_queue: &mut VecDeque<IncomingPlayPacket>,
) -> Result<(), NetworkError> {
    match packet {
        IncomingPlayPacket::Azalea(ClientboundGamePacket::BundleDelimiter(_)) => {
            if let Some(bundle_packets) = pending_bundle_packets.take() {
                packet_queue.extend(bundle_packets);
            } else {
                *pending_bundle_packets = Some(Vec::new());
            }
        }
        packet => {
            if let Some(bundle_packets) = pending_bundle_packets.as_mut() {
                if bundle_packets.len() >= PLAY_BUNDLE_SIZE_LIMIT {
                    return Err(NetworkError::PlayDisconnect(
                        "Too many packets in a bundle".to_string(),
                    ));
                }
                bundle_packets.push(packet);
            } else {
                packet_queue.push_back(packet);
            }
        }
    }
    Ok(())
}
fn entity_motion_event(packet: &game::ClientboundSetEntityMotion) -> NetworkEvent {
    let velocity = packet.delta.to_vec3();
    NetworkEvent::EntityVelocity {
        id: packet.id.0,
        x: velocity.x,
        y: velocity.y,
        z: velocity.z,
    }
}
fn raw_game_packet_id(raw_packet: &[u8]) -> Option<u32> {
    let mut cursor = Cursor::new(raw_packet);
    u32::azalea_read_var(&mut cursor).ok()
}
fn raw_keep_alive_game_id(raw_packet: &[u8]) -> Option<i64> {
    use std::io::Read as _;
    let mut cursor = Cursor::new(raw_packet);
    let packet_id = u32::azalea_read_var(&mut cursor).ok()?;
    if packet_id != clientbound_keep_alive_wire_id() {
        return None;
    }
    let mut long_bytes = [0u8; 8];
    cursor.read_exact(&mut long_bytes).ok()?;
    Some(i64::from_be_bytes(long_bytes))
}
fn clientbound_keep_alive_wire_id() -> u32 {
    static WIRE_ID: OnceLock<u32> = OnceLock::new();
    *WIRE_ID.get_or_init(|| {
        let dummy = ClientboundGamePacket::KeepAlive(game::ClientboundKeepAlive { id: 0 });
        let encoded = azalea_protocol::write::serialize_packet(&dummy)
            .expect("keep-alive packet serialization");
        let mut cursor = Cursor::new(&encoded[..]);
        u32::azalea_read_var(&mut cursor).expect("keep-alive packet id varint")
    })
}
fn should_trace_generated_game_packet_after_menu_click(packet_id: u32) -> bool {
    !matches!(
        packet_id,
        0 | 14
            | 1
            | 13
            | 43
            | 44
            | 46
            | 47
            | 51
            | 52
            | 54
            | 67
            | 68
            | 75
            | 77
            | 81
            | 97
            | 100
            | 123
    )
}
fn is_supported_generated_game_packet(packet_id: u32) -> bool {
    matches!(
        packet_id,
        0 | 14
            | 1
            | 2
            | 4
            | 6
            | 7
            | 8
            | 9
            | 10
            | 11
            | 12
            | 13
            | 15
            | 16
            | 17
            | 18
            | 19
            | 20
            | 21
            | 22
            | 23
            | 24
            | 25
            | 26
            | 27
            | 28
            | 29
            | 31
            | 32
            | 33
            | 34
            | 35
            | 36
            | 37
            | 38
            | 39
            | 40
            | 41
            | 42
            | 43
            | 44
            | 46
            | 47
            | 48
            | 51
            | 52
            | 53
            | 54
            | 55
            | 57
            | 59
            | 62
            | 63
            | 67
            | 68
            | 70
            | 71
            | 75
            | 76
            | 77
            | 78
            | 79
            | 80
            | 81
            | 82
            | 83
            | 85
            | 86
            | 87
            | 88
            | 89
            | 90
            | 91
            | 92
            | 93
            | 94
            | 95
            | 97
            | 99
            | 100
            | 101
            | 102
            | 103
            | 105
            | 106
            | 107
            | 108
            | 109
            | 110
            | 111
            | 112
            | 113
            | 45
            | 114
            | 115
            | 117
            | 116
            | 118
            | 119
            | 120
            | 121
            | 122
            | 123
            | 124
            | 125
            | 126
            | 127
            | 61
            | 72
            | 73
            | 74
            | 131
            | 132
            | 133
            | 69
            | 5
            | 98
            | 3
            | 49
            | 50
            | 56
            | 58
            | 60
            | 64
            | 65
            | 66
            | 96
            | 104
            | 128
            | 129
            | 130
            | 30
            | 84
            | 134
            | 135
            | 136
            | 137
            | 138
    )
}
fn team_parameters(
    params: &azalea_protocol::packets::game::c_set_player_team::Parameters,
) -> (String, String, String, String, i32, String, u8) {
    use azalea_chat::style::ChatFormatting;
    use azalea_protocol::packets::game::c_set_player_team::{CollisionRule, NameTagVisibility};
    let collision_rule = match params.collision_rule {
        CollisionRule::Always => "always",
        CollisionRule::Never => "never",
        CollisionRule::PushOtherTeams => "push_other_teams",
        CollisionRule::PushOwnTeam => "push_own_team",
    };
    let nametag_visibility = match params.nametag_visibility {
        NameTagVisibility::Always => "always",
        NameTagVisibility::Never => "never",
        NameTagVisibility::HideForOtherTeams => "hideForOtherTeams",
        NameTagVisibility::HideForOwnTeam => "hideForOwnTeam",
    };
    let team_color = match params.color {
        ChatFormatting::Black => 0,
        ChatFormatting::DarkBlue => 1,
        ChatFormatting::DarkGreen => 2,
        ChatFormatting::DarkAqua => 3,
        ChatFormatting::DarkRed => 4,
        ChatFormatting::DarkPurple => 5,
        ChatFormatting::Gold => 6,
        ChatFormatting::Gray => 7,
        ChatFormatting::DarkGray => 8,
        ChatFormatting::Blue => 9,
        ChatFormatting::Green => 10,
        ChatFormatting::Aqua => 11,
        ChatFormatting::Red => 12,
        ChatFormatting::LightPurple => 13,
        ChatFormatting::Yellow => 14,
        ChatFormatting::White => 15,
        _ => -1,
    };
    (
        collision_rule.to_string(),
        params.display_name.to_ansi(),
        team_component_to_ansi(&params.player_prefix, params.color),
        team_component_to_ansi(&params.player_suffix, params.color),
        team_color,
        nametag_visibility.to_string(),
        params.options,
    )
}
fn team_component_to_ansi(
    component: &FormattedText,
    team_color: azalea_chat::style::ChatFormatting,
) -> String {
    let parent_color: Option<azalea_chat::style::TextColor> = team_color.try_into().ok();
    let parent_style = azalea_chat::style::Style::empty().color(parent_color);
    component.to_ansi_with_custom_style(&parent_style)
}
fn map_number_format(format: &NumberFormat) -> BackendScoreNumberFormat {
    match format {
        NumberFormat::Blank => BackendScoreNumberFormat::Blank,
        NumberFormat::Styled { style } => BackendScoreNumberFormat::Styled {
            color: style
                .string("color")
                .and_then(|value| {
                    let value = value.to_str();
                    azalea_chat::style::TextColor::parse(value.as_ref())
                })
                .map(|color| color.value as i32),
            style: serde_json::to_value(style).ok().map(|mut value| {
                normalize_dialog_component_booleans(&mut value);
                value
            }),
        },
        NumberFormat::Fixed { value } => BackendScoreNumberFormat::Fixed {
            text: value.to_ansi(),
        },
    }
}
fn map_azalea_particle_to_backend(
    particle: azalea_entity::particle::Particle,
) -> Option<BackendParticle> {
    use azalea_entity::particle::Particle;
    let block_id = |block: azalea_block::BlockState| {
        local_block_state_id_from_azalea(block).map(|id| id as u32)
    };
    let color = |c: azalea_core::color::RgbColor| -> [f32; 3] {
        [
            f32::from(c.red()) / 255.0,
            f32::from(c.green()) / 255.0,
            f32::from(c.blue()) / 255.0,
        ]
    };
    let simple = |path: &str| BackendParticle::Simple {
        name: format!("minecraft:{path}"),
    };
    Some(match particle {
        Particle::Block(p) => BackendParticle::Block {
            block_state_id: block_id(p.block_state)?,
        },
        Particle::BlockMarker(p) => BackendParticle::BlockMarker {
            block_state_id: block_id(p.block_state)?,
        },
        Particle::FallingDust(p) => BackendParticle::FallingDust {
            block_state_id: block_id(p.block_state)?,
        },
        Particle::DustPillar => simple("dust_pillar"),
        Particle::BlockCrumble => simple("block_crumble"),
        Particle::Dust(p) => BackendParticle::Dust {
            color: color(p.color),
            scale: p.scale,
        },
        Particle::DustColorTransition(p) => BackendParticle::DustColorTransition {
            from: color(p.from),
            to: color(p.to),
            scale: p.scale,
        },
        Particle::EntityEffect(p) => BackendParticle::EntityEffect {
            color: color(p.color),
        },
        Particle::TintedLeaves => simple("tinted_leaves"),
        Particle::Effect => simple("effect"),
        Particle::InstantEffect => simple("instant_effect"),
        Particle::DragonBreath => simple("dragon_breath"),
        Particle::Flash => simple("flash"),
        Particle::Item(p) => {
            let (item_location, count) = match &p.item {
                azalea_inventory::ItemStack::Empty => ("minecraft:air".to_string(), 0),
                azalea_inventory::ItemStack::Present(data) => {
                    (data.kind.to_str().to_string(), data.count)
                }
            };
            BackendParticle::Item {
                item_location,
                count,
            }
        }
        Particle::SculkCharge(p) => BackendParticle::SculkCharge { roll: p.roll },
        Particle::Shriek(p) => BackendParticle::Shriek { delay: p.delay },
        Particle::Vibration(p) => {
            let (x, y, z, entity_id, y_offset) = match p.position {
                azalea_entity::particle::PositionSource::Block(pos) => (
                    pos.x as f64 + 0.5,
                    pos.y as f64 + 0.5,
                    pos.z as f64 + 0.5,
                    -1,
                    0.0,
                ),
                azalea_entity::particle::PositionSource::Entity { id, y_offset } => {
                    (0.0, 0.0, 0.0, id.0 as i32, y_offset)
                }
            };
            BackendParticle::Vibration {
                x,
                y,
                z,
                entity_id,
                y_offset,
                ticks: p.ticks,
            }
        }
        Particle::Trail => simple("trail"),
        Particle::AngryVillager => simple("angry_villager"),
        Particle::Bubble => simple("bubble"),
        Particle::Cloud => simple("cloud"),
        Particle::CopperFireFlame => simple("copper_fire_flame"),
        Particle::Crit => simple("crit"),
        Particle::DamageIndicator => simple("damage_indicator"),
        Particle::DrippingLava => simple("dripping_lava"),
        Particle::FallingLava => simple("falling_lava"),
        Particle::LandingLava => simple("landing_lava"),
        Particle::DrippingWater => simple("dripping_water"),
        Particle::FallingWater => simple("falling_water"),
        Particle::ElderGuardian => simple("elder_guardian"),
        Particle::EnchantedHit => simple("enchanted_hit"),
        Particle::Enchant => simple("enchant"),
        Particle::EndRod => simple("end_rod"),
        Particle::ExplosionEmitter => simple("explosion_emitter"),
        Particle::Explosion => simple("explosion"),
        Particle::Gust => simple("gust"),
        Particle::SmallGust => simple("small_gust"),
        Particle::GustEmitterLarge => simple("gust_emitter_large"),
        Particle::GustEmitterSmall => simple("gust_emitter_small"),
        Particle::SonicBoom => simple("sonic_boom"),
        Particle::Firework => simple("firework"),
        Particle::Fishing => simple("fishing"),
        Particle::Flame => simple("flame"),
        Particle::Infested => simple("infested"),
        Particle::CherryLeaves => simple("cherry_leaves"),
        Particle::PaleOakLeaves => simple("pale_oak_leaves"),
        Particle::SculkSoul => simple("sculk_soul"),
        Particle::SculkChargePop => simple("sculk_charge_pop"),
        Particle::SoulFireFlame => simple("soul_fire_flame"),
        Particle::Soul => simple("soul"),
        Particle::HappyVillager => simple("happy_villager"),
        Particle::Composter => simple("composter"),
        Particle::Heart => simple("heart"),
        Particle::ItemSlime => simple("item_slime"),
        Particle::ItemCobweb => simple("item_cobweb"),
        Particle::ItemSnowball => simple("item_snowball"),
        Particle::LargeSmoke => simple("large_smoke"),
        Particle::Lava => simple("lava"),
        Particle::Mycelium => simple("mycelium"),
        Particle::Note => simple("note"),
        Particle::Poof => simple("poof"),
        Particle::Portal => simple("portal"),
        Particle::Rain => simple("rain"),
        Particle::Smoke => simple("smoke"),
        Particle::WhiteSmoke => simple("white_smoke"),
        Particle::Sneeze => simple("sneeze"),
        Particle::Spit => simple("spit"),
        Particle::SquidInk => simple("squid_ink"),
        Particle::SweepAttack => simple("sweep_attack"),
        Particle::TotemOfUndying => simple("totem_of_undying"),
        Particle::Underwater => simple("underwater"),
        Particle::Splash => simple("splash"),
        Particle::Witch => simple("witch"),
        Particle::BubblePop => simple("bubble_pop"),
        Particle::CurrentDown => simple("current_down"),
        Particle::BubbleColumnUp => simple("bubble_column_up"),
        Particle::Nautilus => simple("nautilus"),
        Particle::Dolphin => simple("dolphin"),
        Particle::CampfireCosySmoke => simple("campfire_cosy_smoke"),
        Particle::CampfireSignalSmoke => simple("campfire_signal_smoke"),
        Particle::DrippingHoney => simple("dripping_honey"),
        Particle::FallingHoney => simple("falling_honey"),
        Particle::LandingHoney => simple("landing_honey"),
        Particle::FallingNectar => simple("falling_nectar"),
        Particle::FallingSporeBlossom => simple("falling_spore_blossom"),
        Particle::Ash => simple("ash"),
        Particle::CrimsonSpore => simple("crimson_spore"),
        Particle::WarpedSpore => simple("warped_spore"),
        Particle::SporeBlossomAir => simple("spore_blossom_air"),
        Particle::DrippingObsidianTear => simple("dripping_obsidian_tear"),
        Particle::FallingObsidianTear => simple("falling_obsidian_tear"),
        Particle::LandingObsidianTear => simple("landing_obsidian_tear"),
        Particle::ReversePortal => simple("reverse_portal"),
        Particle::WhiteAsh => simple("white_ash"),
        Particle::SmallFlame => simple("small_flame"),
        Particle::Snowflake => simple("snowflake"),
        Particle::DrippingDripstoneLava => simple("dripping_dripstone_lava"),
        Particle::FallingDripstoneLava => simple("falling_dripstone_lava"),
        Particle::DrippingDripstoneWater => simple("dripping_dripstone_water"),
        Particle::FallingDripstoneWater => simple("falling_dripstone_water"),
        Particle::GlowSquidInk => simple("glow_squid_ink"),
        Particle::Glow => simple("glow"),
        Particle::WaxOn => simple("wax_on"),
        Particle::WaxOff => simple("wax_off"),
        Particle::ElectricSpark => simple("electric_spark"),
        Particle::Scrape => simple("scrape"),
        Particle::EggCrack => simple("egg_crack"),
        Particle::DustPlume => simple("dust_plume"),
        Particle::TrialSpawnerDetection => simple("trial_spawner_detection"),
        Particle::TrialSpawnerDetectionOminous => simple("trial_spawner_detection_ominous"),
        Particle::VaultConnection => simple("vault_connection"),
        Particle::OminousSpawning => simple("ominous_spawning"),
        Particle::RaidOmen => simple("raid_omen"),
        Particle::TrialOmen => simple("trial_omen"),
        Particle::Firefly => simple("firefly"),
    })
}
fn map_objective_number_format(format: &NumberFormat) -> Option<BackendScoreNumberFormat> {
    match format {
        NumberFormat::Blank => None,
        _ => Some(map_number_format(format)),
    }
}
fn debug_subscription_update_name(update: &DebugSubscriptionUpdate) -> &'static str {
    match update {
        DebugSubscriptionUpdate::DedicatedServerTickTime(_) => "dedicated_server_tick_time",
        DebugSubscriptionUpdate::Bees(_) => "bees",
        DebugSubscriptionUpdate::Brains(_) => "brains",
        DebugSubscriptionUpdate::Breezes(_) => "breezes",
        DebugSubscriptionUpdate::GoalSelectors(_) => "goal_selectors",
        DebugSubscriptionUpdate::EntityPaths(_) => "entity_paths",
        DebugSubscriptionUpdate::EntityBlockIntersections(_) => "entity_block_intersections",
        DebugSubscriptionUpdate::BeeHives(_) => "bee_hives",
        DebugSubscriptionUpdate::Pois(_) => "pois",
        DebugSubscriptionUpdate::RedstoneWireOrientations(_) => "redstone_wire_orientations",
        DebugSubscriptionUpdate::VillageSections(_) => "village_sections",
        DebugSubscriptionUpdate::Raids(_) => "raids",
        DebugSubscriptionUpdate::Structures(_) => "structures",
        DebugSubscriptionUpdate::GameEventListeners(_) => "game_event_listeners",
        DebugSubscriptionUpdate::NeighborUpdates(_) => "neighbor_updates",
        DebugSubscriptionUpdate::GameEvents(_) => "game_events",
    }
}
fn debug_subscription_event_name(event: &DebugSubscriptionEvent) -> &'static str {
    match event {
        DebugSubscriptionEvent::DedicatedServerTickTime(_) => "dedicated_server_tick_time",
        DebugSubscriptionEvent::Bees(_) => "bees",
        DebugSubscriptionEvent::Brains(_) => "brains",
        DebugSubscriptionEvent::Breezes(_) => "breezes",
        DebugSubscriptionEvent::GoalSelectors(_) => "goal_selectors",
        DebugSubscriptionEvent::EntityPaths(_) => "entity_paths",
        DebugSubscriptionEvent::EntityBlockIntersections(_) => "entity_block_intersections",
        DebugSubscriptionEvent::BeeHives(_) => "bee_hives",
        DebugSubscriptionEvent::Pois(_) => "pois",
        DebugSubscriptionEvent::RedstoneWireOrientations(_) => "redstone_wire_orientations",
        DebugSubscriptionEvent::VillageSections(_) => "village_sections",
        DebugSubscriptionEvent::Raids(_) => "raids",
        DebugSubscriptionEvent::Structures(_) => "structures",
        DebugSubscriptionEvent::GameEventListeners(_) => "game_event_listeners",
        DebugSubscriptionEvent::NeighborUpdates(_) => "neighbor_updates",
        DebugSubscriptionEvent::GameEvents(_) => "game_events",
    }
}
fn map_debug_block_pos(pos: &AzBlockPos) -> serde_json::Value {
    serde_json::json!([pos.x, pos.y, pos.z])
}
fn map_dialog_holder(
    holder: &azalea_registry::Holder<
        azalea_registry::data::Dialog,
        azalea_protocol::simdnbt::owned::Nbt,
    >,
    registries: &RegistryHolder,
) -> (Option<String>, Option<serde_json::Value>) {
    match holder {
        azalea_registry::Holder::Reference(kind) => {
            let registry_id = Identifier::new("minecraft:dialog");
            let Some(key) =
                registries.protocol_id_to_identifier(registry_id.clone(), kind.to_u32())
            else {
                return (Some(format!("{:?}", kind)), None);
            };
            let mut data = registries
                .extra
                .get(&registry_id)
                .and_then(|registry| registry.map.get(key))
                .and_then(map_dialog_compound);
            if let Some(data) = data.as_mut() {
                let mut active = HashSet::from([key.to_string()]);
                resolve_dialog_list_references(data, registries, 0, &mut active);
            }
            (Some(key.to_string()), data)
        }
        azalea_registry::Holder::Direct(nbt) => {
            let mut data = map_dialog_nbt(nbt);
            if let Some(data) = data.as_mut() {
                resolve_dialog_list_references(data, registries, 0, &mut HashSet::new());
            }
            (None, data)
        }
    }
}
fn pause_screen_dialog_tag_ids(tags: &azalea_protocol::common::tags::TagMap) -> Vec<i32> {
    tags.0
        .get(&Identifier::new("minecraft:dialog"))
        .and_then(|tags| {
            tags.iter()
                .find(|tag| tag.name == Identifier::new("minecraft:pause_screen_additions"))
        })
        .map(|tag| tag.elements.iter().copied().take(128).collect())
        .unwrap_or_default()
}
fn emit_pause_screen_dialog(
    event_tx: &EventSender,
    registries: &RegistryHolder,
    additions: &[i32],
) {
    let registry_id = Identifier::new("minecraft:dialog");
    let key = if additions.len() == 1 {
        registries
            .protocol_id_to_identifier(registry_id.clone(), additions[0] as u32)
            .cloned()
    } else if additions.len() > 1 {
        Some(Identifier::new("minecraft:custom_options"))
    } else {
        Some(Identifier::new("minecraft:server_links"))
    };
    let Some(key) = key else {
        return;
    };
    let mut data = registries
        .extra
        .get(&registry_id)
        .and_then(|registry| registry.map.get(&key))
        .and_then(map_dialog_compound);
    if let Some(value) = data.as_mut() {
        if additions.len() > 1
            && let Some(object) = value.as_object_mut()
        {
            let keys = additions
                .iter()
                .filter_map(|id| {
                    registries.protocol_id_to_identifier(registry_id.clone(), *id as u32)
                })
                .map(ToString::to_string)
                .map(serde_json::Value::String)
                .collect::<Vec<_>>();
            object.insert("dialogs".into(), serde_json::Value::Array(keys));
        }
        resolve_dialog_list_references(value, registries, 0, &mut HashSet::from([key.to_string()]));
        if serde_json::to_vec(value).map_or(true, |bytes| bytes.len() > 262_144) {
            data = None;
        }
    }
    let _ = event_tx.send(NetworkEvent::PauseScreenDialog {
        dialog_kind: key.to_string(),
        dialog_nbt: data,
        requires_server_links: additions.is_empty(),
    });
}
fn normalize_dialog_component_booleans(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                normalize_dialog_component_booleans(value);
            }
        }
        serde_json::Value::Object(object) => {
            for key in [
                "bold",
                "italic",
                "underlined",
                "strikethrough",
                "obfuscated",
                "reset",
                "can_close_with_escape",
                "pause",
            ] {
                if let Some(value) = object.get_mut(key)
                    && let Some(number) = value.as_i64()
                {
                    *value = serde_json::Value::Bool(number != 0);
                }
            }
            for value in object.values_mut() {
                normalize_dialog_component_booleans(value);
            }
        }
        _ => {}
    }
}
fn map_dialog_compound(
    compound: &azalea_protocol::simdnbt::owned::NbtCompound,
) -> Option<serde_json::Value> {
    let mut value = serde_json::to_value(compound).ok()?;
    annotate_dialog_nbt_sidecars(compound, &mut value);
    normalize_dialog_component_booleans(&mut value);
    Some(value)
}
fn resolve_dialog_list_references(
    data: &mut serde_json::Value,
    registries: &RegistryHolder,
    depth: usize,
    active: &mut HashSet<String>,
) {
    const MAX_DIALOG_REFERENCE_DEPTH: usize = 8;
    if depth >= MAX_DIALOG_REFERENCE_DEPTH {
        return;
    }
    let Some(dialogs) = data
        .get_mut("dialogs")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    let registry_id = Identifier::new("minecraft:dialog");
    let mut resolved = Vec::with_capacity(dialogs.len().min(128));
    for entry in dialogs.iter().take(128) {
        let id = entry
            .as_str()
            .or_else(|| entry.get("id").and_then(serde_json::Value::as_str));
        let Some(id) = id else {
            let mut direct = entry.clone();
            resolve_dialog_list_references(&mut direct, registries, depth + 1, active);
            resolved.push(direct);
            continue;
        };
        if !active.insert(id.to_string()) {
            continue;
        }
        let child = registries
            .extra
            .get(&registry_id)
            .and_then(|registry| registry.map.get(&Identifier::new(id)))
            .and_then(map_dialog_compound);
        if let Some(mut child) = child {
            if let Some(object) = child.as_object_mut() {
                if !object.contains_key("external_title") {
                    if let Some(title) = object.get("title").cloned() {
                        object.insert("external_title".to_string(), title);
                    }
                }
            }
            resolve_dialog_list_references(&mut child, registries, depth + 1, active);
            resolved.push(child);
        }
        active.remove(id);
    }
    *dialogs = resolved;
}
fn map_dialog_nbt(nbt: &azalea_protocol::simdnbt::owned::Nbt) -> Option<serde_json::Value> {
    match nbt {
        azalea_protocol::simdnbt::owned::Nbt::Some(base) => {
            map_dialog_compound(&base.clone().as_compound())
        }
        azalea_protocol::simdnbt::owned::Nbt::None => None,
    }
}
fn annotate_dialog_nbt_sidecars(
    compound: &azalea_protocol::simdnbt::owned::NbtCompound,
    value: &mut serde_json::Value,
) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    for (key, tag) in compound.iter() {
        let key = key.to_str();
        let Some(child) = object.get_mut(key.as_ref()) else {
            continue;
        };
        if matches!(key.as_ref(), "additions" | "payload") {
            let mut wire = Vec::new();
            tag.write(&mut wire);
            let encoded = base64::engine::general_purpose::STANDARD.encode(wire);
            if let Some(child_object) = child.as_object_mut() {
                child_object.insert(
                    "__eltharn_nbt_wire".to_string(),
                    serde_json::Value::String(encoded),
                );
            } else {
                *child = serde_json::json!(
                    { "__eltharn_nbt_wire" : encoded, "value" : child.clone(), }
                );
            }
        }
        annotate_dialog_nbt_tag_sidecars(tag, child);
    }
}
fn annotate_dialog_nbt_tag_sidecars(
    tag: &azalea_protocol::simdnbt::owned::NbtTag,
    value: &mut serde_json::Value,
) {
    use azalea_protocol::simdnbt::owned::{NbtList, NbtTag};
    match tag {
        NbtTag::Compound(compound) => annotate_dialog_nbt_sidecars(compound, value),
        NbtTag::List(NbtList::Compound(compounds)) => {
            if let Some(values) = value.as_array_mut() {
                for (compound, value) in compounds.iter().zip(values) {
                    annotate_dialog_nbt_sidecars(compound, value);
                }
            }
        }
        NbtTag::List(NbtList::List(lists)) => {
            if let Some(values) = value.as_array_mut() {
                for (list, value) in lists.iter().zip(values) {
                    annotate_dialog_nbt_list_sidecars(list, value);
                }
            }
        }
        _ => {}
    }
}
fn annotate_dialog_nbt_list_sidecars(
    list: &azalea_protocol::simdnbt::owned::NbtList,
    value: &mut serde_json::Value,
) {
    use azalea_protocol::simdnbt::owned::NbtList;
    match list {
        NbtList::Compound(compounds) => {
            if let Some(values) = value.as_array_mut() {
                for (compound, value) in compounds.iter().zip(values) {
                    annotate_dialog_nbt_sidecars(compound, value);
                }
            }
        }
        NbtList::List(lists) => {
            if let Some(values) = value.as_array_mut() {
                for (list, value) in lists.iter().zip(values) {
                    annotate_dialog_nbt_list_sidecars(list, value);
                }
            }
        }
        _ => {}
    }
}
fn map_debug_path_node(node: &MinecraftPathNode) -> serde_json::Value {
    serde_json::json!(
        { "x" : node.x, "y" : node.y, "z" : node.z, "contents" : { "walked_distance" :
        node.contents.walked_distance, "cost_malus" : node.contents.cost_malus, "closed"
        : node.contents.closed, "kind" : format!("{:?}", node.contents.kind)
        .to_ascii_lowercase(), "f" : node.contents.f, }, }
    )
}
fn map_debug_path(path: &MinecraftPath) -> serde_json::Value {
    serde_json::json!(
        { "reached" : path.reached, "next_node_index" : path.next_node_index, "block_pos"
        : map_debug_block_pos(& path.block_pos), "nodes" : path.nodes.iter()
        .map(map_debug_path_node).collect::< Vec < _ >> (), "debug_data" : {
        "target_nodes" : path.debug_data.target_nodes.iter().map(map_debug_path_node)
        .collect::< Vec < _ >> (), "open_set" : path.debug_data.open_set.iter()
        .map(map_debug_path_node).collect::< Vec < _ >> (), "closed_set" : path
        .debug_data.closed_set.iter().map(map_debug_path_node).collect::< Vec < _ >> (),
        }, }
    )
}
fn map_debug_subscription_value(value: serde_json::Value) -> serde_json::Value {
    value
}
fn map_debug_subscription_update(
    update: DebugSubscriptionUpdate,
) -> BackendDebugSubscriptionUpdate {
    use DebugSubscriptionUpdate as U;
    let subscription = debug_subscription_update_name(&update).to_string();
    let value = match update {
        U::DedicatedServerTickTime(v) => v.map(|()| serde_json::Value::Null),
        U::Bees(v) => v.map(|info| {
            serde_json::json!(
                { "hive_pos" : info.hive_pos.as_ref().map(map_debug_block_pos),
                "flower_pos" : info.flower_pos.as_ref().map(map_debug_block_pos),
                "travel_ticks" : info.travel_ticks, "blacklisted_hives" : info
                .blacklisted_hives.iter().map(map_debug_block_pos).collect::< Vec < _
                >> (), }
            )
        }),
        U::Brains(v) => v.map(|info| {
            serde_json::json!(
                { "name" : info.name, "profession" : info.profession, "xp" : info.xp,
                "health" : info.health, "max_health" : info.max_health, "inventory" :
                info.inventory, "wants_golem" : info.wants_golem, "anger_level" :
                info.anger_level, "activities" : info.activities, "behaviors" : info
                .behaviors, "memories" : info.memories, "gossips" : info.gossips,
                "pois" : info.pois.iter().map(map_debug_block_pos).collect::< Vec < _
                >> (), "potential_pois" : info.potential_pois.iter()
                .map(map_debug_block_pos).collect::< Vec < _ >> (), }
            )
        }),
        U::Breezes(v) => v.map(|info| {
            serde_json::json!(
                { "attack_target" : info.attack_target, "jump_target" : info
                .jump_target.as_ref().map(map_debug_block_pos), }
            )
        }),
        U::GoalSelectors(v) => v.map(|info| {
            serde_json::json!(
                { "priority" : info.priority, "is_running" : info.is_running, "name"
                : info.name, }
            )
        }),
        U::EntityPaths(v) => v.map(|info| {
            serde_json::json!(
                { "path" : map_debug_path(& info.path), "max_node_distance" : info
                .max_node_distance, }
            )
        }),
        U::EntityBlockIntersections(v) => {
            v.map(|info| serde_json::json!(format!("{:?}", info).to_ascii_lowercase()))
        }
        U::BeeHives(v) => v.map(|info| {
            serde_json::json!(
                { "kind" : info.kind.to_str(), "occupant_count" : info
                .occupant_count, "honey_level" : info.honey_level, "sedated" : info
                .sedated, }
            )
        }),
        U::Pois(v) => v.map(|info| {
            serde_json::json!(
                { "pos" : map_debug_block_pos(& info.pos), "poi_kind" : info.poi_kind
                .to_str(), "free_ticket_count" : info.free_ticket_count, }
            )
        }),
        U::RedstoneWireOrientations(v) => v.map(|info| serde_json::json!({ "id" : info.id })),
        U::VillageSections(v) => v.map(|()| serde_json::Value::Null),
        U::Raids(v) => v.map(|positions| {
            serde_json::json!(
                positions
                    .iter()
                    .map(map_debug_block_pos)
                    .collect::<Vec<_>>()
            )
        }),
        U::Structures(v) => v.map(|infos| {
            serde_json::json!(
                infos
                    .iter()
                    .map(|info| {
                        serde_json::json!({ "bounding_box" : {
                    "min" : map_debug_block_pos(& info.bounding_box.min), "max" :
                    map_debug_block_pos(& info.bounding_box.max), }, "pieces" : info
                    .pieces.iter().map(| piece | { serde_json::json!({ "bounding_box" : {
                    "min" : map_debug_block_pos(& piece.bounding_box.min), "max" :
                    map_debug_block_pos(& piece.bounding_box.max), }, "is_start" : piece
                    .is_start, }) }).collect::< Vec < _ >> (), })
                    })
                    .collect::<Vec<_>>()
            )
        }),
        U::GameEventListeners(v) => {
            v.map(|info| serde_json::json!({ "listener_radius" : info.listener_radius }))
        }
        U::NeighborUpdates(v) => v.map(|pos| map_debug_block_pos(&pos)),
        U::GameEvents(v) => v.map(|info| {
            serde_json::json!(
                { "event" : info.event.to_str(), "pos" : [info.pos.x, info.pos.y,
                info.pos.z], }
            )
        }),
    };
    BackendDebugSubscriptionUpdate {
        subscription,
        value: value.map(map_debug_subscription_value),
    }
}
fn map_debug_subscription_event(event: DebugSubscriptionEvent) -> BackendDebugSubscriptionEvent {
    use DebugSubscriptionEvent as E;
    let subscription = debug_subscription_event_name(&event).to_string();
    let value = match event {
        E::DedicatedServerTickTime(()) => serde_json::Value::Null,
        E::Bees(info) => {
            serde_json::json!(
                { "hive_pos" : info.hive_pos.as_ref().map(map_debug_block_pos),
                "flower_pos" : info.flower_pos.as_ref().map(map_debug_block_pos),
                "travel_ticks" : info.travel_ticks, "blacklisted_hives" : info
                .blacklisted_hives.iter().map(map_debug_block_pos).collect::< Vec < _ >>
                (), }
            )
        }
        E::Brains(info) => {
            serde_json::json!(
                { "name" : info.name, "profession" : info.profession, "xp" : info.xp,
                "health" : info.health, "max_health" : info.max_health, "inventory" :
                info.inventory, "wants_golem" : info.wants_golem, "anger_level" : info
                .anger_level, "activities" : info.activities, "behaviors" : info
                .behaviors, "memories" : info.memories, "gossips" : info.gossips, "pois"
                : info.pois.iter().map(map_debug_block_pos).collect::< Vec < _ >> (),
                "potential_pois" : info.potential_pois.iter().map(map_debug_block_pos)
                .collect::< Vec < _ >> (), }
            )
        }
        E::Breezes(info) => {
            serde_json::json!(
                { "attack_target" : info.attack_target, "jump_target" : info.jump_target
                .as_ref().map(map_debug_block_pos), }
            )
        }
        E::GoalSelectors(info) => {
            serde_json::json!(
                { "priority" : info.priority, "is_running" : info.is_running, "name" :
                info.name, }
            )
        }
        E::EntityPaths(info) => {
            serde_json::json!(
                { "path" : map_debug_path(& info.path), "max_node_distance" : info
                .max_node_distance, }
            )
        }
        E::EntityBlockIntersections(info) => {
            serde_json::json!(format!("{:?}", info).to_ascii_lowercase())
        }
        E::BeeHives(info) => {
            serde_json::json!(
                { "kind" : info.kind.to_str(), "occupant_count" : info.occupant_count,
                "honey_level" : info.honey_level, "sedated" : info.sedated, }
            )
        }
        E::Pois(info) => {
            serde_json::json!(
                { "pos" : map_debug_block_pos(& info.pos), "poi_kind" : info.poi_kind
                .to_str(), "free_ticket_count" : info.free_ticket_count, }
            )
        }
        E::RedstoneWireOrientations(info) => serde_json::json!({ "id" : info.id }),
        E::VillageSections(()) => serde_json::Value::Null,
        E::Raids(positions) => {
            serde_json::json!(
                positions
                    .iter()
                    .map(map_debug_block_pos)
                    .collect::<Vec<_>>()
            )
        }
        E::Structures(infos) => {
            serde_json::json!(
                infos
                    .iter()
                    .map(|info| {
                        serde_json::json!({ "bounding_box" : { "min"
                : map_debug_block_pos(& info.bounding_box.min), "max" :
                map_debug_block_pos(& info.bounding_box.max), }, "pieces" : info.pieces
                .iter().map(| piece | { serde_json::json!({ "bounding_box" : { "min" :
                map_debug_block_pos(& piece.bounding_box.min), "max" :
                map_debug_block_pos(& piece.bounding_box.max), }, "is_start" : piece
                .is_start, }) }).collect::< Vec < _ >> (), })
                    })
                    .collect::<Vec<_>>()
            )
        }
        E::GameEventListeners(info) => {
            serde_json::json!({ "listener_radius" : info.listener_radius })
        }
        E::NeighborUpdates(pos) => map_debug_block_pos(&pos),
        E::GameEvents(info) => {
            serde_json::json!(
                { "event" : info.event.to_str(), "pos" : [info.pos.x, info.pos.y, info
                .pos.z], }
            )
        }
    };
    BackendDebugSubscriptionEvent {
        subscription,
        value,
    }
}
fn can_ignore_generated_game_packet_decode_error(packet_id: u32) -> bool {
    matches!(packet_id, 9 | 11 | 12 | 15 | 16 | 24 | 31 | 85 | 113 | 121)
}
fn generated_game_packet_name(packet_id: u32) -> &'static str {
    match packet_id {
        0 => "bundle_delimiter",
        1 => "add_entity",
        2 => "animate",
        3 => "award_stats",
        4 => "block_changed_ack",
        5 => "block_destruction",
        6 => "block_entity_data",
        7 => "block_event",
        8 => "block_update",
        9 => "boss_event",
        10 => "change_difficulty",
        11 => "chunk_batch_finished",
        12 => "chunk_batch_start",
        13 => "chunks_biomes",
        14 => "clear_titles",
        15 => "command_suggestions",
        16 => "commands",
        17 => "container_close",
        18 => "container_set_content",
        19 => "container_set_data",
        20 => "container_set_slot",
        21 => "cookie_request",
        22 => "cooldown",
        23 => "custom_chat_completions",
        24 => "custom_payload",
        25 => "damage_event",
        26 => "debug_block_value",
        27 => "debug_chunk_value",
        28 => "debug_entity_value",
        29 => "debug_event",
        30 => "debug_sample",
        31 => "delete_chat",
        32 => "disconnect",
        33 => "disguised_chat",
        34 => "entity_event",
        35 => "entity_position_sync",
        36 => "explode",
        37 => "forget_level_chunk",
        38 => "game_event",
        39 => "game_test_highlight_pos",
        40 => "mount_screen_open",
        41 => "hurt_animation",
        42 => "initialize_border",
        43 => "keep_alive",
        44 => "level_chunk_with_light",
        45 => "level_event",
        46 => "level_particles",
        47 => "light_update",
        48 => "login",
        49 => "map_item_data",
        50 => "merchant_offers",
        51 => "move_entity_pos",
        52 => "move_entity_pos_rot",
        53 => "move_minecart_along_track",
        54 => "move_entity_rot",
        55 => "move_vehicle",
        56 => "open_book",
        57 => "open_screen",
        58 => "open_sign_editor",
        59 => "ping",
        60 => "pong_response",
        61 => "place_ghost_recipe",
        62 => "player_abilities",
        63 => "player_chat",
        64 => "player_combat_end",
        65 => "player_combat_enter",
        66 => "player_combat_kill",
        67 => "player_info_remove",
        68 => "player_info_update",
        69 => "player_look_at",
        70 => "player_position",
        71 => "player_rotation",
        72 => "recipe_book_add",
        73 => "recipe_book_remove",
        74 => "recipe_book_settings",
        75 => "remove_entities",
        76 => "remove_mob_effect",
        77 => "reset_score",
        78 => "resource_pack_pop",
        79 => "resource_pack_push",
        80 => "respawn",
        81 => "rotate_head",
        82 => "section_blocks_update",
        83 => "select_advancements_tab",
        84 => "server_data",
        85 => "set_action_bar_text",
        86 => "set_border_center",
        87 => "set_border_lerp_size",
        88 => "set_border_size",
        89 => "set_border_warning_delay",
        90 => "set_border_warning_distance",
        91 => "set_camera",
        92 => "set_chunk_cache_center",
        93 => "set_chunk_cache_radius",
        94 => "set_cursor_item",
        95 => "set_default_spawn_position",
        96 => "set_display_objective",
        97 => "set_entity_data",
        98 => "set_entity_link",
        99 => "set_entity_motion",
        100 => "set_equipment",
        101 => "set_experience",
        102 => "set_health",
        103 => "set_held_slot",
        104 => "set_objective",
        105 => "set_passengers",
        106 => "set_player_inventory",
        107 => "set_player_team",
        108 => "set_score",
        109 => "set_simulation_distance",
        110 => "set_subtitle_text",
        111 => "set_time",
        112 => "set_title_text",
        113 => "set_titles_animation",
        114 => "sound_entity",
        115 => "sound",
        116 => "start_configuration",
        117 => "stop_sound",
        118 => "store_cookie",
        119 => "system_chat",
        120 => "tab_list",
        121 => "tag_query",
        122 => "take_item_entity",
        123 => "teleport_entity",
        124 => "test_instance_block_status",
        125 => "ticking_state",
        126 => "ticking_step",
        127 => "transfer",
        128 => "update_advancements",
        129 => "update_attributes",
        130 => "update_mob_effect",
        131 => "update_recipes",
        132 => "update_tags",
        133 => "projectile_power",
        134 => "custom_report_details",
        135 => "server_links",
        136 => "waypoint",
        137 => "clear_dialog",
        138 => "show_dialog",
        _ => "unknown",
    }
}
fn dimension_bounds(
    common: &azalea_protocol::packets::common::CommonPlayerSpawnInfo,
    registries: &RegistryHolder,
) -> Option<(i32, u32)> {
    let (_, dimension): (
        &azalea_registry::identifier::Identifier,
        &DimensionKindElement,
    ) = common.dimension_type(registries)?;
    Some((dimension.min_y, dimension.height))
}
fn extract_biome_render_data(registries: &RegistryHolder) -> Vec<BiomeRenderData> {
    let Some((_, registry)) = registries
        .extra
        .iter()
        .find(|(id, _)| id.namespace() == "minecraft" && id.path() == "worldgen/biome")
    else {
        return Vec::new();
    };
    registry
        .map
        .iter()
        .map(|(id, biome)| {
            let effects = biome.get("effects").and_then(|tag| tag.compound());
            BiomeRenderData {
                id: id.to_string(),
                has_precipitation: biome
                    .get("has_precipitation")
                    .and_then(|tag| tag.byte())
                    .map(|value| value != 0)
                    .unwrap_or(true),
                temperature: biome
                    .get("temperature")
                    .and_then(|tag| tag.float())
                    .unwrap_or(0.8),
                temperature_modifier: biome
                    .get("temperature_modifier")
                    .and_then(|tag| tag.string())
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string()),
                downfall: biome
                    .get("downfall")
                    .and_then(|tag| tag.float())
                    .unwrap_or(0.4),
                water_color: effects
                    .and_then(|effects| effects.get("water_color"))
                    .and_then(nbt_rgb_color)
                    .unwrap_or(0x3f76e4),
                foliage_color: effects
                    .and_then(|effects| effects.get("foliage_color"))
                    .and_then(nbt_rgb_color),
                dry_foliage_color: effects
                    .and_then(|effects| effects.get("dry_foliage_color"))
                    .and_then(nbt_rgb_color),
                grass_color: effects
                    .and_then(|effects| effects.get("grass_color"))
                    .and_then(nbt_rgb_color),
                grass_color_modifier: effects
                    .and_then(|effects| effects.get("grass_color_modifier"))
                    .and_then(|tag| tag.string())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "none".to_string()),
            }
        })
        .collect()
}
fn nbt_rgb_color(tag: &azalea_protocol::simdnbt::owned::NbtTag) -> Option<u32> {
    if let Some(value) = tag.int() {
        return Some(value as u32 & 0x00ff_ffff);
    }
    let value = tag.string()?.to_string();
    u32::from_str_radix(value.trim_start_matches('#'), 16).ok()
}
fn az_direction(direction: u8) -> AzDirection {
    match direction {
        0 => AzDirection::Down,
        1 => AzDirection::Up,
        2 => AzDirection::North,
        3 => AzDirection::South,
        4 => AzDirection::West,
        5 => AzDirection::East,
        _ => AzDirection::Up,
    }
}
fn az_hand(hand: u8) -> s_interact::InteractionHand {
    match hand {
        1 => s_interact::InteractionHand::OffHand,
        _ => s_interact::InteractionHand::MainHand,
    }
}
fn backend_entity_pose_from_azalea(pose: azalea_entity::Pose) -> BackendEntityPose {
    match pose {
        azalea_entity::Pose::Standing => BackendEntityPose::Standing,
        azalea_entity::Pose::FallFlying => BackendEntityPose::FallFlying,
        azalea_entity::Pose::Sleeping => BackendEntityPose::Sleeping,
        azalea_entity::Pose::Swimming => BackendEntityPose::Swimming,
        azalea_entity::Pose::SpinAttack => BackendEntityPose::SpinAttack,
        azalea_entity::Pose::Crouching => BackendEntityPose::Crouching,
        azalea_entity::Pose::LongJumping => BackendEntityPose::LongJumping,
        azalea_entity::Pose::Dying => BackendEntityPose::Dying,
        azalea_entity::Pose::Croaking => BackendEntityPose::Croaking,
        azalea_entity::Pose::UsingTongue => BackendEntityPose::UsingTongue,
        azalea_entity::Pose::Sitting => BackendEntityPose::Sitting,
        azalea_entity::Pose::Roaring => BackendEntityPose::Roaring,
        azalea_entity::Pose::Sniffing => BackendEntityPose::Sniffing,
        azalea_entity::Pose::Emerging => BackendEntityPose::Emerging,
        azalea_entity::Pose::Digging => BackendEntityPose::Digging,
        azalea_entity::Pose::Sliding => BackendEntityPose::Sliding,
        azalea_entity::Pose::Shooting => BackendEntityPose::Shooting,
        azalea_entity::Pose::Inhaling => BackendEntityPose::Inhaling,
    }
}
fn backend_rotations_from_azalea(rotations: &azalea_entity::Rotations) -> BackendRotations {
    BackendRotations {
        x: rotations.x,
        y: rotations.y,
        z: rotations.z,
    }
}
fn backend_sleeping_pos_update_from_azalea(
    sleeping_pos: Option<AzBlockPos>,
) -> BackendSleepingPosUpdate {
    match sleeping_pos {
        Some(pos) => BackendSleepingPosUpdate::Set(BackendBlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        }),
        None => BackendSleepingPosUpdate::Clear,
    }
}
fn backend_end_crystal_beam_target_update_from_azalea(
    beam_target: Option<AzBlockPos>,
) -> BackendEndCrystalBeamTargetUpdate {
    match beam_target {
        Some(pos) => BackendEndCrystalBeamTargetUpdate::Set(BackendBlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        }),
        None => BackendEndCrystalBeamTargetUpdate::Clear,
    }
}
#[derive(Debug, Default, PartialEq)]
struct DecodedEntitySpecialMetadata {
    primed_tnt_fuse: Option<i32>,
    experience_orb_value: Option<i32>,
    primed_tnt_block_state: Option<LocalBlockState>,
    end_crystal_beam_target: Option<BackendEndCrystalBeamTargetUpdate>,
    end_crystal_show_bottom: Option<bool>,
    abstract_arrow_flags: Option<u8>,
    abstract_arrow_pierce_level: Option<u8>,
    abstract_arrow_in_ground: Option<bool>,
    arrow_effect_color: Option<i32>,
    tamable_flags: Option<u8>,
    creaking_can_move: Option<bool>,
    creaking_is_active: Option<bool>,
    creaking_is_tearing_down: Option<bool>,
    block_display_block_state: Option<LocalBlockState>,
    item_display_item: Option<azalea_inventory::ItemStack>,
    item_display_context: Option<u8>,
}
fn entity_item_stack_metadata(
    items: &[azalea_entity::EntityDataItem],
) -> Option<&azalea_inventory::ItemStack> {
    items
        .iter()
        .find_map(|item| match (item.index, &item.value) {
            (8 | 9, azalea_entity::EntityDataValue::ItemStack(stack)) => Some(stack),
            _ => None,
        })
}
fn entity_no_gravity_metadata(items: &[azalea_entity::EntityDataItem]) -> Option<bool> {
    items
        .iter()
        .find_map(|item| match (item.index, &item.value) {
            (5, azalea_entity::EntityDataValue::Boolean(value)) => Some(*value),
            _ => None,
        })
}
fn mob_flags_metadata(items: &[azalea_entity::EntityDataItem]) -> Option<u8> {
    items
        .iter()
        .find_map(|item| match (item.index, &item.value) {
            (15, azalea_entity::EntityDataValue::Byte(flags)) => Some(*flags),
            _ => None,
        })
}
fn mob_boolean_metadata(items: &[azalea_entity::EntityDataItem]) -> Vec<(u8, bool)> {
    items
        .iter()
        .filter_map(|item| match (item.index, &item.value) {
            (16..=18, azalea_entity::EntityDataValue::Boolean(value)) => Some((item.index, *value)),
            _ => None,
        })
        .collect()
}
fn item_frame_direction_metadata(items: &[azalea_entity::EntityDataItem]) -> Option<u8> {
    items
        .iter()
        .find_map(|item| match (item.index, &item.value) {
            (8, azalea_entity::EntityDataValue::Direction(direction)) => Some(*direction as u8),
            _ => None,
        })
}
fn item_frame_rotation_metadata(items: &[azalea_entity::EntityDataItem]) -> Option<i32> {
    items
        .iter()
        .find_map(|item| match (item.index, &item.value) {
            (10, azalea_entity::EntityDataValue::Int(rotation)) => Some(*rotation),
            _ => None,
        })
}
fn decode_entity_special_metadata(
    items: &[azalea_entity::EntityDataItem],
) -> DecodedEntitySpecialMetadata {
    let experience_orb_value = items.iter().find_map(|item| {
        (item.index == 8).then(|| match &item.value {
            azalea_entity::EntityDataValue::Int(value) => Some(*value),
            _ => None,
        })?
    });
    let primed_tnt_fuse = items.iter().find_map(|item| {
        (item.index == 8).then(|| match &item.value {
            azalea_entity::EntityDataValue::Int(fuse) => Some(*fuse),
            _ => None,
        })?
    });
    let primed_tnt_block_state = items.iter().find_map(|item| {
        (item.index == 9).then(|| match &item.value {
            azalea_entity::EntityDataValue::BlockState(state) => {
                local_block_state_from_azalea(state.clone())
            }
            _ => None,
        })?
    });
    let end_crystal_beam_target = items.iter().find_map(|item| {
        (item.index == 8).then(|| match &item.value {
            azalea_entity::EntityDataValue::OptionalBlockPos(target) => {
                Some(backend_end_crystal_beam_target_update_from_azalea(*target))
            }
            _ => None,
        })?
    });
    let end_crystal_show_bottom = items.iter().find_map(|item| {
        (item.index == 9).then(|| match &item.value {
            azalea_entity::EntityDataValue::Boolean(show_bottom) => Some(*show_bottom),
            _ => None,
        })?
    });
    let abstract_arrow_flags = items.iter().find_map(|item| {
        (item.index == 8).then(|| match &item.value {
            azalea_entity::EntityDataValue::Byte(flags) => Some(*flags),
            _ => None,
        })?
    });
    let abstract_arrow_pierce_level = items.iter().find_map(|item| {
        (item.index == 9).then(|| match &item.value {
            azalea_entity::EntityDataValue::Byte(level) => Some(*level),
            _ => None,
        })?
    });
    let abstract_arrow_in_ground = items.iter().find_map(|item| {
        (item.index == 10).then(|| match &item.value {
            azalea_entity::EntityDataValue::Boolean(in_ground) => Some(*in_ground),
            _ => None,
        })?
    });
    let arrow_effect_color = items.iter().find_map(|item| {
        (item.index == 11).then(|| match &item.value {
            azalea_entity::EntityDataValue::Int(color) => Some(*color),
            _ => None,
        })?
    });
    let tamable_flags = items.iter().find_map(|item| {
        (item.index == 17).then(|| match &item.value {
            azalea_entity::EntityDataValue::Byte(flags) => Some(*flags),
            _ => None,
        })?
    });
    let creaking_can_move = items.iter().find_map(|item| {
        (item.index == 16).then(|| match &item.value {
            azalea_entity::EntityDataValue::Boolean(can_move) => Some(*can_move),
            _ => None,
        })?
    });
    let creaking_is_active = items.iter().find_map(|item| {
        (item.index == 17).then(|| match &item.value {
            azalea_entity::EntityDataValue::Boolean(is_active) => Some(*is_active),
            _ => None,
        })?
    });
    let creaking_is_tearing_down = items.iter().find_map(|item| {
        (item.index == 18).then(|| match &item.value {
            azalea_entity::EntityDataValue::Boolean(is_tearing_down) => Some(*is_tearing_down),
            _ => None,
        })?
    });
    let block_display_block_state = items.iter().find_map(|item| {
        (item.index == 23).then(|| match &item.value {
            azalea_entity::EntityDataValue::BlockState(state) => {
                local_block_state_from_azalea(state.clone())
            }
            _ => None,
        })?
    });
    let item_display_item = items.iter().find_map(|item| {
        (item.index == 23).then(|| match &item.value {
            azalea_entity::EntityDataValue::ItemStack(stack) => Some(stack.clone()),
            _ => None,
        })?
    });
    let item_display_context = items.iter().find_map(|item| {
        (item.index == 24).then(|| match &item.value {
            azalea_entity::EntityDataValue::Byte(context) => Some(*context),
            _ => None,
        })?
    });
    DecodedEntitySpecialMetadata {
        primed_tnt_fuse,
        experience_orb_value,
        primed_tnt_block_state,
        end_crystal_beam_target,
        end_crystal_show_bottom,
        abstract_arrow_flags,
        abstract_arrow_pierce_level,
        abstract_arrow_in_ground,
        arrow_effect_color,
        tamable_flags,
        creaking_can_move,
        creaking_is_active,
        creaking_is_tearing_down,
        block_display_block_state,
        item_display_item,
        item_display_context,
    }
}
fn packed_rotation_degrees(value: i8) -> f32 {
    value as f32 * 360.0 / 256.0
}
fn game_mode_id(mode: GameMode) -> u8 {
    mode.to_id()
}
fn decode_chunk_loaded_event(
    packet: ClientboundLevelChunkWithLight,
    min_y: i32,
    height: u32,
    registries: RegistryHolder,
) -> Result<NetworkEvent, NetworkError> {
    let mut chunk = decode_chunk_packet(
        packet.x,
        packet.z,
        min_y,
        height,
        packet.chunk_data.data.as_ref().as_ref(),
        &packet.chunk_data.heightmaps,
        &registries,
    )?;
    for block_entity in &packet.chunk_data.block_entities {
        let local_x = i32::from(block_entity.packed_xz >> 4);
        let local_z = i32::from(block_entity.packed_xz & 15);
        let world_y = block_entity.y as i16 as i32;
        let pos = BlockPos::new(
            (packet.x << 4) + local_x,
            world_y,
            (packet.z << 4) + local_z,
        );
        let mut nbt = Vec::new();
        block_entity.data.write(&mut nbt);
        chunk.block_entities.insert(
            pos,
            BlockEntityData {
                kind: block_entity.kind.to_str().to_string(),
                nbt,
            },
        );
    }
    log_decoded_chunk_chests(&chunk, packet.x, packet.z);
    dump_chunk_decode_probe(
        &chunk,
        packet.x,
        packet.z,
        min_y,
        height,
        packet.chunk_data.data.as_ref().as_ref(),
    );
    apply_light_packet_data(&mut chunk, &packet.light_data);
    let non_air_blocks = chunk.non_air_block_count();
    Ok(NetworkEvent::ChunkLoaded {
        x: packet.x,
        z: packet.z,
        non_air_blocks,
        chunk,
    })
}
fn log_decoded_chunk_chests(chunk: &Chunk, chunk_x: i32, chunk_z: i32) {
    if !chest_debug_enabled() {
        return;
    }
    let mut rows = chunk
        .block_entities
        .keys()
        .filter_map(|pos| {
            let state = chunk.get_block_state(pos)?;
            state.block.path.ends_with("chest").then_some((pos, state))
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|(pos, _)| (pos.x, pos.y, pos.z));
    for (pos, state) in rows {
        let line = format!(
            "[ChestDebug][Backend] function=decode_chunk_loaded_event chunk=[{chunk_x},{chunk_z}] \\
             pos=[{},{},{}] block={} state_id={} raw_facing={:?} raw_type={:?} has_block_entity=true",
            pos.x,
            pos.y,
            pos.z,
            state.block,
            state.id,
            state.properties.get("facing").map(String::as_str),
            state.properties.get("type").map(String::as_str),
        );
        eprintln!("{line}");
        if diagnostics::file_enabled() {
            diagnostics::write_line(&line);
        }
    }
}
fn chest_debug_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        chest_debug_requested(std::env::var(ELTHARN_CHEST_DEBUG_ENV).ok().as_deref())
    })
}
fn dump_chunk_decode_probe(
    chunk: &Chunk,
    chunk_x: i32,
    chunk_z: i32,
    min_y: i32,
    height: u32,
    raw_data: &[u8],
) {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    if !*ENABLED.get_or_init(|| {
        !std::env::var("ELTHARN_CHUNK_DUMP")
            .ok()
            .is_some_and(|value| matches!(value.trim(), "off" | "0" | "false" | "no"))
    }) {
        return;
    }
    static PROBE_BOX: OnceLock<[i32; 6]> = OnceLock::new();
    let [min_x, min_y_box, min_z, max_x, max_y_box, max_z] =
        *PROBE_BOX.get_or_init(
            || match std::env::var("ELTHARN_CHUNK_DUMP").ok().as_deref() {
                Some("1") | None => [8, 60, 216, 15, 72, 224],
                Some(value) => {
                    let parsed: Vec<i32> = value
                        .split(',')
                        .filter_map(|part| part.trim().parse::<i32>().ok())
                        .collect();
                    if parsed.len() == 6 {
                        [
                            parsed[0], parsed[1], parsed[2], parsed[3], parsed[4], parsed[5],
                        ]
                    } else {
                        [8, 60, 216, 15, 72, 224]
                    }
                }
            },
        );
    let chunk_min_x = chunk_x << 4;
    let chunk_max_x = chunk_min_x + 15;
    let chunk_min_z = chunk_z << 4;
    let chunk_max_z = chunk_min_z + 15;
    if chunk_max_x < min_x || chunk_min_x > max_x || chunk_max_z < min_z || chunk_min_z > max_z {
        return;
    }
    let dump_dir = std::path::Path::new("/tmp/eltharn_chunk_dumps");
    if std::fs::create_dir_all(dump_dir).is_ok() {
        let raw_path = dump_dir.join(format!("chunk_{chunk_x}_{chunk_z}.bin"));
        let _ = std::fs::write(&raw_path, raw_data);
        let meta = format!(
            "chunk=[{chunk_x},{chunk_z}] min_y={min_y} height={height} data_len={} bytes\n",
            raw_data.len()
        );
        let _ = std::fs::write(
            dump_dir.join(format!("chunk_{chunk_x}_{chunk_z}.meta")),
            meta,
        );
    }
    let sample_x0 = chunk_min_x.max(min_x);
    let sample_x1 = chunk_max_x.min(max_x);
    let sample_z0 = chunk_min_z.max(min_z);
    let sample_z1 = chunk_max_z.min(max_z);
    for world_y in min_y_box..=max_y_box.min(min_y + height as i32 - 1) {
        for world_z in sample_z0..=sample_z1 {
            for world_x in sample_x0..=sample_x1 {
                let pos = BlockPos::new(world_x, world_y, world_z);
                if let Some(state) = chunk.get_block_state(&pos)
                    && state.id != 0
                    && !state.block.path.ends_with("air")
                {
                    let line = format!(
                        "[ChunkDump][Backend] chunk=[{chunk_x},{chunk_z}] pos=[{world_x},{world_y},{world_z}] id={} name={}",
                        state.id, state.block
                    );
                    eprintln!("{line}");
                    if diagnostics::file_enabled() {
                        diagnostics::write_line(&line);
                    }
                }
            }
        }
    }
}
fn chest_debug_requested(value: Option<&str>) -> bool {
    matches!(value, Some("1"))
}
fn decode_chunk_packet(
    x: i32,
    z: i32,
    min_y: i32,
    height: u32,
    data: &[u8],
    heightmaps: &[(azalea_core::heightmap_kind::HeightmapKind, Box<[u64]>)],
    registries: &RegistryHolder,
) -> Result<Chunk, NetworkError> {
    let mut cursor = Cursor::new(data);
    let azalea_chunk =
        AzaleaChunk::read_with_dimension_height(&mut cursor, height, min_y, heightmaps)
            .map_err(|source| NetworkError::ChunkDecode { x, z, source })?;
    Ok(convert_chunk(
        x,
        z,
        min_y,
        height,
        azalea_chunk,
        heightmaps,
        registries,
    ))
}
fn convert_chunk(
    x: i32,
    z: i32,
    min_y: i32,
    height: u32,
    azalea_chunk: AzaleaChunk,
    heightmaps: &[(azalea_core::heightmap_kind::HeightmapKind, Box<[u64]>)],
    registries: &RegistryHolder,
) -> Chunk {
    let air = BuiltInRegistries::block_states()
        .get_state_by_id(0)
        .cloned()
        .expect("air block state must exist");
    let min_y_section = min_y >> 4;
    let section_count = (height / 16).max(1) as i32;
    let max_y_section = min_y_section + section_count - 1;
    let mut chunk = Chunk::new(x, z, min_y_section, max_y_section, air.clone());
    let mut state_cache: HashMap<u32, Option<el_core::block_state::BlockState>> = HashMap::new();
    for (section_index, section) in azalea_chunk.sections.iter().enumerate() {
        let section_y = min_y_section + section_index as i32;
        let used_fast_block_storage = chunk
            .sections
            .get_mut(section_index)
            .and_then(|local_section| {
                let blocks = convert_azalea_block_storage_fast(&section.states, &air)?;
                local_section.blocks = blocks;
                Some(())
            })
            .is_some();
        if !used_fast_block_storage {
            for packed in 0..4096 {
                let azalea_state = section.states.get_at_index(packed);
                let state_id = u32::from(azalea_state);
                if state_id == 0 {
                    continue;
                }
                let state = state_cache
                    .entry(state_id)
                    .or_insert_with(|| local_block_state_from_azalea(azalea_state))
                    .clone();
                let Some(state) = state else {
                    continue;
                };
                let local_x = (packed & 15) as i32;
                let local_z = ((packed >> 4) & 15) as i32;
                let local_y = ((packed >> 8) & 15) as i32;
                let world_y = (section_y << 4) + local_y;
                let pos = BlockPos::new((x << 4) + local_x, world_y, (z << 4) + local_z);
                let _ = chunk.set_block_state(&pos, state);
            }
        }
        if let Some(local_section) = chunk.sections.get_mut(section_index) {
            local_section.non_empty_block_count = section.block_count;
            let mut fluids = 0usize;
            local_section.blocks.count(|state, count| {
                if !state.get_fluid_state().is_empty() {
                    fluids = fluids.saturating_add(count);
                }
            });
            local_section.fluid_count = fluids.min(u16::MAX as usize) as u16;
            for packed in 0..64usize {
                let biome = section.biomes.get_at_index(packed);
                let protocol_id = u32::from(biome);
                let registry =
                    azalea_registry::identifier::Identifier::new("minecraft:worldgen/biome");
                let location = registries
                    .protocol_id_to_identifier(registry, protocol_id)
                    .map(|id| ResourceLocation::from_string(&id.to_string()))
                    .unwrap_or_else(|| {
                        log_biome_fallback_once(protocol_id, "level_chunk");
                        ResourceLocation::new("minecraft", "plains")
                    });
                let quart_x = (packed & 3) as i32;
                let quart_z = ((packed >> 2) & 3) as i32;
                let quart_y = ((packed >> 4) & 3) as i32;
                local_section.set_biome(quart_x, quart_y, quart_z, location);
            }
        }
    }
    for (kind, data) in heightmaps {
        let expected = expected_heightmap_longs(height);
        if data.len() != expected {
            log::warn!(
                "[AzaleaProtocol] Ignoring heightmap data for chunk [{x},{z}], size does not match; expected: {expected}, got: {}",
                data.len()
            );
            continue;
        }
        let kind = match kind {
            azalea_core::heightmap_kind::HeightmapKind::WorldSurfaceWg => {
                HeightmapType::WorldSurfaceWg
            }
            azalea_core::heightmap_kind::HeightmapKind::WorldSurface => HeightmapType::WorldSurface,
            azalea_core::heightmap_kind::HeightmapKind::OceanFloorWg => HeightmapType::OceanFloorWg,
            azalea_core::heightmap_kind::HeightmapKind::OceanFloor => HeightmapType::OceanFloor,
            azalea_core::heightmap_kind::HeightmapKind::MotionBlocking => {
                HeightmapType::MotionBlocking
            }
            azalea_core::heightmap_kind::HeightmapKind::MotionBlockingNoLeaves => {
                HeightmapType::MotionBlockingNoLeaves
            }
        };
        chunk.heightmaps.insert(kind, data.to_vec());
    }
    chunk
}
fn expected_heightmap_longs(height: u32) -> usize {
    let bits = if height == 0 {
        0
    } else {
        (u32::BITS - height.leading_zeros()) as usize
    };
    (256 * bits + 63) / 64
}
fn convert_azalea_block_storage_fast(
    states: &azalea_world::palette::PalettedContainer<AzaleaBlockState>,
    air: &LocalBlockState,
) -> Option<PalettedContainer<LocalBlockState>> {
    let palette = match &states.palette {
        AzaleaPalette::SingleValue(value) => {
            vec![local_block_state_from_azalea(*value).unwrap_or_else(|| air.clone())]
        }
        AzaleaPalette::Linear(values) | AzaleaPalette::Hashmap(values) => values
            .iter()
            .map(|value| local_block_state_from_azalea(*value).unwrap_or_else(|| air.clone()))
            .collect::<Vec<_>>(),
        AzaleaPalette::Global => return None,
    };
    Some(PalettedContainer::from_raw(
        palette,
        states.storage.data.to_vec(),
        states.bits_per_entry as usize,
        SECTION_VOLUME,
        air.clone(),
    ))
}
fn apply_light_packet_data(chunk: &mut Chunk, data: &ClientboundLightUpdatePacketData) {
    apply_light_layers(
        &mut chunk.sky_light,
        &data.sky_y_mask,
        &data.empty_sky_y_mask,
        &data.sky_updates,
    );
    apply_light_layers(
        &mut chunk.block_light,
        &data.block_y_mask,
        &data.empty_block_y_mask,
        &data.block_updates,
    );
    chunk.is_light_correct = true;
}
fn apply_light_layers(
    target: &mut [Option<DataLayer>],
    update_mask: &azalea_core::bitset::BitSet,
    empty_mask: &azalea_core::bitset::BitSet,
    updates: &[Box<[u8]>],
) {
    for index in empty_mask.iter_ones() {
        if let Some(slot) = target.get_mut(index) {
            *slot = Some(DataLayer::zero());
        }
    }
    for (index, bytes) in update_mask.iter_ones().zip(updates.iter()) {
        if let (Some(slot), Some(layer)) = (target.get_mut(index), DataLayer::from_bytes(bytes)) {
            *slot = Some(layer);
        }
    }
}
fn decoded_light_layers(
    update_mask: &azalea_core::bitset::BitSet,
    empty_mask: &azalea_core::bitset::BitSet,
    updates: &[Box<[u8]>],
) -> Vec<(usize, Option<DataLayer>)> {
    let mut result = empty_mask
        .iter_ones()
        .map(|index| (index, Some(DataLayer::zero())))
        .collect::<Vec<_>>();
    result.extend(
        update_mask
            .iter_ones()
            .zip(updates.iter())
            .map(|(index, bytes)| (index, DataLayer::from_bytes(bytes))),
    );
    result
}
fn local_block_state_id_from_azalea(state: azalea_block::BlockState) -> Option<usize> {
    local_block_state_from_azalea(state).map(|state| state.id)
}
fn canonical_falling_block_state_id_from_add_entity_data(
    kind: &str,
    spawn_data: i32,
) -> Option<usize> {
    let type_key = kind.strip_prefix("minecraft:").unwrap_or(kind);
    if type_key != "falling_block" {
        return None;
    }
    let wire_state = AzaleaBlockState::try_from(spawn_data).unwrap_or(AzaleaBlockState::AIR);
    local_block_state_id_from_azalea(wire_state)
}
fn azalea_item_stack_from_local(stack: &el_core::ItemStack) -> azalea_inventory::ItemStack {
    if stack.is_empty() {
        return azalea_inventory::ItemStack::Empty;
    }
    let location_str = stack.item.location.to_string();
    let id = azalea_registry::identifier::Identifier::new(location_str);
    if let Ok(kind) = azalea_registry::builtin::ItemKind::try_from(&id) {
        let mut az_stack = azalea_inventory::ItemStack::new(kind, stack.count);
        apply_local_components_to_azalea_stack(&mut az_stack, stack);
        az_stack
    } else {
        azalea_inventory::ItemStack::Empty
    }
}
fn apply_local_components_to_azalea_stack(
    az_stack: &mut azalea_inventory::ItemStack,
    local: &el_core::ItemStack,
) {
    use azalea_inventory::components::PotionContents;
    use azalea_registry::builtin::Potion;
    use std::str::FromStr;
    if let Some(el_core::item::DataComponent::Generic {
        value: Tag::Compound(fields),
        ..
    }) = local.components.get("minecraft:potion_contents")
    {
        let mut potion_contents = PotionContents::new();
        if let Some(Tag::String(potion_id)) = compound_get(fields, "potion") {
            let bare = potion_id
                .strip_prefix("minecraft:")
                .unwrap_or(potion_id.as_str());
            potion_contents.potion = Potion::from_str(bare).ok();
        }
        if let Some(tag) = compound_get(fields, "custom_color") {
            let color_val = match tag {
                Tag::Int(c) => *c,
                Tag::Byte(c) => *c as i32,
                _ => 0,
            };
            potion_contents.custom_color = Some(color_val);
        }
        if let Some(Tag::List(effects)) = compound_get(fields, "custom_effects") {
            potion_contents.custom_effects = build_mob_effect_instances(effects);
        }
        if let Some(Tag::String(name)) = compound_get(fields, "custom_name") {
            potion_contents.custom_name = Some(name.clone());
        }
        *az_stack = std::mem::take(az_stack).with_component(potion_contents);
    }
}
fn build_mob_effect_instances(
    effects: &[Tag],
) -> Vec<azalea_inventory::components::MobEffectInstance> {
    use azalea_inventory::components::{MobEffectDetails, MobEffectInstance};
    use azalea_registry::builtin::MobEffect;
    use std::str::FromStr;
    let mut out = Vec::with_capacity(effects.len());
    for entry in effects {
        let Tag::Compound(fields) = entry else {
            continue;
        };
        let Some(Tag::String(id_str)) = compound_get(fields, "id") else {
            continue;
        };
        let bare = id_str.strip_prefix("minecraft:").unwrap_or(id_str.as_str());
        let Ok(id) = MobEffect::from_str(bare) else {
            continue;
        };
        let mut details = MobEffectDetails::new();
        if let Some(tag) = compound_get(fields, "amplifier") {
            details.amplifier = match tag {
                Tag::Int(v) => *v,
                Tag::Byte(v) => *v as i32,
                Tag::Short(v) => *v as i32,
                _ => 0,
            };
        }
        if let Some(tag) = compound_get(fields, "duration") {
            details.duration = match tag {
                Tag::Int(v) => *v,
                Tag::Byte(v) => *v as i32,
                Tag::Short(v) => *v as i32,
                _ => 0,
            };
        }
        if let Some(el_nbt::Tag::Byte(v)) = compound_get(fields, "ambient") {
            details.ambient = *v != 0;
        }
        if let Some(el_nbt::Tag::Byte(v)) = compound_get(fields, "show_particles") {
            details.show_particles = *v != 0;
        }
        if let Some(el_nbt::Tag::Byte(v)) = compound_get(fields, "show_icon") {
            details.show_icon = *v != 0;
        }
        out.push(MobEffectInstance { id, details });
    }
    out
}
fn compound_get<'a>(fields: &'a [(String, Tag)], key: &str) -> Option<&'a Tag> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}
fn azalea_stack_matches_local_ignoring_count(
    raw: &azalea_inventory::ItemStack,
    local: &el_core::ItemStack,
    registries: &RegistryHolder,
) -> bool {
    if local.is_empty() {
        return raw.is_empty();
    }
    let decoded = local_item_stack_from_azalea_with_registries(raw, registries);
    let mut local_without_tooltip_metadata = local.clone();
    local_without_tooltip_metadata
        .components
        .remove("rustore:enchantment_tooltip_metadata");
    decoded.item == local_without_tooltip_metadata.item
        && decoded.components == local_without_tooltip_metadata.components
}
fn azalea_stack_same_item(raw: &azalea_inventory::ItemStack, local: &el_core::ItemStack) -> bool {
    if local.is_empty() {
        return raw.is_empty();
    }
    raw.as_present()
        .map(|raw| raw.kind.to_string() == local.item.location.to_string())
        .unwrap_or(false)
}
fn azalea_stack_with_local_count(
    raw: &azalea_inventory::ItemStack,
    local: &el_core::ItemStack,
) -> azalea_inventory::ItemStack {
    if local.is_empty() {
        return azalea_inventory::ItemStack::Empty;
    }
    let Some(raw) = raw.as_present() else {
        return azalea_item_stack_from_local(local);
    };
    let mut raw = raw.clone();
    raw.count = local.count;
    let mut stack = azalea_inventory::ItemStack::Present(raw);
    stack.update_empty();
    stack
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
enum SkinPropertyDecodeError {
    InvalidBase64,
    InvalidJson,
    MissingSkin,
    MissingUrl,
}
impl SkinPropertyDecodeError {
    const fn diagnostic_name(self) -> &'static str {
        match self {
            Self::InvalidBase64 => "invalid_base64",
            Self::InvalidJson => "invalid_json",
            Self::MissingSkin => "missing_skin",
            Self::MissingUrl => "missing_url",
        }
    }
}
fn decode_skin_texture_property(
    property_value: &str,
) -> Result<(Option<String>, Option<String>, Option<String>), SkinPropertyDecodeError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(property_value)
        .map_err(|_| SkinPropertyDecodeError::InvalidBase64)?;
    let payload = serde_json::from_slice::<serde_json::Value>(&bytes)
        .map_err(|_| SkinPropertyDecodeError::InvalidJson)?;
    let textures = payload
        .get("textures")
        .ok_or(SkinPropertyDecodeError::MissingSkin)?;
    let skin = textures.get("SKIN");
    let texture_url = skin
        .and_then(|skin| skin.get("url"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let model = skin
        .and_then(|skin| skin.get("metadata"))
        .and_then(|metadata| metadata.get("model"))
        .and_then(serde_json::Value::as_str)
        .and_then(|model| match model.trim().to_ascii_lowercase().as_str() {
            "slim" => Some("slim".to_owned()),
            "wide" | "default" => Some("wide".to_owned()),
            _ => None,
        });
    let cape_url = textures
        .get("CAPE")
        .and_then(|cape| cape.get("url"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    if texture_url.is_none() && cape_url.is_none() {
        return Err(SkinPropertyDecodeError::MissingSkin);
    }
    Ok((texture_url, model, cape_url))
}
fn skin_from_profile_properties(
    profile_id: uuid::Uuid,
    properties: &azalea_auth::game_profile::GameProfileProperties,
) -> (Option<String>, Option<String>, Option<String>) {
    let Some(property) = properties.map.get("textures") else {
        skin_debug(format!(
            "phase=player_info_texture profile_uuid={} result=default reason=no_textures_property property_count={}",
            profile_id,
            properties.map.len()
        ));
        return (None, None, None);
    };
    match decode_skin_texture_property(&property.value) {
        Ok((texture_url, model, cape_url)) => {
            skin_debug(format!(
                "phase=player_info_texture profile_uuid={} result=texture model={} {} cape={}",
                profile_id,
                model.as_deref().unwrap_or("wide_default"),
                skin_url_diagnostic(texture_url.as_deref().unwrap_or("")),
                if cape_url.is_some() { "yes" } else { "no" }
            ));
            (texture_url, model, cape_url)
        }
        Err(error) => {
            skin_debug(format!(
                "phase=player_info_texture profile_uuid={} result=default reason={} property_len={}",
                profile_id,
                error.diagnostic_name(),
                property.value.len()
            ));
            (None, None, None)
        }
    }
}
pub fn local_item_stack_from_azalea(stack: &azalea_inventory::ItemStack) -> el_core::ItemStack {
    local_item_stack_from_azalea_with_registries(stack, &RegistryHolder::default())
}
pub fn local_item_stack_from_azalea_with_registries(
    stack: &azalea_inventory::ItemStack,
    registries: &RegistryHolder,
) -> el_core::ItemStack {
    local_item_stack_from_azalea_with_enchantment_tooltips(
        stack,
        registries,
        &EnchantmentTooltipRegistry::default(),
    )
}
fn local_item_stack_from_azalea_with_enchantment_tooltips(
    stack: &azalea_inventory::ItemStack,
    registries: &RegistryHolder,
    enchantment_tooltips: &EnchantmentTooltipRegistry,
) -> el_core::ItemStack {
    let Some(stack) = stack.as_present() else {
        return el_core::ItemStack::empty();
    };
    let item_id = ResourceLocation::from_string(&stack.kind.to_string());
    let item = BuiltInRegistries::items().get_by_location(&item_id).clone();
    let mut local = el_core::ItemStack::new(item, stack.count);
    apply_missing_default_item_components(stack, &mut local);
    let Some(serde_json::Value::Object(components)) =
        item_component_patch_json(&stack.component_patch, registries, enchantment_tooltips)
    else {
        return local;
    };
    for (kind, value) in components {
        if let Some(removed_kind) = kind.strip_prefix('!') {
            local.components.remove(removed_kind);
            continue;
        }
        let key = if kind.contains(':') {
            kind
        } else {
            format!("minecraft:{kind}")
        };
        let formatted_text_json = if matches!(
            key.as_str(),
            "minecraft:custom_name" | "minecraft:item_name"
        ) {
            serde_json::to_string(&value).ok()
        } else {
            None
        };
        let Some(value) = json_value_to_eltharn_nbt(&value) else {
            continue;
        };
        if key == "minecraft:custom_name" {
            let raw = formatted_text_json.unwrap_or_default();
            local
                .components
                .set(&key, el_core::item::DataComponent::CustomName(raw));
        } else if key == "minecraft:item_name" {
            local.components.set(
                &key,
                el_core::item::DataComponent::Generic {
                    type_name: key.clone(),
                    value: Tag::String(formatted_text_json.unwrap_or_default()),
                },
            );
        } else {
            local.components.set(
                &key,
                el_core::item::DataComponent::Generic {
                    type_name: key.clone(),
                    value,
                },
            );
        }
    }
    local
}
fn apply_missing_default_item_components(
    stack: &azalea_inventory::ItemStackData,
    local: &mut el_core::ItemStack,
) {
    use azalea_inventory::components::{AttributeModifiers, Consumable, Equippable, Food};
    use azalea_inventory::default_components::get_default_component;
    for (name, value) in [
        (
            "minecraft:attribute_modifiers",
            get_default_component::<AttributeModifiers>(stack.kind)
                .and_then(|value| serde_json::to_value(value).ok()),
        ),
        (
            "minecraft:equippable",
            get_default_component::<Equippable>(stack.kind)
                .and_then(|value| serde_json::to_value(value).ok()),
        ),
    ] {
        if local.components.get(name).is_none()
            && let Some(tag) = value.as_ref().and_then(json_value_to_eltharn_nbt)
        {
            local.components.set(
                name,
                el_core::item::DataComponent::Generic {
                    type_name: name.to_string(),
                    value: tag,
                },
            );
        }
    }
    let Some(present) = Some(stack) else {
        return;
    };
    if local.components.get("minecraft:consumable").is_none()
        && let Some(default_consumable) = get_default_component::<Consumable>(present.kind)
    {
        local.components.set(
            "minecraft:consumable",
            el_core::item::DataComponent::Generic {
                type_name: "minecraft:consumable".to_string(),
                value: consumable_component_to_tag(&default_consumable),
            },
        );
    }
    if local.components.get("minecraft:food").is_none()
        && let Some(default_food) = get_default_component::<Food>(present.kind)
    {
        local.components.set(
            "minecraft:food",
            el_core::item::DataComponent::Generic {
                type_name: "minecraft:food".to_string(),
                value: Tag::Compound(vec![
                    ("nutrition".to_string(), Tag::Int(default_food.nutrition)),
                    (
                        "saturation".to_string(),
                        Tag::Float(default_food.saturation),
                    ),
                    (
                        "can_always_eat".to_string(),
                        Tag::Byte(i8::from(default_food.can_always_eat)),
                    ),
                ]),
            },
        );
    }
}
fn consumable_component_to_tag(consumable: &azalea_inventory::components::Consumable) -> Tag {
    Tag::Compound(vec![
        (
            "consume_seconds".to_string(),
            Tag::Float(consumable.consume_seconds),
        ),
        (
            "animation".to_string(),
            Tag::String(consumable_animation_name(consumable.animation).to_string()),
        ),
    ])
}
fn consumable_animation_name(
    animation: azalea_inventory::components::ItemUseAnimation,
) -> &'static str {
    use azalea_inventory::components::ItemUseAnimation as A;
    match animation {
        A::None => "none",
        A::Eat => "eat",
        A::Drink => "drink",
        A::BlockKind => "block",
        A::Bow => "bow",
        A::Spear => "spear",
        A::Crossbow => "crossbow",
        A::Spyglass => "spyglass",
        A::TootHorn => "toot_horn",
        A::Brush => "brush",
    }
}
fn item_component_patch_json(
    patch: &azalea_inventory::DataComponentPatch,
    registries: &RegistryHolder,
    enchantment_tooltips: &EnchantmentTooltipRegistry,
) -> Option<serde_json::Value> {
    use azalea_inventory::components::{Enchantments, StoredEnchantments};
    use azalea_registry::builtin::DataComponentKind;
    let has_enchantments = patch.get::<Enchantments>().is_some();
    let has_stored_enchantments = patch.get::<StoredEnchantments>().is_some();
    if let Ok(mut value) = serde_json::to_value(patch) {
        if let serde_json::Value::Object(components) = &mut value
            && (has_enchantments || has_stored_enchantments)
        {
            let mut enchantment_ids = HashSet::new();
            if let Some(component) = patch.get::<Enchantments>() {
                enchantment_ids.extend(resolved_enchantment_ids(&component.levels, registries));
            }
            if let Some(component) = patch.get::<StoredEnchantments>() {
                enchantment_ids.extend(resolved_enchantment_ids(
                    &component.enchantments,
                    registries,
                ));
            }
            if let Some(metadata) = enchantment_tooltips.item_component_json(&enchantment_ids) {
                components.insert("rustore:enchantment_tooltip_metadata".to_string(), metadata);
            }
        }
        if let serde_json::Value::Object(components) = &mut value {
            resolve_item_trim_component(components, patch, registries);
        }
        return Some(value);
    }
    if !has_enchantments && !has_stored_enchantments {
        return None;
    }
    let mut sanitized = patch.clone();
    unsafe {
        if has_enchantments {
            sanitized.unchecked_insert_component(DataComponentKind::Enchantments, None);
        }
        if has_stored_enchantments {
            sanitized.unchecked_insert_component(DataComponentKind::StoredEnchantments, None);
        }
    }
    let mut value = match serde_json::to_value(&sanitized) {
        Ok(serde_json::Value::Object(value)) => value,
        _ => return None,
    };
    let mut enchantment_ids = HashSet::new();
    if has_enchantments {
        value.remove("!minecraft:enchantments");
        value.remove("!enchantments");
        let component = patch.get::<Enchantments>()?;
        enchantment_ids.extend(resolved_enchantment_ids(&component.levels, registries));
        value.insert(
            "minecraft:enchantments".to_string(),
            enchantment_component_json(&component.levels, registries),
        );
    }
    if has_stored_enchantments {
        value.remove("!minecraft:stored_enchantments");
        value.remove("!stored_enchantments");
        let component = patch.get::<StoredEnchantments>()?;
        enchantment_ids.extend(resolved_enchantment_ids(
            &component.enchantments,
            registries,
        ));
        value.insert(
            "minecraft:stored_enchantments".to_string(),
            enchantment_component_json(&component.enchantments, registries),
        );
    }
    if let Some(metadata) = enchantment_tooltips.item_component_json(&enchantment_ids) {
        value.insert("rustore:enchantment_tooltip_metadata".to_string(), metadata);
    }
    resolve_item_trim_component(&mut value, patch, registries);
    Some(serde_json::Value::Object(value))
}
fn resolve_item_trim_component(
    components: &mut serde_json::Map<String, serde_json::Value>,
    patch: &azalea_inventory::DataComponentPatch,
    registries: &RegistryHolder,
) {
    use azalea_registry::DataRegistry;
    let Some(trim) = patch.get::<azalea_inventory::components::Trim>() else {
        return;
    };
    let resolve = |registry: &str, id: u32| {
        let values = registries.extra.get(&Identifier::new(registry))?;
        let (name, compound) = values.map.get_index(id as usize)?;
        let mut value = map_dialog_compound(compound)?;
        if let serde_json::Value::Object(fields) = &mut value {
            fields.insert(
                "id".to_string(),
                serde_json::Value::String(name.to_string()),
            );
        }
        Some(value)
    };
    if let (Some(material), Some(pattern)) = (
        resolve("minecraft:trim_material", trim.material.protocol_id()),
        resolve("minecraft:trim_pattern", trim.pattern.protocol_id()),
    ) {
        components.insert(
            "minecraft:trim".to_string(),
            serde_json::json!({ "material" : material, "pattern" : pattern }),
        );
    }
}
fn enchantment_component_json<K>(
    levels: &HashMap<K, i32>,
    registries: &RegistryHolder,
) -> serde_json::Value
where
    K: azalea_registry::DataRegistry,
{
    let mut map = serde_json::Map::new();
    for (enchantment, level) in levels {
        let id = resolved_enchantment_id(enchantment, registries);
        map.insert(id, serde_json::Value::Number((*level).into()));
    }
    serde_json::json!({ "levels" : map })
}
fn resolved_enchantment_ids<K>(
    levels: &HashMap<K, i32>,
    registries: &RegistryHolder,
) -> HashSet<String>
where
    K: azalea_registry::DataRegistry,
{
    levels
        .keys()
        .map(|enchantment| resolved_enchantment_id(enchantment, registries))
        .collect()
}
fn resolved_enchantment_id<K>(enchantment: &K, registries: &RegistryHolder) -> String
where
    K: azalea_registry::DataRegistry,
{
    registries
        .protocol_id_to_identifier(
            Identifier::new("minecraft:enchantment"),
            enchantment.protocol_id(),
        )
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("minecraft:enchantment#{}", enchantment.protocol_id()))
}
fn json_value_to_eltharn_nbt(value: &serde_json::Value) -> Option<Tag> {
    match value {
        serde_json::Value::Null => Some(Tag::End),
        serde_json::Value::Bool(value) => Some(Tag::Byte(if *value { 1 } else { 0 })),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                if let Ok(value) = i32::try_from(value) {
                    Some(Tag::Int(value))
                } else {
                    Some(Tag::Long(value))
                }
            } else if let Some(value) = value.as_u64() {
                if let Ok(value) = i32::try_from(value) {
                    Some(Tag::Int(value))
                } else {
                    Some(Tag::Long(i64::try_from(value).ok()?))
                }
            } else {
                Some(Tag::Double(value.as_f64()?))
            }
        }
        serde_json::Value::String(value) => Some(Tag::String(value.clone())),
        serde_json::Value::Array(values) => Some(Tag::List(
            values
                .iter()
                .filter_map(json_value_to_eltharn_nbt)
                .collect(),
        )),
        serde_json::Value::Object(values) => Some(Tag::Compound(
            values
                .iter()
                .filter_map(|(key, value)| Some((key.clone(), json_value_to_eltharn_nbt(value)?)))
                .collect(),
        )),
    }
}
const VANILLA_1_21_11_CHEST_STATE_BASES: [(&str, u32); 10] = [
    ("chest", 3786),
    ("trapped_chest", 11005),
    ("copper_chest", 26893),
    ("exposed_copper_chest", 26917),
    ("weathered_copper_chest", 26941),
    ("oxidized_copper_chest", 26965),
    ("waxed_copper_chest", 26989),
    ("waxed_exposed_copper_chest", 27013),
    ("waxed_weathered_copper_chest", 27037),
    ("waxed_oxidized_copper_chest", 27061),
];
fn vanilla_1_21_11_chest_properties_from_protocol_id(
    protocol_id: u32,
) -> Option<(ResourceLocation, HashMap<String, String>)> {
    let (path, offset) =
        VANILLA_1_21_11_CHEST_STATE_BASES
            .iter()
            .find_map(|(path, first_state_id)| {
                (protocol_id >= *first_state_id && protocol_id < *first_state_id + 24)
                    .then(|| (*path, protocol_id - *first_state_id))
            })?;
    const FACINGS: [&str; 4] = ["north", "south", "west", "east"];
    const TYPES: [&str; 3] = ["single", "left", "right"];
    const WATERLOGGED: [&str; 2] = ["true", "false"];
    let mut properties = HashMap::with_capacity(3);
    properties.insert(
        "facing".to_string(),
        FACINGS[(offset / 6) as usize].to_string(),
    );
    properties.insert(
        "type".to_string(),
        TYPES[((offset / 2) % 3) as usize].to_string(),
    );
    properties.insert(
        "waterlogged".to_string(),
        WATERLOGGED[(offset % 2) as usize].to_string(),
    );
    Some((ResourceLocation::new("minecraft", path), properties))
}
const VANILLA_1_21_11_PISTON_FAMILY_STATE_BASES: [(&str, u32); 4] = [
    ("sticky_piston", 2035),
    ("piston", 2057),
    ("piston_head", 2069),
    ("moving_piston", 2109),
];
fn vanilla_1_21_11_piston_family_properties_from_protocol_id(
    protocol_id: u32,
) -> Option<(ResourceLocation, HashMap<String, String>)> {
    let (path, offset) =
        VANILLA_1_21_11_PISTON_FAMILY_STATE_BASES
            .iter()
            .find_map(|(path, first_state_id)| {
                let states = if *path == "piston_head" { 24 } else { 12 };
                (protocol_id >= *first_state_id && protocol_id < first_state_id + states)
                    .then(|| (*path, protocol_id - *first_state_id))
            })?;
    const FACINGS: [&str; 6] = ["north", "east", "south", "west", "up", "down"];
    const EXTENDED: [&str; 2] = ["true", "false"];
    const SHORTS: [&str; 2] = ["true", "false"];
    const TYPES: [&str; 2] = ["normal", "sticky"];
    let mut properties = HashMap::with_capacity(3);
    match path {
        "piston" | "sticky_piston" => {
            properties.insert(
                "extended".to_string(),
                EXTENDED[(offset / 6) as usize].to_string(),
            );
            properties.insert(
                "facing".to_string(),
                FACINGS[(offset % 6) as usize].to_string(),
            );
        }
        "moving_piston" => {
            properties.insert(
                "facing".to_string(),
                FACINGS[(offset / 2) as usize].to_string(),
            );
            properties.insert("type".to_string(), TYPES[(offset % 2) as usize].to_string());
        }
        "piston_head" => {
            properties.insert(
                "facing".to_string(),
                FACINGS[(offset / 4) as usize].to_string(),
            );
            properties.insert(
                "short".to_string(),
                SHORTS[((offset / 2) % 2) as usize].to_string(),
            );
            properties.insert("type".to_string(), TYPES[(offset % 2) as usize].to_string());
        }
        _ => return None,
    }
    Some((ResourceLocation::new("minecraft", path), properties))
}
fn local_block_state_from_azalea(
    state: azalea_block::BlockState,
) -> Option<el_core::block_state::BlockState> {
    if let Some((block_name, properties)) =
        vanilla_1_21_11_chest_properties_from_protocol_id(u32::from(state))
    {
        let state_id =
            BuiltInRegistries::block_states().get_id_by_state(&block_name, &properties)?;
        return BuiltInRegistries::block_states()
            .get_state_by_id(state_id)
            .cloned();
    }
    if let Some((block_name, properties)) =
        vanilla_1_21_11_piston_family_properties_from_protocol_id(u32::from(state))
    {
        let state_id =
            BuiltInRegistries::block_states().get_id_by_state(&block_name, &properties)?;
        return BuiltInRegistries::block_states()
            .get_state_by_id(state_id)
            .cloned();
    }
    let block = Box::<dyn BlockTrait>::from(state);
    let block_name = ResourceLocation::new("minecraft", block.id());
    let properties: HashMap<String, String> = block
        .property_map()
        .into_iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    if let Some(id) = BuiltInRegistries::block_states().get_id_by_state(&block_name, &properties) {
        return BuiltInRegistries::block_states()
            .get_state_by_id(id)
            .cloned();
    }
    log_block_state_fallback(&block_name, &properties);
    let default_state_id = BuiltInRegistries::blocks()
        .get_by_location(&block_name)
        .default_state_id;
    BuiltInRegistries::block_states()
        .get_state_by_id(default_state_id)
        .cloned()
}
fn log_block_state_fallback(block_name: &ResourceLocation, properties: &HashMap<String, String>) {
    let mut sorted_props = properties
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>();
    sorted_props.sort();
    let key = format!("{block_name}[{}]", sorted_props.join(","));
    let log = BLOCK_STATE_FALLBACK_LOG.get_or_init(|| Mutex::new(HashSet::new()));
    let Ok(mut seen) = log.lock() else {
        return;
    };
    if !seen.insert(key.clone()) {
        return;
    }
    if VERBOSE_PROTOCOL_LOGS {
        eprintln!("[AzaleaProtocol] BlockState fallback to default for {key}");
    }
    log::warn!("[AzaleaProtocol] BlockState fallback to default for {key}");
}
fn map_boss_bar_color(
    c: azalea_protocol::packets::game::c_boss_event::BossBarColor,
) -> BossBarColor {
    match c {
        azalea_protocol::packets::game::c_boss_event::BossBarColor::Pink => BossBarColor::Pink,
        azalea_protocol::packets::game::c_boss_event::BossBarColor::Blue => BossBarColor::Blue,
        azalea_protocol::packets::game::c_boss_event::BossBarColor::Red => BossBarColor::Red,
        azalea_protocol::packets::game::c_boss_event::BossBarColor::Green => BossBarColor::Green,
        azalea_protocol::packets::game::c_boss_event::BossBarColor::Yellow => BossBarColor::Yellow,
        azalea_protocol::packets::game::c_boss_event::BossBarColor::Purple => BossBarColor::Purple,
        azalea_protocol::packets::game::c_boss_event::BossBarColor::White => BossBarColor::White,
    }
}
fn map_boss_bar_overlay(
    o: azalea_protocol::packets::game::c_boss_event::BossBarOverlay,
) -> BossBarOverlay {
    match o {
        azalea_protocol::packets::game::c_boss_event::BossBarOverlay::Progress => {
            BossBarOverlay::Progress
        }
        azalea_protocol::packets::game::c_boss_event::BossBarOverlay::Notched6 => {
            BossBarOverlay::Notched6
        }
        azalea_protocol::packets::game::c_boss_event::BossBarOverlay::Notched10 => {
            BossBarOverlay::Notched10
        }
        azalea_protocol::packets::game::c_boss_event::BossBarOverlay::Notched12 => {
            BossBarOverlay::Notched12
        }
        azalea_protocol::packets::game::c_boss_event::BossBarOverlay::Notched20 => {
            BossBarOverlay::Notched20
        }
    }
}
fn map_boss_event_operation(
    op: &azalea_protocol::packets::game::c_boss_event::Operation,
) -> BossEventOperation {
    match op {
        azalea_protocol::packets::game::c_boss_event::Operation::Add(add) => {
            BossEventOperation::Add(BossEventAdd {
                name: add.name.to_ansi(),
                progress: add.progress,
                style: BossEventStyle {
                    color: map_boss_bar_color(add.style.color),
                    overlay: map_boss_bar_overlay(add.style.overlay),
                },
                properties: BossEventProperties {
                    darken_screen: add.properties.darken_screen,
                    play_music: add.properties.play_music,
                    create_world_fog: add.properties.create_world_fog,
                },
            })
        }
        azalea_protocol::packets::game::c_boss_event::Operation::Remove => {
            BossEventOperation::Remove
        }
        azalea_protocol::packets::game::c_boss_event::Operation::UpdateProgress(p) => {
            BossEventOperation::UpdateProgress(*p)
        }
        azalea_protocol::packets::game::c_boss_event::Operation::UpdateName(name) => {
            BossEventOperation::UpdateName(name.to_ansi())
        }
        azalea_protocol::packets::game::c_boss_event::Operation::UpdateStyle(style) => {
            BossEventOperation::UpdateStyle(BossEventStyle {
                color: map_boss_bar_color(style.color),
                overlay: map_boss_bar_overlay(style.overlay),
            })
        }
        azalea_protocol::packets::game::c_boss_event::Operation::UpdateProperties(props) => {
            BossEventOperation::UpdateProperties(BossEventProperties {
                darken_screen: props.darken_screen,
                play_music: props.play_music,
                create_world_fog: props.create_world_fog,
            })
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum BossBarColor {
    Pink = 0,
    Blue = 1,
    Red = 2,
    Green = 3,
    Yellow = 4,
    Purple = 5,
    White = 6,
}
#[derive(Clone, Debug, PartialEq)]
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
