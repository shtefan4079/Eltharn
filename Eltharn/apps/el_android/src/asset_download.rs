use reqwest::blocking::{Client, Response};
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

const GAME_ASSET_VERSION: &str = "26.2";
const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const ASSET_BASE_URL: &str = "https://resources.download.minecraft.net";
const MAX_METADATA_BYTES: usize = 32 * 1024 * 1024;
const MAX_ASSET_INDEX_BYTES: usize = 128 * 1024 * 1024;
const MAX_ASSET_OBJECT_BYTES: usize = 64 * 1024 * 1024;
const MAX_CLIENT_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_CLIENT_ASSET_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_EXTRACTED_CLIENT_ASSET_BYTES: u64 = 128 * 1024 * 1024;
const MAX_CLIENT_ASSET_FILES: usize = 50_000;
const MAX_CLIENT_ASSET_MANIFEST_BYTES: usize = 16 * 1024 * 1024;
const CLIENT_ASSET_MANIFEST: &str = ".eltharn-client-assets-v1";

struct ClientAssetReceipt {
    file_count: usize,
    total_bytes: u64,
    manifest_sha1: String,
}

#[derive(Deserialize)]
struct VersionManifest {
    versions: Vec<VersionEntry>,
}

#[derive(Deserialize)]
struct VersionEntry {
    id: String,
    url: String,
    sha1: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionMetadata {
    id: String,
    asset_index: AssetIndexMetadata,
    downloads: VersionDownloads,
}

#[derive(Deserialize)]
struct VersionDownloads {
    client: DownloadMetadata,
}

#[derive(Deserialize)]
struct DownloadMetadata {
    sha1: String,
    url: String,
}

#[derive(Deserialize)]
struct AssetIndexMetadata {
    id: String,
    sha1: String,
    url: String,
}

#[derive(Deserialize)]
struct AssetIndex {
    objects: std::collections::BTreeMap<String, AssetObject>,
}

#[derive(Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

pub fn prepare_assets(asset_root: &Path) -> Result<String, String> {
    fs::create_dir_all(asset_root).map_err(|error| format!("Create asset directory: {error}"))?;
    let marker = asset_root.join(".eltharn-assets-v10");
    if let Some(asset_index) = cached_asset_index(asset_root, &marker) {
        log::info!("Minecraft {GAME_ASSET_VERSION} resources already cached");
        return Ok(asset_index);
    }

    let client = Client::builder()
        .https_only(true)
        .user_agent("Eltharn Gate/0.1.0")
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|error| format!("Create asset downloader: {error}"))?;

    let manifest: VersionManifest = get_json(
        &client,
        VERSION_MANIFEST_URL,
        MAX_METADATA_BYTES,
        "Mojang version manifest",
    )?;
    let version_entry = manifest
        .versions
        .into_iter()
        .find(|entry| entry.id == GAME_ASSET_VERSION)
        .ok_or_else(|| format!("Minecraft asset version {GAME_ASSET_VERSION} is unavailable"))?;
    require_mojang_url(&version_entry.url)?;

    let metadata_bytes = get_bytes(&client, &version_entry.url, MAX_METADATA_BYTES)?;
    if let Some(expected) = &version_entry.sha1 {
        verify_sha1(&metadata_bytes, expected, "Minecraft version metadata")?;
    }
    let metadata: VersionMetadata = serde_json::from_slice(&metadata_bytes)
        .map_err(|error| format!("Parse Minecraft version metadata: {error}"))?;
    if metadata.id != GAME_ASSET_VERSION {
        return Err(format!(
            "Requested Minecraft {GAME_ASSET_VERSION}, received {}",
            metadata.id
        ));
    }
    require_mojang_url(&metadata.asset_index.url)?;
    validate_sha1(&metadata.asset_index.sha1)?;
    require_mojang_url(&metadata.downloads.client.url)?;
    validate_sha1(&metadata.downloads.client.sha1)?;
    if !valid_asset_index_id(&metadata.asset_index.id) {
        return Err("Minecraft asset index identifier is invalid".into());
    }

