use serde::Deserialize;
use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};
const ENV_DIR: &str = "ELTHARN_SHADERPACK_PROBE_CONTROL_DIR";
const LIMIT: u64 = 4096;
const WINDOW_MS: u64 = 200;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Action {
    Report,
    Disable,
    Resize,
    Exit,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    seq: u64,
    action: Action,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
}
pub(crate) struct NativeControl {
    dir: PathBuf,
    last: u64,
    at: Option<Instant>,
}
thread_local! {
    static PENDING : RefCell < Option < NativeControl >> = RefCell::new(None);
}
fn name(a: Action) -> &'static str {
    match a {
        Action::Report => "report",
        Action::Disable => "disable",
        Action::Resize => "resize",
        Action::Exit => "exit",
    }
}
fn parse(bytes: &[u8], last: u64) -> Option<(u64, Action, Option<(u32, u32)>)> {
    let r = serde_json::from_slice::<Request>(bytes).ok()?;
    if !(1..=100_000).contains(&r.seq) || r.seq <= last {
        return None;
    }
    match r.action {
        Action::Resize => {
            let (w, h) = (r.width?, r.height?);
            if !(1..=4096).contains(&w) || !(1..=4096).contains(&h) {
                return None;
            }
            Some((r.seq, r.action, Some((w, h))))
        }
        _ => {
            if r.width.is_some() || r.height.is_some() {
                return None;
            }
            Some((r.seq, r.action, None))
        }
    }
}
pub fn configure_native_probe(bytes: &[u8], dir: PathBuf) -> Result<(), String> {
    let prepared = NativeControl::prepare(dir)?;
    pollster::block_on(super::configure_packet(bytes))?;
    PENDING.with(|p| *p.borrow_mut() = Some(prepared));
    Ok(())
}
impl NativeControl {
    fn prepare(dir: PathBuf) -> Result<Self, String> {
        if !dir.is_absolute() {
            return Err("probe control disabled: control dir not absolute".to_string());
        }
        let md = match std::fs::metadata(&dir) {
            Ok(m) => m,
            Err(_) => {
                return Err("probe control disabled: control dir missing".to_string());
            }
        };
        if !md.is_dir() {
            return Err("probe control disabled: control dir not a directory".to_string());
        }
        let mut entries = match std::fs::read_dir(&dir) {
            Ok(it) => it,
            Err(_) => {
                return Err("probe control disabled: control dir unreadable".to_string());
            }
        };
        if entries.next().is_some() {
            return Err("probe control disabled: control dir not empty".to_string());
        }
        Ok(Self {
            dir,
            last: 0,
            at: None,
        })
    }
    pub(crate) fn from_env() -> Option<Self> {
        let raw = match std::env::var(ENV_DIR) {
            Ok(v) => v,
            Err(_) => {
                return None;
            }
        };
        if !super::is_configured() {
            eprintln!("probe control disabled: shaderpacks not configured");
            return None;
        }
        match Self::prepare(PathBuf::from(&raw)) {
            Ok(c) => {
                eprintln!("[M03Control] ready");
                Some(c)
            }
            Err(msg) => {
                eprintln!("{msg}");
                None
            }
        }
    }
    pub(crate) fn for_launch() -> Option<Self> {
        if let Some(c) = PENDING.with(|p| p.borrow_mut().take()) {
            eprintln!("[M03Control] ready");
            return Some(c);
        }
        Self::from_env()
    }
    pub(crate) fn poll(&mut self, window: &winit::window::Window) -> bool {
        if let Some(t) = self.at {
            if t.elapsed() < Duration::from_millis(WINDOW_MS) {
                return false;
            }
        }
        self.at = Some(Instant::now());
        let mut buf = Vec::new();
        let f = match File::open(self.dir.join("request.json")) {
            Ok(f) => f,
            Err(_) => return false,
        };
        let n = match f.take(LIMIT + 1).read_to_end(&mut buf) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("probe control: request read failed: {e}");
                return true;
            }
        };
        if n == 0 || n as u64 > LIMIT {
            return false;
        }
        let (seq, action, dims) = match parse(&buf, self.last) {
            Some(v) => v,
            None => return false,
        };
        match action {
            Action::Report => {}
            Action::Disable => super::request_disable(),
            Action::Resize => {
                if let Some((w, h)) = dims {
                    let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(w, h));
                }
            }
            Action::Exit => {}
        }
        self.last = seq;
        let size = window.inner_size();
        let body = serde_json::json!(
            { "seq" : seq, "action" : name(action), "window" : [size.width, size.height],
            "report" : super::report(), }
        );
        let bytes = match serde_json::to_vec(&body) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("probe control: encode failed: {e}");
                return true;
            }
        };
        let path = self.dir.join(format!("response-{seq}.json"));
        let mut out = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("probe control: response create failed: {e}");
                return true;
            }
        };
        if out.write_all(&bytes).is_err() {
            eprintln!("probe control: response write failed");
            return true;
        }
        matches!(action, Action::Exit)
    }
}
