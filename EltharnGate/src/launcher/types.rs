use el_launcher_core::touch_controls::TouchControlProfile;
pub use el_launcher_core::{NavLayout, StoredAccountV1 as Account};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Page {
    Eltharn,
    Settings,
}
#[derive(Debug, Clone)]
pub enum AssetDownloadState {
    Idle,
    Downloading {
        completed: usize,
        total: usize,
        status_text: String,
    },
    Completed,
    Failed(String),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub nav_layout: NavLayout,
    pub selected_protocol: String,
    #[serde(alias = "custom_rustore_executable")]
    pub custom_eltharn_executable: Option<String>,
    pub active_account_index: usize,
    pub corner_radius: f32,
    #[serde(default, deserialize_with = "deserialize_touch_controls")]
    pub touch_controls: TouchControlProfile,
}
fn deserialize_touch_controls<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<TouchControlProfile, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    match serde_json::from_value::<TouchControlProfile>(value) {
        Ok(profile) if profile.validate().is_ok() => Ok(profile),
        _ => {
            eprintln!("Invalid saved touch profile; restoring the default layout");
            Ok(TouchControlProfile::default())
        }
    }
}
impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            nav_layout: NavLayout::TopBar,
            selected_protocol: crate::bundled_runtime()
                .map(|runtime| runtime.manifest.minecraft_version.clone())
                .unwrap_or_default(),
            custom_eltharn_executable: None,
            active_account_index: 0,
            corner_radius: 12.0,
            touch_controls: TouchControlProfile::default(),
        }
    }
}
#[derive(Debug, Clone)]
pub enum Message {
    PageChanged(Page),
    NavLayoutChanged(NavLayout),
    MinimizeWindow,
    MaximizeWindow,
    CloseWindow,
    TitleBarPressed,
    ResizeWindow(iced::window::Direction),
    Tick(f32),
    UiCaptured(iced::window::Screenshot),
    WindowEvent(iced::window::Id, iced::window::Event),
    WindowMaximizedStatusReceived(bool),
    BlurApplied,
    EltharnProtocolVersionSelected(String),
    LaunchEltharn,
    LaunchAfterFolderSync(bool),
    Extensions(crate::launcher::extensions::ExtMessage),
    EltharnPrepared {
        ticket: crate::launcher::launch_guard::Ticket,
        result: Result<crate::launcher::executor::LaunchParams, String>,
    },
    EltharnProtocolInspected(Result<crate::launcher::executor::ClientProtocolInfo, String>),
    EltharnExited(Result<i32, String>),
    StartAssetDownload(String),
    AssetDownloadProgressUpdate {
        completed: usize,
        total: usize,
        status: String,
    },
    AssetDownloadFinished(Result<String, String>),
    OpenAddAccountModal,
    CloseAddAccountModal,
    NewAccountUsernameChanged(String),
    SubmitAddOfflineAccount,
    SelectAccount(usize),
    OpenDeleteAccountConfirmation(usize),
    ConfirmDeleteAccount,
    CancelDeleteAccount,
    PlayerSkinLoaded {
        username: String,
        skin_bytes: Result<Vec<u8>, String>,
    },
    OpenDataFolder,
    OpenProtocolsFolder,
    OpenLogsFolder,
    InstallToSystem,
    DismissToast,
}
