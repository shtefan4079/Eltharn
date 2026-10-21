use serde::{Deserialize, Serialize};
use std::fmt;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtocolImplementationId(String);
impl ProtocolImplementationId {
    pub fn new(value: impl Into<String>) -> Result<Self, LogicalIdError> {
        let value = value.into();
        validate_logical_id(&value, "protocol implementation")?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn into_inner(self) -> String {
        self.0
    }
}
impl fmt::Display for ProtocolImplementationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl AsRef<str> for ProtocolImplementationId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl TryFrom<String> for ProtocolImplementationId {
    type Error = LogicalIdError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl TryFrom<&str> for ProtocolImplementationId {
    type Error = LogicalIdError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalIdError {
    Empty { label: &'static str },
    TooLong { label: &'static str },
    Unsafe { label: &'static str, value: String },
}
impl fmt::Display for LogicalIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty { label } => write!(formatter, "{label} cannot be empty"),
            Self::TooLong { label } => write!(formatter, "{label} is too long"),
            Self::Unsafe { label, value } => write!(formatter, "unsafe {label}: {value}"),
        }
    }
}
impl std::error::Error for LogicalIdError {}
fn validate_logical_id(value: &str, label: &'static str) -> Result<(), LogicalIdError> {
    if value.trim().is_empty() {
        return Err(LogicalIdError::Empty { label });
    }
    if value.len() > 128 {
        return Err(LogicalIdError::TooLong { label });
    }
    if value.contains("..")
        || value.contains('/')
        || value.contains('\\')
        || value.chars().any(char::is_control)
    {
        return Err(LogicalIdError::Unsafe {
            label,
            value: value.to_string(),
        });
    }
    Ok(())
}
