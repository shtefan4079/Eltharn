use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const MAX_BYTES: usize = 32 * 1024;
pub const MAX_VALUE_BYTES: usize = 8 * 1024;
pub const MAX_REVISION: u64 = 9_007_199_254_740_991;
pub const READ_CAP: &str = "config.read";
pub const WRITE_CAP: &str = "config.write";

pub fn owner(key: &str) -> Result<(), String> {
    let (id, side) = key
        .split_once('/')
        .ok_or("Configuration owner must be a stable mod ID and side")?;
    let id = id
        .parse::<el_extension_api::ExtensionId>()
        .map_err(|e| e.to_string())?;
    if id.is_reserved_namespace()
        || !matches!(
            side,
            "client"
                | "server"
                | "shared"
                | "client/client"
                | "client/server"
                | "server/client"
                | "server/server"
                | "shared/client"
                | "shared/server"
        )
    {
        return Err("Invalid configuration owner".into());
    }
    Ok(())
}
pub fn storage_owner(actor: &str, side: u32) -> Result<String, String> {
    owner(actor)?;
    if actor.matches('/').count() != 1 {
        return Err("Expected an actor key".into());
    }
    let side = match side {
        1 => "client",
        2 => "server",
        _ => return Err("Invalid runtime side".into()),
    };
    Ok(format!("{actor}/{side}"))
}
pub fn storage_key(owner_key: &str) -> Result<(), String> {
    owner(owner_key)?;
    if owner_key.matches('/').count() != 2 {
        return Err("Configuration store needs a runtime-side namespace".into());
    }
    Ok(())
}
pub fn key(key: &str) -> Result<(), String> {
    if key.is_empty()
        || key.len() > 64
        || !key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
    {
        return Err("Configuration keys require 1–64 ASCII letters, digits, _, - or .".into());
    }
    Ok(())
}
pub fn value(value: &Value) -> Result<(), String> {
    let mut pending = vec![(value, 0)];
    let mut count = 0;
    while let Some((v, depth)) = pending.pop() {
        count += 1;
        if count > 128 || depth > 8 {
            return Err("Configuration value nesting/element limit".into());
        }
        match v {
            Value::Array(a) => {
                if a.len() > 128 {
                    return Err("Configuration array limit".into());
                }
                pending.extend(a.iter().map(|v| (v, depth + 1)));
            }
            Value::Object(m) => {
                if m.len() > 64 {
                    return Err("Configuration map limit".into());
                }
                for (k, v) in m {
                    key(k)?;
                    pending.push((v, depth + 1));
                }
            }
            Value::String(s) if s.len() > MAX_VALUE_BYTES => {
                return Err("Configuration string limit".into());
            }
            Value::Number(n) if !n.as_f64().is_some_and(f64::is_finite) => {
                return Err("Configuration number must be finite".into());
            }
            Value::Number(n)
                if n.as_i64().is_some_and(|v| v.unsigned_abs() > MAX_REVISION)
                    || n.as_u64().is_some_and(|v| v > MAX_REVISION) =>
            {
                return Err("Configuration integer exceeds portable JSON precision".into());
            }
            _ => {}
        }
    }
    if serde_json::to_vec(value).map_err(|e| e.to_string())?.len() > MAX_VALUE_BYTES {
        return Err("Configuration value exceeds 8 KiB".into());
    }
    Ok(())
}
fn canonical(value: &Value) -> Value {
    match value {
        Value::Number(n)
            if n.as_f64().is_some_and(|v| {
                v.fract() == 0.0
                    && v.abs() <= MAX_REVISION as f64
                    && !(v == 0.0 && v.is_sign_negative())
            }) =>
        {
            Value::from(n.as_f64().unwrap() as i64)
        }
        Value::Array(a) => Value::Array(a.iter().map(canonical).collect()),
        Value::Object(m) => {
            Value::Object(m.iter().map(|(k, v)| (k.clone(), canonical(v))).collect())
        }
        _ => value.clone(),
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub revision: u64,
    pub values: BTreeMap<String, Value>,
}
impl Record {
    pub fn validate(&self) -> Result<(), String> {
        if self.revision > MAX_REVISION || self.values.len() > 64 {
            return Err("Configuration record limit".into());
        }
        for (k, v) in &self.values {
            key(k)?;
            value(v)?;
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > MAX_BYTES {
            return Err("Configuration exceeds 32 KiB".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub outcome: String,
    pub revision: u64,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub record: Record,
    pub status: Option<Status>,
}
impl Snapshot {
    pub fn validate(&self) -> Result<(), String> {
        self.record.validate()?;
        if let Some(s) = &self.status {
            if s.revision != self.record.revision
                || !matches!(s.outcome.as_str(), "saved" | "conflict" | "error")
                || s.error.as_ref().is_some_and(|e| e.len() > 512)
            {
                return Err("Invalid configuration status".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Change {
    Set { key: String, value: Value },
    Delete { key: String },
}
impl Change {
    pub fn apply(&self, record: &mut Record) -> Result<(), String> {
        match self {
            Self::Set { key: k, value: v } => {
                key(k)?;
                value(v)?;
                record.values.insert(k.clone(), canonical(v));
            }
            Self::Delete { key: k } => {
                key(k)?;
                record.values.remove(k);
            }
        }
        record.validate()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Write {
    pub owner: String,
    pub expected: u64,
    pub values: BTreeMap<String, Value>,
}
impl Write {
    pub fn record(&self) -> Result<Record, String> {
        owner(&self.owner)?;
        for (key_name, entry) in &self.values {
            key(key_name)?;
            value(entry)?;
        }
        let record = Record {
            revision: self
                .expected
                .checked_add(1)
                .ok_or("Configuration revision overflow")?,
            values: self
                .values
                .iter()
                .map(|(k, v)| (k.clone(), canonical(v)))
                .collect(),
        };
        record.validate()?;
        Ok(record)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub record: Record,
    pub conflict: bool,
}

pub trait Store: Send {
    fn load(&mut self, owner: &str) -> Result<Record, String>;
    fn compare_and_set(&mut self, write: &Write) -> Result<Receipt, String>;
}

#[derive(Clone, Default)]
pub(crate) struct State {
    records: BTreeMap<String, Snapshot>,
    pending: BTreeMap<String, Write>,
}
impl State {
    pub fn statuses(&self) -> BTreeMap<String, Option<Status>> {
        self.records
            .iter()
            .map(|(owner, snapshot)| (owner.clone(), snapshot.status.clone()))
            .collect()
    }
    pub fn initialize(&mut self, records: BTreeMap<String, Record>) -> Result<(), String> {
        if !self.records.is_empty() || !self.pending.is_empty() || records.len() > 16 {
            return Err("Configuration already initialized or oversized".into());
        }
        for (id, record) in &records {
            owner(id)?;
            record.validate()?;
        }
        self.records = records
            .into_iter()
            .map(|(id, record)| {
                (
                    id,
                    Snapshot {
                        record,
                        status: None,
                    },
                )
            })
            .collect();
        Ok(())
    }
    pub fn snapshot(&self, owner: &str) -> Option<Snapshot> {
        self.records.get(owner).cloned()
    }
    pub fn stage(&mut self, owner: &str, changes: &[Change]) -> Result<(), String> {
        if changes.is_empty() {
            return Ok(());
        }
        if self.pending.contains_key(owner) {
            return Err("Configuration persistence is still pending".into());
        }
        let current = self
            .records
            .get(owner)
            .ok_or("Configuration store unavailable")?;
        let mut record = current.record.clone();
        for change in changes {
            change.apply(&mut record)?;
        }
        if record.values == current.record.values {
            return Ok(());
        }
        let write = Write {
            owner: owner.into(),
            expected: current.record.revision,
            values: record.values,
        };
        write.record()?;
        self.pending.insert(owner.into(), write);
        Ok(())
    }
    pub fn pending(&self) -> Vec<Write> {
        self.pending.values().cloned().collect()
    }
    pub fn acknowledge(
        &mut self,
        write: &Write,
        result: Result<Receipt, String>,
    ) -> Result<(), String> {
        let expected = self
            .pending
            .get(&write.owner)
            .ok_or("Unexpected configuration acknowledgment")?;
        if expected.expected != write.expected || expected.values != write.values {
            return Err("Stale configuration acknowledgment".into());
        }
        let current = self
            .records
            .get_mut(&write.owner)
            .ok_or("Unknown configuration owner")?;
        let (outcome, error) = match result {
            Ok(receipt) => {
                receipt.record.validate()?;
                if receipt.conflict && receipt.record.revision == write.expected {
                    return Err("Conflict receipt did not change the expected revision".into());
                }
                if !receipt.conflict && receipt.record != write.record()? {
                    return Err("Configuration acknowledgment differs from write".into());
                }
                current.record = receipt.record;
                if receipt.conflict {
                    ("conflict", Some("Configuration changed in another session; read the latest value before retrying".into()))
                } else {
                    ("saved", None)
                }
            }
            Err(error) => ("error", Some(error.chars().take(128).collect())),
        };
        current.status = Some(Status {
            outcome: outcome.into(),
            revision: current.record.revision,
            error,
        });
        self.pending.remove(&write.owner);
        Ok(())
    }
}
