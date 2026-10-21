use serde::{Deserialize, Serialize};
use std::fmt;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtocolId(String);
impl ProtocolId {
    pub fn new(value: impl Into<String>) -> Result<Self, ProtocolIdError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ProtocolIdError::Empty);
        }
        if value.len() > 128 {
            return Err(ProtocolIdError::TooLong);
        }
        if value.contains("..")
            || value.contains('/')
            || value.contains('\\')
            || value.chars().any(char::is_control)
        {
            return Err(ProtocolIdError::Unsafe { value });
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn into_inner(self) -> String {
        self.0
    }
}
impl fmt::Display for ProtocolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl AsRef<str> for ProtocolId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl TryFrom<String> for ProtocolId {
    type Error = ProtocolIdError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl TryFrom<&str> for ProtocolId {
    type Error = ProtocolIdError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolIdError {
    Empty,
    TooLong,
    Unsafe { value: String },
}
impl fmt::Display for ProtocolIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("protocol version cannot be empty"),
            Self::TooLong => f.write_str("protocol version is longer than 128 bytes"),
            Self::Unsafe { value } => write!(f, "unsafe protocol version: {value}"),
        }
    }
}
impl std::error::Error for ProtocolIdError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtocolAbiVersion(pub u32);
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetCompatibilityId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetManifestHash(pub String);
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetStoreId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetSelection {
    pub compatibility_id: AssetCompatibilityId,
    pub manifest_hash: AssetManifestHash,
    pub store_id: AssetStoreId,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LaunchId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GameVersionId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AccountId(String);
impl AccountId {
    pub fn new(value: impl Into<String>) -> Result<Self, AccountIdError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(AccountIdError::Empty);
        }
        if value.len() > 256 || value.chars().any(char::is_control) {
            return Err(AccountIdError::Invalid { value });
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountIdError {
    Empty,
    Invalid { value: String },
}
impl fmt::Display for AccountIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("account id cannot be empty"),
            Self::Invalid { value } => write!(f, "invalid account id: {value}"),
        }
    }
}
impl std::error::Error for AccountIdError {}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerTarget {
    pub host: String,
    pub port: u16,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionCompatibilityBinding {
    pub game_version: GameVersionId,
    pub wire_protocol: ProtocolId,
    pub protocol_abi_version: ProtocolAbiVersion,
    pub asset_compatibility: AssetCompatibilityId,
    pub manifest_hash: AssetManifestHash,
}
impl VersionCompatibilityBinding {
    pub fn new(
        game_version: impl Into<String>,
        wire_protocol: ProtocolId,
        protocol_abi_version: ProtocolAbiVersion,
        asset_compatibility: impl Into<String>,
        manifest_hash: impl Into<String>,
    ) -> Self {
        Self {
            game_version: GameVersionId(game_version.into()),
            wire_protocol,
            protocol_abi_version,
            asset_compatibility: AssetCompatibilityId(asset_compatibility.into()),
            manifest_hash: AssetManifestHash(manifest_hash.into()),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameLaunchConfig {
    pub launch_id: LaunchId,
    pub account_id: AccountId,
    #[serde(default)]
    pub game_version: Option<GameVersionId>,
    pub protocol_id: ProtocolId,
    pub protocol_abi_version: ProtocolAbiVersion,
    pub assets: AssetSelection,
    pub locale: Option<String>,
    pub target_server: Option<ServerTarget>,
}
impl GameLaunchConfig {
    pub fn opens_title_screen(&self) -> bool {
        self.target_server.is_none()
    }
    pub fn version_binding(&self) -> VersionCompatibilityBinding {
        VersionCompatibilityBinding {
            game_version: self
                .game_version
                .clone()
                .unwrap_or_else(|| GameVersionId("26.2".to_string())),
            wire_protocol: self.protocol_id.clone(),
            protocol_abi_version: self.protocol_abi_version,
            asset_compatibility: self.assets.compatibility_id.clone(),
            manifest_hash: self.assets.manifest_hash.clone(),
        }
    }
}
