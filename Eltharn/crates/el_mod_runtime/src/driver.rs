use crate::lifecycle::Phase;
#[derive(Default)]
pub struct Driver {
    active_world: Option<u64>,
    desired_world: Option<u64>,
    entering: Option<u64>,
    queued: u32,
    pending: Option<Phase>,
    closing: bool,
    terminated: bool,
    cancelled: bool,
    app_due: bool,
    settings_app: bool,
    last_was_app: bool,
}
impl Driver {
    pub fn world_is_current(&self) -> bool {
        self.active_world.is_some() && self.active_world == self.desired_world
    }
    pub fn observe_app(&mut self) {
        if !self.closing && !self.terminated && self.desired_world.is_none() {
            self.app_due = true;
        }
    }
    pub fn app_is_current(&self) -> bool {
        !self.closing
            && (self.desired_world.is_none() || (self.settings_app && self.world_is_current()))
    }
    pub fn set_settings_app(&mut self, active: bool) {
        let active = active && !self.closing && !self.terminated;
        self.settings_app = active;
        if !active && self.desired_world.is_some() {
            self.app_due = false;
        }
    }
    pub fn observe_settings_app(&mut self, active: bool) -> bool {
        self.set_settings_app(active);
        if self.settings_app {
            self.app_due = true;
        }
        false
    }
    pub fn observe(&mut self, ticks: u32, world: Option<u64>) -> Result<bool, String> {
        if self.closing || self.terminated {
            return Ok(false);
        }
        if world == Some(0) {
            return Err("World identity exhausted".into());
        }
        let changed = self.desired_world != world;
        if changed {
            self.desired_world = world;
            self.queued = 0;
            self.app_due = false;
            self.settings_app = false;
            self.last_was_app = false;
        }
        if world.is_some() {
            self.queued = self
                .queued
                .checked_add(ticks)
                .ok_or("Mod tick backlog overflow")?;
        }
        if self.queued > 10 {
            return Err("Mods cannot keep up: tick backlog exceeds 10".into());
        }
        let cancel = changed
            && matches!(self.pending, Some(Phase::Tick | Phase::AppTick))
            && !self.cancelled;
        if cancel {
            self.cancelled = true;
        }
        Ok(cancel)
    }
    pub fn next(&mut self) -> Option<Phase> {
        if self.pending.is_some() || self.terminated {
            return None;
        }
        let phase = if self.active_world != self.desired_world {
            if self.active_world.is_some() {
                Phase::WorldExit
            } else {
                self.entering = self.desired_world;
                Phase::WorldEnter
            }
        } else if self.closing {
            Phase::Shutdown
        } else if self.settings_app && self.app_due && (!self.last_was_app || self.queued == 0) {
            self.app_due = false;
            Phase::AppTick
        } else if self.active_world.is_some() && self.queued > 0 {
            self.queued -= 1;
            Phase::Tick
        } else if self.active_world.is_none() && self.app_due {
            self.app_due = false;
            Phase::AppTick
        } else {
            return None;
        };
        self.pending = Some(phase);
        self.last_was_app = phase == Phase::AppTick;
        Some(phase)
    }
    pub fn complete(&mut self, phase: Phase) -> Result<(), String> {
        if self.pending != Some(phase) {
            return Err("Foreign lifecycle response".into());
        }
        self.pending = None;
        self.cancelled = false;
        match phase {
            Phase::WorldEnter => self.active_world = self.entering.take(),
            Phase::WorldExit => self.active_world = None,
            Phase::Shutdown => self.terminated = true,
            _ => {}
        }
        Ok(())
    }
    pub fn next_batch(&mut self) -> Option<(Phase, u32)> {
        let phase = self.next()?;
        let count = if phase == Phase::Tick {
            1 + std::mem::take(&mut self.queued)
        } else {
            1
        };
        Some((phase, count))
    }
    pub fn close(&mut self) -> bool {
        self.closing = true;
        self.desired_world = None;
        self.queued = 0;
        self.app_due = false;
        self.settings_app = false;
        let cancel = matches!(self.pending, Some(Phase::Tick | Phase::AppTick)) && !self.cancelled;
        if cancel {
            self.cancelled = true;
        }
        cancel
    }
    pub fn is_terminated(&self) -> bool {
        self.terminated
    }
}
