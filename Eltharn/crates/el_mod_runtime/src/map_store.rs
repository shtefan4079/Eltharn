use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::collections::BTreeMap;
pub const TILE_SIDE: usize = 32;
pub const TILE_BYTES: usize = TILE_SIDE * TILE_SIDE * 8;
pub const DEFAULT_QUOTA: u64 = 512 * 1024 * 1024;
#[cfg(not(target_arch = "wasm32"))]
const MAX_TILES: usize = 65536;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapMode {
    Surface,
    Cave,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileKey {
    pub scope: String,
    pub dimension: String,
    pub mode: MapMode,
    pub level: i32,
    pub x: i32,
    pub z: i32,
}
impl TileKey {
    pub fn validate(&self) -> Result<(), String> {
        if self.scope.is_empty()
            || self.scope.len() > 128
            || !self
                .scope
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            || self.dimension.len() > 256
            || self
                .dimension
                .parse::<el_extension_api::ExtensionId>()
                .is_err()
            || self.level.abs_diff(0) > 32768
            || self.x.abs_diff(0) > 1_000_000
            || self.z.abs_diff(0) > 1_000_000
        {
            return Err("Invalid map tile key".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileData {
    pub rgba: Vec<u8>,
    pub heights: Vec<i16>,
    pub light: Vec<u8>,
    pub flags: Vec<u8>,
}
impl TileData {
    pub fn validate(&self) -> Result<(), String> {
        if self.rgba.len() != 4096
            || self.heights.len() != 1024
            || self.light.len() != 1024
            || self.flags.len() != 1024
        {
            Err("Map tile must contain 32x32 material samples".into())
        } else {
            Ok(())
        }
    }
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let mut bytes = Vec::with_capacity(TILE_BYTES);
        bytes.extend_from_slice(&self.rgba);
        for height in &self.heights {
            bytes.extend_from_slice(&height.to_le_bytes());
        }
        bytes.extend_from_slice(&self.light);
        bytes.extend_from_slice(&self.flags);
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != TILE_BYTES {
            return Err("Invalid map tile bytes".into());
        }
        Ok(Self {
            rgba: bytes[..4096].to_vec(),
            heights: bytes[4096..6144]
                .chunks_exact(2)
                .map(|c| i16::from_le_bytes([c[0], c[1]]))
                .collect(),
            light: bytes[6144..7168].to_vec(),
            flags: bytes[7168..].to_vec(),
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileMeta {
    pub key: TileKey,
    pub revision: u64,
    pub material_revision: u64,
    pub bytes: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileRecord {
    pub meta: TileMeta,
    pub data: TileData,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreStats {
    pub revision: u64,
    pub bytes: u64,
    pub tiles: usize,
    pub quota: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Receipt {
    Saved(StoreStats),
    Conflict(StoreStats),
}
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    revision: u64,
    bytes: u64,
    tiles: BTreeMap<String, TileMeta>,
}
#[cfg(not(target_arch = "wasm32"))]
pub mod native {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::{
        fs::{self, File, OpenOptions},
        io::{Read, Write},
        path::{Path, PathBuf},
        time::{Duration, Instant},
    };
    const MAX_INDEX_BYTES: u64 = 32 * 1024 * 1024;
    fn hash(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
    fn id(key: &TileKey) -> Result<String, String> {
        key.validate()?;
        Ok(hash(&serde_json::to_vec(key).map_err(|e| e.to_string())?))
    }
    fn io(error: std::io::Error) -> String {
        format!("Map storage: {}", error.kind())
    }
    pub struct NativeStore {
        root: PathBuf,
        quota: u64,
    }
    struct Lock {
        _file: File,
    }
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Pending {
        index: Index,
        tile_id: String,
        erase: bool,
    }
    impl NativeStore {
        pub fn new(root: PathBuf, owner: &str, quota: u64) -> Result<Self, String> {
            if owner.is_empty()
                || owner.len() > 160
                || quota < TILE_BYTES as u64
                || quota > 8 * 1024 * 1024 * 1024
            {
                return Err("Invalid map storage owner/quota".into());
            }
            Ok(Self {
                root: root.join(hash(owner.as_bytes())),
                quota,
            })
        }
        fn lock(&self) -> Result<Lock, String> {
            fs::create_dir_all(&self.root).map_err(io)?;
            let path = self.root.join("lock");
            let deadline = Instant::now() + Duration::from_millis(250);
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&path)
                .map_err(io)?;
            loop {
                match file.try_lock() {
                    Ok(()) => return Ok(Lock { _file: file }),
                    Err(std::fs::TryLockError::WouldBlock) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(std::fs::TryLockError::WouldBlock) => {
                        return Err("Map storage lock deadline exceeded".into());
                    }
                    Err(std::fs::TryLockError::Error(e)) => return Err(io(e)),
                }
            }
        }
        fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
            let file = File::open(path).map_err(io)?;
            if file.metadata().map_err(io)?.len() > limit {
                return Err("Map storage size limit".into());
            }
            let mut bytes = vec![];
            file.take(limit + 1).read_to_end(&mut bytes).map_err(io)?;
            if bytes.len() as u64 > limit {
                return Err("Map storage size limit".into());
            }
            Ok(bytes)
        }
        fn atomic(&self, path: &str, bytes: &[u8]) -> Result<(), String> {
            let tmp = self.root.join(format!("{path}.new"));
            let target = self.root.join(path);
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&tmp)
                .map_err(io)?;
            file.write_all(bytes).map_err(io)?;
            file.sync_all().map_err(io)?;
            drop(file);
            fs::rename(tmp, target).map_err(io)?;
            Ok(())
        }
        fn validate_index(&self, index: &Index) -> Result<(), String> {
            if index.tiles.len() > MAX_TILES
                || index.bytes != index.tiles.len() as u64 * TILE_BYTES as u64
                || index.bytes > self.quota
            {
                return Err("Invalid map storage index/quota".into());
            }
            for (key, tile) in &index.tiles {
                if *key != id(&tile.key)?
                    || tile.bytes != TILE_BYTES as u64
                    || tile.revision == 0
                    || tile.revision > index.revision
                {
                    return Err("Invalid map storage tile metadata".into());
                }
            }
            Ok(())
        }
        fn recover(&self) -> Result<(), String> {
            let pending = self.root.join("pending.json");
            if !pending.exists() {
                return Ok(());
            }
            let bytes = Self::read_bounded(&pending, MAX_INDEX_BYTES)?;
            let p: Pending = crate::wire::strict_json(&bytes, MAX_INDEX_BYTES as usize)
                .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
                .map_err(|_| "Invalid map storage journal")?;
            self.validate_index(&p.index)?;
            if p.tile_id.len() != 64 || !p.tile_id.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("Invalid map journal key".into());
            }
            let target = self.root.join(format!("{}.tile", p.tile_id));
            if p.erase {
                if target.exists() {
                    fs::remove_file(&target).map_err(io)?;
                }
            } else {
                let stage = self.root.join("pending.tile");
                if stage.exists() {
                    if fs::metadata(&stage).map_err(io)?.len() != TILE_BYTES as u64 {
                        return Err("Invalid staged map tile".into());
                    }
                    fs::rename(stage, &target).map_err(io)?;
                }
                if fs::metadata(&target).map_err(io)?.len() != TILE_BYTES as u64 {
                    return Err("Missing committed map tile".into());
                }
            }
            self.atomic(
                "index.json",
                &serde_json::to_vec(&p.index).map_err(|e| e.to_string())?,
            )?;
            fs::remove_file(pending).map_err(io)?;
            Ok(())
        }
        fn index(&self) -> Result<Index, String> {
            self.recover()?;
            let path = self.root.join("index.json");
            let index = if path.exists() {
                let bytes = Self::read_bounded(&path, MAX_INDEX_BYTES)?;
                crate::wire::strict_json(&bytes, MAX_INDEX_BYTES as usize)
                    .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
                    .map_err(|_| "Invalid map storage index")?
            } else {
                Index::default()
            };
            self.validate_index(&index)?;
            Ok(index)
        }
        fn stats_for(&self, index: &Index) -> StoreStats {
            StoreStats {
                revision: index.revision,
                bytes: index.bytes,
                tiles: index.tiles.len(),
                quota: self.quota,
            }
        }
        pub fn stats(&self) -> Result<StoreStats, String> {
            let _lock = self.lock()?;
            Ok(self.stats_for(&self.index()?))
        }
        pub fn read(&self, key: &TileKey) -> Result<Option<TileRecord>, String> {
            let id = id(key)?;
            let _lock = self.lock()?;
            let index = self.index()?;
            let Some(meta) = index.tiles.get(&id) else {
                return Ok(None);
            };
            let data = TileData::decode(&Self::read_bounded(
                &self.root.join(format!("{id}.tile")),
                TILE_BYTES as u64,
            )?)?;
            Ok(Some(TileRecord {
                meta: meta.clone(),
                data,
            }))
        }
        pub fn page(
            &self,
            scope: &str,
            dimension: &str,
            mode: MapMode,
            level: i32,
            after: Option<(i32, i32)>,
            limit: usize,
        ) -> Result<Vec<TileMeta>, String> {
            if !(1..=16).contains(&limit) {
                return Err("Map index page limit".into());
            }
            let _lock = self.lock()?;
            let index = self.index()?;
            let mut found: Vec<_> = index
                .tiles
                .values()
                .filter(|m| {
                    m.key.scope == scope
                        && m.key.dimension == dimension
                        && m.key.mode == mode
                        && m.key.level == level
                        && after.is_none_or(|k| (m.key.x, m.key.z) > k)
                })
                .cloned()
                .collect();
            found.sort_by_key(|m| (m.key.x, m.key.z));
            found.truncate(limit);
            Ok(found)
        }
        pub fn compare_and_set(
            &self,
            expected: u64,
            key: &TileKey,
            data: Option<&TileData>,
            material_revision: u64,
        ) -> Result<Receipt, String> {
            if material_revision > 9_007_199_254_740_991 {
                return Err("Invalid map material revision".into());
            }
            let tile_id = id(key)?;
            let encoded = data.map(TileData::encode).transpose()?;
            let _lock = self.lock()?;
            let mut index = self.index()?;
            if index.revision != expected {
                return Ok(Receipt::Conflict(self.stats_for(&index)));
            }
            let revision = index
                .revision
                .checked_add(1)
                .filter(|n| *n <= 9_007_199_254_740_991)
                .ok_or("Map storage revision exhausted")?;
            if let Some(encoded) = &encoded {
                if !index.tiles.contains_key(&tile_id) {
                    index.bytes = index
                        .bytes
                        .checked_add(TILE_BYTES as u64)
                        .ok_or("Map storage size overflow")?;
                }
                if index.bytes > self.quota
                    || index.tiles.len() >= MAX_TILES && !index.tiles.contains_key(&tile_id)
                {
                    return Err("Map storage quota exceeded".into());
                }
                index.tiles.insert(
                    tile_id.clone(),
                    TileMeta {
                        key: key.clone(),
                        revision,
                        material_revision,
                        bytes: TILE_BYTES as u64,
                    },
                );
                self.atomic("pending.tile", encoded)?;
            } else if index.tiles.remove(&tile_id).is_some() {
                index.bytes -= TILE_BYTES as u64;
            }
            index.revision = revision;
            let pending = Pending {
                index: index.clone(),
                tile_id,
                erase: encoded.is_none(),
            };
            self.atomic(
                "pending.json",
                &serde_json::to_vec(&pending).map_err(|e| e.to_string())?,
            )?;
            self.recover()?;
            Ok(Receipt::Saved(self.stats_for(&index)))
        }
    }
}
