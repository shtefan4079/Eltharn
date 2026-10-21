use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(crate) fn normalize(
    bytes: &[u8],
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>, String> {
    let mut manifest = crate::manifest::strict_json(bytes)?;
    if manifest.get("manifest_version").and_then(Value::as_u64) != Some(2) {
        return Ok(bytes.to_vec());
    }
    let root = manifest
        .as_object_mut()
        .ok_or("manifest must be an object")?;
    if root.get("kind").and_then(Value::as_str) != Some("mod") {
        return Err("manifest_version 2 is for source mods".into());
    }
    if root.contains_key("files") || root.contains_key("extension_abi") {
        return Err(
            "source manifests omit files and extension_abi; inventory is computed on import".into(),
        );
    }
    let modules = root
        .get_mut("modules")
        .and_then(Value::as_object_mut)
        .ok_or("source manifest requires modules")?;
    let sources = files
        .iter()
        .filter(|(path, _)| path.ends_with(".rhai"))
        .map(|(path, bytes)| {
            String::from_utf8(bytes.clone())
                .map(|text| (path.clone(), text))
                .map_err(|_| format!("script is not UTF-8: {path}"))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let names = files.keys().cloned().collect();
    for (slot, module) in modules.iter_mut() {
        let hooks = crate::metadata::parse_hooks(module, slot, true, &names)?;
        let module = module.as_object_mut().ok_or("module must be an object")?;
        if !module.contains_key("script") || module.contains_key("wasm") {
            return Err("source modules require a script entry".into());
        }
        let entry = module["script"]
            .as_str()
            .ok_or("script entry must be a string")?;
        let callbacks = el_mod_runtime::script::ScriptBundle {
            entry: entry.into(),
            sources: sources.clone(),
            hooks,
        }
        .validate_program()?;
        let callbacks: Vec<_> = callbacks
            .into_iter()
            .filter(|name| {
                matches!(
                    name.as_str(),
                    "init" | "tick" | "shutdown" | "on_world_enter" | "on_world_exit" | "app_tick"
                )
            })
            .collect();
        let derived = json!(callbacks);
        if let Some(declared) = module.get("exports") {
            let declared = declared.as_array().ok_or("exports must be an array")?;
            let mut declared: Vec<_> = declared
                .iter()
                .map(|name| name.as_str().ok_or("export must be a string"))
                .collect::<Result<_, _>>()?;
            declared.sort_unstable();
            if json!(declared) != derived {
                return Err("lifecycle declarations differ from source".into());
            }
        }
        module.insert("exports".into(), derived);
    }
    let inventory: Vec<Value> = files.iter()
        .filter(|(path, _)| path.as_str() != el_extension_api::package_path::MANIFEST_FILE_NAME)
        .map(|(path, data)| json!({
            "path":path, "size":data.len(),
            "sha256":el_extension_api::Sha256Digest::from_bytes(Sha256::digest(data).into()).to_string()
        })).collect();
    root.insert("files".into(), inventory.into());
    if !root.contains_key("assets") {
        let resources: Vec<&String> = files
            .keys()
            .filter(|path| path.starts_with("assets/"))
            .collect();
        if !resources.is_empty() {
            root.insert(
                "assets".into(),
                json!({"root":"assets/", "resources":resources}),
            );
        }
    }
    serde_json::to_vec(&manifest).map_err(|e| e.to_string())
}
