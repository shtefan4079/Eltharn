use crate::{Error, ErrorKind};
use core::fmt;
use core::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
const TY: &str = "PackagePath";
const MAX_LEN: usize = 256;
pub const MAX_DEPTH: usize = 16;
pub const MANIFEST_FILE_NAME: &str = "eltharn-extension.json";
pub const LEGACY_MANIFEST_FILE_NAME: &str = "rustore-extension.json";
pub fn is_manifest_file_name(name: &str) -> bool {
    name == MANIFEST_FILE_NAME || name == LEGACY_MANIFEST_FILE_NAME
}
fn is_windows_reserved_stem(stem: &str) -> bool {
    match stem {
        "con" | "prn" | "aux" | "nul" => true,
        _ => {
            let b = stem.as_bytes();
            b.len() == 4
                && ((b[0] == b'c' && b[1] == b'o' && b[2] == b'm')
                    || (b[0] == b'l' && b[1] == b'p' && b[2] == b't'))
                && (b[3] >= b'1' && b[3] <= b'9')
        }
    }
}
fn validate(input: &str) -> Result<(), Error> {
    if input.is_empty() {
        return Err(Error::new(TY, ErrorKind::Empty, input));
    }
    if input.len() > MAX_LEN {
        return Err(Error::new(TY, ErrorKind::TooLong, input));
    }
    let bytes = input.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        let ok = b.is_ascii_lowercase()
            || b.is_ascii_digit()
            || b == b'_'
            || b == b'-'
            || b == b'.'
            || b == b'/';
        if !ok {
            return Err(Error::at(TY, ErrorKind::BadChar, input, i));
        }
    }
    if bytes[0] == b'/' {
        return Err(Error::at(TY, ErrorKind::BadStart, input, 0));
    }
    if bytes[bytes.len() - 1] == b'/' {
        return Err(Error::at(TY, ErrorKind::BadEnd, input, bytes.len() - 1));
    }
    if is_manifest_file_name(input) {
        return Err(Error::new(TY, ErrorKind::ForbiddenValue, input));
    }
    let mut base = 0usize;
    for (depth, component) in input.split('/').enumerate() {
        if depth >= MAX_DEPTH {
            return Err(Error::at(TY, ErrorKind::TooDeep, input, base));
        }
        if component.is_empty() {
            return Err(Error::at(TY, ErrorKind::EmptyPart, input, base));
        }
        let cb = component.as_bytes();
        if !(cb[0].is_ascii_lowercase() || cb[0].is_ascii_digit()) {
            return Err(Error::at(TY, ErrorKind::BadStart, input, base));
        }
        if component == "." || component == ".." {
            return Err(Error::at(TY, ErrorKind::BadSegment, input, base));
        }
        if cb[cb.len() - 1] == b'.' {
            return Err(Error::at(TY, ErrorKind::BadEnd, input, base + cb.len() - 1));
        }
        let stem = component.split('.').next().unwrap_or(component);
        if is_windows_reserved_stem(stem) {
            return Err(Error::at(TY, ErrorKind::ForbiddenValue, input, base));
        }
        base += component.len() + 1;
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackagePath(String);
impl PackagePath {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn components(&self) -> impl DoubleEndedIterator<Item = &str> + '_ {
        self.0.split('/')
    }
    pub fn file_name(&self) -> &str {
        self.0.rsplit('/').next().expect("validated non-empty")
    }
}
impl FromStr for PackagePath {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate(s)?;
        Ok(Self(s.to_owned()))
    }
}
impl TryFrom<&str> for PackagePath {
    type Error = Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl fmt::Display for PackagePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl Serialize for PackagePath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}
impl<'de> Deserialize<'de> for PackagePath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        crate::de::from_str_deserializer(deserializer)
    }
}
