use el_platform_api::{MemoryResourceStore, ResourceSnapshotRequest, ResourceStore, VirtualPath};
use el_shaderpack::model::ValidatedPack;
use serde_json::{Value, json};
use std::cell::RefCell;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod native_control;
#[cfg(not(target_arch = "wasm32"))]
pub use native_control::configure_native_probe;
mod observer;
pub(crate) mod v4_resize;
pub enum PendingPack {
    Released(ValidatedPack),
    V4 {
        pack: el_shaderpack::graph_v4::ValidatedPackV4,
        id: String,
    },
}
pub struct Pending {
    pub mode: String,
    pub pack: Option<PendingPack>,
    pub budget: u64,
}
pub const V4_DIAGNOSTIC_VARIANT: &str = "v4_post_only";
#[derive(Default)]
struct Control {
    configured: bool,
    started: bool,
    pending: Option<Pending>,
    server: Option<String>,
    disable: bool,
}
thread_local! {
    static CONTROL : RefCell < Control > = RefCell::new(Control::default());
}
fn install_prepared(mode: &str, pack: Option<PendingPack>, budget: u64) -> Result<(), String> {
    CONTROL.with(|cell| {
        let mut c = cell.borrow_mut();
        if c.configured || c.started {
            return Err("Shader selection already consumed".into());
        }
        c.pending = Some(Pending {
            mode: mode.into(),
            pack,
            budget,
        });
        c.configured = true;
        Ok(())
    })
}
pub fn configure_prepared(pack: Option<ValidatedPack>, enabled: bool) -> Result<(), String> {
    if pack.is_some() && !enabled {
        return Err("Shader pack supplied while disabled".into());
    }
    let mode = if pack.is_some() {
        "pack"
    } else if enabled {
        "empty"
    } else {
        "off"
    };
    install_prepared(mode, pack.map(PendingPack::Released), 64 * 1024 * 1024)
}
pub fn configure_prepared_v4(
    pack: el_shaderpack::graph_v4::ValidatedPackV4,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    if id.is_empty() {
        return Err("v4 prepared pack id missing".into());
    }
    if !enabled {
        return Err("Shader pack supplied while disabled".into());
    }
    let budget = if matches!(
        pack.profile.as_str(),
        el_shaderpack::graph_v4::PACK_PROFILE_FRAME
            | el_shaderpack::graph_v4::PACK_PROFILE_TEMPORAL
            | el_shaderpack::graph_v4::PACK_PROFILE_LIGHTING
    ) {
        256 * 1024 * 1024
    } else {
        64 * 1024 * 1024
    };
    install_prepared("pack", Some(PendingPack::V4 { pack, id }), budget)
}
fn server(value: Option<&Value>) -> Result<Option<String>, String> {
    let Some(v) = value else {
        return Ok(None);
    };
    let s = v.as_str().ok_or("server_address must be a string")?;
    let (host, port) = s.split_once(':').ok_or("loopback host:port required")?;
    if !matches!(host, "127.0.0.1" | "localhost")
        || port.is_empty()
        || !port.bytes().all(|b| b.is_ascii_digit())
        || port.parse::<u16>().ok().is_none_or(|p| p == 0)
    {
        return Err("diagnostic server must be loopback with a valid port".into());
    }
    Ok(Some(s.into()))
}
pub async fn configure_packet(bytes: &[u8]) -> Result<(), String> {
    if CONTROL.with(|c| {
        let c = c.borrow();
        c.configured || c.started
    }) {
        return Err("shader configuration already consumed".into());
    }
    let p = el_shaderpack::package::settings(bytes)?;
    let o = p.as_object().ok_or("packet object")?;
    let required = ["mode", "budget_bytes", "files", "receipt", "settings"];
    if !required.iter().all(|k| o.contains_key(*k))
        || o.keys()
            .any(|k| !required.contains(&k.as_str()) && k != "server_address")
    {
        return Err("shader packet fields".into());
    }
    let mode = p["mode"].as_str().ok_or("mode string")?;
    if !matches!(mode, "off" | "empty" | "pack") {
        return Err("unknown shader mode".into());
    }
    let budget = p["budget_bytes"]
        .as_u64()
        .filter(|n| (1..=536870912).contains(n))
        .ok_or("shader budget")?;
    let files = p["files"].as_object().ok_or("files object")?;
    let settings = p["settings"].as_object().ok_or("settings object")?;
    let address = server(p.get("server_address"))?;
    if files.len() > 128 {
        return Err("diagnostic file count".into());
    }
    let pack = if mode == "pack" {
        let store = MemoryResourceStore::new();
        for (path, bytes) in files {
            if path != el_extension_api::package_path::MANIFEST_FILE_NAME
                && !path.starts_with("shaders/")
            {
                return Err("diagnostic package namespace".into());
            }
            store.insert(
                VirtualPath::new(path).map_err(|e| e.to_string())?,
                bytes
                    .as_str()
                    .ok_or("UTF-8 shader payload expected")?
                    .as_bytes(),
            );
        }
        let snapshot = store
            .prepare_snapshot(ResourceSnapshotRequest::all())
            .await
            .map_err(|e| e.to_string())?;
        let manifest_api = snapshot
            .read_view(
                &VirtualPath::new(el_extension_api::package_path::MANIFEST_FILE_NAME)
                    .map_err(|e| e.to_string())?,
            )
            .ok()
            .flatten()
            .and_then(|bytes| serde_json::from_slice::<Value>(bytes.as_ref()).ok())
            .and_then(|manifest| {
                manifest
                    .get("shader_api")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
            });
        if manifest_api.as_deref() == Some(el_shaderpack::graph_v4::SHADER_API_V4) {
            let receipt_bytes = serde_json::to_vec(&p["receipt"]).map_err(|e| e.to_string())?;
            let pack = el_shaderpack::graph_v4::load_v4_with_settings(
                &*snapshot,
                &receipt_bytes,
                &p["settings"],
            )?;
            let id = p["receipt"]
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or("v4 receipt id")?;
            Some(PendingPack::V4 {
                pack,
                id: id.to_owned(),
            })
        } else {
            Some(PendingPack::Released(el_shaderpack::package::load(
                &*snapshot,
                &serde_json::to_vec(&p["receipt"]).map_err(|e| e.to_string())?,
                &p["settings"],
            )?))
        }
    } else {
        if !files.is_empty() || !p["receipt"].is_null() || !settings.is_empty() {
            return Err("off/empty cannot contain package data".into());
        }
        None
    };
    CONTROL.with(|c| -> Result<(), String> {
        let mut c = c.borrow_mut();
        if c.configured || c.started {
            return Err("shader configuration already consumed".into());
        }
        c.configured = true;
        c.pending = Some(Pending {
            mode: mode.into(),
            pack,
            budget,
        });
        c.server = address;
        Ok(())
    })?;
    observer::configure();
    Ok(())
}
pub fn take_pending() -> Option<Pending> {
    CONTROL.with(|c| {
        let mut c = c.borrow_mut();
        c.started = true;
        c.pending.take()
    })
}
pub fn take_quick_play() -> Option<String> {
    CONTROL.with(|c| c.borrow_mut().server.take())
}
pub fn request_disable() {
    CONTROL.with(|c| c.borrow_mut().disable = true);
}
pub fn take_disable() -> bool {
    CONTROL.with(|c| std::mem::take(&mut c.borrow_mut().disable))
}
pub fn active_pack_fault() -> bool {
    let current = report();
    current.get("mode").and_then(|v| v.as_str()) == Some("pack")
        && matches!(
            current.get("status").and_then(|v| v.as_str()),
            Some("failed" | "rejected")
        )
        && current
            .get("error")
            .and_then(|v| v.as_str())
            .is_some_and(|text| !text.is_empty())
}
pub fn publish(mode: &str, status: &str, gpu: Value, error: Option<String>) {
    observer::publish(mode, status, gpu, error);
}
pub fn record_present(in_world: bool, frame_us: u64, shader_us: u64, shader_frames: u64) {
    observer::present(in_world, frame_us, shader_us, shader_frames);
}
pub fn report() -> Value {
    observer::report()
}
pub fn is_configured() -> bool {
    observer::active()
}
pub fn zero_stats() -> Value {
    json!(
        { "shader_programs" : 0, "internal_programs" : 0, "pipelines" : 0, "textures" :
        0, "samplers" : 0, "buffers" : 0, "bindgroups" : 0, "logical_bytes" : 0 }
    )
}
