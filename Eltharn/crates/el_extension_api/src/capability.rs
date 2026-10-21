use crate::{Error, ErrorKind};
use core::fmt;
use core::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
const TY: &str = "CapabilityName";
const MAX_LEN: usize = 128;
const MAX_SEGMENT: usize = 64;
fn validate(input: &str) -> Result<(), Error> {
    if input.is_empty() {
        return Err(Error::new(TY, ErrorKind::Empty, input));
    }
    if input.len() > MAX_LEN {
        return Err(Error::new(TY, ErrorKind::TooLong, input));
    }
    if !input.contains('.') {
        return Err(Error::new(TY, ErrorKind::MissingSeparator, input));
    }
    let segments = input.split('.').count();
    if segments > 4 {
        return Err(Error::new(TY, ErrorKind::ExtraSeparator, input));
    }
    let mut base = 0usize;
    for segment in input.split('.') {
        if segment.is_empty() {
            return Err(Error::at(TY, ErrorKind::EmptyPart, input, base));
        }
        if segment.len() > MAX_SEGMENT {
            return Err(Error::at(TY, ErrorKind::BadSegment, input, base));
        }
        let bytes = segment.as_bytes();
        if !bytes[0].is_ascii_lowercase() {
            return Err(Error::at(TY, ErrorKind::BadStart, input, base));
        }
        for (i, &b) in bytes.iter().enumerate().skip(1) {
            let ok = b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-';
            if !ok {
                return Err(Error::at(TY, ErrorKind::BadChar, input, base + i));
            }
        }
        base += segment.len() + 1;
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityName(String);
impl CapabilityName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn segment_count(&self) -> usize {
        self.0.split('.').count()
    }
    pub fn segments(&self) -> impl DoubleEndedIterator<Item = &str> + '_ {
        self.0.split('.')
    }
}
impl FromStr for CapabilityName {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate(s)?;
        Ok(Self(s.to_owned()))
    }
}
impl TryFrom<&str> for CapabilityName {
    type Error = Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl fmt::Display for CapabilityName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl Serialize for CapabilityName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}
impl<'de> Deserialize<'de> for CapabilityName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        crate::de::from_str_deserializer(deserializer)
    }
}
