use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Init,
    WorldEnter,
    Tick,
    AppTick,
    WorldExit,
    Shutdown,
}
impl Phase {
    pub fn export(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::WorldEnter => "on_world_enter",
            Self::Tick => "tick",
            Self::AppTick => "app_tick",
            Self::WorldExit => "on_world_exit",
            Self::Shutdown => "shutdown",
        }
    }
    pub fn code(self) -> u32 {
        match self {
            Self::Init => 1,
            Self::WorldEnter => 2,
            Self::Tick => 3,
            Self::WorldExit => 4,
            Self::Shutdown => 5,
            Self::AppTick => 6,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ActorState {
    Prepared,
    Ready,
    InWorld,
    Disabled,
    Terminated,
}
#[derive(Clone, Debug, Serialize)]
pub struct Actor {
    pub key: String,
    pub required_dependencies: Vec<usize>,
    pub state: ActorState,
    pub callback_id: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub actor: usize,
    pub phase: Phase,
    pub epoch: u64,
    pub tick: u64,
    pub callback_id: u64,
    pub sequence: u64,
}
#[derive(Debug)]
struct Batch {
    phase: Phase,
    order: Vec<usize>,
    cursor: usize,
    pending: Option<Request>,
    tick: u64,
}
#[derive(Debug)]
pub struct Scheduler {
    actors: Vec<Actor>,
    initialized: bool,
    in_world: bool,
    epoch: u64,
    tick: u64,
    sequence: u64,
    app_tick: u64,
    batch: Option<Batch>,
    terminated: bool,
    cancelled_tick: bool,
}
impl Scheduler {
    pub fn new(specs: Vec<(String, Vec<usize>)>) -> Result<Self, String> {
        if specs.len() > crate::policy::Policy::default().max_actors {
            return Err("Too many actors".into());
        }
        let mut keys = BTreeSet::new();
        let mut actors = Vec::new();
        for (rank, (key, deps)) in specs.into_iter().enumerate() {
            if key.is_empty() || key.len() > 160 || !keys.insert(key.clone()) {
                return Err("Invalid/duplicate actor key".into());
            }
            let unique: BTreeSet<_> = deps.iter().copied().collect();
            if unique.len() != deps.len() || deps.iter().any(|&d| d >= rank) {
                return Err("Dependency is not an earlier unique actor".into());
            }
            actors.push(Actor {
                key,
                required_dependencies: deps,
                state: ActorState::Prepared,
                callback_id: 0,
            });
        }
        Ok(Self {
            actors,
            initialized: false,
            in_world: false,
            epoch: 0,
            tick: 0,
            sequence: 0,
            app_tick: 0,
            batch: None,
            terminated: false,
            cancelled_tick: false,
        })
    }
    pub fn actors(&self) -> &[Actor] {
        &self.actors
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn is_busy(&self) -> bool {
        self.batch.is_some()
    }
    pub fn begin(&mut self, phase: Phase) -> Result<(), String> {
        if self.is_busy() || self.terminated {
            return Err("Session busy or terminated".into());
        }
        let allowed = match phase {
            Phase::Init => !self.initialized,
            Phase::WorldEnter => self.initialized && !self.in_world,
            Phase::Tick => self.in_world && !self.cancelled_tick,
            Phase::AppTick => self.initialized,
            Phase::WorldExit => self.in_world,
            Phase::Shutdown => self.initialized && !self.in_world,
        };
        if !allowed {
            return Err(format!("Wrong phase: {phase:?}"));
        }
        let epoch = if phase == Phase::WorldEnter {
            self.epoch.checked_add(1).ok_or("Epoch overflow")?
        } else {
            self.epoch
        };
        let tick = if phase == Phase::AppTick {
            self.app_tick.checked_add(1).ok_or("App tick overflow")?
        } else if phase == Phase::Tick {
            self.tick.checked_add(1).ok_or("Tick overflow")?
        } else {
            0
        };
        let mut order: Vec<_> = (0..self.actors.len()).collect();
        if matches!(phase, Phase::WorldExit | Phase::Shutdown) {
            order.reverse();
        }
        self.epoch = epoch;
        self.batch = Some(Batch {
            phase,
            order,
            cursor: 0,
            pending: None,
            tick,
        });
        Ok(())
    }
    pub fn next(&mut self) -> Result<Option<Request>, String> {
        let batch = self.batch.as_ref().ok_or("No active lifecycle batch")?;
        if batch.pending.is_some() {
            return Err("Callback already in flight".into());
        }
        loop {
            let b = self.batch.as_ref().unwrap();
            if b.cursor == b.order.len() {
                let b = self.batch.take().unwrap();
                match b.phase {
                    Phase::Init => self.initialized = true,
                    Phase::WorldEnter => {
                        self.in_world = true;
                        self.tick = 0;
                        self.cancelled_tick = false;
                    }
                    Phase::Tick => self.tick = b.tick,
                    Phase::AppTick => self.app_tick = b.tick,
                    Phase::WorldExit => {
                        self.in_world = false;
                        self.cancelled_tick = false;
                    }
                    Phase::Shutdown => self.terminated = true,
                }
                return Ok(None);
            }
            let rank = b.order[b.cursor];
            if matches!(
                self.actors[rank].state,
                ActorState::Disabled | ActorState::Terminated
            ) {
                self.batch.as_mut().unwrap().cursor += 1;
                continue;
            }
            if self.actors[rank]
                .required_dependencies
                .iter()
                .any(|&d| self.actors[d].state == ActorState::Disabled)
            {
                self.cascade(rank);
                continue;
            }
            let id = self.actors[rank]
                .callback_id
                .checked_add(1)
                .ok_or("Callback ID overflow")?;
            let seq = self.sequence.checked_add(1).ok_or("Sequence overflow")?;
            let b = self.batch.as_mut().unwrap();
            let request = Request {
                actor: rank,
                phase: b.phase,
                epoch: self.epoch,
                tick: b.tick,
                callback_id: id,
                sequence: seq,
            };
            self.actors[rank].callback_id = id;
            self.sequence = seq;
            b.cursor += 1;
            b.pending = Some(request.clone());
            return Ok(Some(request));
        }
    }
    pub fn complete(&mut self, request: &Request, success: bool) -> Result<Vec<usize>, String> {
        let b = self.batch.as_mut().ok_or("Stale callback: no batch")?;
        if b.pending.as_ref() != Some(request) {
            return Err("Stale, foreign or duplicate callback".into());
        }
        b.pending = None;
        if !success {
            return Ok(self.cascade(request.actor));
        }
        self.actors[request.actor].state = match request.phase {
            Phase::AppTick if self.in_world => ActorState::InWorld,
            Phase::Init | Phase::WorldExit | Phase::AppTick => ActorState::Ready,
            Phase::WorldEnter | Phase::Tick => ActorState::InWorld,
            Phase::Shutdown => ActorState::Terminated,
        };
        Ok(Vec::new())
    }
    pub fn cancel_tick(&mut self) -> Result<Vec<usize>, String> {
        if !self
            .batch
            .as_ref()
            .is_some_and(|b| matches!(b.phase, Phase::Tick | Phase::AppTick))
        {
            return Err("No tick to cancel".into());
        }
        let b = self.batch.take().unwrap();
        self.cancelled_tick = b.phase == Phase::Tick;
        Ok(b.pending.map(|p| self.cascade(p.actor)).unwrap_or_default())
    }
    pub fn disable(&mut self, rank: usize) -> Result<Vec<usize>, String> {
        if rank >= self.actors.len() {
            return Err("Unknown actor".into());
        }
        if self.batch.as_ref().is_some_and(|b| b.pending.is_some()) {
            return Err("Complete pending callback first".into());
        }
        Ok(self.cascade(rank))
    }
    fn cascade(&mut self, rank: usize) -> Vec<usize> {
        let mut disabled = BTreeSet::from([rank]);
        for i in rank..self.actors.len() {
            if self.actors[i]
                .required_dependencies
                .iter()
                .any(|d| disabled.contains(d))
            {
                disabled.insert(i);
            }
        }
        disabled
            .into_iter()
            .filter(|&i| {
                if matches!(
                    self.actors[i].state,
                    ActorState::Disabled | ActorState::Terminated
                ) {
                    false
                } else {
                    self.actors[i].state = ActorState::Disabled;
                    true
                }
            })
            .collect()
    }
}
