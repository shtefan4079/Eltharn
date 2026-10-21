use crate::resolver::{Conflict, Dependency, ResolvePackage};
use el_extension_api::services::{
    BINARY_BUFFERS_CAPABILITY, CLIENT_ENTITIES_READ_CAPABILITY, CLIENT_HOOK_STATE_WRITE_CAPABILITY,
    CLIENT_KEYBINDINGS_CAPABILITY, CLIENT_OVERLAY_WRITE_CAPABILITY,
};
use el_extension_api::{CapabilityName, ExtensionId, PackagePath};
use el_launcher_core::extensions::PackagePin;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug)]
pub struct ModuleMetadata {
    pub path: String,
    pub language: ModuleLanguage,
    pub hooks: Option<el_extension_api::hooks::HookModule>,
    pub exports: BTreeSet<String>,
    pub required: Vec<CapabilityName>,
    pub optional: Vec<CapabilityName>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleLanguage {
    Wasm,
    Rhai,
}
#[derive(Clone, Debug)]
pub struct Assets {
    pub root: String,
    pub resources: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct ModMetadata {
    pub modules: BTreeMap<String, ModuleMetadata>,
    pub dependencies: Vec<Dependency>,
    pub conflicts: Vec<Conflict>,
    pub assets: Option<Assets>,
    pub data_schemas: BTreeMap<String, u16>,
}
impl ModMetadata {
    pub fn resolve_package(&self, pin: PackagePin) -> ResolvePackage {
        ResolvePackage {
            pin,
            modules: self.modules.keys().cloned().collect(),
            dependencies: self.dependencies.clone(),
            conflicts: self.conflicts.clone(),
        }
    }
}
const SLOTS: [&str; 3] = ["client", "server", "shared"];
const LIFECYCLE: [&str; 6] = [
    "init",
    "tick",
    "shutdown",
    "on_world_enter",
    "on_world_exit",
    "app_tick",
];
const REQUIRED_FNS: [&str; 3] = ["init", "tick", "shutdown"];
const EXTENSION_API_FIELD: &str = "extension_api";
const MIN_API_KEYBINDINGS: &str = "1.12";
const MIN_API_OVERLAY: &str = "1.13";
const MIN_API_BINARY_BUFFERS: &str = "1.5";
const MIN_API_HOOK_STATE: &str = "1.5";
const MIN_API_ENTITIES: &str = "1.6";
const MIN_API_HOOKS: &str = "1.4";
const MIN_API_ITEM_TOOLTIP: &str = "1.7";
const MIN_API_ITEM_TOOLTIP_LAYOUT: &str = "1.9";
const MIN_API_HOOK_POINTS: &str = "1.5";
fn e(s: impl Into<String>) -> String {
    s.into().chars().take(256).collect()
}
fn q(s: &str) -> String {
    format!("{:?}", s.chars().take(64).collect::<String>())
}
fn must_typed<T>(s: &str, ctx: &str) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value::<T>(Value::String(s.to_owned()))
        .map_err(|er| e(format!("{ctx}: invalid {}: {er}", q(s))))
}
fn collect_files(root: &serde_json::Map<String, Value>) -> Result<BTreeSet<String>, String> {
    let arr = match root.get("files") {
        Some(Value::Array(a)) => a,
        _ => return Err(e("files: missing array of {path,size,sha256}")),
    };
    if arr.is_empty() || arr.len() > 4096 {
        return Err(e("files: need 1..=4096 entries"));
    }
    let mut set = BTreeSet::new();
    for en in arr {
        let o = en
            .as_object()
            .ok_or_else(|| e("files entry: not an object"))?;
        let p = match o.get("path") {
            Some(Value::String(s)) => s.clone(),
            _ => return Err(e("files entry.path: missing string (null rejected)")),
        };
        let _: PackagePath = must_typed(&p, "files entry.path")?;
        if p == el_extension_api::package_path::MANIFEST_FILE_NAME {
            return Err(e("files: root manifest must not be declared"));
        }
        if !set.insert(p.clone()) {
            return Err(e(format!("files: duplicate path {}", q(&p))));
        }
    }
    Ok(set)
}
fn parse_cap_list(
    v: Option<&Value>,
    slot: &str,
    kind: &str,
) -> Result<Vec<CapabilityName>, String> {
    match v {
        None => Ok(Vec::new()),
        Some(Value::Null) => Err(e(format!("capabilities.{slot}.{kind}: null rejected"))),
        Some(Value::Array(a)) => {
            if a.len() > 64 {
                return Err(e(format!("capabilities.{slot}.{kind}: >64 names")));
            }
            let mut seen = BTreeSet::new();
            let mut out = Vec::new();
            for item in a {
                let s = match item {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(e(format!("capabilities.{slot}.{kind}: not a string")));
                    }
                };
                if !seen.insert(s.clone()) {
                    return Err(e(format!(
                        "capabilities.{slot}.{kind}: duplicate {}",
                        q(&s)
                    )));
                }
                let c: CapabilityName = must_typed(&s, &format!("capabilities.{slot}.{kind}"))?;
                out.push(c);
            }
            Ok(out)
        }
        Some(_) => Err(e(format!("capabilities.{slot}.{kind}: not an array"))),
    }
}
fn raw_id(v: &Value) -> String {
    v.get("id")
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_owned()
}
fn parse_deps(
    root: &serde_json::Map<String, Value>,
    self_id: &str,
) -> Result<Vec<Dependency>, String> {
    match root.get("dependencies") {
        None => Ok(Vec::new()),
        Some(Value::Null) => Err(e("dependencies: null rejected, omit field")),
        Some(Value::Array(a)) => {
            if a.len() > 256 {
                return Err(e("dependencies: >256 entries"));
            }
            let mut seen = BTreeSet::new();
            let mut out = Vec::new();
            for item in a {
                let d: Dependency = serde_json::from_value(item.clone())
                    .map_err(|er| e(format!("dependencies entry: {er}")))?;
                let id = raw_id(item);
                if id.is_empty() {
                    return Err(e("dependencies entry.id: missing string"));
                }
                if id == self_id {
                    return Err(e("dependencies: self id forbidden"));
                }
                if !seen.insert(id.clone()) {
                    return Err(e(format!("dependencies: duplicate id {}", q(&id))));
                }
                let _ = d;
                out.push(d);
            }
            Ok(out)
        }
        Some(_) => Err(e("dependencies: not an array")),
    }
}
fn valid_reason(s: &str) -> bool {
    let n = s.chars().count();
    (1..=256).contains(&n) && !s.chars().any(|c| c.is_control())
}
fn parse_confs(
    root: &serde_json::Map<String, Value>,
    self_id: &str,
) -> Result<Vec<Conflict>, String> {
    match root.get("conflicts") {
        None => Ok(Vec::new()),
        Some(Value::Null) => Err(e("conflicts: null rejected, omit field")),
        Some(Value::Array(a)) => {
            if a.len() > 256 {
                return Err(e("conflicts: >256 entries"));
            }
            let mut seen = BTreeSet::new();
            let mut out = Vec::new();
            for item in a {
                let c: Conflict = serde_json::from_value(item.clone())
                    .map_err(|er| e(format!("conflicts entry: {er}")))?;
                let id = raw_id(item);
                if id.is_empty() {
                    return Err(e("conflicts entry.id: missing string"));
                }
                if id == self_id {
                    return Err(e("conflicts: self id forbidden"));
                }
                if !seen.insert(id.clone()) {
                    return Err(e(format!("conflicts: duplicate id {}", q(&id))));
                }
                let rs = item
                    .get("reason")
                    .and_then(|x| x.as_str())
                    .ok_or_else(|| e("conflicts entry.reason: missing string"))?;
                if !valid_reason(rs) {
                    return Err(e(
                        "conflicts entry.reason: need 1..=256 scalars, no controls",
                    ));
                }
                let _ = c;
                out.push(c);
            }
            Ok(out)
        }
        Some(_) => Err(e("conflicts: not an array")),
    }
}
fn parse_assets(
    root: &serde_json::Map<String, Value>,
    files: &BTreeSet<String>,
    wasm_map: &BTreeMap<String, String>,
) -> Result<Option<Assets>, String> {
    match root.get("assets") {
        None => Ok(None),
        Some(Value::Null) => Err(e("assets: null rejected, omit field")),
        Some(v) => {
            let o = v.as_object().ok_or_else(|| e("assets: not an object"))?;
            if o.len() != 2 || !o.contains_key("root") || !o.contains_key("resources") {
                return Err(e("assets: object must be exactly {root, resources}"));
            }
            let r = match o.get("root") {
                Some(Value::String(s)) => s.clone(),
                _ => return Err(e("assets.root: missing string")),
            };
            if !r.ends_with('/') {
                return Err(e("assets.root: must end with '/'"));
            }
            let pre = &r[..r.len() - 1];
            if pre.is_empty() {
                return Err(e("assets.root: empty prefix"));
            }
            let _: PackagePath = must_typed(pre, "assets.root prefix")?;
            let arr = o
                .get("resources")
                .and_then(|x| x.as_array())
                .ok_or_else(|| e("assets.resources: missing array (null rejected)"))?;
            if arr.len() > 4096 {
                return Err(e("assets.resources: >4096 entries"));
            }
            let mut set = BTreeSet::new();
            let mut vec = Vec::new();
            for item in arr {
                let s = match item {
                    Value::String(x) => x.clone(),
                    _ => return Err(e("assets.resources: not a string")),
                };
                let _: PackagePath = must_typed(&s, "assets.resources entry")?;
                if !set.insert(s.clone()) {
                    return Err(e(format!("assets.resources: duplicate {}", q(&s))));
                }
                if !files.contains(&s) {
                    return Err(e(format!("assets.resources: not in files {}", q(&s))));
                }
                if !s.starts_with(&r) {
                    return Err(e(format!("assets.resources: not under root {}", q(&s))));
                }
                vec.push(s);
            }
            let under: BTreeSet<String> = files
                .iter()
                .filter(|p| p.starts_with(&r))
                .cloned()
                .collect();
            if set != under {
                return Err(e(
                    "assets.resources: must equal exactly all files under root",
                ));
            }
            for (slot, w) in wasm_map {
                if w.starts_with(&r) {
                    return Err(e(format!("assets: module {slot} under assets root")));
                }
            }
            Ok(Some(Assets {
                root: r,
                resources: vec,
            }))
        }
    }
}
fn parse_schemas(
    root: &serde_json::Map<String, Value>,
    mobj: &serde_json::Map<String, Value>,
) -> Result<BTreeMap<String, u16>, String> {
    match root.get("data_schemas") {
        None => Ok(BTreeMap::new()),
        Some(Value::Null) => Err(e("data_schemas: null rejected, omit field")),
        Some(v) => {
            let o = v
                .as_object()
                .ok_or_else(|| e("data_schemas: not an object"))?;
            let mut out = BTreeMap::new();
            for (k, val) in o {
                if !mobj.contains_key(k) {
                    return Err(e(format!("data_schemas: unknown slot {}", q(k))));
                }
                let n = val
                    .as_u64()
                    .ok_or_else(|| e(format!("data_schemas.{k}: need integer token 1..=9999")))?;
                if !(1..=9999).contains(&n) {
                    return Err(e(format!("data_schemas.{k}: out of 1..=9999")));
                }
                out.insert(k.clone(), n as u16);
            }
            Ok(out)
        }
    }
}
fn api_supports(api: el_extension_api::ApiVersion, gate: &str) -> bool {
    gate.parse::<el_extension_api::ApiVersion>()
        .is_ok_and(|required| api.supports(&required))
}
pub fn parse(manifest: &Value) -> Result<ModMetadata, String> {
    let root = manifest
        .as_object()
        .ok_or_else(|| e("manifest: root must be object"))?;
    match root.get("kind") {
        Some(Value::String(s)) if s == "mod" => {}
        _ => return Err(e("kind: must be string `mod`")),
    }
    let id_str = match root.get("id") {
        Some(Value::String(s)) => s.clone(),
        _ => return Err(e("id: missing string ExtensionId")),
    };
    let id: ExtensionId = must_typed(&id_str, "id")?;
    if id.is_reserved_namespace() {
        return Err(e("id: namespace minecraft/eltharn reserved"));
    }
    let files = collect_files(root)?;
    let mval = root
        .get("modules")
        .ok_or_else(|| e("modules: missing object"))?;
    let mobj = mval
        .as_object()
        .ok_or_else(|| e("modules: not an object (null rejected)"))?;
    if mobj.is_empty() {
        return Err(e(
            "modules: nonempty subset of client/server/shared required",
        ));
    }
    for k in mobj.keys() {
        if !SLOTS.contains(&k.as_str()) {
            return Err(e(format!("modules: unknown slot {}", q(k))));
        }
    }
    let mut wasm_map: BTreeMap<String, String> = BTreeMap::new();
    let is_source = root.get("manifest_version").and_then(Value::as_u64) == Some(2);
    let entry_field = if is_source { "script" } else { "wasm" };
    let suffix = if is_source { ".rhai" } else { ".wasm" };
    let mut exp_map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (slot, v) in mobj {
        let o = v
            .as_object()
            .ok_or_else(|| e(format!("modules.{slot}: not an object")))?;
        for k in o.keys() {
            if k != entry_field && k != "exports" && k != "hooks" {
                return Err(e(format!("modules.{slot}: unknown field {}", q(k))));
            }
        }
        let w = match o.get(entry_field) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err(e(format!("modules.{slot}.{entry_field}: missing string"))),
        };
        let _: PackagePath = must_typed(&w, &format!("modules.{slot}.{entry_field}"))?;
        if !w.ends_with(suffix) {
            return Err(e(format!(
                "modules.{slot}.{entry_field}: need {suffix} suffix"
            )));
        }
        if !files.contains(&w) {
            return Err(e(format!(
                "modules.{slot}.{entry_field}: not in files {}",
                q(&w)
            )));
        }
        let arr = o
            .get("exports")
            .and_then(|x| x.as_array())
            .ok_or_else(|| e(format!("modules.{slot}.exports: missing array")))?;
        let mut set = BTreeSet::new();
        for item in arr {
            let s = match item {
                Value::String(x) => x.clone(),
                _ => return Err(e(format!("modules.{slot}.exports: not a string"))),
            };
            if !LIFECYCLE.contains(&s.as_str()) || (!is_source && s == "app_tick") {
                return Err(e(format!("modules.{slot}.exports: unknown {}", q(&s))));
            }
            if !set.insert(s.clone()) {
                return Err(e(format!("modules.{slot}.exports: duplicate {}", q(&s))));
            }
        }
        for r in REQUIRED_FNS {
            if !set.contains(r) {
                return Err(e(format!("modules.{slot}.exports: missing required {r}")));
            }
        }
        wasm_map.insert(slot.clone(), w);
        exp_map.insert(slot.clone(), set);
    }
    let mut caps: BTreeMap<String, (Vec<CapabilityName>, Vec<CapabilityName>)> = BTreeMap::new();
    match root.get("capabilities") {
        None => {}
        Some(Value::Null) => return Err(e("capabilities: null rejected, omit field")),
        Some(v) => {
            let o = v
                .as_object()
                .ok_or_else(|| e("capabilities: not an object"))?;
            for (slot, sv) in o {
                if !mobj.contains_key(slot) {
                    return Err(e(format!("capabilities: unknown slot {}", q(slot))));
                }
                let so = sv
                    .as_object()
                    .ok_or_else(|| e(format!("capabilities.{slot}: not an object")))?;
                for k in so.keys() {
                    if k != "required" && k != "optional" {
                        return Err(e(format!("capabilities.{slot}: unknown field {}", q(k))));
                    }
                }
                let req = parse_cap_list(so.get("required"), slot, "required")?;
                let opt = parse_cap_list(so.get("optional"), slot, "optional")?;
                if req.iter().any(|c| opt.contains(c)) {
                    return Err(e(format!("capabilities.{slot}: required/optional overlap")));
                }
                caps.insert(slot.clone(), (req, opt));
            }
        }
    }
    let deps = parse_deps(root, &id_str)?;
    let confs = parse_confs(root, &id_str)?;
    let assets = parse_assets(root, &files, &wasm_map)?;
    let schemas = parse_schemas(root, mobj)?;
    let mut modules = BTreeMap::new();
    for slot in mobj.keys() {
        let (r, o) = caps.remove(slot).unwrap_or((Vec::new(), Vec::new()));
        let hooks = parse_hooks(&mobj[slot], slot, is_source, &files)?;
        if r.iter()
            .chain(&o)
            .any(|cap| cap.as_str() == CLIENT_KEYBINDINGS_CAPABILITY)
        {
            let api: el_extension_api::ApiVersion = root
                .get(EXTENSION_API_FIELD)
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    e(format!(
                        "keybindings require extension_api {MIN_API_KEYBINDINGS}"
                    ))
                })?
                .parse()
                .map_err(|error: el_extension_api::Error| e(error.to_string()))?;
            if !is_source || slot != "client" || !api_supports(api, MIN_API_KEYBINDINGS) {
                return Err(e(format!(
                    "keybindings require an API {MIN_API_KEYBINDINGS} client source module"
                )));
            }
        }
        if r.iter()
            .chain(&o)
            .any(|cap| cap.as_str() == CLIENT_OVERLAY_WRITE_CAPABILITY)
        {
            let api: el_extension_api::ApiVersion = root
                .get(EXTENSION_API_FIELD)
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    e(format!(
                        "overlay messages require extension_api {MIN_API_OVERLAY}"
                    ))
                })?
                .parse()
                .map_err(|error: el_extension_api::Error| e(error.to_string()))?;
            if !is_source || slot != "client" || !api_supports(api, MIN_API_OVERLAY) {
                return Err(e(format!(
                    "overlay messages require an API {MIN_API_OVERLAY} client source module"
                )));
            }
        }
        if r.iter()
            .chain(&o)
            .any(|cap| cap.as_str() == BINARY_BUFFERS_CAPABILITY)
        {
            let api: el_extension_api::ApiVersion = root
                .get(EXTENSION_API_FIELD)
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    e(format!(
                        "binary buffers require extension_api {MIN_API_BINARY_BUFFERS}"
                    ))
                })?
                .parse()
                .map_err(|error: el_extension_api::Error| e(error.to_string()))?;
            if !is_source || !api_supports(api, MIN_API_BINARY_BUFFERS) {
                return Err(e(format!(
                    "binary buffers require an API {MIN_API_BINARY_BUFFERS} source module"
                )));
            }
        }
        if r.iter()
            .chain(&o)
            .any(|cap| cap.as_str() == CLIENT_HOOK_STATE_WRITE_CAPABILITY)
        {
            let api: el_extension_api::ApiVersion = root
                .get(EXTENSION_API_FIELD)
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    e(format!(
                        "hook state requires extension_api {MIN_API_HOOK_STATE}"
                    ))
                })?
                .parse()
                .map_err(|error: el_extension_api::Error| e(error.to_string()))?;
            if !is_source
                || slot != "client"
                || hooks.is_none()
                || !api_supports(api, MIN_API_HOOK_STATE)
            {
                return Err(e(format!(
                    "hook state requires an API {MIN_API_HOOK_STATE} client source hook module"
                )));
            }
        }
        if r.iter()
            .chain(&o)
            .any(|cap| cap.as_str() == CLIENT_ENTITIES_READ_CAPABILITY)
        {
            let api: el_extension_api::ApiVersion = root
                .get(EXTENSION_API_FIELD)
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    e(format!(
                        "entity observations require extension_api {MIN_API_ENTITIES}"
                    ))
                })?
                .parse()
                .map_err(|error: el_extension_api::Error| e(error.to_string()))?;
            if !is_source || slot != "client" || !api_supports(api, MIN_API_ENTITIES) {
                return Err(e(format!(
                    "entity observations require an API {MIN_API_ENTITIES} client source module"
                )));
            }
        }
        if let Some(hooks) = &hooks {
            let api: el_extension_api::ApiVersion = root
                .get(EXTENSION_API_FIELD)
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    e(format!(
                        "hooks require extension_api {MIN_API_HOOKS} or newer"
                    ))
                })?
                .parse()
                .map_err(|error: el_extension_api::Error| e(error.to_string()))?;
            if !api_supports(api, MIN_API_HOOKS) {
                return Err(e(format!(
                    "hooks require extension_api {MIN_API_HOOKS} or newer"
                )));
            }
            if !api_supports(api, MIN_API_ITEM_TOOLTIP)
                && hooks
                    .bindings
                    .iter()
                    .any(|binding| binding.point == el_extension_api::hooks::HookPoint::ItemTooltip)
            {
                return Err(e(format!(
                    "item tooltip hooks require extension_api {MIN_API_ITEM_TOOLTIP} or newer"
                )));
            }
            if !api_supports(api, MIN_API_ITEM_TOOLTIP_LAYOUT)
                && hooks.bindings.iter().any(|binding| {
                    binding.point == el_extension_api::hooks::HookPoint::ItemTooltipLayout
                })
            {
                return Err(e(format!(
                    "item tooltip layout hooks require extension_api \
                     {MIN_API_ITEM_TOOLTIP_LAYOUT} or newer"
                )));
            }
            if !api_supports(api, MIN_API_HOOK_POINTS)
                && hooks.bindings.iter().any(|binding| {
                    matches!(
                        binding.point,
                        el_extension_api::hooks::HookPoint::CameraHurt
                            | el_extension_api::hooks::HookPoint::HudHealthBlink
                    )
                })
            {
                return Err(e(format!(
                    "hurt-operation and health-blink hooks require extension_api \
                     {MIN_API_HOOK_POINTS} or newer",
                )));
            }
            for binding in &hooks.bindings {
                if !r.iter().any(|c| c.as_str() == binding.point.capability()) {
                    return Err(e(format!(
                        "modules.{slot}.hooks: {} must be a required capability",
                        binding.point.capability()
                    )));
                }
            }
        }
        modules.insert(
            slot.clone(),
            ModuleMetadata {
                path: wasm_map[slot].clone(),
                language: if is_source {
                    ModuleLanguage::Rhai
                } else {
                    ModuleLanguage::Wasm
                },
                hooks,
                exports: exp_map[slot].clone(),
                required: r,
                optional: o,
            },
        );
    }
    Ok(ModMetadata {
        modules,
        dependencies: deps,
        conflicts: confs,
        assets,
        data_schemas: schemas,
    })
}
pub(crate) fn parse_hooks(
    module: &Value,
    slot: &str,
    is_source: bool,
    files: &BTreeSet<String>,
) -> Result<Option<el_extension_api::hooks::HookModule>, String> {
    let Some(value) = module.get("hooks") else {
        return Ok(None);
    };
    if !is_source || slot != "client" {
        return Err(e("hooks require a manifest-version-2 client source module"));
    }
    let hooks: el_extension_api::hooks::HookModule = serde_json::from_value(value.clone())
        .map_err(|error| e(format!("modules.{slot}.hooks: {error}")))?;
    hooks.validate().map_err(e)?;
    if !files.contains(&hooks.script) {
        return Err(e("hook script is missing from the package inventory"));
    }
    if module.get("script").and_then(Value::as_str) == Some(hooks.script.as_str()) {
        return Err(e("hook script must be separate from the lifecycle script"));
    }
    Ok(Some(hooks))
}
