use crate::admission;
use crate::host::{Context, Effects, Host, Snapshot};
use crate::lifecycle::Phase;
use crate::policy::Policy;
use el_extension_api::abi::{ErrorCode, HOST_CALL_FN, HOST_MODULE, HostCallLayout};
use serde::{Deserialize, Serialize};
use wasmtime::{
    Caller, Config, Engine, Extern, Instance, Linker, Module, Store, StoreLimits,
    StoreLimitsBuilder,
};
struct State {
    host: Host,
    limits: StoreLimits,
}
fn trunc(s: &str) -> String {
    s.chars().take(512).collect()
}
fn host_call(mut caller: Caller<'_, State>, op: i32, rp: i32, rl: i32, wp: i32, wl: i32) -> i32 {
    let memory = match caller.get_export("memory") {
        Some(Extern::Memory(m)) => m,
        _ => return ErrorCode::HostFailure.code(),
    };
    let size = memory.data_size(&caller) as u64;
    let layout = match HostCallLayout::validate(rp, rl, wp, wl, size) {
        Ok(l) => l,
        Err(e) => return e.code(),
    };
    let req = match layout.request().resolve(size) {
        Ok(r) => r,
        Err(e) => return e.code(),
    };
    let response = match layout.response().resolve(size) {
        Ok(r) => r,
        Err(e) => return e.code(),
    };
    let request = memory.data(&caller)[req].to_vec();
    let (bytes, state) = memory.data_and_store_mut(&mut caller);
    state.host.dispatch(op, &request, &mut bytes[response])
}
pub fn new_engine(policy: &Policy) -> Result<Engine, String> {
    policy.validate()?;
    let mut c = Config::new();
    c.consume_fuel(true);
    c.max_wasm_stack(policy.stack_bytes);
    c.memory_reservation(64 * 1024 * 1024);
    c.memory_guard_size(65536);
    Engine::new(&c).map_err(|e| trunc(&format!("{e:#}")))
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Outcome {
    pub code: Option<i32>,
    pub error: Option<String>,
    pub effects: Effects,
    pub fuel_used: u64,
    pub memory_bytes: u64,
}
impl Outcome {
    pub fn success(&self) -> bool {
        self.code == Some(0) && self.error.is_none()
    }
}
pub struct NativeActor {
    store: Store<State>,
    instance: Instance,
    policy: Policy,
    failed: bool,
}
impl NativeActor {
    pub fn new(
        engine: &Engine,
        bytes: &[u8],
        grants: Vec<String>,
        policy: Policy,
    ) -> Result<Self, String> {
        policy.validate().map_err(|e| trunc(&e))?;
        admission::validate(bytes).map_err(|e| trunc(&format!("{e:#}")))?;
        let module = Module::new(engine, bytes).map_err(|e| trunc(&format!("{e:#}")))?;
        let limits = StoreLimitsBuilder::new()
            .memory_size(64 * 1024 * 1024)
            .memories(1)
            .tables(1)
            .table_elements(1024)
            .instances(1)
            .build();
        let mut store = Store::new(
            engine,
            State {
                host: Host::new(grants),
                limits,
            },
        );
        store.limiter(|s| &mut s.limits);
        store
            .set_fuel(policy.init_fuel)
            .map_err(|e| trunc(&format!("{e:#}")))?;
        let mut l = Linker::new(engine);
        l.func_wrap(HOST_MODULE, HOST_CALL_FN, host_call)
            .map_err(|e| trunc(&format!("{e:#}")))?;
        let instance = l
            .instantiate(&mut store, &module)
            .map_err(|e| trunc(&format!("{e:#}")))?;
        Ok(Self {
            store,
            instance,
            policy,
            failed: false,
        })
    }
    pub fn invoke(&mut self, context: Context, snapshot: Snapshot) -> Result<Outcome, String> {
        if self.failed {
            return Err("actor failed; no retry".to_string());
        }
        let export = context.phase.export().to_string();
        let optional = export == "on_world_enter" || export == "on_world_exit";
        let fuel0 = if context.phase == Phase::Init {
            self.policy.init_fuel
        } else {
            self.policy.callback_fuel
        };
        self.store
            .set_fuel(fuel0)
            .map_err(|e| trunc(&format!("{e:#}")))?;
        self.store
            .data_mut()
            .host
            .begin(context.clone(), snapshot)
            .map_err(|e| trunc(&e))?;
        if context.phase == Phase::AppTick {
            let effects = self.store.data_mut().host.finish(true)?;
            return Ok(Outcome {
                code: Some(0),
                error: None,
                effects,
                fuel_used: 0,
                memory_bytes: mem_of_bytes(&mut self.store, &self.instance),
            });
        }
        let f = match self
            .instance
            .get_typed_func::<(), i32>(&mut self.store, &export)
        {
            Ok(f) => f,
            Err(_) if optional && self.instance.get_func(&mut self.store, &export).is_none() => {
                let effects = self
                    .store
                    .data_mut()
                    .host
                    .finish(true)
                    .map_err(|e| trunc(&e))?;
                let rem = self.store.get_fuel().unwrap_or(0);
                let mb = mem_of_bytes(&mut self.store, &self.instance);
                return Ok(Outcome {
                    code: Some(0),
                    error: None,
                    effects,
                    fuel_used: fuel0.saturating_sub(rem),
                    memory_bytes: mb,
                });
            }
            Err(e) => {
                let es = trunc(&format!("{e:#}"));
                let effects = self
                    .store
                    .data_mut()
                    .host
                    .finish(false)
                    .map_err(|x| trunc(&x))?;
                self.failed = true;
                let rem = self.store.get_fuel().unwrap_or(0);
                let mb = mem_of_bytes(&mut self.store, &self.instance);
                return Ok(Outcome {
                    code: None,
                    error: Some(es),
                    effects,
                    fuel_used: fuel0.saturating_sub(rem),
                    memory_bytes: mb,
                });
            }
        };
        match f.call(&mut self.store, ()) {
            Ok(code) => {
                let ok = code == 0;
                let err = if ok {
                    None
                } else {
                    Some(format!("nonzero exit: {code}"))
                };
                let effects = self
                    .store
                    .data_mut()
                    .host
                    .finish(ok)
                    .map_err(|e| trunc(&e))?;
                if !ok {
                    self.failed = true;
                }
                let rem = self.store.get_fuel().unwrap_or(0);
                let mb = mem_of_bytes(&mut self.store, &self.instance);
                Ok(Outcome {
                    code: Some(code),
                    error: err,
                    effects,
                    fuel_used: fuel0.saturating_sub(rem),
                    memory_bytes: mb,
                })
            }
            Err(e) => {
                let es = trunc(&format!("{e:#}"));
                let effects = self
                    .store
                    .data_mut()
                    .host
                    .finish(false)
                    .map_err(|x| trunc(&x))?;
                self.failed = true;
                let rem = self.store.get_fuel().unwrap_or(0);
                let mb = mem_of_bytes(&mut self.store, &self.instance);
                Ok(Outcome {
                    code: None,
                    error: Some(es),
                    effects,
                    fuel_used: fuel0.saturating_sub(rem),
                    memory_bytes: mb,
                })
            }
        }
    }
}
fn mem_of_bytes(store: &mut Store<State>, inst: &Instance) -> u64 {
    match inst.get_export(&mut *store, "memory") {
        Some(Extern::Memory(m)) => m.data_size(&*store) as u64,
        _ => 0,
    }
}
