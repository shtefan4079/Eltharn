use core::fmt;
use core::str::FromStr;

use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub const MAX_NAMESPACE_LEN: usize = 64;
pub const MAX_PATH_LEN: usize = 192;
pub const MAX_TOTAL_LEN: usize = 255;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentId {
    inner: String,
    ns_len: u8,
}

fn is_ns_byte(b: u8) -> bool {
    matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'.')
}

fn is_path_byte(b: u8) -> bool {
    matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'.' | b'/')
}

fn validate(s: &str) -> Result<u8, String> {
    if s.is_empty() {
        return Err("empty identifier".to_string());
    }
    if s.len() > MAX_TOTAL_LEN {
        return Err("identifier too long".to_string());
    }
    let Some(colon) = s.find(':') else {
        return Err("missing single colon".to_string());
    };
    if s[colon + 1..].contains(':') {
        return Err("multiple colons".to_string());
    }
    let (ns, path) = (&s[..colon], &s[colon + 1..]);
    if ns.is_empty() || ns.len() > MAX_NAMESPACE_LEN {
        return Err("bad namespace length".to_string());
    }
    if path.is_empty() || path.len() > MAX_PATH_LEN {
        return Err("bad path length".to_string());
    }
    if ns == "." || ns == ".." || ns == "/" {
        return Err("reserved namespace component".to_string());
    }
    for &b in ns.as_bytes() {
        if !is_ns_byte(b) {
            return Err(format!("bad namespace byte: {s}"));
        }
    }
    for &b in path.as_bytes() {
        if !is_path_byte(b) {
            return Err(format!("bad path byte: {s}"));
        }
    }
    for seg in path.split('/') {
        if seg.is_empty() {
            return Err("empty path segment".to_string());
        }
        if seg == "." || seg == ".." {
            return Err("reserved path component".to_string());
        }
    }
    Ok(ns.len() as u8)
}

impl ContentId {
    pub fn as_str(&self) -> &str {
        &self.inner
    }
    pub fn namespace(&self) -> &str {
        &self.inner[..self.ns_len as usize]
    }
    pub fn path(&self) -> &str {
        &self.inner[self.ns_len as usize + 1..]
    }
    pub fn is_reserved_namespace(&self) -> bool {
        let ns = self.namespace();
        ns == "minecraft" || ns == "eltharn"
    }
}

impl FromStr for ContentId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > MAX_TOTAL_LEN {
            return Err("identifier too long".to_string());
        }
        let ns_len = validate(s)?;
        Ok(Self {
            inner: s.to_owned(),
            ns_len,
        })
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for ContentId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

struct IdVisitor;

impl Visitor<'_> for IdVisitor {
    type Value = ContentId;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("namespace:path identifier")
    }
    fn visit_str<E: DeError>(self, v: &str) -> Result<ContentId, E> {
        v.parse::<ContentId>().map_err(E::custom)
    }
    fn visit_string<E: DeError>(self, v: String) -> Result<ContentId, E> {
        v.parse::<ContentId>().map_err(E::custom)
    }
    fn visit_bytes<E: DeError>(self, v: &[u8]) -> Result<ContentId, E> {
        let s = core::str::from_utf8(v).map_err(E::custom)?;
        s.parse::<ContentId>().map_err(E::custom)
    }
    fn visit_borrowed_str<E: DeError>(self, v: &str) -> Result<ContentId, E> {
        v.parse::<ContentId>().map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for ContentId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(IdVisitor)
    }
}
