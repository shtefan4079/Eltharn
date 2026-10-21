use crate::{lifecycle::Phase, wire::strict_json};
use el_extension_api::{ApiVersion, ExtensionId, abi::ErrorCode, services};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
mod app;
mod avatar;
mod binary;
mod canvas;
mod client_input;
mod config;
mod hook_state;
mod keybindings;
mod overlay;
mod surface;
mod world_region;

const MAX_OPS: usize = 64;
const MAX_DATA: usize = 64 * 1024;
pub(crate) const MAX_ITEM: usize = 16384;
const MAX_CALLS: usize = 128;
const MAX_LOGS: usize = 64;
const MAX_LOG_ITEM: usize = 4096;
const MAX_JOBS: usize = 2;
const MAX_BUF: usize = 65536;
const MAX_REPLY_TO: usize = 160;
const MAX_NET_DATA: usize = 8192;

fn ec(e: ErrorCode) -> i32 {
    e as i32
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub actor: String,
    pub side: u32,
    pub phase: Phase,
    pub epoch: u64,
    pub tick: u64,
    pub callback_id: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub id: ExtensionId,
    pub version: ApiVersion,
    pub owner: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub sender: String,
    pub channel: String,
    pub token: u64,
    pub data: Vec<u8>,
    pub kind: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keybindings: Option<crate::keybindings::Snapshot>,
    #[serde(skip)]
    pub binary: Option<crate::binary::Store>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<crate::app::Snapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<crate::config::Snapshot>,
    pub services: Vec<Service>,
    pub inbox: Vec<Message>,
    #[serde(default)]
    pub content: Option<crate::content::ContentSnapshot>,
    #[serde(default)]
    pub server: Option<crate::server_types::ServerFrame>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    KeybindingsRegister {
        bindings: Vec<crate::keybindings::Binding>,
    },
    OverlayMessage {
        segments: Vec<crate::overlay::Segment>,
    },
    Binary {
        operation: crate::binary::Mutation,
    },
    HookState {
        value: Value,
    },
    AppUi {
        ui: crate::app::Ui,
    },
    Config {
        change: crate::config::Change,
    },
    Register {
        id: ExtensionId,
        version: ApiVersion,
    },
    Subscribe {
        topic: ExtensionId,
    },
    Publish {
        topic: ExtensionId,
        data: Vec<u8>,
    },
    Call {
        service: ExtensionId,
        version: ApiVersion,
        token: u64,
        data: Vec<u8>,
    },
    Reply {
        to: String,
        token: u64,
        data: Vec<u8>,
    },
    Job {
        token: u64,
        data: Vec<u8>,
    },
    Content {
        request: Box<crate::content::ContentOperation>,
    },
    Network {
        to: Option<String>,
        channel: String,
        data: Vec<u8>,
    },
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effects {
    pub commands: Vec<Command>,
    pub logs: Vec<String>,
    pub calls: usize,
}
pub struct Host {
    context: Option<Context>,
    snapshot: Snapshot,
    grants: BTreeSet<String>,
    effects: Effects,
    bytes: usize,
    operations: usize,
    binary_work: usize,
}

impl Host {
    pub fn new(grants: Vec<String>) -> Self {
        Self {
            context: None,
            snapshot: Snapshot::default(),
            grants: grants.into_iter().collect(),
            effects: Effects::default(),
            bytes: 0,
            operations: 0,
            binary_work: 0,
        }
    }
    pub fn begin(&mut self, context: Context, snapshot: Snapshot) -> Result<(), String> {
        if self.context.is_some() {
            return Err("nested".into());
        }
        if context.side != 1 && context.side != 2 {
            return Err("side".into());
        }
        if context.callback_id == 0 {
            return Err("callback".into());
        }
        match context.phase {
            Phase::Tick | Phase::AppTick => {
                if context.tick == 0 {
                    return Err("tick".into());
                }
            }
            _ => {
                if context.tick != 0 {
                    return Err("tick".into());
                }
            }
        }
        if context.actor.is_empty()
            || context.actor.len() > 160
            || snapshot.inbox.len() > 64
            || snapshot.services.len() > 1024
        {
            return Err("Host snapshot limit".into());
        }
        if let Some(app) = &snapshot.app {
            app.validate()?;
        }
        if let Some(keybindings) = &snapshot.keybindings {
            keybindings.validate()?;
        }
        if let Some(content) = snapshot.content.as_ref() {
            content.validate().map_err(|e| e.to_string())?;
        }
        if let Some(config) = &snapshot.config {
            config.validate()?;
        }
        if let Some(server) = snapshot.server.as_ref() {
            server.validate()?;
        }
        if serde_json::to_vec(&snapshot)
            .map_err(|e| e.to_string())?
            .len()
            > 262144
        {
            return Err("Host snapshot byte limit".into());
        }
        self.context = Some(context);
        self.snapshot = snapshot;
        self.effects = Effects::default();
        self.bytes = 0;
        self.operations = 0;
        self.binary_work = 0;
        Ok(())
    }
    pub fn finish(&mut self, success: bool) -> Result<Effects, String> {
        if self.context.is_none() {
            return Err("no active".into());
        }
        self.context = None;
        self.snapshot = Snapshot::default();
        self.bytes = 0;
        self.operations = 0;
        let e = std::mem::take(&mut self.effects);
        if success {
            Ok(e)
        } else {
            Ok(Effects {
                commands: vec![],
                logs: e.logs,
                calls: e.calls,
            })
        }
    }
    fn has(&self, cap: &str) -> bool {
        self.grants.contains(cap)
    }
    fn phase_of(&self) -> Result<Phase, ErrorCode> {
        self.context
            .as_ref()
            .map(|c| c.phase.clone())
            .ok_or(ErrorCode::WrongPhase)
    }
    fn check_cmd(&self, cmd: &Command) -> Result<(usize, bool), ErrorCode> {
        let phase = self.phase_of()?;
        let tick = matches!(phase, Phase::Tick);
        let init = matches!(phase, Phase::Init);
        let enter = matches!(phase, Phase::WorldEnter);
        match cmd {
            Command::KeybindingsRegister { bindings } => {
                if !self.has(crate::keybindings::CAPABILITY) {
                    return Err(ErrorCode::CapabilityDenied);
                }
                if !init || self.context.as_ref().is_none_or(|c| c.side != 1) {
                    return Err(ErrorCode::WrongPhase);
                }
                let mut combined = Vec::new();
                for command in &self.effects.commands {
                    if let Command::KeybindingsRegister { bindings } = command {
                        combined.extend_from_slice(bindings);
                    }
                }
                combined.extend_from_slice(bindings);
                crate::keybindings::validate_bindings(&combined)
                    .map_err(|_| ErrorCode::InvalidRequest)?;
                Ok((
                    serde_json::to_vec(bindings)
                        .map_err(|_| ErrorCode::InvalidRequest)?
                        .len(),
                    false,
                ))
            }
            Command::OverlayMessage { segments } => {
                if !self.has(crate::overlay::CAPABILITY) {
                    return Err(ErrorCode::CapabilityDenied);
                }
                if self.context.as_ref().is_none_or(|c| c.side != 1) || !(enter || tick) {
                    return Err(ErrorCode::WrongPhase);
                }
                crate::overlay::validate(segments).map_err(|_| ErrorCode::InvalidRequest)?;
                let bytes = serde_json::to_vec(segments)
                    .map_err(|_| ErrorCode::InvalidRequest)?
                    .len();
                if bytes > crate::overlay::MAX_REQUEST_BYTES {
                    return Err(ErrorCode::LimitExceeded);
                }
                Ok((bytes, false))
            }
            Command::Binary { operation } => {
                if !self.has(crate::binary::CAPABILITY) {
                    return Err(ErrorCode::CapabilityDenied);
                }
                operation
                    .validate()
                    .map_err(|_| ErrorCode::InvalidRequest)?;
                Ok((
                    serde_json::to_vec(operation)
                        .map_err(|_| ErrorCode::InvalidRequest)?
                        .len(),
                    false,
                ))
            }
            Command::HookState { value } => {
                if !self.has(crate::hook_state::CAPABILITY) {
                    return Err(ErrorCode::CapabilityDenied);
                }
                if self.context.as_ref().is_none_or(|c| c.side != 1)
                    || !matches!(
                        phase,
                        Phase::Init | Phase::WorldEnter | Phase::Tick | Phase::AppTick
                    )
                {
                    return Err(ErrorCode::WrongPhase);
                }
                crate::hook_state::validate(value).map_err(|_| ErrorCode::LimitExceeded)?;
                Ok((
                    serde_json::to_vec(value)
                        .map_err(|_| ErrorCode::InvalidRequest)?
                        .len(),
                    false,
                ))
            }
            Command::AppUi { ui } => {
                if !self.has(crate::app::WRITE_CAP) {
                    return Err(ErrorCode::CapabilityDenied);
                }
                if phase != Phase::AppTick || self.context.as_ref().is_none_or(|c| c.side != 1) {
                    return Err(ErrorCode::WrongPhase);
                }
                ui.validate().map_err(|_| ErrorCode::InvalidRequest)?;
                ui.validate_owner(&self.context.as_ref().unwrap().actor)
                    .map_err(|_| ErrorCode::CapabilityDenied)?;
                if ui.requests_close() || ui.layout.as_ref().is_some_and(|l| !l.footer.is_empty()) {
                    let input = &self
                        .snapshot
                        .app
                        .as_ref()
                        .ok_or(ErrorCode::InvalidRequest)?
                        .input;
                    ui.validate_input_context(input)
                        .map_err(|_| ErrorCode::InvalidRequest)?;
                }
                Ok((
                    serde_json::to_vec(ui)
                        .map_err(|_| ErrorCode::InvalidRequest)?
                        .len(),
                    false,
                ))
            }
            Command::Config { change } => {
                if !self.has(crate::config::WRITE_CAP) {
                    return Err(ErrorCode::CapabilityDenied);
                }
                let mut record = self
                    .snapshot
                    .config
                    .as_ref()
                    .ok_or(ErrorCode::InvalidRequest)?
                    .record
                    .clone();
                for command in &self.effects.commands {
                    if let Command::Config { change } = command {
                        change
                            .apply(&mut record)
                            .map_err(|_| ErrorCode::LimitExceeded)?;
                    }
                }
                change
                    .apply(&mut record)
                    .map_err(|_| ErrorCode::LimitExceeded)?;
                Ok((
                    serde_json::to_vec(change)
                        .map_err(|_| ErrorCode::InvalidRequest)?
                        .len(),
                    false,
                ))
            }
            Command::Register { id, .. } => {
                if !init {
                    return Err(ErrorCode::WrongPhase);
                }
                if id.is_reserved_namespace() {
                    return Err(ErrorCode::InvalidRequest);
                }
                Ok((0, false))
            }
            Command::Subscribe { topic } => {
                if !(init || enter) {
                    return Err(ErrorCode::WrongPhase);
                }
                if topic.is_reserved_namespace() {
                    return Err(ErrorCode::InvalidRequest);
                }
                Ok((0, false))
            }
            Command::Publish { topic, data } => {
                if !tick {
                    return Err(ErrorCode::WrongPhase);
                }
                if topic.is_reserved_namespace() {
                    return Err(ErrorCode::InvalidRequest);
                }
                if data.len() > MAX_ITEM {
                    return Err(ErrorCode::LimitExceeded);
                }
                Ok((data.len(), false))
            }
            Command::Call {
                service,
                version,
                token,
                data,
            } => {
                if !tick {
                    return Err(ErrorCode::WrongPhase);
                }
                if service.is_reserved_namespace() {
                    return Err(ErrorCode::InvalidRequest);
                }
                if *token == 0 {
                    return Err(ErrorCode::InvalidRequest);
                }
                if data.len() > MAX_ITEM {
                    return Err(ErrorCode::LimitExceeded);
                }
                match self.snapshot.services.iter().find(|s| s.id == *service) {
                    None => Err(ErrorCode::Unavailable),
                    Some(s) if !s.version.supports(version) => Err(ErrorCode::Unavailable),
                    Some(_) => Ok((data.len(), false)),
                }
            }
            Command::Reply { to, token, data } => {
                if !tick {
                    return Err(ErrorCode::WrongPhase);
                }
                if to.is_empty() || to.len() > MAX_REPLY_TO {
                    return Err(ErrorCode::InvalidRequest);
                }
                if *token == 0 {
                    return Err(ErrorCode::InvalidRequest);
                }
                if data.len() > MAX_ITEM {
                    return Err(ErrorCode::LimitExceeded);
                }
                Ok((data.len(), false))
            }
            Command::Job { token, data } => {
                if !tick {
                    return Err(ErrorCode::WrongPhase);
                }
                if *token == 0 {
                    return Err(ErrorCode::InvalidRequest);
                }
                if data.len() > MAX_ITEM {
                    return Err(ErrorCode::LimitExceeded);
                }
                Ok((data.len(), true))
            }
            Command::Content { request } => {
                let side = self
                    .context
                    .as_ref()
                    .map(|c| c.side)
                    .ok_or(ErrorCode::WrongPhase)?;
                match request.as_ref() {
                    crate::content::ContentOperation::Cape { deformation, .. } => {
                        if side != 1 || !self.has("client.avatar.write") {
                            return Err(ErrorCode::CapabilityDenied);
                        }
                        if !(enter || tick) {
                            return Err(ErrorCode::WrongPhase);
                        }
                        if let Some(deformation) = deformation {
                            let avatar = self
                                .snapshot
                                .content
                                .as_ref()
                                .and_then(|c| c.input.avatar.as_ref())
                                .ok_or(ErrorCode::Unavailable)?;
                            if deformation.handle != avatar.handle {
                                return Err(ErrorCode::InvalidRequest);
                            }
                        }
                    }
                    crate::content::ContentOperation::Canvas { .. } => {
                        if side != 1 || !self.has("client.canvas.write") {
                            return Err(ErrorCode::CapabilityDenied);
                        }
                        if !(enter || tick) {
                            return Err(ErrorCode::WrongPhase);
                        }
                    }
                    crate::content::ContentOperation::WorldWatch { .. }
                    | crate::content::ContentOperation::WorldRegion { .. }
                    | crate::content::ContentOperation::WorldSurface { .. } => {
                        if side != 1 || !self.has("world.blocks.read") {
                            return Err(ErrorCode::CapabilityDenied);
                        }
                        if !(enter || tick) {
                            return Err(ErrorCode::WrongPhase);
                        }
                    }
                    crate::content::ContentOperation::Camera { .. } => {
                        if side != 1 || !self.has("client.camera.write") {
                            return Err(ErrorCode::CapabilityDenied);
                        }
                        if !(enter || tick) {
                            return Err(ErrorCode::WrongPhase);
                        }
                    }
                    crate::content::ContentOperation::Register { .. } => {
                        if side != 1 && side != 2 {
                            return Err(ErrorCode::CapabilityDenied);
                        }
                        if !self.has("content.register") {
                            return Err(ErrorCode::CapabilityDenied);
                        }
                        if !init {
                            return Err(ErrorCode::WrongPhase);
                        }
                    }
                    crate::content::ContentOperation::Apply { .. } => {
                        let cap = if side == 2 {
                            "content.server.write"
                        } else if side == 1 {
                            "content.client.write"
                        } else {
                            return Err(ErrorCode::CapabilityDenied);
                        };
                        if !self.has(cap) {
                            return Err(ErrorCode::CapabilityDenied);
                        }
                        if !(enter || tick) {
                            return Err(ErrorCode::WrongPhase);
                        }
                    }
                }
                if request.validate(phase).is_err() {
                    return Err(ErrorCode::InvalidRequest);
                }
                let n = serde_json::to_vec(request)
                    .map(|v| v.len())
                    .unwrap_or(usize::MAX);
                if n > MAX_DATA {
                    return Err(ErrorCode::LimitExceeded);
                }
                Ok((n, false))
            }
            Command::Network { to, channel, data } => {
                let side = self
                    .context
                    .as_ref()
                    .map(|c| c.side)
                    .ok_or(ErrorCode::WrongPhase)?;
                if !tick {
                    return Err(ErrorCode::WrongPhase);
                }
                if data.len() > MAX_NET_DATA {
                    return Err(ErrorCode::LimitExceeded);
                }
                let context = self.context.as_ref().ok_or(ErrorCode::WrongPhase)?;
                match side {
                    1 => {
                        if !self.has("network.client.send") {
                            return Err(ErrorCode::CapabilityDenied);
                        }
                        crate::server_types::validate_client_network(
                            &context.actor,
                            to.as_deref(),
                            channel,
                            data,
                        )
                        .map_err(|_| ErrorCode::InvalidRequest)?;
                    }
                    2 => {
                        if !self.has("network.server.send") {
                            return Err(ErrorCode::CapabilityDenied);
                        }
                        let server = self
                            .snapshot
                            .server
                            .as_ref()
                            .ok_or(ErrorCode::Unavailable)?;
                        crate::server_types::validate_network(
                            &context.actor,
                            to.as_deref(),
                            channel,
                            data,
                            server,
                        )
                        .map_err(|_| ErrorCode::InvalidRequest)?;
                    }
                    _ => return Err(ErrorCode::CapabilityDenied),
                }
                Ok((data.len(), false))
            }
        }
    }
    fn stage_cmd(&mut self, cmd: Command) -> Result<(), ErrorCode> {
        let (n, is_job) = self.check_cmd(&cmd)?;
        if self.operations >= MAX_OPS {
            return Err(ErrorCode::LimitExceeded);
        }
        if is_job
            && self
                .effects
                .commands
                .iter()
                .filter(|c| matches!(c, Command::Job { .. }))
                .count()
                >= MAX_JOBS
        {
            return Err(ErrorCode::LimitExceeded);
        }
        if self.bytes + n > MAX_DATA {
            return Err(ErrorCode::LimitExceeded);
        }
        self.bytes += n;
        self.operations += 1;
        self.effects.commands.push(cmd);
        Ok(())
    }
    fn do_context(&mut self, req: &[u8], res: &mut [u8]) -> i32 {
        if !self.has("host.context") {
            return ec(ErrorCode::CapabilityDenied);
        }
        if !req.is_empty() {
            return ec(ErrorCode::InvalidRequest);
        }
        if res.len() < 24 {
            return ec(ErrorCode::BufferTooSmall);
        }
        let c = match self.context.as_ref() {
            Some(c) => c,
            None => return ec(ErrorCode::WrongPhase),
        };
        res[0..4].copy_from_slice(&c.side.to_le_bytes());
        res[4..8].copy_from_slice(&c.phase.code().to_le_bytes());
        res[8..16].copy_from_slice(&c.tick.to_le_bytes());
        res[16..24].copy_from_slice(&c.callback_id.to_le_bytes());
        24
    }
    fn do_log(&mut self, req: &[u8], _res: &mut [u8]) -> i32 {
        if !self.has("host.log") {
            return ec(ErrorCode::CapabilityDenied);
        }
        if self.context.is_none() {
            return ec(ErrorCode::WrongPhase);
        }
        let s = match std::str::from_utf8(req) {
            Ok(s) => s,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if s.len() > MAX_LOG_ITEM {
            return ec(ErrorCode::InvalidRequest);
        }
        if self.effects.logs.len() >= MAX_LOGS {
            return ec(ErrorCode::LimitExceeded);
        }
        if self.bytes + s.len() > MAX_BUF {
            return ec(ErrorCode::LimitExceeded);
        }
        self.bytes += s.len();
        self.effects.logs.push(s.to_owned());
        0
    }
    fn is_poll(v: &Value) -> bool {
        match v {
            Value::Object(m) => {
                m.len() == 1 && m.get("op") == Some(&Value::String("poll".to_owned()))
            }
            _ => false,
        }
    }
    fn do_rpc(&mut self, req: &[u8], res: &mut [u8]) -> i32 {
        if !self.has("host.messaging") {
            return ec(ErrorCode::CapabilityDenied);
        }
        if self.context.is_none() {
            return ec(ErrorCode::WrongPhase);
        }
        let v: Value = match strict_json(req, MAX_BUF) {
            Ok(v) => v,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if Self::is_poll(&v) {
            if self.phase_of() != Ok(Phase::Tick) {
                return ec(ErrorCode::WrongPhase);
            }
            let buf = match serde_json::to_vec(&self.snapshot.inbox) {
                Ok(b) => b,
                Err(_) => return ec(ErrorCode::InvalidRequest),
            };
            if buf.len() > res.len() {
                return ec(ErrorCode::BufferTooSmall);
            }
            res[..buf.len()].copy_from_slice(&buf);
            return buf.len() as i32;
        }
        let cmd: Command = match serde_json::from_value(v) {
            Ok(c) => c,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if matches!(
            cmd,
            Command::Job { .. }
                | Command::Content { .. }
                | Command::Network { .. }
                | Command::Config { .. }
        ) {
            return ec(ErrorCode::InvalidOpcode);
        }
        match self.stage_cmd(cmd) {
            Ok(()) => 0,
            Err(e) => ec(e),
        }
    }
    fn do_job(&mut self, req: &[u8], _res: &mut [u8]) -> i32 {
        if !self.has("host.jobs") {
            return ec(ErrorCode::CapabilityDenied);
        }
        if self.context.is_none() {
            return ec(ErrorCode::WrongPhase);
        }
        let v: Value = match strict_json(req, MAX_BUF) {
            Ok(v) => v,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        let cmd: Command = match serde_json::from_value(v) {
            Ok(c) => c,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if !matches!(cmd, Command::Job { .. }) {
            return ec(ErrorCode::InvalidRequest);
        }
        match self.stage_cmd(cmd) {
            Ok(()) => 0,
            Err(e) => ec(e),
        }
    }
    fn do_content(&mut self, req: &[u8], _res: &mut [u8]) -> i32 {
        let def = match el_mod_content::Definition::parse(req) {
            Ok(d) => d,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        let op: crate::content::ContentOperation = match serde_json::from_value(def.value()) {
            Ok(o) => o,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        match self.stage_cmd(Command::Content {
            request: Box::new(op),
        }) {
            Ok(()) => 0,
            Err(e) => ec(e),
        }
    }
    fn do_content_read(&mut self, req: &[u8], res: &mut [u8]) -> i32 {
        if !self.has("content.client.read") {
            return ec(ErrorCode::CapabilityDenied);
        }
        let side = match self.context.as_ref() {
            Some(c) => c.side,
            None => return ec(ErrorCode::WrongPhase),
        };
        if side != 1 {
            return ec(ErrorCode::CapabilityDenied);
        }
        match self.phase_of() {
            Ok(Phase::WorldEnter) | Ok(Phase::Tick) => {}
            _ => return ec(ErrorCode::WrongPhase),
        }
        if !req.is_empty() {
            return ec(ErrorCode::InvalidRequest);
        }
        let snapshot = match self.snapshot.content.as_ref() {
            Some(s) => s,
            None => return ec(ErrorCode::Unavailable),
        };
        let mut snapshot = snapshot.clone();
        if !self.has("client.camera.read") {
            snapshot.scene.actor.camera = None;
        }
        snapshot.scene.actor.cape = None;
        if !self.has("world.blocks.read") {
            snapshot.scene.actor.world_watch = None;
            snapshot.scene.actor.world_region = None;
            snapshot.scene.actor.world_surface = None;
        }
        snapshot.input.world_blocks = None;
        snapshot.input.world_region = None;
        snapshot.input.world_surface = None;
        snapshot.input.player = None;
        snapshot.input.avatar = None;
        snapshot.input.entities = None;
        for canvas in &mut snapshot.scene.actor.canvases {
            canvas.rgba_base64 = None;
        }
        let buf = match serde_json::to_vec(&snapshot) {
            Ok(b) => b,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if buf.len() > MAX_BUF {
            return ec(ErrorCode::LimitExceeded);
        }
        if buf.len() > res.len() {
            return ec(ErrorCode::BufferTooSmall);
        }
        res[..buf.len()].copy_from_slice(&buf);
        buf.len() as i32
    }
    fn do_world_watch(&mut self, req: &[u8], _res: &mut [u8]) -> i32 {
        let watch = match strict_json(req, MAX_BUF)
            .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        {
            Ok(watch) => watch,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        let Some(context) = self.context.as_ref() else {
            return ec(ErrorCode::WrongPhase);
        };
        let request = crate::content::ContentOperation::WorldWatch {
            epoch: context.epoch,
            watch,
        };
        match self.stage_cmd(Command::Content {
            request: Box::new(request),
        }) {
            Ok(()) => 0,
            Err(error) => ec(error),
        }
    }
    fn do_world_read(&mut self, req: &[u8], res: &mut [u8]) -> i32 {
        if !self.has("world.blocks.read") || self.context.as_ref().is_none_or(|c| c.side != 1) {
            return ec(ErrorCode::CapabilityDenied);
        }
        if !matches!(self.phase_of(), Ok(Phase::WorldEnter | Phase::Tick)) {
            return ec(ErrorCode::WrongPhase);
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Request {
            index: usize,
        }
        let request: Request = match strict_json(req, MAX_BUF)
            .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        {
            Ok(request) => request,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        let Some(snapshot) = self.snapshot.content.as_ref() else {
            return ec(ErrorCode::Unavailable);
        };
        let Some(watch) = snapshot.scene.actor.world_watch.as_ref() else {
            return ec(ErrorCode::Unavailable);
        };
        let Some(position) = watch.positions.get(request.index) else {
            return ec(ErrorCode::InvalidRequest);
        };
        let cell = snapshot
            .input
            .world_blocks
            .as_ref()
            .filter(|sample| sample.watch == *watch)
            .and_then(|sample| sample.cells.get(request.index));
        let reply = serde_json::json!({"version":1,"query":watch.query,"position":position,
            "sequence":snapshot.input.sequence,"ready":cell.is_some(),"cell":cell});
        let bytes = match serde_json::to_vec(&reply) {
            Ok(bytes) => bytes,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if bytes.len() > res.len() {
            return ec(ErrorCode::BufferTooSmall);
        }
        res[..bytes.len()].copy_from_slice(&bytes);
        bytes.len() as i32
    }
    fn do_camera_write(&mut self, req: &[u8], _res: &mut [u8]) -> i32 {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Request {
            #[serde(default)]
            hurt_scale_milli: Option<u16>,
        }
        let value = match strict_json(req, MAX_BUF) {
            Ok(v) => v,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        let request: Request = match serde_json::from_value(value) {
            Ok(r) => r,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        let Some(context) = self.context.as_ref() else {
            return ec(ErrorCode::WrongPhase);
        };
        let operation = crate::content::ContentOperation::Camera {
            epoch: context.epoch,
            settings: request.hurt_scale_milli.map(|hurt_scale_milli| {
                el_mod_content::scene_types::CameraSettings { hurt_scale_milli }
            }),
        };
        match self.stage_cmd(Command::Content {
            request: Box::new(operation),
        }) {
            Ok(()) => 0,
            Err(error) => ec(error),
        }
    }
    fn do_camera_read(&mut self, req: &[u8], res: &mut [u8]) -> i32 {
        if !self.has("client.camera.read") || self.context.as_ref().is_none_or(|c| c.side != 1) {
            return ec(ErrorCode::CapabilityDenied);
        }
        if !matches!(self.phase_of(), Ok(Phase::WorldEnter | Phase::Tick)) {
            return ec(ErrorCode::WrongPhase);
        }
        if !req.is_empty() {
            return ec(ErrorCode::InvalidRequest);
        }
        let Some(snapshot) = self.snapshot.content.as_ref() else {
            return ec(ErrorCode::Unavailable);
        };
        let settings =
            snapshot
                .scene
                .actor
                .camera
                .unwrap_or(el_mod_content::scene_types::CameraSettings {
                    hurt_scale_milli: 1000,
                });
        let bytes = match serde_json::to_vec(&settings) {
            Ok(bytes) => bytes,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if bytes.len() > res.len() {
            return ec(ErrorCode::BufferTooSmall);
        }
        res[..bytes.len()].copy_from_slice(&bytes);
        bytes.len() as i32
    }
    fn do_server_content_read(&mut self, req: &[u8], res: &mut [u8]) -> i32 {
        if !self.has("content.server.read") {
            return ec(ErrorCode::CapabilityDenied);
        }
        let side = match self.context.as_ref() {
            Some(c) => c.side,
            None => return ec(ErrorCode::WrongPhase),
        };
        if side != 2 {
            return ec(ErrorCode::CapabilityDenied);
        }
        match self.phase_of() {
            Ok(Phase::WorldEnter) | Ok(Phase::Tick) => {}
            _ => return ec(ErrorCode::WrongPhase),
        }
        if !req.is_empty() {
            return ec(ErrorCode::InvalidRequest);
        }
        let content = match self.snapshot.content.as_ref() {
            Some(s) => s,
            None => return ec(ErrorCode::Unavailable),
        };
        let server = match self.snapshot.server.as_ref() {
            Some(s) => s,
            None => return ec(ErrorCode::Unavailable),
        };
        let buf = match serde_json::to_vec(&serde_json::json!({
            "scene": &content.scene,
            "input": server,
        })) {
            Ok(b) => b,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if buf.len() > MAX_BUF {
            return ec(ErrorCode::LimitExceeded);
        }
        if buf.len() > res.len() {
            return ec(ErrorCode::BufferTooSmall);
        }
        res[..buf.len()].copy_from_slice(&buf);
        buf.len() as i32
    }
    fn do_network(&mut self, req: &[u8], _res: &mut [u8]) -> i32 {
        if self.context.is_none() {
            return ec(ErrorCode::WrongPhase);
        }
        let v: Value = match strict_json(req, MAX_BUF) {
            Ok(v) => v,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        let cmd: Command = match serde_json::from_value(v) {
            Ok(c) => c,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if !matches!(cmd, Command::Network { .. }) {
            return ec(ErrorCode::InvalidRequest);
        }
        match self.stage_cmd(cmd) {
            Ok(()) => 0,
            Err(e) => ec(e),
        }
    }
    pub fn dispatch(&mut self, op: i32, request: &[u8], response: &mut [u8]) -> i32 {
        if self.context.is_none() {
            return ec(ErrorCode::WrongPhase);
        }
        if self.effects.calls >= MAX_CALLS {
            return ec(ErrorCode::LimitExceeded);
        }
        self.effects.calls += 1;
        if request.len() > MAX_BUF || response.len() > MAX_BUF {
            return ec(ErrorCode::LimitExceeded);
        }
        match op {
            crate::binary::OPCODE => self.do_binary(request, response),
            services::CLIENT_INPUT_READ_OPCODE => self.do_client_input_read(request, response),
            services::CLIENT_INPUT_EVENTS_OPCODE => self.do_client_input_events(request, response),
            services::CLIENT_UI_WRITE_OPCODE => self.do_client_ui(request, response),
            services::CLIENT_AVATAR_READ_OPCODE => self.do_avatar_read(request, response),
            services::CLIENT_AVATAR_WRITE_OPCODE => self.do_cape_write(request, response),
            services::WORLD_SURFACE_WATCH_OPCODE..=services::WORLD_SURFACE_STATE_OPCODE => {
                self.do_surface(op, request, response)
            }
            services::CLIENT_PLAYER_READ_OPCODE => self.do_player_read(request, response),
            services::CLIENT_ENTITIES_READ_OPCODE => self.do_entities_read(request, response),
            services::CONFIG_READ_OPCODE..=services::CONFIG_STATUS_OPCODE => {
                self.do_config(op, request, response)
            }
            crate::hook_state::OPCODE => self.do_hook_state(request),
            services::CLIENT_KEYBINDINGS_REGISTER_OPCODE => {
                self.do_keybindings_register(request, response)
            }
            services::CLIENT_KEYBINDING_READ_OPCODE => self.do_keybinding_read(request, response),
            crate::overlay::WRITE_OPCODE => self.do_overlay_write(request, response),
            services::CLIENT_APP_READ_OPCODE => self.do_app_read(request, response),
            services::CLIENT_APP_WRITE_OPCODE => self.do_app_ui(request, response),
            services::CLIENT_APP_EVENT_OPCODE => self.do_app_event(request, response),
            services::CLIENT_APP_MENU_OPCODE => self.do_app_menu(request, response),
            services::CONTEXT_OPCODE => self.do_context(request, response),
            services::LOG_OPCODE => self.do_log(request, response),
            services::HOST_MESSAGING_OPCODE => self.do_rpc(request, response),
            services::HOST_JOBS_OPCODE => self.do_job(request, response),
            services::CONTENT_OPCODE => self.do_content(request, response),
            services::CONTENT_CLIENT_READ_OPCODE => self.do_content_read(request, response),
            services::CONTENT_SERVER_READ_OPCODE => self.do_server_content_read(request, response),
            services::NETWORK_SEND_OPCODE => self.do_network(request, response),
            services::CLIENT_CAMERA_WRITE_OPCODE => self.do_camera_write(request, response),
            services::CLIENT_CAMERA_READ_OPCODE => self.do_camera_read(request, response),
            services::WORLD_BLOCKS_WATCH_OPCODE => self.do_world_watch(request, response),
            services::WORLD_BLOCKS_READ_OPCODE => self.do_world_read(request, response),
            services::WORLD_REGION_WATCH_OPCODE => self.do_world_region_watch(request, response),
            services::CLIENT_CANVAS_WRITE_OPCODE => self.do_canvas(request, response),
            services::WORLD_REGION_READ_OPCODE => self.do_world_region_read(request, response),
            services::WORLD_REGION_PAGE_OPCODE => self.do_world_region_page(request, response),
            services::WORLD_REGION_PALETTE_OPCODE => {
                self.do_world_region_palette(request, response)
            }
            _ => ec(ErrorCode::InvalidOpcode),
        }
    }
}