    let index_path = asset_root
        .join("indexes")
        .join(format!("{}.json", metadata.asset_index.id));
    log::info!("Downloading Minecraft {GAME_ASSET_VERSION} resources");
    let index_bytes = get_bytes(&client, &metadata.asset_index.url, MAX_ASSET_INDEX_BYTES)?;
    verify_sha1(
        &index_bytes,
        &metadata.asset_index.sha1,
        "Minecraft asset index",
    )?;
    let asset_index: AssetIndex = serde_json::from_slice(&index_bytes)
        .map_err(|error| format!("Parse Minecraft asset index: {error}"))?;
    write_atomic(&index_path, &index_bytes)?;

    let object_root = asset_root.join("objects");
    download_objects(&client, &object_root, &asset_index.objects)?;
    let client_archive = asset_root.join(".eltharn-client.jar.pending");
    download_verified_file(
        &client,
        &metadata.downloads.client.url,
        &client_archive,
        &metadata.downloads.client.sha1,
        MAX_CLIENT_ARCHIVE_BYTES,
        "Minecraft client archive",
    )?;
    let receipt = match extract_client_assets(&client_archive, asset_root) {
        Ok(receipt) => receipt,
        Err(error) => {
            let _ = fs::remove_file(&client_archive);
            return Err(error);
        }
    };
    fs::remove_file(&client_archive)
        .map_err(|error| format!("Remove temporary Minecraft client archive: {error}"))?;
    let marker_contents = format!(
        "{GAME_ASSET_VERSION}\n{}\n{}\n{}\n{}\n{}\n{}\n",
        metadata.asset_index.id,
        metadata.asset_index.sha1,
        metadata.downloads.client.sha1,
        receipt.file_count,
        receipt.total_bytes,
        receipt.manifest_sha1
    );
    write_atomic(marker.as_path(), marker_contents.as_bytes())?;
    log::info!(
        "Minecraft resources ready: {} indexed objects",
        asset_index.objects.len()
    );
    Ok(metadata.asset_index.id)
}

fn cached_asset_index(asset_root: &Path, marker: &Path) -> Option<String> {
    let marker = fs::read_to_string(marker).ok()?;
    let lines: Vec<_> = marker.lines().collect();
    if lines.len() != 7 || lines[0] != GAME_ASSET_VERSION || !valid_asset_index_id(lines[1]) {
        return None;
    }
    validate_sha1(lines[2]).ok()?;
    validate_sha1(lines[3]).ok()?;
    let expected_file_count = lines[4].parse::<usize>().ok()?;
    if expected_file_count == 0 || expected_file_count > MAX_CLIENT_ASSET_FILES {
        return None;
    }
    let expected_total_bytes = lines[5].parse::<u64>().ok()?;
    if expected_total_bytes > MAX_EXTRACTED_CLIENT_ASSET_BYTES {
        return None;
    }
    validate_sha1(lines[6]).ok()?;

    let index_path = asset_root
        .join("indexes")
        .join(format!("{}.json", lines[1]));
    let metadata = fs::metadata(&index_path).ok()?;
    if metadata.len() > MAX_ASSET_INDEX_BYTES as u64 {
        return None;
    }
    let index_bytes = fs::read(index_path).ok()?;
    verify_sha1(&index_bytes, lines[2], "Cached Minecraft asset index").ok()?;
    let asset_index = serde_json::from_slice::<AssetIndex>(&index_bytes).ok()?;
    if !cached_asset_objects_present(asset_root, &asset_index.objects) {
        return None;
    }
    let client_assets_manifest = asset_root.join(CLIENT_ASSET_MANIFEST);
    let manifest_metadata = fs::metadata(&client_assets_manifest).ok()?;
    if manifest_metadata.len() > MAX_CLIENT_ASSET_MANIFEST_BYTES as u64 {
        return None;
    }
    let manifest_bytes = fs::read(client_assets_manifest).ok()?;
    verify_sha1(&manifest_bytes, lines[6], "Cached client asset manifest").ok()?;
    let (actual_file_count, actual_total_bytes) =
        validate_client_asset_manifest(asset_root, &manifest_bytes).ok()?;
    if actual_file_count != expected_file_count || actual_total_bytes != expected_total_bytes {
        return None;
    }
    Some(lines[1].to_owned())
}

