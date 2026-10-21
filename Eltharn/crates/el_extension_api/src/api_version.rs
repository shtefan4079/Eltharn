use crate::{Error, ErrorKind};
use core::fmt;
use core::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
const API_TY: &str = "ApiVersion";
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiVersion {
    major: u16,
    minor: u16,
}
fn parse_part(input: &str, part: &str, offset: usize) -> Result<u16, Error> {
    if part.is_empty() {
        return Err(Error::new(API_TY, ErrorKind::EmptyPart, input));
    }
    if part.len() > 3 {
        return Err(Error::new(API_TY, ErrorKind::NumberOutOfRange, input));
    }
    if part.len() > 1 && part.starts_with('0') {
        return Err(Error::at(API_TY, ErrorKind::LeadingZero, input, offset));
    }
    for (i, b) in part.bytes().enumerate() {
        if !b.is_ascii_digit() {
            return Err(Error::at(API_TY, ErrorKind::BadChar, input, offset + i));
        }
    }
    Ok(part.parse().expect("digits checked"))
}
fn parse_api(input: &str) -> Result<(u16, u16), Error> {
    if input.is_empty() {
        return Err(Error::new(API_TY, ErrorKind::Empty, input));
    }
    if input.len() > 7 {
        return Err(Error::new(API_TY, ErrorKind::TooLong, input));
    }
    let Some(dot) = input.find('.') else {
        return Err(Error::new(API_TY, ErrorKind::MissingSeparator, input));
    };
    if input[dot + 1..].contains('.') {
        return Err(Error::new(API_TY, ErrorKind::ExtraSeparator, input));
    }
    let major = parse_part(input, &input[..dot], 0)?;
    let minor = parse_part(input, &input[dot + 1..], dot + 1)?;
    Ok((major, minor))
}
impl ApiVersion {
    pub fn major(&self) -> u16 {
        self.major
    }
    pub fn minor(&self) -> u16 {
        self.minor
    }
    pub fn supports(&self, required: &ApiVersion) -> bool {
        self.major == required.major && self.minor >= required.minor
    }
}
impl FromStr for ApiVersion {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (major, minor) = parse_api(s)?;
        Ok(Self { major, minor })
    }
}
impl TryFrom<&str> for ApiVersion {
    type Error = Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}
impl Serialize for ApiVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
impl<'de> Deserialize<'de> for ApiVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        crate::de::from_str_deserializer(deserializer)
    }
}
const ABI_TY: &str = "AbiVersion";
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AbiVersion(u32);
impl AbiVersion {
    pub fn new(value: u32) -> Result<Self, Error> {
        if value == 0 {
            return Err(Error::new(ABI_TY, ErrorKind::Zero, "0"));
        }
        Ok(Self(value))
    }
    pub fn get(&self) -> u32 {
        self.0
    }
    pub fn supports(&self, required: &AbiVersion) -> bool {
        self.0 == required.0
    }
}
impl FromStr for AbiVersion {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(Error::new(ABI_TY, ErrorKind::Empty, s));
        }
        if s.len() > 16 {
            return Err(Error::new(ABI_TY, ErrorKind::TooLong, s));
        }
        for (i, b) in s.bytes().enumerate() {
            if !b.is_ascii_digit() {
                return Err(Error::at(ABI_TY, ErrorKind::BadChar, s, i));
            }
        }
        match s.parse::<u32>() {
            Ok(0) => Err(Error::new(ABI_TY, ErrorKind::Zero, s)),
            Ok(v) => Ok(Self(v)),
            Err(_) => Err(Error::new(ABI_TY, ErrorKind::NumberOutOfRange, s)),
        }
    }
}
impl TryFrom<&str> for AbiVersion {
    type Error = Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl TryFrom<u32> for AbiVersion {
    type Error = Error;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl fmt::Display for AbiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl Serialize for AbiVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.0)
    }
}
struct AbiVisitor;
impl<'de> de::Visitor<'de> for AbiVisitor {
    type Value = AbiVersion;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a positive u32 ABI version number")
    }
    fn visit_u8<E: de::Error>(self, v: u8) -> Result<Self::Value, E> {
        self.visit_u32(u32::from(v))
    }
    fn visit_u16<E: de::Error>(self, v: u16) -> Result<Self::Value, E> {
        self.visit_u32(u32::from(v))
    }
    fn visit_u32<E: de::Error>(self, v: u32) -> Result<Self::Value, E> {
        AbiVersion::new(v)
            .map_err(|_| E::custom(Error::new(ABI_TY, ErrorKind::Zero, &v.to_string())))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        match u32::try_from(v) {
            Ok(n) => self.visit_u32(n),
            Err(_) => Err(E::custom(Error::new(
                ABI_TY,
                ErrorKind::NumberOutOfRange,
                &v.to_string(),
            ))),
        }
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        if v < 1 {
            return Err(E::custom(Error::new(
                ABI_TY,
                if v == 0 {
                    ErrorKind::Zero
                } else {
                    ErrorKind::NumberOutOfRange
                },
                &v.to_string(),
            )));
        }
        self.visit_u64(v as u64)
    }
}
impl<'de> Deserialize<'de> for AbiVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_u64(AbiVisitor)
    }
}
