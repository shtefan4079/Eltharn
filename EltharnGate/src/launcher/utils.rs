#![allow(dead_code)]
use sha1::{Digest, Sha1};
pub const USER_AGENT: &str = "EltharnGate/0.1.0 (Eltharn Launcher)";
pub fn sha1_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
pub fn player_initials(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "?".to_string();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() >= 2 && chars[0].is_alphabetic() && chars[1].is_alphabetic() {
        format!("{}{}", chars[0].to_uppercase(), chars[1].to_lowercase())
    } else {
        chars[0].to_uppercase().to_string()
    }
}
pub fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}
pub fn desktop_exec_quote(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for ch in value.chars() {
        if matches!(ch, '\\' | '"' | '`' | '$') {
            quoted.push('\\');
        }
        quoted.push(ch);
    }
    quoted.push('"');
    quoted
}
