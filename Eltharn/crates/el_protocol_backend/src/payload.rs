pub const SERVERBOUND_CUSTOM_PAYLOAD_MAX: usize = 32767;
pub const CLIENTBOUND_CUSTOM_PAYLOAD_MAX: usize = 1048576;
const CHANNEL_MAX_UTF8_BYTES: usize = 32767;
fn is_namespace_byte(b: u8) -> bool {
    matches!(b, b'a'..= b'z' | b'0'..= b'9' | b'_' | b'.' | b'-')
}
fn is_path_byte(b: u8) -> bool {
    matches!(b, b'a'..= b'z' | b'0'..= b'9' | b'/' | b'.' | b'_' | b'-')
}
pub fn validate_custom_payload(
    channel: &str,
    data: &[u8],
    serverbound: bool,
) -> Result<(), String> {
    let channel_bytes = channel.as_bytes();
    if channel_bytes.len() > CHANNEL_MAX_UTF8_BYTES {
        return Err("Invalid custom payload channel or size".into());
    }
    if channel.is_empty() {
        return Err("invalid custom payload channel \"\": empty identifier".to_owned());
    }
    let colon_count = channel_bytes.iter().filter(|&&b| b == b':').count();
    if colon_count != 1 {
        return Err("Invalid custom payload channel or size".into());
    }
    let (namespace, path) = channel.split_once(':').unwrap_or(("", ""));
    if namespace.is_empty() {
        return Err("Invalid custom payload channel or size".into());
    }
    if path.is_empty() {
        return Err("Invalid custom payload channel or size".into());
    }
    for &b in namespace.as_bytes() {
        if !is_namespace_byte(b) {
            return Err("Invalid custom payload channel or size".into());
        }
    }
    for &b in path.as_bytes() {
        if !is_path_byte(b) {
            return Err("Invalid custom payload channel or size".into());
        }
    }
    let limit = if serverbound {
        SERVERBOUND_CUSTOM_PAYLOAD_MAX
    } else {
        CLIENTBOUND_CUSTOM_PAYLOAD_MAX
    };
    if data.len() > limit {
        return Err("Invalid custom payload channel or size".into());
    }
    Ok(())
}
