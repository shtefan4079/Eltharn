use crate::Sha256Digest;
use core::fmt;
use serde::{Deserialize, Serialize};
pub const LAUNCH_SET_SCHEMA_VERSION: u32 = 1;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchMode {
    Base,
    Mods,
    Shaders,
    ModsAndShaders,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ExtensionFlags {
    #[serde(default)]
    pub mods_enabled: bool,
    #[serde(default)]
    pub shaders_enabled: bool,
}
impl ExtensionFlags {
    pub fn mode(&self) -> LaunchMode {
        match (self.mods_enabled, self.shaders_enabled) {
            (false, false) => LaunchMode::Base,
            (true, false) => LaunchMode::Mods,
            (false, true) => LaunchMode::Shaders,
            (true, true) => LaunchMode::ModsAndShaders,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LaunchError {
    UnsupportedSchemaVersion {
        got: u32,
    },
    BaseWithPreparedSet,
    MissingPreparedSet {
        mode: LaunchMode,
    },
    FlagMismatch {
        launch: LaunchMode,
        prepared: LaunchMode,
    },
}
impl fmt::Display for LaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion { got } => {
                write!(f, "unsupported prepared set schema version: {got}")
            }
            Self::BaseWithPreparedSet => {
                write!(f, "base launch must not carry a prepared set")
            }
            Self::MissingPreparedSet { mode } => {
                write!(f, "missing prepared set for launch mode {mode:?}")
            }
            Self::FlagMismatch { launch, prepared } => {
                write!(
                    f,
                    "flag mismatch: launch {launch:?} != prepared {prepared:?}"
                )
            }
        }
    }
}
impl std::error::Error for LaunchError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct PreparedSetRef {
    schema_version: u32,
    flags: ExtensionFlags,
    digest: Sha256Digest,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedSetRaw {
    schema_version: u32,
    flags: ExtensionFlags,
    digest: Sha256Digest,
}
impl<'de> Deserialize<'de> for PreparedSetRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = PreparedSetRaw::deserialize(deserializer)?;
        Self::new(raw.schema_version, raw.flags, raw.digest).map_err(serde::de::Error::custom)
    }
}
impl PreparedSetRef {
    pub fn new(
        schema_version: u32,
        flags: ExtensionFlags,
        digest: Sha256Digest,
    ) -> Result<Self, LaunchError> {
        if schema_version != LAUNCH_SET_SCHEMA_VERSION {
            return Err(LaunchError::UnsupportedSchemaVersion {
                got: schema_version,
            });
        }
        if flags.mode() == LaunchMode::Base {
            return Err(LaunchError::BaseWithPreparedSet);
        }
        Ok(Self {
            schema_version,
            flags,
            digest,
        })
    }
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }
    pub fn flags(&self) -> ExtensionFlags {
        self.flags
    }
    pub fn mode(&self) -> LaunchMode {
        self.flags.mode()
    }
    pub fn digest(&self) -> &Sha256Digest {
        &self.digest
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct ExtensionLaunch {
    flags: ExtensionFlags,
    prepared: Option<PreparedSetRef>,
}
impl Default for ExtensionLaunch {
    fn default() -> Self {
        Self {
            flags: ExtensionFlags::default(),
            prepared: None,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtensionLaunchRaw {
    #[serde(default)]
    flags: ExtensionFlags,
    #[serde(default)]
    prepared: Option<PreparedSetRef>,
}
impl<'de> Deserialize<'de> for ExtensionLaunch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = ExtensionLaunchRaw::deserialize(deserializer)?;
        Self::new(raw.flags, raw.prepared).map_err(serde::de::Error::custom)
    }
}
impl ExtensionLaunch {
    pub fn new(
        flags: ExtensionFlags,
        prepared: Option<PreparedSetRef>,
    ) -> Result<Self, LaunchError> {
        match (flags.mode(), prepared) {
            (LaunchMode::Base, None) => Ok(Self {
                flags,
                prepared: None,
            }),
            (LaunchMode::Base, Some(_)) => Err(LaunchError::BaseWithPreparedSet),
            (mode, None) => Err(LaunchError::MissingPreparedSet { mode }),
            (_, Some(set)) if set.flags() != flags => Err(LaunchError::FlagMismatch {
                launch: flags.mode(),
                prepared: set.mode(),
            }),
            (_, Some(set)) => Ok(Self {
                flags,
                prepared: Some(set),
            }),
        }
    }
    pub fn flags(&self) -> ExtensionFlags {
        self.flags
    }
    pub fn prepared(&self) -> Option<&PreparedSetRef> {
        self.prepared.as_ref()
    }
    pub fn mode(&self) -> LaunchMode {
        self.flags.mode()
    }
}
