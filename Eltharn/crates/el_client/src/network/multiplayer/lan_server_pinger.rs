pub const MULTICAST_GROUP: &str = "224.0.2.60";
pub const PING_PORT: u16 = 4445;
#[allow(dead_code)]
pub fn create_ping_string(motd: &str, address: &str) -> String {
    format!("[MOTD]{}[/MOTD][AD]{}[/AD]", motd, address)
}
pub fn parse_motd(ping_string: &str) -> String {
    let Some(start_index) = ping_string.find("[MOTD]") else {
        return "missing no".to_string();
    };
    let content_start = start_index + "[MOTD]".len();
    match ping_string[content_start..].find("[/MOTD]") {
        Some(relative_end) => ping_string[content_start..content_start + relative_end].to_string(),
        None => "missing no".to_string(),
    }
}
pub fn parse_address(ping_string: &str) -> Option<String> {
    let end_motd_index = ping_string.find("[/MOTD]")?;
    let after_motd = end_motd_index + "[/MOTD]".len();
    if ping_string[after_motd..].contains("[/MOTD]") {
        return None;
    }
    let start_index = after_motd + ping_string[after_motd..].find("[AD]")?;
    let content_start = start_index + "[AD]".len();
    let relative_end = ping_string[content_start..].find("[/AD]")?;
    Some(ping_string[content_start..content_start + relative_end].to_string())
}
