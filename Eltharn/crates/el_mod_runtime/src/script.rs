use crate::host::{Context, Effects, Host, Snapshot};
use crate::lifecycle::Phase;
use crate::policy::{MAX_PAYLOAD_BYTES, Policy};
use el_extension_api::services;
use rhai::{
    AST, Array, Blob, CallFnOptions, Dynamic, Engine, EvalAltResult, INT, ImmutableString, Map,
    Module, ModuleResolver, OptimizationLevel, Position, Scope, Shared,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub const MEMORY_RESERVATION: u64 = 64 * 1024 * 1024;
pub const MAX_SOURCE_FILES: usize = 64;
pub const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_FILE_BYTES: usize = 256 * 1024;
const MAX_IMPORT_DEPTH: usize = 16;
const MAX_DATA_DEPTH: usize = 32;
const MAX_DATA_NODES: usize = 16384;
const MAX_STATE_BYTES: usize = 1024 * 1024;
type ScriptResult<T> = Result<T, Box<EvalAltResult>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptBundle {
    pub entry: String,
    pub sources: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hooks: Option<el_extension_api::hooks::HookModule>,
}

fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 240
        && path.ends_with(".rhai")
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b))
}

impl ScriptBundle {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_path(&self.entry) || !self.sources.contains_key(&self.entry) {
            return Err("Script entry must name a packaged .rhai source".into());
        }
        if self.sources.is_empty() || self.sources.len() > MAX_SOURCE_FILES {
            return Err("Script source file count limit".into());
        }
        let mut total = 0usize;
        for (path, source) in &self.sources {
            if !valid_path(path) || source.len() > MAX_FILE_BYTES || source.contains('\0') {
                return Err(format!(
                    "Invalid script path/source or file size limit: {path}"
                ));
            }
            total = total
                .checked_add(source.len())
                .ok_or("Script source size overflow")?;
        }
        if total > MAX_SOURCE_BYTES {
            return Err("Script source byte limit".into());
        }
        if let Some(hooks) = &self.hooks {
            hooks.validate()?;
            if !self.sources.contains_key(&hooks.script) {
                return Err("Hook source is absent from package".into());
            }
        }
        Ok(())
    }

    pub fn validate_program(&self) -> Result<BTreeSet<String>, String> {
        self.validate()?;
        crate::hooks::validate_bundle(self)?;
        let engine = bounded_engine();
        compile_bundle(self, &engine).map(|(_, _, exports)| exports)
    }
}

fn bounded_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_optimization_level(OptimizationLevel::None);
    engine.set_max_operations(Policy::default().init_fuel);
    engine.set_max_call_levels(16);
    engine.set_max_expr_depths(32, 32);
    engine.set_max_variables(128);
    engine.set_max_functions(1024);
    engine.set_max_modules(MAX_SOURCE_FILES);
    engine.set_max_string_size(MAX_PAYLOAD_BYTES);
    engine.set_max_array_size(128);
    engine.set_max_map_size(128);
    engine.set_max_strings_interned(4096);
    for symbol in ["eval", "print", "debug", "Fn"] {
        engine.disable_symbol(symbol);
    }
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    engine
}

fn compile_bundle(
    bundle: &ScriptBundle,
    engine: &Engine,
) -> Result<(BTreeMap<String, AST>, AST, BTreeSet<String>), String> {
    bundle.validate()?;
    let mut asts = BTreeMap::new();
    for (path, source) in &bundle.sources {
        let mut ast = engine
            .compile(source)
            .map_err(|e| diagnostic(format!("{path}: {e}")))?;
        ast.set_source(path);
        asts.insert(path.clone(), ast);
    }
    let entry_ast = asts.get(&bundle.entry).unwrap().clone();
    let callbacks: BTreeSet<_> = entry_ast
        .iter_functions()
        .filter(|f| f.params.is_empty() && f.access == rhai::FnAccess::Public)
        .map(|f| f.name.to_owned())
        .collect();
    for function in entry_ast.iter_functions() {
        if [
            "init",
            "tick",
            "app_tick",
            "shutdown",
            "on_world_enter",
            "on_world_exit",
        ]
        .contains(&function.name)
            && (function.access != rhai::FnAccess::Public || !function.params.is_empty())
        {
            return Err(format!(
                "{}: lifecycle {} must be public with zero arguments",
                bundle.entry, function.name
            ));
        }
    }
    for required in ["init", "tick", "shutdown"] {
        if !callbacks.contains(required) {
            return Err(format!(
                "{}: missing public zero-argument {required}()",
                bundle.entry
            ));
        }
    }
    Ok((asts, entry_ast, callbacks))
}

