#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerAddress {
    pub host: String,
    pub port: u16,
}
pub const DEFAULT_PORT: u16 = 25565;
impl ServerAddress {
    pub fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }
    pub fn invalid() -> Self {
        Self::new("server.invalid".to_string(), DEFAULT_PORT)
    }
    pub fn host(&self) -> &str {
        &self.host
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn parse_string(input: &str) -> Self {
        match parse_host_and_port(input) {
            Some((host, port)) => Self::new(host, port.unwrap_or(DEFAULT_PORT)),
            None => {
                log::error!("Failed to parse URL {}", input);
                Self::invalid()
            }
        }
    }
}
fn parse_host_and_port(input: &str) -> Option<(String, Option<u16>)> {
    if input.is_empty() {
        return None;
    }
    let (host, port_str): (&str, Option<&str>) = if let Some(stripped) = input.strip_prefix('[') {
        let close_bracket_idx = stripped.find(']')?;
        let host = &stripped[..close_bracket_idx];
        if host.contains('[') {
            return None;
        }
        let rest = &stripped[close_bracket_idx + 1..];
        if rest.is_empty() {
            (host, None)
        } else {
            (host, Some(rest.strip_prefix(':')?))
        }
    } else {
        match input.split_once(':') {
            Some((host, port)) => {
                if port.contains(':') {
                    return None;
                }
                (host, Some(port))
            }
            None => (input, None),
        }
    };
    let port = match port_str {
        Some(port) => {
            if port.is_empty() || !port.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            Some(port.parse::<u16>().ok()?)
        }
        None => None,
    };
    Some((host.to_string(), port))
}
