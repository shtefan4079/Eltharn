#[cfg(not(target_arch = "wasm32"))]
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
#[cfg(target_os = "android")]
mod android_bundle;
pub mod app;
pub use el_launcher_core::ELTHARN_CLIENT_ASSETS_COMPATIBILITY;
mod client;
#[cfg(feature = "extensions")]
mod extension_catalog;
#[cfg(feature = "extensions")]
pub mod extension_launch;
mod gameplay;
mod input;
#[cfg(feature = "extensions")]
pub mod modding;
mod network;
mod render;
mod resources;
pub mod runtime;
#[cfg(feature = "shaderpacks")]
pub mod shaderpacks;
mod ui;
mod world;
static BUILTIN_BACKEND: std::sync::OnceLock<el_protocol_backend::ProtocolBackendManifest> =
    std::sync::OnceLock::new();
pub fn register_builtin_backend(
    manifest: el_protocol_backend::ProtocolBackendManifest,
) -> Result<(), String> {
    if manifest.ipc_version != el_protocol_backend::ELTHARN_PROTOCOL_BACKEND_IPC_VERSION
        || manifest.protocol_version <= 0
        || manifest.assets_compatibility.as_deref()
            != Some(el_launcher_core::ELTHARN_CLIENT_ASSETS_COMPATIBILITY)
    {
        return Err("Incompatible built-in backend descriptor".to_owned());
    }
    BUILTIN_BACKEND
        .set(manifest)
        .map_err(|_| "Built-in backend already registered".to_owned())
}
pub(crate) fn builtin_backend_manifest()
-> Option<&'static el_protocol_backend::ProtocolBackendManifest> {
    BUILTIN_BACKEND.get()
}
#[cfg(not(target_arch = "wasm32"))]
use crate::app::main_entry::GameConfig;
#[cfg(not(target_arch = "wasm32"))]
use client::state::State;
use el_core::block_state::BlockState;
use el_core::resource_location::ResourceLocation;
use render::atlas::AtlasDefinition;
use render::baked_model::{
    BakedModel, BakedModelGroup, BakedQuad, WeightedBakedModel, WeightedGroupRandomMode,
};
use render::blockstate::{BlockStateDefinition, VariantValue};
use render::model::BlockModel;
use render::texture::TextureAtlas;
use resources::manager::{ClientResourceManager, ResourceError};
#[cfg(not(target_arch = "wasm32"))]
use runtime::{DesktopGameSession, SessionAction};
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
#[cfg(target_os = "android")]
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
const VERBOSE_STARTUP_LOGS: bool = false;
#[cfg(all(feature = "extensions", not(target_arch = "wasm32")))]
const MOD_CONFIG_DIR: &str = "mod-config";
#[cfg(not(target_arch = "wasm32"))]
fn install_panic_reporter() {
    use std::io::Write;
    let args: Vec<_> = std::env::args_os().collect();
    let game_dir = args
        .windows(2)
        .find(|pair| pair[0] == "--gameDir")
        .map(|pair| PathBuf::from(&pair[1]))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default();
    let report_path = std::env::var_os("ELTHARN_CRASH_REPORT_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| game_dir.join("eltharn-crash.log"));
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|location| {
                format!(
                    "{}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                )
            })
            .unwrap_or_else(|| "unknown".to_string());
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("unnamed");
        let backtrace = std::backtrace::Backtrace::force_capture();
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&report_path)
        {
            let _ = writeln!(
                file,
                "\n[EltharnPanic] unix_seconds={timestamp} thread={thread_name} location={location}"
            );
            let _ = writeln!(file, "{backtrace}");
            let _ = file.sync_all();
        }
        eprintln!(
            "Eltharn hit an internal panic; diagnostic stack was written to {}",
            report_path.display()
        );
    }));
}
#[cfg(target_os = "android")]
fn android_client_trace(message: impl AsRef<str>) {
    use std::io::Write;
    static ENABLED: OnceLock<bool> = OnceLock::new();
    if !*ENABLED.get_or_init(|| cfg!(debug_assertions) || env_truthy("ELTHARN_ANDROID_LOG")) {
        return;
    }
    let Some(path) = android_diagnostic_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "client: {}", message.as_ref());
    }
}
#[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
fn android_client_trace(_message: impl AsRef<str>) {}
#[cfg(target_os = "android")]
pub fn submit_android_mouse_motion(dx: f64, dy: f64) {
    input::android_touch::submit_hardware_event(
        input::android_touch::AndroidHardwareEvent::MouseMotion { dx, dy },
    );
}
#[cfg(target_os = "android")]
pub fn submit_android_mouse_wheel(x: f64, y: f64) {
    input::android_touch::submit_hardware_event(
        input::android_touch::AndroidHardwareEvent::MouseWheel { x, y },
    );
}
#[cfg(target_os = "android")]
pub fn submit_android_mouse_button(button: u16, pressed: bool) {
    input::android_touch::submit_hardware_event(
        input::android_touch::AndroidHardwareEvent::MouseButton { button, pressed },
    );
}
#[cfg(target_os = "android")]
pub fn submit_android_text_input(text: String) {
    input::android_touch::submit_text_input(text);
}
#[cfg(target_os = "android")]
pub fn submit_android_backspace() {
    input::android_touch::submit_text_input("\x08".to_string());
}
#[cfg(target_os = "android")]
pub fn submit_android_enter() {
    input::android_touch::submit_key_action(input::android_touch::AndroidKeyAction::Enter);
}
#[cfg(target_os = "android")]
pub fn submit_android_escape() {
    input::android_touch::submit_key_action(input::android_touch::AndroidKeyAction::Escape);
}
#[cfg(target_os = "android")]
pub fn android_touch_profile_json(value: &str) -> Result<String, String> {
    let profile = el_launcher_core::touch_controls::TouchControlProfile::from_launcher_value(value)
        .map_err(|error| error.to_string())?;
    serde_json::to_string(&profile).map_err(|error| error.to_string())
}
#[cfg(target_os = "android")]
pub fn android_touch_preview_json(value: &str, width: f32, height: f32) -> Result<String, String> {
    let profile = el_launcher_core::touch_controls::TouchControlProfile::from_launcher_value(value)
        .map_err(|error| error.to_string())?;
    let layout = profile
        .resolve(width, height)
        .map_err(|error| error.to_string())?;
    serde_json::to_string(&layout).map_err(|error| error.to_string())
}
#[cfg(target_os = "android")]
static ANDROID_APP: Mutex<Option<winit::platform::android::activity::AndroidApp>> =
    Mutex::new(None);