fn script_error(message: impl Into<String>) -> Box<EvalAltResult> {
    EvalAltResult::ErrorRuntime(message.into().into(), Position::NONE).into()
}
fn diagnostic(message: impl std::fmt::Display) -> String {
    message.to_string().chars().take(1024).collect()
}

struct BundleResolver {
    asts: BTreeMap<String, AST>,
    loaded: Mutex<BTreeMap<String, Shared<Module>>>,
    loading: Mutex<Vec<String>>,
}
impl ModuleResolver for BundleResolver {
    fn resolve(
        &self,
        engine: &Engine,
        _source: Option<&str>,
        path: &str,
        pos: Position,
    ) -> ScriptResult<Shared<Module>> {
        if !valid_path(path) {
            return Err(script_error("Import must be a package-root .rhai path"));
        }
        if let Some(module) = self
            .loaded
            .lock()
            .map_err(|_| script_error("Import cache poisoned"))?
            .get(path)
        {
            return Ok(module.clone());
        }
        let ast = self
            .asts
            .get(path)
            .ok_or_else(|| Box::new(EvalAltResult::ErrorModuleNotFound(path.into(), pos)))?;
        {
            let mut loading = self
                .loading
                .lock()
                .map_err(|_| script_error("Import stack poisoned"))?;
            if loading.len() >= MAX_IMPORT_DEPTH || loading.iter().any(|p| p == path) {
                return Err(script_error(format!(
                    "Cyclic import or import depth limit: {path}"
                )));
            }
            loading.push(path.into());
        }
        let result = Module::eval_ast_as_new(Scope::new(), ast, engine);
        self.loading
            .lock()
            .map_err(|_| script_error("Import stack poisoned"))?
            .pop();
        let module: Shared<Module> = result
            .map_err(|error| Box::new(EvalAltResult::ErrorInModule(path.into(), error, pos)))?
            .into();
        self.loaded
            .lock()
            .map_err(|_| script_error("Import cache poisoned"))?
            .insert(path.into(), module.clone());
        Ok(module)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScriptOutcome {
    pub code: Option<i32>,
    pub error: Option<String>,
    pub effects: Effects,
    pub fuel_used: u64,
    pub memory_bytes: u64,
}
impl ScriptOutcome {
    pub fn success(&self) -> bool {
        self.code == Some(0) && self.error.is_none()
    }
}

fn data_size(value: &Dynamic, depth: usize, nodes: &mut usize) -> ScriptResult<usize> {
    *nodes += 1;
    if depth > MAX_DATA_DEPTH || *nodes > MAX_DATA_NODES {
        return Err(script_error("Script data depth/node limit"));
    }
    if value.is::<Map>() {
        let map = value
            .read_lock::<Map>()
            .ok_or_else(|| script_error("Invalid map"))?;
        let mut bytes = 2usize;
        for (key, value) in map.iter() {
            bytes = bytes.saturating_add(key.len() + data_size(value, depth + 1, nodes)? + 4);
            if bytes > MAX_STATE_BYTES {
                return Err(script_error("Script data byte limit"));
            }
        }
        Ok(bytes)
    } else if value.is::<Array>() {
        let array = value
            .read_lock::<Array>()
            .ok_or_else(|| script_error("Invalid array"))?;
        let mut bytes = 2usize;
        for value in array.iter() {
            bytes = bytes.saturating_add(data_size(value, depth + 1, nodes)? + 1);
            if bytes > MAX_STATE_BYTES {
                return Err(script_error("Script data byte limit"));
            }
        }
        Ok(bytes)
    } else if value.is::<ImmutableString>() {
        Ok(value
            .read_lock::<ImmutableString>()
            .ok_or_else(|| script_error("Invalid string"))?
            .len()
            + 2)
    } else if value.is::<Blob>() {
        Ok(value
            .read_lock::<Blob>()
            .ok_or_else(|| script_error("Invalid blob"))?
            .len()
            * 4
            + 2)
    } else if value.is_unit()
        || value.is::<INT>()
        || value.is::<bool>()
        || value.is::<rhai::FLOAT>()
        || value.is::<char>()
    {
        Ok(24)
    } else {
        Err(script_error(
            "Only data values may cross the host boundary or persist in actor state",
        ))
    }
}

fn encode(value: &Dynamic) -> ScriptResult<Vec<u8>> {
    if data_size(value, 0, &mut 0)? > MAX_PAYLOAD_BYTES {
        return Err(script_error("Host request byte limit"));
    }
    let value: serde_json::Value = rhai::serde::from_dynamic(value)?;
    let bytes = serde_json::to_vec(&value).map_err(|e| script_error(e.to_string()))?;
    if bytes.len() > MAX_PAYLOAD_BYTES {
        return Err(script_error("Host request byte limit"));
    }
    Ok(bytes)
}

fn dispatch(host: &Arc<Mutex<Host>>, operation: i32, request: &[u8]) -> ScriptResult<Vec<u8>> {
    if request.len() > MAX_PAYLOAD_BYTES {
        return Err(script_error("Host request byte limit"));
    }
    let mut response = vec![0; MAX_PAYLOAD_BYTES];
    let code = host
        .lock()
        .map_err(|_| script_error("Host lock poisoned"))?
        .dispatch(operation, request, &mut response);
    if code < 0 {
        return Err(script_error(format!(
            "Host operation 0x{operation:x} failed ({code}); check capability, phase and request"
        )));
    }
    response.truncate(code as usize);
    Ok(response)
}

fn decode(bytes: &[u8]) -> ScriptResult<Dynamic> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| script_error(e.to_string()))?;
    rhai::serde::to_dynamic(value)
}

