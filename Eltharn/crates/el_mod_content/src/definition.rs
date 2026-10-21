use serde::Serialize;
use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};

use core::fmt;

pub const MAX_DEFINITION_BYTES: usize = 65536;
pub const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    canonical: Vec<u8>,
}

struct DepthSeed {
    depth: usize,
}

struct StrictVisitor {
    depth: usize,
}

impl StrictVisitor {
    fn depth_ok<E: serde::de::Error>(&self) -> Result<(), E> {
        if self.depth > MAX_DEPTH {
            return Err(E::custom("depth limit exceeded"));
        }
        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for DepthSeed {
    type Value = serde_json::Value;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_any(StrictVisitor { depth: self.depth })
    }
}

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = serde_json::Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("integer-only JSON value")
    }
    fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
        self.depth_ok()?;
        Ok(serde_json::Value::Bool(v))
    }
    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
        self.depth_ok()?;
        Ok(serde_json::Value::from(v))
    }
    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
        self.depth_ok()?;
        Ok(serde_json::Value::from(v))
    }
    fn visit_f64<E: serde::de::Error>(self, _v: f64) -> Result<Self::Value, E> {
        Err(E::custom("floats rejected"))
    }
    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        self.depth_ok()?;
        Ok(serde_json::Value::String(v.to_owned()))
    }
    fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
        self.depth_ok()?;
        Ok(serde_json::Value::String(v))
    }
    fn visit_borrowed_str<E: serde::de::Error>(self, v: &'de str) -> Result<Self::Value, E> {
        self.depth_ok()?;
        Ok(serde_json::Value::String(v.to_owned()))
    }
    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        self.depth_ok()?;
        Ok(serde_json::Value::Null)
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        self.depth_ok()?;
        Ok(serde_json::Value::Null)
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        DepthSeed { depth: self.depth }.deserialize(d)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        self.depth_ok()?;
        let mut out = Vec::new();
        while let Some(v) = seq.next_element_seed(DepthSeed {
            depth: self.depth + 1,
        })? {
            out.push(v);
        }
        Ok(serde_json::Value::Array(out))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        self.depth_ok()?;
        let mut obj = serde_json::Map::new();
        while let Some(k) = map.next_key::<String>()? {
            if obj.contains_key(&k) {
                return Err(serde::de::Error::custom("duplicate object key"));
            }
            let v = map.next_value_seed(DepthSeed {
                depth: self.depth + 1,
            })?;
            obj.insert(k, v);
        }
        Ok(serde_json::Value::Object(obj))
    }
}

struct Canon<'a>(&'a serde_json::Value);

impl Serialize for Canon<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            serde_json::Value::Null
            | serde_json::Value::Bool(_)
            | serde_json::Value::Number(_)
            | serde_json::Value::String(_) => self.0.serialize(s),
            serde_json::Value::Array(items) => {
                let mut seq = s.serialize_seq(Some(items.len()))?;
                for v in items {
                    seq.serialize_element(&Canon(v))?;
                }
                seq.end()
            }
            serde_json::Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                let mut m = s.serialize_map(Some(map.len()))?;
                for k in keys {
                    m.serialize_entry(k, &Canon(&map[k]))?;
                }
                m.end()
            }
        }
    }
}

fn to_canonical(v: &serde_json::Value) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&Canon(v)).map_err(|e| e.to_string())
}

fn validate_number_tokens(input: &[u8]) -> Result<(), String> {
    let n = input.len();
    let mut i = 0;
    let mut in_str = false;
    while i < n {
        let b = input[i];
        if in_str {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if b == b'"' {
            in_str = true;
            i += 1;
            continue;
        }
        if b == b'-' || b.is_ascii_digit() {
            let start = i;
            i += 1;
            while i < n
                && (input[i].is_ascii_digit()
                    || matches!(input[i], b'.' | b'e' | b'E' | b'+' | b'-'))
            {
                i += 1;
            }
            let s = std::str::from_utf8(&input[start..i])
                .map_err(|_| "invalid number token".to_string())?;
            if s.contains('.') || s.contains('e') || s.contains('E') {
                return Err("non-integer number not allowed".to_string());
            }
            if let Some(rest) = s.strip_prefix('-') {
                if rest.is_empty() || !rest.bytes().all(|c| c.is_ascii_digit()) {
                    return Err("invalid number token".to_string());
                }
                if rest.bytes().all(|c| c == b'0') {
                    return Err("negative zero not allowed".to_string());
                }
                s.parse::<i64>()
                    .map_err(|_| "integer out of range".to_string())?;
            } else {
                if s.bytes().any(|c| !c.is_ascii_digit()) {
                    return Err("invalid number token".to_string());
                }
                s.parse::<u64>()
                    .map_err(|_| "integer out of range".to_string())?;
            }
            continue;
        }
        i += 1;
    }
    Ok(())
}

impl Definition {
    pub fn parse(input: &[u8]) -> Result<Self, String> {
        if input.len() > MAX_DEFINITION_BYTES {
            return Err("definition input too large".to_string());
        }
        if input.is_empty() {
            return Err("empty definition".to_string());
        }
        validate_number_tokens(input)?;
        let mut de = serde_json::Deserializer::from_slice(input);
        let value: serde_json::Value = DepthSeed { depth: 0 }
            .deserialize(&mut de)
            .map_err(|e| e.to_string())?;
        de.end().map_err(|e| e.to_string())?;
        let bytes = to_canonical(&value)?;
        if bytes.len() > MAX_DEFINITION_BYTES {
            return Err("canonical definition too large".to_string());
        }
        Ok(Self { canonical: bytes })
    }

    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }

    pub fn value(&self) -> serde_json::Value {
        let mut de = serde_json::Deserializer::from_slice(&self.canonical);
        let value = DepthSeed { depth: 0 }
            .deserialize(&mut de)
            .expect("validated canonical definition");
        de.end().expect("validated canonical definition");
        value
    }
}