#[cfg(target_os = "android")]
pub fn set_android_app(app: winit::platform::android::activity::AndroidApp) {
    *ANDROID_APP
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(app);
}
#[cfg(target_os = "android")]
fn current_android_app() -> Option<winit::platform::android::activity::AndroidApp> {
    ANDROID_APP
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}
#[cfg(target_os = "android")]
fn is_current_android_app(expected: &winit::platform::android::activity::AndroidApp) -> bool {
    ANDROID_APP
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
        .is_some_and(|app| app.activity_as_ptr() == expected.activity_as_ptr())
}
#[cfg(target_os = "android")]
pub(crate) fn android_diagnostic_path() -> Option<PathBuf> {
    let app = current_android_app()?;
    app.external_data_path()
        .or_else(|| app.internal_data_path())
        .map(|directory| directory.join("android-main.log"))
}
#[cfg(target_os = "android")]
pub fn clear_android_app(expected: &winit::platform::android::activity::AndroidApp) {
    let mut current = ANDROID_APP
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if current
        .as_ref()
        .is_some_and(|app| app.activity_as_ptr() == expected.activity_as_ptr())
    {
        *current = None;
    }
}
#[cfg(target_os = "android")]
pub fn clear_android_activity(env: &mut jni::JNIEnv<'_>, activity: &jni::objects::JObject<'_>) {
    let Some(app) = current_android_app() else {
        return;
    };
    let expected = unsafe { jni::objects::JObject::from_raw(app.activity_as_ptr().cast()) };
    if env.is_same_object(&expected, activity).unwrap_or(false) {
        clear_android_app(&app);
    }
}
#[cfg(target_os = "android")]
pub(crate) fn android_touch_safe_insets(width: f32, height: f32) -> [f32; 4] {
    let Some(app) = current_android_app() else {
        return [0.0; 4];
    };
    let rect = app.content_rect();
    crate::input::android_touch::content_rect_insets(
        width,
        height,
        [rect.left, rect.top, rect.right, rect.bottom],
    )
}
#[cfg(target_os = "android")]
pub fn open_android_url(url: &str) -> Result<(), String> {
    use jni::{JavaVM, objects::JObject, objects::JValue};
    let app =
        current_android_app().ok_or_else(|| "Android activity is not available".to_string())?;
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }
        .map_err(|error| format!("Could not access Android JVM: {error}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|error| format!("Could not attach to Android JVM: {error}"))?;
    env.with_local_frame(24, |env| -> jni::errors::Result<()> {
        let action = env.new_string("android.intent.action.VIEW")?;
        let action = JObject::from(action);
        let intent = env.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )?;
        let url = env.new_string(url)?;
        let url = JObject::from(url);
        let uri = env
            .call_static_method(
                "android/net/Uri",
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&url)],
            )?
            .l()?;
        env.call_method(
            &intent,
            "setData",
            "(Landroid/net/Uri;)Landroid/content/Intent;",
            &[JValue::Object(&uri)],
        )?;
        let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
        env.call_method(
            activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;
        Ok(())
    })
    .map_err(|error| format!("Android could not open the link: {error}"))
}
#[cfg(target_os = "android")]
pub fn request_android_keyboard() {
    if let Some(app) = current_android_app() {
        app.show_soft_input(true);
    }
}
#[cfg(target_os = "android")]
pub fn hide_android_keyboard() {
    if let Some(app) = current_android_app() {
        app.hide_soft_input(true);
    }
}
#[cfg(not(target_os = "android"))]
pub fn request_android_keyboard() {}
#[cfg(not(target_os = "android"))]
pub fn hide_android_keyboard() {}
fn resource_pack_debug(message: impl AsRef<str>) {
    el_protocol_backend::diagnostics::resource_debug_line(&format!(
        "[ResourcePackDebug] {}",
        message.as_ref()
    ));
}
#[cfg(not(target_arch = "wasm32"))]
fn env_truthy(name: &str) -> bool {
    std::env::var(name)
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false)
}
#[cfg(not(target_arch = "wasm32"))]
fn verbose_console_logs_enabled() -> bool {
    env_truthy("ELTHARN_NOISY_DEBUG")
        || env_truthy("ELTHARN_VERBOSE_LOGS")
        || matches!(
            std::env::var("ELTHARN_LOGS").as_deref(),
            Ok("all" | "trace" | "debug" | "info")
        )
}
#[cfg(not(target_arch = "wasm32"))]
fn configured_log_level() -> log::LevelFilter {
    if verbose_console_logs_enabled() {
        return log::LevelFilter::Trace;
    }
    match std::env::var("ELTHARN_LOGS").as_deref() {
        Ok("trace") => log::LevelFilter::Trace,
        Ok("debug") => log::LevelFilter::Debug,
        Ok("info") => log::LevelFilter::Info,
        Ok("warn") => log::LevelFilter::Warn,
        Ok("error") => log::LevelFilter::Error,
        Ok("all") => log::LevelFilter::Trace,
        _ => log::LevelFilter::Off,
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn verbose_startup_logs_enabled() -> bool {
    VERBOSE_STARTUP_LOGS || verbose_console_logs_enabled()
}
#[cfg(not(target_arch = "wasm32"))]
fn resolve_game_directory(requested: &Path) -> PathBuf {
    std::path::absolute(requested).unwrap_or_else(|_| requested.to_path_buf())
}
#[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
fn ensure_dir_exists(dir: &Path) -> bool {
    if dir.is_dir() {
        return false;
    }
    match std::fs::create_dir_all(dir) {
        Ok(()) => true,
        Err(err) => {
            log::debug!(
                "[DesktopIntegration] cannot create {}: {err}",
                dir.display()
            );
            false
        }
    }
}
#[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
fn write_file_if_changed(path: &Path, contents: &[u8]) -> bool {
    match std::fs::read(path) {
        Ok(existing) if existing == contents => return false,
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            log::debug!("[DesktopIntegration] cannot read {}: {err}", path.display())
        }
    }
    match std::fs::write(path, contents) {
        Ok(()) => true,
        Err(err) => {
            log::debug!(
                "[DesktopIntegration] cannot write {}: {err}",
                path.display()
            );
            false
        }
    }
}
#[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
fn refresh_desktop_caches(app_dir: &Path, home: &Path) {
    let run = |tool: &str, args: &[&std::ffi::OsStr]| match std::process::Command::new(tool)
        .args(args)
        .output()
    {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            log::debug!("[DesktopIntegration] {tool} exited with {}", output.status)
        }
        Err(err) => log::debug!("[DesktopIntegration] {tool} could not run: {err}"),
    };
    run("update-desktop-database", &[app_dir.as_os_str()]);
    let hicolor = home.join(".local/share/icons/hicolor");
    run(
        "gtk-update-icon-cache",
        &[
            std::ffi::OsStr::new("-f"),
            std::ffi::OsStr::new("-t"),
            hicolor.as_os_str(),
        ],
    );
    run("kbuildsycoca6", &[std::ffi::OsStr::new("--noincremental")]);
    run("kbuildsycoca5", &[std::ffi::OsStr::new("--noincremental")]);
}
#[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
fn ensure_desktop_integration() {
    if std::env::var_os("ELTHARN_GATE_DATA_DIR").is_some() {
        return;
    }
    if std::env::var_os("ELTHARN_EXTENSION_REQUEST").is_some() {
        return;
    }
    if env_truthy("ELTHARN_NO_DESKTOP_INTEGRATION") {
        return;
    }
    #[cfg(feature = "shaderpacks")]
    if std::env::var_os("ELTHARN_SHADERPACK_PROBE_PACKET").is_some() {
        return;
    }
    if let Some(home) = dirs::home_dir() {
        let app_dir = home.join(".local/share/applications");
        let desktop_file = app_dir.join("eltharn.desktop");
        const SVG_LOGO: &str = include_str!("icon.svg");
        const ICON_256_PNG: &[u8] = include_bytes!("icon_256.png");
        let mut changed = false;
        let icon_dir_svg = home.join(".local/share/icons/hicolor/scalable/apps");
        let icon_file_svg = icon_dir_svg.join("eltharn.svg");
        changed |= ensure_dir_exists(&icon_dir_svg);
        changed |= write_file_if_changed(&icon_file_svg, SVG_LOGO.as_bytes());
        let loaded_img = image::load_from_memory(ICON_256_PNG).ok();
        let sizes = [16u32, 32, 48, 64, 128, 256, 512];
        let mut png_cache: HashMap<u32, Vec<u8>> = HashMap::new();
        png_cache.insert(256, ICON_256_PNG.to_vec());
        for &size in &sizes {
            let size_dir = home.join(format!(".local/share/icons/hicolor/{}x{}/apps", size, size));
            let size_file = size_dir.join("eltharn.png");
            changed |= ensure_dir_exists(&size_dir);
            if size == 256 {
                changed |= write_file_if_changed(&size_file, ICON_256_PNG);
            } else if let Some(ref img) = loaded_img {
                let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
                let mut buf = std::io::Cursor::new(Vec::new());
                if resized.write_to(&mut buf, image::ImageFormat::Png).is_ok() {
                    let bytes = buf.into_inner();
                    changed |= write_file_if_changed(&size_file, &bytes);
                    png_cache.insert(size, bytes);
                }
            }
        }
        if let Ok(entries) = std::fs::read_dir(home.join(".local/share/icons")) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let apps_svg = p.join("apps/eltharn.svg");
                    if apps_svg.exists() || p.join("apps").is_dir() {
                        changed |= write_file_if_changed(&apps_svg, SVG_LOGO.as_bytes());
                    }
                    let apps_png = p.join("apps/eltharn.png");
                    if apps_png.exists() || p.join("apps").is_dir() {
                        changed |= write_file_if_changed(&apps_png, ICON_256_PNG);
                    }
                    let scalable_svg = p.join("scalable/apps/eltharn.svg");
                    if scalable_svg.exists() || p.join("scalable/apps").is_dir() {
                        changed |= write_file_if_changed(&scalable_svg, SVG_LOGO.as_bytes());
                    }
                    for &size in &sizes {
                        let theme_size_dir = p.join(format!("{}x{}/apps", size, size));
                        if theme_size_dir.is_dir() {
                            if let Some(png_bytes) = png_cache.get(&size) {
                                changed |= write_file_if_changed(
                                    &theme_size_dir.join("eltharn.png"),
                                    png_bytes,
                                );
                            }
                        }
                    }
                }
            }
        }
        changed |= ensure_dir_exists(&app_dir);
        let bin_path = if let Ok(current_exe) = std::env::current_exe() {
            current_exe.display().to_string()
        } else {
            home.join(".local/bin/eltharn").display().to_string()
        };
        let desktop_entry = format!(
            "[Desktop Entry]\n\
             Name=Eltharn\n\
             Comment=Eltharn Next-Gen Client\n\
             Exec={bin_path}\n\
             Icon=eltharn\n\
             Terminal=false\n\
             Type=Application\n\
             Categories=Game;Application;\n\
             StartupWMClass=eltharn\n\
             StartupNotify=true\n\
             Keywords=minecraft;eltharn;game;\n"
        );
        changed |= write_file_if_changed(&desktop_file, desktop_entry.as_bytes());
        if changed {
            refresh_desktop_caches(&app_dir, &home);
        } else {
            log::debug!("[DesktopIntegration] desktop entry and icons are already up to date");
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub fn run_desktop_client() {
    let args_raw: Vec<String> = std::env::args().collect();
    if args_raw
        .iter()
        .any(|arg| arg == "--eltharnPrintProtocolInfo")
    {
        let info = serde_json::json!(
            { "networking" : "ipc", "assets_compatibility" : "26.2", "ipc_version" :
            el_protocol_backend::ELTHARN_PROTOCOL_BACKEND_IPC_VERSION, "extensions" : {
            "launch_request" : if cfg!(feature = "extensions") { 2 } else { 0 },
            "shaderpacks" : cfg!(feature = "extensions"), "mod_execution" : cfg!(feature
            = "extensions"), }, }
        );
        println!("{}", serde_json::to_string_pretty(&info).unwrap());
        return;
    }
    install_panic_reporter();
    #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
    ensure_desktop_integration();
    if verbose_console_logs_enabled() {
        println!(
            "[EltharnBuild] client={} executable={} assets_version={} networking=ipc",
            env!("CARGO_PKG_NAME"),
            std::env::current_exe()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| "<unknown>".to_string()),
            el_launcher_core::ELTHARN_CLIENT_ASSETS_COMPATIBILITY,
        );
    }
    env_logger::Builder::new()
        .filter_level(configured_log_level())
        .init();
    if std::env::var("ELTHARN_DISABLE_FACE_CULL")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false)
        && verbose_startup_logs_enabled()
    {
        println!("[Debug] Face culling disabled for block models (ELTHARN_DISABLE_FACE_CULL=1)");
    }
    let total_startup_start = web_time::Instant::now();
    let t_args_start = web_time::Instant::now();
    let args: Vec<String> = std::env::args().collect();
    let game_config = match GameConfig::parse_args(&args) {
        Ok(cfg) => cfg,
        Err(err) => {
            log::error!("Failed to parse arguments: {}", err);
            std::process::exit(-5);
        }
    };
    let t_args = t_args_start.elapsed();
    let t_resources_start = web_time::Instant::now();
    let resource_manager = if let Some(receipt) = &game_config.prepared_asset_manifest {
        match game_config.location.asset_directory.as_deref() {
            Some(assets_dir) => {
                pollster::block_on(resources::prepared::prepare_launcher_resources(
                    assets_dir,
                    receipt,
                    &game_config.game.launch_version,
                ))
            }
            None => Err("Prepared resources require --assetsDir".to_string()),
        }
    } else {
        ClientResourceManager::from_configured_location(
            &game_config.location,
            game_config.game.offline_developer_mode,
        )
        .map_err(|error| error.to_string())
    }
    .unwrap_or_else(|error| {
        eprintln!("Eltharn resource preparation failed: {error}");
        std::process::exit(1);
    });
    let game_directory = if game_config.game.offline_developer_mode {
        resolve_game_directory(&game_config.location.game_directory)
    } else {
        game_config.location.game_directory.clone()
    };
    #[cfg(feature = "extensions")]
    crate::modding::set_config_root(game_directory.join(MOD_CONFIG_DIR));
    if let Some(path) = std::env::var_os("ELTHARN_EXTENSION_REQUEST") {
        #[cfg(feature = "extensions")]
        if let Err(error) =
            crate::extension_launch::configure_native(&std::path::PathBuf::from(path))
        {
            eprintln!("Extensions could not start: {error}");
            std::process::exit(2);
        }
        #[cfg(not(feature = "extensions"))]
        {
            let _ = path;
            eprintln!("This client was built without extension support.");
            std::process::exit(2);
        }
    }
    let resource_pack_directory = game_config
        .location
        .resource_pack_directory
        .clone()
        .unwrap_or_else(|| {
            resources::pack_loader::resource_packs_dir(&game_directory.join("options.txt"))
        });
    let options_path = game_directory.join("options.txt");
    let pack_directory = resource_pack_directory.clone();
    let resource_manager = resources::pack_loader::with_selected_resource_packs_in_dir(
        resource_manager,
        &options_path,
        &pack_directory,
    );
    let persisted_language_source = std::fs::read_to_string(&options_path).unwrap_or_default();
    let persisted_language_code =
        crate::ui::i18n::active_language_code_from_options(&persisted_language_source);
    crate::ui::i18n::reload_from_resource_manager(&resource_manager, persisted_language_code);
    let t_resources = t_resources_start.elapsed();
    let touch_profile = match load_desktop_touch_profile(
        &game_directory,
        game_config.touch_control_profile.as_deref(),
    ) {
        Ok(profile) => profile,
        Err(error) => {
            eprintln!("Touch controls could not be loaded: {error}");
            return;
        }
    };
    run_client_with_event_loop(
        EventLoop::new().expect(
            "could not create the OS event loop; a display server (X11/Wayland) or a supported \
             backend is required",
        ),
        game_config,
        resource_manager,
        game_directory,
        resource_pack_directory,
        t_resources,
        total_startup_start,
        t_args,
        touch_profile,
    );
}
#[cfg(not(target_arch = "wasm32"))]
fn load_desktop_touch_profile(
    game_directory: &Path,
    explicit_path: Option<&Path>,
) -> Result<el_launcher_core::touch_controls::TouchControlProfile, String> {
    use el_launcher_core::touch_controls::{TouchControlProfile, TouchPreset};
    match explicit_path {
        Some(path) => load_native_touch_profile(game_directory, Some(path), TouchPreset::Disabled),
        None => Ok(TouchControlProfile::for_preset(TouchPreset::Disabled)),
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn load_native_touch_profile(
    game_directory: &Path,
    explicit_path: Option<&Path>,
    fallback: el_launcher_core::touch_controls::TouchPreset,
) -> Result<el_launcher_core::touch_controls::TouchControlProfile, String> {
    use el_launcher_core::touch_controls::{
        MAX_TOUCH_PROFILE_BYTES, TOUCH_CONTROL_PROFILE_FILE, TouchControlProfile,
    };
    use std::io::Read;
    let default_path = game_directory.join(TOUCH_CONTROL_PROFILE_FILE);
    let path = explicit_path.unwrap_or(&default_path);
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if explicit_path.is_none() && error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(TouchControlProfile::for_preset(fallback));
        }
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let mut bytes = Vec::new();
    file.take((MAX_TOUCH_PROFILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let json =
        std::str::from_utf8(&bytes).map_err(|_| "Touch profile must be UTF-8".to_string())?;
    TouchControlProfile::from_launcher_value(json).map_err(|error| error.to_string())
}
#[cfg(all(not(target_arch = "wasm32"), target_os = "android"))]
fn save_native_touch_profile(
    game_directory: &Path,
    profile: &el_launcher_core::touch_controls::TouchControlProfile,
) -> Result<(), String> {
    use std::io::Write;
    profile.validate().map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec_pretty(profile).map_err(|error| error.to_string())?;
    std::fs::create_dir_all(game_directory).map_err(|error| error.to_string())?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(game_directory).map_err(|error| error.to_string())?;
    temporary
        .write_all(&bytes)
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|error| error.to_string())?;
    temporary
        .persist(game_directory.join(el_launcher_core::touch_controls::TOUCH_CONTROL_PROFILE_FILE))
        .map_err(|error| error.to_string())?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
fn run_client_with_event_loop(
    event_loop: EventLoop<()>,
    game_config: GameConfig,
    resource_manager: ClientResourceManager,
    game_directory: PathBuf,
    resource_pack_directory: PathBuf,
    t_resources: Duration,
    total_startup_start: web_time::Instant,
    t_args: Duration,
    touch_profile: el_launcher_core::touch_controls::TouchControlProfile,
) {
    #[cfg(not(target_os = "android"))]
    let _ = touch_profile;
    #[cfg(target_os = "android")]
    let android_activity = match current_android_app() {
        Some(activity) => activity,
        None => {
            log::error!("Android client started without an active GameActivity");
            return;
        }
    };
    android_client_trace(format!(
        "event_loop_start requested_size={}x{}",
        game_config.display.width, game_config.display.height
    ));
    let t_window_start = web_time::Instant::now();
    let mut window_builder =
        WindowBuilder::new()
            .with_title("Eltharn")
            .with_inner_size(winit::dpi::LogicalSize::new(
                game_config.display.width as f64,
                game_config.display.height as f64,
            ));
    #[cfg(not(target_arch = "wasm32"))]
    {
        const ICON_RGBA: &[u8] = include_bytes!("icon_rgba.bin");
        if let Ok(icon) = winit::window::Icon::from_rgba(ICON_RGBA.to_vec(), 256, 256) {
            window_builder = window_builder.with_window_icon(Some(icon));
        }
    }
    #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
    {
        use winit::platform::wayland::WindowBuilderExtWayland;
        use winit::platform::x11::WindowBuilderExtX11;
        window_builder = WindowBuilderExtWayland::with_name(window_builder, "eltharn", "eltharn");
        window_builder = WindowBuilderExtX11::with_name(window_builder, "eltharn", "eltharn");
    }
    if game_config.display.fullscreen {
        window_builder =
            window_builder.with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
    }
    let window = std::sync::Arc::new(window_builder.build(&event_loop).expect(
        "could not create the game window; check the display server (X11/Wayland) and graphics \
         drivers",
    ));
    #[cfg(target_os = "android")]
    window.set_ime_allowed(true);
    android_client_trace(format!(
        "window_created inner={}x{} scale={}",
        window.inner_size().width,
        window.inner_size().height,
        window.scale_factor()
    ));
    let t_window = t_window_start.elapsed();
    let t_state_start = web_time::Instant::now();
    #[cfg(feature = "shaderpacks")]
    if let Some(path) = std::env::var_os("ELTHARN_SHADERPACK_PROBE_PACKET") {
        use std::io::Read;
        let loaded = std::fs::File::open(&path).and_then(|file| {
            let mut bytes = Vec::new();
            file.take(262145).read_to_end(&mut bytes)?;
            Ok(bytes)
        });
        match loaded {
            Ok(bytes) => {
                if let Err(error) = pollster::block_on(crate::shaderpacks::configure_packet(&bytes))
                {
                    log::error!("Shader probe launch rejected: {error}");
                    return;
                }
            }
            Err(error) => {
                log::error!("Shader probe packet cannot be read: {error}");
                return;
            }
        }
    }
    let mut state = pollster::block_on(State::new(
        window.clone(),
        &resource_manager,
        game_config.game.debug_plus,
    ));
    #[cfg(all(feature = "extensions", not(target_os = "android")))]
    if crate::extension_launch::configured() {
        let result = crate::extension_launch::finish_gpu(state.extension_launch_error());
        if let Err(error) = crate::extension_launch::write_native_status() {
            log::error!("Cannot report extension startup: {error}");
        }
        if let Err(error) = result {
            log::error!("Extensions could not start: {error}");
            if std::env::var_os("ELTHARN_EXTENSION_REQUEST").is_some() {
                std::process::exit(2);
            }
            return;
        }
    }
    #[cfg(target_os = "android")]
    {
        state.configure_android_controls(touch_profile);
        android_client_trace(format!(
            "android_controls_configured logical_scale={}",
            state
                .gui_manager
                .get_gui_scale_factor(state.size.width as f32, state.size.height as f32)
        ));
    }
    android_client_trace(format!(
        "state_created size={}x{} config={}x{}",
        state.size.width, state.size.height, state.config.width, state.config.height
    ));
    state.gui_manager.configure_game_directories(
        game_directory.clone(),
        Some(resource_pack_directory.clone()),
    );
    state.gui_manager.reload_options_from_disk();
    let persisted_language_source = std::fs::read_to_string(&state.gui_manager.options_path)
        .or_else(|_| std::fs::read_to_string("options.txt"))
        .unwrap_or_default();
    let persisted_language_code =
        crate::ui::i18n::active_language_code_from_options(&persisted_language_source);
    crate::ui::i18n::reload_from_resource_manager(
        &state.gui_manager.resource_manager_clone(),
        persisted_language_code,
    );
    state.apply_pending_window_settings();
    if verbose_console_logs_enabled() {
        println!(
            "[EltharnPaths] game_dir={} resourcepacks_dir={}",
            game_directory.display(),
            resource_pack_directory.display()
        );
    }
    let t_state = t_state_start.elapsed();
    if verbose_startup_logs_enabled() {
        println!("[Debug] verbose startup/render diagnostics enabled");
    }
    log::info!("Window and WGPU initialized successfully!");
    let fallback_atlas_started = web_time::Instant::now();
    let (texture_atlas, rebuilt_atlas) = match state.renderer.block_atlas.take() {
        Some(atlas) => (atlas, false),
        None => {
            let (device, queue) = state.get_device_queue();
            (
                build_render_texture_atlas(
                    device,
                    queue,
                    &state.renderer.texture_bind_group_layout,
                    &resource_manager,
                )
                .expect("initial resource reload and fallback atlas build both failed"),
                true,
            )
        }
    };
    let t_atlas = rebuilt_atlas
        .then(|| fallback_atlas_started.elapsed())
        .unwrap_or(Duration::ZERO);
    let must_rebuild_models = rebuilt_atlas || state.renderer.model_cache.is_empty();
    let fallback_baking_started = web_time::Instant::now();
    if must_rebuild_models {
        state.renderer.model_cache = bake_render_model_cache(&resource_manager, &texture_atlas);
    }
    let t_baking = must_rebuild_models
        .then(|| fallback_baking_started.elapsed())
        .unwrap_or(Duration::ZERO);
    let states_baked = state.renderer.model_cache.len();
    state.update_sky_celestial_buffers(&texture_atlas);
    state.renderer.block_atlas = Some(texture_atlas);
    if verbose_startup_logs_enabled() {
        println!(
            "[Render] Initial resources reused={} baked_states={states_baked}",
            !rebuilt_atlas
        );
    }
    let total_startup = total_startup_start.elapsed();
    el_protocol_backend::diagnostics::write_render_line(&format!(
        "[StartupProfiler] total={:.2}ms args={:.2}ms resources={:.2}ms window={:.2}ms state_wgpu_initial_resources={:.2}ms duplicate_block_colors=0.00ms fallback_atlas={:.2}ms fallback_models={:.2}ms preview_level=none preview_chunks=0 initial_resources_reused={} baked_states={states_baked}",
        total_startup.as_secs_f64() * 1_000.0,
        t_args.as_secs_f64() * 1_000.0,
        t_resources.as_secs_f64() * 1_000.0,
        t_window.as_secs_f64() * 1_000.0,
        t_state.as_secs_f64() * 1_000.0,
        t_atlas.as_secs_f64() * 1_000.0,
        t_baking.as_secs_f64() * 1_000.0,
        !rebuilt_atlas && !must_rebuild_models,
    ));
    if game_config.game.debug_plus && verbose_startup_logs_enabled() {
        println!(
            "\x1b[1;36m================================================================================\x1b[0m\n\
             \x1b[1mEltharn Client Startup Performance Profile\x1b[0m\n\
             --------------------------------------------------------------------------------\n\
               CLI Args Parsing:              {:.2} ms\n\
               Resource Discovery:            {:.2} ms\n\
               Window & Event Loop Creation:  {:.2} ms\n\
               State, WGPU & Resources:       {:.2} ms\n\
               Fallback Atlas Build:          {:.2} ms\n\
               Fallback Model Bake ({} states): {:.2} ms\n\
               Preview Level:                 none (Java parity)\n\
             --------------------------------------------------------------------------------\n\
               \x1b[1mTotal Startup Duration:\x1b[0m        {:.2} ms\n\
             \x1b[1;36m================================================================================\x1b[0m",
            t_args.as_secs_f64() * 1000.0,
            t_resources.as_secs_f64() * 1000.0,
            t_window.as_secs_f64() * 1000.0,
            t_state.as_secs_f64() * 1000.0,
            t_atlas.as_secs_f64() * 1000.0,
            states_baked,
            t_baking.as_secs_f64() * 1000.0,
            total_startup.as_secs_f64() * 1000.0
        );
    }
    state.gui_manager.configure_title_screen(
        game_config.game.launch_version.clone(),
        game_config.game.demo,
        !game_config.game.disable_multiplayer,
    );
    let launcher_profile_id = match uuid::Uuid::parse_str(&game_config.user.user.uuid) {
        Ok(profile_id) => profile_id,
        Err(_) => {
            let message = format!(
                "Eltharn cannot start: the launcher did not provide a valid account UUID \
                 (received '{}'). Start the game through Eltharn Gate or pass a valid --uuid.",
                game_config.user.user.uuid
            );
            log::error!("{message}");
            eprintln!("{message}");
            std::process::exit(3);
        }
    };
    state
        .gui_manager
        .configure_player_profile(game_config.user.user.username.clone(), launcher_profile_id);
    state
        .gui_manager
        .configure_protocol_backend(game_config.protocol_backend.clone());
    log::info!(
        "Eltharn protocol selection: requested={} backend={:?}; launch/assets version={}",
        game_config.protocol_backend.requested_version,
        game_config.protocol_backend.backend_path,
        game_config.game.launch_version
    );
    if let Some(server_address) = game_config.quick_play.multiplayer_server_address() {
        state
            .gui_manager
            .start_quick_play_multiplayer(server_address.to_owned());
        android_client_trace("quick_play_multiplayer_started");
    } else {
        if game_config.quick_play.is_enabled() {
            log::warn!(
                "Quick play requested a currently unported singleplayer or Realms route; opening title screen"
            );
        }
        state.gui_manager.open_title_screen();
        android_client_trace("title_screen_opened");
    }
    event_loop.set_control_flow(ControlFlow::Poll);
    #[cfg(feature = "shaderpacks")]
    if let Some(address) = crate::shaderpacks::take_quick_play() {
        state.gui_manager.start_quick_play_multiplayer(address);
    }
    android_client_trace("event_loop_run");
    let mut session = DesktopGameSession::new(state);
    #[cfg(feature = "shaderpacks")]
    let mut shader_probe_control = crate::shaderpacks::native_control::NativeControl::for_launch();
    let run_result = event_loop.run(move |event, elwt| match event {
        Event::LoopExiting => {
            #[cfg(feature = "extensions")]
            crate::modding::shutdown();
            session.finish_performance_summary();
            el_protocol_backend::diagnostics::shutdown_render_log();
        }
        Event::Suspended => {
            android_client_trace("app_event_suspended");
            session.on_suspended();
        }
        Event::Resumed => {
            android_client_trace("app_event_resumed");
            if let Err(error) = session.on_resumed() {
                log::error!("Could not resume render surface: {error}");
                elwt.exit();
            }
        }
        Event::DeviceEvent { event, .. } => session.on_device_event(&event),
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            if matches!(session.on_window_event(event), SessionAction::ExitRequested) {
                elwt.exit();
            }
        }
        Event::AboutToWait => {
            #[cfg(target_os = "android")]
            if !is_current_android_app(&android_activity) {
                session.on_suspended();
                elwt.exit();
                return;
            }
            #[cfg(feature = "shaderpacks")]
            if shader_probe_control
                .as_mut()
                .is_some_and(|control| control.poll(&window))
            {
                elwt.exit();
                return;
            }
            if session.is_suspended() {
                return;
            }
            match session.on_about_to_wait() {
                SessionAction::ExitRequested => elwt.exit(),
                SessionAction::RequestRedraw => {
                    #[cfg(target_os = "android")]
                    {
                        if matches!(session.render_now(), SessionAction::ExitRequested) {
                            elwt.exit();
                        }
                    }
                    #[cfg(not(target_os = "android"))]
                    window.request_redraw();
                }
                SessionAction::Continue => {}
            }
        }
        _ => {}
    });
    if let Err(error) = run_result {
        match error {
            winit::error::EventLoopError::ExitFailure(code) => {
                log::warn!(
                    "Event loop exited with status {}; treating it as window close",
                    code
                );
            }
            other => {
                log::error!("Event loop failed: {}", other);
            }
        }
    }
}
#[cfg(target_os = "android")]
fn android_native_lib_dir() -> Option<PathBuf> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    let line = maps
        .lines()
        .find(|line| line.contains("libeltharn_android.so"))?;
    let path = line.split_whitespace().last()?;
    Path::new(path).parent().map(|dir| dir.to_path_buf())
}
#[cfg(target_os = "android")]
pub fn run_android_client(
    app: winit::platform::android::activity::AndroidApp,
    game_directory: PathBuf,
    assets_root: PathBuf,
    asset_index: String,
    username: String,
    protocol_version: String,
    controls_mode: String,
) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    set_android_app(app.clone());
    let touch_profile =
        match el_launcher_core::touch_controls::TouchControlProfile::from_launcher_value(
            &controls_mode,
        ) {
            Ok(profile) => profile,
            Err(error) => {
                log::error!("Android touch profile was rejected: {error}");
                return;
            }
        };
    if let Err(error) = save_native_touch_profile(&game_directory, &touch_profile) {
        log::error!("Android touch profile could not be saved: {error}");
        return;
    }
    #[cfg(feature = "extensions")]
    crate::modding::set_config_root(game_directory.join(MOD_CONFIG_DIR));
    let args = vec![
        "eltharn_client".to_string(),
        "--accessToken".to_string(),
        "0".to_string(),
        "--version".to_string(),
        ELTHARN_CLIENT_ASSETS_COMPATIBILITY.to_string(),
        "--eltharnProtocolVersion".to_string(),
        protocol_version.clone(),
        "--username".to_string(),
        username,
        "--gameDir".to_string(),
        game_directory.display().to_string(),
        "--assetsDir".to_string(),
        assets_root.display().to_string(),
        "--width".to_string(),
        "1280".to_string(),
        "--height".to_string(),
        "720".to_string(),
    ];
    let mut args = args;
    let backend_path = android_native_lib_dir().map(|dir| dir.join("libeltharn-protocol.so"));
    match backend_path {
        Some(ref path) if path.is_file() => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
            }
            let manifest = match android_bundle::probe_manifest(path, &protocol_version) {
                Ok(manifest) => manifest,
                Err(error) => {
                    log::error!("Android packaged backend was rejected: {error}");
                    return;
                }
            };
            android_bundle::select(path.clone(), manifest);
            args.push("--eltharnProtocolBackend".to_string());
            args.push(path.display().to_string());
        }
        _ => {
            log::error!("Android packaged protocol backend is missing");
            return;
        }
    }
    let game_config = GameConfig::parse_args(&args)
        .expect("Android Eltharn startup arguments are constructed locally");
    let resource_pack_directory = game_directory.join("resourcepacks");
    std::fs::create_dir_all(&resource_pack_directory)
        .expect("Android game directory must be writable");
    let resource_manager = ClientResourceManager::from_assets_root(
        assets_root.clone(),
        Some(assets_root.clone()),
        Some(&asset_index),
    );
    let event_loop = winit::event_loop::EventLoopBuilder::new()
        .with_android_app(app)
        .build()
        .expect("Android winit event loop requires GameActivity AndroidApp");
    run_client_with_event_loop(
        event_loop,
        game_config,
        resource_manager,
        game_directory,
        resource_pack_directory,
        Duration::ZERO,
        web_time::Instant::now(),
        Duration::ZERO,
        touch_profile,
    );
}
pub(crate) fn collect_render_sprite_ids(
    resource_manager: &ClientResourceManager,
) -> Result<Vec<ResourceLocation>, ResourceError> {
    let blocks_atlas_id = ResourceLocation::from_string("minecraft:blocks");
    let blocks_atlas_def =
        AtlasDefinition::load(resource_manager, &blocks_atlas_id).map_err(|e| {
            ResourceError::Parse {
                id: blocks_atlas_id.clone(),
                message: e.to_string(),
            }
        })?;
    let mut sprite_ids = blocks_atlas_def
        .collect_sprite_ids(resource_manager, "minecraft")
        .map_err(|e| ResourceError::Parse {
            id: blocks_atlas_id.clone(),
            message: e.to_string(),
        })?;
    let namespaces = resource_manager
        .namespaces()
        .unwrap_or_else(|_| vec!["minecraft".to_string()]);
    for ns in &namespaces {
        if ns != "minecraft" {
            if let Ok(ns_blocks_atlas) =
                AtlasDefinition::load(resource_manager, &ResourceLocation::new(ns, "blocks"))
            {
                if let Ok(ids) = ns_blocks_atlas.collect_sprite_ids(resource_manager, ns) {
                    sprite_ids.extend(ids);
                }
            }
        }
        if let Ok(items_atlas_def) =
            AtlasDefinition::load(resource_manager, &ResourceLocation::new(ns, "items"))
        {
            if let Ok(item_sprite_ids) = items_atlas_def.collect_sprite_ids(resource_manager, ns) {
                sprite_ids.extend(item_sprite_ids);
            }
        }
    }
    sprite_ids.extend([
        ResourceLocation::from_string("minecraft:block/water_still"),
        ResourceLocation::from_string("minecraft:block/water_flow"),
        ResourceLocation::from_string("minecraft:block/lava_still"),
        ResourceLocation::from_string("minecraft:block/lava_flow"),
        ResourceLocation::from_string("minecraft:environment/celestial/sun"),
        ResourceLocation::from_string("minecraft:environment/celestial/moon/full_moon"),
        ResourceLocation::from_string("minecraft:environment/celestial/moon/waning_gibbous"),
        ResourceLocation::from_string("minecraft:environment/celestial/moon/third_quarter"),
        ResourceLocation::from_string("minecraft:environment/celestial/moon/waning_crescent"),
        ResourceLocation::from_string("minecraft:environment/celestial/moon/new_moon"),
        ResourceLocation::from_string("minecraft:environment/celestial/moon/waxing_crescent"),
        ResourceLocation::from_string("minecraft:environment/celestial/moon/first_quarter"),
        ResourceLocation::from_string("minecraft:environment/celestial/moon/waxing_gibbous"),
        ResourceLocation::from_string("minecraft:environment/clouds"),
    ]);
    for (source, prefix) in [
        ("entity/banner", "entity/banner/"),
        ("entity/chest", "entity/chest/"),
        ("entity/conduit", "entity/conduit/"),
        ("entity/decorated_pot", "entity/decorated_pot/"),
        ("entity/shield", "entity/shield/"),
        ("entity/trident", "entity/trident/"),
        ("entity/shulker", "entity/shulker/"),
        ("entity/skeleton", "entity/skeleton/"),
        ("entity/zombie", "entity/zombie/"),
        ("entity/creeper", "entity/creeper/"),
        ("entity/piglin", "entity/piglin/"),
        ("entity/enderdragon", "entity/enderdragon/"),
        ("entity/player/wide", "entity/player/wide/"),
        ("entity/copper_golem", "entity/copper_golem/"),
    ] {
        for ns in &namespaces {
            if let Ok(ids) = resource_manager.list_texture_ids_in_directory(ns, source, prefix) {
                sprite_ids.extend(ids);
            }
        }
    }
    sprite_ids.sort_by(|left, right| left.to_string().cmp(&right.to_string()));
    sprite_ids.dedup();
    Ok(sprite_ids)
}
pub(crate) fn build_render_texture_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
    resource_manager: &ClientResourceManager,
) -> Result<TextureAtlas, ResourceError> {
    let sprite_ids = collect_render_sprite_ids(resource_manager)?;
    TextureAtlas::from_resource_ids(
        device,
        queue,
        texture_bind_group_layout,
        resource_manager,
        &sprite_ids,
    )
}
pub(crate) fn bake_render_model_cache(
    resource_manager: &ClientResourceManager,
    texture_atlas: &TextureAtlas,
) -> Arc<HashMap<String, BakedModel>> {
    let all_block_states = el_core::registry::BuiltInRegistries::all_block_states();
    resource_pack_debug(format!(
        "bake_model_cache phase=begin total_states={}",
        all_block_states.len()
    ));
    let mut blockstate_cache: HashMap<ResourceLocation, Option<BlockStateDefinition>> =
        HashMap::new();
    let mut block_model_cache: HashMap<ResourceLocation, Option<BlockModel>> = HashMap::new();
    let mut baked_variant_cache: HashMap<(ResourceLocation, i32, i32, bool), BakedModel> =
        HashMap::new();
    let mut models = HashMap::new();
    let bake_progress = Arc::new(Mutex::new(String::from("initializing")));
    let bake_stop = Arc::new(AtomicBool::new(false));
    #[cfg(not(target_arch = "wasm32"))]
    let heartbeat_progress = Arc::clone(&bake_progress);
    #[cfg(not(target_arch = "wasm32"))]
    let heartbeat_stop = Arc::clone(&bake_stop);
    #[cfg(not(target_arch = "wasm32"))]
    let heartbeat = std::thread::spawn(move || {
        while !heartbeat_stop.load(Ordering::Relaxed) {
            std::thread::park_timeout(Duration::from_secs(5));
            if heartbeat_stop.load(Ordering::Relaxed) {
                break;
            }
            let current = heartbeat_progress
                .lock()
                .map(|state| state.clone())
                .unwrap_or_else(|_| "progress_lock_poisoned".to_string());
            resource_pack_debug(format!(
                "bake_model_cache phase=heartbeat current={}",
                current
            ));
        }
    });
    for (index, block_state) in all_block_states.iter().enumerate() {
        if index % 256 == 0 {
            if let Ok(mut progress) = bake_progress.lock() {
                *progress = format!(
                    "index={} block={} key={}",
                    index,
                    block_state.block,
                    render::mesher::ChunkMesher::block_state_model_key(block_state)
                );
            }
            resource_pack_debug(format!(
                "bake_model_cache phase=progress index={} total={} block={}",
                index,
                all_block_states.len(),
                block_state.block
            ));
        }
        if block_state.block.path == "air" || block_state.block.path.is_empty() {
            continue;
        }
        let model = load_baked_blockstate_model(
            resource_manager,
            texture_atlas,
            block_state,
            &mut blockstate_cache,
            &mut block_model_cache,
            &mut baked_variant_cache,
        )
        .or_else(|| manual_fluid_block_model(texture_atlas, block_state))
        .or_else(|| {
            if !should_install_missing_block_model_fallback(block_state) {
                return None;
            }
            let fallback_id = ResourceLocation::from_string("minecraft:block/dirt");
            cached_block_model(resource_manager, &mut block_model_cache, &fallback_id)
                .map(|fallback| BakedModel::bake(fallback, texture_atlas, 0, 0, false))
        });
        if let Some(mut model) = model {
            apply_vanilla_terrain_layer(block_state, &mut model);
            models.insert(
                render::mesher::ChunkMesher::block_state_model_key(block_state),
                model,
            );
        }
    }
    bake_stop.store(true, Ordering::Relaxed);
    #[cfg(not(target_arch = "wasm32"))]
    {
        heartbeat.thread().unpark();
        let _ = heartbeat.join();
    }
    resource_pack_debug(format!(
        "bake_model_cache phase=end states={} models={} blockstates_cached={} block_models_cached={} baked_variants_cached={}",
        all_block_states.len(),
        models.len(),
        blockstate_cache.len(),
        block_model_cache.len(),
        baked_variant_cache.len()
    ));
    Arc::new(models)
}
fn should_install_missing_block_model_fallback(state: &BlockState) -> bool {
    let path = state.block.path.as_str();
    !matches!(path, "water" | "lava")
        && !path.ends_with("chest")
        && !path.ends_with("shulker_box")
        && !path.ends_with("banner")
        && !path.ends_with("_head")
        && !path.ends_with("_wall_head")
        && !path.ends_with("_skull")
        && !path.ends_with("_wall_skull")
        && path != "conduit"
        && path != "decorated_pot"
        && !path.ends_with("copper_golem_statue")
}
fn apply_vanilla_terrain_layer(state: &BlockState, model: &mut BakedModel) {
    let layer = if render::mesher::ChunkMesher::block_uses_translucent_render_layer(
        state.block.path.as_str(),
    ) {
        render::chunk::ChunkSectionLayer::Translucent
    } else if vanilla_cutout_render_layer(state.block.path.as_str()) {
        render::chunk::ChunkSectionLayer::Cutout
    } else {
        render::chunk::ChunkSectionLayer::Solid
    };
    apply_vanilla_terrain_layer_to_model(model, layer);
}
fn apply_vanilla_terrain_layer_to_model(
    model: &mut BakedModel,
    layer: render::chunk::ChunkSectionLayer,
) {
    for quad in &mut model.quads {
        quad.layer = layer;
    }
    for group in &mut model.weighted_groups {
        match group {
            BakedModelGroup::Single(model) => apply_vanilla_terrain_layer_to_model(model, layer),
            BakedModelGroup::Weighted(weighted_models) => {
                for weighted in weighted_models {
                    apply_vanilla_terrain_layer_to_model(&mut weighted.model, layer);
                }
            }
        }
    }
}
fn vanilla_cutout_render_layer(path: &str) -> bool {
    path == "redstone_wire"
        || path == "tripwire"
        || path == "cobweb"
        || path == "bamboo_sapling"
        || path == "bamboo"
        || path == "scaffolding"
        || path == "chain"
        || path == "iron_bars"
        || path == "brewing_stand"
        || path == "lantern"
        || path == "soul_lantern"
        || path == "campfire"
        || path == "soul_campfire"
        || path == "torch"
        || path == "wall_torch"
        || path == "soul_torch"
        || path == "soul_wall_torch"
        || path == "redstone_torch"
        || path == "redstone_wall_torch"
        || path == "ladder"
        || path == "vine"
        || path == "weeping_vines"
        || path == "weeping_vines_plant"
        || path == "twisting_vines"
        || path == "twisting_vines_plant"
        || path == "kelp"
        || path == "kelp_plant"
        || path == "seagrass"
        || path == "tall_seagrass"
        || path == "sea_pickle"
        || path == "sugar_cane"
        || path == "cactus_flower"
        || path == "spore_blossom"
        || path == "pointed_dripstone"
        || path == "decorated_pot"
        || path == "lightning_rod"
        || path == "trial_spawner"
        || path == "vault"
        || path.ends_with("_sapling")
        || path.ends_with("_leaves")
        || path.ends_with("_door")
        || path.ends_with("_trapdoor")
        || path.ends_with("_rail")
        || path.ends_with("_coral")
        || path.ends_with("_coral_fan")
        || path.ends_with("_coral_wall_fan")
        || path.ends_with("_mushroom")
        || path.ends_with("_roots")
        || path.ends_with("_sprouts")
        || path.ends_with("_bush")
        || path.ends_with("_grass")
        || path.ends_with("_fern")
        || path.ends_with("_flower")
        || path.ends_with("_crop")
        || path.ends_with("_stem")
        || path.ends_with("_bars")
        || path.ends_with("_grate")
        || matches!(
            path,
            "grass"
                | "tall_grass"
                | "fern"
                | "large_fern"
                | "dead_bush"
                | "short_dry_grass"
                | "tall_dry_grass"
                | "wheat"
                | "carrots"
                | "potatoes"
                | "beetroots"
                | "nether_wart"
                | "melon_stem"
                | "pumpkin_stem"
                | "attached_melon_stem"
                | "attached_pumpkin_stem"
                | "brown_mushroom"
                | "red_mushroom"
                | "crimson_roots"
                | "warped_roots"
                | "nether_sprouts"
                | "azalea"
                | "flowering_azalea"
                | "leaf_litter"
                | "pink_petals"
                | "wildflowers"
                | "firefly_bush"
                | "glow_lichen"
                | "hanging_roots"
                | "big_dripleaf"
                | "big_dripleaf_stem"
                | "small_dripleaf"
                | "cave_vines"
                | "cave_vines_plant"
                | "pitcher_plant"
                | "pitcher_crop"
                | "torchflower"
                | "torchflower_crop"
                | "closed_eyeblossom"
                | "open_eyeblossom"
                | "chorus_flower"
                | "chorus_plant"
        )
}
fn manual_fluid_block_model(atlas: &TextureAtlas, state: &BlockState) -> Option<BakedModel> {
    let sprite = match state.block.path.as_str() {
        "water" => ResourceLocation::from_string("minecraft:block/water_still"),
        "lava" => ResourceLocation::from_string("minecraft:block/lava_still"),
        _ => return None,
    };
    let rect = atlas.sprites.get(&sprite).copied()?;
    let top_y = 0.875;
    let mut quads = Vec::new();
    push_manual_quad(
        &mut quads,
        el_math::Direction::Up,
        [
            glam::Vec3::new(0.0, top_y, 0.0),
            glam::Vec3::new(1.0, top_y, 0.0),
            glam::Vec3::new(1.0, top_y, 1.0),
            glam::Vec3::new(0.0, top_y, 1.0),
        ],
        rect,
        Some("up"),
    );
    push_manual_quad(
        &mut quads,
        el_math::Direction::Down,
        [
            glam::Vec3::new(0.0, 0.0, 1.0),
            glam::Vec3::new(1.0, 0.0, 1.0),
            glam::Vec3::new(1.0, 0.0, 0.0),
            glam::Vec3::new(0.0, 0.0, 0.0),
        ],
        rect,
        Some("down"),
    );
    for (direction, cullface, positions) in [
        (
            el_math::Direction::North,
            "north",
            [
                glam::Vec3::new(1.0, 0.0, 0.0),
                glam::Vec3::new(0.0, 0.0, 0.0),
                glam::Vec3::new(0.0, top_y, 0.0),
                glam::Vec3::new(1.0, top_y, 0.0),
            ],
        ),
        (
            el_math::Direction::South,
            "south",
            [
                glam::Vec3::new(0.0, 0.0, 1.0),
                glam::Vec3::new(1.0, 0.0, 1.0),
                glam::Vec3::new(1.0, top_y, 1.0),
                glam::Vec3::new(0.0, top_y, 1.0),
            ],
        ),
        (
            el_math::Direction::West,
            "west",
            [
                glam::Vec3::new(0.0, 0.0, 0.0),
                glam::Vec3::new(0.0, 0.0, 1.0),
                glam::Vec3::new(0.0, top_y, 1.0),
                glam::Vec3::new(0.0, top_y, 0.0),
            ],
        ),
        (
            el_math::Direction::East,
            "east",
            [
                glam::Vec3::new(1.0, 0.0, 1.0),
                glam::Vec3::new(1.0, 0.0, 0.0),
                glam::Vec3::new(1.0, top_y, 0.0),
                glam::Vec3::new(1.0, top_y, 1.0),
            ],
        ),
    ] {
        push_manual_quad(&mut quads, direction, positions, rect, Some(cullface));
    }
    if state.block.path == "water" {
        for quad in &mut quads {
            quad.tint_index = Some(0);
        }
    }
    Some(BakedModel {
        quads,
        weighted_groups: Vec::new(),
        weighted_group_random_mode: WeightedGroupRandomMode::Sequential,
        particle_sprite: Some(rect),
    })
}
fn push_manual_quad(
    quads: &mut Vec<BakedQuad>,
    direction: el_math::Direction,
    vertex_positions: [glam::Vec3; 4],
    rect: render::texture::SpriteUvRect,
    cullface: Option<&str>,
) {
    let vertex_uvs = [
        rect.map_model_uv(0.0, 16.0),
        rect.map_model_uv(16.0, 16.0),
        rect.map_model_uv(16.0, 0.0),
        rect.map_model_uv(0.0, 0.0),
    ];
    quads.push(BakedQuad {
        vertex_positions,
        model_uvs: [[0.0, 16.0], [16.0, 16.0], [16.0, 0.0], [0.0, 0.0]],
        vertex_uvs,
        direction,
        cullface: cullface.map(str::to_string),
        tint_index: None,
        layer: crate::render::chunk::ChunkSectionLayer::Translucent,
        shade: true,
        light_emission: 0,
        ambient_occlusion: false,
    });
}
fn load_baked_blockstate_model(
    resources: &ClientResourceManager,
    atlas: &TextureAtlas,
    state: &BlockState,
    blockstate_cache: &mut HashMap<ResourceLocation, Option<BlockStateDefinition>>,
    block_model_cache: &mut HashMap<ResourceLocation, Option<BlockModel>>,
    baked_variant_cache: &mut HashMap<(ResourceLocation, i32, i32, bool), BakedModel>,
) -> Option<BakedModel> {
    if let Some(definition) =
        cached_blockstate_definition(resources, blockstate_cache, &state.block)
    {
        let mut combined = BakedModel::empty();
        let (selected_groups, is_multipart_root) =
            definition.select_variant_groups_with_random_mode(state);
        if is_multipart_root {
            combined.weighted_group_random_mode = WeightedGroupRandomMode::SharedMultipartSeed;
        }
        for group in selected_groups {
            let is_weighted = matches!(group, VariantValue::Multiple(_));
            let variants = match group {
                VariantValue::Single(variant) => vec![variant],
                VariantValue::Multiple(variants) => variants,
            };
            let mut baked_group = Vec::new();
            for variant in variants {
                let model_id = block_model_location(&variant.model);
                let x = variant.x.unwrap_or(0);
                let y = variant.y.unwrap_or(0);
                let uvlock = variant.uvlock.unwrap_or(false);
                let cache_key = (model_id.clone(), x, y, uvlock);
                let baked = if let Some(baked) = baked_variant_cache.get(&cache_key) {
                    baked.clone()
                } else {
                    let Some(block_model) =
                        cached_block_model(resources, block_model_cache, &model_id)
                    else {
                        log::warn!("Failed to load blockstate model {}", model_id);
                        continue;
                    };
                    let baked = BakedModel::bake(block_model, atlas, x, y, uvlock);
                    baked_variant_cache.insert(cache_key, baked.clone());
                    baked
                };
                baked_group.push(WeightedBakedModel {
                    weight: variant.weight.unwrap_or(1).max(1) as u32,
                    model: baked,
                });
            }
            if !baked_group.is_empty() {
                if is_weighted {
                    combined
                        .weighted_groups
                        .push(BakedModelGroup::Weighted(baked_group));
                } else if let Some(single) = baked_group.into_iter().next() {
                    combined
                        .weighted_groups
                        .push(BakedModelGroup::Single(single.model));
                }
            }
        }
        if !combined.quads.is_empty() || !combined.weighted_groups.is_empty() {
            return Some(combined);
        }
    }
    let model_id = ResourceLocation::new(
        &state.block.namespace,
        &format!("block/{}", state.block.path),
    );
    cached_block_model(resources, block_model_cache, &model_id)
        .map(|model| BakedModel::bake(model, atlas, 0, 0, false))
        .filter(|model| !model.quads.is_empty())
}
fn cached_resource<'a, T>(
    cache: &'a mut HashMap<ResourceLocation, Option<T>>,
    id: &ResourceLocation,
    load: impl FnOnce() -> Option<T>,
) -> Option<&'a T> {
    cache.entry(id.clone()).or_insert_with(load).as_ref()
}
fn cached_blockstate_definition<'a>(
    resources: &ClientResourceManager,
    cache: &'a mut HashMap<ResourceLocation, Option<BlockStateDefinition>>,
    id: &ResourceLocation,
) -> Option<&'a BlockStateDefinition> {
    cached_resource(cache, id, || BlockStateDefinition::load(resources, id).ok())
}
fn cached_block_model<'a>(
    resources: &ClientResourceManager,
    cache: &'a mut HashMap<ResourceLocation, Option<BlockModel>>,
    id: &ResourceLocation,
) -> Option<&'a BlockModel> {
    cached_resource(cache, id, || BlockModel::load(resources, id).ok())
}
fn block_model_location(model: &str) -> ResourceLocation {
    let id = ResourceLocation::from_string(model);
    if id.path.contains('/') {
        id
    } else {
        ResourceLocation::new(&id.namespace, &format!("block/{}", id.path))
    }
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn web_startup_surface_size(
    configured: winit::dpi::PhysicalSize<u32>,
    live: winit::dpi::PhysicalSize<u32>,
) -> Option<winit::dpi::PhysicalSize<u32>> {
    if live.width > 0 && live.height > 0 && live != configured {
        Some(live)
    } else {
        None
    }
}
#[cfg(target_arch = "wasm32")]
pub mod web {
    use std::cell::RefCell;
    use std::io::Cursor;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;
    use winit::event::{Event, WindowEvent};
    use winit::event_loop::{ControlFlow, DeviceEvents, EventLoopBuilder};
    use winit::platform::web::{EventLoopExtWebSys, WindowBuilderExtWebSys};
    use winit::window::WindowBuilder;
    #[cfg(target_arch = "wasm32")]
    fn update_wasm_splash(progress: f32, status_text: &str) {
        use wasm_bindgen::JsCast;
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(bar) = doc.get_element_by_id("eltharn_gate-splash-progress-bar") {
                if let Ok(html_bar) = bar.dyn_into::<web_sys::HtmlElement>() {
                    let _ = html_bar
                        .style()
                        .set_property("width", &format!("{:.1}%", progress.clamp(0.0, 100.0)));
                }
            }
            if let Some(status) = doc.get_element_by_id("eltharn_gate-splash-status") {
                status.set_text_content(Some(status_text));
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    fn hide_wasm_splash() {
        use wasm_bindgen::JsCast;
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(splash) = doc.get_element_by_id("eltharn_gate-splash") {
                if let Ok(html_splash) = splash.dyn_into::<web_sys::HtmlElement>() {
                    let _ = html_splash.style().set_property("opacity", "0");
                    let splash_clone = html_splash.clone();
                    let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
                        let _ = splash_clone.set_attribute("hidden", "");
                    });
                    if let Some(win) = web_sys::window() {
                        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                            cb.as_ref().unchecked_ref(),
                            400,
                        );
                    }
                }
            }
        }
    }
    async fn load_assets_from_zip()
    -> Result<crate::resources::manager::ClientResourceManager, String> {
        use el_platform_api::{
            MemoryResourceStore, ResourceSnapshotRequest, ResourceStore, VirtualPath,
        };
        use wasm_bindgen::JsCast;
        let window = web_sys::window().ok_or("no window")?;
        let resp_value = JsFuture::from(window.fetch_with_str("assets.zip"))
            .await
            .map_err(|e| format!("fetch failed: {e:?}"))?;
        let resp: web_sys::Response = resp_value
            .dyn_into()
            .map_err(|_| "fetch response is not a Response")?;
        if !resp.ok() {
            return Err(format!("assets.zip HTTP {}", resp.status()));
        }
        update_wasm_splash(10.0, "Downloading assets.zip...");
        let ab_value = JsFuture::from(
            resp.array_buffer()
                .map_err(|e| format!("array_buffer() failed: {e:?}"))?,
        )
        .await
        .map_err(|e| format!("array_buffer await failed: {e:?}"))?;
        let array_buf: js_sys::ArrayBuffer =
            ab_value.dyn_into().map_err(|_| "body is not ArrayBuffer")?;
        let bytes = js_sys::Uint8Array::new(&array_buf).to_vec();
        log::info!("WASM: assets.zip downloaded ({} bytes)", bytes.len());
        update_wasm_splash(40.0, "Unpacking game resources...");
        let store = MemoryResourceStore::new();
        let cursor = Cursor::new(bytes);
        let mut archive =
            zip::ZipArchive::new(cursor).map_err(|e| format!("zip open failed: {e}"))?;
        let mut count = 0usize;
        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| format!("zip entry {i} failed: {e}"))?;
            if entry.is_dir() {
                continue;
            }
            let name = entry.name().to_string();
            let virtual_path_str = format!("assets/{name}");
            let Ok(vpath) = VirtualPath::new(&virtual_path_str) else {
                continue;
            };
            let mut buf = Vec::with_capacity(entry.size() as usize);
            use std::io::Read;
            if entry.read_to_end(&mut buf).is_ok() {
                store.insert(vpath, buf);
                count += 1;
            }
        }
        log::info!("WASM: unpacked {count} assets from zip");
        update_wasm_splash(60.0, "Building resource snapshot...");
        let snapshot = store
            .prepare_snapshot(ResourceSnapshotRequest::all())
            .await
            .map_err(|e| format!("snapshot failed: {e:?}"))?;
        crate::resources::manager::ClientResourceManager::from_prepared_snapshot(snapshot.as_ref())
            .map_err(|e| format!("resource manager init failed: {e:?}"))
    }
    #[wasm_bindgen]
    pub fn run_web_client(canvas_id: &str, username: &str, protocol: &str) {
        console_error_panic_hook::set_once();
        console_log::init_with_level(log::Level::Info).ok();
        let canvas_id = canvas_id.to_owned();
        let protocol = protocol.to_owned();
        let username_clean = if username.trim().is_empty() {
            "Player".to_string()
        } else {
            username.trim().to_string()
        };
        let dom_window = web_sys::window().expect("no browser window");
        let document = dom_window.document().expect("no document");
        let element = document
            .get_element_by_id(&canvas_id)
            .unwrap_or_else(|| panic!("canvas #{canvas_id} not found"));
        let canvas: web_sys::HtmlCanvasElement =
            element.dyn_into().expect("element is not a canvas");
        let event_loop = EventLoopBuilder::<()>::new()
            .build()
            .expect("failed to create event loop");
        let winit_window = std::sync::Arc::new(
            WindowBuilder::new()
                .with_canvas(Some(canvas))
                .build(&event_loop)
                .expect("failed to create window"),
        );
        let state: Rc<RefCell<Option<crate::client::state::State>>> = Rc::new(RefCell::new(None));
        {
            use wasm_bindgen::JsCast;
            let state_for_lock_change = state.clone();
            let on_lock_change =
                Closure::<dyn FnMut(web_sys::Event)>::new(move |_e: web_sys::Event| {
                    let is_locked = web_sys::window()
                        .and_then(|w| w.document())
                        .and_then(|d| d.pointer_lock_element())
                        .is_some();
                    if !is_locked {
                        if let Some(s) = state_for_lock_change.borrow().as_ref() {
                            s.notify_pointer_lock_lost();
                        }
                    }
                });
            let state_for_lock_error = state.clone();
            let on_lock_error =
                Closure::<dyn FnMut(web_sys::Event)>::new(move |_e: web_sys::Event| {
                    log::warn!("[WASM] pointerlockerror — pointer lock request was denied");
                    if let Some(s) = state_for_lock_error.borrow().as_ref() {
                        s.notify_pointer_lock_lost();
                    }
                });
            if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                let _ = document.add_event_listener_with_callback(
                    "pointerlockchange",
                    on_lock_change.as_ref().unchecked_ref(),
                );
                let _ = document.add_event_listener_with_callback(
                    "pointerlockerror",
                    on_lock_error.as_ref().unchecked_ref(),
                );
            }
            on_lock_change.forget();
            on_lock_error.forget();
        }
        {
            use wasm_bindgen::JsCast;
            let on_keydown = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
                move |e: web_sys::KeyboardEvent| {
                    let locked = web_sys::window()
                        .and_then(|w| w.document())
                        .and_then(|d| d.pointer_lock_element())
                        .is_some();
                    if locked {
                        e.prevent_default();
                    }
                },
            );
            if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                let _ = document.add_event_listener_with_callback(
                    "keydown",
                    on_keydown.as_ref().unchecked_ref(),
                );
            }
            on_keydown.forget();
        }
        let last_frame_ms: Rc<std::cell::Cell<f64>> = Rc::new(std::cell::Cell::new(
            web_sys::window()
                .and_then(|w| w.performance())
                .map(|p| p.now())
                .unwrap_or(0.0),
        ));
        let state_init = state.clone();
        let window_init = winit_window.clone();
        let last_frame_ms_init = last_frame_ms.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let resource_manager = match load_assets_from_zip().await {
                Ok(mgr) => {
                    log::info!("WASM: assets.zip loaded successfully");
                    mgr
                }
                Err(e) => {
                    log::error!(
                        "WASM: failed to load assets.zip: {e}; falling back to empty manager"
                    );
                    crate::resources::manager::ClientResourceManager::empty_for_wasm()
                }
            };
            #[cfg(target_arch = "wasm32")]
            update_wasm_splash(70.0, "Initializing Web Workers...");
            #[cfg(target_arch = "wasm32")]
            {
                const MESH_WORKERS: usize = 4;
                match wasm_bindgen_rayon::init_thread_pool(MESH_WORKERS).await {
                    Ok(_) => {
                        log::info!(
                            "WASM: rayon thread pool initialized with {MESH_WORKERS} workers — chunk meshing offloaded to Web Workers"
                        )
                    }
                    Err(e) => {
                        log::warn!(
                            "WASM: rayon thread pool init failed ({e:?}); chunk meshing will be synchronous (open DevTools → Application → Service Workers to check coi-serviceworker.js status)"
                        )
                    }
                }
            }
            #[cfg(target_arch = "wasm32")]
            update_wasm_splash(85.0, "Creating Graphics Pipeline & Shaders...");
            let mut s =
                crate::client::state::State::new(window_init, &resource_manager, false).await;
            #[cfg(feature = "extensions")]
            if crate::extension_launch::configured() {
                if let Err(error) = crate::extension_launch::finish_gpu(s.extension_launch_error())
                {
                    crate::extension_launch::browser_failure(&error);
                    hide_wasm_splash();
                    return;
                }
            }
            let offline_uuid = uuid::Uuid::new_v3(
                &uuid::Uuid::NAMESPACE_DNS,
                format!("OfflinePlayer:{username_clean}").as_bytes(),
            );
            s.gui_manager
                .configure_player_profile(username_clean.clone(), offline_uuid);
            s.gui_manager
                .configure_protocol_backend(crate::app::main_entry::ProtocolBackendData {
                    requested_version: protocol.clone(),
                    backend_path: None,
                });
            s.gui_manager.open_title_screen();
            #[cfg(feature = "shaderpacks")]
            if let Some(address) = crate::shaderpacks::take_quick_play() {
                s.gui_manager.start_quick_play_multiplayer(address);
            }
            {
                let live = s.window.inner_size();
                if let Some(size) = crate::web_startup_surface_size(s.size, live) {
                    log::warn!(
                        "[WASM] startup surface sync: configured={}x{} live={}x{}; resizing before first frame",
                        s.size.width,
                        s.size.height,
                        live.width,
                        live.height
                    );
                    s.resize(size);
                } else if live.width == 0 || live.height == 0 {
                    log::error!(
                        "[WASM] startup surface sync: live browser canvas size is {}x{}; keeping configured {}x{} until the first Resized event",
                        live.width,
                        live.height,
                        s.size.width,
                        s.size.height
                    );
                }
            }
            let now_ms = web_sys::window()
                .and_then(|w| w.performance())
                .map(|p| p.now())
                .unwrap_or(0.0);
            last_frame_ms_init.set(now_ms);
            *state_init.borrow_mut() = Some(s);
            log::info!("WASM: State initialized for '{username_clean}' — Title Screen active");
            #[cfg(target_arch = "wasm32")]
            {
                update_wasm_splash(100.0, "Ready!");
                hide_wasm_splash();
            }
        });
        let window_loop = winit_window.clone();
        let last_frame_ms_loop = last_frame_ms.clone();
        let performance = web_sys::window().and_then(|w| w.performance());
        event_loop.spawn(move |event, window_target| {
            window_target.set_control_flow(ControlFlow::Poll);
            window_target.listen_device_events(DeviceEvents::Always);
            match event {
                Event::DeviceEvent { event, .. } => {
                    if let winit::event::DeviceEvent::MouseMotion { delta } = event {
                        if let Some(s) = state.borrow_mut().as_mut() {
                            s.process_mouse_motion(delta.0, delta.1);
                        }
                    }
                }
                Event::WindowEvent { event, .. } => {
                    if let Some(s) = state.borrow_mut().as_mut() {
                        s.input(&event);
                    }
                    match event {
                        WindowEvent::CloseRequested => {
                            #[cfg(feature = "extensions")]
                            crate::modding::shutdown();
                            window_target.exit();
                        }
                        WindowEvent::Resized(new_size) => {
                            if let Some(s) = state.borrow_mut().as_mut() {
                                s.resize(new_size);
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            let now_ms = performance.as_ref().map(|p| p.now()).unwrap_or(0.0);
                            let dt = (((now_ms - last_frame_ms_loop.get()) / 1000.0) as f32)
                                .clamp(0.0001, 0.05);
                            last_frame_ms_loop.set(now_ms);
                            if let Some(s) = state.borrow_mut().as_mut() {
                                s.update(dt.max(0.0001));
                                match s.render() {
                                    Ok(_) => {}
                                    Err(
                                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                                    ) => {
                                        let size = s.size;
                                        s.resize(size);
                                    }
                                    Err(wgpu::SurfaceError::OutOfMemory) => {
                                        window_target.exit();
                                    }
                                    Err(e) => log::warn!("render error: {:?}", e),
                                }
                            }
                            let should_quit = state
                                .borrow_mut()
                                .as_mut()
                                .map(|s| s.take_requested_quit())
                                .unwrap_or(false);
                            if should_quit {
                                #[cfg(feature = "extensions")]
                                crate::modding::shutdown();
                                if let Some(document) = web_sys::window().and_then(|w| w.document())
                                {
                                    let _ = document.exit_pointer_lock();
                                    if let Some(canvas_elem) =
                                        document.get_element_by_id(&canvas_id)
                                    {
                                        let _ = canvas_elem.set_attribute("hidden", "true");
                                    }
                                    if let Some(launcher_elem) =
                                        document.get_element_by_id("eltharn_gate-launcher")
                                    {
                                        let _ = launcher_elem.remove_attribute("hidden");
                                    }
                                }
                                *state.borrow_mut() = None;
                                log::info!("WASM: Game quit requested — returned to launcher");
                                return;
                            }
                        }
                        _ => {}
                    }
                }
                Event::AboutToWait => {
                    window_loop.request_redraw();
                }
                _ => {}
            }
        });
    }
}