fn register_host(engine: &mut Engine, host: Arc<Mutex<Host>>) {
    engine.register_fn(
        "host_keybinding_name",
        |physical: ImmutableString| -> Dynamic {
            crate::keybindings::binding_from_physical(physical.as_str())
                .map(Dynamic::from)
                .unwrap_or(Dynamic::UNIT)
        },
    );
    let h = host.clone();
    engine.register_fn(
        "host_keybindings_register",
        move |bindings: rhai::Array| -> ScriptResult<()> {
            dispatch(
                &h,
                crate::keybindings::REGISTER_OPCODE,
                &encode(&bindings.into())?,
            )
            .map(|_| ())
        },
    );
    let h = host.clone();
    engine.register_fn(
        "host_overlay_message",
        move |segments: rhai::Array| -> ScriptResult<()> {
            dispatch(&h, crate::overlay::WRITE_OPCODE, &encode(&segments.into())?).map(|_| ())
        },
    );
    let h = host.clone();
    engine.register_fn(
        "host_keybinding",
        move |id: ImmutableString| -> ScriptResult<Dynamic> {
            decode(&dispatch(
                &h,
                crate::keybindings::READ_OPCODE,
                id.as_bytes(),
            )?)
        },
    );
    let h = host.clone();
    engine.register_fn(
        "host_buffer",
        move |request: Map| -> ScriptResult<Dynamic> {
            decode(&dispatch(
                &h,
                crate::binary::OPCODE,
                &encode(&request.into())?,
            )?)
        },
    );
    let h = host.clone();
    engine.register_fn("host_hook_state", move |value: Map| -> ScriptResult<()> {
        dispatch(&h, crate::hook_state::OPCODE, &encode(&value.into())?).map(|_| ())
    });
    let h = host.clone();
    engine.register_fn("host_app_read", move || -> ScriptResult<Dynamic> {
        decode(&dispatch(&h, services::CLIENT_APP_READ_OPCODE, &[])?)
    });
    let h = host.clone();
    engine.register_fn("host_cape_reset", move || -> ScriptResult<()> {
        dispatch(&h, services::CLIENT_AVATAR_WRITE_OPCODE, &[]).map(|_| ())
    });
    let h = host.clone();
    engine.register_fn("host_app_ui", move |ui: Map| -> ScriptResult<()> {
        dispatch(&h, services::CLIENT_APP_WRITE_OPCODE, &encode(&ui.into())?).map(|_| ())
    });
    let h = host.clone();
    engine.register_fn("host_app_menu", move |menu: Map| -> ScriptResult<()> {
        dispatch(&h, services::CLIENT_APP_MENU_OPCODE, &encode(&menu.into())?).map(|_| ())
    });
    let h = host.clone();
    engine.register_fn(
        "host_app_event",
        move |index: INT| -> ScriptResult<Dynamic> {
            let index =
                u32::try_from(index).map_err(|_| script_error("Invalid app event index"))?;
            decode(&dispatch(
                &h,
                services::CLIENT_APP_EVENT_OPCODE,
                &index.to_le_bytes(),
            )?)
        },
    );
    let h = host.clone();
    engine.register_fn(
        "host_config_read",
        move |key: ImmutableString| -> ScriptResult<Dynamic> {
            let request = serde_json::to_vec(&serde_json::json!({"key":key.as_str()}))
                .map_err(|e| script_error(e.to_string()))?;
            decode(&dispatch(&h, services::CONFIG_READ_OPCODE, &request)?)
        },
    );
    let h = host.clone();
    engine.register_fn(
        "host_config_write",
        move |key: ImmutableString, value: Dynamic| -> ScriptResult<()> {
            let bytes = encode(&value)?;
            let value: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|e| script_error(e.to_string()))?;
            let request =
                serde_json::to_vec(&serde_json::json!({"key":key.as_str(),"value":value}))
                    .map_err(|e| script_error(e.to_string()))?;
            dispatch(&h, services::CONFIG_WRITE_OPCODE, &request).map(|_| ())
        },
    );
    let h = host.clone();
    engine.register_fn(
        "host_config_delete",
        move |key: ImmutableString| -> ScriptResult<()> {
            let request = serde_json::to_vec(&serde_json::json!({"key":key.as_str()}))
                .map_err(|e| script_error(e.to_string()))?;
            dispatch(&h, services::CONFIG_DELETE_OPCODE, &request).map(|_| ())
        },
    );
    let h = host.clone();
    engine.register_fn("host_config_status", move || -> ScriptResult<Dynamic> {
        decode(&dispatch(&h, services::CONFIG_STATUS_OPCODE, &[])?)
    });
    let h = host.clone();
    engine.register_fn(
        "host_log",
        move |message: ImmutableString| -> ScriptResult<()> {
            dispatch(&h, services::LOG_OPCODE, message.as_bytes()).map(|_| ())
        },
    );
    let h = host.clone();
    engine.register_fn("host_context", move || -> ScriptResult<Dynamic> {
        let bytes = dispatch(&h, services::CONTEXT_OPCODE, &[])?;
        let integer = |start: usize| -> ScriptResult<INT> {
            let value = u64::from_le_bytes(bytes[start..start + 8].try_into().unwrap());
            INT::try_from(value).map_err(|_| script_error("Context integer overflow"))
        };
        let mut map = Map::new();
        map.insert(
            "side".into(),
            INT::from(u32::from_le_bytes(bytes[0..4].try_into().unwrap())).into(),
        );
        map.insert(
            "phase".into(),
            INT::from(u32::from_le_bytes(bytes[4..8].try_into().unwrap())).into(),
        );
        map.insert("tick".into(), integer(8)?.into());
        map.insert("callback_id".into(), integer(16)?.into());
        Ok(map.into())
    });
    for (name, opcode) in [
        ("host_client_ui", services::CLIENT_UI_WRITE_OPCODE),
        ("host_content", services::CONTENT_OPCODE),
        ("host_world_surface", services::WORLD_SURFACE_WATCH_OPCODE),
        ("host_message", services::HOST_MESSAGING_OPCODE),
        ("host_job", services::HOST_JOBS_OPCODE),
        ("host_network", services::NETWORK_SEND_OPCODE),
        ("host_cape", services::CLIENT_AVATAR_WRITE_OPCODE),
        ("host_camera", services::CLIENT_CAMERA_WRITE_OPCODE),
        ("host_world_watch", services::WORLD_BLOCKS_WATCH_OPCODE),
        ("host_world_region", services::WORLD_REGION_WATCH_OPCODE),
        ("host_canvas", services::CLIENT_CANVAS_WRITE_OPCODE),
    ] {
        let h = host.clone();
        engine.register_fn(name, move |request: Map| -> ScriptResult<()> {
            dispatch(&h, opcode, &encode(&request.into())?).map(|_| ())
        });
    }
    for (name, opcode) in [
        ("host_client_input", services::CLIENT_INPUT_READ_OPCODE),
        ("host_content_read", services::CONTENT_CLIENT_READ_OPCODE),
        ("host_avatar_read", services::CLIENT_AVATAR_READ_OPCODE),
        ("host_player_read", services::CLIENT_PLAYER_READ_OPCODE),
        ("host_surface_read", services::WORLD_SURFACE_READ_OPCODE),
        (
            "host_server_content_read",
            services::CONTENT_SERVER_READ_OPCODE,
        ),
        ("host_camera_read", services::CLIENT_CAMERA_READ_OPCODE),
        ("host_world_region_read", services::WORLD_REGION_READ_OPCODE),
    ] {
        let h = host.clone();
        engine.register_fn(name, move || -> ScriptResult<Dynamic> {
            decode(&dispatch(&h, opcode, &[])?)
        });
    }
    for (name, opcode) in [
        ("host_client_events", services::CLIENT_INPUT_EVENTS_OPCODE),
        ("host_entities_read", services::CLIENT_ENTITIES_READ_OPCODE),
        ("host_surface_page", services::WORLD_SURFACE_PAGE_OPCODE),
        ("host_surface_state", services::WORLD_SURFACE_STATE_OPCODE),
        ("host_world_read", services::WORLD_BLOCKS_READ_OPCODE),
        ("host_world_page", services::WORLD_REGION_PAGE_OPCODE),
        ("host_world_palette", services::WORLD_REGION_PALETTE_OPCODE),
    ] {
        let h = host.clone();
        engine.register_fn(name, move |request: Map| -> ScriptResult<Dynamic> {
            decode(&dispatch(&h, opcode, &encode(&request.into())?)?)
        });
    }
    let h = host.clone();
    engine.register_fn("host_messages", move || -> ScriptResult<Dynamic> {
        decode(&dispatch(
            &h,
            services::HOST_MESSAGING_OPCODE,
            br#"{"op":"poll"}"#,
        )?)
    });
}

