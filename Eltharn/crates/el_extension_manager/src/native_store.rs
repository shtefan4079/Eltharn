use crate::{archive::MAX_ARCHIVE_BYTES, package::PackageInfo};
use el_launcher_core::extensions::{ExtensionPreferences, ExtensionProfile, PackagePin};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct NativeStore {
    root: PathBuf,
}
fn read(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let file =
        crate::store_io::open(path, false).map_err(|e| format!("{}: {e}", path.display()))?;
    let meta = file
        .metadata()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if !meta.file_type().is_file() || meta.len() > limit as u64 {
        return Err(format!(
            "{}: expected a bounded regular file",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("File exceeds size limit".into());
    }
    Ok(bytes)
}
fn optional_read(path: &Path, limit: usize) -> Result<Option<Vec<u8>>, String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
        Ok(_) => read(path, limit).map(Some),
    }
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Missing store directory")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let parent = crate::store_io::parent(parent).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(&parent.path).map_err(|e| e.to_string())?;
    temp.write_all(bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(
        parent
            .path
            .join(path.file_name().ok_or("Missing destination name")?),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
impl NativeStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    fn lock(&self) -> Result<fs::File, String> {
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let file = crate::store_io::open(&self.root.join("store.lock"), true)
            .map_err(|e| e.to_string())?;
        if !file.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err("Store lock is not a regular file".into());
        }
        file.lock().map_err(|e| e.to_string())?;
        Ok(file)
    }
    pub fn preferences(&self) -> Result<ExtensionPreferences, String> {
        match optional_read(&self.root.join("profiles.json"), 262144)? {
            None => Ok(ExtensionPreferences::default()),
            Some(b) => ExtensionPreferences::decode(&b),
        }
    }
    pub fn save_preferences(&self, prefs: &ExtensionPreferences) -> Result<(), String> {
        self.save_preferences_inner(None, prefs)
    }
    pub fn compare_and_save_preferences(
        &self,
        previous: &ExtensionPreferences,
        prefs: &ExtensionPreferences,
    ) -> Result<(), String> {
        self.save_preferences_inner(Some(previous), prefs)
    }
    fn save_preferences_inner(
        &self,
        previous: Option<&ExtensionPreferences>,
        prefs: &ExtensionPreferences,
    ) -> Result<(), String> {
        prefs.validate()?;
        let bytes = serde_json::to_vec_pretty(prefs).map_err(|e| e.to_string())?;
        ExtensionPreferences::decode(&bytes)?;
        let _lock = self.lock()?;
        if let Some(previous) = previous {
            if &self.preferences()? != previous {
                return Err(
                    "Profiles changed in another EltharnGate window. Reopen settings before editing."
                        .into(),
                );
            }
        }
        atomic_write(&self.root.join("profiles.json"), &bytes)
    }
    pub fn catalog(&self) -> Result<Vec<PackageInfo>, String> {
        optional_read(
            &self.root.join("catalog.json"),
            crate::catalog::MAX_CATALOG_BYTES,
        )?
        .map(|b| crate::catalog::decode(&b))
        .unwrap_or_else(|| Ok(Vec::new()))
    }
    fn write_catalog(&self, entries: &[PackageInfo]) -> Result<(), String> {
        crate::catalog::validate(entries)?;
        let bytes = serde_json::to_vec(entries).map_err(|e| e.to_string())?;
        crate::catalog::decode(&bytes)?;
        atomic_write(&self.root.join("catalog.json"), &bytes)
    }
    fn archive_path(&self, pin: &PackagePin) -> PathBuf {
        self.root.join("packages").join(format!(
            "{}.zip",
            pin.content_id.to_string().trim_start_matches("sha256:")
        ))
    }
    pub fn read_import(path: &Path) -> Result<Vec<u8>, String> {
        read(path, MAX_ARCHIVE_BYTES)
    }
    pub async fn import(&self, bytes: Vec<u8>) -> Result<PackageInfo, String> {
        let package = crate::package::inspect(bytes).await?;
        let _lock = self.lock()?;
        let mut catalog = self.catalog()?;
        if !catalog.iter().any(|p| p.pin == package.info.pin) {
            catalog.push(package.info.clone());
        }
        crate::catalog::validate(&catalog)?;
        atomic_write(&self.archive_path(&package.info.pin), &package.archive)?;
        self.write_catalog(&catalog)?;
        Ok(package.info)
    }
    pub async fn replace_library(
        &self,
        previous: &ExtensionPreferences,
        prefs: &ExtensionPreferences,
        archives: Vec<Vec<u8>>,
    ) -> Result<Vec<PackageInfo>, String> {
        if archives.len() > 256
            || archives.iter().map(|bytes| bytes.len() as u64).sum::<u64>() > 128 * 1024 * 1024
        {
            return Err("Installed library exceeds 256 archives or 128 MiB".into());
        }
        let mut packages = Vec::with_capacity(archives.len());
        for bytes in archives {
            packages.push(crate::package::inspect(bytes).await?);
        }
        let catalog: Vec<_> = packages
            .iter()
            .map(|package| package.info.clone())
            .collect();
        crate::catalog::validate(&catalog)?;
        let catalog_bytes = serde_json::to_vec(&catalog).map_err(|e| e.to_string())?;
        crate::catalog::decode(&catalog_bytes)?;
        let prefs_bytes = serde_json::to_vec_pretty(prefs).map_err(|e| e.to_string())?;
        ExtensionPreferences::decode(&prefs_bytes)?;
        for profile in &prefs.profiles {
            let missing_mod = profile.mods.iter().any(|pin| {
                !catalog
                    .iter()
                    .any(|info| &info.pin == pin && info.kind == "mod")
            });
            let missing_shader = profile.shader.as_ref().is_some_and(|shader| {
                !catalog
                    .iter()
                    .any(|info| info.pin == shader.package && info.kind == "shaderpack")
            });
            if missing_mod || missing_shader {
                return Err("Selected packages are missing from the installed library".into());
            }
        }
        let _lock = self.lock()?;
        if &self.preferences()? != previous {
            return Err(
                "Profiles changed in another EltharnGate window. Refresh the library before editing."
                    .into(),
            );
        }
        let old_catalog = self.catalog()?;
        for package in &packages {
            atomic_write(&self.archive_path(&package.info.pin), &package.archive)?;
        }
        self.write_catalog(&catalog)?;
        if let Err(error) = atomic_write(&self.root.join("profiles.json"), &prefs_bytes) {
            if let Err(rollback) = self.write_catalog(&old_catalog) {
                return Err(format!(
                    "{error}; could not restore the prior catalog: {rollback}"
                ));
            }
            return Err(error);
        }
        Ok(catalog)
    }
    pub fn remove(&self, pin: &PackagePin) -> Result<(), String> {
        let _lock = self.lock()?;
        let prefs = self.preferences()?;
        if prefs.profiles.iter().any(|p| {
            p.mods.iter().any(|m| m == pin) || p.shader.as_ref().is_some_and(|s| &s.package == pin)
        }) {
            return Err("This version is selected in a profile. Deselect it first.".into());
        }
        let mut catalog = self.catalog()?;
        catalog.retain(|p| &p.pin != pin);
        self.write_catalog(&catalog)?;
        let archive = self.archive_path(pin);
        let parent = crate::store_io::parent(archive.parent().ok_or("Missing archive directory")?)
            .map_err(|e| e.to_string())?;
        match fs::remove_file(
            parent
                .path
                .join(archive.file_name().ok_or("Missing archive name")?),
        ) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!(
                "Package removed from library; archive cleanup failed: {e}"
            )),
        }
    }
    pub fn archives(
        &self,
        profile: &ExtensionProfile,
    ) -> Result<BTreeMap<String, Vec<u8>>, String> {
        let mut pins = Vec::new();
        if profile.flags.mods_enabled {
            pins.extend(profile.mods.iter());
        }
        if profile.flags.shaders_enabled {
            if let Some(s) = &profile.shader {
                pins.push(&s.package);
            }
        }
        let mut out = BTreeMap::new();
        if pins.is_empty() {
            return Ok(out);
        }
        let catalog = self.catalog()?;
        let mut total = 0usize;
        for pin in pins {
            if !catalog.iter().any(|p| &p.pin == pin) {
                return Err(format!(
                    "Missing selected package {} {}",
                    pin.id, pin.version
                ));
            }
            let bytes = read(&self.archive_path(pin), MAX_ARCHIVE_BYTES)?;
            total = total
                .checked_add(bytes.len())
                .ok_or("Archive total overflow")?;
            if total > MAX_ARCHIVE_BYTES {
                return Err("Selected packages exceed 32 MiB".into());
            }
            out.insert(pin.content_id.to_string(), bytes);
        }
        Ok(out)
    }
    pub fn save_launch(&self, bytes: &[u8]) -> Result<PathBuf, String> {
        if bytes.len() > crate::launch::MAX_LAUNCH_BYTES {
            return Err("Launch request is too large".into());
        }
        let hash = crate::launch::digest(bytes).to_string();
        let dir = self.root.join("launches");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let parent = crate::store_io::parent(&dir).map_err(|e| e.to_string())?;
        let mut temp = tempfile::Builder::new()
            .prefix(&format!("{}-", &hash[7..19]))
            .suffix(".json")
            .tempfile_in(&parent.path)
            .map_err(|e| e.to_string())?;
        temp.write_all(bytes).map_err(|e| e.to_string())?;
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        let (_, path) = temp.keep().map_err(|e| e.to_string())?;
        fs::canonicalize(path).map_err(|e| e.to_string())
    }
}
