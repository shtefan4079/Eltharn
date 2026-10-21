#![allow(dead_code)]
use crate::launcher::paths::EltharnGatePaths;
use crate::launcher::utils::{USER_AGENT, sha1_hex};
use el_launcher_core::PreparedAssetManifest;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::sync::mpsc::UnboundedSender;
const MOJANG_VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const MOJANG_RESOURCES_BASE_URL: &str = "https://resources.download.minecraft.net";
#[derive(Debug, Clone, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionEntry>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct VersionEntry {
    pub id: String,
    pub r#type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub sha1: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct VersionMetadata {
    pub id: String,
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndexInfo,
    pub downloads: VersionDownloads,
}
#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndexInfo {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
    #[serde(rename = "totalSize")]
    pub total_size: Option<u64>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct VersionDownloads {
    pub client: DownloadInfo,
    pub server: Option<DownloadInfo>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct DownloadInfo {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndexFile {
    pub objects: HashMap<String, AssetObject>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}
#[derive(Debug, Clone)]
pub struct VerificationReport {
    pub is_valid: bool,
    pub version_id: String,
    pub total_indexed_objects: usize,
    pub valid_indexed_objects: usize,
    pub missing_indexed_objects: usize,
    pub client_archive_resources: usize,
    pub details: Vec<String>,
}
pub async fn download_file_verified(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    expected_sha1: Option<&str>,
    max_retries: usize,
) -> Result<(), String> {
    if dest.is_file() {
        if let Some(expected) = expected_sha1 {
            if let Ok(bytes) = fs::read(dest) {
                if sha1_hex(&bytes).eq_ignore_ascii_case(expected) {
                    return Ok(());
                }
            }
        } else if dest.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            return Ok(());
        }
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {e}", parent.display()))?;
    }
    let mut last_error = String::new();
    for attempt in 0..=max_retries {
        if attempt > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(200 * (1 << attempt))).await;
        }
        let response = match client
            .get(url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                last_error = format!("HTTP send error: {e}");
                continue;
            }
        };
        if !response.status().is_success() {
            last_error = format!("HTTP error status: {}", response.status());
            continue;
        }
        let bytes = match response.bytes().await {
            Ok(b) => b,
            Err(e) => {
                last_error = format!("Failed to read response body: {e}");
                continue;
            }
        };
        if let Some(expected) = expected_sha1 {
            let actual = sha1_hex(&bytes);
            if !actual.eq_ignore_ascii_case(expected) {
                last_error = format!("SHA-1 mismatch: expected {expected}, got {actual}");
                continue;
            }
        }
        let temp_filename = format!(
            ".tmp_{}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            dest.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("download")
        );
        let temp_path = dest
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(temp_filename);
        if let Err(e) = fs::write(&temp_path, &bytes) {
            last_error = format!("Failed to write temporary file: {e}");
            continue;
        }
        if let Err(e) = fs::rename(&temp_path, dest) {
            let _ = fs::remove_file(&temp_path);
            last_error = format!(
                "Failed to atomically rename file to {}: {e}",
                dest.display()
            );
            continue;
        }
        return Ok(());
    }
    Err(format!(
        "Failed to download {url} to {} after {} attempts. Last error: {last_error}",
        dest.display(),
        max_retries + 1
    ))
}
pub fn verify_version_assets(
    paths: &EltharnGatePaths,
    version_id: &str,
) -> Result<VerificationReport, String> {
    el_launcher_core::ProtocolId::new(version_id.to_owned()).map_err(|e| e.to_string())?;
    if !prepared_manifest_path(paths, version_id).is_file() {
        return Ok(VerificationReport {
            is_valid: false,
            version_id: version_id.to_owned(),
            total_indexed_objects: 0,
            valid_indexed_objects: 0,
            missing_indexed_objects: 0,
            client_archive_resources: 0,
            details: vec![
                "No verified Mojang asset receipt; download the requested release.".into(),
            ],
        });
    }
    verify_prepared_release(paths, &read_prepared_manifest(paths, version_id)?)
}
pub async fn download_version_assets(
    paths: &EltharnGatePaths,
    version_id: &str,
    progress_sender: Option<UnboundedSender<(usize, usize, String)>>,
) -> Result<VerificationReport, String> {
    download_version_assets_from(
        paths,
        version_id,
        progress_sender,
        MOJANG_VERSION_MANIFEST_URL,
        MOJANG_RESOURCES_BASE_URL,
        false,
    )
    .await
}
async fn download_version_assets_from(
    paths: &EltharnGatePaths,
    version_id: &str,
    progress_sender: Option<UnboundedSender<(usize, usize, String)>>,
    manifest_url: &str,
    resources_base_url: &str,
    refresh: bool,
) -> Result<VerificationReport, String> {
    el_launcher_core::ProtocolId::new(version_id.to_owned()).map_err(|e| e.to_string())?;
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;
    let report = |completed: usize, total: usize, status: &str| {
        if let Some(ref sender) = progress_sender {
            let _ = sender.send((completed, total, status.to_string()));
        }
    };
    paths.ensure_directories();
    if !refresh && let Ok(cached) = verify_version_assets(paths, version_id) {
        if cached.is_valid && read_prepared_manifest(paths, version_id).is_ok() {
            return Ok(cached);
        }
    }
    report(10, 100, "Fetching Mojang version manifest...");
    let manifest: VersionManifest = client
        .get(manifest_url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch version manifest: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse version manifest: {e}"))?;
    let version_entry = resolve_exact_version(&manifest, version_id)?;
    report(
        20,
        100,
        &format!("Fetching metadata for {}...", version_entry.id),
    );
    let metadata_bytes = client
        .get(&version_entry.url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch version metadata: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Version metadata HTTP error: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read version metadata: {e}"))?;
    if let Some(expected) = &version_entry.sha1 {
        if !sha1_hex(&metadata_bytes).eq_ignore_ascii_case(expected) {
            return Err(format!("Version metadata hash mismatch for {version_id}"));
        }
    }
    let metadata: VersionMetadata = serde_json::from_slice(&metadata_bytes)
        .map_err(|e| format!("Failed to parse version metadata: {e}"))?;
    if metadata.id != version_id {
        return Err(format!(
            "Requested version {version_id}, received metadata for {}",
            metadata.id
        ));
    }
    el_launcher_core::ProtocolId::new(metadata.asset_index.id.clone())
        .map_err(|e| e.to_string())?;
    let prepared = PreparedAssetManifest {
        format_version: 2,
        compatibility_version: version_id.to_owned(),
        source_version: metadata.id.clone(),
        asset_index: metadata.asset_index.id.clone(),
        index_sha1: metadata.asset_index.sha1.clone(),
        client_sha1: metadata.downloads.client.sha1.clone(),
    };
    prepared.validate(version_id)?;
    report(25, 100, "Downloading asset index...");
    let asset_index_path = paths.asset_indexes_dir().join(prepared.index_file_name());
    download_file_verified(
        &client,
        &metadata.asset_index.url,
        &asset_index_path,
        Some(&metadata.asset_index.sha1),
        3,
    )
    .await?;
    let asset_index_bytes =
        fs::read(&asset_index_path).map_err(|e| format!("Failed to read asset index file: {e}"))?;
    let asset_index: AssetIndexFile = serde_json::from_slice(&asset_index_bytes)
        .map_err(|e| format!("Failed to parse asset index JSON: {e}"))?;
    let total_objects = asset_index.objects.len();
    report(
        30,
        100,
        &format!("Synchronizing {total_objects} asset objects..."),
    );
    let objects_dir = paths.asset_objects_dir();
    let semaphore = Arc::new(Semaphore::new(32));
    let client_arc = Arc::new(client.clone());
    let mut tasks = tokio::task::JoinSet::new();
    for asset in asset_index.objects.values() {
        let hash = asset.hash.clone();
        if !valid_sha1(&hash) {
            return Err(format!(
                "Invalid asset object SHA-1 in index for {version_id}"
            ));
        }
        let prefix = hash[0..2].to_string();
        let dest = objects_dir.join(&prefix).join(&hash);
        let url = format!("{resources_base_url}/{prefix}/{hash}");
        let sem = semaphore.clone();
        let cl = client_arc.clone();
        tasks.spawn(async move {
            let _permit = sem.acquire().await.map_err(|e| e.to_string())?;
            download_file_verified(&cl, &url, &dest, Some(&hash), 3).await
        });
    }
    let mut completed_objects = 0;
    while let Some(task) = tasks.join_next().await {
        match task {
            Ok(Ok(())) => {
                completed_objects += 1;
                if completed_objects % 100 == 0 || completed_objects == total_objects {
                    let pct = 30 + (completed_objects * 50 / total_objects.max(1));
                    report(
                        pct,
                        100,
                        &format!("Downloading assets ({completed_objects}/{total_objects})..."),
                    );
                }
            }
            Ok(Err(e)) => return Err(format!("Asset preparation incomplete: {e}")),
            Err(e) => return Err(format!("Asset preparation task failed: {e}")),
        }
    }
    report(85, 100, "Downloading Minecraft client JAR...");
    let jar_dest = paths.assets_dir().join(prepared.client_archive_name());
    download_file_verified(
        &client,
        &metadata.downloads.client.url,
        &jar_dest,
        Some(&metadata.downloads.client.sha1),
        3,
    )
    .await?;
    report(98, 100, "Verifying asset integrity...");
    let report_result = verify_prepared_release(paths, &prepared)?;
    if !report_result.is_valid {
        return Err(format!(
            "Asset preparation failed verification: {}",
            report_result.details.join("; ")
        ));
    }
    let receipt = prepared_manifest_path(paths, version_id);
    fs::create_dir_all(receipt.parent().unwrap()).map_err(|e| e.to_string())?;
    let pending = receipt.with_extension(format!("json.{}.pending", std::process::id()));
    fs::write(
        &pending,
        serde_json::to_vec_pretty(&prepared).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("Could not stage prepared assets: {e}"))?;
    fs::rename(&pending, &receipt)
        .map_err(|e| format!("Could not publish prepared assets: {e}"))?;
    report(
        100,
        100,
        &format!(
            "Assets ready! Verified archive and {} indexed objects.",
            report_result.valid_indexed_objects
        ),
    );
    Ok(report_result)
}
fn verify_prepared_release(
    paths: &EltharnGatePaths,
    manifest: &PreparedAssetManifest,
) -> Result<VerificationReport, String> {
    manifest.validate(&manifest.compatibility_version)?;
    let index_path = paths.asset_indexes_dir().join(manifest.index_file_name());
    let index_bytes = fs::read(&index_path).map_err(|e| format!("Prepared asset index: {e}"))?;
    if !sha1_hex(&index_bytes).eq_ignore_ascii_case(&manifest.index_sha1) {
        return Err("Prepared asset index hash mismatch".into());
    }
    let index: AssetIndexFile =
        serde_json::from_slice(&index_bytes).map_err(|e| format!("Asset index: {e}"))?;
    let archive_bytes = fs::read(paths.assets_dir().join(manifest.client_archive_name()))
        .map_err(|e| format!("Prepared client archive: {e}"))?;
    if !sha1_hex(&archive_bytes).eq_ignore_ascii_case(&manifest.client_sha1) {
        return Err("Prepared client archive hash mismatch".into());
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(archive_bytes))
        .map_err(|e| format!("Client archive: {e}"))?;
    let mut archive_resources = std::collections::HashSet::new();
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Client archive entry: {e}"))?;
        if entry.is_dir() || !entry.name().starts_with("assets/") {
            continue;
        }
        let name = entry.name().to_owned();
        let relative = name.strip_prefix("assets/").unwrap_or_default();
        if !valid_asset_resource_path(relative) || entry.enclosed_name().is_none() {
            return Err(format!("Invalid client archive asset path: {name}"));
        }
        if !archive_resources.insert(relative.to_owned()) {
            return Err(format!("Duplicate client archive resource: {name}"));
        }
        std::io::copy(&mut entry, &mut std::io::sink())
            .map_err(|e| format!("Client archive resource {name}: {e}"))?;
    }
    if archive_resources.is_empty() {
        return Err("Client archive contains no asset resources".into());
    }
    let mut details = Vec::new();
    let mut valid = 0;
    for (name, object) in &index.objects {
        if !valid_asset_resource_path(name) {
            return Err(format!("Invalid indexed resource path: {name}"));
        }
        let verified = valid_sha1(&object.hash)
            && fs::read(
                paths
                    .asset_objects_dir()
                    .join(&object.hash[..2])
                    .join(&object.hash),
            )
            .is_ok_and(|bytes| {
                bytes.len() as u64 == object.size
                    && sha1_hex(&bytes).eq_ignore_ascii_case(&object.hash)
            });
        if verified {
            valid += 1;
        } else {
            details.push(format!("Missing or corrupt indexed resource: {name}"));
        }
    }
    let missing = index.objects.len() - valid;
    if index.objects.is_empty() {
        details.push("Prepared asset index contains no resource objects".into());
    }
    Ok(VerificationReport {
        is_valid: !index.objects.is_empty() && !archive_resources.is_empty() && missing == 0,
        version_id: manifest.compatibility_version.clone(),
        total_indexed_objects: index.objects.len(),
        valid_indexed_objects: valid,
        missing_indexed_objects: missing,
        client_archive_resources: archive_resources.len(),
        details,
    })
}
fn valid_asset_resource_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != ".." && !part.contains(':'))
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}
fn valid_sha1(hash: &str) -> bool {
    hash.len() == 40 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}
