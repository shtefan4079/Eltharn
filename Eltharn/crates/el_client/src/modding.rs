use el_mod_runtime::lifecycle::Phase;
pub(crate) mod app_ui;
pub(crate) mod audio;
pub(crate) mod avatar;
pub(crate) mod camera;
pub(crate) mod client_content;
pub(crate) mod hook_state;
pub(crate) mod hooks;
pub(crate) mod input;
pub(crate) mod keybindings;
pub(crate) mod material;
pub(crate) mod overlay;
pub(crate) mod player;
pub(crate) mod remote;
pub(crate) mod resources;
pub(crate) mod surface;
pub(crate) mod world;
pub(crate) mod world_region;
use el_mod_runtime::{driver::Driver, session::Definition};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::BTreeMap;
thread_local! {
    static RUNTIME : RefCell < Option < Bridge >>= const { RefCell::new(None) }; static
    REPORT : RefCell < Value >= const { RefCell::new(Value::Null) }; static ERROR :
    RefCell < Option < String >>= const { RefCell::new(None) }; #[cfg(not(target_arch =
    "wasm32"))] static CONFIG_ROOT : RefCell < Option < std::path::PathBuf >>= const {
    RefCell::new(None) };
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn set_config_root(root: std::path::PathBuf) {
    CONFIG_ROOT.with(|cell| *cell.borrow_mut() = Some(root));
}
struct Bridge {
    driver: Driver,
    last_app: Option<web_time::Instant>,
    observed_world: Option<u64>,
    closing: bool,
    #[cfg(not(target_arch = "wasm32"))]
    executor: el_mod_runtime::native_session::Executor,
    #[cfg(target_arch = "wasm32")]
    completed: std::rc::Rc<RefCell<Option<(Phase, Result<Value, String>)>>>,
}
#[cfg(target_arch = "wasm32")]
mod browser {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen(module = "/src/modding/web_bridge.js")]
    extern "C" {
        #[wasm_bindgen(catch, js_name = startMods)]
        pub async fn start(config: &str, bytes: js_sys::Array) -> Result<JsValue, JsValue>;
        #[wasm_bindgen(catch, js_name = phaseMods)]
        pub async fn phase(
            phase: &str,
            count: u32,
            input: &str,
            keybindings: &str,
        ) -> Result<JsValue, JsValue>;
        #[wasm_bindgen(catch, js_name = cancelModTick)]
        pub fn cancel() -> Result<(), JsValue>;
        #[wasm_bindgen(js_name = disposeMods)]
        pub fn dispose();
    }
    pub fn decode(value: JsValue) -> Result<serde_json::Value, String> {
        serde_json::from_str(&value.as_string().ok_or("Invalid browser runtime reply")?)
            .map_err(|e| e.to_string())
    }
}
pub fn report() -> Value {
    let mut report = REPORT.with(|r| r.borrow().clone());
    if let Some(map) = report.as_object_mut() {
        map.insert("client_content".into(), client_content::observation());
        map.insert("remote".into(), remote::report());
        map.insert("hooks".into(), hooks::report());
    }
    report
}
pub fn error() -> Option<String> {
    ERROR.with(|r| r.borrow().clone())
}
pub fn take_error() -> Option<String> {
    ERROR.with(|r| r.borrow_mut().take())
}
pub(crate) fn catalog_actor_states() -> (Vec<Option<String>>, bool) {
    let states = REPORT.with(|cell| {
        cell.borrow()["states"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|state| state.as_str().map(str::to_owned))
            .collect()
    });
    (states, RUNTIME.with(|cell| cell.borrow().is_some()))
}
pub async fn configure(definitions: Vec<Definition>) -> Result<Value, String> {
    if RUNTIME.with(|r| r.borrow().is_some()) {
        return Err("Mods already configured".into());
    }
    if definitions.is_empty() {
        hook_state::clear();
        hooks::clear();
        keybindings::clear();
        overlay::clear();
        let report = json!(
            { "actors" : [], "live_instances" : 0, "workers" : 0, "resources" : [0, 0, 0,
            0, 0], "keybindings" : { "bindings" : {} } }
        );
        client_content::configure(&report, Vec::new())?;
        app_ui::configure(&report, Vec::new())?;
        REPORT.with(|cell| *cell.borrow_mut() = report.clone());
        return Ok(report);
    }
    let prepared_hooks = hooks::prepare(&definitions)?;
    hook_state::configure(&definitions);
    overlay::configure(&definitions);
    let app_owners = definitions
        .iter()
        .filter(|d| {
            d.grants
                .iter()
                .any(|g| matches!(g.as_str(), "client.app.read" | "client.app.write"))
        })
        .map(|d| d.key.clone())
        .collect();
    let entity_readers = definitions
        .iter()
        .filter(|d| d.grants.iter().any(|g| g == "client.entities.read"))
        .map(|d| d.key.clone())
        .collect();
    let readers = definitions
        .iter()
        .filter(|d| {
            d.grants.iter().any(|g| {
                matches!(
                    g.as_str(),
                    "content.client.read"
                        | "world.blocks.read"
                        | "client.player.read"
                        | "client.entities.read"
                        | "client.avatar.read"
                        | "client.avatar.write"
                )
            })
        })
        .map(|d| d.key.clone())
        .collect();
    #[cfg(not(target_arch = "wasm32"))]
    let (bridge, report) = {
        let needs_config = definitions.iter().any(|definition| {
            definition
                .grants
                .iter()
                .any(|grant| matches!(grant.as_str(), "config.read" | "config.write"))
        });
        let store: Option<Box<dyn el_mod_runtime::config::Store>> = if needs_config {
            let root = CONFIG_ROOT
                .with(|cell| cell.borrow().clone())
                .ok_or("The engine configuration directory is unavailable")?;
            Some(Box::new(
                el_extension_manager::native_config::NativeConfigStore::new(root),
            ))
        } else {
            None
        };
        let mut executor =
            el_mod_runtime::native_session::Executor::spawn_with_config(definitions, 1, store)?
                .ok_or("Missing executor")?;
        let until = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let mut backoff = std::time::Duration::from_millis(1);
        let report = loop {
            if let Some(r) = executor.poll()? {
                break serde_json::to_value(r.result?).map_err(|e| e.to_string())?;
            }
            if std::time::Instant::now() >= until {
                log::error!(
                    "Mod preparation deadline (15 s) reached; reporting preparation failure"
                );
                return Err("Mod preparation deadline (15 s)".into());
            }
            std::thread::sleep(backoff);
            backoff = (backoff * 2).min(std::time::Duration::from_millis(10));
        };
        (
            Bridge {
                driver: Driver::default(),
                last_app: None,
                observed_world: None,
                closing: false,
                executor,
            },
            report,
        )
    };
    #[cfg(target_arch = "wasm32")]
    let (bridge, report) = {
        let configs: Vec<_> = definitions
            .iter()
            .map(|d| {
                json!(
                    { "key" : d.key, "dependencies" : d.dependencies, "grants" : d
                    .grants, "required" : d.required, "language" : match & d.module {
                    el_mod_runtime::session::ModuleCode::Wasm(_) => "wasm",
                    el_mod_runtime::session::ModuleCode::Script(_) => "rhai" } }
                )
            })
            .collect();
        let buffers = js_sys::Array::new();
        for d in definitions {
            match d.module {
                el_mod_runtime::session::ModuleCode::Wasm(bytes) => {
                    buffers.push(&js_sys::Uint8Array::from(bytes.as_slice()));
                }
                el_mod_runtime::session::ModuleCode::Script(bundle) => {
                    buffers.push(&wasm_bindgen::JsValue::from_str(
                        &serde_json::to_string(&bundle).map_err(|e| e.to_string())?,
                    ));
                }
            }
        }
        let raw = browser::start(
            &serde_json::to_string(&configs).map_err(|e| e.to_string())?,
            buffers,
        )
        .await
        .map_err(|e| format!("{e:?}"))?;
        let report = browser::decode(raw)?;
        (
            Bridge {
                driver: Driver::default(),
                last_app: None,
                observed_world: None,
                closing: false,
                completed: std::rc::Rc::new(RefCell::new(None)),
            },
            report,
        )
    };
    if let Err(e) = (|| {
        client_content::configure(&report, readers)?;
        client_content::set_entity_readers(entity_readers);
        app_ui::configure(&report, app_owners)?;
        keybindings::accept(&report)?;
        overlay::accept(&report)?;
        hooks::install(prepared_hooks);
        hooks::reconcile(&report)?;
        hook_state::accept(&report)
    })() {
        fail(e.clone());
        return Err(e);
    }
    RUNTIME.with(|r| *r.borrow_mut() = Some(bridge));
    REPORT.with(|r| *r.borrow_mut() = report.clone());
    Ok(report)
}
impl Bridge {
    fn cancel(&mut self) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        self.executor.cancel_tick();
        #[cfg(target_arch = "wasm32")]
        {
            if self.completed.borrow().is_none() {
                browser::cancel().map_err(|e| format!("{e:?}"))?;
            }
        }
        Ok(())
    }
    fn pump(
        &mut self,
        mut controls: Option<(
            &mut crate::input::VanillaKeyMappings,
            &std::collections::HashMap<String, String>,
        )>,
    ) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        let completed = self.executor.poll()?.map(|r| {
            (
                r.phase,
                r.result
                    .and_then(|v| serde_json::to_value(v).map_err(|e| e.to_string())),
            )
        });
        #[cfg(target_arch = "wasm32")]
        let completed = self.completed.borrow_mut().take();
        if let Some((phase, result)) = completed {
            let report = result?;
            self.driver.complete(phase)?;
            if !self.closing {
                keybindings::accept(&report)?;
                overlay::accept(&report)?;
            }
            if !self.closing {
                hooks::reconcile(&report)?;
            }
            if phase != Phase::AppTick || self.driver.app_is_current() {
                if !self.closing {
                    hook_state::accept(&report)?;
                }
                app_ui::accept(&report)?;
                self.driver.set_settings_app(app_ui::is_settings_open());
            }
            if phase != Phase::WorldEnter || self.driver.world_is_current() {
                client_content::accept(&report, phase)?;
                if phase == Phase::Tick && self.driver.world_is_current() {
                    remote::queue_guest_outputs(&report)?;
                }
            }
            log::debug!(
                "[ModReport] actors={} live_instances={} workers={} failed_diagnostics={} in_world={}",
                report["actors"].as_array().map_or(0, |actors| actors.len()),
                report["live_instances"].as_u64().unwrap_or(0),
                report["workers"].as_u64().unwrap_or(0),
                report["diagnostics"].as_array().map_or(0, |entries| entries
                    .iter()
                    .filter(|entry| entry["success"] == false)
                    .count()),
                report["in_world"].as_bool().unwrap_or(false),
            );
            REPORT.with(|r| *r.borrow_mut() = report);
        }
        if let Some((mappings, saved)) = controls.as_mut() {
            keybindings::sync(mappings, saved);
        }
        if let Some((phase, count)) = self.driver.next_batch() {
            if phase == Phase::AppTick {
                self.last_app = Some(web_time::Instant::now());
            }
            let input = if phase == Phase::Tick {
                client_content::take_input()?
            } else {
                BTreeMap::new()
            };
            let bindings = if matches!(phase, Phase::Tick | Phase::AppTick) {
                controls
                    .as_mut()
                    .map(|(mappings, _)| mappings.drain_mod_input())
                    .unwrap_or_default()
            } else {
                BTreeMap::new()
            };
            let app = if phase == Phase::AppTick {
                app_ui::take_input()?
            } else {
                BTreeMap::new()
            };
            #[cfg(not(target_arch = "wasm32"))]
            self.executor
                .request_inputs(phase, count, input, app, bindings)?;
            #[cfg(target_arch = "wasm32")]
            {
                let completed = self.completed.clone();
                let input = if phase == Phase::AppTick {
                    serde_json::to_string(&app)
                } else {
                    serde_json::to_string(&input)
                }
                .map_err(|e| e.to_string())?;
                let bindings = serde_json::to_string(&bindings).map_err(|e| e.to_string())?;
                wasm_bindgen_futures::spawn_local(async move {
                    let result = browser::phase(&format!("{phase:?}"), count, &input, &bindings)
                        .await
                        .map_err(|e| format!("{e:?}"))
                        .and_then(browser::decode);
                    *completed.borrow_mut() = Some((phase, result));
                });
            }
        }
        Ok(())
    }
}
pub fn observe_update(client_ticks: u32, world: Option<u64>) {
    observe_update_inner(client_ticks, world, None);
}
pub fn observe_update_with_controls(
    client_ticks: u32,
    world: Option<u64>,
    mappings: &mut crate::input::VanillaKeyMappings,
    saved: &std::collections::HashMap<String, String>,
) {
    observe_update_inner(client_ticks, world, Some((mappings, saved)));
}
fn observe_update_inner(
    client_ticks: u32,
    world: Option<u64>,
    mut controls: Option<(
        &mut crate::input::VanillaKeyMappings,
        &std::collections::HashMap<String, String>,
    )>,
) {
    client_content::observe_world(world);
    let result = RUNTIME.with(|r| {
        let mut state = r.borrow_mut();
        let Some(bridge) = state.as_mut() else {
            return Ok(());
        };
        let cancel = bridge.driver.observe(0, world)?;
        if bridge.observed_world != world {
            bridge.observed_world = world;
            hooks::reset_world();
        }
        if cancel {
            bridge.cancel()?;
        }
        bridge.driver.set_settings_app(app_ui::is_settings_open());
        bridge.pump(
            controls
                .as_mut()
                .map(|(mappings, saved)| (&mut **mappings, *saved)),
        )?;
        bridge.driver.observe(client_ticks, world)?;
        let now = web_time::Instant::now();
        if (world.is_none() || app_ui::is_settings_open())
            && bridge
                .last_app
                .is_none_or(|last| now.duration_since(last) >= std::time::Duration::from_millis(50))
        {
            if world.is_none() {
                bridge.driver.observe_app();
            } else {
                bridge.driver.observe_settings_app(true);
            }
        } else if world.is_some() && !app_ui::is_settings_open() {
            bridge.last_app = Some(now);
        }
        bridge.pump(
            controls
                .as_mut()
                .map(|(mappings, saved)| (&mut **mappings, *saved)),
        )
    });
    if let Err(e) = result {
        fail(e);
    }
    if let Some((mappings, saved)) = controls {
        keybindings::sync(mappings, saved);
    }
}
pub(crate) fn fail(message: String) {
    hook_state::clear();
    hooks::clear();
    keybindings::clear();
    overlay::clear();
    app_ui::clear();
    remote::reset();
    input::clear();
    client_content::clear();
    log::error!("Mod runtime: {message}");
    ERROR.with(|r| *r.borrow_mut() = Some(message.clone()));
    RUNTIME.with(|r| *r.borrow_mut() = None);
    #[cfg(target_arch = "wasm32")]
    {
        browser::dispose();
        crate::extension_launch::browser_failure(&message);
    }
}
pub fn shutdown() {
    hook_state::clear();
    hooks::clear();
    keybindings::clear();
    overlay::clear();
    app_ui::hide();
    remote::reset();
    input::clear();
    client_content::clear();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let bridge = RUNTIME.with(|r| r.borrow_mut().take());
        if let Some(mut bridge) = bridge {
            bridge.closing = true;
            if bridge.driver.close() {
                let _ = bridge.cancel();
            }
            let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while !bridge.driver.is_terminated() && std::time::Instant::now() < until {
                if let Err(e) = bridge.pump(None) {
                    log::error!("Mod shutdown: {e}");
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        app_ui::clear();
    }
    #[cfg(target_arch = "wasm32")]
    {
        let bridge = RUNTIME.with(|r| r.borrow_mut().take());
        if let Some(mut bridge) = bridge {
            bridge.closing = true;
            if bridge.driver.close() {
                let _ = bridge.cancel();
            }
            wasm_bindgen_futures::spawn_local(async move {
                let Some(window) = web_sys::window() else {
                    browser::dispose();
                    return;
                };
                let Some(clock) = window.performance() else {
                    browser::dispose();
                    return;
                };
                let deadline = clock.now() + 15_000.0;
                while !bridge.driver.is_terminated() && clock.now() < deadline {
                    if let Err(e) = bridge.pump(None) {
                        log::error!("Mod shutdown: {e}");
                        break;
                    }
                    let wait = js_sys::Promise::new(&mut |resolve, reject| {
                        if let Err(e) = window
                            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 10)
                        {
                            let _ = reject.call1(&wasm_bindgen::JsValue::NULL, &e);
                        }
                    });
                    if wasm_bindgen_futures::JsFuture::from(wait).await.is_err() {
                        break;
                    }
                }
                if !bridge.driver.is_terminated() {
                    log::error!("Mod shutdown did not complete; disposing owned Workers");
                }
                app_ui::clear();
                browser::dispose();
            });
        }
    }
}
