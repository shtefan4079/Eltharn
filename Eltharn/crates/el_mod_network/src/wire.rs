use el_extension_api::ExtensionId;
use el_launcher_core::extensions::PackagePin;
use el_mod_content::{
    ContentId,
    catalog::{Catalog, OwnedSpec},
    scene_types::{InputEvent, SceneView},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
pub const PROTOCOL: u32 = 1;
pub const API: &str = "1.3";
pub const MAX_WIRE_BYTES: usize = 524_288;
const MAX_SAFE: u64 = 9_007_199_254_740_991;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMessage {
    Hello {
        nonce: String,
        protocol: u32,
        api: String,
        packages: Vec<PackagePin>,
        fingerprint: String,
    },
    Action {
        nonce: String,
        sequence: u64,
        revision: u64,
        owner: String,
        event: InputEvent,
    },
    Channel {
        nonce: String,
        sequence: u64,
        revision: u64,
        owner: String,
        channel: String,
        data: Vec<u8>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ServerMessage {
    Offer {
        nonce: String,
        protocol: u32,
        api: String,
        world: String,
        packages: Vec<PackagePin>,
        actors: Vec<String>,
        catalog: Vec<OwnedSpec>,
        fingerprint: String,
    },
    State {
        nonce: String,
        revision: u64,
        scene: SceneView,
        ack: Option<Ack>,
        messages: Vec<ChannelMessage>,
    },
    Reject {
        reason: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ack {
    pub sequence: u64,
    pub accepted: bool,
    pub reason: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelMessage {
    pub owner: String,
    pub channel: String,
    pub data: Vec<u8>,
}
fn hex32(s: &str) -> bool {
    s.len() == 32
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn hex64(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn safe(n: u64) -> bool {
    n <= MAX_SAFE
}
fn owner_package(owner: &str) -> Result<ExtensionId, String> {
    if owner.len() > 160 {
        return Err("owner too long".into());
    }
    let (pkg, slot) = owner.rsplit_once('/').ok_or("owner needs package/slot")?;
    if !matches!(slot, "server" | "shared") {
        return Err("invalid server actor".into());
    }
    pkg.parse::<ExtensionId>()
        .map_err(|_| "invalid owner package".into())
}
fn packages_ok(packages: &[PackagePin]) -> Result<(), String> {
    if packages.len() > 64 {
        return Err("too many packages".into());
    }
    let mut seen = BTreeSet::new();
    for p in packages {
        if p.id.is_reserved_namespace() || !seen.insert(p.id.to_string()) {
            return Err("invalid package roster".into());
        }
    }
    Ok(())
}
fn channel_ok(owner: &str, channel: &str, data: &[u8]) -> Result<(), String> {
    if data.len() > 8192 {
        return Err("channel data too large".into());
    }
    let p = owner_package(owner)?;
    let channel: ContentId = channel.parse().map_err(|_| "invalid channel")?;
    if channel.is_reserved_namespace() || channel.namespace() != p.namespace() {
        return Err("foreign channel".into());
    }
    Ok(())
}
fn reason_ok(s: &str, empty_allowed: bool) -> bool {
    s.len() <= 256 && !s.contains('\0') && (empty_allowed || !s.is_empty())
}
impl ClientMessage {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Hello {
                nonce,
                protocol,
                api,
                packages,
                fingerprint,
            } => {
                if !hex32(nonce) || *protocol != PROTOCOL || api != API || !hex64(fingerprint) {
                    return Err("incompatible hello".into());
                }
                packages_ok(packages)
            }
            Self::Action {
                nonce,
                sequence,
                revision,
                owner,
                event,
            } => {
                if !hex32(nonce) || *sequence == 0 || !safe(*sequence) || !safe(*revision) {
                    return Err("invalid action header".into());
                }
                owner_package(owner)?;
                event.validate_local()?;
                if event.sequence != *sequence {
                    return Err("action sequence mismatch".into());
                }
                Ok(())
            }
            Self::Channel {
                nonce,
                sequence,
                revision,
                owner,
                channel,
                data,
            } => {
                if !hex32(nonce) || *sequence == 0 || !safe(*sequence) || !safe(*revision) {
                    return Err("invalid channel header".into());
                }
                channel_ok(owner, channel, data)
            }
        }
    }
}
impl ServerMessage {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Offer {
                nonce,
                protocol,
                api,
                world,
                packages,
                actors,
                catalog,
                fingerprint,
            } => {
                if !hex32(nonce) || *protocol != PROTOCOL || api != API || !hex64(fingerprint) {
                    return Err("incompatible offer".into());
                }
                let _: ContentId = world.parse().map_err(|_| "invalid world")?;
                packages_ok(packages)?;
                if actors.len() > 16 {
                    return Err("too many server actors".into());
                }
                let mut seen = BTreeSet::new();
                for actor in actors {
                    let package = owner_package(actor)?;
                    if !packages.iter().any(|p| p.id == package) || !seen.insert(actor) {
                        return Err("invalid server actor roster".into());
                    }
                }
                if catalog.len() > 4096 {
                    return Err("catalog too large".into());
                }
                let checked = Catalog::from_entries(catalog.clone())?;
                if checked.fingerprint() != fingerprint {
                    return Err("catalogue fingerprint mismatch".into());
                }
                Ok(())
            }
            Self::State {
                nonce,
                revision,
                scene,
                ack,
                messages,
            } => {
                if !hex32(nonce) || !safe(*revision) || scene.revision != *revision {
                    return Err("invalid state revision".into());
                }
                scene.validate_local()?;
                if messages.len() > 64 {
                    return Err("too many messages".into());
                }
                for m in messages {
                    channel_ok(&m.owner, &m.channel, &m.data)?;
                }
                if let Some(a) = ack {
                    if a.sequence == 0 || !safe(a.sequence) || !reason_ok(&a.reason, true) {
                        return Err("invalid acknowledgement".into());
                    }
                }
                Ok(())
            }
            Self::Reject { reason } => {
                if reason_ok(reason, false) {
                    Ok(())
                } else {
                    Err("invalid rejection".into())
                }
            }
        }
    }
}
pub fn decode_client(bytes: &[u8]) -> Result<ClientMessage, String> {
    if bytes.is_empty() || bytes.len() > MAX_WIRE_BYTES {
        return Err("client message size".into());
    }
    let value = el_launcher_core::extension_json::strict_json(bytes, MAX_WIRE_BYTES)
        .map_err(|_| "invalid client JSON")?;
    let msg: ClientMessage = serde_json::from_value(value).map_err(|_| "invalid client message")?;
    msg.validate()?;
    Ok(msg)
}
pub fn decode_server(bytes: &[u8]) -> Result<ServerMessage, String> {
    if bytes.is_empty() || bytes.len() > MAX_WIRE_BYTES {
        return Err("server message size".into());
    }
    let value = el_launcher_core::extension_json::strict_json(bytes, MAX_WIRE_BYTES)
        .map_err(|_| "invalid server JSON")?;
    let msg: ServerMessage = serde_json::from_value(value).map_err(|_| "invalid server message")?;
    msg.validate()?;
    Ok(msg)
}
pub fn encode_client(msg: &ClientMessage) -> Result<Vec<u8>, String> {
    msg.validate()?;
    let bytes = serde_json::to_vec(msg).map_err(|_| "encode client message")?;
    if bytes.len() > MAX_WIRE_BYTES {
        return Err("client message size".into());
    }
    Ok(bytes)
}
pub fn encode_server(msg: &ServerMessage) -> Result<Vec<u8>, String> {
    msg.validate()?;
    let bytes = serde_json::to_vec(msg).map_err(|_| "encode server message")?;
    if bytes.len() > MAX_WIRE_BYTES {
        return Err("server message size".into());
    }
    Ok(bytes)
}
