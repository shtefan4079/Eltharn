use crate::{GameLaunchConfig, LaunchProgress, LaunchRequest, LaunchStage};
use std::fmt;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherFailureKind {
    Account,
    Assets,
    Platform,
    Protocol,
    Transport,
    UserCancelled,
    Other,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherFailure {
    pub kind: LauncherFailureKind,
    pub message: String,
}
impl LauncherFailure {
    pub fn new(kind: LauncherFailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparationState {
    pub request: LaunchRequest,
    pub progress: LaunchProgress,
}
impl PreparationState {
    pub fn new(request: LaunchRequest, total_steps: u32, detail: impl Into<String>) -> Self {
        Self {
            request,
            progress: LaunchProgress::new(
                LaunchStage::ValidatingAccount,
                0,
                total_steps,
                detail.into(),
            ),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherFlow {
    Boot,
    Launcher,
    Preparing(PreparationState),
    Game(GameLaunchConfig),
    Error(LauncherFailure),
}
impl Default for LauncherFlow {
    fn default() -> Self {
        Self::Boot
    }
}
impl LauncherFlow {
    pub fn enter_launcher(&mut self) {
        *self = Self::Launcher;
    }
    pub fn begin_preparing(
        &mut self,
        request: LaunchRequest,
        total_steps: u32,
        detail: impl Into<String>,
    ) -> Result<(), TransitionError> {
        if !matches!(self, Self::Launcher) {
            return Err(TransitionError::InvalidSource {
                operation: "begin preparation",
                current: self.name(),
            });
        }
        *self = Self::Preparing(PreparationState::new(request, total_steps, detail));
        Ok(())
    }
    pub fn update_progress(&mut self, progress: LaunchProgress) -> Result<(), TransitionError> {
        let Self::Preparing(preparation) = self else {
            return Err(TransitionError::InvalidSource {
                operation: "update preparation progress",
                current: self.name(),
            });
        };
        if !is_preparation_stage(progress.stage) {
            return Err(TransitionError::InvalidProgressStage {
                stage: progress.stage,
            });
        }
        if preparation.progress.total_steps != 0
            && progress.total_steps != 0
            && progress.total_steps != preparation.progress.total_steps
        {
            return Err(TransitionError::ChangedProgressTotal {
                expected: preparation.progress.total_steps,
                found: progress.total_steps,
            });
        }
        if progress.completed_steps < preparation.progress.completed_steps {
            return Err(TransitionError::RegressedProgress {
                previous: preparation.progress.completed_steps,
                next: progress.completed_steps,
            });
        }
        if preparation_stage_rank(progress.stage)
            < preparation_stage_rank(preparation.progress.stage)
        {
            return Err(TransitionError::RegressedStage {
                previous: preparation.progress.stage,
                next: progress.stage,
            });
        }
        preparation.progress = progress;
        Ok(())
    }
    pub fn complete_preparation(
        &mut self,
        config: GameLaunchConfig,
    ) -> Result<(), TransitionError> {
        let Self::Preparing(preparation) = self else {
            return Err(TransitionError::InvalidSource {
                operation: "complete preparation",
                current: self.name(),
            });
        };
        if preparation.request.account_id != config.account_id {
            return Err(TransitionError::AccountMismatch);
        }
        if preparation.request.protocol_id != config.protocol_id {
            return Err(TransitionError::ProtocolMismatch);
        }
        *self = Self::Game(config);
        Ok(())
    }
    pub fn fail_preparation(&mut self, failure: LauncherFailure) -> Result<(), TransitionError> {
        if !matches!(self, Self::Preparing(_)) {
            return Err(TransitionError::InvalidSource {
                operation: "fail preparation",
                current: self.name(),
            });
        }
        *self = Self::Error(failure);
        Ok(())
    }
    pub fn cancel_preparation(&mut self) -> Result<(), TransitionError> {
        if !matches!(self, Self::Preparing(_)) {
            return Err(TransitionError::InvalidSource {
                operation: "cancel preparation",
                current: self.name(),
            });
        }
        *self = Self::Launcher;
        Ok(())
    }
    pub fn recover_to_launcher(&mut self) -> Result<(), TransitionError> {
        if !matches!(self, Self::Error(_) | Self::Game(_)) {
            return Err(TransitionError::InvalidSource {
                operation: "return to launcher",
                current: self.name(),
            });
        }
        *self = Self::Launcher;
        Ok(())
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::Boot => "Boot",
            Self::Launcher => "Launcher",
            Self::Preparing(_) => "Preparing",
            Self::Game(_) => "Game",
            Self::Error(_) => "Error",
        }
    }
}
fn is_preparation_stage(stage: LaunchStage) -> bool {
    matches!(
        stage,
        LaunchStage::ValidatingAccount
            | LaunchStage::ResolvingProtocol
            | LaunchStage::PreparingAssets
            | LaunchStage::StartingGame
    )
}
fn preparation_stage_rank(stage: LaunchStage) -> u8 {
    match stage {
        LaunchStage::ValidatingAccount => 0,
        LaunchStage::ResolvingProtocol => 1,
        LaunchStage::PreparingAssets => 2,
        LaunchStage::StartingGame => 3,
        LaunchStage::Idle | LaunchStage::Complete | LaunchStage::Failed => u8::MAX,
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionError {
    InvalidSource {
        operation: &'static str,
        current: &'static str,
    },
    InvalidProgressStage {
        stage: LaunchStage,
    },
    ChangedProgressTotal {
        expected: u32,
        found: u32,
    },
    RegressedProgress {
        previous: u32,
        next: u32,
    },
    RegressedStage {
        previous: LaunchStage,
        next: LaunchStage,
    },
    AccountMismatch,
    ProtocolMismatch,
}
impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSource { operation, current } => {
                write!(f, "cannot {operation} while launcher flow is {current}")
            }
            Self::InvalidProgressStage { stage } => {
                write!(f, "{stage:?} is not a preparation stage")
            }
            Self::ChangedProgressTotal { expected, found } => {
                write!(f, "preparation total changed from {expected} to {found}")
            }
            Self::RegressedProgress { previous, next } => {
                write!(
                    f,
                    "preparation progress regressed from {previous} to {next}"
                )
            }
            Self::RegressedStage { previous, next } => {
                write!(
                    f,
                    "preparation stage regressed from {previous:?} to {next:?}"
                )
            }
            Self::AccountMismatch => {
                f.write_str("prepared launch account does not match the active request")
            }
            Self::ProtocolMismatch => {
                f.write_str("prepared launch protocol does not match the active request")
            }
        }
    }
}
impl std::error::Error for TransitionError {}
