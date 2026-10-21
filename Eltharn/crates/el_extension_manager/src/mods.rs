use el_extension_api::{
    CapabilityName,
    services::{
        BINARY_BUFFERS_CAPABILITY, CLIENT_APP_READ_CAPABILITY, CLIENT_APP_WRITE_CAPABILITY,
        CLIENT_AVATAR_HOOK_CAPABILITY, CLIENT_AVATAR_READ_CAPABILITY,
        CLIENT_AVATAR_WRITE_CAPABILITY, CLIENT_CAMERA_HOOK_CAPABILITY,
        CLIENT_CAMERA_READ_CAPABILITY, CLIENT_CAMERA_WRITE_CAPABILITY,
        CLIENT_CANVAS_WRITE_CAPABILITY, CLIENT_ENTITIES_READ_CAPABILITY,
        CLIENT_HOOK_STATE_WRITE_CAPABILITY, CLIENT_INPUT_HOOK_CAPABILITY,
        CLIENT_KEYBINDINGS_CAPABILITY, CLIENT_OVERLAY_WRITE_CAPABILITY,
        CLIENT_PLAYER_READ_CAPABILITY, CLIENT_RENDER_HOOK_CAPABILITY,
        CLIENT_TOOLTIP_HOOK_CAPABILITY, CONFIG_READ_CAPABILITY, CONFIG_WRITE_CAPABILITY,
        CONTENT_CLIENT_READ_CAPABILITY, CONTENT_CLIENT_WRITE_CAPABILITY,
        CONTENT_REGISTER_CAPABILITY, CONTENT_SERVER_READ_CAPABILITY,
        CONTENT_SERVER_WRITE_CAPABILITY, HOST_CONTEXT_CAPABILITY, HOST_JOBS_CAPABILITY,
        HOST_LOG_CAPABILITY, HOST_MESSAGING_CAPABILITY, NETWORK_CLIENT_SEND_CAPABILITY,
        NETWORK_SERVER_SEND_CAPABILITY, RuntimeSide, WORLD_BLOCKS_READ_CAPABILITY, negotiate,
    },
};
use el_mod_runtime::{
    policy::Policy,
    session::{ActorConfig, Definition, ModuleCode},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const API: &str = "1.14";
pub const CAPABILITIES: [&str; 32] = [
    CLIENT_KEYBINDINGS_CAPABILITY,
    CLIENT_OVERLAY_WRITE_CAPABILITY,
    CLIENT_TOOLTIP_HOOK_CAPABILITY,
    CLIENT_ENTITIES_READ_CAPABILITY,
    BINARY_BUFFERS_CAPABILITY,
    CONFIG_READ_CAPABILITY,
    CONFIG_WRITE_CAPABILITY,
    HOST_CONTEXT_CAPABILITY,
    HOST_LOG_CAPABILITY,
    HOST_MESSAGING_CAPABILITY,
    HOST_JOBS_CAPABILITY,
    CONTENT_REGISTER_CAPABILITY,
    CONTENT_CLIENT_READ_CAPABILITY,
    CONTENT_CLIENT_WRITE_CAPABILITY,
    CONTENT_SERVER_READ_CAPABILITY,
    CONTENT_SERVER_WRITE_CAPABILITY,
    NETWORK_SERVER_SEND_CAPABILITY,
    NETWORK_CLIENT_SEND_CAPABILITY,
    CLIENT_CAMERA_READ_CAPABILITY,
    CLIENT_PLAYER_READ_CAPABILITY,
    CLIENT_AVATAR_READ_CAPABILITY,
    CLIENT_AVATAR_WRITE_CAPABILITY,
    CLIENT_APP_READ_CAPABILITY,
    CLIENT_APP_WRITE_CAPABILITY,
    CLIENT_CAMERA_WRITE_CAPABILITY,
    CLIENT_CANVAS_WRITE_CAPABILITY,
    WORLD_BLOCKS_READ_CAPABILITY,
    CLIENT_CAMERA_HOOK_CAPABILITY,
    CLIENT_AVATAR_HOOK_CAPABILITY,
    CLIENT_RENDER_HOOK_CAPABILITY,
    CLIENT_INPUT_HOOK_CAPABILITY,
    CLIENT_HOOK_STATE_WRITE_CAPABILITY,
];
#[derive(Clone, Debug, Serialize)]
pub struct PreparedActor {
    pub package: el_launcher_core::extensions::PackagePin,
    pub slot: String,
    pub config: ActorConfig,
    pub module_sha256: el_extension_api::Sha256Digest,
    pub language: &'static str,
    pub disabled_optional: Vec<String>,
}
pub struct PreparedMods {
    pub actors: Vec<PreparedActor>,
    pub definitions: Vec<Definition>,
    pub assets: Vec<crate::content_assets::OwnedAssets>,
    pub disabled_optional: Vec<String>,
}
impl PreparedMods {
    pub fn descriptor(&self) -> serde_json::Value {
        serde_json::json!({"api":API,"abi":1,"policy":Policy::default(),"actors":self.actors,"disabled_optional":self.disabled_optional})
    }
}
pub fn prepare(
    packages: &[crate::package::Package],
    side: RuntimeSide,
) -> Result<PreparedMods, String> {
    let selected: Result<Vec<_>, _> = packages
        .iter()
        .map(|p| {
            p.metadata
                .as_ref()
                .map(|m| m.resolve_package(p.info.pin.clone()))
                .ok_or("Not a mod package")
        })
        .collect();
    let roster = crate::resolver::resolve(&selected?, side)?;
    let owners = packages
        .iter()
        .map(|package| package.info.pin.id.clone())
        .collect();
    let assets = crate::content_assets::collect(packages, &owners)?;
    let available: BTreeSet<CapabilityName> = CAPABILITIES
        .iter()
        .filter(|s| match side {
            RuntimeSide::Client => !matches!(
                **s,
                CONTENT_SERVER_READ_CAPABILITY
                    | CONTENT_SERVER_WRITE_CAPABILITY
                    | NETWORK_SERVER_SEND_CAPABILITY
            ),
            RuntimeSide::Server => !matches!(
                **s,
                CONTENT_CLIENT_READ_CAPABILITY
                    | CONTENT_CLIENT_WRITE_CAPABILITY
                    | NETWORK_CLIENT_SEND_CAPABILITY
                    | CLIENT_CAMERA_READ_CAPABILITY
                    | CLIENT_PLAYER_READ_CAPABILITY
                    | CLIENT_ENTITIES_READ_CAPABILITY
                    | CLIENT_AVATAR_READ_CAPABILITY
                    | CLIENT_AVATAR_WRITE_CAPABILITY
                    | CLIENT_APP_READ_CAPABILITY
                    | CLIENT_APP_WRITE_CAPABILITY
                    | CLIENT_KEYBINDINGS_CAPABILITY
                    | CLIENT_OVERLAY_WRITE_CAPABILITY
                    | CLIENT_CAMERA_WRITE_CAPABILITY
                    | CLIENT_CANVAS_WRITE_CAPABILITY
                    | WORLD_BLOCKS_READ_CAPABILITY
                    | CLIENT_CAMERA_HOOK_CAPABILITY
                    | CLIENT_TOOLTIP_HOOK_CAPABILITY
                    | CLIENT_AVATAR_HOOK_CAPABILITY
                    | CLIENT_RENDER_HOOK_CAPABILITY
                    | CLIENT_INPUT_HOOK_CAPABILITY
                    | CLIENT_HOOK_STATE_WRITE_CAPABILITY
            ),
        })
        .map(|s| s.parse().unwrap())
        .collect();
    let mut definitions = Vec::new();
    let mut actors = Vec::new();
    let mut exclusive_hooks = BTreeSet::new();
    for actor in roster.actors {
        let package = packages
            .iter()
            .find(|p| p.info.pin == actor.package)
            .ok_or("Resolved package missing")?;
        let metadata = package.metadata.as_ref().ok_or("Missing module metadata")?;
        let module = metadata
            .modules
            .get(&actor.slot)
            .ok_or("Resolved module missing")?;
        let key = format!("{}/{}", actor.package.id, actor.slot);
        let capabilities = negotiate(&module.required, &module.optional, &available, &available)
            .map_err(|e| format!("{key}: {e}"))?;
        if let Some(hooks) = &module.hooks {
            for binding in &hooks.bindings {
                if binding.mode == el_extension_api::hooks::HookMode::Replace
                    && !exclusive_hooks.insert(binding.point)
                {
                    return Err(format!(
                        "{key}: ambiguous exclusive replacements at {}",
                        binding.point.name()
                    ));
                }
            }
        }
        let code = match module.language {
            crate::metadata::ModuleLanguage::Wasm => ModuleCode::Wasm(
                package
                    .files
                    .get(&module.path)
                    .ok_or("Resolved bytes missing")?
                    .clone(),
            ),
            crate::metadata::ModuleLanguage::Rhai => {
                let sources = package
                    .files
                    .iter()
                    .filter(|(path, _)| path.ends_with(".rhai"))
                    .map(|(path, bytes)| {
                        let source = String::from_utf8(bytes.clone())
                            .map_err(|_| format!("Script is not UTF-8: {path}"))?;
                        Ok((path.clone(), source))
                    })
                    .collect::<Result<std::collections::BTreeMap<_, _>, String>>()?;
                ModuleCode::Script(el_mod_runtime::script::ScriptBundle {
                    entry: module.path.clone(),
                    sources,
                    hooks: module.hooks.clone(),
                })
            }
        };
        let bytes = serde_json::to_vec(&code).map_err(|e| e.to_string())?;
        let config = ActorConfig {
            key: key.clone(),
            dependencies: actor.required_dependencies,
            grants: capabilities
                .granted()
                .iter()
                .map(ToString::to_string)
                .collect(),
            memory_max: code.memory_reservation()?,
            required: false,
        };
        definitions.push(Definition {
            key,
            dependencies: config.dependencies.clone(),
            grants: config.grants.clone(),
            module: code,
            required: false,
        });
        actors.push(PreparedActor {
            package: actor.package,
            slot: actor.slot,
            config,
            module_sha256: el_extension_api::Sha256Digest::from_bytes(
                Sha256::digest(&bytes).into(),
            ),
            language: match module.language {
                crate::metadata::ModuleLanguage::Wasm => "wasm-v1",
                crate::metadata::ModuleLanguage::Rhai => "rhai-v1",
            },
            disabled_optional: capabilities
                .disabled_optional()
                .iter()
                .map(ToString::to_string)
                .collect(),
        });
    }
    el_mod_runtime::session::admit(&definitions, &Policy::default())?;
    Ok(PreparedMods {
        actors,
        definitions,
        assets,
        disabled_optional: roster.disabled_optional,
    })
}
