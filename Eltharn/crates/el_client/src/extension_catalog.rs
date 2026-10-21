use el_extension_manager::{mods::PreparedActor, package::PackageInfo};
use el_gui::screens::extension_catalog::{ExtensionCatalog, ExtensionCatalogEntry};
use el_launcher_core::extensions::ExtensionProfile;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::BTreeSet;
#[derive(Default)]
struct Catalog {
    profile: Option<ExtensionProfile>,
    packages: Vec<PackageInfo>,
    actors: Vec<PreparedActor>,
}
thread_local! {
    static CATALOG : RefCell < Catalog > = RefCell::new(Catalog::default());
}
pub(crate) fn install(
    profile: ExtensionProfile,
    packages: Vec<PackageInfo>,
    actors: Vec<PreparedActor>,
) {
    CATALOG.with(|cell| {
        *cell.borrow_mut() = Catalog {
            profile: Some(profile),
            packages,
            actors,
        }
    });
}
pub(crate) fn snapshot() -> ExtensionCatalog {
    let (states, runtime_live) = crate::modding::catalog_actor_states();
    let hook_report = crate::modding::hooks::report();
    let disabled_hooks = hook_report["owners"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|owner| owner["disabled"] == true)
        .filter_map(|owner| owner["key"].as_str().map(str::to_owned))
        .collect();
    let shader = crate::shaderpacks::report();
    CATALOG.with(|cell| {
        project(
            &cell.borrow(),
            &states,
            runtime_live,
            &disabled_hooks,
            &shader,
        )
    })
}
fn label(value: &str, limit: usize) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_control())
        .take(limit)
        .collect()
}
fn project(
    catalog: &Catalog,
    states: &[Option<String>],
    runtime_live: bool,
    disabled_hooks: &BTreeSet<String>,
    shader: &Value,
) -> ExtensionCatalog {
    let Some(profile) = &catalog.profile else {
        return ExtensionCatalog::default();
    };
    let mut result = ExtensionCatalog {
        mods_enabled: profile.flags.mods_enabled,
        shaderpacks_enabled: profile.flags.shaders_enabled,
        ..ExtensionCatalog::default()
    };
    for package in &catalog.packages {
        let is_mod = package.kind == "mod";
        let selected = if is_mod {
            profile.mods.iter().any(|pin| pin == &package.pin)
        } else {
            profile
                .shader
                .as_ref()
                .is_some_and(|s| s.package == package.pin)
        };
        let mut entry = ExtensionCatalogEntry {
            id: package.pin.id.to_string(),
            name: label(&package.name, 128),
            version: package.pin.version.to_string(),
            content_id: package.pin.content_id.to_string(),
            description: label(&package.description, 512),
            status: "Installed (not selected)".into(),
            selected,
            settings_owner: None,
        };
        if selected && is_mod {
            let owners: Vec<_> = catalog
                .actors
                .iter()
                .enumerate()
                .filter(|(_, actor)| actor.package == package.pin)
                .collect();
            entry.status = if !profile.flags.mods_enabled {
                "Disabled in profile"
            } else if owners.is_empty() {
                "Selected (no client module)"
            } else if !runtime_live {
                "Runtime unavailable"
            } else if owners.iter().any(|(index, _)| {
                states.get(*index).and_then(|state| state.as_deref()) == Some("Disabled")
            }) {
                "Disabled after failure"
            } else if owners.iter().any(|(index, _)| {
                states.get(*index).and_then(|state| state.as_deref()) == Some("Terminated")
            }) {
                "Stopped"
            } else if owners.iter().any(|(index, _)| {
                !matches!(
                    states.get(*index).and_then(|state| state.as_deref()),
                    Some("Ready" | "InWorld")
                )
            }) {
                "Preparing"
            } else if owners
                .iter()
                .any(|(_, actor)| disabled_hooks.contains(&actor.config.key))
            {
                "Running; hooks disabled"
            } else {
                "Running"
            }
            .into();
            if runtime_live && profile.flags.mods_enabled {
                entry.settings_owner = owners.into_iter().find_map(|(index, actor)| {
                    (matches!(
                        states.get(index).and_then(|s| s.as_deref()),
                        Some("Ready" | "InWorld")
                    ) && actor.config.grants.iter().any(|g| g == "client.app.read")
                        && actor.config.grants.iter().any(|g| g == "client.app.write"))
                    .then(|| actor.config.key.clone())
                });
            }
        } else if selected {
            entry.status = shader_status(profile.flags.shaders_enabled, shader).into();
        }
        if is_mod {
            result.mods.push(entry);
        } else {
            result.shaderpacks.push(entry);
        }
    }
    let order = |a: &ExtensionCatalogEntry, b: &ExtensionCatalogEntry| {
        b.selected
            .cmp(&a.selected)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.version.cmp(&b.version))
            .then_with(|| a.content_id.cmp(&b.content_id))
    };
    result.mods.sort_by(order);
    result.shaderpacks.sort_by(order);
    result
}
fn shader_status(enabled: bool, report: &Value) -> &'static str {
    if !enabled {
        return "Disabled in profile";
    }
    match (report["mode"].as_str(), report["status"].as_str()) {
        (_, Some("failed" | "rejected")) => "Failed",
        (Some("pack"), Some("ready")) if report["shader_frames"].as_u64().unwrap_or(0) > 0 => {
            "Rendering"
        }
        (Some("pack"), Some("ready")) => "Ready (no world frame yet)",
        (Some("off"), Some("ready")) => "Disabled in game",
        _ => "Selected (preparing)",
    }
}
