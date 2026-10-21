use super::CatalogTab;
use el_extension_manager::{
    catalog,
    native_store::NativeStore,
    package::{self, PackageInfo},
};
use el_launcher_core::extensions::{ExtensionPreferences, ShaderSelection};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};
const MAX_FILES: usize = 256;
const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;
const MARKER: &str = "folder-library.json";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
pub(super) type Fingerprint = Vec<(PathBuf, u64, Option<SystemTime>)>;
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub prefs: ExtensionPreferences,
    pub catalog: Vec<PackageInfo>,
    pub fingerprint: Fingerprint,
}
pub(super) fn folder(root: &Path, tab: CatalogTab) -> PathBuf {
    root.parent().unwrap_or(root).join(match tab {
        CatalogTab::Mods => "mods",
        CatalogTab::ShaderPacks => "shaderpacks",
    })
}
fn directory(path: &Path, create: bool) -> Result<bool, String> {
    let mut current = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                return Err("Extension folder must not contain '..'".into());
            }
            Component::CurDir => continue,
            _ => current.push(component.as_os_str()),
        }
        let metadata = match fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && create => {
                match fs::create_dir(&current) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(e) => return Err(format!("{}: {e}", current.display())),
                }
                fs::symlink_metadata(&current).map_err(|e| e.to_string())?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(format!("{}: {e}", current.display())),
        };
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(format!(
                "{}: expected a directory, without symlinks",
                current.display()
            ));
        }
    }
    Ok(true)
}
pub(super) fn fingerprint(root: &Path) -> Result<Fingerprint, String> {
    let mut files = Vec::new();
    let mut total = 0u64;
    for tab in [CatalogTab::Mods, CatalogTab::ShaderPacks] {
        let path = folder(root, tab);
        if !directory(&path, false)? {
            continue;
        }
        for entry in fs::read_dir(&path).map_err(|e| format!("{}: {e}", path.display()))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if !path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
            {
                continue;
            }
            let metadata =
                fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(format!(
                    "{}: ZIP must be a regular file, without symlinks",
                    path.display()
                ));
            }
            if metadata.len() > MAX_FILE_BYTES {
                return Err(format!("{}: ZIP exceeds 32 MiB", path.display()));
            }
            total = total
                .checked_add(metadata.len())
                .ok_or("ZIP size overflow")?;
            if total > MAX_TOTAL_BYTES {
                return Err("Mods and shaderpacks folders exceed 128 MiB in total".into());
            }
            if files.len() >= MAX_FILES {
                return Err("Mods and shaderpacks folders contain more than 256 ZIPs".into());
            }
            files.push((path, metadata.len(), metadata.modified().ok()));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}
