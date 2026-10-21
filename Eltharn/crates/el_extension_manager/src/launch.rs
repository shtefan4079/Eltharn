use base64::{Engine as _, engine::general_purpose::STANDARD};
use el_extension_api::Sha256Digest;
use el_launcher_core::extensions::{ExtensionPreferences, ExtensionProfile};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
pub const MAX_LAUNCH_BYTES: usize = 48 * 1024 * 1024;
const MAX_DECODED: usize = 32 * 1024 * 1024;
pub const V4_POST_ONLY_VARIANT: &str = "v4_post_only";
const VER: u32 = 1;
pub const CATALOG_REQUEST_VERSION: u32 = 2;
const DOMAIN: &[u8] = b"rustore.launch-request.v1\0";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchRequest {
    pub schema_version: u32,
    pub profile: ExtensionProfile,
    pub packages: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub catalog: Vec<crate::package::PackageInfo>,
}
pub struct VerifiedLaunch {
    pub mods: crate::mods::PreparedMods,
    pub profile: ExtensionProfile,
    pub catalog: Vec<crate::package::PackageInfo>,
    pub shader: Option<el_shaderpack::model::ValidatedPack>,
    pub shader_v4: Option<el_shaderpack::graph_v4::ValidatedPackV4>,
    pub selection_digest: Sha256Digest,
    pub variant_settings: BTreeMap<String, serde_json::Value>,
}
pub fn digest(bytes: &[u8]) -> Sha256Digest {
    let mut h = Sha256::new();
    h.update(DOMAIN);
    h.update(bytes);
    let out: [u8; 32] = h.finalize().into();
    Sha256Digest::from_bytes(out)
}
fn valid(p: &ExtensionProfile) -> Result<(), String> {
    ExtensionPreferences {
        schema_version: 1,
        active_profile: p.id.clone(),
        profiles: vec![p.clone()],
    }
    .validate()
    .map_err(|e| format!("invalid profile: {e}"))
}
fn eff(p: &ExtensionProfile) -> ExtensionProfile {
    let mut e = p.clone();
    if !e.flags.mods_enabled {
        e.mods.clear();
    }
    if !e.flags.shaders_enabled {
        e.shader = None;
    }
    e
}
fn encode(req: &LaunchRequest) -> Result<Vec<u8>, String> {
    let b = serde_json::to_vec(req).map_err(|e| format!("encode launch request: {e}"))?;
    if b.len() > MAX_LAUNCH_BYTES {
        return Err("launch request too large".to_owned());
    }
    Ok(b)
}
pub async fn prepare(
    profile: ExtensionProfile,
    archives: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>, String> {
    prepare_request(profile, archives, None).await
}

pub async fn prepare_with_catalog(
    profile: ExtensionProfile,
    archives: &BTreeMap<String, Vec<u8>>,
    catalog: &[crate::package::PackageInfo],
) -> Result<Vec<u8>, String> {
    prepare_request(profile, archives, Some(catalog)).await
}

async fn prepare_request(
    profile: ExtensionProfile,
    archives: &BTreeMap<String, Vec<u8>>,
    catalog: Option<&[crate::package::PackageInfo]>,
) -> Result<Vec<u8>, String> {
    valid(&profile)?;
    let e = eff(&profile);
    let mut pins = e.mods.clone();
    if let Some(s) = e.shader.as_ref() {
        pins.push(s.package.clone());
    }
    let mut seen = BTreeSet::new();
    for pin in &pins {
        if !seen.insert(pin.content_id.to_string()) {
            return Err("duplicate content id in profile".to_owned());
        }
    }
    let mut pkgs = BTreeMap::new();
    let mut total = 0usize;
    for pin in &pins {
        let k = pin.content_id.to_string();
        let b = archives
            .get(&k)
            .ok_or_else(|| format!("missing archive for package {k}"))?;
        total = total.checked_add(b.len()).ok_or("archive total overflow")?;
        if total > MAX_DECODED {
            return Err("selected archives exceed 32MiB".to_owned());
        }
        pkgs.insert(k, STANDARD.encode(b));
    }
    let bytes = encode(&LaunchRequest {
        schema_version: if catalog.is_some() {
            CATALOG_REQUEST_VERSION
        } else {
            VER
        },
        profile: e,
        packages: pkgs,
        catalog: catalog.unwrap_or_default().to_vec(),
    })?;
    verify(&bytes).await.map(|_| bytes)
}
pub async fn verify(bytes: &[u8]) -> Result<VerifiedLaunch, String> {
    if bytes.len() > MAX_LAUNCH_BYTES {
        return Err("launch request too large".to_owned());
    }
    let value = el_launcher_core::extension_json::strict_json(bytes, MAX_LAUNCH_BYTES)?;
    let req: LaunchRequest =
        serde_json::from_value(value).map_err(|e| format!("invalid launch request: {e}"))?;
    if !matches!(req.schema_version, VER | CATALOG_REQUEST_VERSION) {
        return Err("unsupported launch request version".to_owned());
    }
    if req.schema_version == VER && !req.catalog.is_empty() {
        return Err("legacy launch request cannot carry an installed catalog".to_owned());
    }
    crate::catalog::validate(&req.catalog)?;
    if serde_json::to_vec(&req.catalog)
        .map_err(|e| e.to_string())?
        .len()
        > crate::catalog::MAX_CATALOG_BYTES
    {
        return Err("installed launch catalog is too large".to_owned());
    }
    valid(&req.profile)?;
    if !req.profile.flags.mods_enabled && !req.profile.mods.is_empty() {
        return Err("request carries disabled mods".to_owned());
    }
    if !req.profile.flags.shaders_enabled && req.profile.shader.is_some() {
        return Err("request carries disabled shader".to_owned());
    }
    let mut exp = BTreeSet::new();
    let mut want: BTreeMap<String, (el_launcher_core::extensions::PackagePin, &str)> =
        BTreeMap::new();
    for pin in &req.profile.mods {
        let k = pin.content_id.to_string();
        if !exp.insert(k.clone()) {
            return Err("duplicate content id in profile".to_owned());
        }
        want.insert(k, (pin.clone(), "mod"));
    }
    if let Some(s) = req.profile.shader.as_ref() {
        let k = s.package.content_id.to_string();
        if !exp.insert(k.clone()) {
            return Err("duplicate content id in profile".to_owned());
        }
        want.insert(k, (s.package.clone(), "shaderpack"));
    }
    if req.packages.keys().cloned().collect::<BTreeSet<_>>() != exp {
        return Err("package inventory does not match profile selection".to_owned());
    }
    let mut est = 0usize;
    for s in req.packages.values() {
        let padding = s
            .as_bytes()
            .iter()
            .rev()
            .take_while(|b| **b == b'=')
            .count()
            .min(2);
        est = est
            .checked_add((s.len() * 3 / 4).saturating_sub(padding))
            .ok_or("archive total overflow")?;
        if est > MAX_DECODED {
            return Err("selected archives exceed 32MiB".to_owned());
        }
    }
    let mut total = 0usize;
    let mut shader = None;
    let mut shader_v4 = None;
    let mut variant_settings = BTreeMap::new();
    let mut mod_packages = Vec::new();
    let mut selected_catalog = Vec::new();
    for (k, v) in &req.packages {
        let (pin, kind) = want.get(k).cloned().ok_or("extra package")?;
        let raw = STANDARD
            .decode(v)
            .map_err(|_| format!("package {k}: invalid base64"))?;
        if raw.len() > crate::archive::MAX_ARCHIVE_BYTES {
            return Err(format!("package {k}: exceeds archive limit"));
        }
        total = total
            .checked_add(raw.len())
            .ok_or("archive total overflow")?;
        if total > MAX_DECODED {
            return Err("selected archives exceed 32MiB".to_owned());
        }
        let pkg = crate::package::inspect(raw)
            .await
            .map_err(|e| format!("package {k}: {e}"))?;
        if pkg.info.pin != pin {
            return Err(format!("package {k}: pin mismatch"));
        }
        if pkg.info.kind != kind {
            return Err(format!("package {k}: wrong kind (expected {kind})"));
        }
        if req.schema_version == CATALOG_REQUEST_VERSION
            && !req.catalog.iter().any(|entry| entry == &pkg.info)
        {
            return Err(format!(
                "package {k}: installed catalog metadata mismatch or missing entry"
            ));
        }
        selected_catalog.push(pkg.info.clone());
        if kind == "shaderpack" {
            let manifest = crate::manifest::strict_json(
                pkg.files
                    .get(el_extension_api::package_path::MANIFEST_FILE_NAME)
                    .ok_or_else(|| format!("package {k}: missing manifest"))?,
            )
            .map_err(|e| format!("package {k}: manifest: {e}"))?;
            if manifest.get("shader_api").and_then(|v| v.as_str())
                == Some(el_shaderpack::graph_v4::SHADER_API_V4)
            {
                let settings = req
                    .profile
                    .shader
                    .as_ref()
                    .map(|s| s.settings.clone())
                    .ok_or("missing shader selection")?;
                let pack = pkg
                    .shader_v4_with_settings(&settings)
                    .await
                    .map_err(|e| format!("package {k}: {e}"))?;
                if pack.needs_compute() {
                    return Err(format!(
                        "package {k}: v4 production selection currently supports post-only graphs"
                    ));
                }
                variant_settings.insert(V4_POST_ONLY_VARIANT.into(), pack.settings.clone());
                shader_v4 = Some(pack);
                continue;
            }
            let settings = req
                .profile
                .shader
                .as_ref()
                .map(|s| s.settings.clone())
                .ok_or("missing shader selection")?;
            let vp = pkg
                .shader(&settings)
                .await
                .map_err(|e| format!("package {k}: {e}"))?;
            shader = Some(vp);
            let manifest = crate::manifest::strict_json(
                &pkg.files[el_extension_api::package_path::MANIFEST_FILE_NAME],
            )?;
            let descriptor_path = manifest["shader"]["descriptor"]
                .as_str()
                .ok_or("missing shader descriptor")?;
            let descriptor = crate::manifest::strict_json(
                pkg.files
                    .get(descriptor_path)
                    .ok_or("missing shader descriptor bytes")?,
            )?;
            for variant in descriptor["variants"]
                .as_array()
                .ok_or("shader variants missing")?
            {
                let mut resolved = serde_json::Map::new();
                for param in variant["params"]
                    .as_array()
                    .ok_or("shader params missing")?
                {
                    let id = param["id"].as_str().ok_or("parameter id")?;
                    resolved.insert(
                        id.into(),
                        settings.get(id).unwrap_or(&param["default"]).clone(),
                    );
                }
                variant_settings.insert(
                    variant["id"].as_str().ok_or("variant id")?.into(),
                    resolved.into(),
                );
            }
        } else {
            mod_packages.push(pkg);
        }
    }
    let mods = crate::mods::prepare(
        &mod_packages,
        el_extension_api::services::RuntimeSide::Client,
    )?;
    Ok(VerifiedLaunch {
        mods,
        catalog: if req.schema_version == CATALOG_REQUEST_VERSION {
            req.catalog
        } else {
            selected_catalog
        },
        profile: req.profile,
        shader,
        shader_v4,
        selection_digest: digest(bytes),
        variant_settings,
    })
}
