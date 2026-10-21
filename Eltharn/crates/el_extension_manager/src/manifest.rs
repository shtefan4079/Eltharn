use el_extension_api::{
    AbiVersion, ApiVersion, CapabilityName, ExtensionId, PackagePath, PackageVersion, Sha256Digest,
};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fmt};

pub(crate) type Result<T> = std::result::Result<T, String>;

const JSON_BYTE_LIMIT: usize = 262144;

pub fn strict_json(bytes: &[u8]) -> Result<Value> {
    el_strict_json::strict_json(bytes, JSON_BYTE_LIMIT)
}

fn ensure(ok: bool, msg: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(msg.into()) }
}
pub(crate) fn object(v: &Value) -> Result<&Map<String, Value>> {
    v.as_object().ok_or("expected object".into())
}
pub(crate) fn array(v: &Value) -> Result<&[Value]> {
    v.as_array()
        .map(Vec::as_slice)
        .ok_or("expected array".into())
}
pub(crate) fn string(v: &Value) -> Result<&str> {
    v.as_str().ok_or("expected string".into())
}
pub(crate) fn uint(v: &Value) -> Result<u64> {
    v.as_u64().ok_or("expected unsigned integer token".into())
}
pub(crate) fn field<'a>(v: &'a Value, name: &str) -> Result<&'a Value> {
    v.get(name).ok_or_else(|| format!("missing {name}"))
}
pub(crate) fn keys(v: &Value, required: &[&str], optional: &[&str]) -> Result<()> {
    let o = object(v)?;
    ensure(
        required.iter().all(|k| o.contains_key(*k)),
        "missing required field",
    )?;
    ensure(
        o.keys()
            .all(|k| required.contains(&k.as_str()) || optional.contains(&k.as_str())),
        "unsupported fixture field",
    )
}
fn typed<T: std::str::FromStr>(v: &Value) -> Result<T>
where
    T::Err: fmt::Display,
{
    string(v)?.parse().map_err(|e: T::Err| e.to_string())
}
fn text_limit(v: &Value, min: usize, max: usize) -> Result<()> {
    let s = string(v)?;
    ensure(
        (min..=max).contains(&s.chars().count()) && !s.chars().any(char::is_control),
        "text bounds/control",
    )
}
fn hash(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::from_bytes(Sha256::digest(bytes).into())
}

type ReadPayload<'a> = dyn Fn(&str, u64) -> Result<Vec<u8>> + 'a;
type VerifyShader<'a> = dyn Fn(&Value, &BTreeSet<String>) -> Result<Value> + 'a;

pub(crate) fn verify_source(
    bytes: &[u8],
    receipt_bytes: &[u8],
    actual: &BTreeSet<String>,
    read: &ReadPayload<'_>,
    shader: &VerifyShader<'_>,
) -> Result<Value> {
    let m = strict_json(&bytes)?;
    let kind = string(field(&m, "kind")?)?;
    let source_mod = kind == "mod" && m["manifest_version"].as_u64() == Some(2);
    let mut required = vec![
        "manifest_version",
        "kind",
        "id",
        "version",
        "display_name",
        "license",
        "files",
    ];
    let optional: &[&str] = match kind {
        "mod" => {
            required.extend(["extension_api", "modules"]);
            if !source_mod {
                required.push("extension_abi");
            }
            &[
                "description",
                "capabilities",
                "dependencies",
                "conflicts",
                "assets",
                "data_schemas",
            ]
        }
        "shaderpack" => {
            required.extend(["shader_api", "shader"]);
            &["description"]
        }
        _ => return Err("unsupported kind".into()),
    };
    keys(&m, &required, optional)?;
    ensure(
        uint(field(&m, "manifest_version")?)? == 1 || source_mod,
        "manifest version",
    )?;
    let id: ExtensionId = typed(field(&m, "id")?)?;
    ensure(
        !id.is_reserved_namespace(),
        "reserved third-party namespace",
    )?;
    let version: PackageVersion = typed(field(&m, "version")?)?;
    text_limit(field(&m, "display_name")?, 1, 64)?;
    if let Some(v) = m.get("description") {
        text_limit(v, 0, 512)?;
    }
    let license = string(field(&m, "license")?)?;
    ensure(
        (1..=128).contains(&license.len())
            && license
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".+-".contains(&b)),
        "license syntax",
    )?;
    let files = array(field(&m, "files")?)?;
    ensure((1..=4096).contains(&files.len()), "file count")?;
    let mut names = BTreeSet::new();
    let mut total = 0u64;
    for f in files {
        keys(f, &["path", "size", "sha256"], &[])?;
        let path: PackagePath = typed(field(f, "path")?)?;
        ensure(names.insert(path.to_string()), "duplicate path")?;
        let size = uint(field(f, "size")?)?;
        ensure(size <= 67108864, "file size")?;
        total += size;
        ensure(total <= 536870912, "payload total")?;
        let expected: Sha256Digest = typed(field(f, "sha256")?)?;
        let data = read(path.as_str(), size)?;
        ensure(
            data.len() as u64 == size && hash(&data) == expected,
            "payload size/hash",
        )?;
    }
    for name in &names {
        for (pos, _) in name.match_indices('/') {
            ensure(!names.contains(&name[..pos]), "file/directory collision")?;
        }
    }
    ensure(*actual == names, "unlisted or missing payload")?;
    let receipt = strict_json(receipt_bytes)?;
    keys(&receipt, &["id", "version", "content_id"], &[])?;
    let rid: ExtensionId = typed(field(&receipt, "id")?)?;
    let rv: PackageVersion = typed(field(&receipt, "version")?)?;
    let content: Sha256Digest = typed(field(&receipt, "content_id")?)?;
    let mut hasher = Sha256::new();
    hasher.update(b"rustore.package.v1\0");
    hasher.update(&bytes);
    ensure(
        rid == id && rv == version && content == Sha256Digest::from_bytes(hasher.finalize().into()),
        "receipt mismatch",
    )?;
    let details = if kind == "mod" {
        verify_mod(&m, &names, read)?
    } else {
        shader(&m, &names)?
    };
    Ok(
        json!({"kind":kind,"id":id,"version":version,"content_id":content,"payload_files":names.len(),"payload_bytes":total,"details":details}),
    )
}

