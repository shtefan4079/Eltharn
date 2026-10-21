use crate::ProtocolId;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendTarget {
    Builtin(ProtocolId),
    External(String),
}
impl BackendTarget {
    pub fn parse(selection: &str) -> Result<Self, String> {
        if let Some(version) = selection.strip_prefix("builtin:") {
            ProtocolId::new(version.to_owned())
                .map(Self::Builtin)
                .map_err(|error| error.to_string())
        } else if selection.trim().is_empty() {
            Err("Protocol backend selection is empty".to_owned())
        } else {
            Ok(Self::External(selection.to_owned()))
        }
    }
    pub fn builtin_version(&self) -> Option<&str> {
        match self {
            Self::Builtin(version) => Some(version.as_str()),
            Self::External(_) => None,
        }
    }
}
