use base64::{Engine as _, engine::general_purpose::STANDARD};
use el_launcher_core::extensions::{ExtensionPreferences, ExtensionProfile};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

fn err(e: impl ToString) -> JsValue {
    JsValue::from_str(&e.to_string())
}

#[wasm_bindgen]
pub fn extension_validate_catalog(json: &str) -> Result<String, JsValue> {
    let entries = el_extension_manager::catalog::decode(json.as_bytes()).map_err(err)?;
    serde_json::to_string(&entries).map_err(err)
}

#[wasm_bindgen]
pub fn extension_default_preferences() -> String {
    serde_json::to_string(&ExtensionPreferences::default()).expect("default preferences")
}
#[wasm_bindgen]
pub fn extension_validate_preferences(json: &str) -> Result<String, JsValue> {
    let p = ExtensionPreferences::decode(json.as_bytes()).map_err(err)?;
    serde_json::to_string(&p).map_err(err)
}
#[wasm_bindgen]
pub async fn extension_inspect(bytes: Vec<u8>) -> Result<String, JsValue> {
    let p = el_extension_manager::package::inspect(bytes)
        .await
        .map_err(err)?;
    serde_json::to_string(&p.info).map_err(err)
}
#[wasm_bindgen]
pub async fn extension_shader_settings(bytes: Vec<u8>, settings: &str) -> Result<String, JsValue> {
    if settings.len() > 65_536 {
        return Err(err("Shader settings are too large"));
    }
    let settings = serde_json::from_str(settings).map_err(err)?;
    let package = el_extension_manager::package::inspect(bytes)
        .await
        .map_err(err)?;
    let mut editor = package.shader_settings(&settings).await.map_err(err)?;
    editor["package"] = serde_json::to_value(&package.info.pin).map_err(err)?;
    serde_json::to_string(&editor).map_err(err)
}
#[wasm_bindgen]
pub async fn extension_prepare(profile: &str, archives: &str) -> Result<Vec<u8>, JsValue> {
    let (profile, packages) = decode_selection(profile, archives)?;
    el_extension_manager::launch::prepare(profile, &packages)
        .await
        .map_err(err)
}

#[wasm_bindgen]
pub async fn extension_prepare_with_catalog(
    profile: &str,
    archives: &str,
    catalog: &str,
) -> Result<Vec<u8>, JsValue> {
    let catalog = el_extension_manager::catalog::decode(catalog.as_bytes()).map_err(err)?;
    let (profile, packages) = decode_selection(profile, archives)?;
    el_extension_manager::launch::prepare_with_catalog(profile, &packages, &catalog)
        .await
        .map_err(err)
}

fn decode_selection(
    profile: &str,
    archives: &str,
) -> Result<(ExtensionProfile, BTreeMap<String, Vec<u8>>), JsValue> {
    if profile.len() > 262144 || archives.len() > el_extension_manager::launch::MAX_LAUNCH_BYTES {
        return Err(err("Extension selection is too large"));
    }
    let profile: ExtensionProfile = serde_json::from_str(profile).map_err(err)?;
    let encoded: BTreeMap<String, String> = serde_json::from_str(archives).map_err(err)?;
    let mut packages = BTreeMap::new();
    let mut total = 0usize;
    for (key, text) in encoded {
        let padding = text
            .as_bytes()
            .iter()
            .rev()
            .take_while(|b| **b == b'=')
            .count()
            .min(2);
        total = total
            .checked_add((text.len() * 3 / 4).saturating_sub(padding))
            .ok_or_else(|| err("Archive size overflow"))?;
        if total > 32 * 1024 * 1024 {
            return Err(err("Selected packages exceed 32 MiB"));
        }
        packages.insert(key, STANDARD.decode(text).map_err(err)?);
    }
    Ok((profile, packages))
}
#[wasm_bindgen]
pub async fn extension_configure(bytes: &[u8]) -> Result<(), JsValue> {
    el_client::extension_launch::configure(bytes)
        .await
        .map_err(err)
}
#[wasm_bindgen]
pub fn extension_launch_receipt() -> String {
    el_client::extension_launch::receipt().to_string()
}
