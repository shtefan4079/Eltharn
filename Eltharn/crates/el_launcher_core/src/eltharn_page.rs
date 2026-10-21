use serde::{Deserialize, Serialize};
pub const ELTHARN_PAGE_TITLE: &str = "Eltharn";
pub const ELTHARN_PAGE_SUBTITLE: &str = "Select a protocol, then press Launch Eltharn";
pub const ELTHARN_LAUNCH_BUTTON_LABEL: &str = "Launch Eltharn";
pub const ELTHARN_DEFAULT_ACCOUNT_LABEL: &str = "Not selected";
pub const ELTHARN_CLIENT_ASSETS_COMPATIBILITY: &str = "26.2";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EltharnBackendStatus {
    Ready {
        backend_name: String,
        protocol_version: i32,
        protocol_source: String,
    },
    Missing,
    Broken {
        reason: String,
    },
    VersionMismatch {
        expected: String,
        found: String,
    },
    IpcMismatch {
        expected: u32,
        found: u32,
    },
    UnsafeExecutablePath {
        path: String,
    },
    AssetsMismatch {
        expected: String,
        found: String,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EltharnStatusTone {
    Ready,
    Error,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EltharnLaunchPageInput {
    pub protocol_options: Vec<String>,
    pub selected_protocol: String,
    pub active_account_name: Option<String>,
    pub backend_status: EltharnBackendStatus,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EltharnLaunchHero {
    pub title: String,
    pub tone: EltharnStatusTone,
    pub identity_line: String,
    pub detail: String,
    pub launch_enabled: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EltharnModEntry {
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub builtin: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EltharnModListPanel {
    pub title: String,
    pub subtitle: String,
    pub notice: String,
    pub mods: Vec<EltharnModEntry>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EltharnLaunchPageModel {
    pub title: String,
    pub subtitle: String,
    pub launch_button_label: String,
    pub protocol_options: Vec<String>,
    pub selected_protocol: String,
    pub hero: EltharnLaunchHero,
    pub mod_list: EltharnModListPanel,
}
pub fn build_eltharn_launch_page(input: EltharnLaunchPageInput) -> EltharnLaunchPageModel {
    let account_label = input
        .active_account_name
        .unwrap_or_else(|| ELTHARN_DEFAULT_ACCOUNT_LABEL.to_string());
    let selected_protocol = input.selected_protocol;
    let (title, tone, launch_enabled, detail) = match input.backend_status {
        EltharnBackendStatus::Ready {
            backend_name,
            protocol_version,
            ..
        } => (
            "Ready".to_string(),
            EltharnStatusTone::Ready,
            true,
            format!("{backend_name} · protocol id {protocol_version}"),
        ),
        EltharnBackendStatus::Missing => (
            "Not installed".to_string(),
            EltharnStatusTone::Error,
            false,
            "This protocol has no backend in the EltharnGate protocol folder.".to_string(),
        ),
        EltharnBackendStatus::Broken { reason } => (
            "Unavailable".to_string(),
            EltharnStatusTone::Error,
            false,
            reason,
        ),
        EltharnBackendStatus::VersionMismatch { expected, found } => (
            "Version mismatch".to_string(),
            EltharnStatusTone::Error,
            false,
            format!("Expected {expected}, found {found}"),
        ),
        EltharnBackendStatus::IpcMismatch { expected, found } => (
            "IPC mismatch".to_string(),
            EltharnStatusTone::Error,
            false,
            format!("Required IPC {expected}, manifest has {found}"),
        ),
        EltharnBackendStatus::UnsafeExecutablePath { path } => (
            "Invalid backend".to_string(),
            EltharnStatusTone::Error,
            false,
            path,
        ),
        EltharnBackendStatus::AssetsMismatch { expected, found } => (
            "Assets mismatch".to_string(),
            EltharnStatusTone::Error,
            false,
            format!("Required {expected}, manifest has {found}"),
        ),
    };
    EltharnLaunchPageModel {
        title: ELTHARN_PAGE_TITLE.to_string(),
        subtitle: ELTHARN_PAGE_SUBTITLE.to_string(),
        launch_button_label: ELTHARN_LAUNCH_BUTTON_LABEL.to_string(),
        protocol_options: input.protocol_options,
        selected_protocol: selected_protocol.clone(),
        hero: EltharnLaunchHero {
            title,
            tone,
            identity_line: format!(
                "Protocol: {selected_protocol}  ·  Account: {account_label}"
            ),
            detail,
            launch_enabled,
        },
        mod_list: EltharnModListPanel {
            title: "Mods & Add-ons".to_string(),
            subtitle: "Modding support is in development".to_string(),
            notice: "The modding system is currently under development. All built-in engine modules run natively."
                .to_string(),
            mods: Vec::new(),
        },
    }
}