pub struct ScriptActor {
    engine: Engine,
    entry_ast: AST,
    host: Arc<Mutex<Host>>,
    state: Dynamic,
    callbacks: BTreeSet<String>,
    policy: Policy,
    operations: Arc<AtomicU64>,
    operation_limit: Arc<AtomicU64>,
    source_bytes: u64,
    initialized: bool,
    failed: bool,
}

impl ScriptActor {
    pub fn new(bundle: ScriptBundle, grants: Vec<String>, policy: Policy) -> Result<Self, String> {
        policy.validate()?;
        let source_bytes = bundle.sources.values().map(|s| s.len() as u64).sum();
        let host = Arc::new(Mutex::new(Host::new(grants)));
        let operations = Arc::new(AtomicU64::new(0));
        let operation_limit = Arc::new(AtomicU64::new(policy.init_fuel));
        let mut engine = bounded_engine();
        let count = operations.clone();
        let limit = operation_limit.clone();
        engine.on_progress(move |_| {
            (count.fetch_add(1, Ordering::Relaxed) >= limit.load(Ordering::Relaxed))
                .then(|| Dynamic::from("Script operation budget exceeded"))
        });
        register_host(&mut engine, host.clone());
        let (asts, entry_ast, callbacks) = compile_bundle(&bundle, &engine)?;
        engine.set_module_resolver(BundleResolver {
            asts,
            loaded: Mutex::new(BTreeMap::new()),
            loading: Mutex::new(Vec::new()),
        });
        Ok(Self {
            engine,
            entry_ast,
            host,
            state: Map::new().into(),
            callbacks,
            policy,
            operations,
            operation_limit,
            source_bytes,
            initialized: false,
            failed: false,
        })
    }

