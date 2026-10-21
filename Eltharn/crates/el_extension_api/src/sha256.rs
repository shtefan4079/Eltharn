use crate::{Error, ErrorKind};
use core::fmt;
use core::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
const TY: &str = "Sha256Digest";
const PREFIX: &str = "sha256:";
const HEX_LEN: usize = 64;
fn hex_val(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}
fn validate(input: &str) -> Result<[u8; 32], Error> {
    if input.is_empty() {
        return Err(Error::new(TY, ErrorKind::Empty, input));
    }
    if input.len() != PREFIX.len() + HEX_LEN {
        return Err(Error::new(TY, ErrorKind::BadLength, input));
    }
    if !input.starts_with(PREFIX) {
        return Err(Error::at(TY, ErrorKind::BadPrefix, input, 0));
    }
    let mut out = [0u8; 32];
    for (i, chunk) in input.as_bytes()[PREFIX.len()..].chunks_exact(2).enumerate() {
        let (Some(hi), Some(lo)) = (hex_val(chunk[0]), hex_val(chunk[1])) else {
            let pos = PREFIX.len() + i * 2 + usize::from(hex_val(chunk[0]).is_some());
            return Err(Error::at(TY, ErrorKind::BadChar, input, pos));
        };
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sha256Digest([u8; 32]);
impl Sha256Digest {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub fn bytes(&self) -> &[u8; 32] {
        &self.0
    }
    pub fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}
impl FromStr for Sha256Digest {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(validate(s)?))
    }
}
impl TryFrom<&str> for Sha256Digest {
    type Error = Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl fmt::Display for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(PREFIX)?;
        for b in &self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}
impl Serialize for Sha256Digest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
impl<'de> Deserialize<'de> for Sha256Digest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        crate::de::from_str_deserializer(deserializer)
    }
}
