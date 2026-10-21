use super::manager::ClientResourceManager;
use el_core::ResourceLocation;
use el_gui::{PackCompatibility, PackEntry, PackSource, pack_compatibility_for_formats};
use serde::Deserialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
#[derive(Deserialize, Debug)]
struct PackMcMeta {
    pack: Option<PackSection>,
}
#[derive(Deserialize, Debug)]
struct PackSection {
    pack_format: Option<u32>,
    #[serde(default)]
    supported_formats: Option<SupportedFormats>,
    min_format: Option<FormatValue>,
    max_format: Option<FormatValue>,
    description: Option<serde_json::Value>,
}
#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum SupportedFormats {
    Single(u32),
    Array(Vec<u32>),
    Range {
        min_inclusive: u32,
        max_inclusive: u32,
    },
}
#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum FormatValue {
    Major(u32),
    Version(Vec<u32>),
}
impl FormatValue {
    fn major(&self) -> Option<u32> {
        match self {
            Self::Major(value) => Some(*value),
            Self::Version(values) => values.first().copied(),
        }
    }
}
impl SupportedFormats {
    fn to_range(&self, pack_format: Option<u32>) -> Option<(u32, u32)> {
        match self {
            SupportedFormats::Single(v) => Some((*v, *v)),
            SupportedFormats::Array(values) => match values.as_slice() {
                [min, max, ..] => Some((*min, *max)),
                [value] => Some((*value, *value)),
                _ => pack_format.map(|value| (value, value)),
            },
            SupportedFormats::Range {
                min_inclusive,
                max_inclusive,
            } => Some((*min_inclusive, *max_inclusive)),
        }
    }
}
fn extract_description(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(obj) => {
            let mut text = obj
                .get("text")
                .and_then(|v| v.as_str())
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    obj.get("translate")
                        .and_then(|v| v.as_str())
                        .map(crate::ui::i18n::translate)
                        .unwrap_or_default()
                        .into_owned()
                })
                .to_string();
            if let Some(extra) = obj.get("extra") {
                text.push_str(&extract_description(extra));
            }
            text
        }
        serde_json::Value::Array(arr) => arr
            .iter()
            .map(extract_description)
            .collect::<Vec<_>>()
            .join(""),
        _ => val.to_string(),
    }
}
fn read_zip_file(zip_path: &Path, name: &str) -> io::Result<Vec<u8>> {
    let file = fs::File::open(zip_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut entry = archive
        .by_name(name)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e))?;
    let mut buf = Vec::new();
    io::Read::read_to_end(&mut entry, &mut buf)?;
    Ok(buf)
}
fn parse_mcmeta(bytes: &[u8]) -> Option<PackSection> {
    let meta: PackMcMeta = serde_json::from_slice(bytes).ok()?;
    meta.pack
}
fn declared_range(section: &PackSection) -> Option<(u32, u32)> {
    if let (Some(min), Some(max)) = (&section.min_format, &section.max_format) {
        return Some((min.major()?, max.major()?));
    }
    section
        .supported_formats
        .as_ref()
        .and_then(|formats| formats.to_range(section.pack_format))
        .or_else(|| section.pack_format.map(|value| (value, value)))
}
fn compatibility(section: &PackSection) -> PackCompatibility {
    declared_range(section)
        .map(|(min, max)| pack_compatibility_for_formats(min, max))
        .unwrap_or_else(PackCompatibility::unknown)
}
pub fn scan_resource_packs(pack_dir: &Path) -> Vec<PackEntry> {
    let _ = fs::create_dir_all(pack_dir);
    let entries = match fs::read_dir(pack_dir) {
        Ok(e) => e,
        Err(err) => {
            log::warn!("Failed to read resourcepacks dir {:?}: {}", pack_dir, err);
            return Vec::new();
        }
    };
    let mut result = Vec::new();
    for dir_entry in entries.flatten() {
        let path = dir_entry.path();
        let file_name = dir_entry.file_name().to_string_lossy().into_owned();
        if file_name.starts_with('.') {
            continue;
        }
        if path.is_file() {
            if let Some(entry) = load_zip_pack(&path, &file_name) {
                result.push(entry);
            }
        } else if path.is_dir() {
            if path.join("pack.mcmeta").exists() {
                if let Some(entry) = load_dir_pack(&path, &file_name) {
                    result.push(entry);
                }
            }
        }
    }
    result
}
fn load_zip_pack(zip_path: &Path, file_name: &str) -> Option<PackEntry> {
    let mcmeta_bytes = read_zip_file(zip_path, "pack.mcmeta").ok()?;
    let section = parse_mcmeta(&mcmeta_bytes)?;
    let compat = compatibility(&section);
    let title = file_name.trim_end_matches(".zip").to_string();
    let description = section
        .description
        .as_ref()
        .map(extract_description)
        .unwrap_or_default();
    let has_icon = read_zip_file(zip_path, "pack.png").is_ok();
    let icon_path = String::new();
    Some(PackEntry {
        id: format!("file/{}", file_name),
        title,
        description,
        compatibility: compat,
        source: PackSource::Local,
        icon_path,
        has_icon,
        fixed_position: false,
        required: false,
    })
}
fn load_dir_pack(dir_path: &Path, dir_name: &str) -> Option<PackEntry> {
    let mcmeta_path = dir_path.join("pack.mcmeta");
    let bytes = fs::read(&mcmeta_path).ok()?;
    let section = parse_mcmeta(&bytes)?;
    let compat = compatibility(&section);
    let title = dir_name.to_string();
    let description = section
        .description
        .as_ref()
        .map(extract_description)
        .unwrap_or_default();
    let has_icon = dir_path.join("pack.png").exists();
    let icon_path = String::new();
    Some(PackEntry {
        id: format!("file/{}", dir_name),
        title,
        description,
        compatibility: compat,
        source: PackSource::Local,
        icon_path,
        has_icon,
        fixed_position: false,
        required: false,
    })
}
pub fn resource_packs_dir(options_path: &Path) -> PathBuf {
    options_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("resourcepacks")
}
#[allow(dead_code)]
pub fn selected_resource_pack_roots(options_path: &Path) -> Vec<PathBuf> {
    selected_resource_pack_roots_in_dir(options_path, &resource_packs_dir(options_path))
}
pub fn selected_resource_pack_roots_in_dir(options_path: &Path, pack_dir: &Path) -> Vec<PathBuf> {
    let value = fs::read_to_string(options_path)
        .ok()
        .and_then(|contents| {
            contents.lines().find_map(|line| {
                let (key, value) = line.split_once(':')?;
                (key.trim() == "resourcePacks").then_some(value.trim().to_string())
            })
        })
        .unwrap_or_default();
    option_value_to_pack_ids(&value)
        .into_iter()
        .filter_map(|id| {
            let relative = id.strip_prefix("file/").unwrap_or(&id);
            let path = pack_dir.join(relative);
            path.exists().then_some(path)
        })
        .collect()
}
#[allow(dead_code)]
pub fn with_selected_resource_packs(
    manager: ClientResourceManager,
    options_path: &Path,
) -> ClientResourceManager {
    manager.with_extra_roots(selected_resource_pack_roots(options_path))
}
#[cfg(not(target_arch = "wasm32"))]
pub fn with_selected_resource_packs_in_dir(
    manager: ClientResourceManager,
    options_path: &Path,
    pack_dir: &Path,
) -> ClientResourceManager {
    manager.with_extra_roots(selected_resource_pack_roots_in_dir(options_path, pack_dir))
}
pub fn register_pack_icons(
    pack_dir: &Path,
    entries: &mut [PackEntry],
) -> Vec<(ResourceLocation, PathBuf)> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let cache_dir = pack_dir.join(".eltharn_icons");
    let _ = fs::create_dir_all(&cache_dir);
    let mut mappings = Vec::new();
    for entry in entries {
        let relative = entry.id.strip_prefix("file/").unwrap_or(&entry.id);
        let source = pack_dir.join(relative);
        let bytes = if source.is_dir() {
            fs::read(source.join("pack.png")).ok()
        } else if source.is_file() {
            read_zip_file(&source, "pack.png").ok()
        } else {
            None
        };
        let Some(bytes) = bytes else {
            entry.icon_path.clear();
            entry.has_icon = false;
            continue;
        };
        let mut hasher = DefaultHasher::new();
        entry.id.hash(&mut hasher);
        let hash = hasher.finish();
        let icon_path = cache_dir.join(format!("{hash:016x}.png"));
        if fs::write(&icon_path, bytes).is_err() {
            entry.icon_path.clear();
            entry.has_icon = false;
            continue;
        }
        let id = ResourceLocation::new("eltharn", format!("pack_icons/{hash:016x}").as_str());
        entry.icon_path = id.to_string();
        entry.has_icon = true;
        mappings.push((id, icon_path));
    }
    mappings
}
pub fn resource_pack_selection_changed(stored_option_value: &str, selected: &[PackEntry]) -> bool {
    stored_option_value != selected_pack_ids_to_option_value(selected)
}
pub fn selected_pack_ids_to_option_value(selected: &[PackEntry]) -> String {
    selected
        .iter()
        .filter(|pack| pack.id != "vanilla")
        .rev()
        .map(|p| p.id.as_str())
        .collect::<Vec<_>>()
        .join(",")
}
pub fn option_value_to_pack_ids(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|part| part.trim().trim_matches('"').trim_matches('\''))
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}
