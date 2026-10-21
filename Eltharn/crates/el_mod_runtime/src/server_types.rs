use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const SAFE_MAX: u64 = 9007199254740991;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServerPeer {
    pub id: String,
    pub world: String,
    pub position_milli: [i64; 3],
    pub look_milli: [i32; 3],
    pub reach_milli: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServerEvent {
    pub peer: String,
    pub sequence: u64,
    pub event: Option<el_mod_content::scene_types::InputEvent>,
    pub channel: Option<String>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct ServerFrame {
    pub sequence: u64,
    pub peers: Vec<ServerPeer>,
    pub events: Vec<ServerEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NetworkOutput {
    pub owner: String,
    pub to: Option<String>,
    pub channel: String,
    pub data: Vec<u8>,
}

fn is_uuid(s: &str) -> bool {
    if s.len() != 36 {
        return false;
    }
    let b = s.as_bytes();
    if b[8] != b'-' || b[13] != b'-' || b[18] != b'-' || b[23] != b'-' {
        return false;
    }
    for (i, c) in b.iter().enumerate() {
        if i == 8 || i == 13 || i == 18 || i == 23 {
            continue;
        }
        if !matches!(c, b'0'..=b'9' | b'a'..=b'f') {
            return false;
        }
    }
    true
}

impl ServerFrame {
    pub fn validate(&self) -> Result<(), String> {
        if self.sequence > SAFE_MAX {
            return Err(format!("sequence {:?} > {SAFE_MAX}", self.sequence));
        }
        if self.sequence == 0 && (!self.peers.is_empty() || !self.events.is_empty()) {
            return Err("sequence 0 only for empty frame".into());
        }
        if self.peers.len() > 64 {
            return Err(format!("peers {} > 64", self.peers.len()));
        }
        if self.events.len() > 32 {
            return Err(format!("events {} > 32", self.events.len()));
        }
        let mut seen = HashSet::new();
        for (i, p) in self.peers.iter().enumerate() {
            if !is_uuid(&p.id) {
                return Err(format!("peers[{i}].id not lowercase uuid"));
            }
            if !seen.insert(&p.id) {
                return Err(format!("peers[{i}].id duplicate"));
            }
            if p.world.len() > 128 {
                return Err(format!("peers[{i}].world > 128 bytes"));
            }
            p.world
                .parse::<el_mod_content::ContentId>()
                .map_err(|e| format!("peers[{i}].world: {e:?}"))?;
            if p.position_milli
                .iter()
                .any(|&v| v < -30000000000 || v > 30000000000)
            {
                return Err(format!("peers[{i}].position_milli out of range"));
            }
            if p.look_milli.iter().any(|&v| v < -1000 || v > 1000) {
                return Err(format!("peers[{i}].look_milli out of range"));
            }
            if p.reach_milli > 64000 {
                return Err(format!("peers[{i}].reach_milli > 64000"));
            }
        }
        for (i, e) in self.events.iter().enumerate() {
            if !seen.contains(&e.peer) {
                return Err(format!("events[{i}].peer unknown"));
            }
            if e.sequence == 0 || e.sequence > SAFE_MAX {
                return Err(format!("events[{i}].sequence not positive safe"));
            }
            match (&e.event, &e.channel) {
                (Some(ev), None) => {
                    if !e.data.is_empty() {
                        return Err(format!("events[{i}].data must be empty for input"));
                    }
                    if ev.sequence != e.sequence {
                        return Err("event sequence mismatch".into());
                    }
                    ev.validate_local()
                        .map_err(|err| format!("events[{i}].event: {err:?}"))?;
                }
                (None, Some(c)) => {
                    if e.data.len() > 8192 {
                        return Err(format!("events[{i}].data > 8192"));
                    }
                    let id = c
                        .parse::<el_mod_content::ContentId>()
                        .map_err(|err| format!("events[{i}].channel: {err:?}"))?;
                    if id.is_reserved_namespace() {
                        return Err(format!("events[{i}].channel reserved"));
                    }
                }
                _ => return Err(format!("events[{i}] need exactly one of event/channel")),
            }
        }
        let n = serde_json::to_vec(self)
            .map_err(|e| format!("json: {e:?}"))?
            .len();
        if n > 65536 {
            return Err(format!("frame {n} > 65536 bytes"));
        }
        Ok(())
    }
}

pub fn validate_network(
    owner: &str,
    to: Option<&str>,
    channel: &str,
    data: &[u8],
    frame: &ServerFrame,
) -> Result<(), String> {
    if data.len() > 8192 {
        return Err("network data limit".into());
    }
    let (package, slot) = owner.rsplit_once('/').ok_or("bad owner")?;
    if !matches!(slot, "server" | "shared") {
        return Err("not a server actor".into());
    }
    let package: el_extension_api::ExtensionId = package.parse().map_err(|_| "bad owner id")?;
    let channel: el_mod_content::ContentId = channel.parse().map_err(|_| "bad channel")?;
    if channel.is_reserved_namespace() || channel.namespace() != package.namespace() {
        return Err("foreign channel".into());
    }
    if let Some(peer) = to {
        if !frame.peers.iter().any(|p| p.id == peer) {
            return Err("unknown peer".into());
        }
    }
    Ok(())
}

pub fn validate_client_network(
    owner: &str,
    to: Option<&str>,
    channel: &str,
    data: &[u8],
) -> Result<(), String> {
    if data.len() > 8192 {
        return Err("network data limit".into());
    }
    let (package, slot) = owner.rsplit_once('/').ok_or("bad owner")?;
    if !matches!(slot, "client" | "shared") {
        return Err("not a client actor".into());
    }
    let package: el_extension_api::ExtensionId = package.parse().map_err(|_| "bad owner id")?;
    let channel: el_mod_content::ContentId = channel.parse().map_err(|_| "bad channel")?;
    if channel.is_reserved_namespace() || channel.namespace() != package.namespace() {
        return Err("foreign channel".into());
    }
    if let Some(target) = to {
        let (target_package, target_slot) = target.rsplit_once('/').ok_or("bad target")?;
        if target_package != package.to_string() || !matches!(target_slot, "server" | "shared") {
            return Err("foreign server target".into());
        }
    }
    Ok(())
}