fn cached_asset_objects_present(
    asset_root: &Path,
    objects: &std::collections::BTreeMap<String, AssetObject>,
) -> bool {
    let object_root = asset_root.join("objects");
    let mut unique_objects = std::collections::BTreeMap::<&str, u64>::new();
    for (path, object) in objects {
        if !valid_virtual_asset_path(path)
            || validate_sha1(&object.hash).is_err()
            || object.size > MAX_ASSET_OBJECT_BYTES as u64
        {
            return false;
        }
        if let Some(previous_size) = unique_objects.insert(&object.hash, object.size) {
            if previous_size != object.size {
                return false;
            }
            continue;
        }
        let object_path = object_root.join(&object.hash[..2]).join(&object.hash);
        let Ok(metadata) = fs::metadata(object_path) else {
            return false;
        };
        if !metadata.is_file() || metadata.len() != object.size {
            return false;
        }
    }
    !unique_objects.is_empty()
}

fn valid_asset_index_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn get_json<T: for<'de> Deserialize<'de>>(
    client: &Client,
    url: &str,
    limit: usize,
    name: &str,
) -> Result<T, String> {
    let bytes = get_bytes(client, url, limit)?;
    serde_json::from_slice(&bytes).map_err(|error| format!("Parse {name}: {error}"))
}

fn get_bytes(client: &Client, url: &str, limit: usize) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .send()
        .and_then(Response::error_for_status)
        .map_err(|error| format!("Download {url}: {error}"))?;
    require_mojang_url(response.url().as_str())?;
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(format!("Download exceeds the {limit}-byte limit: {url}"));
    }
    let mut bytes = Vec::new();
    response
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Read download {url}: {error}"))?;
    if bytes.len() > limit {
        return Err(format!("Download exceeds the {limit}-byte limit: {url}"));
    }
    Ok(bytes)
}

fn require_mojang_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|error| format!("Invalid Mojang URL: {error}"))?;
    let host = parsed.host_str().unwrap_or_default();
    let allowed = ["mojang.com", "minecraft.net"]
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")));
    if parsed.scheme() != "https"
        || !allowed
        || parsed.username() != ""
        || parsed.password().is_some()
    {
        return Err(format!("Rejected Mojang download URL: {url}"));
    }
    Ok(())
}

fn validate_sha1(hash: &str) -> Result<(), String> {
    if hash.len() != 40 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Mojang metadata contains an invalid SHA-1 value".into());
    }
    Ok(())
}

fn verify_sha1(bytes: &[u8], expected: &str, name: &str) -> Result<(), String> {
    validate_sha1(expected)?;
    let actual = sha1_hex(bytes);
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(format!("{name} SHA-1 mismatch"));
    }
    Ok(())
}

fn sha1_hex(bytes: &[u8]) -> String {
    Sha1::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn download_verified_file(
    client: &Client,
    url: &str,
    destination: &Path,
    expected_sha1: &str,
    max_bytes: u64,
    name: &str,
) -> Result<(), String> {
    validate_sha1(expected_sha1)?;
    let response = client
        .get(url)
        .send()
        .and_then(Response::error_for_status)
        .map_err(|error| format!("Download {name}: {error}"))?;
    require_mojang_url(response.url().as_str())?;
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes)
    {
        return Err(format!("{name} exceeds the {max_bytes}-byte limit"));
    }

    let parent = destination
        .parent()
        .ok_or_else(|| format!("Invalid {name} path: {}", destination.display()))?;
    fs::create_dir_all(parent).map_err(|error| format!("Create {}: {error}", parent.display()))?;
    let temporary = parent.join(format!(".eltharn-{}-download.pending", std::process::id()));
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| format!("Create temporary {name}: {error}"))?;
    let mut response = response.take(max_bytes + 1);
    let mut hasher = Sha1::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|error| format!("Read {name}: {error}"))?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > max_bytes {
            let _ = fs::remove_file(&temporary);
            return Err(format!("{name} exceeds the {max_bytes}-byte limit"));
        }
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])
            .map_err(|error| format!("Write temporary {name}: {error}"))?;
    }
    let actual_sha1: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if !actual_sha1.eq_ignore_ascii_case(expected_sha1) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("{name} SHA-1 mismatch"));
    }
    file.sync_all()
        .map_err(|error| format!("Flush temporary {name}: {error}"))?;
    fs::rename(&temporary, destination).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("Publish {name}: {error}")
    })
}

