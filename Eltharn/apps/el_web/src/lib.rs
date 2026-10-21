#![forbid(unsafe_code)]
use el_app::{Application, LifecycleRoute};
use el_platform_api::{SurfaceLifecycleEvent, SurfaceSize};
#[cfg(all(target_arch = "wasm32", feature = "extensions"))]
mod extensions;
#[cfg(all(target_arch = "wasm32", feature = "shaderpacks"))]
mod shader_probe;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WebCanvasMetrics {
    pub width: u32,
    pub height: u32,
    pub device_pixel_ratio: f64,
}
impl WebCanvasMetrics {
    pub const fn new(width: u32, height: u32, device_pixel_ratio: f64) -> Self {
        Self {
            width,
            height,
            device_pixel_ratio,
        }
    }
    pub fn to_surface_size(self) -> Option<SurfaceSize> {
        if self.width == 0
            || self.height == 0
            || !self.device_pixel_ratio.is_finite()
            || self.device_pixel_ratio <= 0.0
        {
            return None;
        }
        Some(SurfaceSize::new(
            self.width,
            self.height,
            self.device_pixel_ratio,
        ))
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WebLifecycleInput {
    Visible,
    Hidden,
    CanvasAvailable(WebCanvasMetrics),
    CanvasResized(WebCanvasMetrics),
    CanvasLost,
    LowMemory,
    Destroyed,
}
#[derive(Debug, Clone, PartialEq)]
pub enum WebLifecycleRoute {
    Routed(LifecycleRoute),
    IgnoredInvalidCanvasMetrics(WebCanvasMetrics),
    Destroyed {
        hidden: LifecycleRoute,
        canvas_lost: LifecycleRoute,
    },
    IgnoredAfterDestroy(WebLifecycleInput),
}
#[derive(Debug)]
pub struct WebLifecycleHost {
    application: Application,
    destroyed: bool,
}
impl Default for WebLifecycleHost {
    fn default() -> Self {
        Self::new()
    }
}
impl WebLifecycleHost {
    pub fn new() -> Self {
        let mut application = Application::new();
        application
            .finish_boot()
            .expect("a new Web lifecycle host always starts at application boot");
        Self {
            application,
            destroyed: false,
        }
    }
    pub fn application(&self) -> &Application {
        &self.application
    }
    pub fn is_running(&self) -> bool {
        !self.destroyed
    }
    pub fn route(&mut self, input: WebLifecycleInput) -> WebLifecycleRoute {
        if self.destroyed {
            return WebLifecycleRoute::IgnoredAfterDestroy(input);
        }
        match input {
            WebLifecycleInput::Visible => self.route_event(SurfaceLifecycleEvent::Resumed),
            WebLifecycleInput::Hidden => self.route_event(SurfaceLifecycleEvent::Suspended),
            WebLifecycleInput::CanvasAvailable(metrics) => {
                self.route_canvas(metrics, SurfaceLifecycleEvent::SurfaceAvailable)
            }
            WebLifecycleInput::CanvasResized(metrics) => {
                self.route_canvas(metrics, SurfaceLifecycleEvent::SurfaceResized)
            }
            WebLifecycleInput::CanvasLost => self.route_event(SurfaceLifecycleEvent::SurfaceLost),
            WebLifecycleInput::LowMemory => self.route_event(SurfaceLifecycleEvent::LowMemory),
            WebLifecycleInput::Destroyed => {
                self.destroyed = true;
                let hidden = self
                    .application
                    .route_lifecycle_event(SurfaceLifecycleEvent::Suspended);
                let canvas_lost = self
                    .application
                    .route_lifecycle_event(SurfaceLifecycleEvent::SurfaceLost);
                WebLifecycleRoute::Destroyed {
                    hidden,
                    canvas_lost,
                }
            }
        }
    }
    fn route_canvas(
        &mut self,
        metrics: WebCanvasMetrics,
        make_event: impl FnOnce(SurfaceSize) -> SurfaceLifecycleEvent,
    ) -> WebLifecycleRoute {
        let Some(size) = metrics.to_surface_size() else {
            return WebLifecycleRoute::IgnoredInvalidCanvasMetrics(metrics);
        };
        self.route_event(make_event(size))
    }
    fn route_event(&mut self, event: SurfaceLifecycleEvent) -> WebLifecycleRoute {
        WebLifecycleRoute::Routed(self.application.route_lifecycle_event(event))
    }
}
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub struct BrowserWssAdapter {
    #[cfg(target_arch = "wasm32")]
    socket: web_sys::WebSocket,
    driver: el_transport::LocalWssDriver,
    #[cfg(target_arch = "wasm32")]
    on_message: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::MessageEvent)>,
    #[cfg(target_arch = "wasm32")]
    on_close: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::Event)>,
    #[cfg(target_arch = "wasm32")]
    on_error: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::Event)>,
    #[cfg(target_arch = "wasm32")]
    pending_outbound: Option<Vec<u8>>,
    #[cfg(target_arch = "wasm32")]
    failed: std::rc::Rc<std::cell::Cell<bool>>,
}
impl BrowserWssAdapter {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn unavailable_for_native() -> el_platform_api::PlatformError {
        el_platform_api::PlatformError::unsupported(
            "browser WSS adapter is only available on the wasm32 target",
        )
    }
    #[cfg(target_arch = "wasm32")]
    pub fn connect(
        url: &str,
        inbound_capacity: usize,
        outbound_capacity: usize,
    ) -> Result<(Self, el_transport::LocalWssByteStream), wasm_bindgen::JsValue> {
        use wasm_bindgen::{JsCast, closure::Closure};
        use web_sys::{BinaryType, Event, MessageEvent, WebSocket};
        if !url.starts_with("wss://") {
            return Err(wasm_bindgen::JsValue::from_str(
                "Eltharn browser transport requires a wss:// gateway URL",
            ));
        }
        let socket = WebSocket::new(url)?;
        socket.set_binary_type(BinaryType::Arraybuffer);
        let (stream, driver) =
            el_transport::LocalWssByteStream::new(inbound_capacity, outbound_capacity)
                .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))?;
        let failed = std::rc::Rc::new(std::cell::Cell::new(false));
        let message_driver = driver.clone();
        let message_failed = std::rc::Rc::clone(&failed);
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let data = event.data();
            if data.is_instance_of::<js_sys::ArrayBuffer>() {
                let buffer = data.unchecked_into::<js_sys::ArrayBuffer>();
                let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
                if message_driver.receive_binary_frame(&bytes).is_err() {
                    message_failed.set(true);
                    message_driver.close_remote();
                }
            } else {
                message_failed.set(true);
                message_driver.close_remote();
            }
        });
        socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        let close_driver = driver.clone();
        let on_close = Closure::<dyn FnMut(Event)>::new(move |_event: Event| {
            close_driver.close_remote();
        });
        socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        let error_driver = driver.clone();
        let error_failed = std::rc::Rc::clone(&failed);
        let on_error = Closure::<dyn FnMut(Event)>::new(move |_event: Event| {
            error_failed.set(true);
            error_driver.close_remote();
        });
        socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        Ok((
            Self {
                socket,
                driver,
                on_message,
                on_close,
                on_error,
                pending_outbound: None,
                failed,
            },
            stream,
        ))
    }
    #[cfg(target_arch = "wasm32")]
    pub fn send_available(&mut self, max_bytes: usize) -> Result<usize, wasm_bindgen::JsValue> {
        use wasm_bindgen::JsValue;
        use web_sys::WebSocket;
        if self.failed.get() {
            return Err(JsValue::from_str(
                "browser WSS adapter is in an error state",
            ));
        }
        if self.socket.ready_state() != WebSocket::OPEN {
            return Ok(0);
        }
        if self.pending_outbound.is_none() {
            self.pending_outbound = self.driver.take_outbound_chunk(max_bytes);
        }
        let Some(chunk) = self.pending_outbound.as_ref() else {
            return Ok(0);
        };
        if let Err(error) = self.socket.send_with_u8_array(chunk.as_slice()) {
            self.failed.set(true);
            self.driver.close_remote();
            return Err(error);
        }
        let sent = chunk.len();
        self.pending_outbound = None;
        Ok(sent)
    }
    pub fn driver(&self) -> &el_transport::LocalWssDriver {
        &self.driver
    }
}
#[cfg(target_arch = "wasm32")]
fn deployed_protocol_catalogue(
    manifests: &[el_launcher_core::ProtocolBackendManifest],
    preferred: &str,
) -> (Vec<String>, String) {
    let mut protocols: Vec<_> = manifests
        .iter()
        .map(|manifest| manifest.minecraft_version.clone())
        .filter(|version| el_launcher_core::ProtocolId::new(version.clone()).is_ok())
        .collect();
    protocols.sort();
    protocols.dedup();
    let selected = protocols
        .iter()
        .find(|version| version.as_str() == preferred)
        .or_else(|| protocols.first())
        .cloned()
        .unwrap_or_default();
    (protocols, selected)
}
#[cfg(target_arch = "wasm32")]
mod browser_entrypoint {
    use super::{
        WebCanvasMetrics, WebLifecycleHost, WebLifecycleInput, deployed_protocol_catalogue,
    };
    use el_launcher_core::{
        EltharnBackendStatus, EltharnLaunchPageInput, EltharnLaunchPageModel, EltharnStatusTone,
        build_eltharn_launch_page,
    };
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use web_sys::{
        Document, Event, HtmlButtonElement, HtmlCanvasElement, HtmlElement, HtmlInputElement,
        HtmlSelectElement, Storage, Window,
    };
    const ACCOUNT_STORAGE_KEY: &str = "eltharn_gate.account.name";
    const PROTOCOL_STORAGE_KEY: &str = "eltharn_gate.protocol";
    const GATEWAY_STORAGE_KEY: &str = "eltharn_gate.gateway.url";
    const LEGACY_ACCOUNT_STORAGE_KEY: &str = "ronner.account.name";
    const LEGACY_PROTOCOL_STORAGE_KEY: &str = "ronner.protocol";
    const LEGACY_GATEWAY_STORAGE_KEY: &str = "ronner.gateway.url";
    fn load_preference(storage: &Storage, key: &str, legacy_key: &str) -> Option<String> {
        storage
            .get_item(key)
            .ok()
            .flatten()
            .or_else(|| storage.get_item(legacy_key).ok().flatten())
    }
    #[derive(Clone, Debug)]
    struct BrowserDeploymentConfig {
        default_protocol: String,
        protocols: Vec<String>,
        backend_manifests: Vec<el_launcher_core::ProtocolBackendManifest>,
    }
    impl BrowserDeploymentConfig {
        fn from_window(window: &Window) -> Self {
            let value =
                js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("ELTHARN_GATE_CONFIG"))
                    .unwrap_or(JsValue::UNDEFINED);
            let string = |key: &str| {
                js_sys::Reflect::get(&value, &JsValue::from_str(key))
                    .ok()
                    .and_then(|value| value.as_string())
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            };
            let backend_manifests: Vec<el_launcher_core::ProtocolBackendManifest> =
                js_sys::Reflect::get(&value, &JsValue::from_str("backend_manifests"))
                    .ok()
                    .and_then(|value| js_sys::JSON::stringify(&value).ok())
                    .and_then(|json| json.as_string())
                    .and_then(|json| {
                        serde_json::from_str::<Vec<el_launcher_core::ProtocolBackendManifest>>(
                            &json,
                        )
                        .ok()
                    })
                    .unwrap_or_default();
            let backend_manifests: Vec<_> = backend_manifests
                .into_iter()
                .filter(|manifest| {
                    el_launcher_core::ProtocolId::new(manifest.minecraft_version.clone()).is_ok()
                        && manifest.ipc_version
                            == el_protocol_backend::ELTHARN_PROTOCOL_BACKEND_IPC_VERSION
                        && manifest.protocol_version > 0
                        && manifest.assets_compatibility.as_deref()
                            == Some(el_launcher_core::ELTHARN_CLIENT_ASSETS_COMPATIBILITY)
                })
                .collect();
            let (protocols, default_protocol) =
                deployed_protocol_catalogue(&backend_manifests, &string("default_protocol"));
            Self {
                default_protocol,
                protocols,
                backend_manifests,
            }
        }
    }
    struct BrowserLauncherState {
        account_name: Option<String>,
        selected_protocol: String,
        protocol_options: Vec<String>,
        backend_manifests: Vec<el_launcher_core::ProtocolBackendManifest>,
    }
    impl BrowserLauncherState {
        fn load(window: &Window, deployment: &BrowserDeploymentConfig) -> Self {
            let storage = window.local_storage().ok().flatten();
            let account_name = storage
                .as_ref()
                .and_then(|storage| {
                    load_preference(storage, ACCOUNT_STORAGE_KEY, LEGACY_ACCOUNT_STORAGE_KEY)
                })
                .filter(|name| !name.trim().is_empty());
            let selected_protocol = storage
                .as_ref()
                .and_then(|storage| {
                    load_preference(storage, PROTOCOL_STORAGE_KEY, LEGACY_PROTOCOL_STORAGE_KEY)
                })
                .filter(|protocol| deployment.protocols.iter().any(|item| item == protocol))
                .unwrap_or_else(|| deployment.default_protocol.clone());
            Self {
                account_name,
                selected_protocol,
                protocol_options: deployment.protocols.clone(),
                backend_manifests: deployment.backend_manifests.clone(),
            }
        }
        fn save(&self, storage: Option<Storage>) {
            let Some(storage) = storage else {
                return;
            };
            if let Some(account_name) = self.account_name.as_deref() {
                if storage.set_item(ACCOUNT_STORAGE_KEY, account_name).is_ok() {
                    let _ = storage.remove_item(LEGACY_ACCOUNT_STORAGE_KEY);
                }
            } else {
                let _ = storage.remove_item(ACCOUNT_STORAGE_KEY);
                let _ = storage.remove_item(LEGACY_ACCOUNT_STORAGE_KEY);
            }
            if storage
                .set_item(PROTOCOL_STORAGE_KEY, &self.selected_protocol)
                .is_ok()
            {
                let _ = storage.remove_item(LEGACY_PROTOCOL_STORAGE_KEY);
            }
            let _ = storage.remove_item(GATEWAY_STORAGE_KEY);
            let _ = storage.remove_item(LEGACY_GATEWAY_STORAGE_KEY);
        }
        fn page_model(&self) -> EltharnLaunchPageModel {
            build_eltharn_launch_page(EltharnLaunchPageInput {
                protocol_options: self.protocol_options.clone(),
                selected_protocol: self.selected_protocol.clone(),
                active_account_name: self.account_name.clone(),
                backend_status: match self
                    .backend_manifests
                    .iter()
                    .find(|manifest| {
                        manifest.minecraft_version == self.selected_protocol
                    })
                {
                    None => EltharnBackendStatus::Missing,
                    Some(manifest) => {
                        match el_launcher_core::validate_protocol_backend_compatibility(
                            manifest,
                            &self.selected_protocol,
                            el_launcher_core::ELTHARN_CLIENT_ASSETS_COMPATIBILITY,
                            el_protocol_backend::ELTHARN_PROTOCOL_BACKEND_IPC_VERSION,
                        ) {
                            Ok(
                                (),
                            ) if manifest.protocol_version > 0
                                && manifest.assets_compatibility.as_deref()
                                    == Some(
                                        el_launcher_core::ELTHARN_CLIENT_ASSETS_COMPATIBILITY,
                                    ) => {
                                EltharnBackendStatus::Ready {
                                    backend_name: manifest.backend_name.clone(),
                                    protocol_version: manifest.protocol_version,
                                    protocol_source: manifest.protocol_source.clone(),
                                }
                            }
                            Ok(()) => {
                                EltharnBackendStatus::Broken {
                                    reason: "Gateway backend manifest requires an explicit wire protocol and asset version."
                                        .into(),
                                }
                            }
                            Err(error) => {
                                EltharnBackendStatus::Broken {
                                    reason: error.to_string(),
                                }
                            }
                        }
                    }
                },
            })
        }
    }
    #[allow(dead_code)]
    struct BrowserRuntime {
        host: WebLifecycleHost,
        canvas: HtmlCanvasElement,
        launcher_state: Rc<RefCell<BrowserLauncherState>>,
        on_resize: Closure<dyn FnMut(Event)>,
        on_visibility: Closure<dyn FnMut(Event)>,
        on_protocol_change: Closure<dyn FnMut(Event)>,
        on_account_save: Closure<dyn FnMut(Event)>,
        on_launch: Closure<dyn FnMut(Event)>,
    }
    thread_local! {
        static RUNTIME : RefCell < Option < BrowserRuntime >> = const {
        RefCell::new(None) };
    }
    #[wasm_bindgen::prelude::wasm_bindgen(start)]
    pub fn start() -> Result<(), JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("window is unavailable"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
        let canvas = find_or_create_canvas(&document)?;
        canvas.set_tab_index(0);
        canvas.set_hidden(true);
        let root = find_or_create_launcher(&document)?;
        let deployment = BrowserDeploymentConfig::from_window(&window);
        let launcher_state = Rc::new(RefCell::new(BrowserLauncherState::load(
            &window,
            &deployment,
        )));
        render_launcher(&root, &launcher_state.borrow().page_model())?;
        let protocol_select = query::<HtmlSelectElement>(&root, "#eltharn_gate-protocol")?;
        let account_input = query::<HtmlInputElement>(&root, "#eltharn_gate-account-input")?;
        let account_save = query::<HtmlButtonElement>(&root, "#eltharn_gate-account-save")?;
        let launch_button = query::<HtmlButtonElement>(&root, "#eltharn_gate-launch")?;
        let protocol_root = root.clone();
        let protocol_state = Rc::clone(&launcher_state);
        let protocol_window = window.clone();
        let on_protocol_change = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
            let Some(select) = event
                .target()
                .and_then(|target| target.dyn_into::<HtmlSelectElement>().ok())
            else {
                return;
            };
            let mut state = protocol_state.borrow_mut();
            state.selected_protocol = select.value();
            state.save(protocol_window.local_storage().ok().flatten());
            let model = state.page_model();
            drop(state);
            let _ = render_launcher(&protocol_root, &model);
        });
        protocol_select.add_event_listener_with_callback(
            "change",
            on_protocol_change.as_ref().unchecked_ref(),
        )?;
        let account_root = root.clone();
        let account_state = Rc::clone(&launcher_state);
        let account_window = window.clone();
        let account_input_for_save = account_input.clone();
        let on_account_save = Closure::<dyn FnMut(Event)>::new(move |_event: Event| {
            let name = account_input_for_save.value().trim().to_string();
            if !el_launcher_core::valid_name(&name) {
                let _ = set_text(
                    &account_root,
                    "#eltharn_gate-form-error",
                    "Enter a username (1–16 characters).",
                );
                return;
            }
            let mut state = account_state.borrow_mut();
            state.account_name = Some(name);
            state.save(account_window.local_storage().ok().flatten());
            account_input_for_save.set_value("");
            let model = state.page_model();
            drop(state);
            let _ = set_text(
                &account_root,
                "#eltharn_gate-form-error",
                "Account saved on this device.",
            );
            let _ = render_launcher(&account_root, &model);
        });
        account_save
            .add_event_listener_with_callback("click", on_account_save.as_ref().unchecked_ref())?;
        let launch_root = root.clone();
        let launch_state = Rc::clone(&launcher_state);
        let launch_canvas = canvas.clone();
        let on_launch = Closure::<dyn FnMut(Event)>::new(move |_event: Event| {
            let state = launch_state.borrow();
            let Some(username) = state.account_name.clone() else {
                let _ = set_text(
                    &launch_root,
                    "#eltharn_gate-form-error",
                    "Please save your account first.",
                );
                return;
            };
            let protocol = state.selected_protocol.clone();
            if !state.page_model().hero.launch_enabled {
                let _ = set_text(
                    &launch_root,
                    "#eltharn_gate-form-error",
                    "The selected IPC backend is unavailable.",
                );
                return;
            }
            drop(state);
            if let Some(win) = web_sys::window() {
                win.scroll_to_with_x_and_y(0.0, 0.0);
                if let Some(doc) = win.document() {
                    if let Some(splash) = doc.get_element_by_id("eltharn_gate-splash") {
                        let _ = splash.remove_attribute("hidden");
                        if let Ok(html_splash) = splash.dyn_into::<web_sys::HtmlElement>() {
                            let _ = html_splash.style().set_property("opacity", "1");
                        }
                    }
                }
            }
            launch_root.set_hidden(true);
            launch_canvas.set_hidden(false);
            let _ = launch_canvas.focus();
            el_client::web::run_web_client("eltharn_gate-canvas", &username, &protocol);
        });
        launch_button
            .add_event_listener_with_callback("click", on_launch.as_ref().unchecked_ref())?;
        let mut host = WebLifecycleHost::new();
        host.route(WebLifecycleInput::Visible);
        sync_canvas(&window, &canvas, &mut host, true);
        let resize_window = window.clone();
        let resize_canvas = canvas.clone();
        let on_resize = Closure::<dyn FnMut(Event)>::new(move |_event: Event| {
            RUNTIME.with(|runtime| {
                let mut runtime_ref = runtime.borrow_mut();
                let Some(runtime) = runtime_ref.as_mut() else {
                    return;
                };
                sync_canvas(&resize_window, &resize_canvas, &mut runtime.host, false);
            });
        });
        window.add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref())?;
        let visibility_window = window.clone();
        let on_visibility = Closure::<dyn FnMut(Event)>::new(move |_event: Event| {
            let hidden = visibility_window
                .document()
                .map(|document| document.hidden())
                .unwrap_or(false);
            RUNTIME.with(|runtime| {
                if let Some(runtime) = runtime.borrow_mut().as_mut() {
                    runtime.host.route(if hidden {
                        WebLifecycleInput::Hidden
                    } else {
                        WebLifecycleInput::Visible
                    });
                }
            });
        });
        document.add_event_listener_with_callback(
            "visibilitychange",
            on_visibility.as_ref().unchecked_ref(),
        )?;
        RUNTIME.with(|runtime| {
            *runtime.borrow_mut() = Some(BrowserRuntime {
                host,
                canvas,
                launcher_state,
                on_resize,
                on_visibility,
                on_protocol_change,
                on_account_save,
                on_launch,
            });
        });
        Ok(())
    }
    fn query<T>(root: &HtmlElement, selector: &str) -> Result<T, JsValue>
    where
        T: JsCast,
    {
        root.query_selector(selector)?
            .ok_or_else(|| JsValue::from_str("launcher control is missing"))?
            .dyn_into::<T>()
            .map_err(|_| JsValue::from_str("launcher control has an unexpected type"))
    }
    fn set_text(root: &HtmlElement, selector: &str, text: &str) -> Result<(), JsValue> {
        let element = root
            .query_selector(selector)?
            .ok_or_else(|| JsValue::from_str("launcher text node is missing"))?;
        element.set_text_content(Some(text));
        Ok(())
    }
    fn find_or_create_launcher(document: &Document) -> Result<HtmlElement, JsValue> {
        if let Some(existing) = document.get_element_by_id("eltharn_gate-launcher") {
            return existing
                .dyn_into::<HtmlElement>()
                .map_err(|_| JsValue::from_str("#eltharn_gate-launcher is not an HTML element"));
        }
        let root = document
            .create_element("main")?
            .dyn_into::<HtmlElement>()
            .map_err(|_| JsValue::from_str("could not create the Eltharn Gate launcher"))?;
        root.set_id("eltharn_gate-launcher");
        root.set_inner_html(
            r#"
            <section class="eltharn_gate-hero">
              <div class="eltharn_gate-kicker">ELTHARN GATE / ELTHARN</div>
              <h1 id="eltharn_gate-title">Eltharn</h1>
              <p id="eltharn_gate-subtitle"></p>
              <div id="eltharn_gate-identity" class="eltharn_gate-identity"></div>
              <div id="eltharn_gate-status" class="eltharn_gate-status" data-tone="error">
                <span id="eltharn_gate-status-title"></span>
                <span id="eltharn_gate-status-detail"></span>
              </div>
            </section>
            <section class="eltharn_gate-card">
              <div class="eltharn_gate-section-label">PROTOCOL</div>
              <label for="eltharn_gate-protocol">Server Protocol</label>
              <select id="eltharn_gate-protocol" class="eltharn_gate-control"></select>
              <p id="eltharn_gate-protocol-help" class="eltharn_gate-help"></p>
            </section>
            <section class="eltharn_gate-card">
              <div class="eltharn_gate-section-label">ACCOUNT</div>
              <label for="eltharn_gate-account-input">Offline account</label>
              <div class="eltharn_gate-row">
              <input id="eltharn_gate-account-input" class="eltharn_gate-control" autocomplete="nickname" maxlength="16" />
                <button id="eltharn_gate-account-save" class="eltharn_gate-secondary" type="button">Save</button>
              </div>
              <p id="eltharn_gate-account-active" class="eltharn_gate-help"></p>
            </section>
            <button id="eltharn_gate-launch" class="eltharn_gate-primary" type="button" disabled>Launch Eltharn</button>
            <p id="eltharn_gate-form-error" class="eltharn_gate-form-error" role="status"></p>
            "#,
        );
        document
            .body()
            .ok_or_else(|| JsValue::from_str("document body is unavailable"))?
            .append_child(&root)?;
        Ok(root)
    }
    fn render_launcher(root: &HtmlElement, model: &EltharnLaunchPageModel) -> Result<(), JsValue> {
        set_text(
            root,
            "#eltharn_gate-protocol-help",
            &format!(
                "Select a compatible protocol. Asset version: {}.",
                el_launcher_core::ELTHARN_CLIENT_ASSETS_COMPATIBILITY,
            ),
        )?;
        set_text(root, "#eltharn_gate-title", &model.title)?;
        set_text(root, "#eltharn_gate-subtitle", &model.subtitle)?;
        set_text(root, "#eltharn_gate-identity", &model.hero.identity_line)?;
        set_text(root, "#eltharn_gate-status-title", &model.hero.title)?;
        set_text(root, "#eltharn_gate-status-detail", &model.hero.detail)?;
        let status = root
            .query_selector("#eltharn_gate-status")?
            .ok_or_else(|| JsValue::from_str("launcher status is missing"))?;
        status.set_attribute(
            "data-tone",
            if model.hero.tone == EltharnStatusTone::Ready {
                "ready"
            } else {
                "error"
            },
        )?;
        let select = query::<HtmlSelectElement>(root, "#eltharn_gate-protocol")?;
        let selected = model.selected_protocol.as_str();
        select.set_inner_html("");
        for protocol in &model.protocol_options {
            let option = root
                .owner_document()
                .ok_or_else(|| JsValue::from_str("launcher document is missing"))?
                .create_element("option")?;
            option.set_text_content(Some(protocol));
            option.set_attribute("value", protocol)?;
            if protocol == selected {
                option.set_attribute("selected", "selected")?;
            }
            select.append_child(&option)?;
        }
        let account_active = model
            .hero
            .identity_line
            .split("Account: ")
            .nth(1)
            .unwrap_or("Not selected");
        set_text(
            root,
            "#eltharn_gate-account-active",
            &format!(
                "Active account: {}",
                if account_active == "Not selected" {
                    "None selected"
                } else {
                    account_active
                }
            ),
        )?;
        let launch = query::<HtmlButtonElement>(root, "#eltharn_gate-launch")?;
        launch.set_disabled(!model.hero.launch_enabled || account_active == "Not selected");
        Ok(())
    }
    fn find_or_create_canvas(document: &Document) -> Result<HtmlCanvasElement, JsValue> {
        if let Some(existing) = document.get_element_by_id("eltharn_gate-canvas") {
            return existing.dyn_into::<HtmlCanvasElement>().map_err(|_| {
                JsValue::from_str("#eltharn_gate-canvas exists but is not a canvas element")
            });
        }
        let canvas = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| JsValue::from_str("could not create the Eltharn Gate canvas"))?;
        canvas.set_id("eltharn_gate-canvas");
        document
            .body()
            .ok_or_else(|| JsValue::from_str("document body is unavailable"))?
            .append_child(&canvas)?;
        Ok(canvas)
    }
    fn sync_canvas(
        window: &Window,
        canvas: &HtmlCanvasElement,
        host: &mut WebLifecycleHost,
        initial: bool,
    ) {
        let css_width = canvas.client_width().max(0) as u32;
        let css_height = canvas.client_height().max(0) as u32;
        let ratio = window.device_pixel_ratio();
        let width = if css_width != 0 {
            css_width
        } else {
            canvas.width()
        };
        let height = if css_height != 0 {
            css_height
        } else {
            canvas.height()
        };
        let width = ((f64::from(width) * ratio).round() as u32).max(1);
        let height = ((f64::from(height) * ratio).round() as u32).max(1);
        canvas.set_width(width);
        canvas.set_height(height);
        let metrics = WebCanvasMetrics::new(width, height, ratio);
        host.route(if initial {
            WebLifecycleInput::CanvasAvailable(metrics)
        } else {
            WebLifecycleInput::CanvasResized(metrics)
        });
    }
}
