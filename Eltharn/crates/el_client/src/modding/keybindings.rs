use crate::input::VanillaKeyMappings;
use el_mod_runtime::content::{MAX_INPUT_OWNERS, MAX_OWNER_LEN};
use el_mod_runtime::keybindings::{Binding, Report, validate_bindings};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
thread_local! {
    static REGISTRY : RefCell < BTreeMap < String, Vec < Binding >>> = const {
    RefCell::new(BTreeMap::new()) }; static UI_DIRTY : Cell < bool > = const {
    Cell::new(false) }; static RESET_PENDING : Cell < bool > = const { Cell::new(true) };
}
pub fn accept(report: &serde_json::Value) -> Result<(), String> {
    let value = report
        .get("keybindings")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({ "bindings" : {} }));
    let registry: Report = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if registry.bindings.len() > MAX_INPUT_OWNERS {
        return Err("Mod keybinding owner limit".into());
    }
    for (owner, bindings) in &registry.bindings {
        if owner.is_empty() || owner.len() > MAX_OWNER_LEN || owner.chars().any(char::is_control) {
            return Err("Invalid mod keybinding owner".into());
        }
        validate_bindings(bindings)?;
    }
    REGISTRY.with(|cell| *cell.borrow_mut() = registry.bindings);
    Ok(())
}
pub fn sync(controls: &mut VanillaKeyMappings, saved: &HashMap<String, String>) -> bool {
    let reset = RESET_PENDING.with(|pending| pending.replace(false));
    let removed = reset && controls.sync_mod_bindings(&BTreeMap::new(), saved);
    let changed =
        REGISTRY.with(|cell| controls.sync_mod_bindings(&cell.borrow(), saved)) || removed;
    if changed {
        UI_DIRTY.with(|dirty| dirty.set(true));
    }
    changed
}
pub fn take_ui_refresh() -> bool {
    UI_DIRTY.with(|dirty| dirty.replace(false))
}
pub fn clear() {
    REGISTRY.with(|cell| cell.borrow_mut().clear());
    RESET_PENDING.with(|pending| pending.set(true));
}
