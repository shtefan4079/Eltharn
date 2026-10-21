use crate::CapabilityName;
use core::fmt;
use std::collections::BTreeSet;
pub const HOST_CONTEXT_CAPABILITY: &str = "host.context";
pub const HOST_LOG_CAPABILITY: &str = "host.log";
pub const CONTEXT_OPCODE: i32 = 1;
pub const LOG_OPCODE: i32 = 2;
pub const HOST_MESSAGING_OPCODE: i32 = 0x200;
pub const HOST_JOBS_OPCODE: i32 = 0x201;
pub const HOST_MESSAGING_CAPABILITY: &str = "host.messaging";
pub const HOST_JOBS_CAPABILITY: &str = "host.jobs";
pub const CONTENT_REGISTER_CAPABILITY: &str = "content.register";
pub const CONTENT_CLIENT_READ_CAPABILITY: &str = "content.client.read";
pub const CONTENT_CLIENT_WRITE_CAPABILITY: &str = "content.client.write";
pub const CONTENT_SERVER_READ_CAPABILITY: &str = "content.server.read";
pub const CONTENT_SERVER_WRITE_CAPABILITY: &str = "content.server.write";
pub const NETWORK_CLIENT_SEND_CAPABILITY: &str = "network.client.send";
pub const NETWORK_SERVER_SEND_CAPABILITY: &str = "network.server.send";
pub const CONTENT_OPCODE: i32 = 0x300;
pub const CONTENT_CLIENT_READ_OPCODE: i32 = 0x301;
pub const CONTENT_SERVER_READ_OPCODE: i32 = 0x310;
pub const NETWORK_SEND_OPCODE: i32 = 0x311;
pub const CLIENT_CAMERA_WRITE_CAPABILITY: &str = "client.camera.write";
pub const CLIENT_CAMERA_READ_CAPABILITY: &str = "client.camera.read";
pub const CLIENT_CAMERA_WRITE_OPCODE: i32 = 0x320;
pub const CLIENT_CAMERA_READ_OPCODE: i32 = 0x321;
pub const CLIENT_PLAYER_READ_CAPABILITY: &str = "client.player.read";
pub const CLIENT_PLAYER_READ_OPCODE: i32 = 0x322;
pub const CLIENT_ENTITIES_READ_CAPABILITY: &str = "client.entities.read";
pub const CLIENT_ENTITIES_READ_OPCODE: i32 = 0x323;
pub const WORLD_BLOCKS_READ_CAPABILITY: &str = "world.blocks.read";
pub const WORLD_BLOCKS_WATCH_OPCODE: i32 = 0x330;
pub const WORLD_BLOCKS_READ_OPCODE: i32 = 0x331;
pub const WORLD_REGION_WATCH_OPCODE: i32 = 0x332;
pub const WORLD_REGION_READ_OPCODE: i32 = 0x333;
pub const WORLD_REGION_PAGE_OPCODE: i32 = 0x334;
pub const WORLD_REGION_PALETTE_OPCODE: i32 = 0x335;
pub const WORLD_SURFACE_WATCH_OPCODE: i32 = 0x336;
pub const WORLD_SURFACE_READ_OPCODE: i32 = 0x337;
pub const WORLD_SURFACE_PAGE_OPCODE: i32 = 0x338;
pub const WORLD_SURFACE_STATE_OPCODE: i32 = 0x339;
pub const CLIENT_CANVAS_WRITE_CAPABILITY: &str = "client.canvas.write";
pub const CLIENT_CANVAS_WRITE_OPCODE: i32 = 0x340;
pub const CONFIG_READ_CAPABILITY: &str = "config.read";
pub const CONFIG_WRITE_CAPABILITY: &str = "config.write";
pub const CONFIG_READ_OPCODE: i32 = 0x350;
pub const CONFIG_WRITE_OPCODE: i32 = 0x351;
pub const CONFIG_DELETE_OPCODE: i32 = 0x352;
pub const CONFIG_STATUS_OPCODE: i32 = 0x353;
pub const CLIENT_APP_READ_CAPABILITY: &str = "client.app.read";
pub const CLIENT_APP_WRITE_CAPABILITY: &str = "client.app.write";
pub const CLIENT_APP_READ_OPCODE: i32 = 0x360;
pub const CLIENT_APP_WRITE_OPCODE: i32 = 0x361;
pub const CLIENT_APP_EVENT_OPCODE: i32 = 0x362;
pub const CLIENT_APP_MENU_OPCODE: i32 = 0x363;
pub const CLIENT_AVATAR_READ_CAPABILITY: &str = "client.avatar.read";
pub const CLIENT_AVATAR_WRITE_CAPABILITY: &str = "client.avatar.write";
pub const CLIENT_AVATAR_READ_OPCODE: i32 = 0x370;
pub const CLIENT_AVATAR_WRITE_OPCODE: i32 = 0x371;
pub const CLIENT_INPUT_READ_OPCODE: i32 = 0x380;
pub const CLIENT_INPUT_EVENTS_OPCODE: i32 = 0x381;
pub const CLIENT_UI_WRITE_OPCODE: i32 = 0x382;
pub const CLIENT_TOOLTIP_HOOK_CAPABILITY: &str = "client.tooltip.hook";
pub const CLIENT_CAMERA_HOOK_CAPABILITY: &str = "client.camera.hook";
pub const CLIENT_AVATAR_HOOK_CAPABILITY: &str = "client.avatar.hook";
pub const CLIENT_RENDER_HOOK_CAPABILITY: &str = "client.render.hook";
pub const CLIENT_INPUT_HOOK_CAPABILITY: &str = "client.input.hook";
pub const CLIENT_HOOK_STATE_WRITE_CAPABILITY: &str = "client.hooks.state.write";
pub const CLIENT_HOOK_STATE_WRITE_OPCODE: i32 = 0x390;
pub const BINARY_BUFFERS_CAPABILITY: &str = "binary.buffers";
pub const BINARY_BUFFERS_OPCODE: i32 = 0x3b0;
pub const CLIENT_KEYBINDINGS_CAPABILITY: &str = "client.keybindings";
pub const CLIENT_KEYBINDINGS_REGISTER_OPCODE: i32 = 0x3c0;
pub const CLIENT_KEYBINDING_READ_OPCODE: i32 = 0x3c1;
pub const CLIENT_OVERLAY_WRITE_CAPABILITY: &str = "client.overlay.write";
pub const CLIENT_OVERLAY_WRITE_OPCODE: i32 = 0x3d0;
pub const PROTOTYPE_CAPABILITIES: [&str; 2] = [HOST_CONTEXT_CAPABILITY, HOST_LOG_CAPABILITY];
pub const RESERVED_FUTURE_CAPABILITIES: [&str; 10] = [
    WORLD_BLOCKS_READ_CAPABILITY,
    "world.blocks.write",
    "registry.items.read",
    "entity.state.read",
    "entity.state.write",
    "render.commands.submit",
    "audio.commands.submit",
    "input.state.read",
    "network.messages.send",
    "save.jobs.write",
];
pub const MAX_MANIFEST_CAPABILITIES: usize = 64;
pub const MAX_HOST_CATALOG: usize = 256;
pub const MAX_LOG_MESSAGE_BYTES: usize = 4096;
pub const CONTEXT_RESPONSE_BYTES: usize = 24;
pub const WASM_PAGE_BYTES: usize = 65536;
pub const WASM_MAX_PAGES: usize = 1024;
pub const WASM_MAX_BYTES: usize = 1024 * 65536;
pub const WASM_GUEST_FUNCTABLE_MAX: u32 = 1024;
pub const WASM_ALLOW_START_SECTION: bool = false;
pub const WASM_BASELINE_FEATURE_MASK_CANDIDATE: u64 =
    1 | (1 << 1) | (1 << 2) | (1 << 5) | (1 << 11) | (1 << 35);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeSide {
    Client,
    Server,
}
impl RuntimeSide {
    pub fn context_code(self) -> u32 {
        match self {
            RuntimeSide::Client => 1,
            RuntimeSide::Server => 2,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CallbackPhase {
    Init,
    WorldEnter,
    Tick,
    WorldExit,
    Shutdown,
}
impl CallbackPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            CallbackPhase::Init => "init",
            CallbackPhase::WorldEnter => "world_enter",
            CallbackPhase::Tick => "tick",
            CallbackPhase::WorldExit => "world_exit",
            CallbackPhase::Shutdown => "shutdown",
        }
    }
    pub fn context_code(self) -> u32 {
        match self {
            CallbackPhase::Init => 1,
            CallbackPhase::WorldEnter => 2,
            CallbackPhase::Tick => 3,
            CallbackPhase::WorldExit => 4,
            CallbackPhase::Shutdown => 5,
        }
    }
}
impl fmt::Display for CallbackPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ServiceOpcode {
    Context,
    Log,
}
impl ServiceOpcode {
    pub fn code(self) -> i32 {
        match self {
            ServiceOpcode::Context => CONTEXT_OPCODE,
            ServiceOpcode::Log => LOG_OPCODE,
        }
    }
    pub fn capability(self) -> &'static str {
        match self {
            ServiceOpcode::Context => HOST_CONTEXT_CAPABILITY,
            ServiceOpcode::Log => HOST_LOG_CAPABILITY,
        }
    }
    pub fn allowed_in(self, _phase: CallbackPhase) -> bool {
        match self {
            ServiceOpcode::Context | ServiceOpcode::Log => true,
        }
    }
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            CONTEXT_OPCODE => Some(ServiceOpcode::Context),
            LOG_OPCODE => Some(ServiceOpcode::Log),
            _ => None,
        }
    }
    pub fn from_capability(name: &CapabilityName) -> Option<Self> {
        match name.as_str() {
            HOST_CONTEXT_CAPABILITY => Some(ServiceOpcode::Context),
            HOST_LOG_CAPABILITY => Some(ServiceOpcode::Log),
            _ => None,
        }
    }
    pub fn authorize(
        self,
        _side: RuntimeSide,
        phase: CallbackPhase,
        grants: &NegotiatedCapabilities,
    ) -> Result<(), AuthorizationError> {
        if !grants.contains_name(self.capability()) {
            return Err(AuthorizationError::CapabilityNotGranted { opcode: self });
        }
        if !self.allowed_in(phase) {
            return Err(AuthorizationError::PhaseNotAllowed {
                opcode: self,
                phase,
            });
        }
        Ok(())
    }
}
pub fn is_prototype_service(name: &CapabilityName) -> bool {
    PROTOTYPE_CAPABILITIES.contains(&name.as_str())
}
pub fn is_reserved_future_service(name: &CapabilityName) -> bool {
    RESERVED_FUTURE_CAPABILITIES.contains(&name.as_str())
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiatedCapabilities {
    granted: BTreeSet<CapabilityName>,
    disabled_optional: Vec<CapabilityName>,
}
impl NegotiatedCapabilities {
    pub fn granted(&self) -> &BTreeSet<CapabilityName> {
        &self.granted
    }
    pub fn disabled_optional(&self) -> &[CapabilityName] {
        &self.disabled_optional
    }
    pub fn contains(&self, name: &CapabilityName) -> bool {
        self.granted.contains(name)
    }
    pub fn contains_name(&self, name: &str) -> bool {
        self.granted.iter().any(|n| n.as_str() == name)
    }
}
pub fn negotiate(
    required: &[CapabilityName],
    optional: &[CapabilityName],
    advertised: &BTreeSet<CapabilityName>,
    authorized: &BTreeSet<CapabilityName>,
) -> Result<NegotiatedCapabilities, NegotiationError> {
    for (list, items) in [("required", required), ("optional", optional)] {
        if items.len() > MAX_MANIFEST_CAPABILITIES {
            return Err(NegotiationError::TooManyManifestEntries {
                list,
                count: items.len(),
            });
        }
        let mut seen = BTreeSet::new();
        for name in items {
            if !seen.insert(name) {
                return Err(NegotiationError::DuplicateEntry {
                    list,
                    name: name.clone(),
                });
            }
        }
    }
    if advertised.len() > MAX_HOST_CATALOG {
        return Err(NegotiationError::HostCatalogTooLarge {
            count: advertised.len(),
        });
    }
    if authorized.len() > MAX_HOST_CATALOG {
        return Err(NegotiationError::HostCatalogTooLarge {
            count: authorized.len(),
        });
    }
    for name in required {
        if optional.contains(name) {
            return Err(NegotiationError::RequiredOverlapsOptional { name: name.clone() });
        }
    }
    let mut unavailable = Vec::new();
    let mut denied = Vec::new();
    for name in required {
        if !advertised.contains(name) {
            unavailable.push(name.clone());
        } else if !authorized.contains(name) {
            denied.push(name.clone());
        }
    }
    unavailable.sort();
    denied.sort();
    if !unavailable.is_empty() || !denied.is_empty() {
        return Err(NegotiationError::MissingRequired {
            unavailable,
            denied,
        });
    }
    let mut granted = BTreeSet::new();
    for name in required.iter().chain(optional.iter()) {
        if advertised.contains(name) && authorized.contains(name) {
            granted.insert(name.clone());
        }
    }
    let mut disabled_optional = Vec::new();
    for name in optional {
        if !granted.contains(name) {
            disabled_optional.push(name.clone());
        }
    }
    disabled_optional.sort();
    Ok(NegotiatedCapabilities {
        granted,
        disabled_optional,
    })
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NegotiationError {
    TooManyManifestEntries {
        list: &'static str,
        count: usize,
    },
    DuplicateEntry {
        list: &'static str,
        name: CapabilityName,
    },
    RequiredOverlapsOptional {
        name: CapabilityName,
    },
    HostCatalogTooLarge {
        count: usize,
    },
    MissingRequired {
        unavailable: Vec<CapabilityName>,
        denied: Vec<CapabilityName>,
    },
}
impl fmt::Display for NegotiationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NegotiationError::TooManyManifestEntries { list, count } => {
                write!(f, "capability list '{list}' has {count} entries")
            }
            NegotiationError::DuplicateEntry { list, name } => {
                write!(f, "duplicate '{name}' in '{list}' capabilities")
            }
            NegotiationError::RequiredOverlapsOptional { name } => {
                write!(f, "capability '{name}' is both required and optional")
            }
            NegotiationError::HostCatalogTooLarge { count } => {
                write!(f, "host catalog has {count} entries")
            }
            NegotiationError::MissingRequired {
                unavailable,
                denied,
            } => {
                write!(f, "missing required: unavailable [")?;
                write_list(f, unavailable)?;
                f.write_str("], policy-denied [")?;
                write_list(f, denied)?;
                f.write_str("]")
            }
        }
    }
}
fn write_list(f: &mut fmt::Formatter<'_>, items: &[CapabilityName]) -> fmt::Result {
    for (i, name) in items.iter().enumerate() {
        if i > 0 {
            f.write_str(", ")?;
        }
        write!(f, "{name}")?;
    }
    Ok(())
}
impl std::error::Error for NegotiationError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AuthorizationError {
    CapabilityNotGranted {
        opcode: ServiceOpcode,
    },
    PhaseNotAllowed {
        opcode: ServiceOpcode,
        phase: CallbackPhase,
    },
}
impl fmt::Display for AuthorizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthorizationError::CapabilityNotGranted { opcode } => {
                write!(f, "opcode {} is not granted", opcode.code())
            }
            AuthorizationError::PhaseNotAllowed { opcode, phase } => {
                write!(
                    f,
                    "opcode {} is not allowed in phase {phase}",
                    opcode.code()
                )
            }
        }
    }
}
impl std::error::Error for AuthorizationError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WasmFeatureDecision {
    ProposedAllowed,
    Denied,
}
impl WasmFeatureDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            WasmFeatureDecision::ProposedAllowed => "proposed-allowed",
            WasmFeatureDecision::Denied => "denied",
        }
    }
}
pub fn baseline_feature(name: &str) -> WasmFeatureDecision {
    match name {
        "mvp"
        | "mutable-globals"
        | "sign-extension-ops"
        | "saturating-float-to-int"
        | "bulk-memory"
        | "bulk-memory-opt"
        | "floats"
        | "guest-funcref-table" => WasmFeatureDecision::ProposedAllowed,
        _ => WasmFeatureDecision::Denied,
    }
}