fn extract_client_assets(
    archive_path: &Path,
    asset_root: &Path,
) -> Result<ClientAssetReceipt, String> {
    let archive_file = File::open(archive_path)
        .map_err(|error| format!("Open temporary Minecraft client archive: {error}"))?;
    let mut archive = zip::ZipArchive::new(archive_file)
        .map_err(|error| format!("Read Minecraft client archive: {error}"))?;
    let mut extracted_bytes = 0_u64;
    let mut entries = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("Read Minecraft client archive entry: {error}"))?;
        if entry.is_dir() {
            continue;
        }
        let Some(relative_path) = client_asset_path(entry.name()) else {
            continue;
        };
        let size = entry.size();
        if size > MAX_CLIENT_ASSET_FILE_BYTES
            || extracted_bytes.saturating_add(size) > MAX_EXTRACTED_CLIENT_ASSET_BYTES
        {
            return Err("Minecraft client archive contains oversized assets".into());
        }
        let mut contents = Vec::with_capacity(size as usize);
        entry
            .by_ref()
            .take(MAX_CLIENT_ASSET_FILE_BYTES + 1)
            .read_to_end(&mut contents)
            .map_err(|error| format!("Extract Minecraft client asset: {error}"))?;
        if contents.len() as u64 != size {
            return Err("Minecraft client asset has an invalid size".into());
        }
        extracted_bytes += size;
        let content_sha1 = sha1_hex(&contents);
        write_atomic(&asset_root.join(&relative_path), &contents)?;
        entries.push((relative_path, size, content_sha1));
        if entries.len() > MAX_CLIENT_ASSET_FILES {
            return Err("Minecraft client archive contains too many assets".into());
        }
    }

    if entries.is_empty() {
        return Err("Minecraft client archive contains no assets".into());
    }
    entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    if entries.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err("Minecraft client archive contains duplicate asset paths".into());
    }

    let mut manifest = String::new();
    for (path, size, content_sha1) in &entries {
        manifest.push_str(content_sha1);
        manifest.push('\t');
        manifest.push_str(&size.to_string());
        manifest.push('\t');
        manifest.push_str(path);
        manifest.push('\n');
        if manifest.len() > MAX_CLIENT_ASSET_MANIFEST_BYTES {
            return Err("Minecraft client asset manifest exceeds its size limit".into());
        }
    }
    let manifest_bytes = manifest.as_bytes();
    write_atomic(&asset_root.join(CLIENT_ASSET_MANIFEST), manifest_bytes)?;
    Ok(ClientAssetReceipt {
        file_count: entries.len(),
        total_bytes: extracted_bytes,
        manifest_sha1: sha1_hex(manifest_bytes),
    })
}

fn client_asset_path(archive_path: &str) -> Option<String> {
    if archive_path.contains('\\') || archive_path.chars().any(char::is_control) {
        return None;
    }
    let components: Vec<_> = archive_path.split('/').collect();
    if components.len() < 3
        || components[0] != "assets"
        || components[1].is_empty()
        || components[1] == "."
        || components[1] == ".."
        || components[2..]
            .iter()
            .any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return None;
    }
    Some(components[1..].join("/"))
}

fn validate_client_asset_manifest(
    asset_root: &Path,
    manifest: &[u8],
) -> Result<(usize, u64), String> {
    let manifest = std::str::from_utf8(manifest)
        .map_err(|error| format!("Invalid client asset manifest: {error}"))?;
    let mut file_count = 0_usize;
    let mut total_bytes = 0_u64;
    let mut previous_path = None::<&str>;

    for line in manifest.lines() {
        let mut fields = line.splitn(3, '\t');
        let content_sha1 = fields
            .next()
            .ok_or_else(|| "Invalid client asset manifest entry".to_string())?;
        let size = fields
            .next()
            .ok_or_else(|| "Invalid client asset manifest entry".to_string())?
            .parse::<u64>()
            .map_err(|error| format!("Invalid client asset size: {error}"))?;
        let relative_path = fields
            .next()
            .ok_or_else(|| "Invalid client asset manifest entry".to_string())?;
        validate_sha1(content_sha1)?;
        if !valid_client_asset_relative_path(relative_path)
            || previous_path.is_some_and(|previous| previous >= relative_path)
            || size > MAX_CLIENT_ASSET_FILE_BYTES
        {
            return Err("Invalid client asset manifest path or ordering".into());
        }
        let asset_path = asset_root.join(relative_path);
        let contents = fs::read(&asset_path).map_err(|error| {
            format!("Read cached client asset {}: {error}", asset_path.display())
        })?;
        if contents.len() as u64 != size {
            return Err(format!(
                "Cached client asset has an invalid size: {}",
                asset_path.display()
            ));
        }
        verify_sha1(&contents, content_sha1, "Cached client asset")?;
        previous_path = Some(relative_path);
        file_count += 1;
        total_bytes = total_bytes.saturating_add(size);
        if file_count > MAX_CLIENT_ASSET_FILES || total_bytes > MAX_EXTRACTED_CLIENT_ASSET_BYTES {
            return Err("Cached client asset manifest exceeds its limits".into());
        }
    }
    if file_count == 0 {
        return Err("Cached client asset manifest is empty".into());
    }
    Ok((file_count, total_bytes))
}

