use el_extension_api::{ExtensionId, PackageVersion, Sha256Digest};
use el_launcher_core::extensions::PackagePin;
use el_platform_api::{MemoryResourceStore, ResourceSnapshotRequest, ResourceStore, VirtualPath};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
const MANIFEST: &str = el_extension_api::package_path::MANIFEST_FILE_NAME;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageInfo {
    pub pin: PackagePin,
    pub kind: String,
    pub name: String,
    pub description: String,
}
#[derive(Clone, Debug)]
pub struct Package {
    pub info: PackageInfo,
    pub archive: Vec<u8>,
    pub files: BTreeMap<String, Vec<u8>>,
    pub receipt: Value,
    pub metadata: Option<crate::metadata::ModMetadata>,
}
fn str_field(m: &Value, k: &str) -> Result<String, String> {
    m.get(k)
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned())
        .ok_or_else(|| format!("missing/invalid {k}"))
}
pub async fn inspect(bytes: Vec<u8>) -> Result<Package, String> {
    let files = crate::archive::unpack(&bytes).map_err(|e| format!("unpack: {e}"))?;
    let original_manifest = files
        .get(MANIFEST)
        .ok_or_else(|| format!("missing {MANIFEST}"))?
        .clone();
    let manifest_bytes = crate::source_manifest::normalize(&original_manifest, &files)
        .map_err(|e| format!("source manifest: {e}"))?;
    let m = crate::manifest::strict_json(&manifest_bytes).map_err(|e| format!("manifest: {e}"))?;
    let id_str = str_field(&m, "id")?;
    let ver_str = str_field(&m, "version")?;
    let kind = str_field(&m, "kind")?;
    let name = str_field(&m, "display_name")?;
    let description = m
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    let id: ExtensionId = id_str.parse().map_err(|e| format!("id: {e}"))?;
    let version: PackageVersion = ver_str.parse().map_err(|e| format!("version: {e}"))?;
    let mut h = Sha256::new();
    h.update(b"rustore.package.v1\0");
    h.update(&manifest_bytes);
    let content_id = Sha256Digest::from_bytes(h.finalize().into());
    let receipt = json!({
        "id": id.to_string(),
        "version": version.to_string(),
        "content_id": content_id.to_string(),
    });
    let receipt_bytes = serde_json::to_vec(&receipt).map_err(|e| format!("receipt encode: {e}"))?;
    let actual: BTreeSet<String> = files
        .keys()
        .filter(|k| k.as_str() != MANIFEST)
        .cloned()
        .collect();
    let read = |path: &str, size: u64| -> Result<Vec<u8>, String> {
        let d = files
            .get(path)
            .ok_or_else(|| format!("missing payload {path}"))?;
        if d.len() as u64 > size {
            return Err(format!("payload exceeds declared size {path}"));
        }
        Ok(d.clone())
    };
    crate::manifest::verify_source(
        &manifest_bytes,
        &receipt_bytes,
        &actual,
        &read,
        &|_: &Value, _: &BTreeSet<String>| Ok(json!({})),
    )
    .map_err(|e| format!("verify: {e}"))?;
    let info = PackageInfo {
        pin: PackagePin {
            id,
            version,
            content_id,
        },
        kind: kind.clone(),
        name,
        description,
    };
    let pkg = Package {
        metadata: if kind == "mod" {
            Some(crate::metadata::parse(&m)?)
        } else {
            None
        },
        info,
        archive: bytes,
        files,
        receipt,
    };
    if pkg.info.kind == "shaderpack" {
        let api = m
            .get("shader_api")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "shader defaults: missing/invalid shader_api".to_owned())?;
        match api {
            "1.3" => {
                pkg.shader_v4()
                    .await
                    .map_err(|e| format!("shader defaults: {e}"))?;
            }
            "1.0" | "1.1" | "1.2" => {
                pkg.shader(&json!({}))
                    .await
                    .map_err(|e| format!("shader defaults: {e}"))?;
            }
            _ => return Err(format!("shader defaults: unsupported shader_api {api:?}")),
        }
    }
    Ok(pkg)
}
impl Package {
    pub async fn shader(
        &self,
        settings: &Value,
    ) -> Result<el_shaderpack::model::ValidatedPack, String> {
        if self.info.kind != "shaderpack" {
            return Err("not a shaderpack".into());
        }
        let store = MemoryResourceStore::new();
        for (path, data) in &self.files {
            let vp = VirtualPath::new(path).map_err(|e| format!("path {path}: {e}"))?;
            store.insert(vp, data.as_slice());
        }
        let snap = store
            .prepare_snapshot(ResourceSnapshotRequest::all())
            .await
            .map_err(|e| format!("snapshot: {e}"))?;
        let rb = serde_json::to_vec(&self.receipt).map_err(|e| format!("receipt encode: {e}"))?;
        el_shaderpack::package::load(&*snap, &rb, settings).map_err(|e| format!("shader load: {e}"))
    }
    pub async fn shader_v4(&self) -> Result<el_shaderpack::graph_v4::ValidatedPackV4, String> {
        self.shader_v4_with_settings(&serde_json::json!({})).await
    }

    pub async fn shader_v4_with_settings(
        &self,
        settings: &Value,
    ) -> Result<el_shaderpack::graph_v4::ValidatedPackV4, String> {
        if self.info.kind != "shaderpack" {
            return Err("not a shaderpack".into());
        }
        let store = MemoryResourceStore::new();
        for (path, data) in &self.files {
            let vp = VirtualPath::new(path).map_err(|e| format!("path {path}: {e}"))?;
            store.insert(vp, data.as_slice());
        }
        let snap = store
            .prepare_snapshot(ResourceSnapshotRequest::all())
            .await
            .map_err(|e| format!("snapshot: {e}"))?;
        let rb = serde_json::to_vec(&self.receipt).map_err(|e| format!("receipt encode: {e}"))?;
        el_shaderpack::graph_v4::load_v4_with_settings(&*snap, &rb, settings)
            .map_err(|e| format!("shader v4 load: {e}"))
    }
}