    pub fn invoke(
        &mut self,
        context: Context,
        snapshot: Snapshot,
    ) -> Result<ScriptOutcome, String> {
        if self.failed {
            return Err("actor failed; no retry".into());
        }
        if (context.phase == Phase::Init) == self.initialized {
            return Err("Script actor requires init exactly once before callbacks".into());
        }
        let phase = context.phase;
        let limit = if phase == Phase::Init {
            self.policy.init_fuel
        } else {
            self.policy.callback_fuel
        };
        self.operations.store(0, Ordering::Relaxed);
        self.operation_limit.store(limit, Ordering::Relaxed);
        self.engine.set_max_operations(limit);
        self.host
            .lock()
            .map_err(|_| "Host lock poisoned")?
            .begin(context, snapshot)?;
        let result = (|| -> ScriptResult<Dynamic> {
            if !self.initialized {
                let module = Module::eval_ast_as_new(Scope::new(), &self.entry_ast, &self.engine)?;
                self.engine.register_global_module(module.into());
                self.initialized = true;
            }
            if !self.callbacks.contains(phase.export()) {
                return Ok(Dynamic::UNIT);
            }
            self.engine.call_fn_with_options(
                CallFnOptions::new()
                    .eval_ast(false)
                    .in_all_namespaces(true)
                    .bind_this_ptr(&mut self.state),
                &mut Scope::new(),
                &AST::empty(),
                phase.export(),
                (),
            )
        })();
        let (mut code, mut error) = match result {
            Ok(value) if value.is_unit() => (Some(0), None),
            Ok(value) => match value.try_cast::<INT>().and_then(|n| i32::try_from(n).ok()) {
                Some(0) => (Some(0), None),
                Some(code) => (
                    Some(code),
                    Some(format!("{}: nonzero exit {code}", phase.export())),
                ),
                None => (
                    None,
                    Some(format!(
                        "{} must return () or an i32 status",
                        phase.export()
                    )),
                ),
            },
            Err(error) => (
                None,
                Some(diagnostic(format!("{}: {error}", phase.export()))),
            ),
        };
        let state_bytes = match data_size(&self.state, 0, &mut 0) {
            Ok(bytes) if bytes <= MAX_STATE_BYTES => bytes as u64,
            result => {
                code = None;
                let state_error = diagnostic(match result {
                    Err(e) => e.to_string(),
                    _ => "Persistent script state byte limit".into(),
                });
                error = Some(match error {
                    Some(previous) => diagnostic(format!("{previous}; {state_error}")),
                    None => state_error,
                });
                0
            }
        };
        let success = code == Some(0) && error.is_none();
        let effects = self
            .host
            .lock()
            .map_err(|_| "Host lock poisoned")?
            .finish(success)?;
        self.failed = !success;
        Ok(ScriptOutcome {
            code,
            error,
            effects,
            fuel_used: self.operations.load(Ordering::Relaxed),
            memory_bytes: self.source_bytes + state_bytes,
        })
    }
}
