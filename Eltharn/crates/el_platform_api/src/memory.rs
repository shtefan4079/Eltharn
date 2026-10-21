use crate::{
    AssetObjectId, AssetReleaseCommit, AssetReleaseSnapshot, AssetReleaseSnapshotInfo,
    AssetReleaseSnapshotKey, AssetStagingId, AssetStagingOpen, AssetStagingRequest,
    AssetStagingState, AssetStore, PersistentStore, PlatformError, PlatformErrorKind,
    PlatformFuture, PlatformResult, ResourceSnapshot, ResourceSnapshotRequest, ResourceStore,
    SecretBytes, SecretKey, SecretStore, StoreKey, VerifiedAssetObject, VirtualPath,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;
#[derive(Debug, Default)]
pub struct MemoryPersistentStore {
    values: RefCell<BTreeMap<StoreKey, Vec<u8>>>,
}
impl MemoryPersistentStore {
    pub fn new() -> Self {
        Self::default()
    }
}
impl PersistentStore for MemoryPersistentStore {
    fn get<'a>(&'a self, key: StoreKey) -> PlatformFuture<'a, PlatformResult<Option<Vec<u8>>>> {
        Box::pin(async move { Ok(self.values.borrow().get(&key).cloned()) })
    }
    fn put<'a>(&'a self, key: StoreKey, value: Vec<u8>) -> PlatformFuture<'a, PlatformResult<()>> {
        Box::pin(async move {
            self.values.borrow_mut().insert(key, value);
            Ok(())
        })
    }
    fn delete<'a>(&'a self, key: StoreKey) -> PlatformFuture<'a, PlatformResult<()>> {
        Box::pin(async move {
            self.values.borrow_mut().remove(&key);
            Ok(())
        })
    }
}
#[derive(Debug, Default)]
pub struct MemorySecretStore {
    values: RefCell<BTreeMap<SecretKey, SecretBytes>>,
}
impl MemorySecretStore {
    pub fn new() -> Self {
        Self::default()
    }
}
impl SecretStore for MemorySecretStore {
    fn get<'a>(
        &'a self,
        key: SecretKey,
    ) -> PlatformFuture<'a, PlatformResult<Option<SecretBytes>>> {
        Box::pin(async move { Ok(self.values.borrow().get(&key).cloned()) })
    }
    fn put<'a>(
        &'a self,
        key: SecretKey,
        value: SecretBytes,
    ) -> PlatformFuture<'a, PlatformResult<()>> {
        Box::pin(async move {
            self.values.borrow_mut().insert(key, value);
            Ok(())
        })
    }
    fn delete<'a>(&'a self, key: SecretKey) -> PlatformFuture<'a, PlatformResult<()>> {
        Box::pin(async move {
            self.values.borrow_mut().remove(&key);
            Ok(())
        })
    }
}
#[derive(Debug, Default)]
pub struct MemoryAssetStore {
    state: RefCell<MemoryAssetStoreState>,
}
#[derive(Debug, Default)]
struct MemoryAssetStoreState {
    verified_objects: BTreeMap<AssetObjectId, Arc<Vec<u8>>>,
    staging_by_id: BTreeMap<AssetStagingId, MemoryStaging>,
    staging_by_object: BTreeMap<AssetObjectId, AssetStagingId>,
    releases: BTreeMap<AssetReleaseSnapshotKey, MemoryAssetRelease>,
    next_staging_number: u64,
}
#[derive(Debug)]
struct MemoryStaging {
    object_id: AssetObjectId,
    expected_byte_len: u64,
    bytes: Vec<u8>,
}
#[derive(Debug, Clone)]
struct MemoryAssetRelease {
    info: AssetReleaseSnapshotInfo,
    object_ids: BTreeMap<VirtualPath, AssetObjectId>,
    bytes: BTreeMap<VirtualPath, Arc<Vec<u8>>>,
}
impl MemoryAssetStore {
    pub fn new() -> Self {
        Self::default()
    }
}
impl AssetStore for MemoryAssetStore {
    fn inspect_verified_object<'a>(
        &'a self,
        object_id: AssetObjectId,
    ) -> PlatformFuture<'a, PlatformResult<Option<VerifiedAssetObject>>> {
        Box::pin(async move {
            Ok(self
                .state
                .borrow()
                .verified_objects
                .get(&object_id)
                .map(|bytes| verified_object(object_id, bytes)))
        })
    }
    fn resume_staging<'a>(
        &'a self,
        request: AssetStagingRequest,
    ) -> PlatformFuture<'a, PlatformResult<AssetStagingOpen>> {
        Box::pin(async move {
            let mut state = self.state.borrow_mut();
            if let Some(bytes) = state.verified_objects.get(&request.object_id) {
                verify_expected_byte_len(
                    &request.object_id,
                    request.expected_byte_len,
                    bytes.len(),
                )?;
                return Ok(AssetStagingOpen::Verified(verified_object(
                    request.object_id,
                    bytes,
                )));
            }
            if let Some(staging_id) = state.staging_by_object.get(&request.object_id).cloned() {
                let staging = state.staging_by_id.get(&staging_id).ok_or_else(|| {
                    PlatformError::new(
                        PlatformErrorKind::Other,
                        "asset staging index referred to a missing staging record",
                    )
                })?;
                if staging.expected_byte_len != request.expected_byte_len {
                    return Err(PlatformError::new(
                        PlatformErrorKind::InvalidInput,
                        format!(
                            "asset staging request for {} changed expected byte length from {} to {}",
                            request.object_id, staging.expected_byte_len, request.expected_byte_len
                        ),
                    ));
                }
                return Ok(AssetStagingOpen::Staging(staging_state(
                    staging_id, staging,
                )));
            }
            state.next_staging_number =
                state.next_staging_number.checked_add(1).ok_or_else(|| {
                    PlatformError::new(
                        PlatformErrorKind::Other,
                        "memory asset staging identifier counter overflowed",
                    )
                })?;
            let staging_id =
                AssetStagingId::new(format!("asset-staging/{}", state.next_staging_number))
                    .expect("generated memory asset staging identifier must be valid");
            let staging = MemoryStaging {
                object_id: request.object_id.clone(),
                expected_byte_len: request.expected_byte_len,
                bytes: Vec::new(),
            };
            let result = staging_state(staging_id.clone(), &staging);
            state
                .staging_by_object
                .insert(request.object_id, staging_id.clone());
            state.staging_by_id.insert(staging_id, staging);
            Ok(AssetStagingOpen::Staging(result))
        })
    }
    fn inspect_staging<'a>(
        &'a self,
        staging_id: AssetStagingId,
    ) -> PlatformFuture<'a, PlatformResult<Option<AssetStagingState>>> {
        Box::pin(async move {
            Ok(self
                .state
                .borrow()
                .staging_by_id
                .get(&staging_id)
                .map(|staging| staging_state(staging_id, staging)))
        })
    }
    fn read_staging_bytes<'a>(
        &'a self,
        staging_id: AssetStagingId,
        offset: u64,
        max_bytes: u32,
    ) -> PlatformFuture<'a, PlatformResult<Vec<u8>>> {
        Box::pin(async move {
            let state = self.state.borrow();
            let staging = state
                .staging_by_id
                .get(&staging_id)
                .ok_or_else(|| missing_staging(&staging_id))?;
            let offset = usize::try_from(offset).map_err(|_| {
                PlatformError::new(
                    PlatformErrorKind::InvalidInput,
                    "asset staging read offset does not fit this platform",
                )
            })?;
            if offset > staging.bytes.len() {
                return Err(PlatformError::new(
                    PlatformErrorKind::InvalidInput,
                    format!(
                        "asset staging read offset {offset} exceeds staged byte length {}",
                        staging.bytes.len()
                    ),
                ));
            }
            let max_bytes = usize::try_from(max_bytes).map_err(|_| {
                PlatformError::new(
                    PlatformErrorKind::InvalidInput,
                    "asset staging read length does not fit this platform",
                )
            })?;
            let end = offset.saturating_add(max_bytes).min(staging.bytes.len());
            Ok(staging.bytes[offset..end].to_vec())
        })
    }
    fn append_staging_bytes<'a>(
        &'a self,
        staging_id: AssetStagingId,
        bytes: Vec<u8>,
    ) -> PlatformFuture<'a, PlatformResult<AssetStagingState>> {
        Box::pin(async move {
            let mut state = self.state.borrow_mut();
            let staging = state
                .staging_by_id
                .get_mut(&staging_id)
                .ok_or_else(|| missing_staging(&staging_id))?;
            let new_byte_len = staging
                .bytes
                .len()
                .checked_add(bytes.len())
                .ok_or_else(|| {
                    PlatformError::new(
                        PlatformErrorKind::InvalidInput,
                        "asset staging append length overflowed",
                    )
                })?;
            let new_byte_len_u64 = u64::try_from(new_byte_len).map_err(|_| {
                PlatformError::new(
                    PlatformErrorKind::InvalidInput,
                    "asset staging append length does not fit u64",
                )
            })?;
            if new_byte_len_u64 > staging.expected_byte_len {
                return Err(PlatformError::new(
                    PlatformErrorKind::InvalidInput,
                    format!(
                        "asset staging append would exceed expected byte length {}",
                        staging.expected_byte_len
                    ),
                ));
            }
            staging.bytes.extend(bytes);
            Ok(staging_state(staging_id, staging))
        })
    }
    fn truncate_staging<'a>(
        &'a self,
        staging_id: AssetStagingId,
        byte_len: u64,
    ) -> PlatformFuture<'a, PlatformResult<AssetStagingState>> {
        Box::pin(async move {
            let mut state = self.state.borrow_mut();
            let staging = state
                .staging_by_id
                .get_mut(&staging_id)
                .ok_or_else(|| missing_staging(&staging_id))?;
            let byte_len = usize::try_from(byte_len).map_err(|_| {
                PlatformError::new(
                    PlatformErrorKind::InvalidInput,
                    "asset staging truncate length does not fit this platform",
                )
            })?;
            if byte_len > staging.bytes.len() {
                return Err(PlatformError::new(
                    PlatformErrorKind::InvalidInput,
                    format!(
                        "asset staging truncate length {byte_len} exceeds staged byte length {}",
                        staging.bytes.len()
                    ),
                ));
            }
            staging.bytes.truncate(byte_len);
            Ok(staging_state(staging_id, staging))
        })
    }
    fn restart_staging<'a>(
        &'a self,
        staging_id: AssetStagingId,
    ) -> PlatformFuture<'a, PlatformResult<AssetStagingState>> {
        Box::pin(async move {
            let mut state = self.state.borrow_mut();
            let staging = state
                .staging_by_id
                .get_mut(&staging_id)
                .ok_or_else(|| missing_staging(&staging_id))?;
            staging.bytes.clear();
            Ok(staging_state(staging_id, staging))
        })
    }
    fn publish_verified_object<'a>(
        &'a self,
        staging_id: AssetStagingId,
    ) -> PlatformFuture<'a, PlatformResult<VerifiedAssetObject>> {
        Box::pin(async move {
            let mut state = self.state.borrow_mut();
            let staging = state
                .staging_by_id
                .get(&staging_id)
                .ok_or_else(|| missing_staging(&staging_id))?;
            let actual_byte_len = u64::try_from(staging.bytes.len()).map_err(|_| {
                PlatformError::new(
                    PlatformErrorKind::Integrity,
                    "asset staging byte length does not fit u64",
                )
            })?;
            if actual_byte_len != staging.expected_byte_len {
                return Err(PlatformError::new(
                    PlatformErrorKind::Integrity,
                    format!(
                        "asset {} has {} staged bytes but manifest requires {}",
                        staging.object_id, actual_byte_len, staging.expected_byte_len
                    ),
                ));
            }
            let actual_object_id = AssetObjectId::sha256_of(&staging.bytes);
            if actual_object_id != staging.object_id {
                return Err(PlatformError::new(
                    PlatformErrorKind::Integrity,
                    format!(
                        "asset staging bytes do not match expected object {}",
                        staging.object_id
                    ),
                ));
            }
            let staging = state
                .staging_by_id
                .remove(&staging_id)
                .expect("staging was checked while this RefCell borrow was held");
            state.staging_by_object.remove(&staging.object_id);
            let bytes = state
                .verified_objects
                .entry(staging.object_id.clone())
                .or_insert_with(|| Arc::new(staging.bytes));
            Ok(verified_object(staging.object_id, bytes))
        })
    }
    fn commit_release_snapshot<'a>(
        &'a self,
        commit: AssetReleaseCommit,
    ) -> PlatformFuture<'a, PlatformResult<AssetReleaseSnapshotInfo>> {
        Box::pin(async move {
            let mut state = self.state.borrow_mut();
            if let Some(existing) = state.releases.get(commit.key()) {
                let desired_object_ids = commit
                    .entries()
                    .iter()
                    .map(|entry| (entry.path.clone(), entry.object_id.clone()))
                    .collect::<BTreeMap<_, _>>();
                if existing.object_ids == desired_object_ids {
                    return Ok(existing.info.clone());
                }
                return Err(PlatformError::new(
                    PlatformErrorKind::Integrity,
                    format!(
                        "asset release snapshot {} is immutable and was already committed",
                        commit.key().release_id
                    ),
                ));
            }
            let mut object_ids = BTreeMap::new();
            let mut bytes = BTreeMap::new();
            let mut byte_len = 0_u64;
            for entry in commit.entries() {
                let object_bytes =
                    state
                        .verified_objects
                        .get(&entry.object_id)
                        .ok_or_else(|| {
                            PlatformError::new(
                                PlatformErrorKind::NotFound,
                                format!(
                                    "asset release references unverified object {}",
                                    entry.object_id
                                ),
                            )
                        })?;
                let object_len = u64::try_from(object_bytes.len()).map_err(|_| {
                    PlatformError::new(
                        PlatformErrorKind::Integrity,
                        "verified object byte length does not fit u64",
                    )
                })?;
                byte_len = byte_len.checked_add(object_len).ok_or_else(|| {
                    PlatformError::new(
                        PlatformErrorKind::Integrity,
                        "asset release aggregate byte length overflowed",
                    )
                })?;
                object_ids.insert(entry.path.clone(), entry.object_id.clone());
                bytes.insert(entry.path.clone(), object_bytes.clone());
            }
            let info = AssetReleaseSnapshotInfo {
                key: commit.key().clone(),
                entry_count: u64::try_from(commit.entries().len()).map_err(|_| {
                    PlatformError::new(
                        PlatformErrorKind::Integrity,
                        "asset release entry count does not fit u64",
                    )
                })?,
                byte_len,
            };
            state.releases.insert(
                commit.key().clone(),
                MemoryAssetRelease {
                    info: info.clone(),
                    object_ids,
                    bytes,
                },
            );
            Ok(info)
        })
    }
    fn open_release_snapshot<'a>(
        &'a self,
        key: AssetReleaseSnapshotKey,
    ) -> PlatformFuture<'a, PlatformResult<Option<Box<dyn AssetReleaseSnapshot>>>> {
        Box::pin(async move {
            Ok(self.state.borrow().releases.get(&key).map(|release| {
                Box::new(MemoryAssetReleaseSnapshot {
                    info: release.info.clone(),
                    bytes: release.bytes.clone(),
                }) as Box<dyn AssetReleaseSnapshot>
            }))
        })
    }
}
#[derive(Debug)]
struct MemoryAssetReleaseSnapshot {
    info: AssetReleaseSnapshotInfo,
    bytes: BTreeMap<VirtualPath, Arc<Vec<u8>>>,
}
impl AssetReleaseSnapshot for MemoryAssetReleaseSnapshot {
    fn info(&self) -> &AssetReleaseSnapshotInfo {
        &self.info
    }
    fn read(&self, path: &VirtualPath) -> PlatformResult<Option<Vec<u8>>> {
        Ok(self.bytes.get(path).map(|bytes| bytes.as_ref().clone()))
    }
    fn read_shared(&self, path: &VirtualPath) -> PlatformResult<Option<Arc<Vec<u8>>>> {
        Ok(self.bytes.get(path).cloned())
    }
    fn list_all(&self) -> PlatformResult<Vec<VirtualPath>> {
        Ok(self.bytes.keys().cloned().collect())
    }
    fn list(&self, prefix: &VirtualPath) -> PlatformResult<Vec<VirtualPath>> {
        Ok(self
            .bytes
            .keys()
            .filter(|path| path.starts_with(prefix))
            .cloned()
            .collect())
    }
}
fn verified_object(object_id: AssetObjectId, bytes: &[u8]) -> VerifiedAssetObject {
    VerifiedAssetObject {
        object_id,
        byte_len: u64::try_from(bytes.len()).expect("slice length must fit u64"),
    }
}
fn staging_state(staging_id: AssetStagingId, staging: &MemoryStaging) -> AssetStagingState {
    AssetStagingState {
        staging_id,
        object_id: staging.object_id.clone(),
        expected_byte_len: staging.expected_byte_len,
        staged_byte_len: u64::try_from(staging.bytes.len())
            .expect("in-memory vector length must fit u64"),
    }
}
fn verify_expected_byte_len(
    object_id: &AssetObjectId,
    expected_byte_len: u64,
    actual_byte_len: usize,
) -> PlatformResult<()> {
    let actual_byte_len = u64::try_from(actual_byte_len).map_err(|_| {
        PlatformError::new(
            PlatformErrorKind::Integrity,
            "verified object byte length does not fit u64",
        )
    })?;
    if actual_byte_len != expected_byte_len {
        return Err(PlatformError::new(
            PlatformErrorKind::InvalidInput,
            format!(
                "asset staging request for verified object {object_id} expected {expected_byte_len} bytes but the verified object has {actual_byte_len}"
            ),
        ));
    }
    Ok(())
}
fn missing_staging(staging_id: &AssetStagingId) -> PlatformError {
    PlatformError::new(
        PlatformErrorKind::NotFound,
        format!("asset staging record {staging_id} was not found"),
    )
}
#[derive(Debug, Default)]
pub struct MemoryResourceStore {
    values: RefCell<BTreeMap<VirtualPath, Vec<u8>>>,
}
impl MemoryResourceStore {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert(&self, path: VirtualPath, bytes: impl Into<Vec<u8>>) {
        self.values.borrow_mut().insert(path, bytes.into());
    }
}
#[derive(Debug)]
struct MemoryResourceSnapshot {
    values: BTreeMap<VirtualPath, Vec<u8>>,
}
impl ResourceSnapshot for MemoryResourceSnapshot {
    fn read(&self, path: &VirtualPath) -> PlatformResult<Option<Vec<u8>>> {
        Ok(self.values.get(path).cloned())
    }
    fn list(&self, prefix: &VirtualPath) -> PlatformResult<Vec<VirtualPath>> {
        Ok(self
            .values
            .keys()
            .filter(|path| path.starts_with(prefix))
            .cloned()
            .collect())
    }
}
impl ResourceStore for MemoryResourceStore {
    fn prepare_snapshot<'a>(
        &'a self,
        request: ResourceSnapshotRequest,
    ) -> PlatformFuture<'a, PlatformResult<Box<dyn ResourceSnapshot>>> {
        Box::pin(async move {
            let values = self
                .values
                .borrow()
                .iter()
                .filter(|(path, _)| request.includes(path))
                .map(|(path, bytes)| (path.clone(), bytes.clone()))
                .collect();
            Ok(Box::new(MemoryResourceSnapshot { values }) as Box<dyn ResourceSnapshot>)
        })
    }
}
