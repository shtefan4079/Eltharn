use crate::app::main_entry::ProtocolBackendData;
#[allow(unused_imports)]
pub use crate::network::backend_mappers::{
    map_backend_chunk, map_backend_item_stack, map_backend_nbt_to_nbt, map_item_stack_to_backend,
};
use crossbeam_channel::{Receiver, Sender};
use el_protocol::packets::game::{
    ServerboundContainerClickPacket, ServerboundSetCreativeModeSlotPacket,
};
pub use el_protocol_api::ClientConnectRequest;
use el_protocol_backend::{
    BackendConnectArgs, BackendLogLevel, BackendNetworkEvent, BackendOutboundPacket,
    BackendPlayerActionKind, BackendSeenAdvancementsAction, BackendToClientMessage,
    ClientToBackendMessage, ELTHARN_PROTOCOL_BACKEND_IPC_VERSION, decode_json_line,
    encode_json_line,
};
use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::Instant,
};
const ELTHARN_NOISY_DEBUG_ENV: &str = "ELTHARN_NOISY_DEBUG";
#[allow(dead_code)]
const ELTHARN_CHEST_DEBUG_ENV: &str = "ELTHARN_CHEST_DEBUG";
const ELTHARN_BACKEND_EVENT_TRACE_ENV: &str = "ELTHARN_BACKEND_EVENT_TRACE";
fn env_truthy(name: &str) -> bool {
    std::env::var(name)
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false)
}
fn verbose_console_logs_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        env_truthy(ELTHARN_NOISY_DEBUG_ENV)
            || env_truthy("ELTHARN_VERBOSE_LOGS")
            || matches!(
                std::env::var("ELTHARN_LOGS").as_deref(),
                Ok("all" | "trace" | "debug" | "info")
            )
    })
}
fn noisy_debug_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var(ELTHARN_NOISY_DEBUG_ENV)
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
            .unwrap_or(false)
    })
}
#[allow(dead_code)]
fn chest_debug_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        chest_debug_requested(std::env::var(ELTHARN_CHEST_DEBUG_ENV).ok().as_deref())
    })
}
#[allow(dead_code)]
fn chest_debug_requested(value: Option<&str>) -> bool {
    matches!(value, Some("1"))
}
#[allow(dead_code)]
fn log_external_backend_chunk_chests(chunk: &el_world::chunk::Chunk) {
    if !chest_debug_enabled() {
        return;
    }
    let mut rows = chunk
        .block_entities
        .keys()
        .filter_map(|pos| {
            let state = chunk.get_block_state(pos)?;
            state.block.path.ends_with("chest").then_some((pos, state))
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|(pos, _)| (pos.x, pos.y, pos.z));
    for (pos, state) in rows {
        let line = format!(
            "[ChestDebug][Client] function=map_external_backend_chunk chunk=[{},{}] \\
             pos=[{},{},{}] block={} state_id={} raw_facing={:?} raw_type={:?} has_block_entity=true",
            chunk.x,
            chunk.z,
            pos.x,
            pos.y,
            pos.z,
            state.block,
            state.id,
            state.properties.get("facing").map(String::as_str),
            state.properties.get("type").map(String::as_str),
        );
        eprintln!("{line}");
        if el_protocol_backend::diagnostics::file_enabled() {
            el_protocol_backend::diagnostics::write_line(&line);
        }
    }
}
#[allow(dead_code)]
fn item_component_debug_enabled() -> bool {
    env_truthy("ELTHARN_ITEM_COMPONENT_DEBUG") || noisy_debug_enabled()
}
#[allow(dead_code)]
fn diagnostic_component_preview(text: String) -> String {
    const MAX_CHARS: usize = 768;
    let text = text.replace('\r', "\\r").replace('\n', "\\n");
    let mut preview = text.chars().take(MAX_CHARS).collect::<String>();
    if text.chars().count() > MAX_CHARS {
        preview.push('…');
    }
    preview
}
#[allow(dead_code)]
fn write_item_component_debug(stack: &el_core::ItemStack) {
    if !item_component_debug_enabled() {
        return;
    }
    const PRESENTATION_COMPONENTS: &[&str] = &[
        "minecraft:custom_name",
        "minecraft:item_name",
        "minecraft:lore",
        "minecraft:enchantments",
        "minecraft:stored_enchantments",
        "minecraft:potion_contents",
        "minecraft:tooltip_display",
    ];
    let entries = PRESENTATION_COMPONENTS
        .iter()
        .filter_map(|key| {
            let component = stack.components.get(key)?;
            let payload = match component {
                el_core::item::DataComponent::CustomName(value) => {
                    diagnostic_component_preview(value.clone())
                }
                el_core::item::DataComponent::Lore(lines) => {
                    diagnostic_component_preview(format!("{lines:?}"))
                }
                el_core::item::DataComponent::Generic { value, .. } => {
                    diagnostic_component_preview(format!("{value:?}"))
                }
                other => diagnostic_component_preview(format!("{other:?}")),
            };
            Some(format!("{}={}", key, payload))
        })
        .collect::<Vec<_>>();
    if !entries.is_empty() {
        println!(
            "[ItemComponentDebug] phase=backend_to_client item={} count={} components=[{}]",
            stack.item.location,
            stack.count,
            entries.join("; ")
        );
    }
}
fn should_print_console_line(line: &str) -> bool {
    if verbose_console_logs_enabled() || noisy_debug_enabled() {
        return true;
    }
    let diagnostic_line = line
        .strip_prefix("[EltharnProtocolBackend stderr] ")
        .unwrap_or(line);
    if el_protocol_backend::diagnostics::resource_debug_enabled()
        && diagnostic_line.starts_with("[ResourcePackDebug]")
    {
        return true;
    }
    if el_protocol_backend::diagnostics::console_enabled()
        && (line.starts_with("[MovementDebug]")
            || line.starts_with("[MoveDebug]")
            || line.starts_with("[SlideDebug]")
            || line.starts_with("[NetDebug]"))
    {
        return true;
    }
    if !error_console_logs_enabled() {
        return false;
    }
    let lower = diagnostic_line.to_ascii_lowercase();
    diagnostic_line.starts_with("[ERROR]")
        || lower.contains("error")
        || lower.contains("failed")
        || lower.contains("fatal")
        || lower.contains("disconnect")
        || lower.contains("kicked")
        || lower.contains("kick reason")
}
fn error_console_logs_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        matches!(
            std::env::var("ELTHARN_LOGS").as_deref(),
            Ok("error" | "warn")
        )
    })
}
fn backend_event_trace_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        backend_event_trace_requested(
            std::env::var(ELTHARN_BACKEND_EVENT_TRACE_ENV)
                .ok()
                .as_deref(),
        ) || verbose_console_logs_enabled()
    })
}
fn backend_event_trace_requested(value: Option<&str>) -> bool {
    matches!(
        value.map(str::trim),
        Some("1" | "true" | "TRUE" | "yes" | "on")
    )
}
macro_rules! println {
    ($($arg:tt)*) => {
        { if verbose_console_logs_enabled() { let line = format!($($arg)*);
        el_protocol_backend::diagnostics::write_diagnostic_line(& line); if
        should_print_console_line(& line) { std::println!("{}", line); } } }
    };
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IpcTelemetrySnapshot {
    pub route: &'static str,
    pub available: bool,
    pub stdout_read_calls: u64,
    pub stdout_read_eofs: u64,
    pub stdout_read_errors: u64,
    pub stdout_bytes: u64,
    pub stdout_read_wait_ns: u64,
    pub stdout_read_wait_max_ns: u64,
    pub inbound_decode_calls: u64,
    pub inbound_decode_errors: u64,
    pub inbound_decode_ns: u64,
    pub inbound_decode_max_ns: u64,
    pub inbound_map_enqueue_calls: u64,
    pub inbound_map_enqueue_errors: u64,
    pub inbound_map_enqueue_ns: u64,
    pub inbound_map_enqueue_max_ns: u64,
    pub stdin_encode_calls: u64,
    pub stdin_encode_errors: u64,
    pub stdin_encoded_bytes: u64,
    pub stdin_encode_ns: u64,
    pub stdin_encode_max_ns: u64,
    pub stdin_write_flush_calls: u64,
    pub stdin_write_flush_errors: u64,
    pub stdin_write_attempted_bytes: u64,
    pub stdin_write_flush_ns: u64,
    pub stdin_write_flush_max_ns: u64,
}
impl IpcTelemetrySnapshot {
    fn unavailable(route: &'static str) -> Self {
        Self {
            route,
            available: false,
            ..Self::default()
        }
    }
}
#[derive(Default)]
struct ExternalIpcTelemetry {
    stdout_read_calls: AtomicU64,
    stdout_read_eofs: AtomicU64,
    stdout_read_errors: AtomicU64,
    stdout_bytes: AtomicU64,
    stdout_read_wait_ns: AtomicU64,
    stdout_read_wait_max_ns: AtomicU64,
    inbound_decode_calls: AtomicU64,
    inbound_decode_errors: AtomicU64,
    inbound_decode_ns: AtomicU64,
    inbound_decode_max_ns: AtomicU64,
    inbound_map_enqueue_calls: AtomicU64,
    inbound_map_enqueue_errors: AtomicU64,
    inbound_map_enqueue_ns: AtomicU64,
    inbound_map_enqueue_max_ns: AtomicU64,
    stdin_encode_calls: AtomicU64,
    stdin_encode_errors: AtomicU64,
    stdin_encoded_bytes: AtomicU64,
    stdin_encode_ns: AtomicU64,
    stdin_encode_max_ns: AtomicU64,
    stdin_write_flush_calls: AtomicU64,
    stdin_write_flush_errors: AtomicU64,
    stdin_write_attempted_bytes: AtomicU64,
    stdin_write_flush_ns: AtomicU64,
    stdin_write_flush_max_ns: AtomicU64,
}
impl ExternalIpcTelemetry {
    fn record_stdout_read(&self, bytes: usize, elapsed_ns: u64) {
        self.stdout_read_calls.fetch_add(1, Ordering::Relaxed);
        self.stdout_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
        record_duration(
            &self.stdout_read_wait_ns,
            &self.stdout_read_wait_max_ns,
            elapsed_ns,
        );
    }
    fn record_stdout_eof(&self) {
        self.stdout_read_eofs.fetch_add(1, Ordering::Relaxed);
    }
    fn record_stdout_read_error(&self) {
        self.stdout_read_errors.fetch_add(1, Ordering::Relaxed);
    }
    fn record_inbound_decode(&self, elapsed_ns: u64, succeeded: bool) {
        self.inbound_decode_calls.fetch_add(1, Ordering::Relaxed);
        if !succeeded {
            self.inbound_decode_errors.fetch_add(1, Ordering::Relaxed);
        }
        record_duration(
            &self.inbound_decode_ns,
            &self.inbound_decode_max_ns,
            elapsed_ns,
        );
    }
    fn record_inbound_map_enqueue(&self, elapsed_ns: u64, succeeded: bool) {
        self.inbound_map_enqueue_calls
            .fetch_add(1, Ordering::Relaxed);
        if !succeeded {
            self.inbound_map_enqueue_errors
                .fetch_add(1, Ordering::Relaxed);
        }
        record_duration(
            &self.inbound_map_enqueue_ns,
            &self.inbound_map_enqueue_max_ns,
            elapsed_ns,
        );
    }
    fn record_stdin_encode(&self, bytes: usize, elapsed_ns: u64, succeeded: bool) {
        self.stdin_encode_calls.fetch_add(1, Ordering::Relaxed);
        if !succeeded {
            self.stdin_encode_errors.fetch_add(1, Ordering::Relaxed);
        } else {
            self.stdin_encoded_bytes
                .fetch_add(bytes as u64, Ordering::Relaxed);
        }
        record_duration(&self.stdin_encode_ns, &self.stdin_encode_max_ns, elapsed_ns);
    }
    fn record_stdin_write_flush(&self, bytes: usize, elapsed_ns: u64, succeeded: bool) {
        self.stdin_write_flush_calls.fetch_add(1, Ordering::Relaxed);
        self.stdin_write_attempted_bytes
            .fetch_add(bytes as u64, Ordering::Relaxed);
        if !succeeded {
            self.stdin_write_flush_errors
                .fetch_add(1, Ordering::Relaxed);
        }
        record_duration(
            &self.stdin_write_flush_ns,
            &self.stdin_write_flush_max_ns,
            elapsed_ns,
        );
    }
    fn snapshot(&self, route: &'static str) -> IpcTelemetrySnapshot {
        IpcTelemetrySnapshot {
            route,
            available: true,
            stdout_read_calls: load_counter(&self.stdout_read_calls),
            stdout_read_eofs: load_counter(&self.stdout_read_eofs),
            stdout_read_errors: load_counter(&self.stdout_read_errors),
            stdout_bytes: load_counter(&self.stdout_bytes),
            stdout_read_wait_ns: load_counter(&self.stdout_read_wait_ns),
            stdout_read_wait_max_ns: load_counter(&self.stdout_read_wait_max_ns),
            inbound_decode_calls: load_counter(&self.inbound_decode_calls),
            inbound_decode_errors: load_counter(&self.inbound_decode_errors),
            inbound_decode_ns: load_counter(&self.inbound_decode_ns),
            inbound_decode_max_ns: load_counter(&self.inbound_decode_max_ns),
            inbound_map_enqueue_calls: load_counter(&self.inbound_map_enqueue_calls),
            inbound_map_enqueue_errors: load_counter(&self.inbound_map_enqueue_errors),
            inbound_map_enqueue_ns: load_counter(&self.inbound_map_enqueue_ns),
            inbound_map_enqueue_max_ns: load_counter(&self.inbound_map_enqueue_max_ns),
            stdin_encode_calls: load_counter(&self.stdin_encode_calls),
            stdin_encode_errors: load_counter(&self.stdin_encode_errors),
            stdin_encoded_bytes: load_counter(&self.stdin_encoded_bytes),
            stdin_encode_ns: load_counter(&self.stdin_encode_ns),
            stdin_encode_max_ns: load_counter(&self.stdin_encode_max_ns),
            stdin_write_flush_calls: load_counter(&self.stdin_write_flush_calls),
            stdin_write_flush_errors: load_counter(&self.stdin_write_flush_errors),
            stdin_write_attempted_bytes: load_counter(&self.stdin_write_attempted_bytes),
            stdin_write_flush_ns: load_counter(&self.stdin_write_flush_ns),
            stdin_write_flush_max_ns: load_counter(&self.stdin_write_flush_max_ns),
        }
    }
}
fn load_counter(counter: &AtomicU64) -> u64 {
    counter.load(Ordering::Relaxed)
}
fn record_duration(total: &AtomicU64, max: &AtomicU64, elapsed_ns: u64) {
    total.fetch_add(elapsed_ns, Ordering::Relaxed);
    let mut seen = max.load(Ordering::Relaxed);
    while elapsed_ns > seen {
        match max.compare_exchange_weak(seen, elapsed_ns, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(current) => seen = current,
        }
    }
}
fn elapsed_ns(start: Instant) -> u64 {
    start.elapsed().as_nanos().min(u64::MAX as u128) as u64
}
pub struct RemoteConnectionHandle {
    pub event_rx: Receiver<el_protocol_backend::BackendNetworkEvent>,
    pub packet_tx: RemotePacketSender,
    _backend: RemoteConnectionBackend,
    game_rule_editor_supported: Arc<AtomicBool>,
}
#[allow(dead_code)]
pub type ClientNetworkEvent = el_protocol_backend::BackendNetworkEvent;
impl RemoteConnectionHandle {
    pub fn performance_route(&self) -> &'static str {
        match &self._backend {
            RemoteConnectionBackend::External { .. } => "external_ipc",
            RemoteConnectionBackend::Disconnected => "synthetic_disconnected",
        }
    }
    pub fn ipc_telemetry_snapshot(&self) -> IpcTelemetrySnapshot {
        match &self._backend {
            RemoteConnectionBackend::External { _backend } => {
                _backend.telemetry.snapshot("external_ipc")
            }
            RemoteConnectionBackend::Disconnected => {
                IpcTelemetrySnapshot::unavailable("synthetic_disconnected")
            }
        }
    }
    pub fn supports_bundle_item_selection(&self) -> bool {
        matches!(&self._backend, RemoteConnectionBackend::External { .. })
    }
    pub fn supports_game_rule_editor(&self) -> bool {
        self.game_rule_editor_supported.load(Ordering::Acquire)
    }
}
enum RemoteConnectionBackend {
    External { _backend: ExternalProtocolBackend },
    Disconnected,
}
struct ExternalProtocolBackend {
    tx: mpsc::Sender<ClientToBackendMessage>,
    child: Child,
    telemetry: Arc<ExternalIpcTelemetry>,
    _writer_task: thread::JoinHandle<()>,
    _reader_task: thread::JoinHandle<()>,
    _stderr_task: Option<thread::JoinHandle<()>>,
}
impl Drop for ExternalProtocolBackend {
    fn drop(&mut self) {
        let _ = self.tx.send(ClientToBackendMessage::Disconnect {
            reason: "Eltharn client dropped protocol backend".to_string(),
        });
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
#[derive(Clone)]
pub enum RemotePacketSender {
    External(ExternalProtocolPacketSender),
}
#[derive(Clone)]
pub struct ExternalProtocolPacketSender {
    tx: mpsc::Sender<ClientToBackendMessage>,
}
fn protocol_backend_mode_diagnostic(
    mode: &str,
    requested_version: &str,
    backend_path_present: bool,
) -> String {
    format!(
        "[ProtocolBackendDebug] mode={mode} requested_protocol={requested_version} backend_path_present={backend_path_present}"
    )
}
fn protocol_backend_hello_diagnostic(
    requested_version: &str,
    manifest: &el_protocol_backend::ProtocolBackendManifest,
) -> String {
    format!(
        "[ProtocolBackendDebug] mode=external requested_protocol={requested_version} backend_path_present=true manifest_ipc_version={} manifest_minecraft_version={} manifest_protocol_version={} manifest_protocol_source={}",
        manifest.ipc_version,
        manifest.minecraft_version,
        manifest.protocol_version,
        manifest.protocol_source,
    )
}
pub fn spawn_connection(
    args: ClientConnectRequest,
    config: &ProtocolBackendData,
) -> RemoteConnectionHandle {
    let Some(path) = config.backend_path.as_deref() else {
        return spawn_disconnected_connection(format!(
            "Protocol backend required for Minecraft {}. Start through EltharnGate or provide --eltharnProtocolBackend.",
            config.requested_version
        ));
    };
    match spawn_external_connection(&args, &config.requested_version, path) {
        Ok(handle) => {
            el_protocol_backend::diagnostics::write_line(&protocol_backend_mode_diagnostic(
                "external",
                &config.requested_version,
                true,
            ));
            handle
        }
        Err(error) => spawn_disconnected_connection(format!(
            "Failed to start protocol backend for Minecraft {}: {}",
            config.requested_version, error
        )),
    }
}
fn map_connect_request(args: &ClientConnectRequest) -> BackendConnectArgs {
    BackendConnectArgs {
        server: args.server.clone(),
        username: args.username.clone(),
        language: args.language.clone(),
        uuid: args.uuid,
        view_distance: args.view_distance,
        main_hand_left: args.main_hand_left,
        allows_listing: args.allows_listing,
        server_pack_status: args.server_pack_status,
        model_customisation: args.model_customisation,
        server_cookies: args.server_cookies.clone(),
    }
}
fn spawn_disconnected_connection(reason: String) -> RemoteConnectionHandle {
    log::error!("{reason}");
    let (event_tx, event_rx) = crossbeam_channel::unbounded();
    let _ = event_tx.send(BackendNetworkEvent::Disconnected { reason });
    let (tx, _rx) = mpsc::channel::<ClientToBackendMessage>();
    RemoteConnectionHandle {
        event_rx,
        packet_tx: RemotePacketSender::External(ExternalProtocolPacketSender { tx }),
        _backend: RemoteConnectionBackend::Disconnected,
        game_rule_editor_supported: Arc::new(AtomicBool::new(false)),
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn spawn_external_connection(
    args: &ClientConnectRequest,
    requested_version: &str,
    backend_path: &Path,
) -> Result<RemoteConnectionHandle, String> {
    let selection = backend_path
        .to_str()
        .ok_or("Backend selection must be UTF-8")?;
    let target = el_launcher_core::BackendTarget::parse(selection)?;
    let builtin_version = target.builtin_version();
    if let Some(version) = builtin_version {
        if !crate::builtin_backend_manifest()
            .is_some_and(|manifest| manifest.minecraft_version == version)
        {
            return Err(
                "This game executable does not provide the selected built-in backend".to_owned(),
            );
        }
    }
    if builtin_version.is_some_and(|version| version != requested_version) {
        return Err("Built-in backend selection differs from the requested protocol".to_owned());
    }
    if builtin_version.is_none() && !backend_path.exists() {
        return Err(format!(
            "Eltharn protocol backend executable does not exist at {:?}",
            backend_path
        ));
    }
    let executable = if builtin_version.is_some() {
        std::env::current_exe().map_err(|error| error.to_string())?
    } else {
        backend_path.to_path_buf()
    };
    let mut command = Command::new(executable);
    if let Some(version) = builtin_version {
        command.args(["--mode", "backend", "--protocol", version]);
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|err| err.to_string())?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "backend stdin was not captured".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "backend stdout was not captured".to_string())?;
    let stderr = child.stderr.take();
    let (event_tx, event_rx) = crossbeam_channel::unbounded();
    let (packet_tx, packet_rx) = mpsc::channel::<ClientToBackendMessage>();
    let telemetry = Arc::new(ExternalIpcTelemetry::default());
    let game_rule_editor_supported = Arc::new(AtomicBool::new(false));
    let connect_args = map_connect_request(args);
    let (hello_tx, hello_rx) = mpsc::sync_channel(1);
    let writer_event_tx = event_tx.clone();
    let writer_version = requested_version.to_string();
    let writer_telemetry = Arc::clone(&telemetry);
    let writer_task = thread::Builder::new()
        .name("eltharn-protocol-backend-writer".to_string())
        .spawn(move || {
            backend_writer_loop(
                stdin,
                packet_rx,
                writer_version,
                connect_args,
                writer_telemetry,
                hello_rx,
                writer_event_tx,
            );
        })
        .map_err(|err| err.to_string())?;
    let reader_event_tx = event_tx.clone();
    let reader_version = requested_version.to_string();
    let reader_telemetry = Arc::clone(&telemetry);
    let reader_game_rule_support = Arc::clone(&game_rule_editor_supported);
    let reader_task = thread::Builder::new()
        .name("eltharn-protocol-backend-reader".to_string())
        .spawn(move || {
            backend_reader_loop(
                stdout,
                reader_event_tx,
                reader_version,
                reader_telemetry,
                reader_game_rule_support,
                hello_tx,
            );
        })
        .map_err(|err| err.to_string())?;
    let stderr_task = stderr
        .and_then(|stderr| {
            match thread::Builder::new()
                .name("eltharn-protocol-backend-stderr".to_string())
                .spawn(move || backend_stderr_loop(stderr))
            {
                Ok(handle) => Some(handle),
                Err(err) => {
                    log::warn!(
                        "Failed to spawn eltharn-protocol-backend-stderr thread; continuing without stderr capture: {err}"
                    );
                    None
                }
            }
        });
    log::info!(
        "Started Eltharn external protocol backend {:?} for requested {}",
        backend_path,
        requested_version
    );
    Ok(RemoteConnectionHandle {
        event_rx,
        packet_tx: RemotePacketSender::External(ExternalProtocolPacketSender {
            tx: packet_tx.clone(),
        }),
        _backend: RemoteConnectionBackend::External {
            _backend: ExternalProtocolBackend {
                tx: packet_tx,
                child,
                telemetry,
                _writer_task: writer_task,
                _reader_task: reader_task,
                _stderr_task: stderr_task,
            },
        },
        game_rule_editor_supported,
    })
}
#[cfg(target_arch = "wasm32")]
fn spawn_external_connection(
    _args: &ClientConnectRequest,
    _requested_version: &str,
    _backend_path: &Path,
) -> Result<RemoteConnectionHandle, String> {
    Err("External protocol process IPC is not supported on WASM target".to_string())
}
#[cfg(not(target_arch = "wasm32"))]
fn backend_writer_loop(
    mut stdin: std::process::ChildStdin,
    packet_rx: mpsc::Receiver<ClientToBackendMessage>,
    requested_version: String,
    connect_args: BackendConnectArgs,
    telemetry: Arc<ExternalIpcTelemetry>,
    hello_rx: mpsc::Receiver<()>,
    event_tx: Sender<BackendNetworkEvent>,
) {
    if write_backend_message(
        &mut stdin,
        &ClientToBackendMessage::Hello {
            ipc_version: ELTHARN_PROTOCOL_BACKEND_IPC_VERSION,
            requested_minecraft_version: requested_version,
        },
        &telemetry,
    )
    .is_err()
    {
        return;
    }
    if hello_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .is_err()
    {
        let _ = event_tx.send(BackendNetworkEvent::Disconnected {
            reason: "Protocol backend did not complete a compatible IPC Hello".into(),
        });
        return;
    }
    if write_backend_message(
        &mut stdin,
        &ClientToBackendMessage::Connect(connect_args),
        &telemetry,
    )
    .is_err()
    {
        return;
    }
    while let Ok(message) = packet_rx.recv() {
        if write_backend_message(&mut stdin, &message, &telemetry).is_err() {
            return;
        }
        if matches!(message, ClientToBackendMessage::Disconnect { .. }) {
            return;
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn write_backend_message(
    stdin: &mut std::process::ChildStdin,
    message: &ClientToBackendMessage,
    telemetry: &ExternalIpcTelemetry,
) -> Result<(), ()> {
    let encode_started = Instant::now();
    let encoded = encode_json_line(message);
    let encode_elapsed_ns = elapsed_ns(encode_started);
    let line = match encoded {
        Ok(line) => line,
        Err(err) => {
            telemetry.record_stdin_encode(0, encode_elapsed_ns, false);
            log::error!("Failed to encode Eltharn backend IPC message: {err}");
            return Err(());
        }
    };
    telemetry.record_stdin_encode(line.len(), encode_elapsed_ns, true);
    let write_flush_started = Instant::now();
    let write_result = stdin.write_all(&line).and_then(|_| stdin.flush());
    telemetry.record_stdin_write_flush(
        line.len(),
        elapsed_ns(write_flush_started),
        write_result.is_ok(),
    );
    if let Err(err) = write_result {
        log::error!("Failed to write Eltharn backend IPC message: {err}");
        return Err(());
    }
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
fn backend_reader_loop(
    stdout: std::process::ChildStdout,
    event_tx: Sender<BackendNetworkEvent>,
    requested_version: String,
    telemetry: Arc<ExternalIpcTelemetry>,
    game_rule_editor_supported: Arc<AtomicBool>,
    hello_tx: mpsc::SyncSender<()>,
) {
    let mut hello_completed = false;
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        line.clear();
        let read_started = Instant::now();
        let read_result = reader.read_line(&mut line);
        let read_elapsed_ns = elapsed_ns(read_started);
        match read_result {
            Ok(0) => {
                telemetry.record_stdout_read(0, read_elapsed_ns);
                telemetry.record_stdout_eof();
                let _ = event_tx.send(BackendNetworkEvent::Disconnected {
                    reason: "protocol backend stdout closed".to_string(),
                });
                return;
            }
            Ok(bytes) => telemetry.record_stdout_read(bytes, read_elapsed_ns),
            Err(err) => {
                telemetry.record_stdout_read(0, read_elapsed_ns);
                telemetry.record_stdout_read_error();
                let _ = event_tx.send(BackendNetworkEvent::Disconnected {
                    reason: format!("protocol backend read error: {err}"),
                });
                return;
            }
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let decode_started = Instant::now();
        let decoded = decode_json_line::<BackendToClientMessage>(trimmed.as_bytes());
        let decode_elapsed_ns = elapsed_ns(decode_started);
        let message = match decoded {
            Ok(message) => message,
            Err(err) => {
                telemetry.record_inbound_decode(decode_elapsed_ns, false);
                let _ = event_tx.send(BackendNetworkEvent::Disconnected {
                    reason: format!("protocol backend decoded invalid message: {err}"),
                });
                return;
            }
        };
        telemetry.record_inbound_decode(decode_elapsed_ns, true);
        let is_hello = matches!(&message, BackendToClientMessage::Hello { .. });
        if !hello_completed && matches!(&message, BackendToClientMessage::Event(_)) {
            let _ = event_tx.send(BackendNetworkEvent::Disconnected {
                reason: "Protocol backend sent gameplay before a compatible IPC Hello".into(),
            });
            return;
        }
        let map_enqueue_started = Instant::now();
        let map_enqueue_result = handle_backend_message_with_support(
            message,
            &event_tx,
            &requested_version,
            &game_rule_editor_supported,
        );
        telemetry.record_inbound_map_enqueue(
            elapsed_ns(map_enqueue_started),
            map_enqueue_result.is_ok(),
        );
        if map_enqueue_result.is_err() {
            return;
        }
        if is_hello && !hello_completed {
            hello_completed = true;
            let _ = hello_tx.send(());
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn backend_stderr_loop(stderr: std::process::ChildStderr) {
    let mut reader = BufReader::new(stderr);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return,
            Ok(_) => {
                let trimmed = line.trim_end();
                if !trimmed.is_empty() {
                    eprintln!("[EltharnBackendStderr] {trimmed}");
                    log::warn!("[EltharnBackendStderr] {trimmed}");
                    el_protocol_backend::diagnostics::write_line(&format!(
                        "[EltharnProtocolBackend stderr] {trimmed}"
                    ));
                }
            }
            Err(err) => {
                log::warn!("Eltharn protocol backend stderr read error: {err}");
                return;
            }
        }
    }
}
fn handle_backend_message_with_support(
    message: BackendToClientMessage,
    event_tx: &Sender<BackendNetworkEvent>,
    requested_version: &str,
    game_rule_editor_supported: &AtomicBool,
) -> Result<(), ()> {
    let declared_game_rule_support = match &message {
        BackendToClientMessage::Hello { manifest } => Some(manifest.supports_game_rule_editor),
        _ => None,
    };
    match &message {
        BackendToClientMessage::Hello { manifest } => {
            el_protocol_backend::diagnostics::write_line(&protocol_backend_hello_diagnostic(
                requested_version,
                &manifest,
            ));
            log::info!(
                "Eltharn protocol backend ready: {} minecraft={} protocol={} source={}",
                manifest.backend_name,
                manifest.minecraft_version,
                manifest.protocol_version,
                manifest.protocol_source
            );
        }
        BackendToClientMessage::Status { level, message } => log_backend_status(*level, message),
        BackendToClientMessage::Event(event) => {
            if backend_event_trace_enabled() {
                let event_name = backend_event_kind_name(&event);
                let line = format!("[EltharnBackend] event kind={event_name}");
                el_protocol_backend::diagnostics::write_line(&line);
                eprintln!("{line}");
            }
        }
        BackendToClientMessage::Disconnected { reason } => {
            eprintln!("[EltharnBackend] IPC Disconnected: {reason}");
        }
    }
    match crate::network::inbound::into_event(message, requested_version) {
        Ok(Some(event)) => {
            if let Some(supported) = declared_game_rule_support {
                game_rule_editor_supported.store(supported, Ordering::Release);
            }
            let _ = event_tx.send(event);
            Ok(())
        }
        Ok(None) => {
            if let Some(supported) = declared_game_rule_support {
                game_rule_editor_supported.store(supported, Ordering::Release);
            }
            Ok(())
        }
        Err(reason) => {
            log::error!("{reason}");
            let _ = event_tx.send(BackendNetworkEvent::Disconnected { reason });
            Err(())
        }
    }
}
fn backend_event_kind_name(event: &BackendNetworkEvent) -> &'static str {
    match event {
        BackendNetworkEvent::Connected { .. } => "connected",
        BackendNetworkEvent::Disconnected { .. } => "disconnected",
        BackendNetworkEvent::Login { .. } => "login",
        BackendNetworkEvent::Respawn { .. } => "respawn",
        BackendNetworkEvent::PlayerPosition { .. } => "player_position",
        BackendNetworkEvent::PlayerRotation { .. } => "player_rotation",
        BackendNetworkEvent::ChunkLoaded { .. } => "chunk_loaded",
        BackendNetworkEvent::ChunkUnloaded { .. } => "chunk_unloaded",
        BackendNetworkEvent::ChunkCacheCenter { .. } => "chunk_cache_center",
        BackendNetworkEvent::ChunkCacheRadius { .. } => "chunk_cache_radius",
        BackendNetworkEvent::SimulationDistance { .. } => "simulation_distance",
        BackendNetworkEvent::BlockUpdate { .. } => "block_update",
        BackendNetworkEvent::SectionBlocksUpdate { .. } => "section_blocks_update",
        BackendNetworkEvent::Sound { .. } => "sound",
        BackendNetworkEvent::LevelEvent { .. } => "level_event",
        BackendNetworkEvent::SoundEntity { .. } => "sound_entity",
        BackendNetworkEvent::StopSound { .. } => "stop_sound",
        BackendNetworkEvent::BlockEvent { .. } => "block_event",
        BackendNetworkEvent::BlockEntityUpdate { .. } => "block_entity_update",
        BackendNetworkEvent::LightUpdate { .. } => "light_update",
        BackendNetworkEvent::GameEvent { .. } => "game_event",
        BackendNetworkEvent::ChunkBatchStart => "chunk_batch_start",
        BackendNetworkEvent::ChunkBatchFinished { .. } => "chunk_batch_finished",
        BackendNetworkEvent::SystemChat { .. } => "system_chat",
        BackendNetworkEvent::PlayerChat { .. } => "player_chat",
        BackendNetworkEvent::Health { .. } => "health",
        BackendNetworkEvent::Experience { .. } => "experience",
        BackendNetworkEvent::Abilities { .. } => "abilities",
        BackendNetworkEvent::Time { .. } => "time",
        BackendNetworkEvent::EntityAdded { .. } => "entity_added",
        BackendNetworkEvent::EntitiesRemoved { .. } => "entities_removed",
        BackendNetworkEvent::EntityTeleport { .. } => "entity_teleport",
        BackendNetworkEvent::EntityDeltaMoved { .. } => "entity_delta_moved",
        BackendNetworkEvent::EntityHeadLook { .. } => "entity_head_look",
        BackendNetworkEvent::Ping { .. } => "ping",
        BackendNetworkEvent::PongResponse { .. } => "pong_response",
        BackendNetworkEvent::ProjectilePower { .. } => "projectile_power",
        BackendNetworkEvent::PlayerLookAt { .. } => "player_look_at",
        BackendNetworkEvent::BlockDestruction { .. } => "block_destruction",
        BackendNetworkEvent::SetEntityLink { .. } => "set_entity_link",
        BackendNetworkEvent::LevelParticles(_) => "level_particles",
        BackendNetworkEvent::DebugBlockValue { .. } => "debug_block_value",
        BackendNetworkEvent::DebugChunkValue { .. } => "debug_chunk_value",
        BackendNetworkEvent::DebugEntityValue { .. } => "debug_entity_value",
        BackendNetworkEvent::DebugEvent { .. } => "debug_event",
        BackendNetworkEvent::GameTestHighlightPos { .. } => "game_test_highlight_pos",
        BackendNetworkEvent::MountScreenOpen { .. } => "mount_screen_open",
        BackendNetworkEvent::SelectAdvancementsTab { .. } => "select_advancements_tab",
        BackendNetworkEvent::AwardStats { .. } => "award_stats",
        BackendNetworkEvent::UpdateAdvancements { .. } => "update_advancements",
        BackendNetworkEvent::TestInstanceBlockStatus { .. } => "test_instance_block_status",
        BackendNetworkEvent::ShowDialog { .. } => "show_dialog",
        BackendNetworkEvent::ClearDialog => "clear_dialog",
        BackendNetworkEvent::ResetChat => "reset_chat",
        BackendNetworkEvent::AddPlayerInfo { .. } => "add_player_info",
        BackendNetworkEvent::RemovePlayerInfo { .. } => "remove_player_info",
        BackendNetworkEvent::PlayerInfoUpdate { .. } => "player_info_update",
        BackendNetworkEvent::ResourcePackPush { .. } => "resource_pack_push",
        BackendNetworkEvent::CookieRequest { .. } => "cookie_request",
        BackendNetworkEvent::RegistryData { .. } => "registry_data",
        BackendNetworkEvent::SelectKnownPacks { .. } => "select_known_packs",
        BackendNetworkEvent::ServerData { .. } => "server_data",
        _ => "other",
    }
}
fn log_backend_status(level: BackendLogLevel, message: &str) {
    match level {
        BackendLogLevel::Trace => log::trace!("[EltharnProtocolBackend] {message}"),
        BackendLogLevel::Debug => log::debug!("[EltharnProtocolBackend] {message}"),
        BackendLogLevel::Info => log::info!("[EltharnProtocolBackend] {message}"),
        BackendLogLevel::Warn => log::warn!("[EltharnProtocolBackend] {message}"),
        BackendLogLevel::Error => log::error!("[EltharnProtocolBackend] {message}"),
    }
}
impl RemotePacketSender {
    pub fn send_custom_payload(&self, channel: String, data: Vec<u8>) -> Result<(), String> {
        el_protocol_backend::payload::validate_custom_payload(&channel, &data, true)?;
        match self {
            Self::External(sender) => sender
                .try_send_packet(BackendOutboundPacket::CustomPayload { channel, data })
                .then_some(())
                .ok_or_else(|| "Mod payload transport closed".into()),
        }
    }
    pub fn send_resource_pack_status(&self, id: uuid::Uuid, action: u8) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::ResourcePackStatus { id, action })
            }
        }
    }
    pub fn send_chat(&self, message: impl Into<String>) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::Chat {
                message: message.into(),
            }),
        }
    }
    pub fn send_command(&self, command: impl Into<String>) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::Command {
                command: command.into(),
            }),
        }
    }
    pub fn send_custom_click_action(&self, id: String, payload: Option<Vec<u8>>) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::CustomClickAction { id, payload })
            }
        }
    }
    pub fn send_command_suggestion(&self, id: u32, command: impl Into<String>) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::CommandSuggestion {
                    id,
                    command: command.into(),
                })
            }
        }
    }
    pub fn send_player_loaded(&self) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::PlayerLoaded),
        }
    }
    pub fn send_client_tick_end(&self) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::ClientTickEnd),
        }
    }
    pub fn send_request_stats(&self) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::RequestStats),
        }
    }
    pub fn send_request_game_rule_values(&self) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::RequestGameRuleValues)
            }
        }
    }
    pub fn send_pong(&self, id: u32) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::Pong { id }),
        }
    }
    pub fn send_perform_respawn(&self) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::PerformRespawn),
        }
    }
    pub fn send_move_pos_rot(
        &self,
        x: f64,
        y: f64,
        z: f64,
        yaw_degrees: f32,
        pitch_degrees: f32,
        on_ground: bool,
        horizontal_collision: bool,
    ) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::MovePosRot {
                x,
                y,
                z,
                yaw_degrees,
                pitch_degrees,
                on_ground,
                horizontal_collision,
            }),
        }
    }
    pub fn send_move_pos(
        &self,
        x: f64,
        y: f64,
        z: f64,
        on_ground: bool,
        horizontal_collision: bool,
    ) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::MovePos {
                x,
                y,
                z,
                on_ground,
                horizontal_collision,
            }),
        }
    }
    pub fn send_move_rot(
        &self,
        yaw_degrees: f32,
        pitch_degrees: f32,
        on_ground: bool,
        horizontal_collision: bool,
    ) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::MoveRot {
                yaw_degrees,
                pitch_degrees,
                on_ground,
                horizontal_collision,
            }),
        }
    }
    pub fn send_move_status_only(&self, on_ground: bool, horizontal_collision: bool) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::MoveStatusOnly {
                on_ground,
                horizontal_collision,
            }),
        }
    }
    pub fn send_move_vehicle(
        &self,
        x: f64,
        y: f64,
        z: f64,
        yaw_degrees: f32,
        pitch_degrees: f32,
        on_ground: bool,
    ) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::MoveVehicle {
                x,
                y,
                z,
                yaw_degrees,
                pitch_degrees,
                on_ground,
            }),
        }
    }
    pub fn send_paddle_boat(&self, left: bool, right: bool) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::PaddleBoat { left, right })
            }
        }
    }
    pub fn send_player_input(
        &self,
        forward: bool,
        backward: bool,
        left: bool,
        right: bool,
        jump: bool,
        shift: bool,
        sprint: bool,
    ) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::PlayerInput {
                forward,
                backward,
                left,
                right,
                jump,
                shift,
                sprint,
            }),
        }
    }
    pub fn send_player_command(&self, entity_id: i32, action: &str, data: i32) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::PlayerCommand {
                entity_id,
                action: action.to_string(),
                data,
            }),
        }
    }
    pub fn send_accept_teleportation(&self, id: u32) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::AcceptTeleport { teleport_id: id })
            }
        }
    }
    pub fn send_teleport_confirm(
        &self,
        teleport_id: u32,
        x: f64,
        y: f64,
        z: f64,
        yaw_degrees: f32,
        pitch_degrees: f32,
    ) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::TeleportConfirm {
                teleport_id,
                x,
                y,
                z,
                yaw_degrees,
                pitch_degrees,
            }),
        }
    }
    pub fn send_set_carried_item(&self, slot: u16) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::SetCarriedItem { slot })
            }
        }
    }
    pub fn send_start_destroy_block(&self, x: i32, y: i32, z: i32, direction: u8, seq: u32) {
        self.send_player_action(
            BackendPlayerActionKind::StartDestroyBlock,
            x,
            y,
            z,
            direction,
            seq,
        );
    }
    pub fn send_stop_destroy_block(&self, x: i32, y: i32, z: i32, direction: u8, seq: u32) {
        self.send_player_action(
            BackendPlayerActionKind::StopDestroyBlock,
            x,
            y,
            z,
            direction,
            seq,
        );
    }
    pub fn send_abort_destroy_block(&self, x: i32, y: i32, z: i32, direction: u8, seq: u32) {
        self.send_player_action(
            BackendPlayerActionKind::AbortDestroyBlock,
            x,
            y,
            z,
            direction,
            seq,
        );
    }
    pub fn send_swap_item_with_offhand(&self) {
        self.send_player_action(BackendPlayerActionKind::SwapItemWithOffhand, 0, 0, 0, 0, 0);
    }
    pub fn send_drop_item(&self) {
        self.send_player_action(BackendPlayerActionKind::DropItem, 0, 0, 0, 0, 0);
    }
    pub fn send_drop_all_items(&self) {
        self.send_player_action(BackendPlayerActionKind::DropAllItems, 0, 0, 0, 0, 0);
    }
    pub fn send_release_using_item(&self) {
        self.send_player_action(BackendPlayerActionKind::ReleaseUseItem, 0, 0, 0, 0, 0);
    }
    pub fn send_stab(&self) {
        self.send_player_action(BackendPlayerActionKind::Stab, 0, 0, 0, 0, 0);
    }
    fn send_player_action(
        &self,
        action: BackendPlayerActionKind,
        x: i32,
        y: i32,
        z: i32,
        direction: u8,
        seq: u32,
    ) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::PlayerAction {
                action,
                x,
                y,
                z,
                direction,
                seq,
            }),
        }
    }
    pub fn send_use_item_on(
        &self,
        hand: u8,
        x: i32,
        y: i32,
        z: i32,
        direction: u8,
        cursor_x: f32,
        cursor_y: f32,
        cursor_z: f32,
        inside: bool,
        seq: u32,
    ) {
        match self {
            Self::External(sender) => {
                println!(
                    "[MenuDebug] PROTOCOL_SEND_USE_ITEM_ON backend=external hand={} seq={} pos=[{},{},{}] face={} cursor=[{:.3},{:.3},{:.3}] inside={}",
                    hand, seq, x, y, z, direction, cursor_x, cursor_y, cursor_z, inside
                );
                sender.send_packet(BackendOutboundPacket::UseItemOn {
                    hand,
                    x,
                    y,
                    z,
                    direction,
                    cursor_x,
                    cursor_y,
                    cursor_z,
                    inside,
                    seq,
                })
            }
        }
    }
    pub fn send_use_item(&self, hand: u8, seq: u32, yaw_degrees: f32, pitch_degrees: f32) {
        match self {
            Self::External(sender) => {
                println!(
                    "[MenuDebug] PROTOCOL_SEND_USE_ITEM backend=external hand={} seq={} yaw={:.2} pitch={:.2}",
                    hand, seq, yaw_degrees, pitch_degrees
                );
                sender.send_packet(BackendOutboundPacket::UseItem {
                    hand,
                    seq,
                    yaw_degrees,
                    pitch_degrees,
                })
            }
        }
    }
    pub fn send_swing_main_hand(&self) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::SwingMainHand),
        }
    }
    pub fn send_swing_off_hand(&self) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::SwingOffHand),
        }
    }
    pub fn send_attack(&self, entity_id: i32, using_secondary_action: bool) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::Attack {
                entity_id,
                using_secondary_action,
            }),
        }
    }
    pub fn send_interact_entity(
        &self,
        entity_id: i32,
        hand: u8,
        relative_x: f32,
        relative_y: f32,
        relative_z: f32,
        using_secondary_action: bool,
    ) -> bool {
        match self {
            Self::External(sender) => sender.try_send_packet(BackendOutboundPacket::InteractAt {
                entity_id,
                hand,
                relative_x,
                relative_y,
                relative_z,
                using_secondary_action,
            }),
        }
    }
    pub fn send_container_click(&self, packet: ServerboundContainerClickPacket) {
        println!(
            "[MenuDebug] PROTOCOL_SEND_CONTAINER_CLICK backend={} container={} state={} slot={} button={} input={:?} changed={} carried={}x{}",
            match self {
                Self::External(_) => "external",
            },
            packet.container_id,
            packet.state_id,
            packet.slot_num,
            packet.button_num,
            packet.container_input,
            packet.changed_slots.len(),
            packet.carried_item.item.location,
            packet.carried_item.count
        );
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::ContainerClick {
                container_id: packet.container_id,
                state_id: packet.state_id,
                slot_num: packet.slot_num,
                button_num: packet.button_num,
                click_type: packet.container_input.id(),
                changed_slots: packet
                    .changed_slots
                    .iter()
                    .map(|(slot, stack)| (*slot, map_item_stack_to_backend(stack)))
                    .collect(),
                carried_item: map_item_stack_to_backend(&packet.carried_item),
            }),
        }
    }
    pub fn send_set_creative_mode_slot(&self, packet: ServerboundSetCreativeModeSlotPacket) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::SetCreativeModeSlot {
                    slot_num: packet.slot_num,
                    item_stack: map_item_stack_to_backend(&packet.item_stack),
                })
            }
        }
    }
    pub fn send_container_close(&self, container_id: i32) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::ContainerClose { container_id })
            }
        }
    }
    pub fn send_player_abilities(&self, is_flying: bool) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::PlayerAbilities { is_flying })
            }
        }
    }
    pub fn send_change_game_mode(&self, mode: crate::gameplay::game_mode::GameType) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::ChangeGameMode {
                mode: mode.name().to_string(),
            }),
        }
    }
    pub fn send_sign_update(
        &self,
        x: i32,
        y: i32,
        z: i32,
        is_front_text: bool,
        lines: [String; 4],
    ) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::SignUpdate {
                x,
                y,
                z,
                is_front_text,
                lines,
            }),
        }
    }
    pub fn send_edit_book(&self, slot: u32, pages: Vec<String>, title: Option<String>) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::EditBook { slot, pages, title })
            }
        }
    }
    pub fn send_recipe_book_seen_recipe(&self, recipe: String) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::RecipeBookSeenRecipe { recipe })
            }
        }
    }
    pub fn send_recipe_book_change_settings(
        &self,
        book_type: u8,
        is_open: bool,
        is_filtering: bool,
    ) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::RecipeBookChangeSettings {
                    book_type,
                    is_open,
                    is_filtering,
                })
            }
        }
    }
    pub fn send_place_recipe(&self, container_id: i32, recipe: String, shift_down: bool) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::PlaceRecipe {
                container_id,
                recipe,
                shift_down,
            }),
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn send_set_structure_block(
        &self,
        x: i32,
        y: i32,
        z: i32,
        update_type: u8,
        mode: u8,
        name: String,
        offset_x: i8,
        offset_y: i8,
        offset_z: i8,
        size_x: u8,
        size_y: u8,
        size_z: u8,
        mirror: u8,
        rotation: u8,
        data: String,
        integrity: f32,
        seed: u64,
        ignore_entities: bool,
        show_air: bool,
        show_bounding_box: bool,
    ) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::SetStructureBlock {
                    x,
                    y,
                    z,
                    update_type,
                    mode,
                    name,
                    offset_x,
                    offset_y,
                    offset_z,
                    size_x,
                    size_y,
                    size_z,
                    mirror,
                    rotation,
                    data,
                    integrity,
                    seed,
                    ignore_entities,
                    show_air,
                    show_bounding_box,
                })
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn send_set_jigsaw_block(
        &self,
        x: i32,
        y: i32,
        z: i32,
        name: String,
        target: String,
        pool: String,
        final_state: String,
        joint: String,
        selection_priority: i32,
        placement_priority: i32,
    ) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::SetJigsawBlock {
                x,
                y,
                z,
                name,
                target,
                pool,
                final_state,
                joint,
                selection_priority,
                placement_priority,
            }),
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn send_set_command_block(
        &self,
        x: i32,
        y: i32,
        z: i32,
        command: String,
        mode: u8,
        track_output: bool,
        conditional: bool,
        automatic: bool,
    ) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::SetCommandBlock {
                x,
                y,
                z,
                command,
                mode,
                track_output,
                conditional,
                automatic,
            }),
        }
    }
    pub fn send_set_command_minecart(&self, entity_id: u32, command: String, track_output: bool) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::SetCommandMinecart {
                    entity_id,
                    command,
                    track_output,
                })
            }
        }
    }
    pub fn send_set_test_block(&self, x: i32, y: i32, z: i32, mode: u8, message: String) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::SetTestBlock {
                x,
                y,
                z,
                mode,
                message,
            }),
        }
    }
    pub fn send_spectator_action(&self, spectate_entity_id: Option<i32>) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::SpectatorAction { spectate_entity_id })
            }
        }
    }
    pub fn send_pick_item(&self, slot: u32) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::PickItem { slot }),
        }
    }
    pub fn send_pick_item_from_entity(&self, id: i32, include_data: bool) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::PickItemFromEntity { id, include_data })
            }
        }
    }
    pub fn send_select_trade(&self, item: u32) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::SelectTrade { item })
            }
        }
    }
    pub fn send_set_beacon(&self, primary: Option<u32>, secondary: Option<u32>) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::SetBeacon { primary, secondary })
            }
        }
    }
    pub fn send_rename_item(&self, name: String) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::RenameItem { name })
            }
        }
    }
    pub fn send_cookie_response(&self, key: String, payload: Option<Vec<u8>>) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::CookieResponse { key, payload })
            }
        }
    }
    pub fn send_accept_code_of_conduct(&self) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::AcceptCodeOfConduct)
            }
        }
    }
    pub fn send_set_game_rule(&self, key: String, value: String) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::SetGameRule { key, value })
            }
        }
    }
    pub fn send_set_game_rules(&self, entries: Vec<(String, String)>) {
        match self {
            Self::External(sender) => {
                for (key, value) in entries {
                    sender.send_packet(BackendOutboundPacket::SetGameRule { key, value });
                }
            }
        }
    }
    pub fn send_lock_difficulty(&self, locked: bool) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::LockDifficulty { locked })
            }
        }
    }
    pub fn send_seen_advancements(&self, action: u8, tab: Option<String>) {
        let action = match action {
            0 => BackendSeenAdvancementsAction::OpenedTab,
            _ => BackendSeenAdvancementsAction::ClosedScreen,
        };
        self.send_seen_advancements_action(action, tab);
    }
    pub fn send_seen_advancements_action(
        &self,
        action: BackendSeenAdvancementsAction,
        tab: Option<String>,
    ) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::SeenAdvancements { action, tab })
            }
        }
    }
    pub fn send_chat_session_update(
        &self,
        session_id: uuid::Uuid,
        expires_at: u64,
        key: Vec<u8>,
        key_signature: Vec<u8>,
    ) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::ChatSessionUpdate {
                    session_id,
                    expires_at,
                    key,
                    key_signature,
                })
            }
        }
    }
    pub fn send_chat_ack(&self, offset: u32) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::ChatAck { offset }),
        }
    }
    pub fn send_bundle_item_selected(&self, slot_id: i32, selected_item_index: u32) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::BundleItemSelected {
                    slot_id,
                    selected_item_index,
                })
            }
        }
    }
    pub fn send_container_button_click(&self, container_id: i32, button_id: u32) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::ContainerButtonClick {
                    container_id,
                    button_id,
                })
            }
        }
    }
    pub fn send_container_slot_state_changed(
        &self,
        slot_id: u32,
        container_id: i32,
        new_state: bool,
    ) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::ContainerSlotStateChanged {
                    slot_id,
                    container_id,
                    new_state,
                })
            }
        }
    }
    pub fn send_block_entity_tag_query(&self, transaction_id: u32, x: i32, y: i32, z: i32) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::BlockEntityTagQuery {
                    transaction_id,
                    x,
                    y,
                    z,
                })
            }
        }
    }
    pub fn send_entity_tag_query(&self, transaction_id: u32, entity_id: i32) {
        match self {
            Self::External(sender) => sender.send_packet(BackendOutboundPacket::EntityTagQuery {
                transaction_id,
                entity_id,
            }),
        }
    }
    pub fn send_debug_subscription(&self, subscriptions: Vec<String>) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::DebugSubscription { subscriptions })
            }
        }
    }
    pub fn send_test_instance_block_action(
        &self,
        x: i32,
        y: i32,
        z: i32,
        action: u8,
        test: Option<String>,
        size_x: i32,
        size_y: i32,
        size_z: i32,
        rotation: u8,
        ignore_entities: bool,
        status: u8,
        error_message: Option<String>,
    ) {
        match self {
            Self::External(sender) => {
                sender.send_packet(BackendOutboundPacket::TestInstanceBlockAction {
                    x,
                    y,
                    z,
                    action,
                    test,
                    size_x,
                    size_y,
                    size_z,
                    rotation,
                    ignore_entities,
                    status,
                    error_message,
                })
            }
        }
    }
}
impl ExternalProtocolPacketSender {
    fn send_packet(&self, packet: BackendOutboundPacket) {
        let _ = self.try_send_packet(packet);
    }
    fn try_send_packet(&self, packet: BackendOutboundPacket) -> bool {
        if let Err(err) = self.tx.send(ClientToBackendMessage::Packet(packet)) {
            log::warn!("Failed to send packet to Eltharn protocol backend: {err}");
            false
        } else {
            true
        }
    }
}
