use crate::{
    broker::{Broker, Job},
    host::{Context, Effects, Snapshot},
    lifecycle::{ActorState, Phase, Request, Scheduler},
    policy::Policy,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone)]
pub struct Definition {
    pub key: String,
    pub dependencies: Vec<usize>,
    pub grants: Vec<String>,
    pub module: ModuleCode,
    pub required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ModuleCode {
    Wasm(Vec<u8>),
    Script(crate::script::ScriptBundle),
}

impl ModuleCode {
    pub fn memory_reservation(&self) -> Result<u64, String> {
        match self {
            Self::Wasm(bytes) => crate::policy::memory_reservation(bytes),
            Self::Script(bundle) => {
                bundle.validate()?;
                Ok(crate::script::MEMORY_RESERVATION)
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorConfig {
    pub key: String,
    pub dependencies: Vec<usize>,
    pub grants: Vec<String>,
    pub memory_max: u64,
    pub required: bool,
}
pub fn admit(definitions: &[Definition], policy: &Policy) -> Result<Vec<ActorConfig>, String> {
    policy.validate()?;
    if definitions.len() > policy.max_actors {
        return Err("Actor limit exceeded".into());
    }
    let mut total = 0u64;
    let mut configs = Vec::new();
    for d in definitions {
        let memory_max = d
            .module
            .memory_reservation()
            .map_err(|e| format!("{}: {e}", d.key))?;
        total = total
            .checked_add(memory_max)
            .ok_or("Memory reservation overflow")?;
        if total > policy.memory_total {
            return Err("Aggregate declared memory exceeds session budget".into());
        }
        configs.push(ActorConfig {
            key: d.key.clone(),
            dependencies: d.dependencies.clone(),
            grants: d.grants.clone(),
            memory_max,
            required: d.required,
        });
    }
    Ok(configs)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invocation {
    pub request: Request,
    pub context: Context,
    pub snapshot: Snapshot,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Completion {
    pub request: Request,
    pub success: bool,
    pub error: Option<String>,
    pub effects: Effects,
    pub fuel_used: Option<u64>,
    pub memory_bytes: u64,
    pub elapsed_us: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub actor: String,
    pub request: Request,
    pub success: bool,
    pub error: Option<String>,
    pub logs: Vec<String>,
    pub host_calls: usize,
    pub fuel_used: Option<u64>,
    pub memory_bytes: u64,
    pub elapsed_us: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct JobTrace {
    pub job: Job,
    pub status: String,
    pub delivery_tick: Option<u64>,
    pub result: Option<Vec<u8>>,
}
pub struct Controller {
    scheduler: Scheduler,
    broker: Broker,
    configs: Vec<ActorConfig>,
    side: u32,
    policy: Policy,
    snapshots: Vec<Option<Snapshot>>,
    staged: BTreeMap<usize, Effects>,
    phase: Option<Phase>,
    pending: Option<Request>,
    last_requests: Vec<Option<Request>>,
    jobs: Vec<Job>,
    diagnostics: VecDeque<Diagnostic>,
    fatal: Option<String>,
    job_trace: VecDeque<JobTrace>,
}
impl Controller {
    pub fn new(configs: Vec<ActorConfig>, side: u32, policy: Policy) -> Result<Self, String> {
        policy.validate()?;
        if !matches!(side, 1 | 2) {
            return Err("Invalid runtime side".into());
        }
        if configs.len() > policy.max_actors {
            return Err("Actor limit exceeded".into());
        }
        let mut sum = 0;
        for c in &configs {
            if side != 1
                && c.grants.iter().any(|g| {
                    matches!(
                        g.as_str(),
                        "content.client.read"
                            | "content.client.write"
                            | "network.client.send"
                            | "client.app.read"
                            | "client.app.write"
                            | crate::keybindings::CAPABILITY
                            | crate::hook_state::CAPABILITY
                    )
                })
            {
                return Err("Client content grants require the client runtime".into());
            }
            if side != 2
                && c.grants.iter().any(|g| {
                    matches!(
                        g.as_str(),
                        "content.server.read" | "content.server.write" | "network.server.send"
                    )
                })
            {
                return Err("Server content grants require the server runtime".into());
            }
            if c.memory_max > 64 * 1024 * 1024 || c.memory_max % 65536 != 0 {
                return Err("Invalid memory ceiling".into());
            }
            sum += c.memory_max;
            if c.grants.len() > 64
                || c.grants
                    .iter()
                    .any(|s| s.parse::<el_extension_api::CapabilityName>().is_err())
            {
                return Err("Invalid actor grants".into());
            }
        }
        if configs.len() > policy.max_actors || sum > policy.memory_total {
            return Err("Session resource limit".into());
        }
        let scheduler = Scheduler::new(
            configs
                .iter()
                .map(|c| (c.key.clone(), c.dependencies.clone()))
                .collect(),
        )?;
        let broker = Broker::new(
            configs.iter().map(|c| c.key.clone()).collect(),
            configs.iter().map(|c| c.grants.clone()).collect(),
        )?;
        let last_requests = vec![None; configs.len()];
        Ok(Self {
            scheduler,
            broker,
            configs,
            side,
            policy,
            snapshots: vec![],
            staged: BTreeMap::new(),
            phase: None,
            pending: None,
            last_requests,
            jobs: vec![],
            diagnostics: VecDeque::new(),
            fatal: None,
            job_trace: VecDeque::new(),
        })
    }
    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }
    pub fn side(&self) -> u32 {
        self.side
    }
    pub fn broker(&self) -> &Broker {
        &self.broker
    }
    pub fn initialize_config(
        &mut self,
        records: BTreeMap<String, crate::config::Record>,
    ) -> Result<(), String> {
        if self.phase.is_some()
            || self.pending.is_some()
            || self.last_requests.iter().any(Option::is_some)
        {
            return Err("Configuration must load before Init".into());
        }
        let expected: std::collections::BTreeSet<_> = self.config_owners().into_iter().collect();
        if records
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            != expected
        {
            return Err("Configuration owner roster mismatch".into());
        }
        self.broker.config.initialize(records)
    }
    pub fn config_owners(&self) -> Vec<String> {
        self.configs
            .iter()
            .filter(|c| {
                c.grants
                    .iter()
                    .any(|g| g == crate::config::READ_CAP || g == crate::config::WRITE_CAP)
            })
            .map(|c| c.key.clone())
            .collect()
    }
    pub fn config_writes(&self) -> Vec<crate::config::Write> {
        self.broker.config.pending()
    }
    pub fn config_statuses(&self) -> BTreeMap<String, Option<crate::config::Status>> {
        self.broker.config.statuses()
    }
    pub fn acknowledge_config(
        &mut self,
        write: &crate::config::Write,
        result: Result<crate::config::Receipt, String>,
    ) -> Result<(), String> {
        if self.phase.is_some() || self.pending.is_some() {
            return Err("Configuration persistence requires a completed phase".into());
        }
        self.broker.config.acknowledge(write, result)
    }
    pub fn set_content_input(
        &mut self,
        frames: BTreeMap<String, el_mod_content::scene_types::InputFrame>,
    ) -> Result<(), String> {
        if self.phase.is_some() || self.pending.is_some() || self.fatal.is_some() {
            return Err("Cannot change content input during a callback phase".into());
        }
        self.broker.set_content_input(frames)
    }
    pub fn diagnostics(&self) -> &VecDeque<Diagnostic> {
        &self.diagnostics
    }
    pub fn set_app_input(
        &mut self,
        frames: BTreeMap<String, crate::app::InputFrame>,
    ) -> Result<(), String> {
        if self.side != 1 || self.phase.is_some() || self.pending.is_some() || self.fatal.is_some()
        {
            return Err("App input requires an idle client".into());
        }
        if self.scheduler.in_world()
            && (frames.len() != 1
                || frames
                    .values()
                    .any(|frame| frame.menu != crate::app::SETTINGS_MENU))
        {
            return Err("In-world app input requires one selected settings owner".into());
        }
        self.broker.set_app_input(frames)
    }
    pub fn set_keybinding_input(
        &mut self,
        frames: BTreeMap<String, crate::keybindings::InputFrame>,
    ) -> Result<(), String> {
        if self.side != 1 || self.phase.is_some() || self.pending.is_some() || self.fatal.is_some()
        {
            return Err("Keybinding input requires an idle client".into());
        }
        self.broker.set_keybinding_input(frames)
    }
    pub fn set_server_input(
        &mut self,
        frames: BTreeMap<String, crate::server_types::ServerFrame>,
    ) -> Result<(), String> {
        self.require_server_idle()?;
        if !self.scheduler.in_world() {
            return Err("No server world".into());
        }
        self.broker.set_server_input(frames)
    }
    pub fn drain_network(&mut self) -> Result<Vec<crate::server_types::NetworkOutput>, String> {
        self.require_server_idle()?;
        Ok(self.broker.drain_network())
    }
    pub fn drain_client_network(
        &mut self,
    ) -> Result<Vec<crate::server_types::NetworkOutput>, String> {
        if self.side != 1 || self.phase.is_some() || self.pending.is_some() || self.fatal.is_some()
        {
            return Err("Operation requires an idle client runtime".into());
        }
        Ok(self.broker.drain_network())
    }
    pub fn checkpoint_server_scene(
        &self,
    ) -> Result<el_mod_content::scene::SceneCheckpoint, String> {
        self.require_server_idle()?;
        if !self.scheduler.in_world() {
            return Err("No server world".into());
        }
        self.broker.checkpoint_server_scene()
    }
    pub fn restore_server_scene(
        &mut self,
        checkpoint: &el_mod_content::scene::SceneCheckpoint,
    ) -> Result<(), String> {
        self.require_server_idle()?;
        if !self.scheduler.in_world() {
            return Err("No server world".into());
        }
        self.broker.restore_server_scene(checkpoint)
    }
    fn require_server_idle(&self) -> Result<(), String> {
        if self.side != 2 || self.phase.is_some() || self.pending.is_some() || self.fatal.is_some()
        {
            return Err("Operation requires an idle server runtime".into());
        }
        Ok(())
    }
    pub fn fatal(&self) -> Option<&str> {
        self.fatal.as_deref()
    }
    pub fn job_trace(&self) -> &VecDeque<JobTrace> {
        &self.job_trace
    }
    pub fn begin(&mut self, phase: Phase) -> Result<(), String> {
        if phase == Phase::AppTick && self.side != 1 {
            return Err("App callbacks require the client runtime".into());
        }
        if phase == Phase::AppTick
            && self.scheduler.in_world()
            && self.broker.app_report().menu != crate::app::SETTINGS_MENU
        {
            return Err("In-world app callbacks require selected settings input".into());
        }
        if !self.broker.config.pending().is_empty() {
            return Err("Configuration persistence is pending".into());
        }
        for owner in self.config_owners() {
            if self.broker.config.snapshot(&owner).is_none() {
                return Err("Configuration store unavailable".into());
            }
        }
        if self.fatal.is_some() {
            return Err(self.fatal.clone().unwrap());
        }
        self.scheduler.begin(phase)?;
        self.phase = Some(phase);
        self.staged.clear();
        self.jobs.clear();
        if phase == Phase::WorldEnter {
            self.broker.enter(self.scheduler.epoch())?;
        }
        if phase == Phase::Tick {
            loop {
                match self.broker.freeze_jobs() {
                    Ok(jobs) => {
                        for job in jobs {
                            self.trace_job(job, "delivered", Some(self.scheduler.tick() + 1), None);
                        }
                        break;
                    }
                    Err(e) => self.fail_actor(e.actor, &e.message)?,
                }
            }
        }
        self.snapshots = (0..self.configs.len())
            .map(|i| self.broker.snapshot(i).ok())
            .collect();
        Ok(())
    }
    pub fn next(&mut self) -> Result<Option<Invocation>, String> {
        if self.fatal.is_some() {
            return Err(self.fatal.clone().unwrap());
        }
        let phase = self.phase.ok_or("No active phase")?;
        match self.scheduler.next()? {
            Some(request) => {
                let context = Context {
                    actor: self.configs[request.actor].key.clone(),
                    side: self.side,
                    phase: request.phase,
                    epoch: request.epoch,
                    tick: request.tick,
                    callback_id: request.callback_id,
                };
                let snapshot = self.snapshots[request.actor]
                    .clone()
                    .ok_or("Missing actor snapshot")?;
                self.pending = Some(request.clone());
                Ok(Some(Invocation {
                    request,
                    context,
                    snapshot,
                }))
            }
            None => {
                loop {
                    match self.broker.commit(phase, &self.staged) {
                        Ok(jobs) => {
                            self.jobs = jobs;
                            break;
                        }
                        Err(e) => {
                            if phase == Phase::Init {
                                return self.abort(format!(
                                    "{}: {}",
                                    self.configs[e.actor].key, e.message
                                ));
                            }
                            self.fail_actor(e.actor, &e.message)?;
                            self.staged.retain(|&i, _| {
                                self.scheduler.actors()[i].state != ActorState::Disabled
                            });
                        }
                    }
                }
                self.phase = None;
                self.staged.clear();
                self.snapshots.clear();
                Ok(None)
            }
        }
    }
    pub fn complete(&mut self, mut reply: Completion) -> Result<Vec<usize>, String> {
        if let Some(e) = &self.fatal {
            return Err(e.clone());
        }
        if self.pending.as_ref() != Some(&reply.request) {
            return Err("Stale, foreign or duplicate completion".into());
        }
        let i = reply.request.actor;
        let e = &reply.effects;
        let fuel = if reply.request.phase == Phase::Init {
            self.policy.init_fuel
        } else {
            self.policy.callback_fuel
        };
        if reply.memory_bytes > self.configs[i].memory_max
            || reply.fuel_used.is_some_and(|n| n > fuel)
            || e.commands.len() > 64
            || e.calls > 128
            || e.logs.len() > 64
            || e.logs.iter().any(|s| s.len() > 4096)
            || e.logs.iter().map(String::len).sum::<usize>() > 65536
        {
            reply.success = false;
            reply.error = Some("Invalid or over-budget callback result".into());
            reply.effects = Effects::default();
        }
        if reply.error.is_some() {
            reply.success = false;
        }
        if !reply.success && reply.error.is_none() {
            reply.error = Some("Guest callback failed".into());
        }
        reply.error = reply.error.map(|s| s.chars().take(512).collect());
        let disabled = self.scheduler.complete(&reply.request, reply.success)?;
        self.pending = None;
        self.last_requests[i] = Some(reply.request.clone());
        self.diagnostics.push_back(Diagnostic {
            actor: self.configs[i].key.clone(),
            request: reply.request.clone(),
            success: reply.success,
            error: reply.error.clone(),
            logs: reply.effects.logs.clone(),
            host_calls: reply.effects.calls,
            fuel_used: reply.fuel_used,
            memory_bytes: reply.memory_bytes,
            elapsed_us: reply.elapsed_us,
        });
        self.trim_diagnostics();
        if reply.success {
            self.staged.insert(i, reply.effects);
        }
        self.broker.disable(&disabled);
        if let Some(&required) = disabled.iter().find(|&&n| self.configs[n].required) {
            return self.abort(format!(
                "Required mod {} disabled: {}",
                self.configs[required].key,
                reply.error.as_deref().unwrap_or("dependency failure")
            ));
        }
        Ok(disabled)
    }
    fn fail_actor(&mut self, actor: usize, message: &str) -> Result<(), String> {
        let disabled = self.scheduler.disable(actor)?;
        if disabled.is_empty() {
            return self.abort("Unable to discard invalid host effects".into());
        }
        self.broker.disable(&disabled);
        let request = self.last_requests[actor]
            .clone()
            .ok_or("Failure before any callback")?;
        self.diagnostics.push_back(Diagnostic {
            actor: self.configs[actor].key.clone(),
            request,
            success: false,
            error: Some(message.chars().take(512).collect()),
            logs: vec![],
            host_calls: 0,
            fuel_used: None,
            memory_bytes: 0,
            elapsed_us: 0,
        });
        self.trim_diagnostics();
        if disabled.iter().any(|&i| self.configs[i].required) {
            return self.abort(format!(
                "Required mod {}: {message}",
                self.configs[actor].key
            ));
        }
        Ok(())
    }
    fn abort<T>(&mut self, message: String) -> Result<T, String> {
        self.fatal = Some(message.chars().take(1024).collect());
        self.staged.clear();
        self.snapshots.clear();
        self.jobs.clear();
        self.pending = None;
        self.phase = None;
        self.broker.shutdown();
        Err(self.fatal.clone().unwrap())
    }
    fn trim_diagnostics(&mut self) {
        let cost = |d: &Diagnostic| {
            512 + d.actor.len()
                + d.error.as_ref().map_or(0, String::len)
                + d.logs.iter().map(String::len).sum::<usize>()
        };
        let mut bytes: usize = self.diagnostics.iter().map(cost).sum();
        while self.diagnostics.len() > 256 || bytes > 262144 {
            if let Some(d) = self.diagnostics.pop_front() {
                bytes = bytes.saturating_sub(cost(&d));
            } else {
                break;
            }
        }
    }
    pub fn cancel_tick(&mut self) -> Result<Vec<usize>, String> {
        if let Some(e) = &self.fatal {
            return Err(e.clone());
        }
        let app = self.phase == Some(Phase::AppTick);
        let disabled = self.scheduler.cancel_tick()?;
        self.pending = None;
        self.phase = None;
        self.staged.clear();
        self.snapshots.clear();
        self.jobs.clear();
        if app {
            self.broker.clear_app();
            self.broker.clear_keybinding_pressed();
        } else {
            self.broker.exit();
        }
        self.broker.disable(&disabled);
        if let Some(&actor) = disabled.iter().find(|&&i| self.configs[i].required) {
            return self.abort(format!(
                "Required mod {}: forced tick cancellation",
                self.configs[actor].key
            ));
        }
        Ok(disabled)
    }
    pub fn take_jobs(&mut self) -> Vec<Job> {
        std::mem::take(&mut self.jobs)
    }
    pub fn complete_job(&mut self, job: &Job, result: Vec<u8>) -> Result<(), String> {
        self.broker.complete_job(job, result.clone())?;
        self.trace_job(job.clone(), "completed", None, Some(result));
        Ok(())
    }
    fn trace_job(
        &mut self,
        job: Job,
        status: &str,
        delivery_tick: Option<u64>,
        result: Option<Vec<u8>>,
    ) {
        self.job_trace.push_back(JobTrace {
            job,
            status: status.into(),
            delivery_tick,
            result,
        });
        let mut bytes: usize = self.job_trace.iter().map(|t| t.job.data.len() + 512).sum();
        while self.job_trace.len() > 64 || bytes > 65536 {
            if let Some(t) = self.job_trace.pop_front() {
                bytes -= t.job.data.len() + 512;
            } else {
                break;
            }
        }
    }
}
