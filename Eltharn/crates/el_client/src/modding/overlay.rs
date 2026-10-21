use el_mod_runtime::{
    overlay::{self, Report, Segment},
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
    last: BTreeMap<String, (u64, u64)>,
    applied: u64,
    epoch: u64,
}
thread_local! {
    static SYNC : RefCell < Sync > = RefCell::new(Sync::default()); static PENDING :
    RefCell < Option < String >> = const { RefCell::new(None) };
}
pub fn configure(definitions: &[Definition]) {
    SYNC.with(|cell| {
        *cell.borrow_mut() = Sync {
            owners: definitions
                .iter()
                .filter(|d| d.grants.iter().any(|g| g == overlay::CAPABILITY))
                .map(|d| d.key.clone())
                .collect(),
            ..Sync::default()
        }
    });
}
pub fn clear() {
    SYNC.with(|cell| *cell.borrow_mut() = Sync::default());
    PENDING.with(|cell| *cell.borrow_mut() = None);
}
pub fn accept(report: &Value) -> Result<(), String> {
    let Some(value) = report.get("overlay") else {
        return Ok(());
    };
    let state: Report =
        serde_json::from_value(value.clone()).map_err(|e| format!("Overlay report: {e}"))?;
    let epoch = report
        .get("epoch")
        .and_then(Value::as_u64)
        .ok_or("Missing overlay epoch")?;
    SYNC.with(|cell| {
        let mut sync = cell.borrow_mut();
        if epoch < sync.epoch {
            return Err("Stale overlay report epoch".into());
        }
        if state.messages.len() > sync.owners.len() {
            return Err("Overlay owner count".into());
        }
        for (owner, record) in &state.messages {
            if !sync.owners.contains(owner) || record.sequence == 0 || record.epoch != epoch {
                return Err("Invalid overlay owner/epoch/sequence".into());
            }
            overlay::validate(&record.segments)?;
            if let Some((old_epoch, old_sequence)) = sync.last.get(owner) {
                if record.epoch < *old_epoch || record.sequence < *old_sequence {
                    return Err("Stale overlay publication".into());
                }
                if record.sequence == *old_sequence && record.epoch != *old_epoch {
                    return Err("Reused overlay sequence".into());
                }
            }
        }
        let newest = state
            .messages
            .values()
            .max_by_key(|record| record.sequence)
            .filter(|record| record.sequence > sync.applied);
        if let Some(record) = newest {
            let text = segments_to_ansi(&record.segments);
            PENDING.with(|pending| *pending.borrow_mut() = Some(text));
            sync.applied = record.sequence;
        }
        for (owner, record) in &state.messages {
            sync.last
                .insert(owner.clone(), (record.epoch, record.sequence));
        }
        sync.epoch = epoch;
        Ok(())
    })
}
fn segments_to_ansi(segments: &[Segment]) -> String {
    let mut out = String::new();
    for segment in segments {
        let mut styled = false;
        if let Some(rgb) = segment
            .color
            .as_deref()
            .and_then(crate::ui::chat::chat_component::named_text_color)
        {
            out.push_str(&format!(
                "\x1b[38;2;{};{};{}m",
                (rgb >> 16) & 0xFF,
                (rgb >> 8) & 0xFF,
                rgb & 0xFF
            ));
            styled = true;
        }
        for (flag, code) in [
            (segment.bold, 1),
            (segment.italic, 3),
            (segment.underlined, 4),
            (segment.strikethrough, 9),
            (segment.obfuscated, 8),
        ] {
            if flag {
                out.push_str(&format!("\x1b[{code}m"));
                styled = true;
            }
        }
        out.push_str(&segment.text);
        if styled {
            out.push_str("\x1b[m");
        }
    }
    out
}
pub fn take_pending() -> Option<String> {
    PENDING.with(|cell| cell.borrow_mut().take())
}