fn valid_client_asset_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn write_atomic(destination: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = destination
        .parent()
        .ok_or_else(|| format!("Invalid asset path: {}", destination.display()))?;
    fs::create_dir_all(parent).map_err(|error| format!("Create {}: {error}", parent.display()))?;
    let temporary = parent.join(format!(
        ".eltharn-{}-{}.pending",
        std::process::id(),
        destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("asset")
    ));
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| format!("Create temporary asset: {error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("Write temporary asset: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("Flush temporary asset: {error}"))?;
    fs::rename(&temporary, destination).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("Publish asset {}: {error}", destination.display())
    })
}

fn download_objects(
    client: &Client,
    object_root: &Path,
    objects: &std::collections::BTreeMap<String, AssetObject>,
) -> Result<(), String> {
    let mut unique_objects = std::collections::BTreeMap::<String, u64>::new();
    for (path, object) in objects {
        if !valid_virtual_asset_path(path)
            || validate_sha1(&object.hash).is_err()
            || object.size > MAX_ASSET_OBJECT_BYTES as u64
        {
            return Err("Minecraft asset index contains an invalid object entry".into());
        }
        if unique_objects
            .insert(object.hash.clone(), object.size)
            .is_some_and(|size| size != object.size)
        {
            return Err("Minecraft asset index assigns conflicting sizes to one object".into());
        }
    }
    let entries: Vec<_> = unique_objects
        .into_iter()
        .map(|(hash, size)| AssetObject { hash, size })
        .collect();
    let next = AtomicUsize::new(0);
    let completed = AtomicUsize::new(0);
    let first_error = Mutex::new(None::<String>);
    let worker_count = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(4)
        .clamp(2, 12)
        .min(entries.len().max(1));

    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            let client = client.clone();
            let entries = &entries;
            let next = &next;
            let completed = &completed;
            let first_error = &first_error;
            scope.spawn(move || {
                loop {
                    if first_error.lock().is_ok_and(|error| error.is_some()) {
                        break;
                    }
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(object) = entries.get(index) else {
                        break;
                    };
                    let result = download_asset_object(&client, object_root, object);
                    if let Err(error) = result {
                        if let Ok(mut slot) = first_error.lock() {
                            if slot.is_none() {
                                *slot = Some(error);
                            }
                        }
                        break;
                    }
                    let count = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    if count % 500 == 0 || count == entries.len() {
                        log::info!("Minecraft assets: {count}/{}", entries.len());
                    }
                }
            });
        }
    });

    if let Ok(mut error) = first_error.lock() {
        if let Some(error) = error.take() {
            return Err(error);
        }
    }
    Ok(())
}

fn download_asset_object(
    client: &Client,
    object_root: &Path,
    object: &AssetObject,
) -> Result<(), String> {
    let prefix = &object.hash[..2];
    let destination = object_root.join(prefix).join(&object.hash);
    if destination.is_file() {
        let metadata = fs::metadata(&destination)
            .map_err(|error| format!("Read cached asset metadata: {error}"))?;
        if metadata.len() == object.size {
            let bytes = fs::read(&destination)
                .map_err(|error| format!("Read cached asset object: {error}"))?;
            if verify_sha1(&bytes, &object.hash, "Cached object").is_ok() {
                return Ok(());
            }
        }
    }
    let url = format!("{ASSET_BASE_URL}/{prefix}/{}", object.hash);
    let bytes = get_bytes(client, &url, MAX_ASSET_OBJECT_BYTES)?;
    if bytes.len() as u64 != object.size {
        return Err(format!(
            "Minecraft asset object {} has the wrong size",
            object.hash
        ));
    }
    verify_sha1(&bytes, &object.hash, "Minecraft asset object")?;
    write_atomic(&destination, &bytes)
}

fn valid_virtual_asset_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}
