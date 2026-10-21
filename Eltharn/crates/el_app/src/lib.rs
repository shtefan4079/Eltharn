#![forbid(unsafe_code)]
use el_launcher_core::{
    GameLaunchConfig, LaunchProgress, LaunchRequest, LauncherFailure, LauncherFlow, TransitionError,
};
use el_platform_api::{SurfaceLifecycleEvent, SurfaceSize};
use std::fmt;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Boot,
    Launcher,
    Preparing,
    Game,
    Error,
}
impl AppMode {
    fn from_flow(flow: &LauncherFlow) -> Self {
        match flow {
            LauncherFlow::Boot => Self::Boot,
            LauncherFlow::Launcher => Self::Launcher,
            LauncherFlow::Preparing(_) => Self::Preparing,
            LauncherFlow::Game(_) => Self::Game,
            LauncherFlow::Error(_) => Self::Error,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct GameLifecycleSnapshot {
    pub resumed: bool,
    pub surface: Option<SurfaceSize>,
    pub surface_epoch: u64,
    pub low_memory_epoch: u64,
}
impl GameLifecycleSnapshot {
    pub fn can_render(&self) -> bool {
        self.resumed && self.surface.is_some()
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct GameEntry {
    pub config: GameLaunchConfig,
    pub lifecycle: GameLifecycleSnapshot,
}
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceLifecycleState {
    resumed: bool,
    surface: Option<SurfaceSize>,
    surface_epoch: u64,
    low_memory_epoch: u64,
}
impl Default for SurfaceLifecycleState {
    fn default() -> Self {
        Self {
            resumed: false,
            surface: None,
            surface_epoch: 0,
            low_memory_epoch: 0,
        }
    }
}
impl SurfaceLifecycleState {
    pub fn resumed(&self) -> bool {
        self.resumed
    }
    pub fn surface(&self) -> Option<SurfaceSize> {
        self.surface
    }
    pub fn surface_epoch(&self) -> u64 {
        self.surface_epoch
    }
    pub fn low_memory_epoch(&self) -> u64 {
        self.low_memory_epoch
    }
    pub fn snapshot(&self) -> GameLifecycleSnapshot {
        GameLifecycleSnapshot {
            resumed: self.resumed,
            surface: self.surface,
            surface_epoch: self.surface_epoch,
            low_memory_epoch: self.low_memory_epoch,
        }
    }
    fn apply(&mut self, event: SurfaceLifecycleEvent) -> LifecycleApplyResult {
        match event {
            SurfaceLifecycleEvent::Resumed if self.resumed => {
                LifecycleApplyResult::IgnoredDuplicate(event)
            }
            SurfaceLifecycleEvent::Resumed => {
                self.resumed = true;
                LifecycleApplyResult::Applied(event)
            }
            SurfaceLifecycleEvent::Suspended if !self.resumed => {
                LifecycleApplyResult::IgnoredDuplicate(event)
            }
            SurfaceLifecycleEvent::Suspended => {
                self.resumed = false;
                LifecycleApplyResult::Applied(event)
            }
            SurfaceLifecycleEvent::SurfaceAvailable(size) => {
                self.surface = Some(size);
                self.surface_epoch = self.surface_epoch.saturating_add(1);
                LifecycleApplyResult::Applied(event)
            }
            SurfaceLifecycleEvent::SurfaceResized(size) => {
                let Some(previous) = self.surface else {
                    return LifecycleApplyResult::IgnoredResizeWithoutSurface(size);
                };
                if previous == size {
                    LifecycleApplyResult::IgnoredDuplicate(event)
                } else {
                    self.surface = Some(size);
                    self.surface_epoch = self.surface_epoch.saturating_add(1);
                    LifecycleApplyResult::Applied(event)
                }
            }
            SurfaceLifecycleEvent::SurfaceLost if self.surface.is_none() => {
                LifecycleApplyResult::IgnoredDuplicate(event)
            }
            SurfaceLifecycleEvent::SurfaceLost => {
                self.surface = None;
                self.surface_epoch = self.surface_epoch.saturating_add(1);
                LifecycleApplyResult::Applied(event)
            }
            SurfaceLifecycleEvent::LowMemory => {
                self.low_memory_epoch = self.low_memory_epoch.saturating_add(1);
                LifecycleApplyResult::Applied(event)
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
enum LifecycleApplyResult {
    Applied(SurfaceLifecycleEvent),
    IgnoredDuplicate(SurfaceLifecycleEvent),
    IgnoredResizeWithoutSurface(SurfaceSize),
}
#[derive(Debug, Clone, PartialEq)]
pub enum LifecycleRoute {
    StoredForNextGame(SurfaceLifecycleEvent),
    DeliverToGame(SurfaceLifecycleEvent),
    IgnoredDuplicate(SurfaceLifecycleEvent),
    IgnoredResizeWithoutSurface(SurfaceSize),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppTransitionError {
    InvalidAppMode {
        operation: &'static str,
        current: AppMode,
    },
    Launcher(TransitionError),
}
impl fmt::Display for AppTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAppMode { operation, current } => {
                write!(
                    f,
                    "cannot {operation} while application mode is {current:?}"
                )
            }
            Self::Launcher(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for AppTransitionError {}
impl From<TransitionError> for AppTransitionError {
    fn from(value: TransitionError) -> Self {
        Self::Launcher(value)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct Application {
    flow: LauncherFlow,
    lifecycle: SurfaceLifecycleState,
}
impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}
impl Application {
    pub fn new() -> Self {
        Self {
            flow: LauncherFlow::Boot,
            lifecycle: SurfaceLifecycleState::default(),
        }
    }
    pub fn flow(&self) -> &LauncherFlow {
        &self.flow
    }
    pub fn mode(&self) -> AppMode {
        AppMode::from_flow(&self.flow)
    }
    pub fn lifecycle(&self) -> &SurfaceLifecycleState {
        &self.lifecycle
    }
    pub fn game_config(&self) -> Option<&GameLaunchConfig> {
        let LauncherFlow::Game(config) = &self.flow else {
            return None;
        };
        Some(config)
    }
    pub fn finish_boot(&mut self) -> Result<(), AppTransitionError> {
        if self.mode() != AppMode::Boot {
            return Err(AppTransitionError::InvalidAppMode {
                operation: "finish boot",
                current: self.mode(),
            });
        }
        self.flow.enter_launcher();
        Ok(())
    }
    pub fn begin_preparing(
        &mut self,
        request: LaunchRequest,
        total_steps: u32,
        detail: impl Into<String>,
    ) -> Result<(), AppTransitionError> {
        self.flow
            .begin_preparing(request, total_steps, detail)
            .map_err(Into::into)
    }
    pub fn update_preparation_progress(
        &mut self,
        progress: LaunchProgress,
    ) -> Result<(), AppTransitionError> {
        self.flow.update_progress(progress).map_err(Into::into)
    }
    pub fn complete_preparation(
        &mut self,
        config: GameLaunchConfig,
    ) -> Result<GameEntry, AppTransitionError> {
        self.flow
            .complete_preparation(config)
            .map_err(AppTransitionError::from)?;
        let config = self
            .game_config()
            .expect("LauncherFlow::complete_preparation always enters Game")
            .clone();
        Ok(GameEntry {
            config,
            lifecycle: self.lifecycle.snapshot(),
        })
    }
    pub fn fail_preparation(&mut self, failure: LauncherFailure) -> Result<(), AppTransitionError> {
        self.flow.fail_preparation(failure).map_err(Into::into)
    }
    pub fn cancel_preparation(&mut self) -> Result<(), AppTransitionError> {
        self.flow.cancel_preparation().map_err(Into::into)
    }
    pub fn recover_to_launcher(&mut self) -> Result<(), AppTransitionError> {
        self.flow.recover_to_launcher().map_err(Into::into)
    }
    pub fn route_lifecycle_event(&mut self, event: SurfaceLifecycleEvent) -> LifecycleRoute {
        match self.lifecycle.apply(event) {
            LifecycleApplyResult::IgnoredDuplicate(event) => {
                LifecycleRoute::IgnoredDuplicate(event)
            }
            LifecycleApplyResult::IgnoredResizeWithoutSurface(size) => {
                LifecycleRoute::IgnoredResizeWithoutSurface(size)
            }
            LifecycleApplyResult::Applied(event) if self.mode() == AppMode::Game => {
                LifecycleRoute::DeliverToGame(event)
            }
            LifecycleApplyResult::Applied(event) => LifecycleRoute::StoredForNextGame(event),
        }
    }
}
