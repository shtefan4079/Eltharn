use crate::{Error, ErrorKind};
use core::cmp::Ordering;
use core::fmt;
use core::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
const TY: &str = "PackageVersion";
const MAX_LEN: usize = 128;
const MAX_CORE: u64 = 999_999;
fn validate(input: &str) -> Result<semver::Version, Error> {
    if input.is_empty() {
        return Err(Error::new(TY, ErrorKind::Empty, input));
    }
    if input.len() > MAX_LEN {
        return Err(Error::new(TY, ErrorKind::TooLong, input));
    }
    if input.as_bytes()[0] == b'v' || input.as_bytes()[0] == b'V' {
        return Err(Error::at(TY, ErrorKind::BadPrefix, input, 0));
    }
    let version =
        semver::Version::parse(input).map_err(|_| Error::new(TY, ErrorKind::BadVersion, input))?;
    if version.major > MAX_CORE || version.minor > MAX_CORE || version.patch > MAX_CORE {
        return Err(Error::new(TY, ErrorKind::NumberOutOfRange, input));
    }
    Ok(version)
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageVersion {
    inner: semver::Version,
}
impl PackageVersion {
    pub fn as_semver(&self) -> &semver::Version {
        &self.inner
    }
    pub fn into_inner(self) -> semver::Version {
        self.inner
    }
    pub fn major(&self) -> u64 {
        self.inner.major
    }
    pub fn minor(&self) -> u64 {
        self.inner.minor
    }
    pub fn patch(&self) -> u64 {
        self.inner.patch
    }
    pub fn pre(&self) -> &semver::Prerelease {
        &self.inner.pre
    }
    pub fn build(&self) -> &semver::BuildMetadata {
        &self.inner.build
    }
    pub fn cmp_precedence(&self, other: &Self) -> Ordering {
        self.inner.cmp_precedence(&other.inner)
    }
    pub fn precedence_eq(&self, other: &Self) -> bool {
        self.cmp_precedence(other) == Ordering::Equal
    }
}
impl FromStr for PackageVersion {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            inner: validate(s)?,
        })
    }
}
impl TryFrom<&str> for PackageVersion {
    type Error = Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl fmt::Display for PackageVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}
impl Serialize for PackageVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
impl<'de> Deserialize<'de> for PackageVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        crate::de::from_str_deserializer(deserializer)
    }
}
