use std::fmt;
const MAX_STORE_KEY_BYTES: usize = 512;
const MAX_SECRET_KEY_BYTES: usize = 512;
const MAX_VIRTUAL_PATH_BYTES: usize = 4096;
const MAX_LOCATION_ID_BYTES: usize = 512;
const MAX_FILE_NAME_BYTES: usize = 255;
const MAX_HTTP_URL_BYTES: usize = 8192;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierError {
    Empty,
    TooLong,
    ContainsControlCharacter,
    ContainsBackslash,
    AbsolutePath,
    Traversal,
    EmptyPathSegment,
    InvalidCharacter,
    InvalidHttpUrl,
}
impl fmt::Display for IdentifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Self::Empty => "cannot be empty",
            Self::TooLong => "is too long",
            Self::ContainsControlCharacter => "contains a control character",
            Self::ContainsBackslash => "contains a backslash",
            Self::AbsolutePath => "must not be an absolute path",
            Self::Traversal => "must not contain a traversal segment",
            Self::EmptyPathSegment => "contains an empty path segment",
            Self::InvalidCharacter => "contains an invalid character",
            Self::InvalidHttpUrl => "is not a supported HTTP URL",
        };
        f.write_str(reason)
    }
}
impl std::error::Error for IdentifierError {}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StoreKey(String);
impl StoreKey {
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        let value = value.into();
        validate_key(&value, MAX_STORE_KEY_BYTES)?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for StoreKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SecretKey(String);
impl SecretKey {
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        let value = value.into();
        validate_key(&value, MAX_SECRET_KEY_BYTES)?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
#[derive(Clone, PartialEq, Eq, Default)]
pub struct SecretBytes(Vec<u8>);
impl SecretBytes {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}
impl fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretBytes(<redacted; {} bytes>)", self.0.len())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtualPath(String);
impl VirtualPath {
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        let value = value.into();
        validate_virtual_path(&value)?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn starts_with(&self, prefix: &VirtualPath) -> bool {
        self.0 == prefix.0
            || self
                .0
                .strip_prefix(prefix.as_str())
                .is_some_and(|suffix| suffix.starts_with('/'))
    }
}
impl fmt::Display for VirtualPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StoreLocationId(String);
impl StoreLocationId {
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        let value = value.into();
        validate_key(&value, MAX_LOCATION_ID_BYTES)?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for StoreLocationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SafeFileName(String);
impl SafeFileName {
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        let value = value.into();
        if value.is_empty() {
            return Err(IdentifierError::Empty);
        }
        if value.len() > MAX_FILE_NAME_BYTES {
            return Err(IdentifierError::TooLong);
        }
        if value.chars().any(char::is_control) {
            return Err(IdentifierError::ContainsControlCharacter);
        }
        if value.contains('/') || value.contains('\\') {
            return Err(IdentifierError::InvalidCharacter);
        }
        if matches!(value.as_str(), "." | "..") {
            return Err(IdentifierError::Traversal);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for SafeFileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HttpUrl(String);
impl HttpUrl {
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        let value = value.into();
        validate_http_url(&value)?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn redacted(&self) -> &str {
        self.0
            .split_once(['?', '#'])
            .map_or(self.0.as_str(), |(base, _)| base)
    }
}
impl fmt::Debug for HttpUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HttpUrl({})", self.redacted())
    }
}
impl fmt::Display for HttpUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.redacted())
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct HttpHeader {
    name: String,
    value: String,
}
impl HttpHeader {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Result<Self, IdentifierError> {
        let name = name.into();
        let value = value.into();
        if name.is_empty() || !name.as_bytes().iter().copied().all(is_http_token_byte) {
            return Err(IdentifierError::InvalidCharacter);
        }
        if value.chars().any(|character| {
            character == '\r' || character == '\n' || character.is_control() && character != '\t'
        }) {
            return Err(IdentifierError::ContainsControlCharacter);
        }
        Ok(Self { name, value })
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn value(&self) -> &str {
        &self.value
    }
    pub fn is_sensitive(&self) -> bool {
        matches!(
            self.name.to_ascii_lowercase().as_str(),
            "authorization" | "cookie" | "set-cookie" | "proxy-authorization" | "x-api-key"
        )
    }
}
impl fmt::Debug for HttpHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = if self.is_sensitive() {
            "<redacted>"
        } else {
            self.value.as_str()
        };
        f.debug_struct("HttpHeader")
            .field("name", &self.name)
            .field("value", &value)
            .finish()
    }
}
fn validate_key(value: &str, max_bytes: usize) -> Result<(), IdentifierError> {
    if value.is_empty() {
        return Err(IdentifierError::Empty);
    }
    if value.len() > max_bytes {
        return Err(IdentifierError::TooLong);
    }
    if value.chars().any(char::is_control) {
        return Err(IdentifierError::ContainsControlCharacter);
    }
    if value.contains('\\') {
        return Err(IdentifierError::ContainsBackslash);
    }
    if value.starts_with('/') {
        return Err(IdentifierError::AbsolutePath);
    }
    if value.contains("..") {
        return Err(IdentifierError::Traversal);
    }
    if value.split('/').any(|segment| segment.is_empty()) {
        return Err(IdentifierError::EmptyPathSegment);
    }
    if value.split('/').any(|segment| segment == ".") {
        return Err(IdentifierError::Traversal);
    }
    if value
        .bytes()
        .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/')))
    {
        return Err(IdentifierError::InvalidCharacter);
    }
    Ok(())
}
fn validate_virtual_path(value: &str) -> Result<(), IdentifierError> {
    if value.is_empty() {
        return Err(IdentifierError::Empty);
    }
    if value.len() > MAX_VIRTUAL_PATH_BYTES {
        return Err(IdentifierError::TooLong);
    }
    if value.chars().any(char::is_control) {
        return Err(IdentifierError::ContainsControlCharacter);
    }
    if value.contains('\\') {
        return Err(IdentifierError::ContainsBackslash);
    }
    if value.starts_with('/') {
        return Err(IdentifierError::AbsolutePath);
    }
    if value.split('/').any(|segment| segment.is_empty()) {
        return Err(IdentifierError::EmptyPathSegment);
    }
    if value
        .split('/')
        .any(|segment| segment == "." || segment == "..")
    {
        return Err(IdentifierError::Traversal);
    }
    Ok(())
}
fn validate_http_url(value: &str) -> Result<(), IdentifierError> {
    if value.is_empty() {
        return Err(IdentifierError::Empty);
    }
    if value.len() > MAX_HTTP_URL_BYTES {
        return Err(IdentifierError::TooLong);
    }
    if value
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(IdentifierError::InvalidHttpUrl);
    }
    let Some((scheme, remainder)) = value.split_once("://") else {
        return Err(IdentifierError::InvalidHttpUrl);
    };
    if !matches!(scheme, "http" | "https") {
        return Err(IdentifierError::InvalidHttpUrl);
    }
    let authority = remainder
        .split(['/', '?', '#'])
        .next()
        .filter(|authority| !authority.is_empty())
        .ok_or(IdentifierError::InvalidHttpUrl)?;
    if authority.contains('@') {
        return Err(IdentifierError::InvalidHttpUrl);
    }
    Ok(())
}
const fn is_http_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}
