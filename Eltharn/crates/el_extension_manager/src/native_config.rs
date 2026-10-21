use crate::store_io;
use el_mod_runtime::config::{self, Receipt, Record, Store, Write};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write as IoWrite},
    path::PathBuf,
};
#[derive(Clone, Debug)]
pub struct NativeConfigStore {
    root: PathBuf,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    schema_version: u32,
    owner: String,
    record: Record,
}
impl NativeConfigStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    fn path(&self, owner: &str) -> Result<PathBuf, String> {
        config::storage_key(owner)?;
        let digest = Sha256::digest(owner.as_bytes());
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        Ok(self.root.join(format!("{hex}.json")))
    }
    fn read(&self, owner: &str) -> Result<Record, String> {
        let path = self.path(owner)?;
        let file = match store_io::open(&path, false) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Record::default());
            }
            Err(e) => return Err(format!("Cannot open mod configuration: {e}")),
        };
        let meta = file.metadata().map_err(|e| e.to_string())?;
        if !meta.file_type().is_file() || meta.len() > (config::MAX_BYTES + 1024) as u64 {
            return Err("Invalid configuration file size/type".into());
        }
        let mut bytes = Vec::new();
        file.take((config::MAX_BYTES + 1025) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        let value = el_mod_runtime::wire::strict_json(&bytes, config::MAX_BYTES + 1024)?;
        let envelope: Envelope = serde_json::from_value(value).map_err(|e| e.to_string())?;
        if envelope.schema_version != 1 || envelope.owner != owner {
            return Err("Configuration schema/owner mismatch".into());
        }
        envelope.record.validate()?;
        Ok(envelope.record)
    }
    fn lock(&self) -> Result<fs::File, String> {
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let file =
            store_io::open(&self.root.join("store.lock"), true).map_err(|e| e.to_string())?;
        if !file.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err("Configuration lock is not a regular file".into());
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(250);
        loop {
            match file.try_lock() {
                Ok(()) => break,
                Err(fs::TryLockError::WouldBlock) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(fs::TryLockError::WouldBlock) => {
                    return Err("Configuration store is busy; retry the setting change".into());
                }
                Err(fs::TryLockError::Error(error)) => return Err(error.to_string()),
            }
        }
        Ok(file)
    }
}
impl Store for NativeConfigStore {
    fn load(&mut self, owner: &str) -> Result<Record, String> {
        self.read(owner)
    }
    fn compare_and_set(&mut self, write: &Write) -> Result<Receipt, String> {
        let next = write.record()?;
        let path = self.path(&write.owner)?;
        let bytes = serde_json::to_vec(&Envelope {
            schema_version: 1,
            owner: write.owner.clone(),
            record: next.clone(),
        })
        .map_err(|e| e.to_string())?;
        let _lock = self.lock()?;
        let current = self.read(&write.owner)?;
        if current.revision != write.expected {
            return Ok(Receipt {
                record: current,
                conflict: true,
            });
        }
        let parent = store_io::parent(&self.root).map_err(|e| e.to_string())?;
        let mut temp = tempfile::NamedTempFile::new_in(&parent.path).map_err(|e| e.to_string())?;
        temp.write_all(&bytes).map_err(|e| e.to_string())?;
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        temp.persist(
            parent
                .path
                .join(path.file_name().ok_or("Missing configuration filename")?),
        )
        .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        fs::File::open(&parent.path)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(Receipt {
            record: next,
            conflict: false,
        })
    }
}
