#![forbid(unsafe_code)]
use el_mod_runtime::{
    host::{Context, Host, Snapshot},
    lifecycle::Phase,
    policy::Policy,
    session::{ActorConfig, Completion, Controller},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;
fn err(e: impl ToString) -> JsValue {
    JsValue::from_str(&e.to_string())
}
fn parse<T: DeserializeOwned>(text: &str, limit: usize) -> Result<T, JsValue> {
    let v = el_mod_runtime::wire::strict_json(text.as_bytes(), limit).map_err(err)?;
    serde_json::from_value(v).map_err(err)
}
fn json(v: &impl Serialize) -> Result<String, JsValue> {
    serde_json::to_string(v).map_err(err)
}

#[wasm_bindgen]
pub fn admit(bytes: &[u8]) -> Result<u32, JsValue> {
    el_mod_runtime::policy::memory_reservation(bytes)
        .map(|n| n as u32)
        .map_err(err)
}

const SCRIPT_JSON_LIMIT: usize = el_mod_runtime::script::MAX_SOURCE_BYTES * 6 + 65536;

#[wasm_bindgen]
pub fn admit_script(bundle: &str) -> Result<u32, JsValue> {
    let bundle: el_mod_runtime::script::ScriptBundle = parse(bundle, SCRIPT_JSON_LIMIT)?;
    bundle.validate_program().map_err(err)?;
    Ok(el_mod_runtime::script::MEMORY_RESERVATION as u32)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HookDefinition {
    key: String,
    #[serde(default)]
    dependencies: Vec<usize>,
    grants: Vec<String>,
    #[serde(default)]
    required: bool,
    bundle: el_mod_runtime::script::ScriptBundle,
}

#[wasm_bindgen]
pub struct HookSession {
    inner: el_mod_runtime::hooks::HookDispatcher,
}

#[wasm_bindgen]
impl HookSession {
    #[wasm_bindgen(constructor)]
    pub fn new(definitions: &str) -> Result<HookSession, JsValue> {
        let definitions: Vec<HookDefinition> = parse(definitions, SCRIPT_JSON_LIMIT * 4)?;
        let definitions: Vec<_> = definitions
            .into_iter()
            .map(|definition| el_mod_runtime::session::Definition {
                key: definition.key,
                dependencies: definition.dependencies,
                grants: definition.grants,
                required: definition.required,
                module: el_mod_runtime::session::ModuleCode::Script(definition.bundle),
            })
            .collect();
        Ok(Self {
            inner: el_mod_runtime::hooks::HookDispatcher::prepare(&definitions).map_err(err)?,
        })
    }

    pub fn begin_frame(&mut self, frame_id: u64) -> bool {
        self.inner.begin_frame(frame_id)
    }

    pub fn invoke(&mut self, point: &str, context: &str) -> Result<String, JsValue> {
        let point = parse(&json(&point)?, 128)?;
        let context = parse(context, 65536)?;
        json(&self.inner.invoke(point, context))
    }

    pub fn set_snapshot(&mut self, owner: &str, snapshot: &str) -> Result<(), JsValue> {
        self.inner
            .set_snapshot(owner, parse(snapshot, 65536)?)
            .map_err(err)
    }

    pub fn disable_owner(&mut self, owner: &str) {
        self.inner.disable_owner(owner);
    }

    pub fn reset_world(&mut self) {
        self.inner.reset_world();
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn report(&self) -> Result<String, JsValue> {
        json(&self.inner.report())
    }
}

#[wasm_bindgen]
pub struct WorkerScript {
    inner: el_mod_runtime::script::ScriptActor,
}
#[wasm_bindgen]
pub fn runtime_policy() -> Result<String, JsValue> {
    json(&Policy::default())
}
#[wasm_bindgen]
impl WorkerScript {
    #[wasm_bindgen(constructor)]
    pub fn new(bundle: &str, grants: &str) -> Result<WorkerScript, JsValue> {
        Ok(Self {
            inner: el_mod_runtime::script::ScriptActor::new(
                parse(bundle, SCRIPT_JSON_LIMIT)?,
                parse(grants, 16384)?,
                Policy::default(),
            )
            .map_err(err)?,
        })
    }
    pub fn invoke(&mut self, context: &str, snapshot: &str) -> Result<String, JsValue> {
        self.invoke_resources(context, snapshot, &[])
    }
    pub fn invoke_resources(
        &mut self,
        context: &str,
        snapshot: &str,
        resources: &[u8],
    ) -> Result<String, JsValue> {
        let mut snapshot: Snapshot = parse(snapshot, 262144)?;
        if !resources.is_empty() {
            snapshot.binary =
                Some(el_mod_runtime::binary::Store::from_wire(resources).map_err(err)?);
        }
        let outcome = self
            .inner
            .invoke(parse(context, 2048)?, snapshot)
            .map_err(err)?;
        json(&serde_json::json!({
            "success":outcome.success(), "error":outcome.error,
            "effects":outcome.effects, "fuel_used":outcome.fuel_used,
            "memory_bytes":outcome.memory_bytes,
        }))
    }
}
#[wasm_bindgen]
pub fn layout(rp: i32, rl: i32, wp: i32, wl: i32, len: u32) -> Vec<i32> {
    use el_extension_api::abi::HostCallLayout;
    match HostCallLayout::validate(rp, rl, wp, wl, len as u64) {
        Ok(l) => match (
            l.request().resolve(len as u64),
            l.response().resolve(len as u64),
        ) {
            (Ok(a), Ok(b)) => vec![
                0,
                a.start as i32,
                a.len() as i32,
                b.start as i32,
                b.len() as i32,
            ],
            (Err(error), _) | (_, Err(error)) => vec![error.code()],
        },
        Err(e) => vec![e.code()],
    }
}
#[wasm_bindgen]
pub struct WorkerHost {
    inner: Host,
}
#[wasm_bindgen]
impl WorkerHost {
    #[wasm_bindgen(constructor)]
    pub fn new(grants: &str) -> Result<WorkerHost, JsValue> {
        let grants: Vec<String> = parse(grants, 16384)?;
        if grants.len() > 64
            || grants
                .iter()
                .any(|s| s.parse::<el_extension_api::CapabilityName>().is_err())
        {
            return Err(err("Invalid grants"));
        }
        Ok(Self {
            inner: Host::new(grants),
        })
    }
    pub fn begin(&mut self, context: &str, snapshot: &str) -> Result<(), JsValue> {
        let context: Context = parse(context, 2048)?;
        let snapshot: Snapshot = parse(snapshot, 262144)?;
        self.inner.begin(context, snapshot).map_err(err)
    }
    pub fn dispatch(&mut self, op: i32, request: &[u8], capacity: u32) -> Vec<u8> {
        if capacity > 65536 || request.len() > 65536 {
            return (-8i32).to_le_bytes().to_vec();
        }
        let mut response = vec![0; capacity as usize];
        let code = self.inner.dispatch(op, request, &mut response);
        let mut result = code.to_le_bytes().to_vec();
        if code >= 0 {
            result.extend_from_slice(&response[..code as usize]);
        }
        result
    }
    pub fn finish(&mut self, success: bool) -> Result<String, JsValue> {
        json(&self.inner.finish(success).map_err(err)?)
    }
}
#[wasm_bindgen]
pub struct Coordinator {
    inner: Controller,
    binary_pending: Option<el_mod_runtime::binary::Store>,
}
#[wasm_bindgen]
impl Coordinator {
    #[wasm_bindgen(constructor)]
    pub fn new(configs: &str, side: u32) -> Result<Coordinator, JsValue> {
        let configs: Vec<ActorConfig> = parse(configs, 65536)?;
        Ok(Self {
            inner: Controller::new(configs, side, Policy::default()).map_err(err)?,
            binary_pending: None,
        })
    }
    pub fn begin(&mut self, phase: &str) -> Result<(), JsValue> {
        self.binary_pending = None;
        self.inner
            .begin(parse::<Phase>(&json(&phase)?, 32)?)
            .map_err(err)
    }
    pub fn initialize_config(&mut self, records: &str) -> Result<(), JsValue> {
        self.inner
            .initialize_config(parse(
                records,
                16 * (el_mod_runtime::config::MAX_BYTES + 256),
            )?)
            .map_err(err)
    }
    pub fn config_writes(&self) -> Result<String, JsValue> {
        json(&self.inner.config_writes())
    }
    pub fn acknowledge_config(
        &mut self,
        write: &str,
        receipt: &str,
        error: Option<String>,
    ) -> Result<(), JsValue> {
        let result = match error {
            Some(error) => Err(error),
            None => Ok(parse(receipt, el_mod_runtime::config::MAX_BYTES + 256)?),
        };
        self.inner
            .acknowledge_config(
                &parse(write, el_mod_runtime::config::MAX_BYTES + 512)?,
                result,
            )
            .map_err(err)
    }
    pub fn set_content_input(&mut self, input: &str) -> Result<(), JsValue> {
        self.inner
            .set_content_input(parse::<
                BTreeMap<String, el_mod_content::scene_types::InputFrame>,
            >(input, 262144)?)
            .map_err(err)
    }
    pub fn set_app_input(&mut self, input: &str) -> Result<(), JsValue> {
        self.inner.set_app_input(parse(input, 262144)?).map_err(err)
    }
    pub fn set_keybinding_input(&mut self, input: &str) -> Result<(), JsValue> {
        self.inner
            .set_keybinding_input(parse(input, 262144)?)
            .map_err(err)
    }
    pub fn next(&mut self) -> Result<String, JsValue> {
        let invocation = self.inner.next().map_err(err)?;
        self.binary_pending = invocation.as_ref().and_then(|i| i.snapshot.binary.clone());
        json(&invocation)
    }
    pub fn binary_resources(&self) -> Vec<u8> {
        self.binary_pending
            .as_ref()
            .map(|s| s.to_wire())
            .unwrap_or_default()
    }
    pub fn complete(&mut self, reply: &str) -> Result<String, JsValue> {
        let reply: Completion = parse(reply, 1048576)?;
        let result = self.inner.complete(reply).map_err(err)?;
        self.binary_pending = None;
        json(&result)
    }
    pub fn cancel_tick(&mut self) -> Result<String, JsValue> {
        let result = self.inner.cancel_tick().map_err(err)?;
        self.binary_pending = None;
        json(&result)
    }
    pub fn take_jobs(&mut self) -> Result<String, JsValue> {
        json(&self.inner.take_jobs())
    }
    pub fn complete_job(&mut self, job: &str, result: &[u8]) -> Result<(), JsValue> {
        let job = parse(job, 131072)?;
        self.inner.complete_job(&job, result.to_vec()).map_err(err)
    }
    pub fn report(&mut self) -> Result<String, JsValue> {
        let network = if self.inner.side() == 1 {
            self.inner.drain_client_network().map_err(err)?
        } else {
            Vec::new()
        };
        let s = self.inner.scheduler();
        json(
            &serde_json::json!({"epoch":s.epoch(),"tick":s.tick(),"in_world":s.in_world(),"actors":s.actors(),"diagnostics":self.inner.diagnostics(),"resources":self.inner.broker().resource_counts(),"binary_resources":self.inner.broker().binary_resource_counts(),"fatal":self.inner.fatal(),"jobs":self.inner.job_trace(),"content":self.inner.broker().content_report(s.epoch()==0),"network":network,"config":self.inner.config_statuses(),"app":self.inner.broker().app_report(),"keybindings":self.inner.broker().keybindings_report(),"overlay":self.inner.broker().overlay_report(),"hook_state":self.inner.broker().hook_state_report()}),
        )
    }
}
