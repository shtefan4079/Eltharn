use el_protocol_backend::{
    BackendToClientMessage, ClientToBackendMessage, ELTHARN_PROTOCOL_BACKEND_IPC_VERSION,
    ProtocolBackendManifest, VANILLA_PROTOCOL_VERSION, decode_json_line, encode_json_line,
};
use futures_util::{SinkExt, StreamExt};
use std::{env, process::Stdio, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    process::{Child, ChildStdin, Command},
    sync::Semaphore,
};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        Message,
        handshake::server::{Request, Response},
    },
};
const DEFAULT_BIND: &str = "127.0.0.1:9001";
const DEFAULT_MAX_MESSAGE_BYTES: usize = 1024 * 1024;
const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 600;
const DEFAULT_MAX_CONNECTIONS: usize = 64;
const EXPECTED_BACKEND_1_21_11_NAME: &str = "eltharn-protocol-1.21.11";
fn default_backend_1_21_11() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.parent().map(|parent| {
                parent
                    .join(if cfg!(windows) {
                        "eltharn-protocol.exe"
                    } else {
                        "eltharn-protocol"
                    })
                    .to_string_lossy()
                    .into_owned()
            })
        })
        .unwrap_or_else(|| "eltharn-protocol".to_owned())
}
#[derive(Clone, Debug)]
struct Config {
    bind: String,
    backend_1_21_11: String,
    backend_bundles: std::collections::BTreeMap<String, String>,
    fixed_server: Option<String>,
    allowed_origin: Option<String>,
    max_connections: usize,
    max_message_bytes: usize,
    idle_timeout: Duration,
}
impl Config {
    fn from_env() -> Result<Self, String> {
        let backend_bundles = match env::var("ELTHARN_GATE_BACKEND_BUNDLES") {
            Ok(json) => serde_json::from_str(&json).map_err(|error| {
                format!(
                    "ELTHARN_GATE_BACKEND_BUNDLES must be a JSON version-to-executable map: {error}"
                )
            })?,
            Err(_) => Default::default(),
        };
        Ok(Self {
            bind: env::var("ELTHARN_GATE_GATEWAY_BIND")
                .unwrap_or_else(|_| DEFAULT_BIND.to_string()),
            backend_1_21_11: env::var("ELTHARN_GATE_BACKEND_1_21_11")
                .unwrap_or_else(|_| default_backend_1_21_11()),
            backend_bundles,
            fixed_server: env::var("ELTHARN_GATE_SERVER")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            allowed_origin: env::var("ELTHARN_GATE_ALLOWED_ORIGIN")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            max_connections: env::var("ELTHARN_GATE_MAX_CONNECTIONS")
                .ok()
                .and_then(|value| value.parse().ok())
                .filter(|value: &usize| *value > 0)
                .unwrap_or(DEFAULT_MAX_CONNECTIONS),
            max_message_bytes: env::var("ELTHARN_GATE_MAX_MESSAGE_BYTES")
                .ok()
                .and_then(|value| value.parse().ok())
                .filter(|value: &usize| *value >= 1024)
                .unwrap_or(DEFAULT_MAX_MESSAGE_BYTES),
            idle_timeout: Duration::from_secs(
                env::var("ELTHARN_GATE_IDLE_TIMEOUT_SECS")
                    .ok()
                    .and_then(|value| value.parse().ok())
                    .filter(|value: &u64| *value > 0)
                    .unwrap_or(DEFAULT_IDLE_TIMEOUT_SECS),
            ),
        })
    }
    fn backend_for_protocol(&self, protocol: &str) -> Result<&str, String> {
        if let Some(path) = self.backend_bundles.get(protocol) {
            return Ok(path.as_str());
        }
        match protocol {
            "1.21.11" => Ok(self.backend_1_21_11.as_str()),
            other => Err(format!("unsupported browser protocol: {other}")),
        }
    }
}
fn print_help() {
    println!(
        concat!(
            "eltharn_gate-gateway — WebSocket to eltharn-protocol bridge for the EltharnGate browser client\n",
            "\n",
            "Usage: eltharn_gate-gateway   (all options are environment variables)\n",
            "\n",
            "Environment:\n",
            "  - ELTHARN_GATE_GATEWAY_BIND       listen address (default {}); a non-loopback address\n",
            "                              requires ELTHARN_GATE_ALLOWED_ORIGIN\n",
            "  - ELTHARN_GATE_ALLOWED_ORIGIN     exact browser Origin allow-list entry, e.g. https://play.example.\n",
            "                              When unset, only requests without an Origin header or from a\n",
            "                              loopback page may connect.\n",
            "  - ELTHARN_GATE_BACKEND_1_21_11    pinned 1.21.11 backend executable\n",
            "  - ELTHARN_GATE_BACKEND_BUNDLES    JSON version-to-executable map for extra versions\n",
            "  - ELTHARN_GATE_SERVER             pins the upstream Minecraft server address\n",
            "  - ELTHARN_GATE_IDLE_TIMEOUT_SECS  idle connection timeout in seconds (default {})\n",
            "  - ELTHARN_GATE_MAX_CONNECTIONS    concurrent connection limit (default {})\n",
            "  - ELTHARN_GATE_MAX_MESSAGE_BYTES  message and backend line limit (default {})",
        ),
        DEFAULT_BIND, DEFAULT_IDLE_TIMEOUT_SECS, DEFAULT_MAX_CONNECTIONS, DEFAULT_MAX_MESSAGE_BYTES,
    );
}
fn bind_is_loopback(bind: &str) -> bool {
    if let Ok(address) = bind.parse::<std::net::SocketAddr>() {
        return address.ip().is_loopback();
    }
    let host = bind
        .rsplit_once(':')
        .map(|(host, _)| host)
        .unwrap_or(bind)
        .trim_matches(|c| c == '[' || c == ']');
    host.eq_ignore_ascii_case("localhost")
}
fn is_loopback_origin(origin: &str) -> bool {
    let Some(rest) = origin.strip_prefix("http://") else {
        return false;
    };
    let host = match rest.rsplit_once(':') {
        Some((host, port)) => {
            if port.is_empty() || !port.bytes().all(|byte| byte.is_ascii_digit()) {
                return false;
            }
            host
        }
        None => rest,
    };
    host == "127.0.0.1" || host.eq_ignore_ascii_case("localhost")
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::args().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(());
    }
    let config = match Config::from_env() {
        Ok(config) => Arc::new(config),
        Err(error) => {
            eprintln!("eltharn_gate-gateway: {error}");
            std::process::exit(1);
        }
    };
    if config.allowed_origin.is_none() && !bind_is_loopback(&config.bind) {
        eprintln!(
            "eltharn_gate-gateway: refusing to bind non-loopback address {:?} without ELTHARN_GATE_ALLOWED_ORIGIN; \
             set the exact browser origin or bind 127.0.0.1",
            config.bind
        );
        std::process::exit(1);
    }
    let connection_slots = Arc::new(Semaphore::new(config.max_connections));
    let listener = TcpListener::bind(&config.bind).await?;
    eprintln!("eltharn_gate-gateway listening on {}", config.bind);
    loop {
        let (stream, peer) = listener.accept().await?;
        let config = Arc::clone(&config);
        let connection_slots = Arc::clone(&connection_slots);
        tokio::spawn(async move {
            let Ok(_slot) = connection_slots.try_acquire_owned() else {
                eprintln!("gateway connection {peer} rejected: connection limit reached");
                return;
            };
            if let Err(error) = handle_connection(stream, config).await {
                eprintln!("gateway connection {peer} failed: {error}");
            }
        });
    }
}
async fn handle_connection(stream: TcpStream, config: Arc<Config>) -> Result<(), String> {
    let allowed_origin = config.allowed_origin.clone();
    let mut websocket = accept_hdr_async(stream, move |request: &Request, response: Response| {
        let actual = request
            .headers()
            .get("Origin")
            .and_then(|value| value.to_str().ok());
        let allowed = match allowed_origin.as_deref() {
            Some(expected) => actual == Some(expected),
            None => actual.is_none_or(is_loopback_origin),
        };
        if !allowed {
            let error = Response::builder()
                .status(403)
                .body(Some("origin is not allowed".to_string()))
                .map_err(|_| ())
                .unwrap();
            return Err(error);
        }
        Ok(response)
    })
    .await
    .map_err(|error| format!("websocket handshake: {error}"))?;
    let first = websocket
        .next()
        .await
        .ok_or_else(|| "client closed before hello".to_string())?
        .map_err(|error| format!("websocket read: {error}"))?;
    let first_bytes = message_bytes(first)?;
    if first_bytes.len() > config.max_message_bytes {
        return send_disconnect(&mut websocket, "hello exceeds gateway message limit").await;
    }
    let hello: ClientToBackendMessage =
        decode_json_line(&first_bytes).map_err(|error| format!("invalid hello JSON: {error}"))?;
    let ClientToBackendMessage::Hello {
        requested_minecraft_version,
        ..
    } = &hello
    else {
        return send_disconnect(&mut websocket, "first gateway message must be hello").await;
    };
    let backend = config.backend_for_protocol(requested_minecraft_version)?;
    let first_bytes = normalize_client_message(
        &first_bytes,
        config.max_message_bytes,
        config.fixed_server.as_deref(),
    )?;
    let mut child = spawn_backend(backend)?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "backend stdin is unavailable".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "backend stdout is unavailable".to_string())?;
    write_backend_message(&mut stdin, &first_bytes).await?;
    let mut stdout = BufReader::new(stdout);
    let backend_hello = match read_and_validate_backend_hello(
        &mut stdout,
        requested_minecraft_version,
        config.max_message_bytes,
    )
    .await
    {
        Ok(hello) => hello,
        Err(reason) => {
            let _ = child.kill().await;
            return send_disconnect(&mut websocket, &reason).await;
        }
    };
    websocket
        .send(Message::Binary(backend_hello.into()))
        .await
        .map_err(|error| format!("websocket write backend hello: {error}"))?;
    bridge(
        &mut websocket,
        &mut stdin,
        stdout,
        &mut child,
        config.max_message_bytes,
        config.fixed_server.as_deref(),
        config.idle_timeout,
    )
    .await
}
async fn read_and_validate_backend_hello<S>(
    stdout: &mut BufReader<S>,
    requested_minecraft_version: &str,
    max_message_bytes: usize,
) -> Result<Vec<u8>, String>
where
    S: AsyncRead + Unpin,
{
    let mut partial = Vec::new();
    let line = match read_capped_line(stdout, &mut partial, max_message_bytes).await? {
        CappedLine::Line(line) => line,
        CappedLine::End => {
            return Err("backend exited before sending its manifest".to_string());
        }
    };
    let message: BackendToClientMessage = decode_json_line(&line)
        .map_err(|error| format!("backend sent invalid hello JSON: {error}"))?;
    let BackendToClientMessage::Hello { manifest } = message else {
        return Err("backend sent a non-hello message before its manifest".to_string());
    };
    validate_backend_manifest(&manifest, requested_minecraft_version)?;
    encode_json_line(&BackendToClientMessage::Hello { manifest })
        .map_err(|error| format!("encode backend hello: {error}"))
}
fn validate_backend_manifest(
    manifest: &ProtocolBackendManifest,
    requested_minecraft_version: &str,
) -> Result<(), String> {
    if manifest.ipc_version != ELTHARN_PROTOCOL_BACKEND_IPC_VERSION {
        return Err(format!(
            "incompatible backend IPC version: backend={} gateway={}",
            manifest.ipc_version, ELTHARN_PROTOCOL_BACKEND_IPC_VERSION
        ));
    }
    if manifest.minecraft_version != requested_minecraft_version {
        return Err(format!(
            "incompatible backend Minecraft version: backend={} requested={requested_minecraft_version}",
            manifest.minecraft_version
        ));
    }
    if requested_minecraft_version == "1.21.11" {
        if manifest.protocol_version != VANILLA_PROTOCOL_VERSION {
            return Err(format!(
                "incompatible 1.21.11 protocol id: backend={} expected={}",
                manifest.protocol_version, VANILLA_PROTOCOL_VERSION
            ));
        }
        if manifest.backend_name != EXPECTED_BACKEND_1_21_11_NAME {
            return Err(format!(
                "incompatible 1.21.11 backend identity: backend={} expected={EXPECTED_BACKEND_1_21_11_NAME}",
                manifest.backend_name
            ));
        }
    }
    Ok(())
}
fn spawn_backend(path: &str) -> Result<Child, String> {
    let mut candidates = vec![path.to_string()];
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(path).to_string_lossy().to_string());
        }
    }
    candidates.push(format!("./target/release/{path}"));
    #[cfg(debug_assertions)]
    candidates.push(format!("./target/debug/{path}"));
    candidates.push(format!("../target/release/{path}"));
    candidates.push(format!("../../target/release/{path}"));
    let mut last_err = String::new();
    for candidate in &candidates {
        match Command::new(candidate)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
        {
            Ok(child) => {
                eprintln!("gateway spawned backend: {candidate}");
                return Ok(child);
            }
            Err(e) => {
                last_err = format!("{candidate}: {e}");
            }
        }
    }
    Err(format!("start backend {path:?}: {last_err}"))
}
async fn bridge<S>(
    websocket: &mut tokio_tungstenite::WebSocketStream<TcpStream>,
    stdin: &mut ChildStdin,
    stdout: S,
    child: &mut Child,
    max_message_bytes: usize,
    fixed_server: Option<&str>,
    idle_timeout: Duration,
) -> Result<(), String>
where
    S: AsyncRead + Unpin,
{
    let mut stdout = BufReader::new(stdout);
    let mut backend_output = Vec::new();
    loop {
        let idle = tokio::time::sleep(idle_timeout);
        tokio::pin!(idle);
        tokio::select! {
            () = & mut idle => {
            eprintln!("gateway connection idle for {idle_timeout:?}; closing"); let _ =
            child.kill(). await; let _ = websocket.close(None). await; return Ok(()); }
            websocket_message = websocket.next() => { let Some(websocket_message) =
            websocket_message else { break; }; let message = websocket_message.map_err(|
            error | format!("websocket read: {error}")) ?; match message {
            Message::Text(text) => { let bytes = text.as_bytes(); if bytes.len() >
            max_message_bytes { return send_disconnect(websocket,
            "message exceeds gateway limit"). await; } let bytes =
            normalize_client_message(bytes, max_message_bytes, fixed_server) ?;
            write_backend_message(stdin, & bytes). await ?; } Message::Binary(bytes) => {
            if bytes.len() > max_message_bytes { return send_disconnect(websocket,
            "message exceeds gateway limit"). await; } let bytes =
            normalize_client_message(& bytes, max_message_bytes, fixed_server) ?;
            write_backend_message(stdin, & bytes). await ?; } Message::Ping(payload) =>
            websocket.send(Message::Pong(payload)). await .map_err(| error |
            format!("websocket pong: {error}")) ?, Message::Close(_) => break,
            Message::Pong(_) => {} Message::Frame(_) => {} } } backend_line =
            read_capped_line(& mut stdout, & mut backend_output, max_message_bytes) => {
            let line = match backend_line { Ok(CappedLine::Line(line)) => line,
            Ok(CappedLine::End) => break, Err(error) => {
            eprintln!("gateway killed backend after stdout error: {error}"); let _ =
            child.kill(). await; let _ = websocket.close(None). await; return Err(error);
            } }; let backend_disconnected = matches!(decode_json_line::<
            BackendToClientMessage > (& line), Ok(BackendToClientMessage::Disconnected {
            .. }) |
            Ok(BackendToClientMessage::Event(el_protocol_backend::BackendNetworkEvent::Disconnected
            { .. },))); let mut bytes = line; bytes.push(b'\n'); websocket
            .send(Message::Binary(bytes.into())). await .map_err(| error |
            format!("websocket write: {error}")) ?; if backend_disconnected { let _ =
            child.kill(). await; let _ = websocket.close(None). await; return Ok(()); } }
        }
    }
    let _ = child.kill().await;
    Ok(())
}
enum CappedLine {
    Line(Vec<u8>),
    End,
}
async fn read_capped_line<S>(
    stdout: &mut BufReader<S>,
    line: &mut Vec<u8>,
    cap: usize,
) -> Result<CappedLine, String>
where
    S: AsyncRead + Unpin,
{
    loop {
        let available = stdout
            .fill_buf()
            .await
            .map_err(|error| format!("backend stdout: {error}"))?;
        if available.is_empty() {
            return if line.is_empty() {
                Ok(CappedLine::End)
            } else {
                Ok(CappedLine::Line(std::mem::take(line)))
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(available.len());
        if line.len() + take > cap {
            return Err(format!("backend line exceeds {cap} bytes"));
        }
        line.extend_from_slice(&available[..take]);
        let consumed = take + usize::from(newline.is_some());
        stdout.consume(consumed);
        if newline.is_some() {
            return Ok(CappedLine::Line(std::mem::take(line)));
        }
    }
}
fn message_bytes(message: Message) -> Result<Vec<u8>, String> {
    match message {
        Message::Text(text) => Ok(text.into_bytes()),
        Message::Binary(bytes) => Ok(bytes.to_vec()),
        Message::Close(_) => Err("client closed before hello".to_string()),
        Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {
            Err("first websocket message must contain hello JSON".to_string())
        }
    }
}
fn normalize_client_message(
    bytes: &[u8],
    max_message_bytes: usize,
    fixed_server: Option<&str>,
) -> Result<Vec<u8>, String> {
    if bytes.len() > max_message_bytes {
        return Err("message exceeds gateway limit".to_string());
    }
    let message: ClientToBackendMessage =
        decode_json_line(bytes).map_err(|error| format!("invalid client message: {error}"))?;
    if let ClientToBackendMessage::Connect(mut args) = message {
        if args.username.trim().is_empty()
            || args.username.chars().any(char::is_control)
            || args.username.chars().count() > 16
        {
            return Err("invalid username".to_string());
        }
        let requested_server = args.server.trim();
        if requested_server.is_empty() {
            return Err("Minecraft server address is required".to_string());
        }
        if let Some(expected) = fixed_server {
            if requested_server != expected {
                return Err("gateway only permits its configured Minecraft server".to_string());
            }
            args.server = expected.to_string();
        } else {
            args.server = requested_server.to_string();
        }
        return encode_json_line(&ClientToBackendMessage::Connect(args))
            .map_err(|error| format!("encode normalized client message: {error}"));
    }
    Ok(bytes.to_vec())
}
async fn write_backend_message(stdin: &mut ChildStdin, bytes: &[u8]) -> Result<(), String> {
    stdin
        .write_all(bytes)
        .await
        .map_err(|error| format!("backend stdin: {error}"))?;
    if !bytes.ends_with(b"\n") {
        stdin
            .write_all(b"\n")
            .await
            .map_err(|error| format!("backend stdin newline: {error}"))?;
    }
    stdin
        .flush()
        .await
        .map_err(|error| format!("backend stdin flush: {error}"))
}
async fn send_disconnect(
    websocket: &mut tokio_tungstenite::WebSocketStream<TcpStream>,
    reason: &str,
) -> Result<(), String> {
    let message = BackendToClientMessage::Disconnected {
        reason: reason.to_string(),
    };
    let bytes = encode_json_line(&message).map_err(|error| error.to_string())?;
    websocket
        .send(Message::Binary(bytes.into()))
        .await
        .map_err(|error| format!("websocket write: {error}"))?;
    websocket
        .close(None)
        .await
        .map_err(|error| error.to_string())
}
