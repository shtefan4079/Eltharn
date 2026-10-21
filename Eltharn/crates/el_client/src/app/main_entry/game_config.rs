use std::path::PathBuf;
#[derive(Clone)]
pub struct User {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub xuid: String,
    pub client_id: String,
}
impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("username", &self.username)
            .field("uuid", &self.uuid)
            .field("access_token", &"<redacted>")
            .field("xuid", &"<redacted>")
            .field("client_id", &"<redacted>")
            .finish()
    }
}
#[derive(Clone)]
pub struct ProxyConfig {
    pub host: Option<String>,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}
impl std::fmt::Debug for ProxyConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}
#[derive(Debug, Clone)]
pub struct UserData {
    pub user: User,
    pub proxy: ProxyConfig,
}
#[derive(Debug, Clone)]
pub struct DisplayData {
    pub width: u32,
    pub height: u32,
    pub fullscreen_width: Option<u32>,
    pub fullscreen_height: Option<u32>,
    pub fullscreen: bool,
}
#[derive(Debug, Clone)]
pub struct FolderData {
    pub game_directory: PathBuf,
    pub resource_pack_directory: Option<PathBuf>,
    pub asset_directory: Option<PathBuf>,
    pub asset_index: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferredGraphicsApi {
    OpenGL,
    Vulkan,
}
#[derive(Debug, Clone)]
pub struct GameData {
    pub demo: bool,
    pub launch_version: String,
    pub version_type: String,
    pub disable_multiplayer: bool,
    pub disable_chat: bool,
    pub capture_tracy_images: bool,
    pub render_debug_labels: bool,
    pub vulkan_validation: bool,
    pub forced_graphics_api: Option<PreferredGraphicsApi>,
    pub offline_developer_mode: bool,
    pub debug_plus: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickPlayVariant {
    Disabled,
    SinglePlayer { world_id: Option<String> },
    Multiplayer { server_address: String },
    Realms { realm_id: String },
}
#[derive(Debug, Clone)]
pub struct QuickPlayData {
    pub log_path: Option<String>,
    pub variant: QuickPlayVariant,
}
impl QuickPlayData {
    pub fn is_enabled(&self) -> bool {
        match &self.variant {
            QuickPlayVariant::Disabled => false,
            QuickPlayVariant::SinglePlayer { .. } => true,
            QuickPlayVariant::Multiplayer { server_address } => !server_address.trim().is_empty(),
            QuickPlayVariant::Realms { realm_id } => !realm_id.trim().is_empty(),
        }
    }
    pub fn multiplayer_server_address(&self) -> Option<&str> {
        match &self.variant {
            QuickPlayVariant::Multiplayer { server_address }
                if !server_address.trim().is_empty() =>
            {
                Some(server_address)
            }
            QuickPlayVariant::Disabled
            | QuickPlayVariant::SinglePlayer { .. }
            | QuickPlayVariant::Multiplayer { .. }
            | QuickPlayVariant::Realms { .. } => None,
        }
    }
}
#[derive(Debug, Clone)]
pub struct ProtocolBackendData {
    pub requested_version: String,
    pub backend_path: Option<PathBuf>,
}
impl ProtocolBackendData {
    pub fn wire_protocol_number(&self) -> Option<i32> {
        let path = self.backend_path.as_ref()?;
        let target = el_launcher_core::BackendTarget::parse(path.to_str()?).ok()?;
        if let Some(version) = target.builtin_version() {
            let manifest = crate::builtin_backend_manifest()?;
            return (version == self.requested_version && manifest.minecraft_version == version)
                .then_some(manifest.protocol_version);
        }
        #[cfg(target_os = "android")]
        if let Some(protocol) = crate::android_bundle::wire_protocol(path, &self.requested_version)
        {
            return Some(protocol);
        }
        let manifest_path = if path.is_dir() {
            path.join("manifest.json")
        } else if path.file_name().is_some_and(|name| name == "manifest.json") {
            path.clone()
        } else {
            path.parent()?.join("manifest.json")
        };
        let bytes = std::fs::read(manifest_path).ok()?;
        let manifest: el_protocol_backend::ProtocolBackendManifest =
            serde_json::from_slice(&bytes).ok()?;
        (manifest.ipc_version == el_protocol_backend::ELTHARN_PROTOCOL_BACKEND_IPC_VERSION
            && manifest.minecraft_version == self.requested_version
            && manifest.protocol_version > 0
            && manifest.assets_compatibility.as_deref()
                == Some(el_launcher_core::ELTHARN_CLIENT_ASSETS_COMPATIBILITY))
        .then_some(manifest.protocol_version)
    }
}
#[derive(Clone)]
pub struct GameConfig {
    pub prepared_asset_manifest: Option<PathBuf>,
    pub touch_control_profile: Option<PathBuf>,
    pub user: UserData,
    pub display: DisplayData,
    pub location: FolderData,
    pub game: GameData,
    pub quick_play: QuickPlayData,
    pub protocol_backend: ProtocolBackendData,
}
impl std::fmt::Debug for GameConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameConfig")
            .field("prepared_asset_manifest", &self.prepared_asset_manifest)
            .field("touch_control_profile", &self.touch_control_profile)
            .field("user", &self.user)
            .field("display", &self.display)
            .field("location", &self.location)
            .field("game", &self.game)
            .field("quick_play", &self.quick_play)
            .field("protocol_backend", &self.protocol_backend)
            .finish()
    }
}
impl GameConfig {
    pub fn new(
        user: UserData,
        display: DisplayData,
        location: FolderData,
        game: GameData,
        quick_play: QuickPlayData,
        protocol_backend: ProtocolBackendData,
    ) -> Self {
        Self {
            prepared_asset_manifest: None,
            touch_control_profile: None,
            user,
            display,
            location,
            game,
            quick_play,
            protocol_backend,
        }
    }
    pub fn from_launcher_handoff(
        launch: &el_launcher_core::GameLaunchConfig,
        game_directory: PathBuf,
        assets_directory: PathBuf,
        resource_pack_directory: Option<PathBuf>,
        prepared_asset_manifest: Option<PathBuf>,
    ) -> Self {
        let user = UserData {
            user: User {
                username: launch.account_id.as_str().to_string(),
                uuid: String::new(),
                access_token: String::new(),
                xuid: String::new(),
                client_id: String::new(),
            },
            proxy: ProxyConfig {
                host: None,
                port: 0,
                username: None,
                password: None,
            },
        };
        let display = DisplayData {
            width: 854,
            height: 480,
            fullscreen_width: None,
            fullscreen_height: None,
            fullscreen: false,
        };
        let location = FolderData {
            game_directory,
            resource_pack_directory,
            asset_directory: Some(assets_directory),
            asset_index: None,
        };
        let game = GameData {
            demo: false,
            launch_version: launch
                .game_version
                .as_ref()
                .map(|v| v.0.clone())
                .unwrap_or_else(|| {
                    el_launcher_core::ELTHARN_CLIENT_ASSETS_COMPATIBILITY.to_string()
                }),
            version_type: "release".to_string(),
            disable_multiplayer: false,
            disable_chat: false,
            capture_tracy_images: false,
            render_debug_labels: false,
            vulkan_validation: false,
            forced_graphics_api: None,
            offline_developer_mode: false,
            debug_plus: false,
        };
        let quick_play = QuickPlayData {
            log_path: None,
            variant: if let Some(target) = &launch.target_server {
                QuickPlayVariant::Multiplayer {
                    server_address: format!("{}:{}", target.host, target.port),
                }
            } else {
                QuickPlayVariant::Disabled
            },
        };
        let protocol_backend = ProtocolBackendData {
            requested_version: launch.protocol_id.as_str().to_string(),
            backend_path: None,
        };
        let mut config = Self::new(user, display, location, game, quick_play, protocol_backend);
        config.prepared_asset_manifest = prepared_asset_manifest;
        config
    }
    pub fn parse_args(args: &[String]) -> Result<Self, String> {
        let mut demo = false;
        let mut disable_multiplayer = false;
        let mut disable_chat = false;
        let mut fullscreen = false;
        let mut render_debug_labels = false;
        let mut vulkan_validation = false;
        let mut forced_graphics_api = None;
        let mut offline_developer_mode = false;
        let mut capture_tracy_images = true;
        let mut debug_plus = false;
        let mut quick_play_path = None;
        let mut quick_play_singleplayer = None;
        let mut quick_play_multiplayer = None;
        let mut quick_play_realms = None;
        let mut game_dir = PathBuf::from(".");
        let mut assets_dir = None;
        let mut resource_pack_dir = None;
        let mut proxy_host = None;
        let mut proxy_port = 8080;
        let mut proxy_user = None;
        let mut proxy_pass = None;
        let mut username = format!(
            "Player{}",
            web_time::SystemTime::now()
                .duration_since(web_time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                % 1000
        );
        let mut uuid_str = None;
        let mut xuid = String::new();
        let mut client_id = String::new();
        let mut access_token = None;
        let mut version = None;
        let mut version_type = "release".to_string();
        let mut width = 854;
        let mut height = 480;
        let mut fullscreen_width = None;
        let mut fullscreen_height = None;
        let mut asset_index = None;
        let mut eltharn_protocol_version = None;
        let mut el_protocol_backend = None;
        let mut prepared_asset_manifest = None;
        let mut touch_control_profile = None;
        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];
            if arg == "--eltharnTouchControls" {
                let value = args
                    .get(i + 1)
                    .filter(|value| !value.starts_with("--"))
                    .ok_or_else(|| "--eltharnTouchControls requires a profile path".to_string())?;
                touch_control_profile = Some(PathBuf::from(value));
                i += 1;
            } else if arg == "--eltharnPreparedAssets" {
                let value = args
                    .get(i + 1)
                    .filter(|value| !value.starts_with("--"))
                    .ok_or_else(|| {
                        "--eltharnPreparedAssets requires a manifest path".to_string()
                    })?;
                prepared_asset_manifest = Some(PathBuf::from(value));
                i += 1;
            } else if arg == "--demo" {
                demo = true;
            } else if arg == "--disableMultiplayer" {
                disable_multiplayer = true;
            } else if arg == "--disableChat" {
                disable_chat = true;
            } else if arg == "--fullscreen" {
                fullscreen = true;
            } else if arg == "--renderDebugLabels" {
                render_debug_labels = true;
            } else if arg == "--vulkanValidation" {
                vulkan_validation = true;
            } else if arg == "--offlineDeveloperMode" {
                offline_developer_mode = true;
            } else if arg == "--tracyNoImages" {
                capture_tracy_images = false;
            } else if arg == "--debug+" {
                debug_plus = true;
            } else if arg == "--quickPlayPath" && i + 1 < args.len() {
                quick_play_path = Some(args[i + 1].clone());
                i += 1;
            } else if arg == "--quickPlaySingleplayer" {
                if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    quick_play_singleplayer = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    quick_play_singleplayer = Some(String::new());
                }
            } else if arg == "--quickPlayMultiplayer" && i + 1 < args.len() {
                quick_play_multiplayer = Some(args[i + 1].clone());
                i += 1;
            } else if arg == "--quickPlayRealms" && i + 1 < args.len() {
                quick_play_realms = Some(args[i + 1].clone());
                i += 1;
            } else if arg == "--gameDir" && i + 1 < args.len() {
                game_dir = PathBuf::from(&args[i + 1]);
                i += 1;
            } else if arg == "--assetsDir" && i + 1 < args.len() {
                assets_dir = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            } else if arg == "--resourcePackDir" && i + 1 < args.len() {
                resource_pack_dir = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            } else if arg == "--proxyHost" && i + 1 < args.len() {
                proxy_host = Some(args[i + 1].clone());
                i += 1;
            } else if arg == "--proxyPort" && i + 1 < args.len() {
                proxy_port = args[i + 1]
                    .parse()
                    .map_err(|_| format!("Invalid proxyPort: {}", args[i + 1]))?;
                i += 1;
            } else if arg == "--proxyUser" && i + 1 < args.len() {
                proxy_user = Some(args[i + 1].clone());
                i += 1;
            } else if arg == "--proxyPass" && i + 1 < args.len() {
                proxy_pass = Some(args[i + 1].clone());
                i += 1;
            } else if arg == "--username" && i + 1 < args.len() {
                username = args[i + 1].clone();
                i += 1;
            } else if arg == "--uuid" && i + 1 < args.len() {
                uuid_str = Some(args[i + 1].clone());
                i += 1;
            } else if arg == "--xuid" {
                if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    xuid = args[i + 1].clone();
                    i += 1;
                }
            } else if arg == "--clientId" {
                if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    client_id = args[i + 1].clone();
                    i += 1;
                }
            } else if arg == "--accessToken" && i + 1 < args.len() {
                access_token = Some(args[i + 1].clone());
                i += 1;
            } else if arg == "--version" && i + 1 < args.len() {
                version = Some(args[i + 1].clone());
                i += 1;
            } else if arg == "--versionType" && i + 1 < args.len() {
                version_type = args[i + 1].clone();
                i += 1;
            } else if arg == "--width" && i + 1 < args.len() {
                width = args[i + 1]
                    .parse()
                    .map_err(|_| format!("Invalid width: {}", args[i + 1]))?;
                i += 1;
            } else if arg == "--height" && i + 1 < args.len() {
                height = args[i + 1]
                    .parse()
                    .map_err(|_| format!("Invalid height: {}", args[i + 1]))?;
                i += 1;
            } else if arg == "--fullscreenWidth" && i + 1 < args.len() {
                fullscreen_width = Some(
                    args[i + 1]
                        .parse()
                        .map_err(|_| format!("Invalid fullscreenWidth: {}", args[i + 1]))?,
                );
                i += 1;
            } else if arg == "--fullscreenHeight" && i + 1 < args.len() {
                fullscreen_height = Some(
                    args[i + 1]
                        .parse()
                        .map_err(|_| format!("Invalid fullscreenHeight: {}", args[i + 1]))?,
                );
                i += 1;
            } else if arg == "--assetIndex" && i + 1 < args.len() {
                asset_index = Some(args[i + 1].clone());
                i += 1;
            } else if arg == "--eltharnProtocolVersion" && i + 1 < args.len() {
                eltharn_protocol_version = Some(args[i + 1].clone());
                i += 1;
            } else if arg == "--eltharnProtocolBackend" && i + 1 < args.len() {
                el_protocol_backend = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            } else if arg == "--graphicsBackend" && i + 1 < args.len() {
                let val = args[i + 1].to_lowercase();
                if val == "opengl" {
                    forced_graphics_api = Some(PreferredGraphicsApi::OpenGL);
                } else if val == "vulkan" {
                    forced_graphics_api = Some(PreferredGraphicsApi::Vulkan);
                }
                i += 1;
            }
            i += 1;
        }
        let access_token =
            access_token.ok_or_else(|| "Missing required option: --accessToken".to_string())?;
        let version = version.ok_or_else(|| "Missing required option: --version".to_string())?;
        let uuid = if let Some(ref s) = uuid_str {
            let parsed_s = s.replace("-", "");
            if parsed_s.len() == 32 && parsed_s.is_ascii() {
                let with_dashes = format!(
                    "{}-{}-{}-{}-{}",
                    &parsed_s[0..8],
                    &parsed_s[8..12],
                    &parsed_s[12..16],
                    &parsed_s[16..20],
                    &parsed_s[20..32]
                );
                uuid::Uuid::parse_str(&with_dashes)
                    .map_err(|e| format!("Invalid UUID '{}': {}", s, e))?
            } else {
                uuid::Uuid::parse_str(s).map_err(|e| format!("Invalid UUID '{}': {}", s, e))?
            }
        } else {
            uuid::Uuid::new_v3(
                &uuid::Uuid::nil(),
                format!("OfflinePlayer:{}", username).as_bytes(),
            )
        };
        let mut enabled_qp_options = 0;
        if quick_play_singleplayer.is_some() {
            enabled_qp_options += 1;
        }
        if quick_play_multiplayer.is_some() {
            enabled_qp_options += 1;
        }
        if quick_play_realms.is_some() {
            enabled_qp_options += 1;
        }
        if enabled_qp_options > 1 {
            return Err("Only one quick play option can be specified".to_string());
        }
        let quick_play_variant = if let Some(ref world_id) = quick_play_singleplayer {
            let wid = if world_id.is_empty() {
                None
            } else {
                Some(world_id.clone())
            };
            QuickPlayVariant::SinglePlayer { world_id: wid }
        } else if let Some(ref server_address) = quick_play_multiplayer {
            if server_address.is_empty() {
                QuickPlayVariant::Disabled
            } else {
                QuickPlayVariant::Multiplayer {
                    server_address: server_address.clone(),
                }
            }
        } else if let Some(ref realm_id) = quick_play_realms {
            if realm_id.is_empty() {
                QuickPlayVariant::Disabled
            } else {
                QuickPlayVariant::Realms {
                    realm_id: realm_id.clone(),
                }
            }
        } else {
            QuickPlayVariant::Disabled
        };
        let user = User {
            username,
            uuid: uuid.to_string(),
            access_token,
            xuid,
            client_id,
        };
        let proxy = ProxyConfig {
            host: proxy_host,
            port: proxy_port,
            username: proxy_user,
            password: proxy_pass,
        };
        let user_data = UserData { user, proxy };
        let display_data = DisplayData {
            width,
            height,
            fullscreen_width,
            fullscreen_height,
            fullscreen,
        };
        let folder_data = FolderData {
            game_directory: game_dir,
            resource_pack_directory: resource_pack_dir,
            asset_directory: assets_dir,
            asset_index,
        };
        let requested_protocol_version = eltharn_protocol_version
            .unwrap_or_else(|| el_protocol_backend::VANILLA_VERSION_NAME.to_owned());
        let game_data = GameData {
            demo,
            launch_version: version,
            version_type,
            disable_multiplayer,
            disable_chat,
            capture_tracy_images,
            render_debug_labels,
            vulkan_validation,
            forced_graphics_api,
            offline_developer_mode,
            debug_plus,
        };
        let quick_play_data = QuickPlayData {
            log_path: quick_play_path,
            variant: quick_play_variant,
        };
        let protocol_backend = ProtocolBackendData {
            requested_version: requested_protocol_version,
            backend_path: el_protocol_backend,
        };
        Ok(Self {
            prepared_asset_manifest,
            touch_control_profile,
            user: user_data,
            display: display_data,
            location: folder_data,
            game: game_data,
            quick_play: quick_play_data,
            protocol_backend,
        })
    }
}
