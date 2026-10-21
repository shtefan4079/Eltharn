use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
const MAX_BYTES: usize = 1_048_576;
const TARGET_DOMAIN: &str = "rustore.target-profile.v1";
const LAUNCH_SET_DOMAIN: &str = "rustore.launch-set.v1";
pub const AUTHORIZED_SHADER_PROFILES: [&str; 7] = [
    "post_color_v1",
    "post_color_depth_v1",
    "post_color_depth_environment_v1",
    el_shaderpack::graph_v4::PACK_PROFILE_V4,
    el_shaderpack::graph_v4::PACK_PROFILE_FRAME,
    el_shaderpack::graph_v4::PACK_PROFILE_TEMPORAL,
    el_shaderpack::graph_v4::PACK_PROFILE_LIGHTING,
];
pub fn shader_api_for_profile(profile: &str) -> Option<&'static str> {
    match profile {
        "post_color_v1" | "post_color_depth_v1" => Some("1.1"),
        "post_color_depth_environment_v1" => Some("1.2"),
        p if p == el_shaderpack::graph_v4::PACK_PROFILE_V4 => Some("1.3"),
        p if p == el_shaderpack::graph_v4::PACK_PROFILE_FRAME => Some("1.3"),
        p if p == el_shaderpack::graph_v4::PACK_PROFILE_TEMPORAL => Some("1.3"),
        p if p == el_shaderpack::graph_v4::PACK_PROFILE_LIGHTING => Some("1.3"),
        _ => None,
    }
}
fn frame_settings_shape(settings: &Value) -> bool {
    settings.as_object().is_some_and(|settings| {
        settings.len() <= 64
            && settings.iter().all(|(id, value)| {
                !id.is_empty()
                    && id.len() <= 64
                    && id
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                    && (value.is_boolean()
                        || value.is_number()
                        || value
                            .as_str()
                            .is_some_and(|s| !s.is_empty() && s.len() <= 64))
            })
    })
}
fn has_frame_settings(profile: &str) -> bool {
    matches!(
        profile,
        el_shaderpack::graph_v4::PACK_PROFILE_FRAME
            | el_shaderpack::graph_v4::PACK_PROFILE_TEMPORAL
            | el_shaderpack::graph_v4::PACK_PROFILE_LIGHTING
    )
}
fn authorized_profiles(target: &Value) -> Result<Vec<String>, String> {
    let auth = target
        .get("authorization")
        .ok_or("target authorization missing")?;
    let list = auth
        .get("shader_profiles")
        .ok_or("target shader_profiles missing")?;
    list.as_array()
        .ok_or("target shader_profiles must be array")?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or_else(|| "target shader_profiles must be strings".to_owned())
        })
        .collect()
}
#[derive(Clone, Serialize)]
pub struct SealedSession {
    pub descriptor_utf8: String,
    pub target_utf8: String,
    pub reference: el_extension_api::launch::PreparedSetRef,
}
fn hash(domain: &str, bytes: &[u8]) -> el_extension_api::Sha256Digest {
    let mut h = Sha256::new();
    h.update(domain.as_bytes());
    h.update([0u8]);
    h.update(bytes);
    el_extension_api::Sha256Digest::from_bytes(h.finalize().into())
}
fn check_target(t: &Value) -> Result<(), String> {
    let o = t.as_object().ok_or("target must be object")?;
    if o.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err("target schema_version must be 1".into());
    }
    if o.get("side").and_then(Value::as_str) != Some("client") {
        return Err("target side must be client".into());
    }
    if o.get("backend").and_then(Value::as_str).is_none() {
        return Err("target backend must be string".into());
    }
    if !o.get("effective_limits").map_or(false, Value::is_object) {
        return Err("target effective_limits must be object".into());
    }
    if !o.get("format_facts").map_or(false, Value::is_array) {
        return Err("target format_facts must be array".into());
    }
    if o.get("budget_bytes").and_then(Value::as_u64).is_none() {
        return Err("target budget_bytes must be u64".into());
    }
    Ok(())
}
fn check_mods(
    pins: &[el_launcher_core::extensions::PackagePin],
    target: &Value,
) -> Result<(), String> {
    if pins.is_empty() && target.get("mod_runtime").is_none() {
        return Ok(());
    }
    let policy = el_mod_runtime::policy::Policy::default();
    let runtime = &target["mod_runtime"];
    if runtime["api"] != crate::mods::API
        || runtime["abi"] != 1
        || runtime["policy"] != serde_json::to_value(&policy).map_err(|e| e.to_string())?
    {
        return Err("Invalid mod runtime policy".into());
    }
    let actors = runtime["actors"].as_array().ok_or("Missing mod roster")?;
    if actors.len() > policy.max_actors {
        return Err("Actor limit".into());
    }
    let mut configs = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for actor in actors {
        let pin: el_launcher_core::extensions::PackagePin =
            serde_json::from_value(actor["package"].clone()).map_err(|e| e.to_string())?;
        if !pins.contains(&pin) {
            return Err("Unselected actor package".into());
        }
        let slot = actor["slot"].as_str().ok_or("Missing actor slot")?;
        if !matches!(slot, "client" | "shared") {
            return Err("Wrong actor side".into());
        }
        let config: el_mod_runtime::session::ActorConfig =
            serde_json::from_value(actor["config"].clone()).map_err(|e| e.to_string())?;
        if config.key != format!("{}/{slot}", pin.id) || !seen.insert(config.key.clone()) {
            return Err("Invalid/duplicate actor identity".into());
        }
        let _: el_extension_api::Sha256Digest =
            serde_json::from_value(actor["module_sha256"].clone()).map_err(|e| e.to_string())?;
        if !matches!(actor["language"].as_str(), Some("wasm-v1" | "rhai-v1")) {
            return Err("Unsupported mod language".into());
        }
        configs.push(config);
    }
    el_mod_runtime::session::Controller::new(configs, 1, policy)?;
    Ok(())
}
pub fn seal(
    profile: &el_launcher_core::extensions::ExtensionProfile,
    selection: Option<(&str, Value)>,
    target: &Value,
    shader_profile: Option<&str>,
) -> Result<SealedSession, String> {
    let flags = profile.flags;
    el_launcher_core::extensions::ExtensionPreferences {
        schema_version: 1,
        active_profile: profile.id.clone(),
        profiles: vec![profile.clone()],
    }
    .validate()?;
    if !flags.mods_enabled && !flags.shaders_enabled {
        return Err("base flags both false".into());
    }
    if !flags.mods_enabled && !profile.mods.is_empty() {
        return Err("mods present while mods_enabled=false".into());
    }
    if !flags.shaders_enabled && profile.shader.is_some() {
        return Err("shader present while shaders_enabled=false".into());
    }
    check_target(target)?;
    check_mods(&profile.mods, target)?;
    let target_utf8 = serde_json::to_string(target).map_err(|e| e.to_string())?;
    if target_utf8.len() > MAX_BYTES {
        return Err("target bytes > 1MiB".into());
    }
    let th = hash(TARGET_DOMAIN, target_utf8.as_bytes());
    let (shader_selection, shader_api) = match (&profile.shader, &selection, shader_profile) {
        (None, None, None) => (Value::Null, "1.1"),
        (Some(s), Some((variant, settings)), Some(validated)) => {
            let api = shader_api_for_profile(validated)
                .ok_or_else(|| format!("unknown shader profile {validated:?}"))?;
            let allowed = authorized_profiles(target)?;
            if !allowed.iter().any(|p| p == validated) {
                return Err("selected shader profile not in target authorization".into());
            }
            if variant.is_empty() {
                return Err("empty variant".into());
            }
            if !settings.is_object() {
                return Err("settings must be object".into());
            }
            if !s.settings.is_object() {
                return Err("profile shader settings must be object".into());
            }
            if api == el_shaderpack::graph_v4::SHADER_API_V4 {
                let frame = has_frame_settings(validated);
                if *variant != crate::launch::V4_POST_ONLY_VARIANT
                    || (!frame && (settings != &json!({}) || s.settings != json!({})))
                    || (frame
                        && (!frame_settings_shape(settings)
                            || !frame_settings_shape(&s.settings)
                            || !s
                                .settings
                                .as_object()
                                .unwrap()
                                .iter()
                                .all(|(key, value)| settings.get(key) == Some(value))))
                {
                    return Err(
                        "v4 shader selection requires v4_post_only and validated profile settings"
                            .into(),
                    );
                }
                if target
                    .pointer("/host_contracts/shader_api")
                    .and_then(Value::as_str)
                    != Some(el_shaderpack::graph_v4::SHADER_API_V4)
                {
                    return Err("target does not advertise shader_api 1.3".into());
                }
            }
            (
                json!(
                    { "package" : s.package.id.to_string(), "version" : s.package.version
                    .to_string(), "content_id" : s.package.content_id.to_string(),
                    "variant" : variant, "settings" : settings, "pack_profile" :
                    validated, "shader_api" : api }
                ),
                api,
            )
        }
        _ => return Err("shader/selection/profile presence mismatch".into()),
    };
    let mods: Vec<Value> = profile
        .mods
        .iter()
        .map(|m| {
            json!(
                { "id" : m.id.to_string(), "version" : m.version.to_string(),
                "content_id" : m.content_id.to_string() }
            )
        })
        .collect();
    let descriptor = json!(
        { "schema_version" : 1, "flags" : { "mods_enabled" : flags.mods_enabled,
        "shaders_enabled" : flags.shaders_enabled }, "mods" : mods, "mod_runtime" :
        target["mod_runtime"], "shader_selection" : shader_selection, "host_contracts" :
        { "extension_api" : crate ::mods::API, "extension_abi" : 1, "shader_api" :
        shader_api, "launch_set_schema" : 1 }, "target_profile" : th.to_string() }
    );
    let descriptor_utf8 = serde_json::to_string(&descriptor).map_err(|e| e.to_string())?;
    if descriptor_utf8.len() > MAX_BYTES {
        return Err("descriptor bytes > 1MiB".into());
    }
    let dh = hash(LAUNCH_SET_DOMAIN, descriptor_utf8.as_bytes());
    let reference =
        el_extension_api::launch::PreparedSetRef::new(1, flags, dh).map_err(|e| e.to_string())?;
    Ok(SealedSession {
        descriptor_utf8,
        target_utf8,
        reference,
    })
}
pub fn recheck(sealed: &SealedSession, target: &Value) -> Result<(), String> {
    let expect = serde_json::to_string(target).map_err(|e| e.to_string())?;
    if expect.as_bytes() != sealed.target_utf8.as_bytes() {
        return Err("target serialization mismatch".into());
    }
    if sealed.target_utf8.len() > MAX_BYTES || sealed.descriptor_utf8.len() > MAX_BYTES {
        return Err("sealed bytes > 1MiB".into());
    }
    check_target(target)?;
    let th = hash(TARGET_DOMAIN, sealed.target_utf8.as_bytes());
    let desc: Value =
        el_launcher_core::extension_json::strict_json(sealed.descriptor_utf8.as_bytes(), MAX_BYTES)
            .map_err(|e| e.to_string())?;
    let expected = [
        "schema_version",
        "flags",
        "mods",
        "mod_runtime",
        "shader_selection",
        "host_contracts",
        "target_profile",
    ];
    let obj = desc.as_object().ok_or("descriptor must be an object")?;
    if obj.len() != expected.len() || !expected.iter().all(|k| obj.contains_key(*k)) {
        return Err("descriptor fields".into());
    }
    let mods: Vec<el_launcher_core::extensions::PackagePin> =
        serde_json::from_value(desc["mods"].clone()).map_err(|e| e.to_string())?;
    check_mods(&mods, target)?;
    if desc["mod_runtime"] != target["mod_runtime"] {
        return Err("mod runtime profile mismatch".into());
    }
    let contracts = desc
        .get("host_contracts")
        .and_then(Value::as_object)
        .ok_or("host contracts must be object")?;
    if contracts.get("extension_api").and_then(Value::as_str) != Some(crate::mods::API)
        || contracts.get("extension_abi").and_then(Value::as_u64) != Some(1)
        || !matches!(
            contracts.get("shader_api").and_then(Value::as_str),
            Some("1.1") | Some("1.2") | Some("1.3")
        )
        || contracts.get("launch_set_schema").and_then(Value::as_u64) != Some(1)
        || contracts.len() != 4
    {
        return Err("host contract mismatch".into());
    }
    let sealed_api = contracts
        .get("shader_api")
        .and_then(Value::as_str)
        .ok_or("host contract shader_api missing")?;
    if desc.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err("descriptor schema_version must be 1".into());
    }
    if desc.get("target_profile").and_then(Value::as_str) != Some(th.to_string().as_str()) {
        return Err("descriptor target_profile mismatch".into());
    }
    let dh = hash(LAUNCH_SET_DOMAIN, sealed.descriptor_utf8.as_bytes());
    if *sealed.reference.digest() != dh {
        return Err("descriptor hash mismatch".into());
    }
    let f = sealed.reference.flags();
    let typed: el_extension_api::launch::ExtensionFlags =
        serde_json::from_value(desc["flags"].clone()).map_err(|e| e.to_string())?;
    if typed != f {
        return Err("flag mismatch".into());
    }
    if !f.mods_enabled && !mods.is_empty() {
        return Err("Disabled mods in sealed session".into());
    }
    let shader = &desc["shader_selection"];
    if !shader.is_null() {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Shader {
            package: el_extension_api::ExtensionId,
            version: el_extension_api::PackageVersion,
            content_id: el_extension_api::Sha256Digest,
            variant: String,
            settings: Value,
            pack_profile: String,
            shader_api: String,
        }
        let s: Shader = serde_json::from_value(shader.clone()).map_err(|e| e.to_string())?;
        if !f.shaders_enabled
            || s.package.is_reserved_namespace()
            || s.variant.is_empty()
            || !s.settings.is_object()
        {
            return Err("invalid selected shader".into());
        }
        let expected_api =
            shader_api_for_profile(&s.pack_profile).ok_or("unknown sealed shader profile")?;
        if s.shader_api != expected_api || sealed_api != expected_api {
            return Err("sealed shader_api/profile mismatch".into());
        }
        if expected_api == el_shaderpack::graph_v4::SHADER_API_V4 {
            let valid_settings = if has_frame_settings(&s.pack_profile) {
                frame_settings_shape(&s.settings)
            } else {
                s.settings == json!({})
            };
            if s.variant != crate::launch::V4_POST_ONLY_VARIANT || !valid_settings {
                return Err("sealed v4 variant/settings mismatch".into());
            }
            if target
                .pointer("/host_contracts/shader_api")
                .and_then(Value::as_str)
                != Some(el_shaderpack::graph_v4::SHADER_API_V4)
            {
                return Err("target does not advertise shader_api 1.3".into());
            }
        }
        let allowed = authorized_profiles(target)?;
        if !allowed.iter().any(|p| p == &s.pack_profile) {
            return Err("sealed shader profile not in target authorization".into());
        }
        let _ = (s.version, s.content_id);
    } else if sealed_api != "1.1" {
        return Err("shaderless session must seal shader_api 1.1".into());
    }
    let df = desc.get("flags").ok_or("descriptor missing flags")?;
    if df.get("mods_enabled").and_then(Value::as_bool) != Some(f.mods_enabled) {
        return Err("descriptor flags.mods_enabled mismatch".into());
    }
    if df.get("shaders_enabled").and_then(Value::as_bool) != Some(f.shaders_enabled) {
        return Err("descriptor flags.shaders_enabled mismatch".into());
    }
    if desc
        .get("host_contracts")
        .and_then(|h| h.get("launch_set_schema"))
        .and_then(Value::as_u64)
        != Some(1)
    {
        return Err("launch_set_schema must be 1".into());
    }
    Ok(())
}
