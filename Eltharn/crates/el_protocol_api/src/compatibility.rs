use crate::{
    AssetCompatibilityId, AssetSelection, GameLaunchConfig, ProtocolAbiVersion, ProtocolId,
    ProtocolImplementationId,
};
use serde::{Deserialize, Serialize};
use std::fmt;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtocolApiVersion(pub u32);
pub const ELTHARN_PROTOCOL_API_VERSION: ProtocolApiVersion = ProtocolApiVersion(1);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolDescriptor {
    pub implementation_id: ProtocolImplementationId,
    pub protocol_id: ProtocolId,
    pub api_version: ProtocolApiVersion,
    pub protocol_abi_version: ProtocolAbiVersion,
    pub minecraft_protocol_version: i32,
    pub asset_compatibility_id: AssetCompatibilityId,
}
impl ProtocolDescriptor {
    pub fn validate(&self) -> Result<(), ProtocolDescriptorError> {
        if self.api_version.0 == 0 {
            return Err(ProtocolDescriptorError::InvalidApiVersion);
        }
        if self.protocol_abi_version.0 == 0 {
            return Err(ProtocolDescriptorError::InvalidAbiVersion);
        }
        if self.minecraft_protocol_version <= 0 {
            return Err(ProtocolDescriptorError::InvalidMinecraftProtocolVersion {
                value: self.minecraft_protocol_version,
            });
        }
        if self.asset_compatibility_id.0.trim().is_empty() {
            return Err(ProtocolDescriptorError::EmptyAssetCompatibility);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolDescriptorError {
    InvalidApiVersion,
    InvalidAbiVersion,
    InvalidMinecraftProtocolVersion { value: i32 },
    EmptyAssetCompatibility,
}
impl fmt::Display for ProtocolDescriptorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidApiVersion => formatter.write_str("protocol API version must be non-zero"),
            Self::InvalidAbiVersion => formatter.write_str("protocol ABI version must be non-zero"),
            Self::InvalidMinecraftProtocolVersion { value } => {
                write!(formatter, "invalid Minecraft protocol version: {value}")
            }
            Self::EmptyAssetCompatibility => {
                formatter.write_str("asset compatibility identifier cannot be empty")
            }
        }
    }
}
impl std::error::Error for ProtocolDescriptorError {}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolPreflightRequest {
    pub protocol_id: ProtocolId,
    pub required_api_version: ProtocolApiVersion,
    pub protocol_abi_version: ProtocolAbiVersion,
    pub assets: AssetSelection,
}
impl ProtocolPreflightRequest {
    pub fn for_launch(
        protocol_id: ProtocolId,
        protocol_abi_version: ProtocolAbiVersion,
        assets: AssetSelection,
    ) -> Self {
        Self {
            protocol_id,
            required_api_version: ELTHARN_PROTOCOL_API_VERSION,
            protocol_abi_version,
            assets,
        }
    }
    pub fn from_game_launch_config(config: &GameLaunchConfig) -> Self {
        Self::for_launch(
            config.protocol_id.clone(),
            config.protocol_abi_version,
            config.assets.clone(),
        )
    }
}
impl From<&GameLaunchConfig> for ProtocolPreflightRequest {
    fn from(config: &GameLaunchConfig) -> Self {
        Self::from_game_launch_config(config)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolPreflightSuccess {
    descriptor: ProtocolDescriptor,
    request: ProtocolPreflightRequest,
}
impl ProtocolPreflightSuccess {
    pub fn descriptor(&self) -> &ProtocolDescriptor {
        &self.descriptor
    }
    pub fn request(&self) -> &ProtocolPreflightRequest {
        &self.request
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolPreflightResult {
    Compatible(ProtocolPreflightSuccess),
    Incompatible(ProtocolPreflightFailure),
}
impl ProtocolPreflightResult {
    pub fn into_result(self) -> Result<ProtocolPreflightSuccess, ProtocolPreflightFailure> {
        match self {
            Self::Compatible(success) => Ok(success),
            Self::Incompatible(error) => Err(error),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolPreflightFailure {
    InvalidDescriptor(ProtocolDescriptorError),
    ProtocolIdMismatch {
        requested: ProtocolId,
        provided: ProtocolId,
    },
    ApiVersionMismatch {
        required: ProtocolApiVersion,
        provided: ProtocolApiVersion,
    },
    AbiVersionMismatch {
        required: ProtocolAbiVersion,
        provided: ProtocolAbiVersion,
    },
    AssetCompatibilityMismatch {
        required: AssetCompatibilityId,
        provided: AssetCompatibilityId,
    },
}
impl fmt::Display for ProtocolPreflightFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDescriptor(error) => {
                write!(formatter, "invalid protocol descriptor: {error}")
            }
            Self::ProtocolIdMismatch {
                requested,
                provided,
            } => {
                write!(
                    formatter,
                    "protocol selection mismatch: requested {requested}, implementation provides {provided}"
                )
            }
            Self::ApiVersionMismatch { required, provided } => {
                write!(
                    formatter,
                    "protocol API mismatch: required {}, implementation provides {}",
                    required.0, provided.0
                )
            }
            Self::AbiVersionMismatch { required, provided } => {
                write!(
                    formatter,
                    "protocol ABI mismatch: required {}, implementation provides {}",
                    required.0, provided.0
                )
            }
            Self::AssetCompatibilityMismatch { required, provided } => {
                write!(
                    formatter,
                    "asset compatibility mismatch: required {}, implementation provides {}",
                    required.0, provided.0
                )
            }
        }
    }
}
impl std::error::Error for ProtocolPreflightFailure {}
pub fn preflight(
    descriptor: &ProtocolDescriptor,
    request: ProtocolPreflightRequest,
) -> ProtocolPreflightResult {
    if let Err(error) = descriptor.validate() {
        return ProtocolPreflightResult::Incompatible(ProtocolPreflightFailure::InvalidDescriptor(
            error,
        ));
    }
    if descriptor.protocol_id != request.protocol_id {
        return ProtocolPreflightResult::Incompatible(
            ProtocolPreflightFailure::ProtocolIdMismatch {
                requested: request.protocol_id,
                provided: descriptor.protocol_id.clone(),
            },
        );
    }
    if descriptor.api_version != request.required_api_version {
        return ProtocolPreflightResult::Incompatible(
            ProtocolPreflightFailure::ApiVersionMismatch {
                required: request.required_api_version,
                provided: descriptor.api_version,
            },
        );
    }
    if descriptor.protocol_abi_version != request.protocol_abi_version {
        return ProtocolPreflightResult::Incompatible(
            ProtocolPreflightFailure::AbiVersionMismatch {
                required: request.protocol_abi_version,
                provided: descriptor.protocol_abi_version,
            },
        );
    }
    if descriptor.asset_compatibility_id != request.assets.compatibility_id {
        return ProtocolPreflightResult::Incompatible(
            ProtocolPreflightFailure::AssetCompatibilityMismatch {
                required: request.assets.compatibility_id,
                provided: descriptor.asset_compatibility_id.clone(),
            },
        );
    }
    ProtocolPreflightResult::Compatible(ProtocolPreflightSuccess {
        descriptor: descriptor.clone(),
        request,
    })
}
