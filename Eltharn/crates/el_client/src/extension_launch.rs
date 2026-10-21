use serde_json::{Value, json};
use std::cell::RefCell;
thread_local! {
    static RECEIPT : RefCell < Option < Value >> = RefCell::new(None); static SELECTION :
    RefCell < Option < (el_launcher_core::extensions::ExtensionProfile,
    std::collections::BTreeMap < String, Value >, Option < String >) >> =
    RefCell::new(None); static TARGET : RefCell < Option < Value >> = RefCell::new(None);
    static MOD_DESCRIPTOR : RefCell < Value > = const { RefCell::new(Value::Null) };
    static MOD_NETWORK_ENABLED : RefCell < bool > = const { RefCell::new(false) }; static
    CONFIGURING : RefCell < bool > = const { RefCell::new(false) };
}
pub fn configured() -> bool {
    RECEIPT.with(|r| r.borrow().is_some())
}
pub(crate) fn selected_mod_pins() -> Vec<el_launcher_core::extensions::PackagePin> {
    SELECTION.with(|s| {
        s.borrow()
            .as_ref()
            .map(|(profile, _, _)| profile.mods.clone())
            .unwrap_or_default()
    })
}
pub(crate) fn mods_enabled() -> bool {
    SELECTION.with(|s| {
        s.borrow()
            .as_ref()
            .is_some_and(|(profile, _, _)| profile.flags.mods_enabled)
    })
}
pub(crate) fn mod_network_enabled() -> bool {
    MOD_NETWORK_ENABLED.with(|enabled| *enabled.borrow())
}
fn should_enable_mod_network(mods_enabled: bool, has_client_network_actor: bool) -> bool {
    mods_enabled && has_client_network_actor
}
pub fn receipt() -> Value {
    let mut receipt = RECEIPT.with(|r| r.borrow().clone().unwrap_or(Value::Null));
    let mods = crate::modding::report();
    if receipt.is_object() && !mods.is_null() {
        receipt["mods"] = mods;
    }
    receipt
}
#[cfg(not(target_arch = "wasm32"))]
pub fn configure_native(path: &std::path::Path) -> Result<(), String> {
    use std::io::Read;
    let limit = el_extension_manager::launch::MAX_LAUNCH_BYTES;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Launch request is not a file".into());
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let result = pollster::block_on(configure(&bytes));
    if let Err(error) = &result {
        RECEIPT.with(|r| {
            *r.borrow_mut() = Some(json!(
                { "schema_version" : 1, "status" : "rejected", "error" : error }
            ))
        });
        let _ = write_native_status();
    }
    result
}
#[cfg(not(target_arch = "wasm32"))]
pub fn write_native_status() -> Result<(), String> {
    use std::io::Write;
    let Some(path) = std::env::var_os("ELTHARN_EXTENSION_STATUS") else {
        return Ok(());
    };
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(receipt().to_string().as_bytes())
        .map_err(|e| e.to_string())
}
pub async fn configure(bytes: &[u8]) -> Result<(), String> {
    if configured() || CONFIGURING.with(|c| *c.borrow()) {
        return Err("extension runtime already configured".to_owned());
    }
    CONFIGURING.with(|c| *c.borrow_mut() = true);
    let result = configure_inner(bytes).await;
    CONFIGURING.with(|c| *c.borrow_mut() = false);
    result
}
async fn configure_inner(bytes: &[u8]) -> Result<(), String> {
    let v = el_extension_manager::launch::verify(bytes).await?;
    if configured() {
        return Err("extension runtime already configured".to_owned());
    }
    let mods_enabled = v.profile.flags.mods_enabled;
    let shaders_enabled = v.profile.flags.shaders_enabled;
    let mod_network_enabled = should_enable_mod_network(
        mods_enabled,
        v.mods.actors.iter().any(|actor| {
            actor
                .config
                .grants
                .iter()
                .any(|grant| grant == "network.client.send")
        }),
    );
    if !mods_enabled && !shaders_enabled {
        return Ok(());
    }
    let descriptor = v.mods.descriptor();
    let shader_profile = v
        .shader
        .as_ref()
        .map(|pack| pack.profile.name().to_owned())
        .or_else(|| v.shader_v4.as_ref().map(|pack| pack.profile.clone()));
    let mods = crate::modding::configure(v.mods.definitions).await?;
    crate::modding::client_content::set_assets(v.mods.assets);
    let prepared = match (v.shader, v.shader_v4) {
        (Some(_), Some(_)) => Err("conflicting released and v4 shader selection".to_owned()),
        (Some(pack), None) => crate::shaderpacks::configure_prepared(Some(pack), shaders_enabled),
        (None, Some(pack)) => {
            let id = v
                .profile
                .shader
                .as_ref()
                .map(|s| s.package.id.to_string())
                .ok_or("missing shader selection")?;
            crate::shaderpacks::configure_prepared_v4(pack, id, shaders_enabled)
        }
        (None, None) => crate::shaderpacks::configure_prepared(None, shaders_enabled),
    };
    if let Err(error) = prepared {
        crate::modding::shutdown();
        return Err(error);
    }
    MOD_DESCRIPTOR.with(|d| *d.borrow_mut() = descriptor);
    MOD_NETWORK_ENABLED.with(|enabled| *enabled.borrow_mut() = mod_network_enabled);
    let receipt = json!(
        { "schema_version" : 1, "selection_digest" : v.selection_digest.to_string(),
        "profile" : & v.profile.id, "flags" : & v.profile.flags, "status" : "prepared",
        "mods" : mods }
    );
    RECEIPT.with(|r| *r.borrow_mut() = Some(receipt));
    crate::extension_catalog::install(v.profile.clone(), v.catalog, v.mods.actors);
    SELECTION.with(|s| {
        *s.borrow_mut() = Some((v.profile, v.variant_settings, shader_profile));
    });
    Ok(())
}
pub(crate) fn record_target(
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: [u32; 2],
) {
    if !configured() {
        return;
    }
    let limits = device.limits();
    let formats: Vec<_> = [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Depth32Float,
        format,
    ]
    .into_iter()
    .map(|f| {
        let facts = adapter.get_texture_format_features(f);
        json!(
            { "format" : format!("{f:?}"), "allowed_usages" : facts.allowed_usages
            .bits(), "flags" : facts.flags.bits() }
        )
    })
    .collect();
    let target = json!(
        { "schema_version" : 1, "side" : "client", "backend" : format!("{:?}", adapter
        .get_info().backend), "host_contracts" : { "extension_api" :
        el_extension_manager::mods::API, "extension_abi" : 1, "shader_api" :
        el_shaderpack::graph_v4::SHADER_API_V4, "launch_set_schema" : 1 }, "mod_runtime"
        : MOD_DESCRIPTOR.with(| d | d.borrow().clone()), "authorization" : {
        "mod_instances" : MOD_DESCRIPTOR.with(| d | d.borrow() ["actors"].as_array()
        .map(| a | a.iter().map(| a | a["config"].clone()).collect::< Vec < _ >> ())
        .unwrap_or_default()), "shader_profiles" :
        el_extension_manager::session::AUTHORIZED_SHADER_PROFILES }, "effective_limits" :
        { "max_texture_dimension_2d" : limits.max_texture_dimension_2d.min(adapter
        .limits().max_texture_dimension_2d), "max_uniform_buffer_binding_size" : limits
        .max_uniform_buffer_binding_size, "max_uniform_buffers_per_shader_stage" : limits
        .max_uniform_buffers_per_shader_stage, "max_sampled_textures_per_shader_stage" :
        limits.max_sampled_textures_per_shader_stage, "device_limits" :
        format!("{limits:?}"), "adapter_limits" : format!("{:?}", adapter.limits()) },
        "device_features" : device.features().bits(), "format_facts" : formats,
        "scene_format" : format!("{format:?}"), "initial_dimensions" : size,
        "budget_bytes" : 67108864 }
    );
    TARGET.with(|t| *t.borrow_mut() = Some(target));
}
pub fn finish_gpu(error: Option<&str>) -> Result<(), String> {
    let gpu = crate::shaderpacks::report()["gpu"].clone();
    let sealed = if error.is_none() {
        SELECTION.with(|s| {
            TARGET.with(|t| {
                let selection = s.borrow();
                let (profile, settings, shader_profile) =
                    selection.as_ref().ok_or("Missing launch selection")?;
                let target = t.borrow();
                let target = target.as_ref().ok_or("Missing device profile")?;
                let variant = if profile.shader.is_some() {
                    let id = gpu["variant"]
                        .as_str()
                        .ok_or("GPU did not select a variant")?;
                    Some((id, settings.get(id).ok_or("Unknown GPU variant")?.clone()))
                } else {
                    None
                };
                let sealed = el_extension_manager::session::seal(
                    profile,
                    variant,
                    target,
                    shader_profile.as_deref(),
                )?;
                el_extension_manager::session::recheck(&sealed, target)?;
                serde_json::to_value(sealed).map_err(|e| e.to_string())
            })
        })
    } else {
        Ok(Value::Null)
    };
    let seal_error = sealed.as_ref().err().map(String::as_str);
    let error = error.or(seal_error);
    if error.is_some() {
        crate::modding::shutdown();
    }
    let snapshot = RECEIPT.with(|r| {
        let mut guard = r.borrow_mut();
        let Some(Value::Object(map)) = guard.as_mut() else {
            return None;
        };
        map.insert("gpu".into(), crate::shaderpacks::report()["gpu"].clone());
        map.insert(
            "probe_observer_active".into(),
            json!(crate::shaderpacks::is_configured()),
        );
        if let Ok(sealed) = &sealed {
            if !sealed.is_null() {
                map.insert("sealed_session".into(), sealed.clone());
            }
        }
        if let Some(msg) = error {
            map.insert("status".to_owned(), Value::from("rejected"));
            map.insert("error".to_owned(), Value::from(msg));
        } else {
            map.insert("status".to_owned(), Value::from("ready"));
            map.remove("error");
        }
        Some(guard.clone().unwrap_or(Value::Null))
    });
    match snapshot {
        None => Err(error.unwrap_or("extension runtime not prepared").to_owned()),
        Some(v) => {
            log::info!(
                "extension receipt: status={} profile={} error={}",
                v["status"].as_str().unwrap_or("unknown"),
                v["profile"].as_str().unwrap_or(""),
                v["error"].as_str().unwrap_or("none"),
            );
            match error {
                Some(e) => Err(e.to_owned()),
                None => Ok(()),
            }
        }
    }
}
#[cfg(target_arch = "wasm32")]
pub fn browser_failure(message: &str) {
    use wasm_bindgen::JsCast;
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(body) = document.body() else {
        return;
    };
    if let Ok(el) = document.create_element("div") {
        let _ = el.set_attribute("id", "eltharn_gate-extension-runtime-error");
        el.set_text_content(Some(&format!(
            "{} Reload to return to EltharnGate (profile unchanged).",
            message
        )));
        let _ = body.append_child(el.unchecked_ref());
    }
    if let Some(canvas) = document.get_element_by_id("eltharn_gate-canvas") {
        let _ = canvas.set_attribute("style", "display:none");
    }
}
