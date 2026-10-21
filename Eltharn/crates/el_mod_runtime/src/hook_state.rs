use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const CAPABILITY: &str = el_extension_api::services::CLIENT_HOOK_STATE_WRITE_CAPABILITY;
pub const OPCODE: i32 = el_extension_api::services::CLIENT_HOOK_STATE_WRITE_OPCODE;
pub const MAX_BYTES: usize = 4096;
pub const MAX_NODES: usize = 256;
pub const MAX_DEPTH: usize = 8;

pub fn validate(value: &Value) -> Result<(), String> {
    fn visit(value: &Value, depth: usize, nodes: &mut usize) -> Result<(), String> {
        *nodes += 1;
        if depth > MAX_DEPTH || *nodes > MAX_NODES {
            return Err("Hook state nesting/node limit".into());
        }
        match value {
            Value::String(s) if s.len() > 256 => return Err("Hook state string limit".into()),
            Value::Number(n) => {
                let x = n.as_f64().ok_or("Hook state number")?;
                if !x.is_finite() || (n.is_i64() || n.is_u64()) && x.abs() > 9007199254740991.0 {
                    return Err("Hook state portable number limit".into());
                }
            }
            Value::Array(a) => {
                for item in a {
                    visit(item, depth + 1, nodes)?;
                }
            }
            Value::Object(map) => {
                for (key, item) in map {
                    if key.is_empty() || key.len() > 64 || key.chars().any(char::is_control) {
                        return Err("Hook state key limit".into());
                    }
                    visit(item, depth + 1, nodes)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    if !value.is_object() {
        return Err("Hook state must be an object".into());
    }
    visit(value, 0, &mut 0)?;
    if serde_json::to_vec(value).map_err(|e| e.to_string())?.len() > MAX_BYTES {
        return Err("Hook state byte limit".into());
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub revision: u64,
    pub epoch: u64,
    pub value: Value,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub records: BTreeMap<String, Record>,
}

#[derive(Clone, Default)]
pub(crate) struct Store {
    report: Report,
    revision: u64,
}
impl Store {
    pub fn publish(&mut self, owner: &str, epoch: u64, value: &Value) -> Result<(), String> {
        validate(value)?;
        if self
            .report
            .records
            .get(owner)
            .is_some_and(|old| old.epoch == epoch && old.value == *value)
        {
            return Ok(());
        }
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or("Hook state revision overflow")?;
        self.report.records.insert(
            owner.into(),
            Record {
                revision: self.revision,
                epoch,
                value: value.clone(),
            },
        );
        Ok(())
    }
    pub fn clear(&mut self) {
        self.report.records.clear();
    }
    pub fn remove(&mut self, owner: &str) {
        self.report.records.remove(owner);
    }
    pub fn report(&self) -> Report {
        self.report.clone()
    }
}
