use crate::{
    host::{Command, Effects, Message, Service, Snapshot},
    lifecycle::Phase,
    policy::MAX_PAYLOAD_BYTES,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub owner: usize,
    pub epoch: u64,
    pub sequence: u64,
    pub token: u64,
    pub data: Vec<u8>,
}
#[derive(Clone)]
struct PendingJob {
    job: Job,
    result: Option<Vec<u8>>,
}
#[derive(Clone)]
struct PendingCall {
    provider: usize,
    service: String,
    delivered: bool,
}
#[derive(Clone)]
pub struct Broker {
    keybindings: crate::keybindings::State,
    overlay: crate::overlay::State,
    binary: Vec<crate::binary::Store>,
    hook_state: crate::hook_state::Store,
    app: crate::app::State,
    pub(crate) config: crate::config::State,
    keys: Vec<String>,
    grants: Vec<BTreeSet<String>>,
    alive: Vec<bool>,
    epoch: u64,
    services: BTreeMap<String, Service>,
    subscriptions: Vec<BTreeSet<String>>,
    inboxes: Vec<Vec<Message>>,
    calls: BTreeMap<(usize, u64), PendingCall>,
    jobs: BTreeMap<u64, PendingJob>,
    last_call: Vec<u64>,
    last_job: Vec<u64>,
    sequence: u64,
    content: crate::content::ContentEngine,
    server_input: BTreeMap<String, crate::server_types::ServerFrame>,
    server_sequence: u64,
    network_output: Vec<crate::server_types::NetworkOutput>,
}
#[derive(Debug)]
pub struct CommitError {
    pub actor: usize,
    pub message: String,
}
impl Broker {
    pub fn new(keys: Vec<String>, grants: Vec<Vec<String>>) -> Result<Self, String> {
        let n = keys.len();
        if n > 16
            || n != grants.len()
            || keys.iter().any(|k| k.is_empty() || k.len() > 160)
            || keys.iter().collect::<BTreeSet<_>>().len() != n
        {
            return Err("Invalid broker roster".into());
        }
        Ok(Self {
            keybindings: crate::keybindings::State::default(),
            overlay: crate::overlay::State::default(),
            binary: keys
                .iter()
                .map(|key| crate::binary::Store::new(key))
                .collect(),
            hook_state: crate::hook_state::Store::default(),
            app: crate::app::State::default(),
            config: crate::config::State::default(),
            keys,
            grants: grants
                .into_iter()
                .map(|g| g.into_iter().collect())
                .collect(),
            alive: vec![true; n],
            epoch: 0,
            services: BTreeMap::new(),
            subscriptions: vec![BTreeSet::new(); n],
            inboxes: vec![vec![]; n],
            calls: BTreeMap::new(),
            jobs: BTreeMap::new(),
            last_call: vec![0; n],
            last_job: vec![0; n],
            sequence: 0,
            content: crate::content::ContentEngine::default(),
            server_input: BTreeMap::new(),
            server_sequence: 0,
            network_output: Vec::new(),
        })
    }
    pub fn enter(&mut self, epoch: u64) -> Result<(), String> {
        if epoch <= self.epoch {
            return Err("Stale world epoch".into());
        }
        self.app.clear();
        self.keybindings.clear_pressed();
        self.overlay.clear();
        self.hook_state.clear();
        self.binary.iter_mut().for_each(crate::binary::Store::clear);
        if self.content.has_catalog()
            || self.grants.iter().any(|g| {
                g.iter().any(|c| {
                    matches!(
                        c.as_str(),
                        "content.register"
                            | "content.client.read"
                            | "content.client.write"
                            | "content.server.read"
                            | "content.server.write"
                            | "client.camera.read"
                            | "client.player.read"
                            | "client.entities.read"
                            | "client.avatar.read"
                            | "client.avatar.write"
                            | "client.camera.write"
                            | "client.canvas.write"
                            | "world.blocks.read"
                    )
                })
            })
        {
            self.content.enter(epoch)?;
        }
        self.epoch = epoch;
        Ok(())
    }
    pub fn snapshot(&self, actor: usize) -> Result<Snapshot, String> {
        if actor >= self.keys.len() || !self.alive[actor] {
            return Err("Unknown or disabled actor".into());
        }
        Ok(Snapshot {
            keybindings: self.grants[actor]
                .contains(crate::keybindings::CAPABILITY)
                .then(|| self.keybindings.snapshot(&self.keys[actor])),
            binary: self.grants[actor]
                .contains(crate::binary::CAPABILITY)
                .then(|| self.binary[actor].clone()),
            app: self.grants[actor]
                .iter()
                .any(|g| g == crate::app::READ_CAP || g == crate::app::WRITE_CAP)
                .then(|| self.app.snapshot(&self.keys[actor])),
            config: self.config.snapshot(&self.keys[actor]),
            services: self.services.values().cloned().collect(),
            inbox: self.inboxes[actor].clone(),
            content: (self.grants[actor].contains("content.client.read")
                || self.grants[actor].contains("content.server.read")
                || self.grants[actor].contains("client.camera.read")
                || self.grants[actor].contains("client.player.read")
                || self.grants[actor].contains("client.entities.read")
                || self.grants[actor].contains("client.avatar.read")
                || self.grants[actor].contains("client.avatar.write")
                || self.grants[actor].contains("world.blocks.read"))
            .then(|| self.content.snapshot(&self.keys[actor])),
            server: (self.grants[actor].contains("content.server.read")
                || self.grants[actor].contains("network.server.send"))
            .then(|| {
                self.server_input
                    .get(&self.keys[actor])
                    .cloned()
                    .unwrap_or_default()
            }),
        })
    }
    pub fn hook_state_report(&self) -> crate::hook_state::Report {
        self.hook_state.report()
    }
    pub fn app_report(&self) -> crate::app::Report {
        crate::app::Report {
            context: self
                .app
                .inputs
                .values()
                .next()
                .map(|f| f.context)
                .unwrap_or(0),
            menu: self
                .app
                .inputs
                .values()
                .next()
                .map(|f| f.menu.clone())
                .unwrap_or_default(),
            readers: self
                .keys
                .iter()
                .enumerate()
                .filter(|(i, _)| {
                    self.alive[*i]
                        && self.grants[*i]
                            .iter()
                            .any(|g| g == crate::app::READ_CAP || g == crate::app::WRITE_CAP)
                })
                .map(|(_, key)| key.clone())
                .collect(),
            scene: self.app.scene.clone(),
            revisions: self.app.revisions.clone(),
        }
    }
    pub fn keybindings_report(&self) -> crate::keybindings::Report {
        self.keybindings.report()
    }
    pub fn overlay_report(&self) -> crate::overlay::Report {
        self.overlay.report()
    }
    pub fn set_keybinding_input(
        &mut self,
        frames: BTreeMap<String, crate::keybindings::InputFrame>,
    ) -> Result<(), String> {
        for key in frames.keys() {
            let index = self
                .keys
                .iter()
                .position(|k| k == key)
                .ok_or("Unknown keybinding input owner")?;
            if !self.alive[index] || !self.grants[index].contains(crate::keybindings::CAPABILITY) {
                return Err("Keybinding input owner disabled or unauthorized".into());
            }
        }
        self.keybindings.set_input(frames)
    }
    pub fn set_app_input(
        &mut self,
        frames: BTreeMap<String, crate::app::InputFrame>,
    ) -> Result<(), String> {
        for key in frames.keys() {
            let index = self
                .keys
                .iter()
                .position(|k| k == key)
                .ok_or("Unknown app input owner")?;
            if !self.alive[index]
                || !self.grants[index]
                    .iter()
                    .any(|g| g == crate::app::READ_CAP || g == crate::app::WRITE_CAP)
            {
                return Err("App input owner disabled or unauthorized".into());
            }
        }
        self.app.set_input(frames)
    }
    pub fn clear_app(&mut self) {
        self.app.clear();
    }
    pub fn clear_keybinding_pressed(&mut self) {
        self.keybindings.clear_pressed();
    }
    pub fn set_server_input(
        &mut self,
        frames: BTreeMap<String, crate::server_types::ServerFrame>,
    ) -> Result<(), String> {
        if self.epoch == 0 || frames.is_empty() || frames.len() > 16 {
            return Err("Server input requires an active world and bounded owners".into());
        }
        let sequence = frames.values().next().ok_or("Empty input")?.sequence;
        if sequence <= self.server_sequence {
            return Err("Stale server frame".into());
        }
        for (key, frame) in &frames {
            let i = self
                .keys
                .iter()
                .position(|k| k == key)
                .ok_or("Unknown server input owner")?;
            if !self.alive[i]
                || !(self.grants[i].contains("content.server.read")
                    || self.grants[i].contains("network.server.send"))
            {
                return Err("Server input owner unauthorized".into());
            }
            frame.validate()?;
            if frame.sequence != sequence {
                return Err("Mixed server frame sequences".into());
            }
        }
        self.server_input = frames;
        self.server_sequence = sequence;
        Ok(())
    }
    pub fn drain_network(&mut self) -> Vec<crate::server_types::NetworkOutput> {
        std::mem::take(&mut self.network_output)
    }
    pub fn checkpoint_server_scene(
        &self,
    ) -> Result<el_mod_content::scene::SceneCheckpoint, String> {
        self.content.checkpoint_scene()
    }
    pub fn restore_server_scene(
        &mut self,
        checkpoint: &el_mod_content::scene::SceneCheckpoint,
    ) -> Result<(), String> {
        for actor in &checkpoint.actors {
            let i = self
                .keys
                .iter()
                .position(|k| k == &actor.view.owner)
                .ok_or("Saved server actor missing")?;
            if !self.alive[i] || !self.grants[i].contains("content.server.write") {
                return Err("Saved server actor unavailable".into());
            }
        }
        self.content.restore_scene(checkpoint, self.epoch)
    }
    pub fn content_report(&self, include_catalog: bool) -> crate::content::ContentReport {
        let mut report = self.content.report(include_catalog);
        report.readers = self
            .keys
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                self.alive[*i]
                    && (self.grants[*i].contains("content.client.read")
                        || self.grants[*i].contains("content.server.read")
                        || self.grants[*i].contains("world.blocks.read")
                        || self.grants[*i].contains("client.player.read")
                        || self.grants[*i].contains("client.entities.read")
                        || self.grants[*i].contains("client.avatar.read")
                        || self.grants[*i].contains("client.avatar.write"))
            })
            .map(|(_, key)| key.clone())
            .collect();
        report
    }
    pub fn set_content_input(
        &mut self,
        mut frames: BTreeMap<String, el_mod_content::scene_types::InputFrame>,
    ) -> Result<(), String> {
        for key in frames.keys() {
            let index = self
                .keys
                .iter()
                .position(|k| k == key)
                .ok_or("Unknown input owner")?;
            if !self.alive[index]
                || !(self.grants[index].contains("content.client.read")
                    || self.grants[index].contains("world.blocks.read")
                    || self.grants[index].contains("client.player.read")
                    || self.grants[index].contains("client.entities.read")
                    || self.grants[index].contains("client.avatar.read")
                    || self.grants[index].contains("client.avatar.write"))
            {
                return Err("Input owner disabled or unauthorized".into());
            }
            if (frames[key].world_blocks.is_some()
                || frames[key].world_region.is_some()
                || frames[key].world_surface.is_some())
                && !self.grants[index].contains("world.blocks.read")
            {
                return Err("World input owner unauthorized".into());
            }
        }
        for (key, frame) in &mut frames {
            let index = self.keys.iter().position(|k| k == key).unwrap();
            if !self.grants[index].contains("client.player.read") {
                frame.player = None;
            }
            if !self.grants[index].contains("client.entities.read") {
                frame.entities = None;
            }
            if !self.grants[index].contains("client.avatar.read")
                && !self.grants[index].contains("client.avatar.write")
            {
                frame.avatar = None;
            }
        }
        self.content.set_input(frames)
    }
    pub fn resource_counts(&self) -> (usize, usize, usize, usize, usize) {
        (
            self.services.len(),
            self.subscriptions.iter().map(BTreeSet::len).sum(),
            self.inboxes.iter().map(Vec::len).sum::<usize>() + self.network_output.len(),
            self.calls.len(),
            self.jobs.len(),
        )
    }
    pub fn binary_resource_counts(&self) -> Vec<(usize, usize)> {
        self.binary
            .iter()
            .map(crate::binary::Store::resource_counts)
            .collect()
    }
    pub fn disable(&mut self, actors: &[usize]) {
        for &i in actors {
            if i < self.alive.len() {
                self.alive[i] = false;
                self.app.disable(&self.keys[i]);
                self.keybindings.disable(&self.keys[i]);
                self.overlay.remove(&self.keys[i]);
                self.hook_state.remove(&self.keys[i]);
                self.subscriptions[i].clear();
                self.inboxes[i].clear();
                self.content.disable(&self.keys[i]);
                self.binary[i].clear();
                self.server_input.remove(&self.keys[i]);
                self.network_output.retain(|o| o.owner != self.keys[i]);
            }
        }
        let dead: BTreeSet<_> = self
            .keys
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.alive[*i])
            .map(|(_, k)| k.clone())
            .collect();
        self.services.retain(|_, s| !dead.contains(&s.owner));
        self.calls
            .retain(|(requester, _), c| self.alive[*requester] && self.alive[c.provider]);
        self.jobs.retain(|_, j| self.alive[j.job.owner]);
        for inbox in &mut self.inboxes {
            inbox.retain(|m| !dead.contains(&m.sender));
        }
    }
    pub fn exit(&mut self) {
        self.app.clear();
        self.keybindings.clear_pressed();
        self.binary.iter_mut().for_each(crate::binary::Store::clear);
        self.overlay.clear();
        self.hook_state.clear();
        self.content.exit();
        self.server_input.clear();
        self.network_output.clear();
        self.server_sequence = 0;
        self.subscriptions.iter_mut().for_each(BTreeSet::clear);
        self.inboxes.iter_mut().for_each(Vec::clear);
        self.calls.clear();
        self.jobs.clear();
        self.last_call.fill(0);
        self.last_job.fill(0);
    }
    pub fn shutdown(&mut self) {
        self.app.clear();
        self.exit();
        self.content.shutdown();
        self.services.clear();
        self.keybindings = crate::keybindings::State::default();
        self.overlay = crate::overlay::State::default();
        self.alive.fill(false);
    }
    pub fn complete_job(&mut self, job: &Job, result: Vec<u8>) -> Result<(), String> {
        let pending = self.jobs.get_mut(&job.sequence).ok_or("Stale job result")?;
        if pending.job != *job
            || job.epoch != self.epoch
            || !self.alive[job.owner]
            || pending.result.is_some()
            || result.len() != 32
        {
            return Err("Stale, duplicate or invalid job result".into());
        }
        pending.result = Some(result);
        Ok(())
    }
    pub fn freeze_jobs(&mut self) -> Result<Vec<Job>, CommitError> {
        let ready: Vec<_> = self
            .jobs
            .iter()
            .filter_map(|(&seq, j)| j.result.as_ref().map(|r| (seq, j.job.clone(), r.clone())))
            .collect();
        let mut candidate = self.clone();
        let mut delivered = Vec::new();
        for (seq, j, data) in ready {
            let sender = candidate.keys[j.owner].clone();
            candidate
                .push(
                    j.owner,
                    Message {
                        sender,
                        channel: "sha256".into(),
                        token: j.token,
                        data,
                        kind: "job".into(),
                    },
                )
                .map_err(|message| CommitError {
                    actor: j.owner,
                    message,
                })?;
            candidate.jobs.remove(&seq);
            delivered.push(j);
        }
        *self = candidate;
        Ok(delivered)
    }
    fn push(&mut self, recipient: usize, message: Message) -> Result<(), String> {
        if !self.alive[recipient] {
            return Err("Recipient disabled".into());
        }
        let inbox = &mut self.inboxes[recipient];
        if inbox.len() >= 64 {
            return Err("Recipient inbox count limit".into());
        }
        inbox.push(message);
        if serde_json::to_vec(inbox).map_err(|e| e.to_string())?.len() > MAX_PAYLOAD_BYTES {
            inbox.pop();
            return Err("Recipient inbox byte limit".into());
        }
        Ok(())
    }
    pub fn commit(
        &mut self,
        phase: Phase,
        effects: &BTreeMap<usize, Effects>,
    ) -> Result<Vec<Job>, CommitError> {
        if effects.is_empty() {
            if phase == Phase::Tick {
                self.begin_tick();
            }
            self.finish_phase(phase)?;
            return Ok(Vec::new());
        }
        let mut next = self.clone();
        let mut jobs = Vec::new();
        if phase == Phase::Tick {
            next.begin_tick();
        }
        for (&actor, e) in effects {
            if actor >= next.alive.len() {
                return Err(CommitError {
                    actor,
                    message: "Unknown effect owner".into(),
                });
            }
            if !next.alive[actor] {
                continue;
            }
            next.apply(actor, phase, e, &mut jobs)
                .map_err(|message| CommitError { actor, message })?;
        }
        next.finish_phase(phase)?;
        *self = next;
        Ok(jobs)
    }
    fn begin_tick(&mut self) {
        for call in self.calls.values_mut() {
            call.delivered = true;
        }
        self.inboxes.iter_mut().for_each(Vec::clear);
    }
    fn finish_phase(&mut self, phase: Phase) -> Result<(), CommitError> {
        if phase == Phase::Init {
            self.content
                .freeze()
                .map_err(|message| CommitError { actor: 0, message })?;
        }
        if phase == Phase::Tick {
            self.content.finish_tick();
            for input in self.server_input.values_mut() {
                input.events.clear();
            }
        }
        if phase == Phase::AppTick {
            self.app.finish_tick();
        }
        if matches!(phase, Phase::Tick | Phase::AppTick) {
            self.keybindings.finish_tick();
        }
        if phase == Phase::WorldExit {
            self.exit();
        }
        if phase == Phase::Shutdown {
            self.shutdown();
        }
        Ok(())
    }
    fn apply(
        &mut self,
        actor: usize,
        phase: Phase,
        e: &Effects,
        jobs: &mut Vec<Job>,
    ) -> Result<(), String> {
        if e.commands.len() > 64 || e.logs.len() > 64 || e.calls > 128 {
            return Err("Effect count limit".into());
        }
        if e.logs.iter().any(|s| s.len() > 4096)
            || e.logs.iter().map(String::len).sum::<usize>() > MAX_PAYLOAD_BYTES
        {
            return Err("Log limit".into());
        }
        if !e.logs.is_empty() && !self.grants[actor].contains("host.log") {
            return Err("Log capability denied".into());
        }
        let mut bytes = e.logs.iter().map(String::len).sum::<usize>();
        let mut config_changes = Vec::new();
        let mut binary_work = 0usize;
        for command in &e.commands {
            if let Command::KeybindingsRegister { bindings } = command {
                if phase != Phase::Init
                    || !self.grants[actor].contains(crate::keybindings::CAPABILITY)
                {
                    return Err("Keybinding registration phase/capability denied".into());
                }
                bytes += serde_json::to_vec(bindings)
                    .map_err(|e| e.to_string())?
                    .len();
                if bytes > MAX_PAYLOAD_BYTES {
                    return Err("Keybinding command byte limit".into());
                }
                self.keybindings.register(&self.keys[actor], bindings)?;
                continue;
            }
            if let Command::OverlayMessage { segments } = command {
                if !self.grants[actor].contains(crate::overlay::CAPABILITY) {
                    return Err("Overlay capability denied".into());
                }
                if !matches!(phase, Phase::WorldEnter | Phase::Tick) {
                    return Err("Overlay phase denied".into());
                }
                bytes += serde_json::to_vec(segments)
                    .map_err(|e| e.to_string())?
                    .len();
                if bytes > MAX_PAYLOAD_BYTES {
                    return Err("Overlay command byte limit".into());
                }
                self.overlay
                    .publish(&self.keys[actor], self.epoch, segments)?;
                continue;
            }
            if let Command::Binary { operation } = command {
                if !self.grants[actor].contains(crate::binary::CAPABILITY) {
                    return Err("Binary capability denied".into());
                }
                bytes += serde_json::to_vec(operation)
                    .map_err(|e| e.to_string())?
                    .len();
                binary_work = binary_work.saturating_add(self.binary[actor].work(operation)?);
                if bytes > MAX_PAYLOAD_BYTES || binary_work > crate::binary::MAX_WORK {
                    return Err("Binary callback budget".into());
                }
                self.binary[actor].apply(operation)?;
                continue;
            }
            if let Command::HookState { value } = command {
                if !self.grants[actor].contains(crate::hook_state::CAPABILITY)
                    || !matches!(
                        phase,
                        Phase::Init | Phase::WorldEnter | Phase::Tick | Phase::AppTick
                    )
                {
                    return Err("Hook state phase/capability denied".into());
                }
                bytes += serde_json::to_vec(value).map_err(|e| e.to_string())?.len();
                if bytes > MAX_PAYLOAD_BYTES {
                    return Err("Hook state command byte limit".into());
                }
                self.hook_state
                    .publish(&self.keys[actor], self.epoch, value)?;
                continue;
            }
            if let Command::AppUi { ui } = command {
                if phase != Phase::AppTick || !self.grants[actor].contains(crate::app::WRITE_CAP) {
                    return Err("App UI phase/capability denied".into());
                }
                ui.validate()?;
                bytes += serde_json::to_vec(ui).map_err(|e| e.to_string())?.len();
                if bytes > MAX_PAYLOAD_BYTES {
                    return Err("App UI command byte limit".into());
                }
                self.app.apply_ui(&self.keys[actor], ui)?;
                continue;
            }
            if let Command::Config { change } = command {
                if !self.grants[actor].contains(crate::config::WRITE_CAP) {
                    return Err("Configuration write capability denied".into());
                }
                bytes += serde_json::to_vec(change).map_err(|e| e.to_string())?.len();
                if bytes > MAX_PAYLOAD_BYTES {
                    return Err("Configuration command byte limit".into());
                }
                config_changes.push(change.clone());
                continue;
            }
            if let Command::Content { request } = command {
                if !self.grants[actor].contains(request.capability())
                    && !self.grants[actor].contains(request.capability_for_side(2))
                {
                    return Err("Content capability denied".into());
                }
                bytes += serde_json::to_vec(request)
                    .map_err(|e| e.to_string())?
                    .len();
                if bytes > MAX_PAYLOAD_BYTES {
                    return Err("Content command byte limit".into());
                }
                self.content.apply(&self.keys[actor], phase, request)?;
                continue;
            }
            if let Command::Network { to, channel, data } = command {
                if phase != Phase::Tick {
                    return Err("Network command phase/capability denied".into());
                }
                if self.grants[actor].contains("network.client.send") {
                    crate::server_types::validate_client_network(
                        &self.keys[actor],
                        to.as_deref(),
                        channel,
                        data,
                    )?;
                } else if self.grants[actor].contains("network.server.send") {
                    let frame = self
                        .server_input
                        .get(&self.keys[actor])
                        .ok_or("Server input unavailable")?;
                    crate::server_types::validate_network(
                        &self.keys[actor],
                        to.as_deref(),
                        channel,
                        data,
                        frame,
                    )?;
                } else {
                    return Err("Network capability denied".into());
                }
                bytes += serde_json::to_vec(command)
                    .map_err(|e| e.to_string())?
                    .len();
                let total: usize = self
                    .network_output
                    .iter()
                    .map(|o| o.data.len() + o.channel.len() + o.owner.len() + 160)
                    .sum();
                if bytes > MAX_PAYLOAD_BYTES
                    || self.network_output.len() >= 64
                    || total + data.len() + channel.len() + 320 > MAX_PAYLOAD_BYTES
                {
                    return Err("Network output limit".into());
                }
                self.network_output
                    .push(crate::server_types::NetworkOutput {
                        owner: self.keys[actor].clone(),
                        to: to.clone(),
                        channel: channel.clone(),
                        data: data.clone(),
                    });
                continue;
            }
            let is_job = matches!(command, Command::Job { .. });
            if !self.grants[actor].contains(if is_job {
                "host.jobs"
            } else {
                "host.messaging"
            }) {
                return Err("Command capability denied".into());
            }
            let data = match command {
                Command::Publish { data, .. }
                | Command::Call { data, .. }
                | Command::Reply { data, .. }
                | Command::Job { data, .. } => data.len(),
                _ => 0,
            };
            bytes += data;
            if data > crate::host::MAX_ITEM || bytes > MAX_PAYLOAD_BYTES {
                return Err("Command byte limit".into());
            }
            match command {
                Command::Content { .. }
                | Command::Binary { .. }
                | Command::Network { .. }
                | Command::Config { .. }
                | Command::HookState { .. }
                | Command::AppUi { .. }
                | Command::OverlayMessage { .. }
                | Command::KeybindingsRegister { .. } => {
                    unreachable!("handled above")
                }
                Command::Register { id, version } => {
                    if phase != Phase::Init || id.is_reserved_namespace() {
                        return Err("Service registration phase/id".into());
                    }
                    if self.services.contains_key(&id.to_string()) {
                        return Err(format!("Duplicate service {id}"));
                    }
                    if self
                        .services
                        .values()
                        .filter(|s| s.owner == self.keys[actor])
                        .count()
                        >= 64
                    {
                        return Err("Service limit".into());
                    }
                    self.services.insert(
                        id.to_string(),
                        Service {
                            id: id.clone(),
                            version: version.clone(),
                            owner: self.keys[actor].clone(),
                        },
                    );
                    if serde_json::to_vec(&self.services.values().collect::<Vec<_>>())
                        .map_err(|e| e.to_string())?
                        .len()
                        > 131072
                    {
                        return Err("Service catalog byte limit".into());
                    }
                }
                Command::Subscribe { topic } => {
                    if !matches!(phase, Phase::Init | Phase::WorldEnter)
                        || topic.is_reserved_namespace()
                    {
                        return Err("Subscription phase/id".into());
                    }
                    let subs = &mut self.subscriptions[actor];
                    if !subs.contains(&topic.to_string()) && subs.len() >= 64 {
                        return Err("Subscription limit".into());
                    }
                    subs.insert(topic.to_string());
                }
                Command::Publish { topic, data } => {
                    if phase != Phase::Tick || topic.is_reserved_namespace() {
                        return Err("Publish phase/id".into());
                    }
                    for recipient in 0..self.keys.len() {
                        if self.alive[recipient]
                            && self.subscriptions[recipient].contains(&topic.to_string())
                        {
                            self.push(
                                recipient,
                                Message {
                                    sender: self.keys[actor].clone(),
                                    channel: topic.to_string(),
                                    token: 0,
                                    data: data.clone(),
                                    kind: "event".into(),
                                },
                            )?;
                        }
                    }
                }
                Command::Call {
                    service,
                    version,
                    token,
                    data,
                } => {
                    if phase != Phase::Tick || *token == 0 || *token <= self.last_call[actor] {
                        return Err("Call phase/token".into());
                    }
                    let service = self
                        .services
                        .get(&service.to_string())
                        .ok_or("Unavailable service")?
                        .clone();
                    if !service.version.supports(version) {
                        return Err("Service version unavailable".into());
                    }
                    let provider = self
                        .keys
                        .iter()
                        .position(|k| *k == service.owner)
                        .ok_or("Service owner missing")?;
                    if self.calls.keys().filter(|(i, _)| *i == actor).count() >= 64 {
                        return Err("Outstanding call limit".into());
                    }
                    self.push(
                        provider,
                        Message {
                            sender: self.keys[actor].clone(),
                            channel: service.id.to_string(),
                            token: *token,
                            data: data.clone(),
                            kind: "call".into(),
                        },
                    )?;
                    self.calls.insert(
                        (actor, *token),
                        PendingCall {
                            provider,
                            service: service.id.to_string(),
                            delivered: false,
                        },
                    );
                    self.last_call[actor] = *token;
                }
                Command::Reply { to, token, data } => {
                    if phase != Phase::Tick {
                        return Err("Reply phase".into());
                    }
                    let recipient = self
                        .keys
                        .iter()
                        .position(|k| k == to)
                        .ok_or("Reply recipient missing")?;
                    let pending = self
                        .calls
                        .get(&(recipient, *token))
                        .ok_or("Unknown or consumed call")?
                        .clone();
                    if pending.provider != actor || !pending.delivered {
                        return Err("Foreign or premature reply".into());
                    }
                    self.push(
                        recipient,
                        Message {
                            sender: self.keys[actor].clone(),
                            channel: pending.service,
                            token: *token,
                            data: data.clone(),
                            kind: "reply".into(),
                        },
                    )?;
                    self.calls.remove(&(recipient, *token));
                }
                Command::Job { token, data } => {
                    if phase != Phase::Tick || *token == 0 || *token <= self.last_job[actor] {
                        return Err("Job phase/token".into());
                    }
                    if self.jobs.values().filter(|j| j.job.owner == actor).count() >= 2 {
                        return Err("Outstanding job limit".into());
                    }
                    self.sequence = self
                        .sequence
                        .checked_add(1)
                        .ok_or("Job sequence overflow")?;
                    let job = Job {
                        owner: actor,
                        epoch: self.epoch,
                        sequence: self.sequence,
                        token: *token,
                        data: data.clone(),
                    };
                    self.jobs.insert(
                        self.sequence,
                        PendingJob {
                            job: job.clone(),
                            result: None,
                        },
                    );
                    self.last_job[actor] = *token;
                    jobs.push(job);
                }
            }
        }
        self.config.stage(&self.keys[actor], &config_changes)?;
        Ok(())
    }
}

pub fn execute_job(job: &Job) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    Sha256::digest(&job.data).to_vec()
}
