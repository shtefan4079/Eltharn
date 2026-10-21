#![allow(dead_code)]
use el_core::Component;
use el_nbt::tag::{Tag, compound_get_tag};
pub fn java_string_hash_code(s: &str) -> i32 {
    let mut hash = 0i32;
    for unit in s.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(unit as i32);
    }
    hash
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerPackStatus {
    Enabled,
    Disabled,
    Prompt,
}
impl ServerPackStatus {
    pub fn name(self) -> Component {
        let suffix = match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::Prompt => "prompt",
        };
        Component::translatable(&format!("manageServer.resourcePack.{}", suffix))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    Initial,
    Pinging,
    Unreachable,
    Incompatible,
    Successful,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerType {
    Lan,
    Realm,
    Other,
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ServerData {
    pub name: String,
    pub ip: String,
    pub status: Component,
    pub motd: Component,
    pub players_max: i32,
    pub players_online: i32,
    pub players_sample: Vec<Component>,
    pub ping: i64,
    pub protocol: i32,
    pub version: Component,
    pub player_list: Vec<Component>,
    pub pack_status: ServerPackStatus,
    pub icon_bytes: Option<Vec<u8>>,
    pub server_type: ServerType,
    pub accepted_code_of_conduct: i32,
    pub state: ServerState,
}
impl ServerData {
    pub fn new(name: String, ip: String, server_type: ServerType) -> Self {
        Self {
            name,
            ip,
            status: Component::default(),
            motd: Component::default(),
            players_max: 0,
            players_online: 0,
            players_sample: Vec::new(),
            ping: 0,
            protocol: 0,
            version: Component::default(),
            player_list: Vec::new(),
            pack_status: ServerPackStatus::Prompt,
            icon_bytes: None,
            server_type,
            accepted_code_of_conduct: 0,
            state: ServerState::Initial,
        }
    }
    pub fn write(&self) -> Tag {
        let mut fields = Vec::new();
        fields.push(("name".to_string(), Tag::String(self.name.clone())));
        fields.push(("ip".to_string(), Tag::String(self.ip.clone())));
        if let Some(ref icon) = self.icon_bytes {
            use base64::{Engine as _, prelude::BASE64_STANDARD};
            fields.push((
                "icon".to_string(),
                Tag::String(BASE64_STANDARD.encode(icon)),
            ));
        }
        match self.pack_status {
            ServerPackStatus::Enabled => {
                fields.push(("acceptTextures".to_string(), Tag::Byte(1)));
            }
            ServerPackStatus::Disabled => {
                fields.push(("acceptTextures".to_string(), Tag::Byte(0)));
            }
            ServerPackStatus::Prompt => {}
        }
        if self.accepted_code_of_conduct != 0 {
            fields.push((
                "acceptedCodeOfConduct".to_string(),
                Tag::Int(self.accepted_code_of_conduct),
            ));
        }
        Tag::Compound(fields)
    }
    pub fn read(tag: &Tag) -> Self {
        let name = if let Some(Tag::String(s)) = compound_get_tag(tag, "name") {
            s.clone()
        } else {
            String::new()
        };
        let ip = if let Some(Tag::String(s)) = compound_get_tag(tag, "ip") {
            let cleaned: String = s
                .chars()
                .filter(|c| !c.is_control() && *c != '\0')
                .collect();
            if cleaned != *s {
                eprintln!(
                    "[ServerData] WARNING: corrupted IP in servers.dat: {:?} → cleaned to {:?}",
                    s, cleaned
                );
            }
            cleaned
        } else {
            String::new()
        };
        let icon_bytes = match compound_get_tag(tag, "icon") {
            Some(Tag::String(s)) => {
                use base64::{Engine as _, prelude::BASE64_STANDARD};
                BASE64_STANDARD.decode(s).ok()
            }
            Some(Tag::ByteArray(arr)) => Some(arr.iter().map(|&x| x as u8).collect()),
            _ => None,
        };
        let pack_status = match compound_get_tag(tag, "acceptTextures") {
            Some(Tag::Byte(b)) => {
                if *b != 0 {
                    ServerPackStatus::Enabled
                } else {
                    ServerPackStatus::Disabled
                }
            }
            _ => ServerPackStatus::Prompt,
        };
        let accepted_code_of_conduct =
            if let Some(Tag::Int(i)) = compound_get_tag(tag, "acceptedCodeOfConduct") {
                *i
            } else {
                0
            };
        let mut data = Self::new(name, ip, ServerType::Other);
        data.icon_bytes = icon_bytes;
        data.pack_status = pack_status;
        data.accepted_code_of_conduct = accepted_code_of_conduct;
        data
    }
    pub fn is_lan(&self) -> bool {
        self.server_type == ServerType::Lan
    }
    pub fn has_accepted_code_of_conduct(&self, code_of_conduct: &str) -> bool {
        self.accepted_code_of_conduct != 0
            && self.accepted_code_of_conduct == java_string_hash_code(code_of_conduct)
    }
    pub fn accept_code_of_conduct(&mut self, code_of_conduct: &str) {
        self.accepted_code_of_conduct = java_string_hash_code(code_of_conduct);
    }
    pub fn clear_code_of_conduct(&mut self) {
        self.accepted_code_of_conduct = 0;
    }
    pub fn is_realm(&self) -> bool {
        self.server_type == ServerType::Realm
    }
    pub fn copy_name_icon_from(&mut self, other: &ServerData) {
        self.ip = other.ip.clone();
        self.name = other.name.clone();
        self.icon_bytes = other.icon_bytes.clone();
    }
    pub fn copy_from(&mut self, other: &ServerData) {
        self.copy_name_icon_from(other);
        self.pack_status = other.pack_status;
        self.server_type = other.server_type;
    }
}