fn verify_mod(m: &Value, names: &BTreeSet<String>, read: &ReadPayload<'_>) -> Result<Value> {
    crate::metadata::parse(m)?;
    let api: ApiVersion = typed(field(m, "extension_api")?)?;
    ensure(
        crate::mods::API
            .parse::<ApiVersion>()
            .unwrap()
            .supports(&api),
        "extension API mismatch",
    )?;
    let source_mod = m["manifest_version"].as_u64() == Some(2);
    if !source_mod {
        let abi: AbiVersion = serde_json::from_value(field(m, "extension_abi")?.clone())
            .map_err(|e| e.to_string())?;
        ensure(abi == AbiVersion::new(1).unwrap(), "extension ABI mismatch")?;
    }
    let modules = field(m, "modules")?;
    keys(modules, &[], &["client", "server", "shared"])?;
    ensure(!object(modules)?.is_empty(), "empty module set")?;
    let known = [
        "init",
        "tick",
        "shutdown",
        "on_world_enter",
        "on_world_exit",
        "app_tick",
    ];
    let mut count = 0;
    for (slot, module) in object(modules)? {
        let entry_field = if source_mod { "script" } else { "wasm" };
        keys(module, &[entry_field, "exports"], &["hooks"])?;
        let hooks = crate::metadata::parse_hooks(module, slot, source_mod, names)?;
        let path: PackagePath = typed(field(module, entry_field)?)?;
        ensure(
            path.as_str()
                .ends_with(if source_mod { ".rhai" } else { ".wasm" })
                && names.contains(path.as_str()),
            "module path",
        )?;
        let mut declared = BTreeSet::new();
        for e in array(field(module, "exports")?)? {
            let e = string(e)?;
            ensure(
                known.contains(&e)
                    && (source_mod || e != "app_tick")
                    && declared.insert(e.to_owned()),
                "lifecycle name/duplicate",
            )?;
        }
        if source_mod {
            let sources = names
                .iter()
                .filter(|p| p.ends_with(".rhai"))
                .map(|p| {
                    let data = read(p, el_mod_runtime::script::MAX_FILE_BYTES as u64)?;
                    let source =
                        String::from_utf8(data).map_err(|_| format!("script is not UTF-8: {p}"))?;
                    Ok((p.clone(), source))
                })
                .collect::<Result<std::collections::BTreeMap<_, _>>>()?;
            let actual = el_mod_runtime::script::ScriptBundle {
                entry: path.to_string(),
                sources,
                hooks,
            }
            .validate_program()?;
            let actual: BTreeSet<_> = actual
                .into_iter()
                .filter(|name| known.contains(&name.as_str()))
                .collect();
            ensure(
                actual == declared,
                "lifecycle declarations differ from source",
            )?;
            count += 1;
            continue;
        }
        let bytes = read(path.as_str(), crate::admission::MAX_MODULE_BYTES as u64)?;
        crate::admission::validate(&bytes).map_err(|e| e.to_string())?;
        let mut actual = BTreeSet::new();
        for p in wasmparser::Parser::new(0).parse_all(&bytes) {
            if let wasmparser::Payload::ExportSection(s) = p.map_err(|e| e.to_string())? {
                for e in s {
                    let e = e.map_err(|e| e.to_string())?;
                    if known.contains(&e.name) && e.name != "app_tick" {
                        actual.insert(e.name.to_owned());
                    }
                }
            }
        }
        ensure(
            actual == declared,
            "lifecycle declarations differ from binary",
        )?;
        count += 1;
    }
    if let Some(caps) = m.get("capabilities") {
        keys(caps, &[], &["client", "server", "shared"])?;
        for (slot, lists) in object(caps)? {
            ensure(
                modules.get(slot).is_some(),
                "capabilities for absent module",
            )?;
            keys(lists, &[], &["required", "optional"])?;
            let mut all = BTreeSet::new();
            for values in object(lists)?.values() {
                let values = array(values)?;
                ensure(values.len() <= 64, "capability limit")?;
                for c in values {
                    let cap: CapabilityName = typed(c)?;
                    ensure(all.insert(cap), "duplicate/overlapping capabilities")?;
                }
            }
        }
    }
    Ok(json!({"admitted_modules":count,"lifecycle_matches":true}))
}
