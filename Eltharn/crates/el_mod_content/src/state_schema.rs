use crate::Definition;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_STATE_PROPERTIES: usize = 16;
pub const MAX_PROPERTY_VALUES: usize = 256;
pub const MAX_PROPERTY_NAME_BYTES: usize = 64;
pub const MAX_STATES_PER_BLOCK: usize = 4096;

fn is_valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_PROPERTY_NAME_BYTES
        && s.bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_'))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateProperty {
    name: String,
    values: Vec<String>,
}

impl StateProperty {
    pub fn new(name: String, values: Vec<String>) -> Result<Self, String> {
        if !is_valid_name(&name) {
            return Err("invalid property name".to_string());
        }
        if values.len() < 2 || values.len() > MAX_PROPERTY_VALUES {
            return Err(format!(
                "values len out of 2..={}: {}",
                MAX_PROPERTY_VALUES,
                values.len()
            ));
        }
        let mut seen = BTreeSet::new();
        for v in &values {
            if !is_valid_name(v) {
                return Err("invalid value name".to_string());
            }
            if !seen.insert(v) {
                return Err(format!("duplicate value: {v:?}"));
            }
        }
        Ok(Self { name, values })
    }

    pub fn boolean(name: impl Into<String>) -> Result<Self, String> {
        Self::new(name.into(), vec!["true".to_string(), "false".to_string()])
    }

    pub fn integer(name: impl Into<String>, min: i32, max: i32) -> Result<Self, String> {
        if min < 0 {
            return Err(format!("min must be >= 0, got {min}"));
        }
        if max <= min {
            return Err(format!("max must be > min ({min}), got {max}"));
        }
        let count = max as i64 - min as i64 + 1;
        if count > MAX_PROPERTY_VALUES as i64 {
            return Err(format!("integer range too large: {count}"));
        }
        let values: Vec<String> = (min..=max).map(|v| v.to_string()).collect();
        Self::new(name.into(), values)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn values(&self) -> &[String] {
        &self.values
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateSchema {
    properties: Vec<StateProperty>,
    defaults: BTreeMap<String, String>,
    state_count: usize,
}

impl StateSchema {
    pub fn new(
        mut properties: Vec<StateProperty>,
        defaults: Vec<(String, String)>,
    ) -> Result<Self, String> {
        if properties.len() > MAX_STATE_PROPERTIES {
            return Err(format!("too many properties: {}", properties.len()));
        }
        if defaults.len() != properties.len() {
            return Err("defaults must cover every property exactly".to_string());
        }
        let mut seen = BTreeSet::new();
        for p in &properties {
            if !seen.insert(p.name().to_string()) {
                return Err(format!("duplicate property: {:?}", p.name()));
            }
        }
        properties.sort_by(|a, b| a.name().cmp(b.name()));
        let mut count: usize = 1;
        for p in &properties {
            count = count
                .checked_mul(p.values().len())
                .ok_or_else(|| "state count overflow".to_string())?;
            if count > MAX_STATES_PER_BLOCK {
                return Err(format!("too many states: {count}"));
            }
        }
        if properties.is_empty() {
            if !defaults.is_empty() {
                return Err("empty schema requires empty defaults".to_string());
            }
            let schema = Self {
                properties,
                defaults: BTreeMap::new(),
                state_count: 1,
            };
            schema.definition()?;
            return Ok(schema);
        }
        let mut map = BTreeMap::new();
        for (k, v) in defaults {
            if !is_valid_name(&k) || !is_valid_name(&v) {
                return Err("invalid default name or value".to_string());
            }
            if map.contains_key(&k) {
                return Err(format!("duplicate default: {k:?}"));
            }
            map.insert(k, v);
        }
        if map.len() != properties.len() {
            return Err(format!(
                "defaults must cover every property exactly: {} != {}",
                map.len(),
                properties.len()
            ));
        }
        for p in &properties {
            match map.get(p.name()) {
                None => return Err(format!("missing default: {:?}", p.name())),
                Some(v) => {
                    if !p.values().contains(v) {
                        return Err(format!("invalid default {:?}={:?}", p.name(), v));
                    }
                }
            }
        }
        for k in map.keys() {
            if !properties.iter().any(|p| p.name() == k) {
                return Err(format!("unknown default: {k:?}"));
            }
        }
        let schema = Self {
            properties,
            defaults: map,
            state_count: count,
        };
        schema.definition()?;
        Ok(schema)
    }

    pub fn properties(&self) -> &[StateProperty] {
        &self.properties
    }

    pub fn defaults(&self) -> &BTreeMap<String, String> {
        &self.defaults
    }

    pub fn state_count(&self) -> usize {
        self.state_count
    }

    pub fn validate_values(&self, values: &BTreeMap<String, String>) -> Result<(), String> {
        if values.len() != self.properties.len() {
            return Err(format!(
                "need {} values, got {}",
                self.properties.len(),
                values.len()
            ));
        }
        for p in &self.properties {
            match values.get(p.name()) {
                None => return Err(format!("missing value: {:?}", p.name())),
                Some(v) => {
                    if !p.values().contains(v) {
                        return Err(format!("invalid value for property {:?}", p.name()));
                    }
                }
            }
        }
        for k in values.keys() {
            if !self.properties.iter().any(|p| p.name() == k) {
                return Err(format!("unknown property: {k:?}"));
            }
        }
        Ok(())
    }

    pub fn definition(&self) -> Result<Definition, String> {
        let mut props = serde_json::Map::new();
        for p in &self.properties {
            let arr: Vec<serde_json::Value> = p
                .values()
                .iter()
                .map(|v| serde_json::Value::String(v.clone()))
                .collect();
            props.insert(p.name().to_string(), serde_json::Value::Array(arr));
        }
        let mut def = serde_json::Map::new();
        for (k, v) in &self.defaults {
            def.insert(k.clone(), serde_json::Value::String(v.clone()));
        }
        let obj = serde_json::json!({"properties": props, "default": def});
        let bytes = serde_json::to_vec(&obj).map_err(|e| format!("{e:?}"))?;
        Definition::parse(&bytes).map_err(|e| format!("{e:?}"))
    }
}
