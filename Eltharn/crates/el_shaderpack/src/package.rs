use crate::{model::*, strict::*};
use el_extension_api::{ExtensionId, PackagePath, PackageVersion, Sha256Digest};
use el_platform_api::{ResourceSnapshot, VirtualPath};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
type Result<T> = std::result::Result<T, String>;
pub(crate) fn read_manifest(snapshot: &dyn ResourceSnapshot) -> Result<Vec<u8>> {
    use el_extension_api::package_path::{LEGACY_MANIFEST_FILE_NAME, MANIFEST_FILE_NAME};
    let current = snapshot
        .read_view(&VirtualPath::new(MANIFEST_FILE_NAME).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let legacy = snapshot
        .read_view(&VirtualPath::new(LEGACY_MANIFEST_FILE_NAME).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let bytes = match (current, legacy) {
        (Some(_), Some(_)) => return Err("multiple root manifests".into()),
        (Some(bytes), None) | (None, Some(bytes)) => bytes,
        (None, None) => return Err(format!("missing {MANIFEST_FILE_NAME}")),
    };
    ensure(bytes.as_ref().len() <= 262144, "snapshot file byte limit")?;
    Ok(bytes.as_ref().to_vec())
}
fn read(snapshot: &dyn ResourceSnapshot, path: &str, limit: usize) -> Result<Vec<u8>> {
    let path = VirtualPath::new(path).map_err(|e| e.to_string())?;
    let bytes = snapshot
        .read_view(&path)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("missing {path}"))?;
    ensure(bytes.as_ref().len() <= limit, "snapshot file byte limit")?;
    Ok(bytes.as_ref().to_vec())
}
pub fn load(
    snapshot: &dyn ResourceSnapshot,
    receipt: &[u8],
    settings: &Value,
) -> Result<ValidatedPack> {
    object(settings)?;
    let bytes = read_manifest(snapshot)?;
    let m = strict_json(&bytes)?;
    keys(
        &m,
        &[
            "manifest_version",
            "kind",
            "id",
            "version",
            "display_name",
            "license",
            "files",
            "shader_api",
            "shader",
        ],
        &["description"],
    )?;
    ensure(
        uint(field(&m, "manifest_version")?)? == 1,
        "manifest version",
    )?;
    eq(field(&m, "kind")?, "shaderpack")?;
    let pack_id: ExtensionId = typed(field(&m, "id")?)?;
    ensure(
        !pack_id.is_reserved_namespace(),
        "reserved package namespace",
    )?;
    let version: PackageVersion = typed(field(&m, "version")?)?;
    text_limit(field(&m, "display_name")?, 1, 64)?;
    if let Some(d) = m.get("description") {
        text_limit(d, 0, 512)?;
    }
    let license = string(field(&m, "license")?)?;
    ensure(
        (1..=128).contains(&license.len())
            && license
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".+-".contains(&b)),
        "license syntax",
    )?;
    let receipt = strict_json(receipt)?;
    keys(&receipt, &["id", "version", "content_id"], &[])?;
    let mut hasher = Sha256::new();
    hasher.update(b"rustore.package.v1\0");
    hasher.update(&bytes);
    let content_id = Sha256Digest::from_bytes(hasher.finalize().into());
    ensure(
        typed::<ExtensionId>(field(&receipt, "id")?)? == pack_id
            && typed::<PackageVersion>(field(&receipt, "version")?)? == version
            && typed::<Sha256Digest>(field(&receipt, "content_id")?)? == content_id,
        "receipt mismatch",
    )?;
    let files = array(field(&m, "files")?)?;
    ensure((1..=4096).contains(&files.len()), "file count")?;
    let mut payload = BTreeMap::new();
    let mut total = 0u64;
    for f in files {
        keys(f, &["path", "size", "sha256"], &[])?;
        let path: PackagePath = typed(field(f, "path")?)?;
        ensure(
            path.as_str().starts_with("shaders/"),
            "M03 payload must be under shaders/",
        )?;
        let limit = if path.as_str().ends_with(".wgsl") {
            1048576
        } else if path.as_str().ends_with(".json") {
            262144
        } else {
            return Err("unsupported shader payload extension".into());
        };
        let size = uint(field(f, "size")?)?;
        ensure(size <= limit as u64, "declared shader file size")?;
        total = total.checked_add(size).ok_or("payload size overflow")?;
        ensure(total <= 32 * 1024 * 1024, "M03 payload budget")?;
        let data = read(snapshot, path.as_str(), limit)?;
        let expected: Sha256Digest = typed(field(f, "sha256")?)?;
        ensure(
            data.len() as u64 == size
                && Sha256Digest::from_bytes(Sha256::digest(&data).into()) == expected,
            "payload size/hash",
        )?;
        ensure(
            payload.insert(path.to_string(), data).is_none(),
            "duplicate payload path",
        )?;
    }
    let actual = snapshot
        .list(&VirtualPath::new("shaders").map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let actual_set: BTreeSet<_> = actual.iter().map(|p| p.as_str()).collect();
    ensure(
        actual_set.len() == actual.len()
            && actual_set == payload.keys().map(String::as_str).collect(),
        "missing/unlisted payload",
    )?;
    for name in payload.keys() {
        for (pos, _) in name.match_indices('/') {
            ensure(
                !payload.contains_key(&name[..pos]),
                "file/directory collision",
            )?;
        }
    }
    let shader = field(&m, "shader")?;
    keys(shader, &["descriptor"], &[])?;
    let path: PackagePath = typed(field(shader, "descriptor")?)?;
    ensure(path.as_str().ends_with(".json"), "descriptor extension")?;
    let d = strict_json(payload.get(path.as_str()).ok_or("unlisted descriptor")?)?;
    keys(
        &d,
        &[
            "descriptor_version",
            "pack_profile",
            "programs",
            "engine_inputs",
            "variants",
        ],
        &[],
    )?;
    let profile = match (
        uint(field(&d, "descriptor_version")?)?,
        string(field(&d, "pack_profile")?)?,
        string(field(&m, "shader_api")?)?,
    ) {
        (1, "post_color_v1", "1.0") => Profile::Color,
        (2, "post_color_depth_v1", "1.1") => Profile::ColorDepth,
        (3, "post_color_depth_environment_v1", "1.2") => Profile::ColorDepthEnvironment,
        _ => return Err("shader API/profile/descriptor mismatch".into()),
    };
    let inputs = array(field(&d, "engine_inputs")?)?;
    let mut expected = vec![
        json!({ "id" : "engine.scene_color", "kind" : "engine_texture", "format" :
        "rgba8unorm", "samples" : 1, "semantic" : "scene_color_v1" }),
    ];
    if profile.has_depth() {
        expected.extend([
            json!(
                { "id" : "engine.scene_depth", "kind" : "engine_texture", "format" :
                "depth32float", "samples" : 1, "semantic" :
                "world_depth_reverse_z_v1" }
            ),
            json!(
                { "id" : "engine.camera", "kind" : "engine_uniform", "semantic" :
                "world_camera_v1", "size" : 176 }
            ),
        ]);
    }
    if profile.has_environment() {
        expected.push(json!(
            { "id" : "engine.environment", "kind" : "engine_uniform", "semantic"
            : "world_environment_v1", "size" : 80 }
        ));
    }
    ensure(inputs.len() == expected.len(), "engine input count")?;
    for input in inputs {
        let pos = expected
            .iter()
            .position(|e| e == input)
            .ok_or("engine input contract")?;
        expected.remove(pos);
    }
    let ps = array(field(&d, "programs")?)?;
    ensure((1..=64).contains(&ps.len()), "program count")?;
    let mut programs = BTreeMap::new();
    for p in ps {
        keys(p, &["id", "path", "vertex_entry", "fragment_entry"], &[])?;
        let name = id(field(p, "id")?)?;
        for k in ["vertex_entry", "fragment_entry"] {
            let e = string(field(p, k)?)?;
            ensure(
                (1..=64).contains(&e.len())
                    && e.bytes().enumerate().all(|(i, b)| {
                        b.is_ascii_alphabetic() || b == b'_' || i > 0 && b.is_ascii_digit()
                    }),
                "entry syntax",
            )?;
        }
        let path: PackagePath = typed(field(p, "path")?)?;
        ensure(path.as_str().ends_with(".wgsl"), "program extension")?;
        let source = std::str::from_utf8(payload.get(path.as_str()).ok_or("unlisted program")?)
            .map_err(|e| e.to_string())?;
        let vertex = string(field(p, "vertex_entry")?)?;
        let fragment = string(field(p, "fragment_entry")?)?;
        let has_params = crate::validation::validate_wgsl(source, vertex, fragment, profile)?;
        ensure(
            programs
                .insert(
                    name.into(),
                    Program {
                        source: source.into(),
                        vertex_entry: vertex.into(),
                        fragment_entry: fragment.into(),
                        has_params,
                    },
                )
                .is_none(),
            "duplicate program",
        )?;
    }
    let vs = array(field(&d, "variants")?)?;
    ensure((1..=16).contains(&vs.len()), "variant count")?;
    let mut variants = Vec::new();
    let mut ids = BTreeSet::new();
    let mut param_ids = BTreeSet::new();
    for v in vs {
        let variant = crate::graph::validate_graph(v, &programs, profile, settings)?;
        ensure(ids.insert(variant.id.clone()), "duplicate variant")?;
        param_ids.extend(crate::params::ids(field(v, "params")?)?);
        variants.push(variant);
    }
    ensure(
        object(settings)?.keys().all(|k| param_ids.contains(k)),
        "unknown setting ID",
    )?;
    Ok(ValidatedPack {
        id: pack_id.to_string(),
        content_id: content_id.to_string(),
        profile,
        programs,
        variants,
    })
}
pub fn settings(bytes: &[u8]) -> Result<Value> {
    let v = strict_json(bytes)?;
    object(&v)?;
    Ok(v)
}
