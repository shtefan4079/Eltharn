use el_app::{Application, LifecycleRoute};
use el_platform_api::{SurfaceLifecycleEvent, SurfaceSize};
#[cfg(target_os = "android")]
mod asset_download;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AndroidSurfaceMetrics {
    pub width: i32,
    pub height: i32,
    pub density_dpi: Option<u32>,
}
impl AndroidSurfaceMetrics {
    pub const fn new(width: i32, height: i32, density_dpi: Option<u32>) -> Self {
        Self {
            width,
            height,
            density_dpi,
        }
    }
    pub fn to_surface_size(self) -> Option<SurfaceSize> {
        let width = u32::try_from(self.width).ok().filter(|width| *width > 0)?;
        let height = u32::try_from(self.height)
            .ok()
            .filter(|height| *height > 0)?;
        let scale_factor = self
            .density_dpi
            .map(|dpi| f64::from(dpi) / 160.0)
            .filter(|scale| scale.is_finite() && *scale > 0.0)
            .unwrap_or(1.0);
        Some(SurfaceSize::new(width, height, scale_factor))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AndroidLifecycleInput {
    Resumed,
    Paused,
    SurfaceAvailable(AndroidSurfaceMetrics),
    SurfaceResized(AndroidSurfaceMetrics),
    ConfigurationChanged(AndroidSurfaceMetrics),
    SurfaceTerminating,
    LowMemory,
    Destroyed,
}
#[derive(Debug, Clone, PartialEq)]
pub enum AndroidLifecycleRoute {
    Routed(LifecycleRoute),
    IgnoredInvalidSurfaceMetrics(AndroidSurfaceMetrics),
    Destroyed {
        suspended: LifecycleRoute,
        surface_lost: LifecycleRoute,
    },
    IgnoredAfterDestroy(AndroidLifecycleInput),
}
#[derive(Debug)]
pub struct AndroidLifecycleHost {
    application: Application,
    destroyed: bool,
}
impl Default for AndroidLifecycleHost {
    fn default() -> Self {
        Self::new()
    }
}
impl AndroidLifecycleHost {
    pub fn new() -> Self {
        let mut application = Application::new();
        application
            .finish_boot()
            .expect("a new Android lifecycle host always starts at application boot");
        Self {
            application,
            destroyed: false,
        }
    }
    pub fn application(&self) -> &Application {
        &self.application
    }
    pub fn is_running(&self) -> bool {
        !self.destroyed
    }
    pub fn route(&mut self, input: AndroidLifecycleInput) -> AndroidLifecycleRoute {
        if self.destroyed {
            return AndroidLifecycleRoute::IgnoredAfterDestroy(input);
        }
        match input {
            AndroidLifecycleInput::Resumed => self.route_event(SurfaceLifecycleEvent::Resumed),
            AndroidLifecycleInput::Paused => self.route_event(SurfaceLifecycleEvent::Suspended),
            AndroidLifecycleInput::SurfaceAvailable(metrics) => {
                self.route_surface(metrics, SurfaceLifecycleEvent::SurfaceAvailable)
            }
            AndroidLifecycleInput::SurfaceResized(metrics)
            | AndroidLifecycleInput::ConfigurationChanged(metrics) => {
                self.route_surface(metrics, SurfaceLifecycleEvent::SurfaceResized)
            }
            AndroidLifecycleInput::SurfaceTerminating => {
                self.route_event(SurfaceLifecycleEvent::SurfaceLost)
            }
            AndroidLifecycleInput::LowMemory => self.route_event(SurfaceLifecycleEvent::LowMemory),
            AndroidLifecycleInput::Destroyed => {
                self.destroyed = true;
                let suspended = self
                    .application
                    .route_lifecycle_event(SurfaceLifecycleEvent::Suspended);
                let surface_lost = self
                    .application
                    .route_lifecycle_event(SurfaceLifecycleEvent::SurfaceLost);
                AndroidLifecycleRoute::Destroyed {
                    suspended,
                    surface_lost,
                }
            }
        }
    }
    fn route_surface(
        &mut self,
        metrics: AndroidSurfaceMetrics,
        make_event: impl FnOnce(SurfaceSize) -> SurfaceLifecycleEvent,
    ) -> AndroidLifecycleRoute {
        let Some(size) = metrics.to_surface_size() else {
            return AndroidLifecycleRoute::IgnoredInvalidSurfaceMetrics(metrics);
        };
        self.route_event(make_event(size))
    }
    fn route_event(&mut self, event: SurfaceLifecycleEvent) -> AndroidLifecycleRoute {
        AndroidLifecycleRoute::Routed(self.application.route_lifecycle_event(event))
    }
}
#[cfg(target_os = "android")]
fn validated_launch_username(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    (!trimmed.is_empty() && trimmed.chars().count() <= 16 && !trimmed.chars().any(char::is_control))
        .then_some(trimmed)
}
#[cfg(target_os = "android")]
mod game_activity {
    use super::{
        AndroidLifecycleHost, AndroidLifecycleInput, AndroidSurfaceMetrics,
        validated_launch_username,
    };
    use android_activity::{AndroidApp, MainEvent, PollEvent};
    use std::ffi::CString;
    #[cfg(feature = "shader-probe")]
    use std::io::Read;
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;
    #[derive(Debug, Clone)]
    struct AndroidLaunchRequest {
        protocol: String,
        username: String,
        controls: String,
        orientation: String,
    }
    static LAUNCH_REQUEST: OnceLock<Mutex<Option<AndroidLaunchRequest>>> = OnceLock::new();
    const ANDROID_LOG_INFO: i32 = 4;
    const ANDROID_LOG_ERROR: i32 = 6;
    unsafe extern "C" {
        fn __android_log_write(
            priority: i32,
            tag: *const std::ffi::c_char,
            text: *const std::ffi::c_char,
        ) -> i32;
    }
    fn android_log(priority: i32, message: impl AsRef<str>) {
        let tag = CString::new("Eltharn").expect("static Android log tag");
        let text = CString::new(message.as_ref()).unwrap_or_else(|_| {
            CString::new("Android log message contained an invalid NUL").unwrap()
        });
        unsafe {
            let _ = __android_log_write(priority, tag.as_ptr(), text.as_ptr());
        }
    }
    struct AndroidLogger;
    impl log::Log for AndroidLogger {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            metadata.level() <= log::Level::Info
        }
        fn log(&self, record: &log::Record<'_>) {
            if !self.enabled(record.metadata()) {
                return;
            }
            let priority = match record.level() {
                log::Level::Error => ANDROID_LOG_ERROR,
                log::Level::Warn => 5,
                log::Level::Info => ANDROID_LOG_INFO,
                log::Level::Debug => 3,
                log::Level::Trace => 2,
            };
            android_log(priority, format!("{}: {}", record.target(), record.args()));
        }
        fn flush(&self) {}
    }
    static ANDROID_LOGGER: AndroidLogger = AndroidLogger;
    fn trace(app: &AndroidApp, priority: i32, message: impl AsRef<str>) {
        let message = message.as_ref();
        android_log(priority, message);
        if let Some(directory) = app.external_data_path() {
            let _ = std::fs::create_dir_all(&directory);
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(directory.join("android-main.log"))
            {
                let _ = writeln!(file, "{message}");
            }
        }
    }
    fn trace_jni(message: impl AsRef<str>) {
        android_log(ANDROID_LOG_INFO, format!("jni: {}", message.as_ref()));
    }
    fn launch_request() -> &'static Mutex<Option<AndroidLaunchRequest>> {
        LAUNCH_REQUEST.get_or_init(|| Mutex::new(None))
    }
    fn finish_activity(
        env: &mut jni::JNIEnv<'_>,
        activity: &jni::objects::JObject<'_>,
        error: &str,
    ) -> jni::errors::Result<()> {
        if env.exception_check()? {
            env.exception_clear()?;
        }
        let error: String = error.chars().take(512).collect();
        let error = jni::objects::JObject::from(env.new_string(error)?);
        env.call_method(
            activity,
            "onNativeGameFinished",
            "(Ljava/lang/String;)V",
            &[jni::objects::JValue::Object(&error)],
        )?;
        Ok(())
    }
    fn finish_game_activity(app: &AndroidApp, error: &str) {
        let result = (|| -> jni::errors::Result<()> {
            let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) }?;
            let mut env = vm.attach_current_thread()?;
            env.with_local_frame(8, |env| -> jni::errors::Result<()> {
                let activity =
                    unsafe { jni::objects::JObject::from_raw(app.activity_as_ptr().cast()) };
                finish_activity(env, &activity, error)
            })
        })();
        if let Err(error) = result {
            android_log(
                ANDROID_LOG_ERROR,
                format!("Could not finish game activity: {error}"),
            );
        }
    }
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_eltharn_gate_EltharnGateGameActivity_nativeActivityDestroyed(
        mut env: jni::JNIEnv<'_>,
        activity: jni::objects::JObject<'_>,
    ) {
        el_client::clear_android_activity(&mut env, &activity);
    }
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_eltharn_gate_EltharnLauncherView_nativeTouchProfile(
        mut env: jni::JNIEnv<'_>,
        _class: jni::objects::JClass<'_>,
        value: jni::objects::JString<'_>,
    ) -> jni::sys::jstring {
        let result = env
            .get_string(&value)
            .ok()
            .and_then(|value| el_client::android_touch_profile_json(&value.to_string_lossy()).ok())
            .unwrap_or_default();
        env.new_string(result)
            .map(|value| value.into_raw())
            .unwrap_or(std::ptr::null_mut())
    }
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_eltharn_gate_EltharnLauncherView_nativeTouchPreview(
        mut env: jni::JNIEnv<'_>,
        _class: jni::objects::JClass<'_>,
        value: jni::objects::JString<'_>,
        width: jni::sys::jint,
        height: jni::sys::jint,
    ) -> jni::sys::jstring {
        let result = env
            .get_string(&value)
            .ok()
            .and_then(|value| {
                el_client::android_touch_preview_json(
                    &value.to_string_lossy(),
                    width as f32,
                    height as f32,
                )
                .ok()
            })
            .unwrap_or_default();
        env.new_string(result)
            .map(|value| value.into_raw())
            .unwrap_or(std::ptr::null_mut())
    }
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_eltharn_gate_EltharnGateGameActivity_nativeStartGame(
        mut env: jni::JNIEnv<'_>,
        activity: jni::objects::JObject<'_>,
        protocol: jni::objects::JString<'_>,
        username: jni::objects::JString<'_>,
        controls: jni::objects::JString<'_>,
        orientation: jni::objects::JString<'_>,
    ) {
        trace_jni("nativeStartGame called");
        let request = (|| -> Result<AndroidLaunchRequest, String> {
            let protocol = env
                .get_string(&protocol)
                .map_err(|_| "Could not read protocol request".to_string())?
                .to_string_lossy()
                .into_owned();
            let username = env
                .get_string(&username)
                .map_err(|_| "Could not read username request".to_string())?
                .to_string_lossy()
                .into_owned();
            let controls = env
                .get_string(&controls)
                .map_err(|_| "Could not read touch profile".to_string())?
                .to_string_lossy()
                .into_owned();
            let orientation = env
                .get_string(&orientation)
                .map_err(|_| "Could not read orientation request".to_string())?
                .to_string_lossy()
                .into_owned();
            if protocol.trim().is_empty()
                || protocol.len() > 128
                || protocol.chars().any(char::is_control)
            {
                return Err("Invalid protocol request".to_string());
            }
            let username = validated_launch_username(&username)
                .ok_or_else(|| "Invalid username request".to_string())?
                .to_owned();
            let controls = el_client::android_touch_profile_json(&controls)
                .map_err(|_| "Invalid touch control profile".to_string())?;
            if !matches!(orientation.as_str(), "landscape" | "auto") {
                return Err("Invalid orientation request".to_string());
            }
            Ok(AndroidLaunchRequest {
                protocol,
                username,
                controls,
                orientation,
            })
        })();
        let result = request.and_then(|request| {
            let mut pending = launch_request()
                .lock()
                .map_err(|_| "Game launch state is unavailable".to_string())?;
            *pending = Some(request);
            Ok(())
        });
        if let Err(error) = result {
            trace_jni(&error);
            if let Err(error) = finish_activity(&mut env, &activity, &error) {
                android_log(
                    ANDROID_LOG_ERROR,
                    format!("Could not finish rejected launch: {error}"),
                );
            }
        } else {
            trace_jni("validated request stored");
        }
    }
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_eltharn_gate_EltharnGateGameActivity_nativeMouseMotion(
        _env: jni::JNIEnv<'_>,
        _activity: jni::objects::JObject<'_>,
        dx: jni::sys::jdouble,
        dy: jni::sys::jdouble,
    ) {
        el_client::submit_android_mouse_motion(dx, dy);
    }
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_eltharn_gate_EltharnGateGameActivity_nativeMouseWheel(
        _env: jni::JNIEnv<'_>,
        _activity: jni::objects::JObject<'_>,
        x: jni::sys::jdouble,
        y: jni::sys::jdouble,
    ) {
        el_client::submit_android_mouse_wheel(x, y);
    }
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_eltharn_gate_EltharnGateGameActivity_nativeMouseButton(
        _env: jni::JNIEnv<'_>,
        _activity: jni::objects::JObject<'_>,
        button: jni::sys::jint,
        pressed: jni::sys::jboolean,
    ) {
        if button >= 0 {
            el_client::submit_android_mouse_button(button as u16, pressed != 0);
        }
    }
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_eltharn_gate_EltharnGateGameActivity_nativeTextInput(
        mut env: jni::JNIEnv<'_>,
        _activity: jni::objects::JObject<'_>,
        text: jni::objects::JString<'_>,
    ) {
        if let Ok(s) = env.get_string(&text) {
            let owned = s.to_string_lossy().into_owned();
            if !owned.is_empty() {
                el_client::submit_android_text_input(owned);
            }
        }
    }
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_eltharn_gate_EltharnGateGameActivity_nativeBackspace(
        _env: jni::JNIEnv<'_>,
        _activity: jni::objects::JObject<'_>,
    ) {
        el_client::submit_android_backspace();
    }
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_eltharn_gate_EltharnGateGameActivity_nativeChatEnter(
        _env: jni::JNIEnv<'_>,
        _activity: jni::objects::JObject<'_>,
    ) {
        el_client::submit_android_enter();
    }
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_eltharn_gate_EltharnGateGameActivity_nativeChatClose(
        _env: jni::JNIEnv<'_>,
        _activity: jni::objects::JObject<'_>,
    ) {
        el_client::submit_android_escape();
    }
    fn take_launch_request() -> Option<AndroidLaunchRequest> {
        launch_request().lock().ok()?.take()
    }
    fn start_game(app: &AndroidApp, request: AndroidLaunchRequest) -> Result<(), String> {
        trace(
            app,
            ANDROID_LOG_INFO,
            format!(
                "start_game protocol={} controls_bytes={} orientation={}",
                request.protocol,
                request.controls.len(),
                request.orientation
            ),
        );
        let internal_root = app
            .internal_data_path()
            .ok_or_else(|| "Android internal data path is unavailable".to_string())?
            .join("eltharn");
        let external_root = app.external_data_path().map(|p| p.join("eltharn"));
        let data_root = external_root
            .clone()
            .unwrap_or_else(|| internal_root.clone());
        let game_directory = data_root.join("game");
        let assets_root = data_root.join("assets");
        if external_root.is_some() {
            let old_game_dir = internal_root.join("game");
            if old_game_dir.exists() && old_game_dir != game_directory {
                let _ = std::fs::create_dir_all(&game_directory);
                for file_name in &["options.txt", "servers.dat", "command_history.txt"] {
                    let src = old_game_dir.join(file_name);
                    let dst = game_directory.join(file_name);
                    if src.is_file() && !dst.is_file() {
                        let _ = std::fs::copy(&src, &dst);
                    }
                }
            }
        }
        std::fs::create_dir_all(&game_directory).map_err(|error| error.to_string())?;
        std::fs::create_dir_all(game_directory.join("resourcepacks"))
            .map_err(|error| error.to_string())?;
        trace(
            app,
            ANDROID_LOG_INFO,
            format!(
                "game_directory={} assets_root={}",
                game_directory.display(),
                assets_root.display()
            ),
        );
        trace(
            app,
            ANDROID_LOG_INFO,
            format!("preparing downloadable assets in {}", assets_root.display()),
        );
        let asset_index = super::asset_download::prepare_assets(&assets_root)?;
        trace(
            app,
            ANDROID_LOG_INFO,
            format!("asset index {asset_index} is ready"),
        );
        #[cfg(feature = "shader-probe")]
        {
            let packet_path = internal_root.join("shader-probe/packet.json");
            let control_dir = internal_root.join("shader-probe/control");
            let file = std::fs::File::open(&packet_path).map_err(|error| {
                format!(
                    "shader probe packet open {}: {error}",
                    packet_path.display()
                )
            })?;
            let mut bytes = Vec::new();
            file.take(262145).read_to_end(&mut bytes).map_err(|error| {
                format!(
                    "shader probe packet read {}: {error}",
                    packet_path.display()
                )
            })?;
            if bytes.len() > 262144 {
                return Err(format!(
                    "shader probe packet too large {}: {} bytes",
                    packet_path.display(),
                    bytes.len()
                ));
            }
            el_client::shaderpacks::configure_native_probe(&bytes, control_dir.clone())
                .map_err(|error| format!("shader probe configure failed: {error}"))?;
            trace(
                app,
                ANDROID_LOG_INFO,
                format!("shader probe prepared control={}", control_dir.display()),
            );
        }
        trace(app, ANDROID_LOG_INFO, "assets ready; entering el_client");
        el_client::run_android_client(
            app.clone(),
            game_directory,
            assets_root,
            asset_index,
            request.username,
            request.protocol,
            request.controls,
        );
        trace(
            app,
            ANDROID_LOG_INFO,
            "el_client returned to Android entrypoint",
        );
        Ok(())
    }
    #[unsafe(no_mangle)]
    pub fn android_main(app: AndroidApp) {
        if log::set_logger(&ANDROID_LOGGER).is_ok() {
            log::set_max_level(log::LevelFilter::Info);
        }
        el_client::set_android_app(app.clone());
        trace(&app, ANDROID_LOG_INFO, "android_main entered");
        let mut host = AndroidLifecycleHost::new();
        let mut resumed = false;
        let mut pending_launch = None;
        let mut game_completed = false;
        while host.is_running() {
            app.poll_events(Some(Duration::from_millis(100)), |poll_event| {
                let PollEvent::Main(main_event) = poll_event else {
                    return;
                };
                match main_event {
                    MainEvent::Resume { .. } => {
                        resumed = true;
                        let _ = host.route(AndroidLifecycleInput::Resumed);
                    }
                    MainEvent::Pause => {
                        resumed = false;
                        let _ = host.route(AndroidLifecycleInput::Paused);
                    }
                    MainEvent::InitWindow { .. } => {
                        route_current_window(&app, &mut host, false);
                    }
                    MainEvent::WindowResized { .. } | MainEvent::ConfigChanged { .. } => {
                        route_current_window(&app, &mut host, true);
                    }
                    MainEvent::TerminateWindow { .. } => {
                        let _ = host.route(AndroidLifecycleInput::SurfaceTerminating);
                    }
                    MainEvent::LowMemory => {
                        let _ = host.route(AndroidLifecycleInput::LowMemory);
                    }
                    MainEvent::Destroy => {
                        el_client::clear_android_app(&app);
                        let _ = host.route(AndroidLifecycleInput::Destroyed);
                    }
                    _ => {}
                }
            });
            if !host.is_running() {
                break;
            }
            if game_completed {
                continue;
            }
            if pending_launch.is_none() {
                pending_launch = take_launch_request();
            }
            let window_ready = app
                .native_window()
                .is_some_and(|window| window.width() > 0 && window.height() > 0);
            if !resumed || !window_ready {
                continue;
            }
            if let Some(request) = pending_launch.take() {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    start_game(&app, request)
                }));
                let completion_error = match result {
                    Ok(Ok(())) => {
                        trace(&app, ANDROID_LOG_INFO, "game returned normally");
                        String::new()
                    }
                    Ok(Err(error)) => {
                        trace(
                            &app,
                            ANDROID_LOG_ERROR,
                            format!("game startup failed: {error}"),
                        );
                        error
                    }
                    Err(payload) => {
                        let detail = payload
                            .downcast_ref::<String>()
                            .map(String::as_str)
                            .or_else(|| payload.downcast_ref::<&str>().copied())
                            .unwrap_or("non-string panic payload");
                        trace(
                            &app,
                            ANDROID_LOG_ERROR,
                            format!("game startup panicked: {detail}"),
                        );
                        "Game startup failed; see application diagnostics".to_string()
                    }
                };
                game_completed = true;
                finish_game_activity(&app, &completion_error);
            }
        }
        trace(&app, ANDROID_LOG_INFO, "android_main exiting");
        el_client::clear_android_app(&app);
    }
    fn route_current_window(app: &AndroidApp, host: &mut AndroidLifecycleHost, resized: bool) {
        let Some(window) = app.native_window() else {
            return;
        };
        let metrics =
            AndroidSurfaceMetrics::new(window.width(), window.height(), app.config().density());
        let input = if resized {
            AndroidLifecycleInput::SurfaceResized(metrics)
        } else {
            AndroidLifecycleInput::SurfaceAvailable(metrics)
        };
        let _ = host.route(input);
    }
}
