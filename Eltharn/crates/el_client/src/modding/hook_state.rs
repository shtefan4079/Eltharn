use el_mod_runtime::{
    hook_state::{self, Record, Report},
    session::Definition,
};
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};
#[derive(Default)]
struct Sync {
    owners: BTreeSet<String>,
    last: BTreeMap<String, Record>,
    present: BTreeSet<String>,
    epoch: u64,
}
thread_local! {
    static SYNC : RefCell < Sync > = RefCell::new(Sync::default());
}
pub fn configure(definitions: &[Definition]) {
    SYNC.with(|cell| {
        *cell.borrow_mut() = Sync {
            owners: definitions
                .iter()
                .filter(|d| d.grants.iter().any(|g| g == hook_state::CAPABILITY))
                .map(|d| d.key.clone())
                .collect(),
            ..Sync::default()
        }
    });
}
pub fn clear() {
    SYNC.with(|cell| *cell.borrow_mut() = Sync::default());
}
pub fn accept(report: &Value) -> Result<(), String> {
    let Some(value) = report.get("hook_state") else {
        return Ok(());
    };
    let state: Report =
        serde_json::from_value(value.clone()).map_err(|e| format!("Hook state report: {e}"))?;
    let epoch = report
        .get("epoch")
        .and_then(Value::as_u64)
        .ok_or("Missing hook state epoch")?;
    SYNC.with(|cell| {
        let mut sync = cell.borrow_mut();
        if epoch < sync.epoch {
            return Err("Stale hook state report epoch".into());
        }
        if state.records.len() > sync.owners.len() {
            return Err("Hook state owner count".into());
        }
        for (
            owner,
            Record {
                revision,
                epoch: record_epoch,
                value,
            },
        ) in &state.records
        {
            if !sync.owners.contains(owner)
                || !super::hooks::accepts_owner(owner)
                || *revision == 0
                || *record_epoch != epoch
            {
                return Err("Invalid hook state owner/epoch/revision".into());
            }
            hook_state::validate(value)?;
            if let Some(old) = sync.last.get(owner) {
                if *record_epoch < old.epoch || *revision < old.revision {
                    return Err("Stale hook state publication".into());
                }
                if *revision == old.revision && (*record_epoch != old.epoch || *value != old.value)
                {
                    return Err("Reused hook state revision".into());
                }
            }
        }
        let removed: Vec<_> = sync
            .present
            .iter()
            .filter(|owner| !state.records.contains_key(*owner))
            .cloned()
            .collect();
        for owner in removed {
            super::hooks::publish_hook_state(&owner, serde_json::json!({}))?;
        }
        for (owner, record) in &state.records {
            if sync
                .last
                .get(owner)
                .is_none_or(|old| old.epoch != record.epoch || old.revision != record.revision)
            {
                super::hooks::publish_hook_state(owner, record.value.clone())?;
                sync.last.insert(owner.clone(), record.clone());
            }
        }
        sync.present = state.records.keys().cloned().collect();
        sync.epoch = epoch;
        Ok(())
    })
}