fn kind(tab: CatalogTab) -> &'static str {
    match tab {
        CatalogTab::Mods => "mod",
        CatalogTab::ShaderPacks => "shaderpack",
    }
}
struct Archive {
    info: PackageInfo,
    bytes: Vec<u8>,
}
fn inspect(root: &Path, files: &Fingerprint) -> Result<Vec<Archive>, String> {
    let mut archives = Vec::new();
    let mut ids = BTreeMap::<String, (PathBuf, PackageInfo)>::new();
    for (path, size, _) in files {
        let bytes =
            NativeStore::read_import(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if bytes.len() as u64 != *size {
            return Err(format!("{} changed during scan; try again", path.display()));
        }
        let info = pollster::block_on(package::inspect(bytes.clone()))
            .map_err(|e| format!("{}: {e}", path.display()))?
            .info;
        let tab = if path.parent() == Some(folder(root, CatalogTab::Mods).as_path()) {
            CatalogTab::Mods
        } else {
            CatalogTab::ShaderPacks
        };
        if info.kind != kind(tab) {
            return Err(format!(
                "{} contains a {} package; move it to the {} folder",
                path.display(),
                info.kind,
                if info.kind == "mod" {
                    "mods"
                } else {
                    "shaderpacks"
                },
            ));
        }
        if let Some((previous_path, previous)) = ids.get(&info.pin.id.to_string()) {
            if previous.pin != info.pin {
                return Err(format!(
                    "Two different packages use ID {}: {} ({}) and {} ({}). Keep only the ZIP version you want.",
                    info.pin.id,
                    previous_path.display(),
                    previous.pin.version,
                    path.display(),
                    info.pin.version,
                ));
            }
            continue;
        }
        ids.insert(info.pin.id.to_string(), (path.clone(), info.clone()));
        archives.push(Archive { info, bytes });
    }
    Ok(archives)
}
fn read_marker(root: &Path) -> Result<Option<Vec<PackageInfo>>, String> {
    let path = root.join(MARKER);
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
        Ok(metadata) => {
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() > catalog::MAX_CATALOG_BYTES as u64
            {
                return Err(format!("{}: invalid folder library marker", path.display()));
            }
            catalog::decode(&NativeStore::read_import(&path)?)
                .map(Some)
                .map_err(|e| format!("{}: {e}", path.display()))
        }
    }
}
struct MarkerWrite {
    temporary: PathBuf,
    destination: PathBuf,
}
impl MarkerWrite {
    fn prepare(root: &Path, entries: &[PackageInfo]) -> Result<Self, String> {
        let bytes = serde_json::to_vec(entries).map_err(|e| e.to_string())?;
        catalog::decode(&bytes)?;
        directory(root, true)?;
        for _ in 0..16 {
            let temporary = root.join(format!(
                ".folder-library-{}-{}.tmp",
                std::process::id(),
                TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            let mut file = match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
            {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(format!("{}: {e}", temporary.display())),
            };
            let write = Self {
                temporary,
                destination: root.join(MARKER),
            };
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|e| e.to_string())?;
            return Ok(write);
        }
        Err("Could not create folder library marker; try again".into())
    }
    fn commit(self) -> Result<(), String> {
        fs::rename(&self.temporary, &self.destination)
            .map_err(|e| format!("Save folder library marker: {e}"))
    }
}
impl Drop for MarkerWrite {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.temporary);
    }
}
fn lock(root: &Path) -> Result<File, String> {
    directory(root, true)?;
    let path = root.join("folder-library.lock");
    if let Ok(metadata) = fs::symlink_metadata(&path) {
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(format!("{}: expected a regular lock file", path.display()));
        }
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x0020_0000);
    }
    let file = options
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Folder library lock must be a regular file".into());
    }
    file.try_lock()
        .map_err(|e| format!("Another folder library scan is in progress; try again: {e}"))?;
    Ok(file)
}
fn unchanged(store: &NativeStore, expected: &ExtensionPreferences) -> Result<(), String> {
    if &store.preferences()? != expected {
        return Err(
            "Profiles changed in another EltharnGate window. Refresh the library before editing."
                .into(),
        );
    }
    Ok(())
}
fn migrate(
    root: &Path,
    store: &NativeStore,
    expected: &ExtensionPreferences,
) -> Result<Vec<PackageInfo>, String> {
    let cached = store.catalog()?;
    let files = fingerprint(root)?;
    let present = inspect(root, &files)?;
    let present_ids: BTreeSet<_> = present.iter().map(|a| a.info.pin.id.to_string()).collect();
    let mut copies = Vec::new();
    for info in &cached {
        if present_ids.contains(&info.pin.id.to_string()) {
            continue;
        }
        let path = root.join("packages").join(format!(
            "{}.zip",
            info.pin
                .content_id
                .to_string()
                .trim_start_matches("sha256:")
        ));
        let bytes = NativeStore::read_import(&path)?;
        let package = pollster::block_on(package::inspect(bytes.clone()))?;
        if package.info != *info {
            return Err(format!(
                "Cached package {} {} does not match its archive",
                info.pin.id, info.pin.version
            ));
        }
        let tab = if info.kind == "mod" {
            CatalogTab::Mods
        } else {
            CatalogTab::ShaderPacks
        };
        let destination = folder(root, tab).join(format!(
            "{}.zip",
            info.pin
                .content_id
                .to_string()
                .trim_start_matches("sha256:")
        ));
        copies.push((destination, bytes));
    }
    let count = files
        .len()
        .checked_add(copies.len())
        .ok_or("ZIP count overflow")?;
    let total = files.iter().map(|f| f.1).sum::<u64>()
        + copies.iter().map(|(_, b)| b.len() as u64).sum::<u64>();
    if count > MAX_FILES || total > MAX_TOTAL_BYTES {
        return Err(
            "Migrating the cached library would exceed 256 ZIPs or 128 MiB. Move unused cached ZIPs out of the library first."
                .into(),
        );
    }
    unchanged(store, expected)?;
    for (destination, bytes) in copies {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
        {
            Ok(mut file) => {
                if let Err(e) = file.write_all(&bytes).and_then(|_| file.sync_all()) {
                    let _ = fs::remove_file(&destination);
                    return Err(format!("{}: {e}", destination.display()));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let bytes = NativeStore::read_import(&destination)?;
                pollster::block_on(package::inspect(bytes))?;
            }
            Err(e) => return Err(format!("{}: {e}", destination.display())),
        }
    }
    if !cached.is_empty() {
        unchanged(store, expected)?;
        MarkerWrite::prepare(root, &cached)?.commit()?;
    }
    Ok(cached)
}
fn reconcile(
    expected: &ExtensionPreferences,
    previous: &[PackageInfo],
    current: &[PackageInfo],
) -> Result<ExtensionPreferences, String> {
    let old_ids: BTreeSet<_> = previous
        .iter()
        .map(|info| info.pin.id.to_string())
        .collect();
    let mods: BTreeMap<_, _> = current
        .iter()
        .filter(|info| info.kind == "mod")
        .map(|info| (info.pin.id.to_string(), &info.pin))
        .collect();
    let shaders: BTreeMap<_, _> = current
        .iter()
        .filter(|info| info.kind == "shaderpack")
        .map(|info| (info.pin.id.to_string(), &info.pin))
        .collect();
    let mut next = expected.clone();
    for profile in &mut next.profiles {
        profile.mods = profile
            .mods
            .iter()
            .filter_map(|pin| mods.get(&pin.id.to_string()).map(|new| (*new).clone()))
            .collect();
        profile.shader = profile.shader.as_ref().and_then(|selected| {
            shaders
                .get(&selected.package.id.to_string())
                .map(|pin| ShaderSelection {
                    package: (*pin).clone(),
                    settings: selected.settings.clone(),
                })
        });
    }
    let active = next.active_mut()?;
    for (id, pin) in mods {
        if !old_ids.contains(&id) && !active.mods.iter().any(|selected| selected.id == pin.id) {
            active.mods.push(pin.clone());
        }
    }
    if active.shader.is_none() {
        if let Some((_, pin)) = shaders.iter().find(|(id, _)| !old_ids.contains(*id)) {
            active.shader = Some(ShaderSelection {
                package: (*pin).clone(),
                settings: serde_json::json!({}),
            });
        }
    }
    next.validate()?;
    Ok(next)
}
pub(super) fn sync(root: PathBuf, expected: ExtensionPreferences) -> Result<Snapshot, String> {
    expected.validate()?;
    if root.parent().is_none() {
        return Err("Missing EltharnGate data directory".into());
    }
    let _lock = lock(&root)?;
    let store = NativeStore::new(root.clone());
    unchanged(&store, &expected)?;
    for tab in [CatalogTab::Mods, CatalogTab::ShaderPacks] {
        directory(&folder(&root, tab), true)?;
    }
    let previous = match read_marker(&root)? {
        Some(previous) => previous,
        None => migrate(&root, &store, &expected)?,
    };
    let files = fingerprint(&root)?;
    let archives = inspect(&root, &files)?;
    let catalog: Vec<_> = archives
        .iter()
        .map(|archive| archive.info.clone())
        .collect();
    let next = reconcile(&expected, &previous, &catalog)?;
    let marker = MarkerWrite::prepare(&root, &catalog)?;
    unchanged(&store, &expected)?;
    if fingerprint(&root)? != files {
        return Err("Mods or shaderpacks changed during scan; try again".into());
    }
    pollster::block_on(store.replace_library(
        &expected,
        &next,
        archives.into_iter().map(|archive| archive.bytes).collect(),
    ))?;
    marker.commit()?;
    Ok(Snapshot {
        prefs: next,
        catalog,
        fingerprint: files,
    })
}
