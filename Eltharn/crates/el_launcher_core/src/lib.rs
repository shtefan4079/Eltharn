#![forbid(unsafe_code)]
mod accounts;
mod backend_target;
pub use backend_target::BackendTarget;
mod eltharn_page;
pub mod extension_json;
pub mod extensions;
mod flow;
mod launch_config;
mod prepared_assets;
pub mod session_accounts;
pub mod touch_controls;
pub use accounts::{
    DecodedAccounts, StoredAccountV1, add_offline_account_legacy, decode_legacy_accounts,
    delete_account, encode_legacy_accounts, legacy_offline_uuid, make_legacy_offline_account,
    migrate_legacy_names, normalize_active_account_index, select_account,
    upsert_account_by_name_legacy,
};
pub use el_protocol_api::shared::{
    AccountId, AccountIdError, AssetCompatibilityId, AssetManifestHash, AssetSelection,
    AssetStoreId, GameLaunchConfig, GameVersionId, LaunchId, ProtocolAbiVersion, ProtocolId,
    ProtocolIdError, ServerTarget, VersionCompatibilityBinding,
};
pub use eltharn_page::{
    ELTHARN_CLIENT_ASSETS_COMPATIBILITY, ELTHARN_DEFAULT_ACCOUNT_LABEL,
    ELTHARN_LAUNCH_BUTTON_LABEL, ELTHARN_PAGE_SUBTITLE, ELTHARN_PAGE_TITLE, EltharnBackendStatus,
    EltharnLaunchHero, EltharnLaunchPageInput, EltharnLaunchPageModel, EltharnModEntry,
    EltharnModListPanel, EltharnStatusTone, build_eltharn_launch_page,
};
pub use flow::{
    LauncherFailure, LauncherFailureKind, LauncherFlow, PreparationState, TransitionError,
};
pub use launch_config::{NavLayout, ordered_protocol_versions};
pub use prepared_assets::PreparedAssetManifest;
use serde::{Deserialize, Serialize};
pub use session_accounts::valid_name;
use std::fmt;
use std::path::Path;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchRequest {
    pub account_id: AccountId,
    pub protocol_id: ProtocolId,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaunchStage {
    Idle,
    ValidatingAccount,
    ResolvingProtocol,
    PreparingAssets,
    StartingGame,
    Complete,
    Failed,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchProgress {
    pub stage: LaunchStage,
    pub completed_steps: u32,
    pub total_steps: u32,
    pub detail: String,
}
impl LaunchProgress {
    pub fn new(stage: LaunchStage, completed_steps: u32, total_steps: u32, detail: String) -> Self {
        Self {
            stage,
            completed_steps: completed_steps.min(total_steps),
            total_steps,
            detail,
        }
    }
    pub fn fraction(&self) -> f32 {
        if self.total_steps == 0 {
            return 0.0;
        }
        self.completed_steps as f32 / self.total_steps as f32
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolBackendManifest {
    pub ipc_version: u32,
    pub backend_name: String,
    pub minecraft_version: String,
    pub protocol_version: i32,
    pub protocol_source: String,
    pub executable: String,
    pub assets_compatibility: Option<String>,
}
pub fn parse_protocol_backend_manifest(
    json: &str,
) -> Result<ProtocolBackendManifest, serde_json::Error> {
    serde_json::from_str(json)
}
pub fn validate_relative_executable(
    manifest: &ProtocolBackendManifest,
) -> Result<(), RelativeExecutableError> {
    let executable = &manifest.executable;
    if executable.contains("..")
        || executable.starts_with('/')
        || executable.contains('\\')
        || Path::new(executable).is_absolute()
    {
        return Err(RelativeExecutableError {
            path: executable.clone(),
        });
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelativeExecutableError {
    pub path: String,
}
impl fmt::Display for RelativeExecutableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsafe executable path: {}", self.path)
    }
}
impl std::error::Error for RelativeExecutableError {}
pub fn validate_protocol_backend_compatibility(
    manifest: &ProtocolBackendManifest,
    expected_minecraft_version: &str,
    expected_assets_version: &str,
    expected_ipc_version: u32,
) -> Result<(), ProtocolBackendCompatibilityError> {
    if manifest.minecraft_version != expected_minecraft_version {
        return Err(ProtocolBackendCompatibilityError::VersionMismatch {
            expected: expected_minecraft_version.to_string(),
            found: manifest.minecraft_version.clone(),
        });
    }
    if manifest.ipc_version != expected_ipc_version {
        return Err(ProtocolBackendCompatibilityError::IpcMismatch {
            expected: expected_ipc_version,
            found: manifest.ipc_version,
        });
    }
    if let Some(assets_compatibility) = &manifest.assets_compatibility
        && assets_compatibility != expected_assets_version
    {
        return Err(ProtocolBackendCompatibilityError::AssetsMismatch {
            expected: expected_assets_version.to_string(),
            found: assets_compatibility.clone(),
        });
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolBackendCompatibilityError {
    VersionMismatch { expected: String, found: String },
    IpcMismatch { expected: u32, found: u32 },
    AssetsMismatch { expected: String, found: String },
}
impl fmt::Display for ProtocolBackendCompatibilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VersionMismatch { expected, found } => {
                write!(
                    f,
                    "protocol version mismatch: expected {expected}, found {found}"
                )
            }
            Self::IpcMismatch { expected, found } => {
                write!(
                    f,
                    "IPC version mismatch: expected {expected}, found {found}"
                )
            }
            Self::AssetsMismatch { expected, found } => {
                write!(
                    f,
                    "assets compatibility mismatch: expected {expected}, found {found}"
                )
            }
        }
    }
}
impl std::error::Error for ProtocolBackendCompatibilityError {}
