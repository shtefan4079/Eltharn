use crate::package::Package;
use el_extension_api::ExtensionId;
use el_mod_runtime::wire::strict_json;
use el_platform_api::VirtualPath;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
const MAX_PATH: usize = 255;
const MAX_ASSET: usize = 8 * 1024 * 1024;
const MAX_FILES: usize = 4096;
const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_JSON: usize = 256 * 1024;
#[derive(Clone, Debug)]
pub struct OwnedAssets {
    pub owner: ExtensionId,
    pub files: BTreeMap<String, Vec<u8>>,
}
pub fn collect(
    packages: &[Package],
    owners: &BTreeSet<ExtensionId>,
) -> Result<Vec<OwnedAssets>, String> {
    let mut selected: Vec<&Package> = Vec::new();
    for owner in owners.iter() {
        let mut found: Option<&Package> = None;
        for p in packages {
            if p.info.pin.id == *owner {
                if found.is_some() {
                    return Err(owner_err("duplicate owner", owner));
                }
                found = Some(p);
            }
        }
        match found {
            Some(p) => selected.push(p),
            None => return Err(owner_err("missing owner", owner)),
        }
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut total_files = 0usize;
    let mut total_bytes = 0usize;
    for pkg in selected.iter() {
        for (path, bytes) in pkg.files.iter().filter(|(k, _)| k.starts_with("assets/")) {
            if !seen.insert(path.as_str()) {
                return Err(emsg(path, "duplicate asset path"));
            }
            total_files += 1;
            total_bytes += bytes.len();
            if total_files > MAX_FILES {
                return Err("too many assets".to_string());
            }
            if total_bytes > MAX_BYTES {
                return Err("assets too large".to_string());
            }
            validate_asset(&pkg.info.pin.id, path, bytes)?;
        }
    }
    let mut out = Vec::new();
    for pkg in selected {
        let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for (k, v) in pkg.files.iter().filter(|(k, _)| k.starts_with("assets/")) {
            files.insert(k.clone(), v.clone());
        }
        if !files.is_empty() {
            out.push(OwnedAssets {
                owner: pkg.info.pin.id.clone(),
                files,
            });
        }
    }
    Ok(out)
}
pub fn validate_asset(owner: &ExtensionId, path: &str, bytes: &[u8]) -> Result<(), String> {
    validate_for_namespace(owner.namespace(), path, bytes)
}
pub(crate) fn validate_for_namespace(ns: &str, path: &str, bytes: &[u8]) -> Result<(), String> {
    if path.is_empty() || path.len() > MAX_PATH {
        return Err(emsg(path, "bad path length"));
    }
    if el_extension_api::extension_id::RESERVED_NAMESPACES.contains(&ns) {
        return Err(emsg(path, "reserved namespace"));
    }
    let expected = format!("assets/{ns}/");
    let rest = path
        .strip_prefix(&expected)
        .ok_or_else(|| emsg(path, "foreign namespace"))?;
    if rest.is_empty() {
        return Err(emsg(path, "empty rest"));
    }
    if el_extension_api::extension_id::RESERVED_NAMESPACES
        .iter()
        .any(|namespace| path.starts_with(&format!("assets/{namespace}/")))
    {
        return Err(emsg(path, "reserved namespace path"));
    }
    if path.contains('\\') {
        return Err(emsg(path, "backslash"));
    }
    VirtualPath::new(path).map_err(|e| emsg(path, &format!("{e:?}")))?;
    for c in path.split('/') {
        if c.is_empty() || c == "." || c == ".." {
            return Err(emsg(path, "bad component"));
        }
    }
    if bytes.len() > MAX_ASSET {
        return Err(emsg(path, "asset too large"));
    }
    enum K {
        Json,
        Lang,
        Png,
        Mcmeta,
        Audio,
    }
    let kind = if rest == "sounds.json" {
        K::Json
    } else if let Some(r) = rest.strip_prefix("models/") {
        if r.ends_with(".json") {
            K::Json
        } else {
            return Err(emsg(path, "allowlist"));
        }
    } else if let Some(r) = rest.strip_prefix("blockstates/") {
        if r.ends_with(".json") {
            K::Json
        } else {
            return Err(emsg(path, "allowlist"));
        }
    } else if let Some(r) = rest.strip_prefix("items/") {
        if r.ends_with(".json") {
            K::Json
        } else {
            return Err(emsg(path, "allowlist"));
        }
    } else if let Some(r) = rest.strip_prefix("textures/") {
        if r.ends_with(".png.mcmeta") {
            K::Mcmeta
        } else if r.ends_with(".png") {
            K::Png
        } else {
            return Err(emsg(path, "allowlist"));
        }
    } else if let Some(r) = rest.strip_prefix("sounds/") {
        if r.ends_with(".ogg") || r.ends_with(".wav") {
            K::Audio
        } else {
            return Err(emsg(path, "allowlist"));
        }
    } else if let Some(r) = rest.strip_prefix("lang/") {
        if !r.contains('/') && r.ends_with(".json") {
            K::Lang
        } else {
            return Err(emsg(path, "allowlist"));
        }
    } else if let Some(r) = rest.strip_prefix("materials/") {
        if r.ends_with(".json") {
            K::Json
        } else {
            return Err(emsg(path, "allowlist"));
        }
    } else if let Some(r) = rest.strip_prefix("atlases/") {
        if !r.contains('/') && r.ends_with(".json") {
            K::Json
        } else {
            return Err(emsg(path, "allowlist"));
        }
    } else {
        return Err(emsg(path, "allowlist"));
    };
    match kind {
        K::Png => check_png(path, bytes),
        K::Audio => Ok(()),
        K::Json | K::Mcmeta => {
            if bytes.len() > MAX_JSON {
                return Err(emsg(path, "json too large"));
            }
            let v = strict_json(bytes, MAX_JSON).map_err(|e| emsg(path, &e))?;
            if !matches!(v, Value::Object(_)) {
                return Err(emsg(path, "json must be object"));
            }
            Ok(())
        }
        K::Lang => {
            if bytes.len() > MAX_JSON {
                return Err(emsg(path, "json too large"));
            }
            let v = strict_json(bytes, MAX_JSON).map_err(|e| emsg(path, &e))?;
            let Value::Object(m) = v else {
                return Err(emsg(path, "lang must be object"));
            };
            if m.len() > 4096 {
                return Err(emsg(path, "lang too many entries"));
            }
            for (_, v) in m.iter() {
                let s = v
                    .as_str()
                    .ok_or_else(|| emsg(path, "lang values strings"))?;
                if s.len() > 8192 {
                    return Err(emsg(path, "lang string too long"));
                }
            }
            Ok(())
        }
    }
}
fn check_png(path: &str, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 24 {
        return Err(emsg(path, "png too short"));
    }
    if bytes[0..8] != [137, 80, 78, 71, 13, 10, 26, 10] {
        return Err(emsg(path, "png signature"));
    }
    if &bytes[12..16] != b"IHDR" {
        return Err(emsg(path, "png ihdr"));
    }
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    if !(1..=2048).contains(&w) || !(1..=2048).contains(&h) {
        return Err(emsg(path, "png dimensions"));
    }
    Ok(())
}
fn emsg(path: &str, msg: &str) -> String {
    let p: String = path.chars().take(96).collect();
    let m: String = msg.chars().take(96).collect();
    format!("{p}: {m}")
}
fn owner_err(prefix: &str, owner: &ExtensionId) -> String {
    let s: String = format!("{owner:?}").chars().take(128).collect();
    format!("{prefix} {s}")
}
