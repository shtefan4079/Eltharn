use crate::network::multiplayer::legacy_server_pinger::{LegacyPingResponse, LegacyServerPinger};
use crate::network::multiplayer::resolver::ServerAddress;
pub enum PingQueryError {
    Protocol,
}
pub struct StatusQueryOutcome {
    pub motd: String,
    pub motd_runs: Vec<el_gui::screens::multiplayer::MotdRun>,
    pub version: Option<(String, i32)>,
    pub players: Option<(i32, i32)>,
    pub player_list: Vec<String>,
    pub ping: Option<i64>,
    pub icon_bytes: Option<Vec<u8>>,
}
fn write_varint(buf: &mut Vec<u8>, mut val: i32) {
    loop {
        let mut temp = (val & 0b0111_1111) as u8;
        val = ((val as u32) >> 7) as i32;
        if val != 0 {
            temp |= 0b1000_0000;
        }
        buf.push(temp);
        if val == 0 {
            break;
        }
    }
}
fn read_varint<R: std::io::Read>(reader: &mut R) -> std::io::Result<i32> {
    let mut num_read = 0;
    let mut result = 0;
    loop {
        let mut read = [0u8; 1];
        reader.read_exact(&mut read)?;
        if num_read >= 5 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "VarInt too big",
            ));
        }
        let value = (read[0] & 0b0111_1111) as i32;
        result |= value << (7 * num_read);
        num_read += 1;
        if (read[0] & 0b1000_0000) == 0 {
            break;
        }
    }
    Ok(result)
}
fn write_string(buf: &mut Vec<u8>, s: &str) {
    write_varint(buf, s.len() as i32);
    buf.extend_from_slice(s.as_bytes());
}
fn flatten_component_string(value: &serde_json::Value, out: &mut String) {
    match value {
        serde_json::Value::String(s) => out.push_str(s),
        serde_json::Value::Array(items) => {
            for item in items {
                flatten_component_string(item, out);
            }
        }
        serde_json::Value::Object(map) => {
            if let Some(text) = map.get("text").and_then(|t| t.as_str()) {
                out.push_str(text);
            }
            if let Some(extra) = map.get("extra").and_then(|e| e.as_array()) {
                for item in extra {
                    flatten_component_string(item, out);
                }
            }
        }
        _ => {}
    }
}
fn strip_legacy_formatting(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{a7}' {
            if chars.next().is_none() {
                break;
            }
        } else {
            out.push(c);
        }
    }
    out
}
fn parse_text_color(value: &serde_json::Value) -> Option<u32> {
    let raw = value.as_str()?;
    if let Some(hex) = raw.strip_prefix('#') {
        return u32::from_str_radix(hex, 16)
            .ok()
            .filter(|v| *v <= 0xFF_FFFF);
    }
    let id = match raw {
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
fn element_color(value: &serde_json::Value, inherited: Option<u32>) -> Option<u32> {
    match value {
        serde_json::Value::Object(map) => map.get("color").and_then(parse_text_color).or(inherited),
        _ => inherited,
    }
}
fn parse_legacy_text_runs(
    text: &str,
    base_color: Option<u32>,
) -> Vec<el_gui::screens::multiplayer::MotdRun> {
    use el_gui::screens::multiplayer::MotdRun;
    let mut runs: Vec<MotdRun> = Vec::new();
    let mut current_color = base_color;
    let mut buffer = String::new();
    let mut chars = text.chars().peekable();
    let flush = |runs: &mut Vec<MotdRun>, buffer: &mut String, color: Option<u32>| {
        if !buffer.is_empty() {
            runs.push(MotdRun {
                text: std::mem::take(buffer),
                color,
            });
        }
    };
    while let Some(c) = chars.next() {
        if c == '\u{a7}' {
            let Some(code) = chars.next() else { break };
            match code.to_ascii_lowercase() {
                '0'..='9' | 'a'..='f' => {
                    let id = if ('0'..='9').contains(&code) {
                        code as i32 - '0' as i32
                    } else {
                        10 + (code as i32 - 'a' as i32)
                    };
                    flush(&mut runs, &mut buffer, current_color);
                    current_color = crate::ui::scoreboard::chat_color_rgb(id);
                }
                'r' => {
                    flush(&mut runs, &mut buffer, current_color);
                    current_color = base_color;
                }
                _ => {}
            }
        } else {
            buffer.push(c);
        }
    }
    flush(&mut runs, &mut buffer, current_color);
    runs
}
fn flatten_component_runs(
    value: &serde_json::Value,
    inherited: Option<u32>,
) -> Vec<el_gui::screens::multiplayer::MotdRun> {
    match value {
        serde_json::Value::String(s) => parse_legacy_text_runs(s, inherited),
        serde_json::Value::Array(items) => {
            let mut out = Vec::new();
            if let Some(first) = items.first() {
                out.extend(flatten_component_runs(first, inherited));
                let inherited_color = element_color(first, inherited);
                for item in items.iter().skip(1) {
                    out.extend(flatten_component_runs(item, inherited_color));
                }
            }
            out
        }
        serde_json::Value::Object(map) => {
            let self_color = map.get("color").and_then(parse_text_color).or(inherited);
            let mut out = Vec::new();
            if let Some(text) = map.get("text").and_then(|t| t.as_str()) {
                out.extend(parse_legacy_text_runs(text, self_color));
            }
            if let Some(extra) = map.get("extra").and_then(|e| e.as_array()) {
                for item in extra {
                    out.extend(flatten_component_runs(item, self_color));
                }
            }
            out
        }
        _ => Vec::new(),
    }
}
pub fn query_minecraft_server<F>(
    stream: std::net::TcpStream,
    host: &str,
    port: u16,
    protocol_version: i32,
    on_status_response: F,
) -> Result<StatusQueryOutcome, PingQueryError>
where
    F: FnOnce(&StatusQueryOutcome),
{
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        query_minecraft_server_inner(stream, host, port, protocol_version, on_status_response)
    })) {
        Ok(result) => result,
        Err(_) => {
            log::warn!("[ServerPing] status query panicked; reporting protocol failure");
            Err(PingQueryError::Protocol)
        }
    }
}
fn query_minecraft_server_inner<F>(
    mut stream: std::net::TcpStream,
    host: &str,
    port: u16,
    protocol_version: i32,
    on_status_response: F,
) -> Result<StatusQueryOutcome, PingQueryError>
where
    F: FnOnce(&StatusQueryOutcome),
{
    use std::io::{Read, Write};
    let _ = stream.set_nodelay(true);
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(30)));
    let mut handshake = Vec::new();
    write_varint(&mut handshake, 0x00);
    write_varint(&mut handshake, protocol_version);
    write_string(&mut handshake, host);
    handshake.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut handshake, 1);
    let mut handshake_packet = Vec::new();
    write_varint(&mut handshake_packet, handshake.len() as i32);
    handshake_packet.extend(handshake);
    stream
        .write_all(&handshake_packet)
        .map_err(|_| PingQueryError::Protocol)?;
    let mut status_req = Vec::new();
    write_varint(&mut status_req, 0x00);
    let mut status_req_packet = Vec::new();
    write_varint(&mut status_req_packet, status_req.len() as i32);
    status_req_packet.extend(status_req);
    stream
        .write_all(&status_req_packet)
        .map_err(|_| PingQueryError::Protocol)?;
    let _packet_len = read_varint(&mut stream).map_err(|_| PingQueryError::Protocol)?;
    let packet_id = read_varint(&mut stream).map_err(|_| PingQueryError::Protocol)?;
    if packet_id != 0x00 {
        return Err(PingQueryError::Protocol);
    }
    let json_len = read_varint(&mut stream).map_err(|_| PingQueryError::Protocol)? as usize;
    if json_len > 32767 {
        return Err(PingQueryError::Protocol);
    }
    let mut json_bytes = vec![0u8; json_len];
    stream
        .read_exact(&mut json_bytes)
        .map_err(|_| PingQueryError::Protocol)?;
    let json_str = String::from_utf8(json_bytes).map_err(|_| PingQueryError::Protocol)?;
    #[derive(serde::Deserialize)]
    struct PingResponse {
        version: Option<VersionInfo>,
        players: Option<PlayersInfo>,
        description: Option<serde_json::Value>,
        favicon: Option<String>,
    }
    #[derive(serde::Deserialize)]
    struct VersionInfo {
        name: Option<String>,
        protocol: Option<i32>,
    }
    #[derive(serde::Deserialize)]
    struct PlayersInfo {
        online: Option<i32>,
        max: Option<i32>,
        sample: Option<Vec<PlayerSample>>,
    }
    #[derive(serde::Deserialize)]
    struct PlayerSample {
        id: Option<String>,
        name: Option<String>,
    }
    let resp: PingResponse =
        serde_json::from_str(&json_str).map_err(|_| PingQueryError::Protocol)?;
    let motd = match resp.description {
        Some(ref desc) => {
            let mut flat = String::new();
            flatten_component_string(desc, &mut flat);
            strip_legacy_formatting(&flat)
        }
        None => String::new(),
    };
    let motd_runs = match resp.description {
        Some(ref desc) => flatten_component_runs(desc, None),
        None => Vec::new(),
    };
    let version = resp.version.and_then(|v| {
        let name = v.name?;
        Some((name, v.protocol.unwrap_or(0)))
    });
    let icon_bytes = resp.favicon.and_then(|value| {
        let encoded = value
            .strip_prefix("data:image/png;base64,")
            .or_else(|| value.strip_prefix("data:image/png;base64;"))
            .unwrap_or(value.as_str());
        let cleaned: String = encoded.chars().filter(|c| !c.is_whitespace()).collect();
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, cleaned).ok()
    });
    let players = resp
        .players
        .as_ref()
        .map(|p| (p.online.unwrap_or(0), p.max.unwrap_or(0)));
    let mut player_list = Vec::new();
    if let Some(players_info) = resp.players.as_ref() {
        let sample = players_info.sample.as_deref().unwrap_or(&[]);
        for p in sample {
            let Some(name) = p.name.as_ref() else {
                continue;
            };
            let is_anonymous =
                p.id.as_deref()
                    .map(|id| {
                        let hex: String = id.chars().filter(|c| *c != '-').collect();
                        hex.len() == 32 && hex.chars().all(|c| c == '0')
                    })
                    .unwrap_or(false)
                    && name == "Anonymous Player";
            if is_anonymous {
                player_list.push(
                    crate::ui::i18n::translate("multiplayer.status.anonymous_player").to_string(),
                );
            } else {
                player_list.push(name.clone());
            }
        }
        let online = players_info.online.unwrap_or(0);
        if !sample.is_empty() && (sample.len() as i32) < online {
            let diff = online - sample.len() as i32;
            let and_more_trans =
                crate::ui::i18n::translate("multiplayer.status.and_more").to_string();
            player_list.push(and_more_trans.replace("%s", &diff.to_string()));
        }
    }
    let mut outcome = StatusQueryOutcome {
        motd,
        motd_runs,
        version,
        players,
        player_list,
        ping: None,
        icon_bytes,
    };
    on_status_response(&outcome);
    let ping_start = web_time::Instant::now();
    let mut ping_req = Vec::new();
    write_varint(&mut ping_req, 0x01);
    let ping_time = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    ping_req.extend_from_slice(&ping_time.to_be_bytes());
    let mut ping_req_packet = Vec::new();
    write_varint(&mut ping_req_packet, ping_req.len() as i32);
    ping_req_packet.extend(ping_req);
    if stream.write_all(&ping_req_packet).is_ok() {
        if let Ok(_pong_len) = read_varint(&mut stream) {
            if let Ok(pong_packet_id) = read_varint(&mut stream) {
                if pong_packet_id == 0x01 {
                    let mut pong_payload = [0u8; 8];
                    if stream.read_exact(&mut pong_payload).is_ok() {
                        outcome.ping = Some(ping_start.elapsed().as_millis() as i64);
                    }
                }
            }
        }
    }
    Ok(outcome)
}
pub fn query_legacy_server(
    sock_addr: &std::net::SocketAddr,
    host: &str,
) -> Result<LegacyPingResponse, ()> {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    let mut stream =
        TcpStream::connect_timeout(sock_addr, std::time::Duration::from_secs(3)).map_err(|_| ())?;
    let _ = stream.set_nodelay(true);
    let addr = ServerAddress::new(host.to_string(), sock_addr.port());
    let pinger = LegacyServerPinger::new(addr);
    let packet = pinger.build_ping_packet();
    stream.write_all(&packet).map_err(|_| ())?;
    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).map_err(|_| ())?;
    LegacyServerPinger::parse_response(&buf[..n]).ok_or(())
}
