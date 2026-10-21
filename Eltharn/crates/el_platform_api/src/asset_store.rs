use crate::{
    PlatformError, PlatformErrorKind, PlatformFuture, PlatformResult, StoreKey, VirtualPath,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;
const SHA256_PREFIX: &str = "sha256:";
const SHA256_HEX_BYTES: usize = 64;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetObjectId(String);
impl AssetObjectId {
    pub fn parse(value: impl Into<String>) -> PlatformResult<Self> {
        let value = value.into();
        validate_sha256_identifier(&value, "asset object identifier")?;
        Ok(Self(value))
    }
    pub fn from_sha256_bytes(bytes: [u8; 32]) -> Self {
        Self(format_sha256(bytes))
    }
    pub fn sha256_of(bytes: impl AsRef<[u8]>) -> Self {
        let digest: [u8; 32] = Sha256::digest(bytes.as_ref()).into();
        Self::from_sha256_bytes(digest)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for AssetObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetReleaseId(String);
impl AssetReleaseId {
    pub fn parse(value: impl Into<String>) -> PlatformResult<Self> {
        let value = value.into();
        validate_sha256_identifier(&value, "asset release identifier")?;
        Ok(Self(value))
    }
    pub fn from_sha256_bytes(bytes: [u8; 32]) -> Self {
        Self(format_sha256(bytes))
    }
    pub fn sha256_of(bytes: impl AsRef<[u8]>) -> Self {
        let digest: [u8; 32] = Sha256::digest(bytes.as_ref()).into();
        Self::from_sha256_bytes(digest)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for AssetReleaseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetStagingId(StoreKey);
impl AssetStagingId {
    pub fn new(value: impl Into<String>) -> PlatformResult<Self> {
        StoreKey::new(value)
            .map(Self)
            .map_err(|error| invalid_input("asset staging identifier", error.to_string()))
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl fmt::Display for AssetStagingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserPackNamespaceId(StoreKey);
impl UserPackNamespaceId {
    pub fn new(value: impl Into<String>) -> PlatformResult<Self> {
        StoreKey::new(value)
            .map(Self)
            .map_err(|error| invalid_input("user-pack namespace identifier", error.to_string()))
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl fmt::Display for UserPackNamespaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AssetNamespace {
    Managed,
    UserPack(UserPackNamespaceId),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetReleaseSnapshotKey {
    pub namespace: AssetNamespace,
    pub release_id: AssetReleaseId,
}
impl AssetReleaseSnapshotKey {
    pub fn managed(release_id: AssetReleaseId) -> Self {
        Self {
            namespace: AssetNamespace::Managed,
            release_id,
        }
    }
    pub fn user_pack(namespace: UserPackNamespaceId, release_id: AssetReleaseId) -> Self {
        Self {
            namespace: AssetNamespace::UserPack(namespace),
            release_id,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedAssetObject {
    pub object_id: AssetObjectId,
    pub byte_len: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetStagingRequest {
    pub object_id: AssetObjectId,
    pub expected_byte_len: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetStagingState {
    pub staging_id: AssetStagingId,
    pub object_id: AssetObjectId,
    pub expected_byte_len: u64,
    pub staged_byte_len: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetStagingOpen {
    Verified(VerifiedAssetObject),
    Staging(AssetStagingState),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssetReleaseEntry {
    pub path: VirtualPath,
    pub object_id: AssetObjectId,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetReleaseCommit {
    key: AssetReleaseSnapshotKey,
    entries: Vec<AssetReleaseEntry>,
}
impl AssetReleaseCommit {
    pub fn new(
        key: AssetReleaseSnapshotKey,
        entries: impl IntoIterator<Item = AssetReleaseEntry>,
    ) -> PlatformResult<Self> {
        let entries: Vec<_> = entries.into_iter().collect();
        let mut paths = BTreeSet::new();
        for entry in &entries {
            if !paths.insert(entry.path.clone()) {
                return Err(PlatformError::new(
                    PlatformErrorKind::InvalidInput,
                    format!("release snapshot contains duplicate path {}", entry.path),
                ));
            }
        }
        Ok(Self { key, entries })
    }
    pub fn key(&self) -> &AssetReleaseSnapshotKey {
        &self.key
    }
    pub fn entries(&self) -> &[AssetReleaseEntry] {
        &self.entries
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetReleaseSnapshotInfo {
    pub key: AssetReleaseSnapshotKey,
    pub entry_count: u64,
    pub byte_len: u64,
}
pub trait AssetReleaseSnapshot {
    fn info(&self) -> &AssetReleaseSnapshotInfo;
    fn read(&self, path: &VirtualPath) -> PlatformResult<Option<Vec<u8>>>;
    fn read_shared(&self, path: &VirtualPath) -> PlatformResult<Option<std::sync::Arc<Vec<u8>>>> {
        self.read(path).map(|bytes| bytes.map(std::sync::Arc::new))
    }
    fn read_view(&self, path: &VirtualPath) -> PlatformResult<Option<crate::ResourceBytes>> {
        self.read_shared(path)
            .map(|bytes| bytes.map(crate::ResourceBytes::from_shared))
    }
    fn list_all(&self) -> PlatformResult<Vec<VirtualPath>>;
    fn list(&self, prefix: &VirtualPath) -> PlatformResult<Vec<VirtualPath>>;
}
impl<S: AssetReleaseSnapshot + ?Sized> AssetReleaseSnapshot for Box<S> {
    fn info(&self) -> &AssetReleaseSnapshotInfo {
        (**self).info()
    }
    fn read(&self, path: &VirtualPath) -> PlatformResult<Option<Vec<u8>>> {
        (**self).read(path)
    }
    fn read_shared(&self, path: &VirtualPath) -> PlatformResult<Option<std::sync::Arc<Vec<u8>>>> {
        (**self).read_shared(path)
    }
    fn read_view(&self, path: &VirtualPath) -> PlatformResult<Option<crate::ResourceBytes>> {
        (**self).read_view(path)
    }
    fn list_all(&self) -> PlatformResult<Vec<VirtualPath>> {
        (**self).list_all()
    }
    fn list(&self, prefix: &VirtualPath) -> PlatformResult<Vec<VirtualPath>> {
        (**self).list(prefix)
    }
}
impl<S: AssetReleaseSnapshot + ?Sized> AssetReleaseSnapshot for &S {
    fn info(&self) -> &AssetReleaseSnapshotInfo {
        (**self).info()
    }
    fn read(&self, path: &VirtualPath) -> PlatformResult<Option<Vec<u8>>> {
        (**self).read(path)
    }
    fn read_shared(&self, path: &VirtualPath) -> PlatformResult<Option<std::sync::Arc<Vec<u8>>>> {
        (**self).read_shared(path)
    }
    fn read_view(&self, path: &VirtualPath) -> PlatformResult<Option<crate::ResourceBytes>> {
        (**self).read_view(path)
    }
    fn list_all(&self) -> PlatformResult<Vec<VirtualPath>> {
        (**self).list_all()
    }
    fn list(&self, prefix: &VirtualPath) -> PlatformResult<Vec<VirtualPath>> {
        (**self).list(prefix)
    }
}
#[derive(Debug)]
pub struct ReleaseSnapshotAdapter<S> {
    inner: S,
}
impl<S: AssetReleaseSnapshot> ReleaseSnapshotAdapter<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
    pub fn inner(&self) -> &S {
        &self.inner
    }
    pub fn into_inner(self) -> S {
        self.inner
    }
}
impl<S: AssetReleaseSnapshot> crate::ResourceSnapshot for ReleaseSnapshotAdapter<S> {
    fn read(&self, path: &VirtualPath) -> PlatformResult<Option<Vec<u8>>> {
        self.inner.read(path)
    }
    fn read_shared(&self, path: &VirtualPath) -> PlatformResult<Option<std::sync::Arc<Vec<u8>>>> {
        self.inner.read_shared(path)
    }
    fn read_view(&self, path: &VirtualPath) -> PlatformResult<Option<crate::ResourceBytes>> {
        self.inner.read_view(path)
    }
    fn list(&self, prefix: &VirtualPath) -> PlatformResult<Vec<VirtualPath>> {
        self.inner.list(prefix)
    }
}
pub trait AssetStore {
    fn inspect_verified_object<'a>(
        &'a self,
        object_id: AssetObjectId,
    ) -> PlatformFuture<'a, PlatformResult<Option<VerifiedAssetObject>>>;
    fn resume_staging<'a>(
        &'a self,
        request: AssetStagingRequest,
    ) -> PlatformFuture<'a, PlatformResult<AssetStagingOpen>>;
    fn inspect_staging<'a>(
        &'a self,
        staging_id: AssetStagingId,
    ) -> PlatformFuture<'a, PlatformResult<Option<AssetStagingState>>>;
    fn read_staging_bytes<'a>(
        &'a self,
        staging_id: AssetStagingId,
        offset: u64,
        max_bytes: u32,
    ) -> PlatformFuture<'a, PlatformResult<Vec<u8>>>;
    fn append_staging_bytes<'a>(
        &'a self,
        staging_id: AssetStagingId,
        bytes: Vec<u8>,
    ) -> PlatformFuture<'a, PlatformResult<AssetStagingState>>;
    fn truncate_staging<'a>(
        &'a self,
        staging_id: AssetStagingId,
        byte_len: u64,
    ) -> PlatformFuture<'a, PlatformResult<AssetStagingState>>;
    fn restart_staging<'a>(
        &'a self,
        staging_id: AssetStagingId,
    ) -> PlatformFuture<'a, PlatformResult<AssetStagingState>>;
    fn publish_verified_object<'a>(
        &'a self,
        staging_id: AssetStagingId,
    ) -> PlatformFuture<'a, PlatformResult<VerifiedAssetObject>>;
    fn commit_release_snapshot<'a>(
        &'a self,
        commit: AssetReleaseCommit,
    ) -> PlatformFuture<'a, PlatformResult<AssetReleaseSnapshotInfo>>;
    fn open_release_snapshot<'a>(
        &'a self,
        key: AssetReleaseSnapshotKey,
    ) -> PlatformFuture<'a, PlatformResult<Option<Box<dyn AssetReleaseSnapshot>>>>;
}
fn format_sha256(bytes: [u8; 32]) -> String {
    let mut result = String::with_capacity(SHA256_PREFIX.len() + SHA256_HEX_BYTES);
    result.push_str(SHA256_PREFIX);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(result, "{byte:02x}");
    }
    result
}
fn validate_sha256_identifier(value: &str, label: &str) -> PlatformResult<()> {
    let Some(hex) = value.strip_prefix(SHA256_PREFIX) else {
        return Err(invalid_input(
            label,
            "must use the sha256:<64 lowercase hexadecimal digits> form",
        ));
    };
    if hex.len() != SHA256_HEX_BYTES
        || !hex
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(invalid_input(
            label,
            "must use the sha256:<64 lowercase hexadecimal digits> form",
        ));
    }
    Ok(())
}
fn invalid_input(subject: &str, reason: impl fmt::Display) -> PlatformError {
    PlatformError::new(
        PlatformErrorKind::InvalidInput,
        format!("{subject} {reason}"),
    )
}
