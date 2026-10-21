use crate::package::PackageInfo;
use std::collections::BTreeSet;

pub const MAX_CATALOG_BYTES: usize = 1024 * 1024;
pub fn decode(bytes: &[u8]) -> Result<Vec<PackageInfo>, String> {
    if bytes.len() > MAX_CATALOG_BYTES {
        return Err("Package catalog is too large".into());
    }
    let value = el_launcher_core::extension_json::strict_json(bytes, MAX_CATALOG_BYTES)?;
    let entries: Vec<PackageInfo> =
        serde_json::from_value(value).map_err(|e| format!("Invalid package catalog: {e}"))?;
    validate(&entries)?;
    Ok(entries)
}
pub fn validate(entries: &[PackageInfo]) -> Result<(), String> {
    if entries.len() > 256 {
        return Err("Package library is full (256 versions)".into());
    }
    let mut hashes = BTreeSet::new();
    for p in entries {
        if p.pin.id.is_reserved_namespace()
            || !matches!(p.kind.as_str(), "mod" | "shaderpack")
            || p.name.trim().is_empty()
            || p.name.len() > 4096
            || p.description.len() > 65536
            || p.name.chars().any(char::is_control)
            || !hashes.insert(p.pin.content_id.to_string())
        {
            return Err("Invalid or duplicate package catalog entry".into());
        }
    }
    Ok(())
}
