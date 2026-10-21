#[cfg(feature = "legacy-wasm")]
use crate::native::{NativeActor, new_engine};
use crate::{
    broker::execute_job,
    lifecycle::{ActorState, Phase},
    policy::Policy,
    script::{ScriptActor, ScriptOutcome},
    session::{Completion, Controller, Definition, Diagnostic, ModuleCode},
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError},
    },
    thread::JoinHandle,
    time::Instant,
};
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub keybindings: crate::keybindings::Report,
    pub overlay: crate::overlay::Report,
    pub binary_resources: Vec<(usize, usize)>,
    pub hook_state: crate::hook_state::Report,
    pub app: crate::app::Report,
    pub config: BTreeMap<String, Option<crate::config::Status>>,
    pub epoch: u64,
    pub tick: u64,
    pub in_world: bool,
    pub actors: usize,
    pub live_instances: usize,
    pub states: Vec<ActorState>,
    pub diagnostics: Vec<Diagnostic>,
    pub resources: (usize, usize, usize, usize, usize),
    pub preparation_us: u64,
    pub jobs: Vec<crate::session::JobTrace>,
    pub content: crate::content::ContentReport,
    pub network: Vec<crate::server_types::NetworkOutput>,
}
pub struct NativeSession {
    config_store: Option<Box<dyn crate::config::Store>>,
    controller: Controller,
    actors: Vec<Option<Actor>>,
    preparation_us: u64,
}
enum Actor {
    #[cfg(feature = "legacy-wasm")]
    Wasm(NativeActor),
    Script(ScriptActor),
}
impl Actor {
    fn invoke(
        &mut self,
        context: crate::host::Context,
        snapshot: crate::host::Snapshot,
    ) -> Result<ScriptOutcome, String> {
        match self {
            Self::Script(actor) => actor.invoke(context, snapshot),
            #[cfg(feature = "legacy-wasm")]
            Self::Wasm(actor) => actor.invoke(context, snapshot).map(|o| ScriptOutcome {
                code: o.code,
                error: o.error,
                effects: o.effects,
                fuel_used: o.fuel_used,
                memory_bytes: o.memory_bytes,
            }),
        }
    }
}
impl NativeSession {
    pub fn prepare(definitions: Vec<Definition>, side: u32) -> Result<Self, String> {
        Self::prepare_with_config(definitions, side, None)
    }
    pub fn prepare_with_config(
        definitions: Vec<Definition>,
        side: u32,
        mut store: Option<Box<dyn crate::config::Store>>,
    ) -> Result<Self, String> {
        let start = Instant::now();
        let policy = Policy::default();
        let configs = crate::session::admit(&definitions, &policy)?;
        let mut controller = Controller::new(configs, side, policy.clone())?;
        let mut records = BTreeMap::new();
        for owner in controller.config_owners() {
            let namespace = crate::config::storage_owner(&owner, side)?;
            let record = store
                .as_mut()
                .ok_or("Configuration capabilities require a durable store")?
                .load(&namespace)?;
            records.insert(owner, record);
        }
        controller.initialize_config(records)?;
        #[cfg(feature = "legacy-wasm")]
        let engine = if definitions
            .iter()
            .any(|d| matches!(d.module, ModuleCode::Wasm(_)))
        {
            Some(new_engine(&policy)?)
        } else {
            None
        };
        let mut actors = Vec::new();
        for definition in definitions {
            let actor = match definition.module {
                #[cfg(feature = "legacy-wasm")]
                ModuleCode::Wasm(bytes) => NativeActor::new(
                    engine.as_ref().unwrap(),
                    &bytes,
                    definition.grants,
                    policy.clone(),
                )
                .map(Actor::Wasm),
                #[cfg(not(feature = "legacy-wasm"))]
                ModuleCode::Wasm(_) => Err(
                    "Legacy Wasm mods require the legacy-wasm build feature; use a source ZIP mod"
                        .into(),
                ),
                ModuleCode::Script(bundle) => {
                    ScriptActor::new(bundle, definition.grants, policy.clone()).map(Actor::Script)
                }
            }
            .map_err(|e| format!("{}: {e}", definition.key))?;
            actors.push(Some(actor));
        }
        let mut session = Self {
            config_store: store,
            controller,
            actors,
            preparation_us: 0,
        };
        session.phase(Phase::Init)?;
        session.preparation_us = start.elapsed().as_micros().min(u64::MAX as u128) as u64;
        Ok(session)
    }
    pub fn phase(&mut self, phase: Phase) -> Result<Report, String> {
        self.phase_interruptible(phase, None, None)
    }
    pub fn set_content_input(
        &mut self,
        frames: BTreeMap<String, el_mod_content::scene_types::InputFrame>,
    ) -> Result<(), String> {
        self.controller.set_content_input(frames)
    }
    pub fn set_server_input(
        &mut self,
        frames: BTreeMap<String, crate::server_types::ServerFrame>,
    ) -> Result<(), String> {
        self.controller.set_server_input(frames)
    }
    pub fn set_app_input(
        &mut self,
        frames: BTreeMap<String, crate::app::InputFrame>,
    ) -> Result<(), String> {
        self.controller.set_app_input(frames)
    }
    pub fn set_keybinding_input(
        &mut self,
        frames: BTreeMap<String, crate::keybindings::InputFrame>,
    ) -> Result<(), String> {
        self.controller.set_keybinding_input(frames)
    }
    pub fn drain_network(&mut self) -> Result<Vec<crate::server_types::NetworkOutput>, String> {
        self.controller.drain_network()
    }
    pub fn checkpoint_server_scene(
        &self,
    ) -> Result<el_mod_content::scene::SceneCheckpoint, String> {
        self.controller.checkpoint_server_scene()
    }
    pub fn restore_server_scene(
        &mut self,
        checkpoint: &el_mod_content::scene::SceneCheckpoint,
    ) -> Result<(), String> {
        self.controller.restore_server_scene(checkpoint)
    }
    fn phase_interruptible(
        &mut self,
        phase: Phase,
        stop: Option<&AtomicBool>,
        cancel: Option<(&AtomicU64, u64)>,
    ) -> Result<Report, String> {
        let result = self.run(phase, stop, cancel);
        if result.is_err() && self.controller.fatal().is_some() {
            self.actors.iter_mut().for_each(|a| *a = None);
        }
        result
    }
    fn run(
        &mut self,
        phase: Phase,
        stop: Option<&AtomicBool>,
        cancel: Option<(&AtomicU64, u64)>,
    ) -> Result<Report, String> {
        self.controller.begin(phase)?;
        loop {
            if matches!(phase, Phase::Tick | Phase::AppTick)
                && cancel.is_some_and(|(flag, seq)| flag.load(Ordering::Acquire) == seq)
            {
                self.controller.cancel_tick()?;
                self.release_disabled();
                return Ok(self.report());
            }
            if stop.is_some_and(|s| s.load(Ordering::Acquire)) {
                if matches!(phase, Phase::Tick | Phase::AppTick) {
                    self.controller.cancel_tick()?;
                }
                self.actors.iter_mut().for_each(|a| *a = None);
                return Err("Executor stopped".into());
            }
            let Some(inv) = self.controller.next()? else {
                break;
            };
            let actor = inv.request.actor;
            let started = Instant::now();
            let outcome = self.actors[actor]
                .as_mut()
                .ok_or("Missing live actor")?
                .invoke(inv.context, inv.snapshot);
            let elapsed_us = started.elapsed().as_micros().min(u64::MAX as u128) as u64;
            let completion = match outcome {
                Ok(o) => Completion {
                    request: inv.request,
                    success: o.success(),
                    error: o.error,
                    effects: o.effects,
                    fuel_used: Some(o.fuel_used),
                    memory_bytes: o.memory_bytes,
                    elapsed_us,
                },
                Err(e) => Completion {
                    request: inv.request,
                    success: false,
                    error: Some(e),
                    effects: Default::default(),
                    fuel_used: None,
                    memory_bytes: 0,
                    elapsed_us,
                },
            };
            self.controller.complete(completion)?;
            self.release_disabled();
        }
        self.release_disabled();
        for write in self.controller.config_writes() {
            let mut scoped = write.clone();
            scoped.owner = crate::config::storage_owner(&write.owner, self.controller.side())?;
            let result = self
                .config_store
                .as_mut()
                .ok_or("Configuration store unavailable")?
                .compare_and_set(&scoped);
            self.controller.acknowledge_config(&write, result)?;
        }
        for job in self.controller.take_jobs() {
            let result = execute_job(&job);
            self.controller.complete_job(&job, result)?;
        }
        let mut report = self.report();
        if self.controller.side() == 1 {
            report.network = self.controller.drain_client_network()?;
        }
        Ok(report)
    }
    fn release_disabled(&mut self) {
        for (i, a) in self.controller.scheduler().actors().iter().enumerate() {
            if matches!(a.state, ActorState::Disabled | ActorState::Terminated) {
                self.actors[i] = None;
            }
        }
    }
    pub fn report(&self) -> Report {
        let s = self.controller.scheduler();
        Report {
            keybindings: self.controller.broker().keybindings_report(),
            overlay: self.controller.broker().overlay_report(),
            hook_state: self.controller.broker().hook_state_report(),
            binary_resources: self.controller.broker().binary_resource_counts(),
            app: self.controller.broker().app_report(),
            config: self.controller.config_statuses(),
            epoch: s.epoch(),
            tick: s.tick(),
            in_world: s.in_world(),
            actors: self.actors.len(),
            live_instances: self.actors.iter().filter(|a| a.is_some()).count(),
            states: s.actors().iter().map(|a| a.state).collect(),
            diagnostics: self.controller.diagnostics().iter().cloned().collect(),
            resources: self.controller.broker().resource_counts(),
            preparation_us: self.preparation_us,
            jobs: self.controller.job_trace().iter().cloned().collect(),
            content: self.controller.broker().content_report(s.epoch() == 0),
            network: Vec::new(),
        }
    }
}
#[derive(Debug)]
pub struct Response {
    pub sequence: u64,
    pub phase: Phase,
    pub result: Result<Report, String>,
}
pub struct Executor {
    tx: SyncSender<(
        u64,
        Phase,
        u32,
        BTreeMap<String, el_mod_content::scene_types::InputFrame>,
        BTreeMap<String, crate::app::InputFrame>,
        BTreeMap<String, crate::keybindings::InputFrame>,
    )>,
    rx: Receiver<Response>,
    stop: Arc<AtomicBool>,
    cancel: Arc<AtomicU64>,
    thread: Option<JoinHandle<()>>,
    sequence: u64,
    pending: Option<(u64, Phase)>,
}
impl Executor {
    pub fn spawn(definitions: Vec<Definition>, side: u32) -> Result<Option<Self>, String> {
        Self::spawn_with_config(definitions, side, None)
    }
    pub fn spawn_with_config(
        definitions: Vec<Definition>,
        side: u32,
        store: Option<Box<dyn crate::config::Store>>,
    ) -> Result<Option<Self>, String> {
        if definitions.is_empty() {
            return Ok(None);
        }
        crate::session::admit(&definitions, &Policy::default())?;
        let (tx, commands) = mpsc::sync_channel::<(
            u64,
            Phase,
            u32,
            BTreeMap<String, el_mod_content::scene_types::InputFrame>,
            BTreeMap<String, crate::app::InputFrame>,
            BTreeMap<String, crate::keybindings::InputFrame>,
        )>(1);
        let (results, rx) = mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let cancel = Arc::new(AtomicU64::new(0));
        let worker_cancel = cancel.clone();
        let thread = std::thread::Builder::new()
            .name("eltharn-mods".into())
            .spawn(move || {
                let mut session = match NativeSession::prepare_with_config(definitions, side, store)
                {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = results.send(Response {
                            sequence: 0,
                            phase: Phase::Init,
                            result: Err(e),
                        });
                        return;
                    }
                };
                if results
                    .send(Response {
                        sequence: 0,
                        phase: Phase::Init,
                        result: Ok(session.report()),
                    })
                    .is_err()
                {
                    return;
                }
                while !worker_stop.load(Ordering::Acquire) {
                    let Ok((sequence, phase, count, input, app_input, keybinding_input)) =
                        commands.recv()
                    else {
                        break;
                    };
                    if worker_stop.load(Ordering::Acquire) {
                        break;
                    }
                    let mut staged = Ok(());
                    if !input.is_empty() {
                        staged = session.set_content_input(input);
                    }
                    if staged.is_ok() && !app_input.is_empty() {
                        staged = session.set_app_input(app_input);
                    }
                    if staged.is_ok() && !keybinding_input.is_empty() {
                        staged = session.set_keybinding_input(keybinding_input);
                    }
                    let mut pending_network = Vec::new();
                    let mut result = match staged {
                        Err(error) => Err(error),
                        Ok(()) => {
                            let mut result = session.phase_interruptible(
                                phase,
                                Some(&worker_stop),
                                Some((&worker_cancel, sequence)),
                            );
                            if let Ok(report) = &mut result {
                                pending_network.append(&mut report.network);
                            }
                            for _ in 1..count {
                                if result.is_err()
                                    || worker_cancel.load(Ordering::Acquire) == sequence
                                {
                                    break;
                                }
                                result = session.phase_interruptible(
                                    phase,
                                    Some(&worker_stop),
                                    Some((&worker_cancel, sequence)),
                                );
                                if let Ok(report) = &mut result {
                                    pending_network.append(&mut report.network);
                                }
                            }
                            result
                        }
                    };
                    if worker_cancel.load(Ordering::Acquire) == sequence {
                        pending_network.clear();
                    }
                    if let Ok(report) = &mut result {
                        report.network = pending_network;
                    }
                    let fatal = session.controller.fatal().is_some();
                    if results
                        .send(Response {
                            sequence,
                            phase,
                            result,
                        })
                        .is_err()
                        || fatal
                        || phase == Phase::Shutdown
                    {
                        break;
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Some(Self {
            tx,
            rx,
            stop,
            cancel,
            thread: Some(thread),
            sequence: 0,
            pending: Some((0, Phase::Init)),
        }))
    }
    pub fn request(&mut self, phase: Phase) -> Result<(), String> {
        self.request_batch(phase, 1)
    }
    pub fn request_batch(&mut self, phase: Phase, count: u32) -> Result<(), String> {
        self.request_batch_with_content(phase, count, BTreeMap::new())
    }
    pub fn request_batch_with_content(
        &mut self,
        phase: Phase,
        count: u32,
        input: BTreeMap<String, el_mod_content::scene_types::InputFrame>,
    ) -> Result<(), String> {
        self.request_inputs(phase, count, input, BTreeMap::new(), BTreeMap::new())
    }
    pub fn request_app(
        &mut self,
        input: BTreeMap<String, crate::app::InputFrame>,
    ) -> Result<(), String> {
        self.request_inputs(Phase::AppTick, 1, BTreeMap::new(), input, BTreeMap::new())
    }
    pub fn request_inputs(
        &mut self,
        phase: Phase,
        count: u32,
        input: BTreeMap<String, el_mod_content::scene_types::InputFrame>,
        app_input: BTreeMap<String, crate::app::InputFrame>,
        keybinding_input: BTreeMap<String, crate::keybindings::InputFrame>,
    ) -> Result<(), String> {
        if count == 0 || count > 10 || (phase != Phase::Tick && count != 1) {
            return Err("Invalid lifecycle batch size".into());
        }
        if self.pending.is_some() {
            return Err("Mod callback already in flight".into());
        }
        let sequence = self
            .sequence
            .checked_add(1)
            .ok_or("Executor sequence overflow")?;
        self.tx
            .try_send((sequence, phase, count, input, app_input, keybinding_input))
            .map_err(|e| e.to_string())?;
        self.sequence = sequence;
        self.pending = Some((sequence, phase));
        Ok(())
    }
    pub fn is_busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn cancel_tick(&self) {
        if let Some((sequence, Phase::Tick | Phase::AppTick)) = self.pending {
            self.cancel.store(sequence, Ordering::Release);
        }
    }
    pub fn poll(&mut self) -> Result<Option<Response>, String> {
        match self.rx.try_recv() {
            Ok(r) => {
                if self.pending != Some((r.sequence, r.phase)) {
                    return Err("Stale executor response".into());
                }
                self.pending = None;
                Ok(Some(r))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err("Mod executor stopped".into()),
        }
    }
}
impl Drop for Executor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.tx.try_send((
            self.sequence,
            Phase::Shutdown,
            1,
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeMap::new(),
        ));
        self.thread.take();
    }
}
