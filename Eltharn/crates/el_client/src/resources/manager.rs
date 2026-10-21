use el_core::resource_location::ResourceLocation;
use el_platform_api::{
    AssetReleaseId, AssetReleaseSnapshot, AssetReleaseSnapshotKey, AssetStore,
    ReleaseSnapshotAdapter, ResourceSnapshot, VirtualPath,
};
use image::DynamicImage;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::env;
use std::fmt;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
const MODS_ICON: &[u8] = include_bytes!("../../assets/icons/mods.png");
fn builtin_texture(id: &ResourceLocation) -> Option<&'static [u8]> {
    (id.namespace == "eltharn" && id.path == "gui/sprites/icon/mods").then_some(MODS_ICON)
}
fn is_zip_pack_root(root: &Path) -> bool {
    if !root.is_file() {
        return false;
    }
    let Ok(mut file) = fs::File::open(root) else {
        return false;
    };
    let mut magic = [0_u8; 4];
    std::io::Read::read_exact(&mut file, &mut magic).is_ok()
        && matches!(
            magic,
            [0x50, 0x4b, 0x03, 0x04] | [0x50, 0x4b, 0x05, 0x06] | [0x50, 0x4b, 0x07, 0x08]
        )
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientResourceKind {
    Atlas,
    BlockState,
    Font,
    ItemDefinition,
    Lang,
    Model,
    SoundMetadata,
    Texture,
    WaypointStyle,
}
#[derive(serde::Deserialize, Debug)]
struct AssetIndexEntry {
    hash: String,
    #[serde(default)]
    size: Option<u64>,
}
#[derive(serde::Deserialize, Debug)]
struct AssetIndex {
    objects: std::collections::HashMap<String, AssetIndexEntry>,
}
#[derive(Clone, Debug)]
pub struct ClientResourceManager {
    assets_root: PathBuf,
    prepared_base_resources: Option<Arc<BTreeMap<String, el_platform_api::ResourceBytes>>>,
    prepared_pack_layers: Vec<Arc<BTreeMap<String, el_platform_api::ResourceBytes>>>,
    extra_roots: Vec<PathBuf>,
    index_map: Option<std::collections::HashMap<String, String>>,
    verified_indexed_objects: Option<Arc<std::collections::HashMap<String, Vec<u8>>>>,
    objects_root: Option<PathBuf>,
    external_images: std::collections::HashMap<ResourceLocation, PathBuf>,
    zip_archive_cache:
        Arc<Mutex<std::collections::HashMap<PathBuf, zip::ZipArchive<Cursor<Vec<u8>>>>>>,
    zip_root_kind_cache: Arc<Mutex<std::collections::HashMap<PathBuf, bool>>>,
}
#[derive(Debug)]
pub enum ResourceError {
    ProjectRootNotFound,
    InvalidLocation(String),
    PreparedSnapshot {
        operation: &'static str,
        message: String,
    },
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Image {
        path: PathBuf,
        source: image::ImageError,
    },
    Parse {
        id: ResourceLocation,
        message: String,
    },
    MissingAssetsDirectory,
    MissingAssetIndexName,
    MissingAssetIndex {
        expected_path: PathBuf,
        index_name: String,
    },
    InvalidAssetIndex {
        path: PathBuf,
        message: String,
    },
    ManifestHashMismatch {
        expected: String,
        actual: String,
    },
    MissingAssetObject {
        object_hash: String,
        expected_path: PathBuf,
    },
    CorruptAssetObject {
        object_hash: String,
        expected_path: PathBuf,
        message: String,
    },
    InvalidManifestHash {
        hash: String,
        message: String,
    },
    SnapshotNotFound {
        key_description: String,
    },
    AssetStoreError {
        operation: &'static str,
        message: String,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextureImageOrigin {
    ExternalImage,
    ExtraRoot(usize),
    PreparedPackLayer(usize),
    Base,
}
fn parse_pack_languages(bytes: &[u8]) -> Vec<el_gui::settings::language::LanguageInfo> {
    #[derive(serde::Deserialize)]
    struct PackLanguage {
        name: String,
        region: String,
        #[serde(default)]
        bidirectional: bool,
    }
    #[derive(serde::Deserialize)]
    struct PackMetadata {
        language: BTreeMap<String, PackLanguage>,
    }
    let Ok(metadata) = serde_json::from_slice::<PackMetadata>(bytes) else {
        return Vec::new();
    };
    if metadata.language.iter().any(|(code, info)| {
        !(1..=16).contains(&code.encode_utf16().count())
            || info.name.is_empty()
            || info.region.is_empty()
    }) {
        return Vec::new();
    }
    metadata
        .language
        .into_iter()
        .map(|(code, info)| {
            el_gui::settings::language::LanguageInfo::new(
                code,
                info.name,
                info.region,
                info.bidirectional,
            )
        })
        .collect()
}
fn materialize_prepared_asset_layer(
    snapshot: &dyn ResourceSnapshot,
    layer_name: &'static str,
) -> Result<Arc<BTreeMap<String, el_platform_api::ResourceBytes>>, ResourceError> {
    let assets_prefix =
        VirtualPath::new("assets").expect("the fixed virtual assets prefix must be valid");
    let paths = snapshot
        .list(&assets_prefix)
        .map_err(|error| ResourceError::PreparedSnapshot {
            operation: "list prepared resource layer",
            message: format!("{layer_name}: {error}"),
        })?;
    let mut resources = BTreeMap::new();
    for path in paths {
        if !path.starts_with(&assets_prefix) {
            return Err(ResourceError::PreparedSnapshot {
                operation: "validate prepared resource layer",
                message: format!("{layer_name}: snapshot listed non-assets path {path}"),
            });
        }
        let bytes = snapshot
            .read_view(&path)
            .map_err(|error| ResourceError::PreparedSnapshot {
                operation: "read prepared resource layer",
                message: format!("{layer_name}: {error}"),
            })?;
        let bytes = bytes.ok_or_else(|| ResourceError::PreparedSnapshot {
            operation: "read prepared resource layer",
            message: format!("{layer_name}: snapshot listed {path} but no bytes were available"),
        })?;
        resources.insert(path.as_str().to_string(), bytes);
    }
    let metadata_path = VirtualPath::new("pack.mcmeta").expect("fixed pack metadata path");
    let metadata_present = snapshot
        .list(&metadata_path)
        .map_err(|error| ResourceError::PreparedSnapshot {
            operation: "list prepared pack metadata",
            message: format!("{layer_name}: {error}"),
        })?
        .iter()
        .any(|path| path == &metadata_path);
    if metadata_present {
        let metadata = snapshot
            .read_view(&metadata_path)
            .map_err(|error| ResourceError::PreparedSnapshot {
                operation: "read prepared pack metadata",
                message: format!("{layer_name}: {error}"),
            })?
            .ok_or_else(|| ResourceError::PreparedSnapshot {
                operation: "read prepared pack metadata",
                message: format!("{layer_name}: listed pack.mcmeta has no bytes"),
            })?;
        resources.insert(metadata_path.as_str().to_string(), metadata);
    }
    Ok(Arc::new(resources))
}
impl ClientResourceManager {
    pub fn from_project_root(project_root: impl Into<PathBuf>) -> Self {
        Self {
            assets_root: project_root.into().join("assets").join("assets"),
            prepared_base_resources: None,
            prepared_pack_layers: Vec::new(),
            extra_roots: Vec::new(),
            index_map: None,
            verified_indexed_objects: None,
            objects_root: None,
            external_images: std::collections::HashMap::new(),
            zip_archive_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
            zip_root_kind_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }
    pub fn empty_for_wasm() -> Self {
        Self {
            assets_root: std::path::PathBuf::from("."),
            prepared_base_resources: None,
            prepared_pack_layers: Vec::new(),
            extra_roots: Vec::new(),
            index_map: None,
            verified_indexed_objects: None,
            objects_root: None,
            external_images: std::collections::HashMap::new(),
            zip_archive_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
            zip_root_kind_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }
    pub fn from_assets_root(
        assets_root: impl Into<PathBuf>,
        objects_dir: Option<PathBuf>,
        asset_index: Option<&str>,
    ) -> Self {
        let assets_root = assets_root.into();
        let mut index_map = None;
        let mut objects_root = None;
        if let (Some(index_name), Some(obj_dir)) = (asset_index, objects_dir) {
            let indexes_dir = obj_dir.join("indexes");
            let index_path = indexes_dir.join(format!("{}.json", index_name));
            if let Ok(content) = fs::read_to_string(&index_path) {
                if let Ok(index) = serde_json::from_str::<AssetIndex>(&content) {
                    let mut map = std::collections::HashMap::new();
                    for (k, v) in index.objects {
                        map.insert(k, v.hash);
                    }
                    let count = map.len();
                    index_map = Some(map);
                    objects_root = Some(obj_dir.join("objects"));
                    log::info!("Loaded asset index {} with {} entries", index_name, count);
                } else {
                    log::warn!("Failed to parse asset index at {:?}", index_path);
                }
            } else {
                log::warn!("Asset index file not found at {:?}", index_path);
            }
        }
        Self {
            assets_root,
            prepared_base_resources: None,
            prepared_pack_layers: Vec::new(),
            extra_roots: Vec::new(),
            index_map,
            verified_indexed_objects: None,
            objects_root,
            external_images: std::collections::HashMap::new(),
            zip_archive_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
            zip_root_kind_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }
    pub fn from_prepared_snapshot(snapshot: &dyn ResourceSnapshot) -> Result<Self, ResourceError> {
        let resources = materialize_prepared_asset_layer(snapshot, "base resource")?;
        Ok(Self {
            assets_root: PathBuf::from("<prepared-resource-snapshot>"),
            prepared_base_resources: Some(resources),
            prepared_pack_layers: Vec::new(),
            extra_roots: Vec::new(),
            index_map: None,
            verified_indexed_objects: None,
            objects_root: None,
            external_images: std::collections::HashMap::new(),
            zip_archive_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
            zip_root_kind_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        })
    }
    pub fn with_prepared_resource_pack_snapshot(
        mut self,
        snapshot: &dyn ResourceSnapshot,
    ) -> Result<Self, ResourceError> {
        self.prepared_pack_layers
            .push(materialize_prepared_asset_layer(snapshot, "resource-pack")?);
        Ok(self)
    }
    pub fn with_extra_roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.extra_roots = roots;
        self
    }
    pub fn extra_roots(&self) -> &[PathBuf] {
        &self.extra_roots
    }
    pub fn with_external_images(
        mut self,
        images: impl IntoIterator<Item = (ResourceLocation, PathBuf)>,
    ) -> Self {
        self.external_images.extend(images);
        self
    }
    pub fn from_prepared_release_snapshot(
        snapshot: &dyn AssetReleaseSnapshot,
    ) -> Result<Self, ResourceError> {
        let adapter = ReleaseSnapshotAdapter::new(snapshot);
        Self::from_prepared_snapshot(&adapter)
    }
    pub async fn from_launch_config_and_store(
        config: &el_launcher_core::GameLaunchConfig,
        store: &dyn AssetStore,
    ) -> Result<Self, ResourceError> {
        let release_id = AssetReleaseId::parse(&config.assets.manifest_hash.0).map_err(|e| {
            ResourceError::PreparedSnapshot {
                operation: "parse manifest hash",
                message: e.to_string(),
            }
        })?;
        let key = AssetReleaseSnapshotKey::managed(release_id);
        let snapshot = store
            .open_release_snapshot(key)
            .await
            .map_err(|e| ResourceError::AssetStoreError {
                operation: "open release snapshot",
                message: e.to_string(),
            })?
            .ok_or_else(|| ResourceError::SnapshotNotFound {
                key_description: format!(
                    "managed release {} for compatibility {}",
                    config.assets.manifest_hash.0, config.assets.compatibility_id.0
                ),
            })?;
        let adapter = ReleaseSnapshotAdapter::new(snapshot);
        Self::from_prepared_snapshot(&adapter)
    }
    pub fn from_indexed_disk_assets(
        assets_dir: impl Into<PathBuf>,
        asset_index: &str,
        expected_manifest_hash: Option<&str>,
    ) -> Result<Self, ResourceError> {
        let assets_dir = assets_dir.into();
        let index_path = assets_dir
            .join("indexes")
            .join(format!("{asset_index}.json"));
        if !index_path.is_file() {
            return Err(ResourceError::MissingAssetIndex {
                expected_path: index_path,
                index_name: asset_index.to_string(),
            });
        }
        let content = fs::read(&index_path).map_err(|source| ResourceError::Io {
            path: index_path.clone(),
            source,
        })?;
        if let Some(expected_hash) = expected_manifest_hash {
            verify_manifest_hash(&content, expected_hash)?;
        }
        let index = serde_json::from_slice::<AssetIndex>(&content).map_err(|e| {
            ResourceError::InvalidAssetIndex {
                path: index_path.clone(),
                message: e.to_string(),
            }
        })?;
        let objects_root = assets_dir.join("objects");
        let mut index_map = std::collections::HashMap::with_capacity(index.objects.len());
        let mut verified_objects = std::collections::HashMap::with_capacity(index.objects.len());
        for (virtual_path, entry) in index.objects {
            let clean_hash =
                clean_and_validate_object_hash(&entry.hash, &index_path, &virtual_path)?;
            let prefix = &clean_hash[..2];
            let object_path = objects_root.join(prefix).join(clean_hash);
            if !object_path.is_file() {
                return Err(ResourceError::MissingAssetObject {
                    object_hash: entry.hash.clone(),
                    expected_path: object_path,
                });
            }
            let metadata = fs::metadata(&object_path).map_err(|source| ResourceError::Io {
                path: object_path.clone(),
                source,
            })?;
            if let Some(expected_size) = entry.size {
                if metadata.len() != expected_size {
                    return Err(ResourceError::CorruptAssetObject {
                        object_hash: entry.hash.clone(),
                        expected_path: object_path.clone(),
                        message: format!(
                            "size mismatch for object '{virtual_path}': expected {expected_size} bytes, found {} bytes",
                            metadata.len()
                        ),
                    });
                }
            }
            let bytes = fs::read(&object_path).map_err(|source| ResourceError::Io {
                path: object_path.clone(),
                source,
            })?;
            let (matches, actual_hash) = compute_and_verify_hash(&bytes, clean_hash);
            if !matches {
                return Err(ResourceError::CorruptAssetObject {
                    object_hash: entry.hash.clone(),
                    expected_path: object_path,
                    message: format!(
                        "content hash mismatch for object '{virtual_path}': expected {clean_hash}, actual {actual_hash}"
                    ),
                });
            }
            index_map.insert(virtual_path.clone(), entry.hash);
            verified_objects.insert(virtual_path, bytes);
        }
        let assets_candidate = assets_dir.join("assets");
        let assets_root = if is_assets_root(&assets_candidate) {
            assets_candidate
        } else if is_assets_root(&assets_dir) {
            assets_dir.clone()
        } else {
            assets_dir.clone()
        };
        Ok(Self {
            assets_root,
            prepared_base_resources: None,
            prepared_pack_layers: Vec::new(),
            extra_roots: Vec::new(),
            index_map: Some(index_map),
            verified_indexed_objects: Some(Arc::new(verified_objects)),
            objects_root: Some(objects_root),
            external_images: std::collections::HashMap::new(),
            zip_archive_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
            zip_root_kind_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        })
    }
    pub fn from_configured_location(
        location: &crate::app::main_entry::game_config::FolderData,
        offline_developer_mode: bool,
    ) -> Result<Self, ResourceError> {
        if !offline_developer_mode {
            let Some(assets_dir) = &location.asset_directory else {
                return Err(ResourceError::MissingAssetsDirectory);
            };
            let index_name = location
                .asset_index
                .as_deref()
                .ok_or(ResourceError::MissingAssetIndexName)?;
            let index_file = assets_dir
                .join("indexes")
                .join(format!("{index_name}.json"));
            if !index_file.is_file() {
                return Err(ResourceError::MissingAssetIndex {
                    expected_path: index_file,
                    index_name: index_name.to_string(),
                });
            }
            return Self::from_indexed_disk_assets(assets_dir, index_name, None);
        }
        if let Some(assets_dir) = &location.asset_directory {
            let index_name = location.asset_index.as_deref();
            let assets_candidate = assets_dir.join("assets");
            if is_assets_root(&assets_candidate) {
                return Ok(Self::from_assets_root(
                    assets_candidate,
                    Some(assets_dir.clone()),
                    index_name,
                ));
            } else if is_assets_root(assets_dir) {
                return Ok(Self::from_assets_root(
                    assets_dir.clone(),
                    Some(assets_dir.clone()),
                    index_name,
                ));
            } else if let Some(index_name) = index_name {
                return Self::from_indexed_disk_assets(assets_dir, index_name, None);
            }
        }
        Self::discover_developer_mode(
            location.asset_directory.clone(),
            location.asset_index.as_deref(),
        )
    }
    pub fn discover_default() -> Result<Self, ResourceError> {
        Self::discover(None, None)
    }
    pub fn discover(
        objects_dir: Option<PathBuf>,
        asset_index: Option<&str>,
    ) -> Result<Self, ResourceError> {
        if let Some(ref dir) = objects_dir {
            let assets_candidate = dir.join("assets");
            if is_assets_root(&assets_candidate) {
                return Ok(Self::from_assets_root(
                    assets_candidate,
                    Some(dir.clone()),
                    asset_index,
                ));
            } else if is_assets_root(dir) {
                return Ok(Self::from_assets_root(
                    dir.clone(),
                    Some(dir.clone()),
                    asset_index,
                ));
            } else if let Some(index_name) = asset_index {
                return Self::from_indexed_disk_assets(dir, index_name, None);
            }
        }
        Self::discover_developer_mode(objects_dir, asset_index)
    }
    pub fn discover_developer_mode(
        objects_dir: Option<PathBuf>,
        asset_index: Option<&str>,
    ) -> Result<Self, ResourceError> {
        let root = env::var_os("ELTHARN_DEV_ASSETS_ROOT")
            .or_else(|| env::var_os("MC_PROJECT_ROOT"))
            .map(PathBuf::from)
            .ok_or(ResourceError::ProjectRootNotFound)?;
        let assets_root = if is_assets_root(&root) {
            Some(root.clone())
        } else if is_assets_root(&root.join("assets")) {
            Some(root.join("assets"))
        } else {
            None
        }
        .ok_or(ResourceError::ProjectRootNotFound)?;
        Ok(Self::from_assets_root(
            assets_root,
            objects_dir,
            asset_index,
        ))
    }
    pub fn assets_root(&self) -> &Path {
        &self.assets_root
    }
    fn prepared_base_bytes(&self, relative_path: &str) -> Option<Vec<u8>> {
        self.prepared_base_resources
            .as_ref()?
            .get(relative_path)
            .map(|bytes| bytes.to_vec())
    }
    fn prepared_pack_bytes(&self, relative_path: &str) -> Option<Vec<u8>> {
        self.prepared_pack_layers
            .iter()
            .rev()
            .find_map(|layer| layer.get(relative_path).map(|bytes| bytes.to_vec()))
    }
    fn prepared_pack_layer_index(&self, relative_path: &str) -> Option<usize> {
        self.prepared_pack_layers
            .iter()
            .rposition(|layer| layer.contains_key(relative_path))
    }
    fn pack_root_contains_resource(&self, root: &Path, relative_path: &str) -> bool {
        if root.is_dir() {
            return root.join(relative_path).is_file();
        }
        self.is_zip_pack_root_cached(root) && self.zip_contains_cached(root, relative_path)
    }
    fn read_optional_pack_root_resource(
        &self,
        root: &Path,
        relative_path: &str,
    ) -> Result<Option<Vec<u8>>, ResourceError> {
        if root.is_dir() {
            let path = root.join(relative_path);
            if path.is_file() {
                return fs::read(&path)
                    .map(Some)
                    .map_err(|source| ResourceError::Io { path, source });
            }
            return Ok(None);
        }
        if self.is_zip_pack_root_cached(root) {
            return Ok(self.read_from_zip_cached(root, relative_path).ok());
        }
        Ok(None)
    }
    fn indexed_asset_path(&self, virtual_path: &str) -> Option<PathBuf> {
        let index_map = self.index_map.as_ref()?;
        let objects_root = self.objects_root.as_ref()?;
        let hash = index_map.get(virtual_path)?;
        let clean = clean_hash(hash);
        if clean.len() < 2 {
            return None;
        }
        let path = objects_root.join(&clean[..2]).join(clean);
        path.is_file().then_some(path)
    }
    fn indexed_asset_expected_path(&self, hash: &str) -> PathBuf {
        let clean = clean_hash(hash);
        let prefix = if clean.len() >= 2 { &clean[..2] } else { "00" };
        self.objects_root
            .as_deref()
            .unwrap_or_else(|| Path::new("."))
            .join(prefix)
            .join(clean)
    }
    fn texture_image_origin(
        &self,
        id: &ResourceLocation,
        relative_path: &str,
    ) -> Option<TextureImageOrigin> {
        if self.external_images.contains_key(id) {
            return Some(TextureImageOrigin::ExternalImage);
        }
        for index in (0..self.extra_roots.len()).rev() {
            if self.pack_root_contains_resource(&self.extra_roots[index], relative_path) {
                return Some(TextureImageOrigin::ExtraRoot(index));
            }
        }
        if let Some(index) = self.prepared_pack_layer_index(relative_path) {
            return Some(TextureImageOrigin::PreparedPackLayer(index));
        }
        let index_path = format!("{}/textures/{}.png", id.namespace, id.path);
        if let Some(verified) = &self.verified_indexed_objects {
            if verified.contains_key(&index_path) {
                return Some(TextureImageOrigin::Base);
            }
        } else if self.indexed_asset_path(&index_path).is_some() {
            return Some(TextureImageOrigin::Base);
        }
        if let Some(base_resources) = &self.prepared_base_resources {
            return base_resources
                .contains_key(relative_path)
                .then_some(TextureImageOrigin::Base);
        }
        self.assets_root
            .join(&id.namespace)
            .join("textures")
            .join(format!("{}.png", id.path))
            .is_file()
            .then_some(TextureImageOrigin::Base)
    }
    fn read_metadata_from_extra_roots_at_or_above(
        &self,
        first_allowed: usize,
        relative_path: &str,
    ) -> Result<Option<Vec<u8>>, ResourceError> {
        for index in (first_allowed.min(self.extra_roots.len())..self.extra_roots.len()).rev() {
            if let Some(bytes) =
                self.read_optional_pack_root_resource(&self.extra_roots[index], relative_path)?
            {
                return Ok(Some(bytes));
            }
        }
        Ok(None)
    }
    fn read_metadata_from_prepared_layers_at_or_above(
        &self,
        first_allowed: usize,
        relative_path: &str,
    ) -> Option<Vec<u8>> {
        self.prepared_pack_layers
            .get(first_allowed..)
            .and_then(|layers| {
                layers
                    .iter()
                    .rev()
                    .find_map(|layer| layer.get(relative_path).map(|bytes| bytes.to_vec()))
            })
    }
    fn read_indexed_resource(&self, virtual_path: &str) -> Result<Option<Vec<u8>>, ResourceError> {
        let Some(index_map) = self.index_map.as_ref() else {
            return Ok(None);
        };
        let Some(hash) = index_map.get(virtual_path) else {
            return Ok(None);
        };
        if let Some(verified) = &self.verified_indexed_objects {
            if let Some(bytes) = verified.get(virtual_path) {
                return Ok(Some(bytes.clone()));
            }
            let path = self.indexed_asset_expected_path(hash);
            return Err(ResourceError::MissingAssetObject {
                object_hash: hash.clone(),
                expected_path: path,
            });
        }
        let path = self.indexed_asset_expected_path(hash);
        if !path.is_file() {
            return Err(ResourceError::MissingAssetObject {
                object_hash: hash.clone(),
                expected_path: path,
            });
        }
        let bytes = fs::read(&path).map_err(|source| ResourceError::Io {
            path: path.clone(),
            source,
        })?;
        let clean = clean_hash(hash);
        let (matches, actual) = compute_and_verify_hash(&bytes, clean);
        if !matches {
            return Err(ResourceError::CorruptAssetObject {
                object_hash: hash.clone(),
                expected_path: path,
                message: format!(
                    "content hash mismatch for indexed resource '{virtual_path}': expected {clean}, actual {actual}"
                ),
            });
        }
        Ok(Some(bytes))
    }
    fn read_base_texture_metadata(
        &self,
        relative_path: &str,
    ) -> Result<Option<Vec<u8>>, ResourceError> {
        if self.prepared_base_resources.is_some() {
            return Ok(self.prepared_base_bytes(relative_path));
        }
        let path = self
            .assets_root
            .join(relative_path.trim_start_matches("assets/"));
        match fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(ResourceError::Io { path, source }),
        }
    }
    fn has_virtual_resource_layers(&self) -> bool {
        self.prepared_base_resources.is_some() || !self.prepared_pack_layers.is_empty()
    }
    fn missing_prepared_base_resource(&self, relative_path: &str) -> ResourceError {
        ResourceError::PreparedSnapshot {
            operation: "read prepared base resource",
            message: format!("{relative_path} was not included in the immutable resource snapshot"),
        }
    }
    fn prepared_display_path(&self, relative_path: &str) -> PathBuf {
        PathBuf::from("<prepared-resource-snapshot>").join(relative_path)
    }
    pub fn namespaces(&self) -> Result<Vec<String>, ResourceError> {
        let mut namespaces = BTreeSet::new();
        namespaces.insert("eltharn".to_owned());
        if let Some(resources) = &self.prepared_base_resources {
            extend_namespaces_from_prepared_resources(resources, &mut namespaces);
        } else if let Ok(entries) = fs::read_dir(&self.assets_root) {
            for entry in entries {
                let entry = entry.map_err(|source| ResourceError::Io {
                    path: self.assets_root.clone(),
                    source,
                })?;
                if entry
                    .file_type()
                    .map_err(|source| ResourceError::Io {
                        path: entry.path(),
                        source,
                    })?
                    .is_dir()
                {
                    namespaces.insert(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
        for layer in &self.prepared_pack_layers {
            extend_namespaces_from_prepared_resources(layer, &mut namespaces);
        }
        if let Some(index_map) = &self.index_map {
            for key in index_map.keys() {
                if let Some((ns, _)) = key.split_once('/') {
                    namespaces.insert(ns.to_string());
                }
            }
        }
        for root in &self.extra_roots {
            extend_namespaces_from_pack_root(root, &mut namespaces)?;
        }
        Ok(namespaces.into_iter().collect())
    }
    pub fn resource_path(
        &self,
        kind: ClientResourceKind,
        id: &ResourceLocation,
    ) -> Result<PathBuf, ResourceError> {
        validate_location(id)?;
        if self.has_virtual_resource_layers() {
            return Err(ResourceError::PreparedSnapshot {
                operation: "resolve native resource path",
                message: format!(
                    "{id} is stored in a virtual prepared snapshot and has no host path"
                ),
            });
        }
        let (directory, extension) = kind.directory_and_extension();
        Ok(self
            .assets_root
            .join(&id.namespace)
            .join(directory)
            .join(format!("{}{}", id.path, extension)))
    }
    pub fn read_bytes(
        &self,
        kind: ClientResourceKind,
        id: &ResourceLocation,
    ) -> Result<Vec<u8>, ResourceError> {
        validate_location(id)?;
        let (directory, extension) = kind.directory_and_extension();
        let rel_path = typed_resource_relative_path(id, directory, extension);
        for root in self.extra_roots.iter().rev() {
            if root.is_dir() {
                let p = root.join(&rel_path);
                if p.is_file() {
                    return fs::read(&p).map_err(|source| ResourceError::Io { path: p, source });
                }
            } else if self.is_zip_pack_root_cached(root) {
                if let Ok(bytes) = self.read_from_zip_cached(root, &rel_path) {
                    return Ok(bytes);
                }
            }
        }
        if let Some(bytes) = self.prepared_pack_bytes(&rel_path) {
            return Ok(bytes);
        }
        if let Some(index_map) = &self.index_map {
            let virtual_path = typed_index_path(id, directory, extension);
            if let Some(hash) = index_map.get(&virtual_path) {
                if let Some(verified) = &self.verified_indexed_objects {
                    if let Some(bytes) = verified.get(&virtual_path) {
                        return Ok(bytes.clone());
                    }
                    let path = self.indexed_asset_expected_path(hash);
                    return Err(ResourceError::MissingAssetObject {
                        object_hash: hash.clone(),
                        expected_path: path,
                    });
                }
                let path = self.indexed_asset_expected_path(hash);
                if !path.is_file() {
                    return Err(ResourceError::MissingAssetObject {
                        object_hash: hash.clone(),
                        expected_path: path,
                    });
                }
                let bytes = fs::read(&path).map_err(|source| ResourceError::Io {
                    path: path.clone(),
                    source,
                })?;
                let clean = clean_hash(hash);
                let (matches, actual) = compute_and_verify_hash(&bytes, clean);
                if !matches {
                    return Err(ResourceError::CorruptAssetObject {
                        object_hash: hash.clone(),
                        expected_path: path,
                        message: format!(
                            "content hash mismatch for indexed resource '{virtual_path}': expected {clean}, actual {actual}"
                        ),
                    });
                }
                return Ok(bytes);
            }
        }
        if self.prepared_base_resources.is_some() {
            return self
                .prepared_base_bytes(&rel_path)
                .or_else(|| {
                    matches!(kind, ClientResourceKind::Texture)
                        .then(|| builtin_texture(id).map(<[u8]>::to_vec))
                        .flatten()
                })
                .ok_or_else(|| self.missing_prepared_base_resource(&rel_path));
        }
        let path = self
            .assets_root
            .join(&id.namespace)
            .join(directory)
            .join(format!("{}{}", id.path, extension));
        match fs::read(&path) {
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                if matches!(kind, ClientResourceKind::Texture) {
                    if let Some(bytes) = builtin_texture(id) {
                        return Ok(bytes.to_vec());
                    }
                }
                Err(ResourceError::Io { path, source })
            }
            result => result.map_err(|source| ResourceError::Io { path, source }),
        }
    }
    pub fn read_resource_bytes(&self, id: &ResourceLocation) -> Result<Vec<u8>, ResourceError> {
        validate_location(id)?;
        let rel_path = format!("assets/{}/{}", id.namespace, id.path);
        for root in self.extra_roots.iter().rev() {
            if root.is_dir() {
                let path = root.join(&rel_path);
                if path.is_file() {
                    return fs::read(&path).map_err(|source| ResourceError::Io { path, source });
                }
            } else if self.is_zip_pack_root_cached(root) {
                if let Ok(bytes) = self.read_from_zip_cached(root, &rel_path) {
                    return Ok(bytes);
                }
            }
        }
        if let Some(bytes) = self.prepared_pack_bytes(&rel_path) {
            return Ok(bytes);
        }
        if let Some(index_map) = &self.index_map {
            let virtual_path = format!("{}/{}", id.namespace, id.path);
            if let Some(hash) = index_map.get(&virtual_path) {
                if let Some(verified) = &self.verified_indexed_objects {
                    if let Some(bytes) = verified.get(&virtual_path) {
                        return Ok(bytes.clone());
                    }
                    let path = self.indexed_asset_expected_path(hash);
                    return Err(ResourceError::MissingAssetObject {
                        object_hash: hash.clone(),
                        expected_path: path,
                    });
                }
                let path = self.indexed_asset_expected_path(hash);
                if !path.is_file() {
                    return Err(ResourceError::MissingAssetObject {
                        object_hash: hash.clone(),
                        expected_path: path,
                    });
                }
                let bytes = fs::read(&path).map_err(|source| ResourceError::Io {
                    path: path.clone(),
                    source,
                })?;
                let clean = clean_hash(hash);
                let (matches, actual) = compute_and_verify_hash(&bytes, clean);
                if !matches {
                    return Err(ResourceError::CorruptAssetObject {
                        object_hash: hash.clone(),
                        expected_path: path,
                        message: format!(
                            "content hash mismatch for indexed resource '{virtual_path}': expected {clean}, actual {actual}"
                        ),
                    });
                }
                return Ok(bytes);
            }
        }
        if self.prepared_base_resources.is_some() {
            return self
                .prepared_base_bytes(&rel_path)
                .ok_or_else(|| self.missing_prepared_base_resource(&rel_path));
        }
        let path = self.assets_root.join(&id.namespace).join(&id.path);
        fs::read(&path).map_err(|source| ResourceError::Io { path, source })
    }
    pub fn read_string(
        &self,
        kind: ClientResourceKind,
        id: &ResourceLocation,
    ) -> Result<String, ResourceError> {
        let bytes = self.read_bytes(kind, id)?;
        String::from_utf8(bytes).map_err(|_| ResourceError::Io {
            path: PathBuf::from(format!("{:?}:{:?}", kind, id)),
            source: std::io::Error::other("invalid utf8"),
        })
    }
    pub fn read_texture_metadata(
        &self,
        id: &ResourceLocation,
    ) -> Result<Option<Vec<u8>>, ResourceError> {
        validate_location(id)?;
        let image_path = format!("assets/{}/textures/{}.png", id.namespace, id.path);
        let metadata_path = format!("{image_path}.mcmeta");
        let indexed_metadata_path = format!("{}/textures/{}.png.mcmeta", id.namespace, id.path);
        let Some(origin) = self.texture_image_origin(id, &image_path) else {
            return Ok(None);
        };
        match origin {
            TextureImageOrigin::ExternalImage => Ok(None),
            TextureImageOrigin::ExtraRoot(source_index) => {
                self.read_metadata_from_extra_roots_at_or_above(source_index, &metadata_path)
            }
            TextureImageOrigin::PreparedPackLayer(source_index) => {
                if let Some(bytes) =
                    self.read_metadata_from_extra_roots_at_or_above(0, &metadata_path)?
                {
                    return Ok(Some(bytes));
                }
                Ok(self
                    .read_metadata_from_prepared_layers_at_or_above(source_index, &metadata_path))
            }
            TextureImageOrigin::Base => {
                if let Some(bytes) =
                    self.read_metadata_from_extra_roots_at_or_above(0, &metadata_path)?
                {
                    return Ok(Some(bytes));
                }
                if let Some(bytes) =
                    self.read_metadata_from_prepared_layers_at_or_above(0, &metadata_path)
                {
                    return Ok(Some(bytes));
                }
                if let Some(bytes) = self.read_indexed_resource(&indexed_metadata_path)? {
                    return Ok(Some(bytes));
                }
                self.read_base_texture_metadata(&metadata_path)
            }
        }
    }
    pub fn read_image(&self, id: &ResourceLocation) -> Result<DynamicImage, ResourceError> {
        validate_location(id)?;
        if let Some(path) = self.external_images.get(id) {
            return image::open(path).map_err(|source| ResourceError::Image {
                path: path.clone(),
                source,
            });
        }
        let (directory, extension) = ClientResourceKind::Texture.directory_and_extension();
        let rel_path = format!(
            "assets/{}/{}/{}{}",
            id.namespace, directory, id.path, extension
        );
        for root in self.extra_roots.iter().rev() {
            if root.is_dir() {
                let p = root.join(&rel_path);
                if p.is_file() {
                    return image::open(&p)
                        .map_err(|source| ResourceError::Image { path: p, source });
                }
            } else if self.is_zip_pack_root_cached(root) {
                if let Ok(bytes) = self.read_from_zip_cached(root, &rel_path) {
                    let img =
                        image::load_from_memory(&bytes).map_err(|source| ResourceError::Image {
                            path: root.clone(),
                            source,
                        })?;
                    return Ok(img);
                }
            }
        }
        if let Some(bytes) = self.prepared_pack_bytes(&rel_path) {
            let path = self.prepared_display_path(&rel_path);
            return image::load_from_memory(&bytes)
                .map_err(|source| ResourceError::Image { path, source });
        }
        if let Some(index_map) = &self.index_map {
            let virtual_path = format!("{}/{}/{}{}", id.namespace, directory, id.path, extension);
            if let Some(hash) = index_map.get(&virtual_path) {
                if let Some(verified) = &self.verified_indexed_objects {
                    if let Some(bytes) = verified.get(&virtual_path) {
                        let path = self.indexed_asset_expected_path(hash);
                        return image::load_from_memory(bytes)
                            .map_err(|source| ResourceError::Image { path, source });
                    }
                    let path = self.indexed_asset_expected_path(hash);
                    return Err(ResourceError::MissingAssetObject {
                        object_hash: hash.clone(),
                        expected_path: path,
                    });
                }
                let path = self.indexed_asset_expected_path(hash);
                if !path.is_file() {
                    return Err(ResourceError::MissingAssetObject {
                        object_hash: hash.clone(),
                        expected_path: path,
                    });
                }
                let bytes = std::fs::read(&path).map_err(|e| ResourceError::Image {
                    path: path.clone(),
                    source: image::ImageError::IoError(e),
                })?;
                let clean = clean_hash(hash);
                let (matches, actual) = compute_and_verify_hash(&bytes, clean);
                if !matches {
                    return Err(ResourceError::CorruptAssetObject {
                        object_hash: hash.clone(),
                        expected_path: path,
                        message: format!(
                            "content hash mismatch for image {virtual_path}: expected {clean}, actual {actual}"
                        ),
                    });
                }
                return image::load_from_memory(&bytes)
                    .map_err(|source| ResourceError::Image { path, source });
            }
        }
        if self.prepared_base_resources.is_some() {
            let bytes = self
                .prepared_base_bytes(&rel_path)
                .or_else(|| builtin_texture(id).map(<[u8]>::to_vec))
                .ok_or_else(|| self.missing_prepared_base_resource(&rel_path))?;
            let path = self.prepared_display_path(&rel_path);
            return image::load_from_memory(&bytes)
                .map_err(|source| ResourceError::Image { path, source });
        }
        let path = self
            .assets_root
            .join(&id.namespace)
            .join(directory)
            .join(format!("{}{}", id.path, extension));
        if !path.exists() {
            if let Some(bytes) = builtin_texture(id) {
                return image::load_from_memory(bytes)
                    .map_err(|source| ResourceError::Image { path, source });
            }
        }
        image::open(&path).map_err(|source| ResourceError::Image { path, source })
    }
    pub fn read_blockstate(&self, id: &ResourceLocation) -> Result<String, ResourceError> {
        self.read_string(ClientResourceKind::BlockState, id)
    }
    pub fn read_model(&self, id: &ResourceLocation) -> Result<String, ResourceError> {
        self.read_string(ClientResourceKind::Model, id)
    }
    fn zip_contains_cached(&self, zip_path: &Path, entry_name: &str) -> bool {
        let Ok(mut cache) = self.zip_archive_cache.lock() else {
            return false;
        };
        let archive = match cache.entry(zip_path.to_path_buf()) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let Ok(bytes) = fs::read(zip_path) else {
                    return false;
                };
                let Ok(archive) = zip::ZipArchive::new(Cursor::new(bytes)) else {
                    return false;
                };
                entry.insert(archive)
            }
        };
        archive.by_name(entry_name).is_ok()
    }
    fn read_from_zip_cached(
        &self,
        zip_path: &Path,
        entry_name: &str,
    ) -> Result<Vec<u8>, std::io::Error> {
        let mut cache = self
            .zip_archive_cache
            .lock()
            .map_err(|_| std::io::Error::other("resource ZIP cache poisoned"))?;
        let archive = match cache.entry(zip_path.to_path_buf()) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let bytes = fs::read(zip_path)?;
                let archive = zip::ZipArchive::new(Cursor::new(bytes))
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                entry.insert(archive)
            }
        };
        let mut entry = archive
            .by_name(entry_name)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let mut output = Vec::new();
        entry.read_to_end(&mut output)?;
        Ok(output)
    }
    fn is_zip_pack_root_cached(&self, root: &Path) -> bool {
        if let Ok(cache) = self.zip_root_kind_cache.lock() {
            if let Some(&is_zip) = cache.get(root) {
                return is_zip;
            }
        }
        let is_zip = is_zip_pack_root(root);
        if let Ok(mut cache) = self.zip_root_kind_cache.lock() {
            cache.insert(root.to_path_buf(), is_zip);
        }
        is_zip
    }
    pub fn read_lang(&self, id: &ResourceLocation) -> Result<String, ResourceError> {
        self.read_string(ClientResourceKind::Lang, id)
    }
    pub fn read_lang_stack(&self, id: &ResourceLocation) -> Result<Vec<Vec<u8>>, ResourceError> {
        validate_location(id)?;
        let relative_path = format!("assets/{}/lang/{}.json", id.namespace, id.path);
        let indexed_path = format!("{}/lang/{}.json", id.namespace, id.path);
        let mut stack = Vec::new();
        if id.namespace == "eltharn" && id.path == "en_us" {
            stack.push(include_bytes!("../../assets/lang/en_us.json").to_vec());
        }
        if let Some(bytes) = self.read_base_language_resource(&relative_path, &indexed_path)? {
            stack.push(bytes);
        }
        for layer in &self.prepared_pack_layers {
            if let Some(bytes) = layer.get(&relative_path) {
                stack.push(bytes.to_vec());
            }
        }
        for root in &self.extra_roots {
            if let Some(bytes) = self.read_optional_pack_root_resource(root, &relative_path)? {
                stack.push(bytes);
            }
        }
        Ok(stack)
    }
    pub fn read_default_language_deprecations(&self) -> Result<Option<Vec<u8>>, ResourceError> {
        self.read_base_language_resource(
            "assets/minecraft/lang/deprecated.json",
            "minecraft/lang/deprecated.json",
        )
    }
    fn read_base_language_resource(
        &self,
        relative_path: &str,
        indexed_path: &str,
    ) -> Result<Option<Vec<u8>>, ResourceError> {
        if self.prepared_base_resources.is_some() {
            return Ok(self.prepared_base_bytes(relative_path));
        }
        if let Some(bytes) = self.read_indexed_resource(indexed_path)? {
            return Ok(Some(bytes));
        }
        let path = self.assets_root.join(
            relative_path
                .strip_prefix("assets/")
                .unwrap_or(relative_path),
        );
        match fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(ResourceError::Io { path, source }),
        }
    }
    pub fn read_atlas(&self, id: &ResourceLocation) -> Result<String, ResourceError> {
        self.read_string(ClientResourceKind::Atlas, id)
    }
    pub fn read_sound_metadata(&self, id: &ResourceLocation) -> Result<String, ResourceError> {
        self.read_string(ClientResourceKind::SoundMetadata, id)
    }
    pub fn read_sound_metadata_stack(&self, namespace: &str) -> Result<Vec<String>, ResourceError> {
        let rel_path = format!("assets/{namespace}/sounds.json");
        let virtual_path = format!("{namespace}/sounds.json");
        let mut contents = Vec::new();
        if self.prepared_base_resources.is_some() {
            if let Some(bytes) = self.prepared_base_bytes(&rel_path) {
                let path = self.prepared_display_path(&rel_path);
                contents.push(String::from_utf8(bytes).map_err(|_| ResourceError::Io {
                    path,
                    source: std::io::Error::other("invalid utf8"),
                })?);
            }
        } else if let Some(index_map) = &self.index_map {
            if let Some(hash) = index_map.get(&virtual_path) {
                let bytes = if let Some(verified) = &self.verified_indexed_objects {
                    if let Some(bytes) = verified.get(&virtual_path) {
                        bytes.clone()
                    } else {
                        let path = self.indexed_asset_expected_path(hash);
                        return Err(ResourceError::MissingAssetObject {
                            object_hash: hash.clone(),
                            expected_path: path,
                        });
                    }
                } else {
                    let path = self.indexed_asset_expected_path(hash);
                    if !path.is_file() {
                        return Err(ResourceError::MissingAssetObject {
                            object_hash: hash.clone(),
                            expected_path: path,
                        });
                    }
                    let b = fs::read(&path).map_err(|source| ResourceError::Io {
                        path: path.clone(),
                        source,
                    })?;
                    let clean = clean_hash(hash);
                    let (matches, actual) = compute_and_verify_hash(&b, clean);
                    if !matches {
                        return Err(ResourceError::CorruptAssetObject {
                            object_hash: hash.clone(),
                            expected_path: path,
                            message: format!(
                                "content hash mismatch for sound definition {virtual_path}: expected {clean}, actual {actual}"
                            ),
                        });
                    }
                    b
                };
                let path = self.indexed_asset_expected_path(hash);
                contents.push(String::from_utf8(bytes).map_err(|_| ResourceError::Io {
                    path,
                    source: std::io::Error::other("invalid utf8"),
                })?);
            }
        } else {
            let base_path = self.assets_root.join(namespace).join("sounds.json");
            if base_path.is_file() {
                let bytes = fs::read(&base_path).map_err(|source| ResourceError::Io {
                    path: base_path.clone(),
                    source,
                })?;
                contents.push(String::from_utf8(bytes).map_err(|_| ResourceError::Io {
                    path: base_path,
                    source: std::io::Error::other("invalid utf8"),
                })?);
            }
        }
        for layer in &self.prepared_pack_layers {
            if let Some(bytes) = layer.get(&rel_path) {
                let path = self.prepared_display_path(&rel_path);
                contents.push(String::from_utf8(bytes.to_vec()).map_err(|_| {
                    ResourceError::Io {
                        path,
                        source: std::io::Error::other("invalid utf8"),
                    }
                })?);
            }
        }
        for root in &self.extra_roots {
            let bytes = if root.is_dir() {
                let path = root.join(&rel_path);
                if path.is_file() {
                    Some(fs::read(&path).map_err(|source| ResourceError::Io {
                        path: path.clone(),
                        source,
                    })?)
                } else {
                    None
                }
            } else if self.is_zip_pack_root_cached(root) {
                self.read_from_zip_cached(root, &rel_path).ok()
            } else {
                None
            };
            if let Some(bytes) = bytes {
                contents.push(String::from_utf8(bytes).map_err(|_| ResourceError::Io {
                    path: root.clone(),
                    source: std::io::Error::other("invalid utf8"),
                })?);
            }
        }
        if contents.is_empty() {
            return Err(ResourceError::Io {
                path: PathBuf::from(&rel_path),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "sounds.json not found"),
            });
        }
        Ok(contents)
    }
    pub fn read_font_stack(&self, id: &ResourceLocation) -> Result<Vec<String>, ResourceError> {
        validate_location(id)?;
        let clean_path = id.path.strip_suffix(".json").unwrap_or(&id.path);
        let rel_path = format!("assets/{}/font/{}.json", id.namespace, clean_path);
        let mut contents = Vec::new();
        if self.prepared_base_resources.is_some() {
            if let Some(bytes) = self.prepared_base_bytes(&rel_path) {
                let path = self.prepared_display_path(&rel_path);
                contents.push(String::from_utf8(bytes).map_err(|_| ResourceError::Io {
                    path,
                    source: std::io::Error::other("invalid utf8"),
                })?);
            }
        } else if let Some(bytes) =
            self.read_indexed_resource(&format!("{}/font/{}.json", id.namespace, clean_path))?
        {
            let path = PathBuf::from(&rel_path);
            contents.push(String::from_utf8(bytes).map_err(|_| ResourceError::Io {
                path,
                source: std::io::Error::other("invalid utf8"),
            })?);
        } else {
            let base_path = self
                .assets_root
                .join(&id.namespace)
                .join("font")
                .join(format!("{}.json", clean_path));
            if base_path.is_file() {
                let bytes = fs::read(&base_path).map_err(|source| ResourceError::Io {
                    path: base_path.clone(),
                    source,
                })?;
                contents.push(String::from_utf8(bytes).map_err(|_| ResourceError::Io {
                    path: base_path,
                    source: std::io::Error::other("invalid utf8"),
                })?);
            }
        }
        for layer in &self.prepared_pack_layers {
            if let Some(bytes) = layer.get(&rel_path) {
                let path = self.prepared_display_path(&rel_path);
                contents.push(String::from_utf8(bytes.to_vec()).map_err(|_| {
                    ResourceError::Io {
                        path,
                        source: std::io::Error::other("invalid utf8"),
                    }
                })?);
            }
        }
        for root in &self.extra_roots {
            let bytes = if root.is_dir() {
                let path = root.join(&rel_path);
                if path.is_file() {
                    Some(fs::read(&path).map_err(|source| ResourceError::Io {
                        path: path.clone(),
                        source,
                    })?)
                } else {
                    None
                }
            } else if self.is_zip_pack_root_cached(root) {
                self.read_from_zip_cached(root, &rel_path).ok()
            } else {
                None
            };
            if let Some(bytes) = bytes {
                contents.push(String::from_utf8(bytes).map_err(|_| ResourceError::Io {
                    path: root.clone(),
                    source: std::io::Error::other("invalid utf8"),
                })?);
            }
        }
        if contents.is_empty() {
            if self.prepared_base_resources.is_some() {
                return Err(self.missing_prepared_base_resource(&rel_path));
            }
            return Err(ResourceError::Io {
                path: self
                    .assets_root
                    .join(&id.namespace)
                    .join("font")
                    .join(format!("{}.json", id.path)),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "font not found"),
            });
        }
        Ok(contents)
    }
    pub fn read_atlas_stack(&self, id: &ResourceLocation) -> Result<Vec<String>, ResourceError> {
        validate_location(id)?;
        let rel_path = format!("assets/{}/atlases/{}.json", id.namespace, id.path);
        let mut contents = Vec::new();
        if self.prepared_base_resources.is_some() {
            if let Some(bytes) = self.prepared_base_bytes(&rel_path) {
                let path = self.prepared_display_path(&rel_path);
                contents.push(String::from_utf8(bytes).map_err(|_| ResourceError::Io {
                    path,
                    source: std::io::Error::other("invalid utf8"),
                })?);
            }
        } else if let Some(bytes) =
            self.read_indexed_resource(&format!("{}/atlases/{}.json", id.namespace, id.path))?
        {
            contents.push(String::from_utf8(bytes).map_err(|_| ResourceError::Io {
                path: PathBuf::from(&rel_path),
                source: std::io::Error::other("invalid utf8"),
            })?);
        } else {
            let base_path = self
                .assets_root
                .join(&id.namespace)
                .join("atlases")
                .join(format!("{}.json", id.path));
            if base_path.is_file() {
                let bytes = fs::read(&base_path).map_err(|source| ResourceError::Io {
                    path: base_path.clone(),
                    source,
                })?;
                contents.push(String::from_utf8(bytes).map_err(|_| ResourceError::Io {
                    path: base_path,
                    source: std::io::Error::other("invalid utf8"),
                })?);
            }
        }
        for layer in &self.prepared_pack_layers {
            if let Some(bytes) = layer.get(&rel_path) {
                let path = self.prepared_display_path(&rel_path);
                contents.push(String::from_utf8(bytes.to_vec()).map_err(|_| {
                    ResourceError::Io {
                        path,
                        source: std::io::Error::other("invalid utf8"),
                    }
                })?);
            }
        }
        for root in &self.extra_roots {
            let bytes = if root.is_dir() {
                let path = root.join(&rel_path);
                if path.is_file() {
                    Some(fs::read(&path).map_err(|source| ResourceError::Io {
                        path: path.clone(),
                        source,
                    })?)
                } else {
                    None
                }
            } else if self.is_zip_pack_root_cached(root) {
                self.read_from_zip_cached(root, &rel_path).ok()
            } else {
                None
            };
            if let Some(bytes) = bytes {
                contents.push(String::from_utf8(bytes).map_err(|_| ResourceError::Io {
                    path: root.clone(),
                    source: std::io::Error::other("invalid utf8"),
                })?);
            }
        }
        if contents.is_empty() {
            if self.prepared_base_resources.is_some() {
                return Err(self.missing_prepared_base_resource(&rel_path));
            }
            return Err(ResourceError::Io {
                path: self
                    .assets_root
                    .join(&id.namespace)
                    .join("atlases")
                    .join(format!("{}.json", id.path)),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "atlas not found"),
            });
        }
        Ok(contents)
    }
    pub fn list_resource_ids(
        &self,
        kind: ClientResourceKind,
        namespace: &str,
    ) -> Result<Vec<ResourceLocation>, ResourceError> {
        validate_namespace(namespace)?;
        if kind == ClientResourceKind::SoundMetadata {
            return self.list_sound_metadata_ids(namespace);
        }
        let (directory, extension) = kind.directory_and_extension();
        let mut ids = if self.prepared_base_resources.is_some() {
            let mut ids = Vec::new();
            self.extend_ids_from_prepared_base(&mut ids, namespace, directory, "", extension)?;
            ids
        } else {
            self.list_ids_from_directory(
                namespace,
                "",
                &self.assets_root.join(namespace).join(directory),
                extension,
            )?
        };
        if let Some(index_map) = &self.index_map {
            self.extend_ids_from_index_map(
                &mut ids, index_map, namespace, directory, "", extension,
            );
        }
        for layer in &self.prepared_pack_layers {
            extend_ids_from_prepared_resources(
                &mut ids, layer, namespace, directory, "", extension,
            )?;
        }
        for root in &self.extra_roots {
            self.extend_ids_from_pack_root(&mut ids, root, namespace, directory, "", extension)?;
        }
        deduplicate_resource_ids(&mut ids);
        Ok(ids)
    }
    pub fn list_resource_ids_in_directory(
        &self,
        kind: ClientResourceKind,
        namespace: &str,
        source_path: &str,
    ) -> Result<Vec<ResourceLocation>, ResourceError> {
        validate_namespace(namespace)?;
        validate_relative_resource_path(source_path)?;
        if kind == ClientResourceKind::SoundMetadata {
            return self.list_sound_metadata_ids(namespace);
        }
        let (directory, extension) = kind.directory_and_extension();
        let mut ids = if self.prepared_base_resources.is_some() {
            let mut ids = Vec::new();
            self.extend_ids_from_prepared_base(
                &mut ids,
                namespace,
                directory,
                source_path,
                extension,
            )?;
            ids
        } else {
            self.list_ids_from_directory(
                namespace,
                source_path,
                &self
                    .assets_root
                    .join(namespace)
                    .join(directory)
                    .join(source_path),
                extension,
            )?
        };
        if let Some(index_map) = &self.index_map {
            self.extend_ids_from_index_map(
                &mut ids,
                index_map,
                namespace,
                directory,
                source_path,
                extension,
            );
        }
        for layer in &self.prepared_pack_layers {
            extend_ids_from_prepared_resources(
                &mut ids,
                layer,
                namespace,
                directory,
                source_path,
                extension,
            )?;
        }
        for root in &self.extra_roots {
            self.extend_ids_from_pack_root(
                &mut ids,
                root,
                namespace,
                directory,
                source_path,
                extension,
            )?;
        }
        deduplicate_resource_ids(&mut ids);
        Ok(ids)
    }
    pub fn list_blockstate_ids(
        &self,
        namespace: &str,
    ) -> Result<Vec<ResourceLocation>, ResourceError> {
        self.list_resource_ids(ClientResourceKind::BlockState, namespace)
    }
    pub fn list_model_ids(&self, namespace: &str) -> Result<Vec<ResourceLocation>, ResourceError> {
        self.list_resource_ids(ClientResourceKind::Model, namespace)
    }
    pub fn list_model_ids_in_directory(
        &self,
        namespace: &str,
        source_path: &str,
    ) -> Result<Vec<ResourceLocation>, ResourceError> {
        self.list_resource_ids_in_directory(ClientResourceKind::Model, namespace, source_path)
    }
    pub fn list_item_definition_ids(
        &self,
        namespace: &str,
    ) -> Result<Vec<ResourceLocation>, ResourceError> {
        self.list_resource_ids(ClientResourceKind::ItemDefinition, namespace)
    }
    pub fn list_lang_ids(&self, namespace: &str) -> Result<Vec<ResourceLocation>, ResourceError> {
        self.list_resource_ids(ClientResourceKind::Lang, namespace)
    }
    pub fn extract_languages(&self) -> Vec<el_gui::settings::language::LanguageInfo> {
        let mut languages = BTreeMap::new();
        let mut append = |bytes: &[u8]| {
            for info in parse_pack_languages(bytes) {
                languages.entry(info.code.clone()).or_insert(info);
            }
        };
        if let Some(bytes) = self.prepared_base_bytes("pack.mcmeta") {
            append(&bytes);
        } else if let Ok(Some(bytes)) = self.read_indexed_resource("pack.mcmeta") {
            append(&bytes);
        } else if self.prepared_base_resources.is_none() {
            if let Some(pack_root) = self.assets_root.parent()
                && let Ok(Some(bytes)) =
                    self.read_optional_pack_root_resource(pack_root, "pack.mcmeta")
            {
                append(&bytes);
            }
        }
        for layer in &self.prepared_pack_layers {
            if let Some(bytes) = layer.get("pack.mcmeta") {
                append(bytes);
            }
        }
        for root in &self.extra_roots {
            if let Ok(Some(bytes)) = self.read_optional_pack_root_resource(root, "pack.mcmeta") {
                append(&bytes);
            }
        }
        for info in el_gui::settings::language::default_languages() {
            languages.entry(info.code.clone()).or_insert(info);
        }
        languages.into_values().collect()
    }
    pub fn list_font_ids(&self, namespace: &str) -> Result<Vec<ResourceLocation>, ResourceError> {
        self.list_resource_ids(ClientResourceKind::Font, namespace)
    }
    pub fn list_sound_metadata_ids(
        &self,
        namespace: &str,
    ) -> Result<Vec<ResourceLocation>, ResourceError> {
        validate_namespace(namespace)?;
        let base_contains_sounds = if self.prepared_base_resources.is_some() {
            self.prepared_base_bytes(&format!("assets/{namespace}/sounds.json"))
                .is_some()
        } else {
            self.assets_root
                .join(namespace)
                .join("sounds.json")
                .is_file()
        };
        let indexed_contains_sounds = self
            .index_map
            .as_ref()
            .map(|m| m.contains_key(&format!("{namespace}/sounds.json")))
            .unwrap_or(false);
        let virtual_pack_contains_sounds = self
            .prepared_pack_layers
            .iter()
            .any(|layer| layer.contains_key(&format!("assets/{namespace}/sounds.json")));
        if base_contains_sounds || indexed_contains_sounds || virtual_pack_contains_sounds {
            Ok(vec![ResourceLocation::new(namespace, "sounds")])
        } else {
            for root in &self.extra_roots {
                if root.is_dir()
                    && root
                        .join("assets")
                        .join(namespace)
                        .join("sounds.json")
                        .is_file()
                {
                    return Ok(vec![ResourceLocation::new(namespace, "sounds")]);
                }
                if self.is_zip_pack_root_cached(root)
                    && self
                        .read_from_zip_cached(root, &format!("assets/{namespace}/sounds.json"))
                        .is_ok()
                {
                    return Ok(vec![ResourceLocation::new(namespace, "sounds")]);
                }
            }
            Ok(Vec::new())
        }
    }
    pub fn list_texture_ids_in_directory(
        &self,
        namespace: &str,
        source_path: &str,
        id_prefix: &str,
    ) -> Result<Vec<ResourceLocation>, ResourceError> {
        let namespace_id = ResourceLocation::new(namespace, "placeholder");
        validate_location(&namespace_id)?;
        validate_relative_resource_path(source_path)?;
        validate_relative_resource_path(id_prefix.trim_end_matches('/'))?;
        let mut ids = Vec::new();
        if namespace == "eltharn" {
            if let Some(suffix) = "gui/sprites/icon/mods"
                .strip_prefix(&format!("{}/", source_path.trim_end_matches('/')))
            {
                ids.push(ResourceLocation::new(
                    namespace,
                    &format!("{id_prefix}{suffix}"),
                ));
            }
        }
        if self.prepared_base_resources.is_some() {
            self.extend_texture_ids_from_prepared_base(
                &mut ids,
                namespace,
                source_path,
                id_prefix,
            )?;
        } else {
            let base = self
                .assets_root
                .join(namespace)
                .join("textures")
                .join(source_path);
            if base.is_dir() {
                self.collect_texture_ids(namespace, id_prefix, &base, &base, &mut ids)?;
            }
        }
        if let Some(index_map) = &self.index_map {
            self.extend_texture_ids_from_index_map(
                &mut ids,
                index_map,
                namespace,
                source_path,
                id_prefix,
            );
        }
        for layer in &self.prepared_pack_layers {
            extend_texture_ids_from_prepared_resources(
                &mut ids,
                layer,
                namespace,
                source_path,
                id_prefix,
            )?;
        }
        for root in &self.extra_roots {
            self.extend_texture_ids_from_pack_root(
                &mut ids,
                root,
                namespace,
                source_path,
                id_prefix,
            )?;
        }
        deduplicate_resource_ids(&mut ids);
        Ok(ids)
    }
    fn extend_ids_from_index_map(
        &self,
        ids: &mut Vec<ResourceLocation>,
        index_map: &std::collections::HashMap<String, String>,
        namespace: &str,
        directory: &str,
        source_path: &str,
        extension: &str,
    ) {
        let expected_ext = if extension.starts_with('.') {
            extension.to_string()
        } else {
            format!(".{extension}")
        };
        let prefix = if directory.is_empty() {
            format!("{namespace}/")
        } else if source_path.is_empty() {
            format!("{namespace}/{directory}/")
        } else {
            format!("{namespace}/{directory}/{}/", source_path.trim_matches('/'))
        };
        for key in index_map.keys() {
            if let Some(rest) = key.strip_prefix(&prefix) {
                if let Some(stem) = rest.strip_suffix(&expected_ext) {
                    if stem.is_empty() || stem.ends_with('/') {
                        continue;
                    }
                    let path = if source_path.is_empty() {
                        stem.to_string()
                    } else {
                        format!("{}/{}", source_path.trim_matches('/'), stem)
                    };
                    if is_valid_path(&path) {
                        ids.push(ResourceLocation::new(namespace, &path));
                    }
                }
            }
        }
    }
    fn extend_texture_ids_from_index_map(
        &self,
        output: &mut Vec<ResourceLocation>,
        index_map: &std::collections::HashMap<String, String>,
        namespace: &str,
        source_path: &str,
        id_prefix: &str,
    ) {
        let prefix = if source_path.is_empty() {
            format!("{namespace}/textures/")
        } else {
            format!("{namespace}/textures/{}/", source_path.trim_matches('/'))
        };
        for key in index_map.keys() {
            if let Some(relative) = key.strip_prefix(&prefix) {
                if let Some(id_suffix) = relative.strip_suffix(".png") {
                    if !id_suffix.is_empty() && !id_suffix.ends_with('/') {
                        let id_path = format!("{id_prefix}{id_suffix}").replace('\\', "/");
                        if is_valid_path(&id_path) {
                            output.push(ResourceLocation::new(namespace, &id_path));
                        }
                    }
                }
            }
        }
    }
    fn extend_ids_from_prepared_base(
        &self,
        output: &mut Vec<ResourceLocation>,
        namespace: &str,
        directory: &str,
        source_path: &str,
        extension: &str,
    ) -> Result<(), ResourceError> {
        let Some(resources) = &self.prepared_base_resources else {
            return Ok(());
        };
        extend_ids_from_prepared_resources(
            output,
            resources,
            namespace,
            directory,
            source_path,
            extension,
        )
    }
    fn extend_texture_ids_from_prepared_base(
        &self,
        output: &mut Vec<ResourceLocation>,
        namespace: &str,
        source_path: &str,
        id_prefix: &str,
    ) -> Result<(), ResourceError> {
        let Some(resources) = &self.prepared_base_resources else {
            return Ok(());
        };
        extend_texture_ids_from_prepared_resources(
            output,
            resources,
            namespace,
            source_path,
            id_prefix,
        )
    }
    fn extend_ids_from_pack_root(
        &self,
        output: &mut Vec<ResourceLocation>,
        root: &Path,
        namespace: &str,
        directory: &str,
        source_path: &str,
        extension: &str,
    ) -> Result<(), ResourceError> {
        let relative = Path::new("assets")
            .join(namespace)
            .join(directory)
            .join(source_path);
        if root.is_dir() {
            let base = root.join(&relative);
            if base.is_dir() {
                self.collect_resource_ids(namespace, source_path, &base, &base, extension, output)?;
            }
        } else if self.is_zip_pack_root_cached(root) {
            collect_zip_resource_ids(root, namespace, directory, source_path, extension, output)?;
        }
        Ok(())
    }
    fn extend_texture_ids_from_pack_root(
        &self,
        output: &mut Vec<ResourceLocation>,
        root: &Path,
        namespace: &str,
        source_path: &str,
        id_prefix: &str,
    ) -> Result<(), ResourceError> {
        let relative = Path::new("assets")
            .join(namespace)
            .join("textures")
            .join(source_path);
        if root.is_dir() {
            let base = root.join(&relative);
            if base.is_dir() {
                self.collect_texture_ids(namespace, id_prefix, &base, &base, output)?;
            }
        } else if self.is_zip_pack_root_cached(root) {
            collect_zip_texture_ids(root, namespace, source_path, id_prefix, output)?;
        }
        Ok(())
    }
    fn list_ids_from_directory(
        &self,
        namespace: &str,
        id_prefix: &str,
        base: &Path,
        extension: &str,
    ) -> Result<Vec<ResourceLocation>, ResourceError> {
        let mut ids = Vec::new();
        if !base.is_dir() {
            return Ok(ids);
        }
        let extension = extension.strip_prefix('.').unwrap_or(extension);
        self.collect_resource_ids(namespace, id_prefix, base, base, extension, &mut ids)?;
        ids.sort_by(|left, right| left.to_string().cmp(&right.to_string()));
        Ok(ids)
    }
    fn collect_resource_ids(
        &self,
        namespace: &str,
        id_prefix: &str,
        base: &Path,
        current: &Path,
        extension: &str,
        output: &mut Vec<ResourceLocation>,
    ) -> Result<(), ResourceError> {
        let entries = fs::read_dir(current).map_err(|source| ResourceError::Io {
            path: current.to_path_buf(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| ResourceError::Io {
                path: current.to_path_buf(),
                source,
            })?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|source| ResourceError::Io {
                path: path.clone(),
                source,
            })?;
            if file_type.is_dir() {
                self.collect_resource_ids(namespace, id_prefix, base, &path, extension, output)?;
            } else if path.extension().and_then(|extension| extension.to_str()) == Some(extension) {
                output.push(resource_id_from_file(namespace, id_prefix, base, &path)?);
            }
        }
        Ok(())
    }
    fn collect_texture_ids(
        &self,
        namespace: &str,
        id_prefix: &str,
        base: &Path,
        current: &Path,
        output: &mut Vec<ResourceLocation>,
    ) -> Result<(), ResourceError> {
        let entries = fs::read_dir(current).map_err(|source| ResourceError::Io {
            path: current.to_path_buf(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| ResourceError::Io {
                path: current.to_path_buf(),
                source,
            })?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|source| ResourceError::Io {
                path: path.clone(),
                source,
            })?;
            if file_type.is_dir() {
                self.collect_texture_ids(namespace, id_prefix, base, &path, output)?;
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("png") {
                let relative = path.strip_prefix(base).map_err(|_| {
                    ResourceError::InvalidLocation(format!(
                        "{}:{}",
                        namespace,
                        path.to_string_lossy()
                    ))
                })?;
                let mut id_path = Path::new(id_prefix).join(relative);
                id_path.set_extension("");
                let id_path = id_path.to_string_lossy().replace('\\', "/");
                output.push(ResourceLocation::new(namespace, &id_path));
            }
        }
        Ok(())
    }
}
fn prepared_resource_prefix(namespace: &str, directory: &str, source_path: &str) -> String {
    let mut prefix = format!("assets/{namespace}/{directory}");
    if !source_path.is_empty() {
        prefix.push('/');
        prefix.push_str(source_path);
    }
    prefix.push('/');
    prefix
}
fn extend_namespaces_from_prepared_resources(
    resources: &BTreeMap<String, el_platform_api::ResourceBytes>,
    namespaces: &mut BTreeSet<String>,
) {
    for path in resources.keys() {
        let mut segments = path.split('/');
        if segments.next() == Some("assets") {
            if let Some(namespace) = segments.next().filter(|value| !value.is_empty()) {
                namespaces.insert(namespace.to_string());
            }
        }
    }
}
fn extend_ids_from_prepared_resources(
    output: &mut Vec<ResourceLocation>,
    resources: &BTreeMap<String, el_platform_api::ResourceBytes>,
    namespace: &str,
    directory: &str,
    source_path: &str,
    extension: &str,
) -> Result<(), ResourceError> {
    let prefix = prepared_resource_prefix(namespace, directory, source_path);
    let extension = extension.strip_prefix('.').unwrap_or(extension);
    let suffix = format!(".{extension}");
    for path in resources.keys() {
        let Some(relative) = path.strip_prefix(&prefix) else {
            continue;
        };
        let Some(id_suffix) = relative.strip_suffix(&suffix) else {
            continue;
        };
        if id_suffix.is_empty() || id_suffix.ends_with('/') {
            continue;
        }
        let id_path = if source_path.is_empty() {
            id_suffix.to_string()
        } else {
            format!("{source_path}/{id_suffix}")
        };
        if is_valid_path(&id_path) {
            output.push(ResourceLocation::new(namespace, &id_path));
        }
    }
    Ok(())
}
fn extend_texture_ids_from_prepared_resources(
    output: &mut Vec<ResourceLocation>,
    resources: &BTreeMap<String, el_platform_api::ResourceBytes>,
    namespace: &str,
    source_path: &str,
    id_prefix: &str,
) -> Result<(), ResourceError> {
    let prefix = prepared_resource_prefix(namespace, "textures", source_path);
    for path in resources.keys() {
        let Some(relative) = path.strip_prefix(&prefix) else {
            continue;
        };
        let Some(id_suffix) = relative.strip_suffix(".png") else {
            continue;
        };
        if id_suffix.is_empty() || id_suffix.ends_with('/') {
            continue;
        }
        let id_path = format!("{id_prefix}{id_suffix}").replace('\\', "/");
        if is_valid_path(&id_path) {
            output.push(ResourceLocation::new(namespace, &id_path));
        }
    }
    Ok(())
}
fn typed_resource_relative_path(id: &ResourceLocation, directory: &str, extension: &str) -> String {
    if directory.is_empty() {
        format!("assets/{}/{}{}", id.namespace, id.path, extension)
    } else {
        format!(
            "assets/{}/{}/{}{}",
            id.namespace, directory, id.path, extension
        )
    }
}
fn typed_index_path(id: &ResourceLocation, directory: &str, extension: &str) -> String {
    if directory.is_empty() {
        format!("{}/{}{}", id.namespace, id.path, extension)
    } else {
        format!("{}/{}/{}{}", id.namespace, directory, id.path, extension)
    }
}
fn deduplicate_resource_ids(ids: &mut Vec<ResourceLocation>) {
    let mut seen = HashSet::new();
    ids.retain(|id| seen.insert(id.clone()));
    ids.sort_by(|left, right| left.to_string().cmp(&right.to_string()));
}
fn extend_namespaces_from_pack_root(
    root: &Path,
    namespaces: &mut BTreeSet<String>,
) -> Result<(), ResourceError> {
    if root.is_dir() {
        let assets = root.join("assets");
        if let Ok(entries) = fs::read_dir(&assets) {
            for entry in entries {
                let entry = entry.map_err(|source| ResourceError::Io {
                    path: assets.clone(),
                    source,
                })?;
                if entry
                    .file_type()
                    .map_err(|source| ResourceError::Io {
                        path: entry.path(),
                        source,
                    })?
                    .is_dir()
                {
                    namespaces.insert(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
    } else if is_zip_pack_root(root) {
        let file = fs::File::open(root).map_err(|source| ResourceError::Io {
            path: root.to_path_buf(),
            source,
        })?;
        let mut archive = zip::ZipArchive::new(file).map_err(|source| ResourceError::Io {
            path: root.to_path_buf(),
            source: std::io::Error::other(source),
        })?;
        for index in 0..archive.len() {
            let entry = archive
                .by_index(index)
                .map_err(|source| ResourceError::Io {
                    path: root.to_path_buf(),
                    source: std::io::Error::other(source),
                })?;
            let name = entry.name().replace('\\', "/");
            let Some(namespace) = name
                .strip_prefix("assets/")
                .and_then(|rest| rest.split('/').next())
            else {
                continue;
            };
            if !namespace.is_empty() {
                namespaces.insert(namespace.to_string());
            }
        }
    }
    Ok(())
}
fn collect_zip_resource_ids(
    zip_path: &Path,
    namespace: &str,
    directory: &str,
    source_path: &str,
    extension: &str,
    output: &mut Vec<ResourceLocation>,
) -> Result<(), ResourceError> {
    let file = fs::File::open(zip_path).map_err(|source| ResourceError::Io {
        path: zip_path.to_path_buf(),
        source,
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(|source| ResourceError::Io {
        path: zip_path.to_path_buf(),
        source: std::io::Error::other(source),
    })?;
    let prefix = format!("assets/{namespace}/{directory}/{source_path}")
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string();
    let extension = extension.strip_prefix('.').unwrap_or(extension);
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|source| ResourceError::Io {
                path: zip_path.to_path_buf(),
                source: std::io::Error::other(source),
            })?;
        let name = entry.name().replace('\\', "/");
        let Some(relative) = name.strip_prefix(&(prefix.clone() + "/")) else {
            continue;
        };
        if entry.is_dir() || !relative.ends_with(&format!(".{extension}")) {
            continue;
        }
        let path = relative[..relative.len() - extension.len() - 1].to_string();
        output.push(ResourceLocation::new(
            namespace,
            &format!("{source_path}/{path}")
                .trim_start_matches('/')
                .replace('\\', "/"),
        ));
    }
    Ok(())
}
fn collect_zip_texture_ids(
    zip_path: &Path,
    namespace: &str,
    source_path: &str,
    id_prefix: &str,
    output: &mut Vec<ResourceLocation>,
) -> Result<(), ResourceError> {
    let file = fs::File::open(zip_path).map_err(|source| ResourceError::Io {
        path: zip_path.to_path_buf(),
        source,
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(|source| ResourceError::Io {
        path: zip_path.to_path_buf(),
        source: std::io::Error::other(source),
    })?;
    let prefix = format!("assets/{namespace}/textures/{source_path}")
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|source| ResourceError::Io {
                path: zip_path.to_path_buf(),
                source: std::io::Error::other(source),
            })?;
        let name = entry.name().replace('\\', "/");
        let Some(relative) = name.strip_prefix(&(prefix.clone() + "/")) else {
            continue;
        };
        if entry.is_dir() || !relative.ends_with(".png") {
            continue;
        }
        let path = relative[..relative.len() - 4].to_string();
        output.push(ResourceLocation::new(
            namespace,
            &format!("{id_prefix}{path}").replace('\\', "/"),
        ));
    }
    Ok(())
}
fn is_assets_root(path: &Path) -> bool {
    path.join("minecraft").is_dir()
}
fn clean_hash(hash: &str) -> &str {
    let hash = hash.trim();
    if let Some(rest) = hash.strip_prefix("sha1:") {
        rest
    } else if let Some(rest) = hash.strip_prefix("sha256:") {
        rest
    } else {
        hash
    }
}
fn clean_and_validate_object_hash<'a>(
    hash: &'a str,
    index_path: &Path,
    virtual_path: &str,
) -> Result<&'a str, ResourceError> {
    let clean = clean_hash(hash);
    if (clean.len() != 40 && clean.len() != 64) || !clean.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ResourceError::InvalidAssetIndex {
            path: index_path.to_path_buf(),
            message: format!(
                "invalid object hash '{hash}' for '{virtual_path}': expected 40 or 64 hex characters"
            ),
        });
    }
    Ok(clean)
}
fn compute_and_verify_hash(bytes: &[u8], expected_clean: &str) -> (bool, String) {
    if expected_clean.len() == 64 {
        let actual = AssetReleaseId::sha256_of(bytes);
        let actual_str = actual.as_str();
        let actual_hex = actual_str.strip_prefix("sha256:").unwrap_or(actual_str);
        let matches = actual_hex.eq_ignore_ascii_case(expected_clean);
        (matches, actual_hex.to_string())
    } else {
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        let mut actual_hex = String::with_capacity(40);
        for b in digest {
            use std::fmt::Write as _;
            let _ = write!(actual_hex, "{b:02x}");
        }
        let matches = actual_hex.eq_ignore_ascii_case(expected_clean);
        (matches, actual_hex)
    }
}
fn verify_manifest_hash(bytes: &[u8], expected: &str) -> Result<(), ResourceError> {
    let expected = expected.trim();
    let (is_sha256, expected_hex) = if let Some(hex) = expected.strip_prefix("sha256:") {
        (true, hex)
    } else if let Some(hex) = expected.strip_prefix("sha1:") {
        (false, hex)
    } else if expected.len() == 64 {
        (true, expected)
    } else if expected.len() == 40 {
        (false, expected)
    } else {
        return Err(ResourceError::InvalidManifestHash {
            hash: expected.to_string(),
            message: format!(
                "expected manifest hash '{expected}' is neither a 40-character SHA-1 nor 64-character SHA-256 hex string"
            ),
        });
    };
    if (is_sha256 && expected_hex.len() != 64)
        || (!is_sha256 && expected_hex.len() != 40)
        || !expected_hex.chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err(ResourceError::InvalidManifestHash {
            hash: expected.to_string(),
            message: format!(
                "manifest hash '{expected}' contains invalid hex characters or length"
            ),
        });
    }
    if is_sha256 {
        let actual = AssetReleaseId::sha256_of(bytes);
        let actual_str = actual.as_str();
        let actual_hex = actual_str.strip_prefix("sha256:").unwrap_or(actual_str);
        if !actual_hex.eq_ignore_ascii_case(expected_hex) {
            return Err(ResourceError::ManifestHashMismatch {
                expected: expected.to_string(),
                actual: actual_str.to_string(),
            });
        }
    } else {
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        let mut actual_hex = String::with_capacity(40);
        for b in digest {
            use std::fmt::Write as _;
            let _ = write!(actual_hex, "{b:02x}");
        }
        if !actual_hex.eq_ignore_ascii_case(expected_hex) {
            return Err(ResourceError::ManifestHashMismatch {
                expected: expected.to_string(),
                actual: actual_hex,
            });
        }
    }
    Ok(())
}
impl ClientResourceKind {
    fn directory_and_extension(self) -> (&'static str, &'static str) {
        match self {
            ClientResourceKind::Atlas => ("atlases", ".json"),
            ClientResourceKind::BlockState => ("blockstates", ".json"),
            ClientResourceKind::Font => ("font", ".json"),
            ClientResourceKind::ItemDefinition => ("items", ".json"),
            ClientResourceKind::Lang => ("lang", ".json"),
            ClientResourceKind::Model => ("models", ".json"),
            ClientResourceKind::SoundMetadata => ("", ".json"),
            ClientResourceKind::Texture => ("textures", ".png"),
            ClientResourceKind::WaypointStyle => ("waypoint_style", ".json"),
        }
    }
}
impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceError::ProjectRootNotFound => {
                write!(
                    f,
                    "could not find project root containing assets/assets or src/assets"
                )
            }
            ResourceError::InvalidLocation(id) => {
                write!(f, "invalid resource location: {}", id)
            }
            ResourceError::PreparedSnapshot { operation, message } => {
                write!(
                    f,
                    "prepared resource snapshot could not {operation}: {message}"
                )
            }
            ResourceError::Io { path, source } => {
                write!(f, "{}: {}", path.display(), source)
            }
            ResourceError::Image { path, source } => {
                write!(f, "{}: {}", path.display(), source)
            }
            ResourceError::Parse { id, message } => {
                write!(f, "parse error for {}: {}", id, message)
            }
            ResourceError::MissingAssetsDirectory => {
                write!(
                    f,
                    "assets directory was not provided; specify --assetsDir or run in developer mode with --offlineDeveloperMode"
                )
            }
            ResourceError::MissingAssetIndexName => {
                write!(
                    f,
                    "An explicit asset index is required; use --assetIndex or a prepared-resource receipt"
                )
            }
            ResourceError::MissingAssetIndex {
                expected_path,
                index_name,
            } => {
                write!(
                    f,
                    "required asset index '{index_name}' was not found at {}",
                    expected_path.display()
                )
            }
            ResourceError::InvalidAssetIndex { path, message } => {
                write!(
                    f,
                    "failed to parse asset index at {}: {}",
                    path.display(),
                    message
                )
            }
            ResourceError::ManifestHashMismatch { expected, actual } => {
                write!(
                    f,
                    "asset manifest content hash mismatch: expected {expected}, actual {actual}"
                )
            }
            ResourceError::MissingAssetObject {
                object_hash,
                expected_path,
            } => {
                write!(
                    f,
                    "required asset object '{object_hash}' was missing at {}",
                    expected_path.display()
                )
            }
            ResourceError::CorruptAssetObject {
                object_hash,
                expected_path,
                message,
            } => {
                write!(
                    f,
                    "corrupt asset object '{object_hash}' at {}: {message}",
                    expected_path.display()
                )
            }
            ResourceError::InvalidManifestHash { hash, message } => {
                write!(f, "invalid asset manifest hash '{hash}': {message}")
            }
            ResourceError::SnapshotNotFound { key_description } => {
                write!(
                    f,
                    "asset release snapshot '{key_description}' was not found in asset store"
                )
            }
            ResourceError::AssetStoreError { operation, message } => {
                write!(f, "asset store error during {operation}: {message}")
            }
        }
    }
}
impl std::error::Error for ResourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ResourceError::Io { source, .. } => Some(source),
            ResourceError::Image { source, .. } => Some(source),
            _ => None,
        }
    }
}
fn validate_location(id: &ResourceLocation) -> Result<(), ResourceError> {
    if is_valid_namespace(&id.namespace) && is_valid_path(&id.path) {
        Ok(())
    } else {
        Err(ResourceError::InvalidLocation(id.to_string()))
    }
}
fn validate_namespace(namespace: &str) -> Result<(), ResourceError> {
    if is_valid_namespace(namespace) {
        Ok(())
    } else {
        Err(ResourceError::InvalidLocation(format!(
            "{}:placeholder",
            namespace
        )))
    }
}
fn validate_relative_resource_path(value: &str) -> Result<(), ResourceError> {
    if value.is_empty() || is_valid_path(value) {
        Ok(())
    } else {
        Err(ResourceError::InvalidLocation(format!(
            "minecraft:{}",
            value
        )))
    }
}
fn is_valid_namespace(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| matches!(byte, b'a'..= b'z' | b'0'..= b'9' | b'_' | b'-' | b'.'))
}
fn is_valid_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains("..")
        && !value.starts_with('/')
        && !value.ends_with('/')
        && value
            .bytes()
            .all(|byte| matches!(byte, b'a'..= b'z' | b'0'..= b'9' | b'_' | b'-' | b'.' | b'/'))
}
fn resource_id_from_file(
    namespace: &str,
    id_prefix: &str,
    base: &Path,
    path: &Path,
) -> Result<ResourceLocation, ResourceError> {
    let relative = path.strip_prefix(base).map_err(|_| {
        ResourceError::InvalidLocation(format!("{}:{}", namespace, path.to_string_lossy()))
    })?;
    let mut id_path = Path::new(id_prefix).join(relative);
    id_path.set_extension("");
    let id_path = id_path.to_string_lossy().replace('\\', "/");
    Ok(ResourceLocation::new(namespace, &id_path))
}
