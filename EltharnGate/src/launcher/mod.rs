mod account_store;
pub mod downloader;
pub mod executor;
pub mod extensions;
pub mod launch_guard;
pub mod paths;
mod touch_settings;
pub mod types;
pub(crate) mod ui_capture;
pub mod utils;
pub mod views;
use crate::launcher::paths::EltharnGatePaths;
use crate::launcher::types::{Account, AssetDownloadState, LauncherConfig, Message, Page};
use el_launcher_core::{EltharnBackendStatus, decode_legacy_accounts};
use iced::{Element, Subscription, Task};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
pub struct EltharnGateApp {
    ui_capture: Option<ui_capture::Capture>,
    launch_after_folder_sync: Option<bool>,
    pub paths: EltharnGatePaths,
    pub config: LauncherConfig,
    pub extensions: extensions::ExtPanel,
    pub accounts: Vec<Account>,
    pub active_page: Page,
    pub status_message: String,
    pub anim_time: f32,
    pub toast: Option<(String, u8, f32)>,
    pub show_add_account_modal: bool,
    pub new_account_username: String,
    pub account_to_delete: Option<usize>,
    pub asset_download_state: AssetDownloadState,
    pub active_process: Option<std::process::Child>,
    pub active_extension_request: Option<PathBuf>,
    pub player_skins: std::collections::HashMap<String, iced::widget::image::Handle>,
    pub window_width: i32,
    pub window_height: i32,
    pub corner_radius: f32,
    pub target_corner_radius: f32,
    pub applied_corner_radius: i32,
    pub pending_minimize: bool,
    pub preparing_eltharn_launch: bool,
    launch_guard: launch_guard::Guard,
    pub client_protocol_info: Option<Result<executor::ClientProtocolInfo, String>>,
}
impl EltharnGateApp {
    pub fn new() -> (Self, Task<Message>) {
        let ui_capture = ui_capture::Capture::from_env().unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(2);
        });
        let capturing = ui_capture.is_some();
        let paths = EltharnGatePaths::discover();
        let mut config: LauncherConfig = if paths.config_file().is_file() {
            fs::read_to_string(paths.config_file())
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            LauncherConfig::default()
        };
        if config.selected_protocol == "26.2" && paths.find_protocol_backend("26.2").is_none() {
            if let Some(runtime) = crate::bundled_runtime() {
                config.selected_protocol = runtime.manifest.minecraft_version.clone();
            }
        }
        if let Err(error) = config.touch_controls.validate() {
            eprintln!("Invalid saved touch profile: {error}; restoring the default layout");
            config.touch_controls = Default::default();
        }
        let accounts = if !capturing && paths.accounts_file().is_file() {
            fs::read_to_string(paths.accounts_file())
                .ok()
                .and_then(|s| decode_legacy_accounts(&s))
                .map(|decoded| match decoded {
                    el_launcher_core::DecodedAccounts::Current(accs)
                    | el_launcher_core::DecodedAccounts::MigratedLegacyNames(accs) => accs,
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let blur_task = if capturing {
            Task::none()
        } else {
            iced::window::latest().and_then(|id| apply_blur_task(id, 960, 620, 12))
        };
        let mut initial_tasks = vec![blur_task];
        for account in &accounts {
            initial_tasks.push(fetch_skin_task(paths.clone(), account.name.clone()));
        }
        if let Some(executable) = paths.find_eltharn_executable().filter(|_| !capturing) {
            let inspection_paths = paths.clone();
            let inspection_protocol = config.selected_protocol.clone();
            initial_tasks.push(Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        executor::inspect_pinned_client_protocol(
                            &inspection_paths,
                            &inspection_protocol,
                            &executable,
                        )
                    })
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|result| result)
                },
                Message::EltharnProtocolInspected,
            ));
        }
        let mut app = Self {
            ui_capture,
            launch_after_folder_sync: None,
            extensions: extensions::ExtPanel::new(paths.root.join("extensions")),
            paths,
            config,
            accounts,
            active_page: if capturing
                && std::env::var("ELTHARN_GATE_UI_CAPTURE_PAGE").as_deref() == Ok("settings")
            {
                Page::Settings
            } else {
                Page::Eltharn
            },
            status_message: "Ready".to_string(),
            anim_time: 0.0,
            toast: None,
            show_add_account_modal: false,
            new_account_username: String::new(),
            account_to_delete: None,
            asset_download_state: AssetDownloadState::Idle,
            active_process: None,
            active_extension_request: None,
            player_skins: std::collections::HashMap::new(),
            window_width: 960,
            window_height: 620,
            corner_radius: 12.0,
            target_corner_radius: 12.0,
            applied_corner_radius: 12,
            pending_minimize: false,
            preparing_eltharn_launch: false,
            launch_guard: launch_guard::Guard::default(),
            client_protocol_info: None,
        };
        if capturing {
            let tab = if std::env::var("ELTHARN_GATE_UI_CAPTURE_TAB").as_deref() == Ok("shaders") {
                extensions::CatalogTab::ShaderPacks
            } else {
                extensions::CatalogTab::Mods
            };
            let _ = app.extensions.update(extensions::ExtMessage::Tab(tab));
        }
        initial_tasks.push(app.extensions.sync_folders().map(Message::Extensions));
        (app, Task::batch(initial_tasks))
    }
    pub fn save_config(&mut self) {
        if let Err(error) =
            touch_settings::save_launcher_config(&self.paths.config_file(), &self.config)
        {
            self.push_toast(format!("Could not save settings: {error}"), 2);
        }
    }
    pub fn save_accounts(&mut self) -> bool {
        match account_store::save(&self.paths.accounts_file(), &self.accounts) {
            Ok(()) => true,
            Err(error) => {
                self.status_message = format!("Could not save profiles: {error}");
                self.push_toast(self.status_message.clone(), 2);
                false
            }
        }
    }
    pub fn active_account(&self) -> Option<&Account> {
        if self.accounts.is_empty() {
            None
        } else {
            self.accounts
                .get(self.config.active_account_index)
                .or_else(|| self.accounts.first())
        }
    }
    pub fn active_player(&self) -> Option<&str> {
        self.active_account().map(|a| a.name.as_str())
    }
    pub fn protocol_versions(&self) -> Vec<String> {
        let mut versions = crate::bundled_runtime()
            .map(|runtime| vec![runtime.manifest.minecraft_version.clone()])
            .unwrap_or_default();
        for directory in [self.paths.protocols_dir(), self.paths.backends_dir()] {
            if let Ok(entries) = fs::read_dir(directory) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            let name = entry.file_name().to_string_lossy().into_owned();
                            if el_launcher_core::ProtocolId::new(name.clone()).is_ok()
                                && entry.path().join("manifest.json").is_file()
                                && !versions.contains(&name)
                            {
                                versions.push(name);
                            }
                        }
                    }
                }
            }
        }
        versions
    }
    pub fn backend_status(&self, protocol: &str) -> EltharnBackendStatus {
        let exe_exists = self.paths.find_eltharn_executable().is_some();
        if !exe_exists {
            return EltharnBackendStatus::Broken {
                reason:
                    "Eltharn client executable not found. Place it in bin/ or build the project."
                        .to_string(),
            };
        }
        let client = match &self.client_protocol_info {
            Some(Ok(info)) => info,
            Some(Err(reason)) => {
                return EltharnBackendStatus::Broken {
                    reason: reason.clone(),
                };
            }
            None => {
                return EltharnBackendStatus::Broken {
                    reason: "Inspecting installed Eltharn executable…".into(),
                };
            }
        };
        let Some(backend_path) = self.paths.find_protocol_backend(protocol) else {
            return EltharnBackendStatus::Missing;
        };
        let manifest = match executor::validated_backend_manifest(
            &backend_path.to_string_lossy(),
            protocol,
            client,
        ) {
            Ok(manifest) => manifest,
            Err(reason) => {
                return EltharnBackendStatus::Broken { reason };
            }
        };
        EltharnBackendStatus::Ready {
            backend_name: manifest.backend_name,
            protocol_version: manifest.protocol_version,
            protocol_source: manifest.protocol_source,
        }
    }
    pub fn push_toast(&mut self, message: impl Into<String>, kind: u8) {
        self.toast = Some((message.into(), kind, self.anim_time + 3500.0));
    }
    fn update_window_size(&mut self, size: iced::Size) {
        self.window_width = size.width.round().max(1.0) as i32;
        self.window_height = size.height.round().max(1.0) as i32;
    }
    pub fn update(&mut self, message: Message) -> Task<Message> {
        if self.ui_capture.is_some()
            && !matches!(
                &message,
                Message::Tick(_)
                    | Message::UiCaptured(_)
                    | Message::WindowEvent(_, _)
                    | Message::Extensions(extensions::ExtMessage::FoldersSynced(_))
            )
        {
            return Task::none();
        }
        match message {
            Message::EltharnProtocolInspected(result) => {
                self.client_protocol_info = Some(result);
                Task::none()
            }
            Message::PageChanged(page) => {
                self.active_page = page;
                Task::none()
            }
            Message::NavLayoutChanged(layout) => {
                self.config.nav_layout = layout;
                self.save_config();
                Task::none()
            }
            Message::MinimizeWindow => {
                if self.target_corner_radius == 0.0 {
                    self.pending_minimize = true;
                    iced::window::latest().and_then(|id| iced::window::maximize(id, false))
                } else {
                    iced::window::latest().and_then(|id| iced::window::minimize(id, true))
                }
            }
            Message::MaximizeWindow => {
                let is_maximized = self.target_corner_radius == 0.0;
                iced::window::latest().and_then(move |id| iced::window::maximize(id, !is_maximized))
            }
            Message::CloseWindow => iced::window::latest().and_then(iced::window::close),
            Message::TitleBarPressed => iced::window::latest().and_then(iced::window::drag),
            Message::ResizeWindow(direction) => {
                iced::window::latest().and_then(move |id| iced::window::drag_resize(id, direction))
            }
            Message::Tick(delta) => {
                if !self.extensions.busy
                    && self
                        .ui_capture
                        .as_mut()
                        .is_some_and(ui_capture::Capture::ready)
                {
                    return iced::window::latest()
                        .and_then(iced::window::screenshot)
                        .map(Message::UiCaptured);
                }
                if let Some(child) = self.active_process.as_mut() {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            return self
                                .update(Message::EltharnExited(Ok(status.code().unwrap_or(-1))));
                        }
                        Err(error) => {
                            return self.update(Message::EltharnExited(Err(error.to_string())));
                        }
                        Ok(None) => {}
                    }
                }
                self.anim_time += delta;
                let diff = self.target_corner_radius - self.corner_radius;
                if diff.abs() > 0.001 {
                    self.corner_radius += diff * 0.15;
                } else {
                    self.corner_radius = self.target_corner_radius;
                }
                let corner_radius = self.corner_radius.round() as i32;
                let corner_task = if self.applied_corner_radius != corner_radius {
                    self.applied_corner_radius = corner_radius;
                    let width = self.window_width;
                    let height = self.window_height;
                    iced::window::latest()
                        .and_then(move |id| apply_blur_task(id, width, height, corner_radius))
                } else {
                    Task::none()
                };
                if let Some((_, _, expiry)) = self.toast {
                    if self.anim_time >= expiry {
                        self.toast = None;
                    }
                }
                if self.pending_minimize {
                    self.pending_minimize = false;
                    return iced::window::latest().and_then(|id| iced::window::minimize(id, true));
                }
                if !self.preparing_eltharn_launch {
                    Task::batch([
                        corner_task,
                        self.extensions.observe_folders().map(Message::Extensions),
                    ])
                } else {
                    corner_task
                }
            }
            Message::UiCaptured(screenshot) => {
                if let Some(capture) = &self.ui_capture {
                    if let Err(error) = capture.save(&screenshot) {
                        eprintln!("UI capture failed: {error}");
                        std::process::exit(1);
                    }
                    return iced::exit();
                }
                Task::none()
            }
            Message::WindowEvent(id, event) => match event {
                iced::window::Event::Opened { size, .. } => {
                    self.update_window_size(size);
                    apply_blur_task(
                        id,
                        self.window_width,
                        self.window_height,
                        self.corner_radius.round() as i32,
                    )
                }
                iced::window::Event::Resized(size) => {
                    self.update_window_size(size);
                    #[cfg(target_os = "linux")]
                    {
                        Task::batch(vec![
                            apply_blur_task(
                                id,
                                self.window_width,
                                self.window_height,
                                self.corner_radius.round() as i32,
                            ),
                            iced::window::is_maximized(id)
                                .map(Message::WindowMaximizedStatusReceived),
                        ])
                    }
                    #[cfg(target_os = "windows")]
                    {
                        iced::window::is_maximized(id).map(Message::WindowMaximizedStatusReceived)
                    }
                    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
                    {
                        Task::none()
                    }
                }
                _ => Task::none(),
            },
            Message::WindowMaximizedStatusReceived(maximized) => {
                if maximized {
                    self.target_corner_radius = 0.0;
                } else {
                    self.target_corner_radius = 12.0;
                }
                Task::none()
            }
            Message::BlurApplied => Task::none(),
            Message::EltharnProtocolVersionSelected(version) => {
                if self.config.selected_protocol != version {
                    self.launch_guard.invalidate();
                }
                self.config.selected_protocol = version;
                self.save_config();
                Task::none()
            }
            Message::Extensions(message) => {
                let folders_finished = matches!(&message, extensions::ExtMessage::FoldersSynced(_));
                let before = self.extensions.profile(false).ok();
                let task = self.extensions.update(message).map(Message::Extensions);
                if self.extensions.profile(false).ok() != before {
                    self.launch_guard.invalidate();
                }
                if folders_finished {
                    if let Some(recovery) = self.launch_after_folder_sync.take() {
                        if let Some(error) = &self.extensions.error {
                            self.push_toast(error.clone(), 2);
                        } else {
                            return Task::batch([
                                task,
                                Task::done(Message::LaunchAfterFolderSync(recovery)),
                            ]);
                        }
                    }
                }
                task
            }
            Message::LaunchEltharn | Message::LaunchAfterFolderSync(_) => {
                if self.active_process.is_some() {
                    self.push_toast("Close the running game before launching again.", 2);
                    return Task::none();
                }
                if self.preparing_eltharn_launch {
                    return Task::none();
                }
                let recovery = matches!(message, Message::LaunchAfterFolderSync(true));
                if !matches!(message, Message::LaunchAfterFolderSync(_))
                    && self.extensions.prefs.is_some()
                {
                    self.launch_after_folder_sync = Some(recovery);
                    return self.extensions.sync_folders().map(Message::Extensions);
                }
                let extension_profile = match self.extensions.profile(recovery) {
                    Ok(p) => p,
                    Err(error) => {
                        self.push_toast(error, 2);
                        return Task::none();
                    }
                };
                let Some(account) = self.active_account() else {
                    self.show_add_account_modal = true;
                    self.status_message = "No profile selected".to_string();
                    self.push_toast("Please create or select a player profile first.", 0);
                    return Task::none();
                };
                let Some(executable) = self.paths.find_eltharn_executable() else {
                    self.status_message = "Eltharn executable not found!".to_string();
                    self.push_toast(
                        "Eltharn binary not found. Place it in bin/ or build the client.",
                        2,
                    );
                    return Task::none();
                };
                let player = account.name.clone();
                let uuid = account.uuid.clone();
                let access_token = account.access_token.clone();
                if let Err(error) =
                    touch_settings::save_profile(&self.paths.root, &self.config.touch_controls)
                {
                    self.push_toast(
                        format!("Could not export Android touch controls: {error}"),
                        2,
                    );
                }
                let protocol_backend = self
                    .paths
                    .find_protocol_backend(&self.config.selected_protocol)
                    .map(|p| p.to_string_lossy().to_string());
                let params = executor::LaunchParams {
                    executable_path: executable,
                    player_name: player.clone(),
                    uuid,
                    access_token,
                    protocol_version: self.config.selected_protocol.clone(),
                    protocol_backend,
                    extension_request: None,
                };
                let Some(ticket) = self.launch_guard.begin() else {
                    return Task::none();
                };
                self.preparing_eltharn_launch = true;
                self.status_message = "Preparing game resources...".into();
                let paths = self.paths.clone();
                let catalog = self.extensions.catalog.clone();
                Task::perform(
                    async move {
                        downloader::download_version_assets(
                            &paths,
                            executor::ELTHARN_ASSETS_COMPATIBILITY_VERSION,
                            None,
                        )
                        .await?;
                        let mut params = params;
                        if extension_profile.flags.mods_enabled
                            || extension_profile.flags.shaders_enabled
                        {
                            let executable = params.executable_path.clone();
                            params.extension_request = Some(
                                tokio::task::spawn_blocking(move || {
                                    executor::verify_extension_support(&executable)?;
                                    let store =
                                        el_extension_manager::native_store::NativeStore::new(
                                            paths.root.join("extensions"),
                                        );
                                    let archives = store.archives(&extension_profile)?;
                                    let bytes = pollster::block_on(
                                        el_extension_manager::launch::prepare_with_catalog(
                                            extension_profile,
                                            &archives,
                                            &catalog,
                                        ),
                                    )?;
                                    store.save_launch(&bytes)
                                })
                                .await
                                .map_err(|e| e.to_string())??,
                            );
                        }
                        Ok(params)
                    },
                    move |result| Message::EltharnPrepared {
                        ticket: ticket.clone(),
                        result,
                    },
                )
            }
            Message::EltharnPrepared { ticket, result } => {
                match self.launch_guard.complete(&ticket) {
                    launch_guard::Completion::Obsolete => {
                        launch_guard::discard(&result, self.active_extension_request.as_deref());
                        return Task::none();
                    }
                    launch_guard::Completion::Cancelled => {
                        launch_guard::discard(&result, self.active_extension_request.as_deref());
                        self.preparing_eltharn_launch = false;
                        self.status_message = "Selection changed; start a new launch.".into();
                        return Task::none();
                    }
                    launch_guard::Completion::Accepted => {}
                }
                self.preparing_eltharn_launch = false;
                let params = match result {
                    Ok(params) => params,
                    Err(error) => {
                        self.status_message = format!("Resource preparation failed: {error}");
                        self.push_toast(self.status_message.clone(), 2);
                        return Task::none();
                    }
                };
                let player = params.player_name.clone();
                match executor::launch_eltharn_process(&self.paths, &params) {
                    Ok(child) => {
                        self.active_extension_request = params.extension_request.clone();
                        self.active_process = Some(child);
                        self.status_message = format!("Running as {player}...");
                        self.push_toast(format!("Launched Eltharn for {player}!"), 1);
                    }
                    Err(e) => {
                        if let Some(request) = &params.extension_request {
                            let _ = fs::remove_file(request);
                        }
                        self.status_message = format!("Launch failed: {e}");
                        self.push_toast(format!("Failed to launch: {e}"), 2);
                    }
                }
                Task::none()
            }
            Message::EltharnExited(status) => {
                self.active_process = None;
                match status {
                    Ok(code) => {
                        self.status_message = format!("Eltharn exited with code {code}");
                    }
                    Err(e) => self.status_message = format!("Eltharn process error: {e}"),
                }
                if let Some(request) = self.active_extension_request.take() {
                    if let Some(error) = executor::extension_startup_error(&request) {
                        self.status_message = format!(
                            "Extensions could not start: {error}. Your profile is unchanged."
                        );
                        self.push_toast(self.status_message.clone(), 2);
                    }
                    let _ = fs::remove_file(request.with_extension("status.json"));
                    let _ = fs::remove_file(request);
                }
                Task::none()
            }
            Message::StartAssetDownload(version) => {
                self.asset_download_state = AssetDownloadState::Downloading {
                    completed: 0,
                    total: 100,
                    status_text: format!("Initializing download for {version}..."),
                };
                let paths_clone = self.paths.clone();
                Task::run(
                    iced::stream::channel(
                        16,
                        move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                            use iced::futures::SinkExt;
                            let (sender, mut progress) = tokio::sync::mpsc::unbounded_channel();
                            let download = downloader::download_version_assets(
                                &paths_clone,
                                &version,
                                Some(sender),
                            );
                            tokio::pin!(download);
                            loop {
                                tokio::select! {
                                    Some((completed, total, status)) = progress.recv() => { if
                                    output.send(Message::AssetDownloadProgressUpdate {
                                    completed, total, status }). await .is_err() { break; } }
                                    result = & mut download => { let _ = output
                                    .send(Message::AssetDownloadFinished(result.map(| _ |
                                    version.clone()))). await; break; }
                                }
                            }
                        },
                    ),
                    std::convert::identity,
                )
            }
            Message::AssetDownloadProgressUpdate {
                completed,
                total,
                status,
            } => {
                self.asset_download_state = AssetDownloadState::Downloading {
                    completed,
                    total,
                    status_text: status,
                };
                Task::none()
            }
            Message::AssetDownloadFinished(result) => {
                match result {
                    Ok(version) => {
                        self.asset_download_state = AssetDownloadState::Completed;
                        self.push_toast(format!("Assets for {version} ready!"), 1);
                    }
                    Err(e) => {
                        self.asset_download_state = AssetDownloadState::Failed(e.clone());
                        self.push_toast(format!("Asset download failed: {e}"), 2);
                    }
                }
                Task::none()
            }
            Message::OpenAddAccountModal => {
                self.show_add_account_modal = true;
                self.new_account_username = String::new();
                Task::none()
            }
            Message::CloseAddAccountModal => {
                self.show_add_account_modal = false;
                Task::none()
            }
            Message::NewAccountUsernameChanged(name) => {
                self.new_account_username = name;
                Task::none()
            }
            Message::SubmitAddOfflineAccount => {
                let name = self.new_account_username.trim().to_string();
                if !name.is_empty() && !el_launcher_core::valid_name(&name) {
                    self.status_message = "Invalid player name".to_string();
                    self.push_toast(
                        "Player names must be 1-16 characters with no control characters.",
                        2,
                    );
                    return Task::none();
                }
                let mut task = Task::none();
                if !name.is_empty() {
                    self.launch_guard.invalidate();
                    self.accounts
                        .push(el_launcher_core::make_legacy_offline_account(name.clone()));
                    self.config.active_account_index = self.accounts.len() - 1;
                    if !self.save_accounts() {
                        return Task::none();
                    }
                    self.save_config();
                    self.push_toast(format!("Added profile '{name}'"), 1);
                    task = fetch_skin_task(self.paths.clone(), name);
                }
                self.show_add_account_modal = false;
                task
            }
            Message::PlayerSkinLoaded {
                username,
                skin_bytes,
            } => {
                if let Ok(bytes) = skin_bytes {
                    let handle = iced::widget::image::Handle::from_bytes(bytes);
                    self.player_skins.insert(username, handle);
                }
                Task::none()
            }
            Message::SelectAccount(index) => {
                if index < self.accounts.len() {
                    if self.config.active_account_index != index {
                        self.launch_guard.invalidate();
                    }
                    self.config.active_account_index = index;
                    self.save_config();
                    let name = &self.accounts[index].name;
                    self.push_toast(format!("Switched to {name}"), 1);
                }
                Task::none()
            }
            Message::OpenDeleteAccountConfirmation(index) => {
                self.account_to_delete = Some(index);
                Task::none()
            }
            Message::ConfirmDeleteAccount => {
                if let Some(index) = self.account_to_delete.take() {
                    if index < self.accounts.len() {
                        self.launch_guard.invalidate();
                        let deleted = self.accounts.remove(index);
                        if self.accounts.is_empty() {
                            self.config.active_account_index = 0;
                        } else if self.config.active_account_index >= self.accounts.len() {
                            self.config.active_account_index = self.accounts.len() - 1;
                        }
                        if !self.save_accounts() {
                            return Task::none();
                        }
                        self.save_config();
                        self.push_toast(format!("Removed profile '{}'", deleted.name), 1);
                    }
                }
                Task::none()
            }
            Message::CancelDeleteAccount => {
                self.account_to_delete = None;
                Task::none()
            }
            Message::OpenDataFolder => {
                let _ = open_folder(&self.paths.root);
                Task::none()
            }
            Message::OpenProtocolsFolder => {
                let _ = open_folder(&self.paths.protocols_dir());
                Task::none()
            }
            Message::OpenLogsFolder => {
                let _ = open_folder(&self.paths.logs_dir());
                Task::none()
            }
            Message::InstallToSystem => {
                install_system_shortcut(&self.paths);
                self.push_toast("Installed EltharnGate shortcut", 1);
                Task::none()
            }
            Message::DismissToast => {
                self.toast = None;
                Task::none()
            }
        }
    }
    pub fn subscription(&self) -> Subscription<Message> {
        let subs = vec![
            iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick(16.0)),
            iced::window::events().map(|(id, event)| Message::WindowEvent(id, event)),
        ];
        Subscription::batch(subs)
    }
    pub fn view(&self) -> Element<'_, Message> {
        views::view_main(self)
    }
}
#[allow(unused_variables)]
fn apply_blur_task(id: iced::window::Id, width: i32, height: i32, radius: i32) -> Task<Message> {
    if std::env::var_os("ELTHARN_GATE_UI_CAPTURE").is_some() {
        return Task::none();
    }
    iced::window::run(id, move |window| {
        #[cfg(target_os = "linux")]
        {
            use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
            if let (Ok(dh), Ok(wh)) = (window.display_handle(), window.window_handle()) {
                if let (RawDisplayHandle::Wayland(dh_w), RawWindowHandle::Wayland(wh_w)) =
                    (dh.as_raw(), wh.as_raw())
                {
                    unsafe {
                        crate::blur::apply_kde_wayland_blur(
                            dh_w.display.as_ptr() as *mut _,
                            wh_w.surface.as_ptr() as *mut _,
                            width,
                            height,
                            radius,
                        );
                    }
                }
            }
        }
        #[cfg(target_os = "windows")]
        {
            use raw_window_handle::RawWindowHandle;
            if let Ok(wh) = window.window_handle() {
                if let RawWindowHandle::Win32(wh_w) = wh.as_raw() {
                    unsafe {
                        apply_windows_blur(wh_w.hwnd.get() as *mut _);
                    }
                }
            }
        }
        Message::BlurApplied
    })
}
#[cfg(target_os = "windows")]
unsafe fn apply_windows_blur(hwnd: *mut std::ffi::c_void) {
    use std::ffi::c_void;
    #[repr(C)]
    struct AccentPolicy {
        accent_state: u32,
        accent_flags: u32,
        gradient_color: u32,
        animation_id: u32,
    }
    #[repr(C)]
    struct WindowCompositionAttributeData {
        attribute: u32,
        data: *mut c_void,
        size_of_data: usize,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetWindowCompositionAttribute(
            hwnd: *mut c_void,
            data: *mut WindowCompositionAttributeData,
        ) -> i32;
    }
    let mut policy = AccentPolicy {
        accent_state: 3,
        accent_flags: 2,
        gradient_color: 0x01000000,
        animation_id: 0,
    };
    let mut data = WindowCompositionAttributeData {
        attribute: 19,
        data: &mut policy as *mut _ as *mut c_void,
        size_of_data: std::mem::size_of::<AccentPolicy>(),
    };
    unsafe {
        SetWindowCompositionAttribute(hwnd, &mut data);
    }
}
fn open_folder(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn install_system_shortcut(_paths: &EltharnGatePaths) {
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = dirs::home_dir() {
            let bin_dir = home.join(".local/bin");
            let _ = fs::create_dir_all(&bin_dir);
            let target_bin = bin_dir.join("eltharn_gate");
            if let Ok(current_exe) = std::env::current_exe() {
                let _ = fs::copy(&current_exe, &target_bin);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&target_bin, fs::Permissions::from_mode(0o755));
                }
            }
            let icon_svg_dir = home.join(".local/share/icons/hicolor/scalable/apps");
            let _ = fs::create_dir_all(&icon_svg_dir);
            let _ = fs::write(
                icon_svg_dir.join("eltharn_gate.svg"),
                crate::icons::SVG_LOGO,
            );
            const ICON_256_PNG: &[u8] = include_bytes!("../icon_256.png");
            let icon_png_dir = home.join(".local/share/icons/hicolor/256x256/apps");
            let _ = fs::create_dir_all(&icon_png_dir);
            let _ = fs::write(icon_png_dir.join("eltharn_gate.png"), ICON_256_PNG);
            let app_dir = home.join(".local/share/applications");
            let _ = fs::create_dir_all(&app_dir);
            let exec = crate::launcher::utils::desktop_exec_quote(&target_bin.to_string_lossy());
            let desktop_entry = format!(
                "[Desktop Entry]\n\
                 Name=EltharnGate\n\
                 Comment=Eltharn Modern Game Launcher\n\
                 Exec={exec}\n\
                 Icon=eltharn_gate\n\
                 Terminal=false\n\
                 Type=Application\n\
                 Categories=Game;Application;\n\
                 StartupWMClass=eltharn_gate\n\
                 StartupNotify=true\n\
                 Keywords=minecraft;eltharn;launcher;\n"
            );
            let _ = fs::write(app_dir.join("eltharn_gate.desktop"), desktop_entry);
            let _ = std::process::Command::new("update-desktop-database")
                .arg(&app_dir)
                .output();
            let _ = std::process::Command::new("gtk-update-icon-cache")
                .args(["-f", "-t"])
                .arg(home.join(".local/share/icons/hicolor"))
                .output();
            let _ = std::process::Command::new("kbuildsycoca6")
                .arg("--noincremental")
                .output();
            let _ = std::process::Command::new("kbuildsycoca5")
                .arg("--noincremental")
                .output();
        }
    }
}
pub fn fetch_skin_task(paths: EltharnGatePaths, username: String) -> Task<Message> {
    let clean_name = username.trim().to_string();
    if clean_name.is_empty() {
        return Task::none();
    }
    let username_for_msg = clean_name.clone();
    Task::perform(
        async move { fetch_player_skin(paths, clean_name).await },
        move |res| match res {
            Ok((name, bytes)) => Message::PlayerSkinLoaded {
                username: name,
                skin_bytes: Ok(bytes),
            },
            Err(e) => Message::PlayerSkinLoaded {
                username: username_for_msg.clone(),
                skin_bytes: Err(e),
            },
        },
    )
}
fn encode_url_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'_' {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}
pub async fn fetch_player_skin(
    paths: EltharnGatePaths,
    username: String,
) -> Result<(String, Vec<u8>), String> {
    let clean_name = username.trim().to_string();
    if clean_name.is_empty() {
        return Err("Empty username".to_string());
    }
    if !el_launcher_core::valid_name(&clean_name) {
        return Err("Invalid player name".to_string());
    }
    let cache_dir = paths.skins_heads_dir();
    let cache_file = cache_dir.join(format!(
        "{}.png",
        crate::launcher::utils::sanitize_name(&clean_name)
    ));
    if cache_file.is_file() {
        if let Ok(bytes) = fs::read(&cache_file) {
            if !bytes.is_empty() {
                return Ok((clean_name, bytes));
            }
        }
    }
    let client = reqwest::Client::builder()
        .user_agent(crate::launcher::utils::USER_AGENT)
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    let encoded_name = encode_url_segment(&clean_name);
    let minotar_url = format!("https://minotar.net/helm/{encoded_name}/64.png");
    if let Ok(resp) = client.get(&minotar_url).send().await {
        if resp.status().is_success() {
            if let Ok(bytes) = resp.bytes().await {
                if !bytes.is_empty() {
                    let _ = fs::create_dir_all(&cache_dir);
                    let _ = fs::write(&cache_file, &bytes);
                    return Ok((clean_name, bytes.to_vec()));
                }
            }
        }
    }
    Err(format!("Could not fetch skin avatar for {clean_name}"))
}
