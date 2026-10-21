#![allow(dead_code)]
use crate::network::multiplayer::resolver::ServerAddress;
pub struct LegacyServerPinger {
    address: ServerAddress,
}
impl LegacyServerPinger {
    const RESPONSE_ID: u8 = 0xFF;
    pub fn new(address: ServerAddress) -> Self {
        LegacyServerPinger { address }
    }
    pub fn build_ping_packet(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(0xFE);
        buf.push(0x01);
        buf.push(0xFA);
        let plugin_name = "MC|PingHost";
        write_legacy_string(&mut buf, plugin_name);
        let size_index = buf.len();
        buf.extend_from_slice(&[0u8; 2]);
        let payload_start = buf.len();
        buf.push(0x7F);
        write_legacy_string(&mut buf, self.address.host());
        let port = self.address.port() as i32;
        buf.extend_from_slice(&port.to_be_bytes());
        let payload_size = (buf.len() - payload_start) as u16;
        let size_bytes = payload_size.to_be_bytes();
        buf[size_index] = size_bytes[0];
        buf[size_index + 1] = size_bytes[1];
        buf
    }
    pub fn parse_response(data: &[u8]) -> Option<LegacyPingResponse> {
        if data.is_empty() || data[0] != Self::RESPONSE_ID {
            return None;
        }
        let payload = read_legacy_string(&data[1..])?;
        let parts: Vec<&str> = payload.splitn(6, '\0').collect();
        if parts.len() != 6 || parts[0] != "\u{00a7}1" {
            return None;
        }
        let protocol_version: i32 = parts[1].parse().unwrap_or(0);
        let game_version = parts[2].to_owned();
        let motd = parts[3].to_owned();
        let current_players: i32 = parts[4].parse().unwrap_or(-1);
        let max_players: i32 = parts[5].parse().unwrap_or(-1);
        Some(LegacyPingResponse {
            protocol_version,
            game_version,
            motd,
            current_players,
            max_players,
        })
    }
}
#[derive(Debug, Clone)]
pub struct LegacyPingResponse {
    pub protocol_version: i32,
    pub game_version: String,
    pub motd: String,
    pub current_players: i32,
    pub max_players: i32,
}
fn write_legacy_string(buf: &mut Vec<u8>, s: &str) {
    let utf16: Vec<u16> = s.encode_utf16().collect();
    let len = utf16.len() as u16;
    buf.extend_from_slice(&len.to_be_bytes());
    for code_unit in utf16 {
        buf.extend_from_slice(&code_unit.to_be_bytes());
    }
}
fn read_legacy_string(data: &[u8]) -> Option<String> {
    if data.len() < 2 {
        return None;
    }
    let len = u16::from_be_bytes([data[0], data[1]]) as usize;
    let byte_len = len * 2;
    if data.len() < 2 + byte_len {
        return None;
    }
    let raw = &data[2..2 + byte_len];
    let utf16: Vec<u16> = raw
        .chunks_exact(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16(&utf16).ok()
}