pub fn prepared_manifest_path(paths: &EltharnGatePaths, compatibility: &str) -> PathBuf {
    paths
        .assets_dir()
        .join("releases")
        .join(format!("{compatibility}.json"))
}
pub fn read_prepared_manifest(
    paths: &EltharnGatePaths,
    compatibility: &str,
) -> Result<PreparedAssetManifest, String> {
    el_launcher_core::ProtocolId::new(compatibility.to_owned()).map_err(|e| e.to_string())?;
    let receipt = prepared_manifest_path(paths, compatibility);
    let manifest: PreparedAssetManifest =
        serde_json::from_slice(&fs::read(&receipt).map_err(|e| {
            format!(
                "Prepare assets before launching ({}): {e}",
                receipt.display()
            )
        })?)
        .map_err(|e| format!("Invalid prepared asset receipt: {e}"))?;
    manifest.validate(compatibility)?;
    for (path, expected) in [
        (
            paths.asset_indexes_dir().join(manifest.index_file_name()),
            &manifest.index_sha1,
        ),
        (
            paths.assets_dir().join(manifest.client_archive_name()),
            &manifest.client_sha1,
        ),
    ] {
        let bytes = fs::read(&path)
            .map_err(|e| format!("Prepared resource missing ({}): {e}", path.display()))?;
        if !sha1_hex(&bytes).eq_ignore_ascii_case(expected) {
            return Err(format!(
                "Prepared resource hash mismatch: {}",
                path.display()
            ));
        }
    }
    Ok(manifest)
}
fn resolve_exact_version<'a>(
    manifest: &'a VersionManifest,
    requested: &str,
) -> Result<&'a VersionEntry, String> {
    manifest
        .versions
        .iter()
        .find(|entry| entry.id == requested)
        .ok_or_else(|| {
            format!(
                "Requested asset version '{requested}' is unavailable; no other version was substituted"
            )
        })
}
