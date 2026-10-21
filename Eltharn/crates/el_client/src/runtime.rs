#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
use crate::client::state::FramerateLimitContext;
use crate::client::state::State;
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
use std::time::Duration;
use web_time::Instant;
use winit::event::{DeviceEvent, WindowEvent};
#[cfg(target_os = "android")]
fn android_render_trace(message: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    if !android_trace_enabled() {
        return;
    }
    let Some(path) = crate::android_diagnostic_path() else {
        return;
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "render: {message}");
    }
}
#[cfg(target_os = "android")]
fn android_trace_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        cfg!(debug_assertions)
            || std::env::var("ELTHARN_ANDROID_LOG")
                .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
                .unwrap_or(false)
    })
}
#[cfg(not(target_os = "android"))]
fn android_render_trace(_message: &str) {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionAction {
    Continue,
    RequestRedraw,
    ExitRequested,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
enum FramerateThrottleReason {
    None,
    WindowIconified,
    LongAfk,
    ShortAfk,
    OutOfLevelMenu,
}
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
impl FramerateThrottleReason {
    fn profiler_label(self) -> &'static str {
        match self {
            Self::None => "configured",
            Self::WindowIconified => "window-iconified",
            Self::LongAfk => "long-afk",
            Self::ShortAfk => "short-afk",
            Self::OutOfLevelMenu => "out-of-level-menu",
        }
    }
}
#[derive(Debug)]
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
struct FramerateLimitTracker {
    latest_input_time: Option<Instant>,
}
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
impl FramerateLimitTracker {
    const OUT_OF_LEVEL_MENU_LIMIT: u32 = 60;
    const ICONIFIED_WINDOW_LIMIT: u32 = 10;
    const AFK_LIMIT: u32 = 30;
    const LONG_AFK_LIMIT: u32 = 10;
    const AFK_THRESHOLD: Duration = Duration::from_secs(60);
    const LONG_AFK_THRESHOLD: Duration = Duration::from_secs(600);
    fn new() -> Self {
        Self {
            latest_input_time: None,
        }
    }
    fn on_input_received(&mut self) {
        self.latest_input_time = Some(Instant::now());
    }
    fn throttle_reason_for_elapsed(
        context: FramerateLimitContext,
        window_iconified: bool,
        elapsed_since_input: Option<Duration>,
    ) -> FramerateThrottleReason {
        if window_iconified {
            return FramerateThrottleReason::WindowIconified;
        }
        if context.inactivity_afk_limit {
            let elapsed = elapsed_since_input.unwrap_or(Duration::MAX);
            if elapsed > Self::LONG_AFK_THRESHOLD {
                return FramerateThrottleReason::LongAfk;
            }
            if elapsed > Self::AFK_THRESHOLD {
                return FramerateThrottleReason::ShortAfk;
            }
        }
        if context.has_active_level || !context.has_screen_or_overlay {
            FramerateThrottleReason::None
        } else {
            FramerateThrottleReason::OutOfLevelMenu
        }
    }
    fn throttle_reason(
        &self,
        context: FramerateLimitContext,
        window_iconified: bool,
    ) -> FramerateThrottleReason {
        let elapsed_since_input = self
            .latest_input_time
            .map(|input_time| Instant::now().saturating_duration_since(input_time));
        Self::throttle_reason_for_elapsed(context, window_iconified, elapsed_since_input)
    }
    fn framerate_limit_with_reason(
        &self,
        context: FramerateLimitContext,
        window_iconified: bool,
    ) -> (u32, FramerateThrottleReason) {
        let reason = self.throttle_reason(context, window_iconified);
        let limit = match reason {
            FramerateThrottleReason::None => context.configured_limit,
            FramerateThrottleReason::WindowIconified => Self::ICONIFIED_WINDOW_LIMIT,
            FramerateThrottleReason::LongAfk => Self::LONG_AFK_LIMIT,
            FramerateThrottleReason::ShortAfk => context.configured_limit.min(Self::AFK_LIMIT),
            FramerateThrottleReason::OutOfLevelMenu => Self::OUT_OF_LEVEL_MENU_LIMIT,
        };
        (limit, reason)
    }
}
#[derive(Debug)]
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
struct FramePacer {
    last_frame_time: Instant,
    average_overshoot: Duration,
    last_framerate_limit: Option<u32>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
struct FramePacerOutcome {
    waited: Duration,
}
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
impl FramePacer {
    const OVERSHOOT_SMOOTHING_DENOMINATOR: u128 = 10;
    const MAX_CURRENT_OVERSHOOT: Duration = Duration::from_millis(25);
    const MAX_AVERAGE_OVERSHOOT: Duration = Duration::from_millis(2);
    const SPIN_SAFETY_BUFFER: Duration = Duration::from_micros(500);
    fn new() -> Self {
        Self {
            last_frame_time: Instant::now(),
            average_overshoot: Duration::ZERO,
            last_framerate_limit: None,
        }
    }
    fn target_frame_time(framerate_limit: u32) -> Duration {
        Duration::from_nanos(1_000_000_000 / u64::from(framerate_limit))
    }
    fn record_sleep_overshoot(&mut self, overshoot: Duration) {
        if overshoot.is_zero() || overshoot >= Self::MAX_CURRENT_OVERSHOOT {
            return;
        }
        let average_ns = self.average_overshoot.as_nanos();
        let current_ns = overshoot.as_nanos();
        let smoothed_ns = (current_ns + average_ns * 9) / Self::OVERSHOOT_SMOOTHING_DENOMINATOR;
        let capped_ns = smoothed_ns.min(Self::MAX_AVERAGE_OVERSHOOT.as_nanos());
        self.average_overshoot = Duration::from_nanos(capped_ns as u64);
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn limit_display_fps(&mut self, framerate_limit: u32) -> FramePacerOutcome {
        if framerate_limit >= 260 {
            return FramePacerOutcome::default();
        }
        if self.last_framerate_limit != Some(framerate_limit) {
            self.average_overshoot = Duration::ZERO;
            self.last_framerate_limit = Some(framerate_limit);
        }
        let wait_started_at = Instant::now();
        let target_time = self
            .last_frame_time
            .checked_add(Self::target_frame_time(framerate_limit))
            .unwrap_or_else(Instant::now);
        while let Some(remaining) = target_time.checked_duration_since(Instant::now()) {
            let sleep_threshold = self
                .average_overshoot
                .saturating_add(Self::SPIN_SAFETY_BUFFER);
            if remaining > sleep_threshold {
                let expected_sleep = remaining.saturating_sub(sleep_threshold);
                let sleep_started_at = Instant::now();
                std::thread::park_timeout(expected_sleep);
                self.record_sleep_overshoot(
                    Instant::now()
                        .saturating_duration_since(sleep_started_at)
                        .saturating_sub(expected_sleep),
                );
            } else {
                std::hint::spin_loop();
            }
        }
        self.last_frame_time = Instant::now();
        FramePacerOutcome {
            waited: self
                .last_frame_time
                .saturating_duration_since(wait_started_at),
        }
    }
}
pub struct DesktopGameSession {
    state: State,
    last_render_time: Instant,
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    framerate_limit_tracker: FramerateLimitTracker,
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    frame_pacer: FramePacer,
    about_to_wait_count: u64,
    render_count: u64,
    suspended: bool,
    #[cfg(target_os = "android")]
    android_surface_resumed: bool,
}
impl DesktopGameSession {
    pub fn new(state: State) -> Self {
        android_render_trace(&format!(
            "session_new window_inner={}x{} config={}x{}",
            state.window.inner_size().width,
            state.window.inner_size().height,
            state.config.width,
            state.config.height,
        ));
        Self {
            state,
            last_render_time: Instant::now(),
            #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
            framerate_limit_tracker: FramerateLimitTracker::new(),
            #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
            frame_pacer: FramePacer::new(),
            about_to_wait_count: 0,
            render_count: 0,
            suspended: false,
            #[cfg(target_os = "android")]
            android_surface_resumed: true,
        }
    }
    pub fn is_suspended(&self) -> bool {
        self.suspended
    }
    pub fn finish_performance_summary(&mut self) {
        self.state.finish_performance_summary();
    }
    pub fn on_suspended(&mut self) {
        android_render_trace("session_suspended");
        self.suspended = true;
        #[cfg(target_os = "android")]
        {
            self.android_surface_resumed = false;
            self.state.suspend_surface();
        }
        self.state.open_pause_screen();
    }
    pub fn on_resumed(&mut self) -> Result<(), String> {
        android_render_trace("session_resumed");
        #[cfg(target_os = "android")]
        {
            self.android_surface_resumed = true;
            if !self.state.resume_surface(self.state.window.inner_size())? {
                self.suspended = true;
                return Ok(());
            }
        }
        self.suspended = false;
        #[cfg(target_os = "android")]
        self.force_high_refresh_rate();
        self.state.resize(self.state.window.inner_size());
        self.last_render_time = Instant::now();
        Ok(())
    }
    #[cfg(target_os = "android")]
    fn force_high_refresh_rate(&self) {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use std::ffi::c_void;
        let Ok(handle) = self.state.window.window_handle() else {
            return;
        };
        let RawWindowHandle::AndroidNdk(ndk_handle) = handle.as_raw() else {
            return;
        };
        let window = ndk_handle.a_native_window.as_ptr().cast();
        android_render_trace("native_window_force_high_refresh");
        const FIXED_SOURCE: i8 = 1;
        unsafe {
            let symbol = libc::dlsym(
                libc::RTLD_DEFAULT,
                c"ANativeWindow_setFrameRate".as_ptr().cast(),
            );
            if symbol.is_null() {
                android_render_trace("native_window_setFrameRate symbol absent");
            } else {
                let set_frame_rate: unsafe extern "C" fn(*mut c_void, f32, i8) -> i32 =
                    std::mem::transmute(symbol);
                let result = set_frame_rate(window, 120.0, FIXED_SOURCE);
                android_render_trace(&format!("native_window_setFrameRate result={result}"));
            }
            let try_allocate = libc::dlsym(
                libc::RTLD_DEFAULT,
                c"ANativeWindow_tryAllocateBuffers".as_ptr().cast(),
            );
            if !try_allocate.is_null() {
                let f: unsafe extern "C" fn(*mut c_void) = std::mem::transmute(try_allocate);
                f(window);
            }
        }
    }
    pub fn on_device_event(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
            if delta.0 != 0.0 || delta.1 != 0.0 {
                self.framerate_limit_tracker.on_input_received();
            }
            self.state.process_mouse_motion(delta.0, delta.1);
        }
    }
    pub fn on_window_event(&mut self, event: &WindowEvent) -> SessionAction {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if matches!(
            event,
            WindowEvent::KeyboardInput { .. }
                | WindowEvent::MouseInput { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::CursorMoved { .. }
                | WindowEvent::DroppedFile(_)
        ) {
            self.framerate_limit_tracker.on_input_received();
        }
        if self.state.input(event) {
            return SessionAction::Continue;
        }
        match event {
            WindowEvent::CloseRequested => SessionAction::ExitRequested,
            WindowEvent::Resized(physical_size) => {
                android_render_trace(&format!(
                    "window_resized {}x{}",
                    physical_size.width, physical_size.height
                ));
                #[cfg(target_os = "android")]
                if self.android_surface_resumed && self.suspended {
                    match self.state.resume_surface(*physical_size) {
                        Ok(true) => {
                            self.suspended = false;
                            self.last_render_time = Instant::now();
                            self.force_high_refresh_rate();
                        }
                        Ok(false) => return SessionAction::Continue,
                        Err(error) => {
                            log::error!("Could not resume render surface: {error}");
                            return SessionAction::ExitRequested;
                        }
                    }
                }
                self.state.resize(*physical_size);
                SessionAction::Continue
            }
            WindowEvent::ScaleFactorChanged { .. } => SessionAction::Continue,
            WindowEvent::RedrawRequested => {
                if self.render_count < 8 || self.render_count.is_multiple_of(120) {
                    android_render_trace(&format!(
                        "redraw_requested count={} state={}x{} config={}x{}",
                        self.render_count,
                        self.state.size.width,
                        self.state.size.height,
                        self.state.config.width,
                        self.state.config.height,
                    ));
                }
                self.render_frame()
            }
            _ => SessionAction::Continue,
        }
    }
    pub fn on_about_to_wait(&mut self) -> SessionAction {
        self.about_to_wait_count = self.about_to_wait_count.saturating_add(1);
        if self.about_to_wait_count <= 8 || self.about_to_wait_count.is_multiple_of(120) {
            android_render_trace(&format!(
                "about_to_wait count={} render_count={}",
                self.about_to_wait_count, self.render_count
            ));
        }
        if self.state.take_requested_quit() {
            return SessionAction::ExitRequested;
        }
        self.state.finish_input_frame();
        SessionAction::RequestRedraw
    }
    fn render_frame(&mut self) -> SessionAction {
        if self.suspended {
            return SessionAction::Continue;
        }
        self.render_count = self.render_count.saturating_add(1);
        let render_number = self.render_count;
        if render_number <= 8 || render_number.is_multiple_of(120) {
            android_render_trace(&format!("render_begin count={render_number}"));
        }
        let now = Instant::now();
        let dt = now.duration_since(self.last_render_time).as_secs_f32();
        self.last_render_time = now;
        self.state.update(dt);
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        let (framerate_limit, framerate_throttle_reason) =
            self.framerate_limit_tracker.framerate_limit_with_reason(
                self.state.framerate_limit_context_like_java(),
                self.state.window.is_minimized().unwrap_or(false),
            );
        match self.state.render() {
            Ok(()) => {
                #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
                {
                    let outcome = self.frame_pacer.limit_display_fps(framerate_limit);
                    self.state.record_frame_pacing(
                        outcome.waited,
                        framerate_limit,
                        framerate_throttle_reason.profiler_label(),
                    );
                }
                if render_number <= 8 || render_number.is_multiple_of(120) {
                    android_render_trace(&format!("render_ok count={render_number}"));
                }
                SessionAction::Continue
            }
            Err(wgpu::SurfaceError::Lost) => {
                android_render_trace(&format!("render_surface_lost count={render_number}"));
                self.state.resize(self.state.size);
                SessionAction::Continue
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                android_render_trace(&format!("render_out_of_memory count={render_number}"));
                SessionAction::ExitRequested
            }
            Err(error) => {
                android_render_trace(&format!(
                    "render_error count={render_number} error={error:?}"
                ));
                log::error!("{:?}", error);
                SessionAction::Continue
            }
        }
    }
    pub fn render_now(&mut self) -> SessionAction {
        self.render_frame()
    }
}
