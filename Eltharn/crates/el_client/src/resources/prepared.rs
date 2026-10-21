use super::manager::ClientResourceManager;
use el_launcher_core::PreparedAssetManifest;
use el_platform_api::{
    AssetObjectId, AssetReleaseCommit, AssetReleaseEntry, AssetReleaseId, AssetReleaseSnapshot,
    AssetReleaseSnapshotInfo, AssetReleaseSnapshotKey, PlatformResult, ResourceBytes, VirtualPath,
};
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::ops::Range;
use std::path::Path;
#[derive(Deserialize)]
struct Index {
    objects: BTreeMap<String, Object>,
}
#[derive(Deserialize)]
struct Object {
    hash: String,
    size: u64,
}
fn verified_bytes(path: &Path, expected: &str) -> Result<Vec<u8>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Resource {}: {e}", path.display()))?;
    let actual: String = Sha1::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if actual != expected.to_ascii_lowercase() {
        return Err(format!("Resource hash mismatch: {}", path.display()));
    }
    Ok(bytes)
}
struct NativePreparedSnapshot {
    info: AssetReleaseSnapshotInfo,
    bytes: ResourceBytes,
    entries: BTreeMap<VirtualPath, Range<usize>>,
}
impl AssetReleaseSnapshot for NativePreparedSnapshot {
    fn info(&self) -> &AssetReleaseSnapshotInfo {
        &self.info
    }
    fn read(&self, path: &VirtualPath) -> PlatformResult<Option<Vec<u8>>> {
        self.read_view(path)
            .map(|bytes| bytes.map(|bytes| bytes.to_vec()))
    }
    fn read_view(&self, path: &VirtualPath) -> PlatformResult<Option<ResourceBytes>> {
        Ok(self
            .entries
            .get(path)
            .map(|range| self.bytes.slice(range.clone())))
    }
    fn list_all(&self) -> PlatformResult<Vec<VirtualPath>> {
        Ok(self.entries.keys().cloned().collect())
    }
    fn list(&self, prefix: &VirtualPath) -> PlatformResult<Vec<VirtualPath>> {
        Ok(self
            .entries
            .keys()
            .filter(|path| path.starts_with(prefix))
            .cloned()
            .collect())
    }
}
struct NativeSnapshotBuilder {
    file: std::fs::File,
    objects: BTreeMap<AssetObjectId, Range<usize>>,
    byte_len: usize,
}
impl NativeSnapshotBuilder {
    fn new(directory: &Path) -> Result<Self, String> {
        let file = tempfile::tempfile_in(directory)
            .map_err(|e| format!("Prepared resource snapshot storage: {e}"))?;
        Ok(Self {
            file,
            objects: BTreeMap::new(),
            byte_len: 0,
        })
    }
    fn publish(&mut self, bytes: &[u8]) -> Result<AssetObjectId, String> {
        let id = AssetObjectId::sha256_of(bytes);
        if !self.objects.contains_key(&id) {
            let end = self
                .byte_len
                .checked_add(bytes.len())
                .ok_or_else(|| "Prepared resource snapshot exceeds address space".to_string())?;
            self.file
                .write_all(bytes)
                .map_err(|e| format!("Prepared resource snapshot: {e}"))?;
            self.objects.insert(id.clone(), self.byte_len..end);
            self.byte_len = end;
        }
        Ok(id)
    }
    fn finish(self, commit: AssetReleaseCommit) -> Result<NativePreparedSnapshot, String> {
        let entries = commit
            .entries()
            .iter()
            .map(|entry| {
                self.objects
                    .get(&entry.object_id)
                    .cloned()
                    .map(|range| (entry.path.clone(), range))
                    .ok_or_else(|| "Prepared release references an unpublished object".to_string())
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let byte_len = entries.values().map(|range| range.len() as u64).sum();
        let bytes = if self.byte_len == 0 {
            ResourceBytes::from_owner(Vec::<u8>::new())
        } else {
            let mapping = unsafe { memmap2::MmapOptions::new().map(&self.file) }
                .map_err(|e| format!("Prepared resource mapping: {e}"))?;
            ResourceBytes::from_owner(mapping)
        };
        Ok(NativePreparedSnapshot {
            info: AssetReleaseSnapshotInfo {
                key: commit.key().clone(),
                entry_count: entries.len() as u64,
                byte_len,
            },
            bytes,
            entries,
        })
    }
}
pub async fn prepare_launcher_resources(
    assets_dir: &Path,
    receipt_path: &Path,
    expected_compatibility: &str,
) -> Result<ClientResourceManager, String> {
    let receipt_bytes =
        std::fs::read(receipt_path).map_err(|e| format!("Prepared asset manifest: {e}"))?;
    let receipt: PreparedAssetManifest = serde_json::from_slice(&receipt_bytes)
        .map_err(|e| format!("Prepared asset manifest: {e}"))?;
    receipt.validate(expected_compatibility)?;
    let index_bytes = verified_bytes(
        &assets_dir.join("indexes").join(receipt.index_file_name()),
        &receipt.index_sha1,
    )?;
    let index: Index =
        serde_json::from_slice(&index_bytes).map_err(|e| format!("Asset index: {e}"))?;
    let archive_bytes = verified_bytes(
        &assets_dir.join(receipt.client_archive_name()),
        &receipt.client_sha1,
    )?;
    let mut archive = zip::ZipArchive::new(Cursor::new(archive_bytes))
        .map_err(|e| format!("Client archive: {e}"))?;
    let mut store = NativeSnapshotBuilder::new(assets_dir)?;
    let mut entries = BTreeMap::new();
    let mut item_tag_documents = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        if !file.is_dir() && file.name().starts_with("data/") {
            let parts: Vec<_> = file.name().splitn(5, '/').collect();
            if parts.len() == 5 && parts[2] == "tags" && parts[3] == "item" {
                if let Some(path) = parts[4].strip_suffix(".json") {
                    let id = el_core::ResourceLocation::new(parts[1], path);
                    let mut bytes = Vec::new();
                    file.read_to_end(&mut bytes)
                        .map_err(|error| error.to_string())?;
                    item_tag_documents.push((id, bytes));
                }
            }
            continue;
        }
        if file.is_dir() || !file.name().starts_with("assets/") {
            continue;
        }
        let path = VirtualPath::new(file.name()).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        let id = store.publish(&bytes)?;
        if entries.insert(path, id).is_some() {
            return Err("Client archive contains duplicate resource paths".into());
        }
    }
    drop(archive);
    for (virtual_name, object) in index.objects {
        if object.hash.len() != 40 || !object.hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("Invalid object hash for {virtual_name}"));
        }
        let prepared_path = if virtual_name == "pack.mcmeta" {
            virtual_name.clone()
        } else {
            format!("assets/{virtual_name}")
        };
        let path = VirtualPath::new(prepared_path).map_err(|e| e.to_string())?;
        let bytes = verified_bytes(
            &assets_dir
                .join("objects")
                .join(&object.hash[..2])
                .join(&object.hash),
            &object.hash,
        )?;
        if bytes.len() as u64 != object.size {
            return Err(format!("Resource size mismatch: {virtual_name}"));
        }
        entries.insert(path, store.publish(&bytes)?);
    }
    let key = AssetReleaseSnapshotKey::managed(AssetReleaseId::sha256_of(&receipt_bytes));
    let commit = AssetReleaseCommit::new(
        key.clone(),
        entries
            .into_iter()
            .map(|(path, object_id)| AssetReleaseEntry { path, object_id }),
    )
    .map_err(|e| e.to_string())?;
    let snapshot = store.finish(commit)?;
    let manager = ClientResourceManager::from_prepared_release_snapshot(&snapshot)
        .map_err(|e| e.to_string())?;
    let item_tags = el_inventory::creative::ItemTagIndex::from_documents(item_tag_documents)?;
    el_inventory::creative::replace_item_tags(item_tags);
    Ok(manager)
}
