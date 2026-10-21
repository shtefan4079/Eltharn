use crate::{Error, ErrorKind};
use core::fmt;
use core::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
pub const RESERVED_NAMESPACES: [&str; 3] = ["minecraft", "eltharn", "rustore"];
const TY: &str = "ExtensionId";
const MAX_PART: usize = 63;
fn check_part(input: &str, part: &str, base: usize) -> Result<(), Error> {
    if part.is_empty() {
        return Err(Error::new(TY, ErrorKind::EmptyPart, input));
    }
    if part.len() > MAX_PART {
        return Err(Error::new(TY, ErrorKind::TooLong, input));
    }
    let bytes = part.as_bytes();
    if !(bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit()) {
        return Err(Error::at(TY, ErrorKind::BadStart, input, base));
    }
    for (i, &b) in bytes.iter().enumerate().skip(1) {
        let ok =
            b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-' || b == b'.';
        if !ok {
            return Err(Error::at(TY, ErrorKind::BadChar, input, base + i));
        }
    }
    Ok(())
}
fn validate(input: &str) -> Result<u8, Error> {
    if input.is_empty() {
        return Err(Error::new(TY, ErrorKind::Empty, input));
    }
    if input.len() > MAX_PART * 2 + 1 {
        return Err(Error::new(TY, ErrorKind::TooLong, input));
    }
    let Some(colon) = input.find(':') else {
        return Err(Error::new(TY, ErrorKind::MissingSeparator, input));
    };
    if input[colon + 1..].contains(':') {
        return Err(Error::new(TY, ErrorKind::ExtraSeparator, input));
    }
    let (ns, name) = (&input[..colon], &input[colon + 1..]);
    check_part(input, ns, 0)?;
    check_part(input, name, colon + 1)?;
    Ok(ns.len() as u8)
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtensionId {
    inner: String,
    ns_len: u8,
}
impl ExtensionId {
    pub fn as_str(&self) -> &str {
        &self.inner
    }
    pub fn namespace(&self) -> &str {
        &self.inner[..self.ns_len as usize]
    }
    pub fn name(&self) -> &str {
        &self.inner[self.ns_len as usize + 1..]
    }
    pub fn is_reserved_namespace(&self) -> bool {
        RESERVED_NAMESPACES.contains(&self.namespace())
    }
}
impl FromStr for ExtensionId {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ns_len = validate(s)?;
        Ok(Self {
            inner: s.to_owned(),
            ns_len,
        })
    }
}
impl TryFrom<&str> for ExtensionId {
    type Error = Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl fmt::Display for ExtensionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl Serialize for ExtensionId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}
impl<'de> Deserialize<'de> for ExtensionId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        crate::de::from_str_deserializer(deserializer)
    }
}
