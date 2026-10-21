#![allow(dead_code)]
use crate::launcher::paths::EltharnGatePaths;
use crate::launcher::utils::sha1_hex;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
pub const ELTHARN_ASSETS_COMPATIBILITY_VERSION: &str =
    el_launcher_core::ELTHARN_CLIENT_ASSETS_COMPATIBILITY;
fn builtin_runtime(selection: &str) -> Result<Option<&'static crate::BundledRuntime>, String> {
    let target = el_launcher_core::BackendTarget::parse(selection)?;
    let Some(protocol) = target.builtin_version() else {
        return Ok(None);
    };
    let runtime =
        crate::bundled_runtime().ok_or("This launcher has no built-in protocol backend")?;
    if runtime.manifest.minecraft_version != protocol {
        return Err(format!("Built-in backend does not support {protocol}"));
    }
    Ok(Some(runtime))
}
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ClientProtocolInfo {
    pub networking: String,
    pub ipc_version: u32,
    pub assets_compatibility: String,
}
pub fn inspect_client_protocol(executable: &std::path::Path) -> Result<ClientProtocolInfo, String> {
    let output = Command::new(executable)
        .arg("--eltharnPrintProtocolInfo")
        .output()
        .map_err(|e| format!("Could not inspect Eltharn executable: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "Eltharn protocol inspection failed: {}",
            output.status
        ));
    }
    let info: ClientProtocolInfo = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Eltharn executable returned invalid protocol information: {e}"))?;
    if info.networking != "ipc"
        || info.ipc_version == 0
        || info.assets_compatibility != ELTHARN_ASSETS_COMPATIBILITY_VERSION
    {
        return Err(
            "Eltharn client requires an IPC-only build with compatible assets. Update the client/backend pair."
                .into(),
        );
    }
    Ok(info)
}
const LAUNCH_PAIR_RECEIPT_FORMAT: u32 = 1;
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct LaunchPairReceipt {
    format_version: u32,
    protocol_version: String,
    executable_sha1: String,
    backend_manifest_sha1: Option<String>,
    backend_binary_sha1: Option<String>,
}
fn hash_file_sha1(path: &std::path::Path) -> Result<String, String> {
    use sha1::{Digest, Sha1};
    use std::io::Read;
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
fn backend_bundle_hashes(
    bundle_dir: &std::path::Path,
) -> Result<(Option<String>, Option<String>), String> {
    let manifest_path = bundle_dir.join("manifest.json");
    let manifest_bytes = fs::read(&manifest_path).map_err(|_| {
        format!(
            "Protocol backend manifest missing at {}",
            manifest_path.display()
        )
    })?;
    let manifest_sha1 = sha1_hex(&manifest_bytes);
    let manifest =
        serde_json::from_slice::<el_launcher_core::ProtocolBackendManifest>(&manifest_bytes)
            .map_err(|e| format!("Invalid protocol backend manifest: {e}"))?;
    el_launcher_core::validate_relative_executable(&manifest)
        .map_err(|e| format!("Unsafe protocol backend executable: {e}"))?;
    let binary_path = bundle_dir.join(&manifest.executable);
    if !binary_path.is_file() {
        return Err(format!(
            "Protocol backend binary missing at {}",
            binary_path.display()
        ));
    }
    Ok((Some(manifest_sha1), Some(hash_file_sha1(&binary_path)?)))
}
fn current_backend_hashes(
    protocol_backend: &Option<String>,
) -> Result<(Option<String>, Option<String>), String> {
    let Some(raw) = protocol_backend else {
        return Err(
            "Protocol backend is required. Install the selected versioned IPC bundle.".into(),
        );
    };
    if let Some(runtime) = builtin_runtime(raw)? {
        let bytes = serde_json::to_vec(&runtime.manifest).map_err(|error| error.to_string())?;
        return Ok((
            Some(sha1_hex(&bytes)),
            Some(hash_file_sha1(&runtime.executable)?),
        ));
    }
    if raw.trim().is_empty() {
        return Err("Protocol backend selection is empty.".into());
    }
    let path = PathBuf::from(raw);
    if path.is_dir() {
        return backend_bundle_hashes(&path);
    }
    if path.is_file() {
        if path
            .file_name()
            .map_or(false, |name| name == "manifest.json")
        {
            let Some(parent) = path.parent() else {
                return Err(format!(
                    "Protocol backend manifest missing at {}",
                    path.display()
                ));
            };
            return backend_bundle_hashes(parent);
        }
        let parent = path
            .parent()
            .ok_or("Protocol backend has no bundle directory")?;
        let declared = backend_executable_path(&parent.to_string_lossy())?;
        if declared != path.canonicalize().map_err(|e| e.to_string())? {
            return Err("Selected backend differs from its manifest executable.".into());
        }
        return backend_bundle_hashes(parent);
    }
    Err(format!("Protocol backend missing at {}", path.display()))
}
fn backend_executable_path(selection: &str) -> Result<PathBuf, String> {
    let selected = PathBuf::from(selection);
    let manifest_path = if selected.is_dir() {
        Some(selected.join("manifest.json"))
    } else if selected
        .file_name()
        .is_some_and(|name| name == "manifest.json")
    {
        Some(selected.clone())
    } else {
        None
    };
    let executable = if let Some(manifest_path) = manifest_path {
        let text = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Could not read {}: {e}", manifest_path.display()))?;
        let manifest = el_launcher_core::parse_protocol_backend_manifest(&text)
            .map_err(|e| format!("Invalid protocol backend manifest: {e}"))?;
        el_launcher_core::validate_relative_executable(&manifest)
            .map_err(|e| format!("Unsafe protocol backend executable: {e}"))?;
        manifest_path.parent().unwrap().join(manifest.executable)
    } else {
        selected
    };
    if !executable.is_file() {
        return Err(format!(
            "Protocol backend executable missing at {}",
            executable.display()
        ));
    }
    executable.canonicalize().map_err(|e| {
        format!(
            "Could not resolve protocol backend {}: {e}",
            executable.display()
        )
    })
}
pub fn validated_backend_manifest(
    selection: &str,
    protocol: &str,
    client: &ClientProtocolInfo,
) -> Result<el_launcher_core::ProtocolBackendManifest, String> {
    el_launcher_core::ProtocolId::new(protocol.to_owned()).map_err(|e| e.to_string())?;
    if let Some(runtime) = builtin_runtime(selection)? {
        el_launcher_core::validate_protocol_backend_compatibility(
            &runtime.manifest,
            protocol,
            &client.assets_compatibility,
            client.ipc_version,
        )
        .map_err(|error| error.to_string())?;
        return Ok(runtime.manifest.clone());
    }
    let path = Path::new(selection);
    let manifest_path = if path.is_dir() {
        path.join("manifest.json")
    } else if path.file_name().is_some_and(|name| name == "manifest.json") {
        path.to_path_buf()
    } else {
        path.parent()
            .ok_or("Protocol backend has no bundle directory")?
            .join("manifest.json")
    };
    let text = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Could not read {}: {e}", manifest_path.display()))?;
    let manifest = el_launcher_core::parse_protocol_backend_manifest(&text)
        .map_err(|e| format!("Invalid protocol backend manifest: {e}"))?;
    el_launcher_core::validate_relative_executable(&manifest).map_err(|e| e.to_string())?;
    if manifest.protocol_version <= 0
        || manifest.assets_compatibility.as_deref() != Some(client.assets_compatibility.as_str())
    {
        return Err(
            "Backend manifest requires a valid wire protocol number and explicit matching asset compatibility."
                .into(),
        );
    }
    el_launcher_core::validate_protocol_backend_compatibility(
        &manifest,
        protocol,
        &client.assets_compatibility,
        client.ipc_version,
    )
    .map_err(|e| e.to_string())?;
    backend_executable_path(selection)?;
    Ok(manifest)
}
pub fn record_launched_pair(paths: &EltharnGatePaths, params: &LaunchParams) -> Result<(), String> {
    let receipt_path = paths
        .launch_pair_receipt_path(&params.protocol_version)
        .ok_or_else(|| format!("Invalid protocol version '{}'", params.protocol_version))?;
    if !params.executable_path.is_file() {
        return Err(format!(
            "Eltharn executable not found at {}",
            params.executable_path.display()
        ));
    }
    let (backend_manifest_sha1, backend_binary_sha1) =
        current_backend_hashes(&params.protocol_backend)?;
    let executable_sha1 = hash_file_sha1(&params.executable_path)?;
    validated_backend_manifest(
        params
            .protocol_backend
            .as_deref()
            .ok_or("Protocol backend is required")?,
        &params.protocol_version,
        &inspect_client_protocol(&params.executable_path)?,
    )?;
    let receipt = LaunchPairReceipt {
        format_version: LAUNCH_PAIR_RECEIPT_FORMAT,
        protocol_version: params.protocol_version.clone(),
        executable_sha1,
        backend_manifest_sha1,
        backend_binary_sha1,
    };
    if let Some(parent) = receipt_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    let pending = receipt_path.with_extension(format!("json.{}.pending", std::process::id()));
    fs::write(
        &pending,
        serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("Could not stage launch pair receipt: {e}"))?;
    fs::rename(&pending, &receipt_path)
        .map_err(|e| format!("Could not publish launch pair receipt: {e}"))?;
    Ok(())
}
pub fn record_pair_cli_usage() -> &'static str {
    "Usage: eltharn_gate --record-pair <protocol-version>"
}
pub fn initialize_builtin_installation(paths: &EltharnGatePaths) -> Result<(), String> {
    let Some(runtime) = crate::bundled_runtime() else {
        return Ok(());
    };
    let version = &runtime.manifest.minecraft_version;
    if paths
        .launch_pair_receipt_read_path(version)
        .is_some_and(|path| path.exists())
    {
        return Ok(());
    }
    record_launched_pair(
        paths,
        &LaunchParams {
            executable_path: runtime.executable.clone(),
            player_name: String::new(),
            uuid: String::new(),
            access_token: String::new(),
            protocol_version: version.clone(),
            protocol_backend: Some(format!("builtin:{version}")),
            extension_request: None,
        },
    )
}
pub fn verify_builtin_installation(paths: &EltharnGatePaths) -> Result<(), String> {
    let runtime = crate::bundled_runtime().ok_or("No built-in desktop runtime")?;
    let protocol = &runtime.manifest.minecraft_version;
    verify_pinned_executable(paths, protocol, &runtime.executable)?;
    let receipt = read_pinned_receipt(paths, protocol)?;
    let (manifest, binary) = current_backend_hashes(&Some(format!("builtin:{protocol}")))?;
    if !pair_leg_matches(&manifest, &receipt.backend_manifest_sha1)
        || !pair_leg_matches(&binary, &receipt.backend_binary_sha1)
    {
        return Err(
            "Built-in backend receipt mismatch: reinstall the desktop executable".to_owned(),
        );
    }
    Ok(())
}
pub fn record_pair_for_protocol(
    paths: &EltharnGatePaths,
    protocol_version: &str,
) -> Result<(), String> {
    let protocol = protocol_version.trim().to_string();
    if paths.launch_pair_receipt_path(&protocol).is_none() {
        return Err(format!(
            "Invalid protocol version '{protocol}'. {}",
            record_pair_cli_usage()
        ));
    }
    let executable = paths
        .find_eltharn_executable()
        .ok_or_else(|| {
            "Eltharn executable not found. Place it in bin/ (or build the client), prepare assets, then reinstall or update the client/backend pair with `eltharn_gate --record-pair <protocol-version>`"
                .to_string()
        })?;
    let backend = paths
        .find_protocol_backend(&protocol)
        .ok_or_else(|| {
            format!(
                "Protocol backend missing for '{protocol}'. Place the versioned bundle in protocols/{protocol}/ (manifest.json + binary), then reinstall or update the client/backend pair with `eltharn_gate --record-pair {protocol}`"
            )
        })?;
    let protocol_backend = Some(backend.to_string_lossy().to_string());
    crate::launcher::downloader::read_prepared_manifest(
        paths,
        ELTHARN_ASSETS_COMPATIBILITY_VERSION,
    )?;
    let verification = crate::launcher::downloader::verify_version_assets(
        paths,
        ELTHARN_ASSETS_COMPATIBILITY_VERSION,
    )?;
    if !verification.is_valid {
        return Err(format!(
            "Resources need repair before recording the pair: {}",
            verification.details.join("; ")
        ));
    }
    record_launched_pair(
        paths,
        &LaunchParams {
            executable_path: executable,
            player_name: String::new(),
            uuid: String::new(),
            access_token: String::new(),
            protocol_version: protocol,
            protocol_backend,
            extension_request: None,
        },
    )
}
fn pair_leg_matches(current: &Option<String>, pinned: &Option<String>) -> bool {
    match (current, pinned) {
        (None, None) => true,
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        _ => false,
    }
}
fn read_pinned_receipt(
    paths: &EltharnGatePaths,
    protocol_version: &str,
) -> Result<LaunchPairReceipt, String> {
    let receipt_path = paths
        .launch_pair_receipt_read_path(protocol_version)
        .ok_or_else(|| format!("Invalid protocol version '{protocol_version}'"))?;
    if !receipt_path.is_file() {
        return Err(format!(
            "Launch pair receipt missing for '{protocol_version}': reinstall or update the client/backend pair with `eltharn_gate --record-pair {protocol_version}`"
        ));
    }
    let raw =
        fs::read(&receipt_path).map_err(|e| format!("Could not read launch pair receipt: {e}"))?;
    let receipt: LaunchPairReceipt =
        serde_json::from_slice(&raw).map_err(|e| format!("Invalid launch pair receipt: {e}"))?;
    if receipt.format_version != LAUNCH_PAIR_RECEIPT_FORMAT
        || receipt.protocol_version != protocol_version
    {
        return Err("Launch pair receipt mismatch: reinstall the client/backend pair".to_string());
    }
    Ok(receipt)
}
pub fn verify_pinned_executable(
    paths: &EltharnGatePaths,
    protocol_version: &str,
    executable: &Path,
) -> Result<(), String> {
    let receipt = read_pinned_receipt(paths, protocol_version)?;
    let executable_sha1 = hash_file_sha1(executable)?;
    if !executable_sha1.eq_ignore_ascii_case(&receipt.executable_sha1) {
        return Err(
            "Eltharn executable hash mismatch: reinstall the client/backend pair".to_string(),
        );
    }
    Ok(())
}
pub fn inspect_pinned_client_protocol(
    paths: &EltharnGatePaths,
    protocol_version: &str,
    executable: &Path,
) -> Result<ClientProtocolInfo, String> {
    verify_pinned_executable(paths, protocol_version, executable)?;
    inspect_client_protocol(executable)
}
pub fn verify_launched_pair(paths: &EltharnGatePaths, params: &LaunchParams) -> Result<(), String> {
    if !params.executable_path.is_file() {
        return Err(format!(
            "Eltharn executable not found at {}",
            params.executable_path.display()
        ));
    }
    let (backend_manifest_sha1, backend_binary_sha1) =
        current_backend_hashes(&params.protocol_backend)?;
    let executable_sha1 = hash_file_sha1(&params.executable_path)?;
    let receipt = read_pinned_receipt(paths, &params.protocol_version)?;
    if !executable_sha1.eq_ignore_ascii_case(&receipt.executable_sha1) {
        return Err(
            "Eltharn executable hash mismatch: reinstall the client/backend pair".to_string(),
        );
    }
    if !pair_leg_matches(&backend_manifest_sha1, &receipt.backend_manifest_sha1)
        || !pair_leg_matches(&backend_binary_sha1, &receipt.backend_binary_sha1)
    {
        return Err(
            "Protocol backend hash mismatch: reinstall the client/backend pair".to_string(),
        );
    }
    let client = inspect_client_protocol(&params.executable_path)?;
    validated_backend_manifest(
        params
            .protocol_backend
            .as_deref()
            .ok_or("Protocol backend is required")?,
        &params.protocol_version,
        &client,
    )?;
    crate::launcher::downloader::read_prepared_manifest(
        paths,
        ELTHARN_ASSETS_COMPATIBILITY_VERSION,
    )?;
    let verification = crate::launcher::downloader::verify_version_assets(
        paths,
        ELTHARN_ASSETS_COMPATIBILITY_VERSION,
    )?;
    if !verification.is_valid {
        return Err(format!(
            "Resources need repair before launch: {}",
            verification.details.join("; ")
        ));
    }
    Ok(())
}
#[derive(Clone)]
pub struct LaunchParams {
    pub executable_path: PathBuf,
    pub player_name: String,
    pub uuid: String,
    pub access_token: String,
    pub protocol_version: String,
    pub protocol_backend: Option<String>,
    pub extension_request: Option<PathBuf>,
}
impl std::fmt::Debug for LaunchParams {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LaunchParams")
            .field("executable_path", &self.executable_path)
            .field("player_name", &self.player_name)
            .field("uuid", &self.uuid)
            .field("access_token", &"[redacted]")
            .field("protocol_version", &self.protocol_version)
            .field("protocol_backend", &self.protocol_backend)
            .field("extension_request", &self.extension_request)
            .finish()
    }
}
pub fn verify_extension_support(executable: &Path) -> Result<(), String> {
    let out = Command::new(executable)
        .arg("--eltharnPrintProtocolInfo")
        .output()
        .map_err(|e| format!("Cannot check extension support: {e}"))?;
    if !out.status.success() || out.stdout.len() > 4096 {
        return Err(
            "This client cannot report extension support. Update the client or launch without extensions."
                .into(),
        );
    }
    let info: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(
        |_| "This client does not support EltharnGate extension profiles. Update it first.",
    )?;
    if info["extensions"]["launch_request"] != el_extension_manager::launch::CATALOG_REQUEST_VERSION
        || info["extensions"]["shaderpacks"] != true
    {
        return Err(
            "This client does not support installed extension catalogs. Update the client/backend pair with an extensions-enabled build."
                .into(),
        );
    }
    Ok(())
}
pub fn extension_startup_error(request: &Path) -> Option<String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    let result = std::fs::File::open(request.with_extension("status.json"))
        .and_then(|f| f.take(8 * 1024 * 1024 + 1).read_to_end(&mut bytes));
    if result.is_err() {
        return Some("The client exited before confirming extension startup".into());
    }
    if bytes.len() > 8 * 1024 * 1024 {
        return Some("Invalid extension startup report".into());
    }
    let v: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return Some("Invalid extension startup report".into()),
    };
    if v["status"] == "ready" {
        None
    } else {
        Some(
            v["error"]
                .as_str()
                .unwrap_or("The client rejected the selected packages")
                .into(),
        )
    }
}
pub fn launch_eltharn_process(
    paths: &EltharnGatePaths,
    params: &LaunchParams,
) -> Result<std::process::Child, String> {
    if !params.executable_path.is_file() {
        return Err(format!(
            "Eltharn executable not found at {}",
            params.executable_path.display()
        ));
    }
    paths.ensure_directories();
    let prepared = crate::launcher::downloader::read_prepared_manifest(
        paths,
        ELTHARN_ASSETS_COMPATIBILITY_VERSION,
    )?;
    let verification = crate::launcher::downloader::verify_version_assets(
        paths,
        ELTHARN_ASSETS_COMPATIBILITY_VERSION,
    )?;
    if !verification.is_valid {
        return Err(format!(
            "Resources need repair before launch: {}",
            verification.details.join("; ")
        ));
    }
    verify_launched_pair(paths, params)?;
    let builtin_backend = params
        .protocol_backend
        .as_deref()
        .map(builtin_runtime)
        .transpose()?
        .flatten()
        .is_some();
    let backend_executable = params
        .protocol_backend
        .as_deref()
        .filter(|backend| !backend.trim().is_empty())
        .map(|selection| {
            if builtin_backend {
                Ok(PathBuf::from(selection))
            } else {
                backend_executable_path(selection)
            }
        })
        .transpose()?;
    let mut command = Command::new(&params.executable_path);
    if crate::bundled_runtime().is_some_and(|runtime| runtime.executable == params.executable_path)
    {
        command.args(["--mode", "game"]);
    }
    command
        .env_remove("ELTHARN_EXTENSION_REQUEST")
        .env_remove("ELTHARN_EXTENSION_STATUS")
        .env_remove("ELTHARN_SHADERPACK_PROBE_PACKET")
        .env_remove("ELTHARN_SHADERPACK_PROBE_CONTROL_DIR");
    if let Some(request) = &params.extension_request {
        verify_extension_support(&params.executable_path)?;
        command.env("ELTHARN_EXTENSION_REQUEST", request);
        command.env(
            "ELTHARN_EXTENSION_STATUS",
            request.with_extension("status.json"),
        );
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command.arg("--accessToken").arg("null");
    if !params.access_token.is_empty() && params.access_token != "null" {
        eprintln!(
            "[EltharnGate] Withheld the account access token from the command line; the client does not consume it yet."
        );
    }
    command
        .current_dir(&paths.root)
        .arg("--username")
        .arg(&params.player_name)
        .arg("--uuid")
        .arg(&params.uuid)
        .arg("--gameDir")
        .arg(&paths.root)
        .arg("--assetsDir")
        .arg(paths.assets_dir())
        .arg("--assetIndex")
        .arg(&prepared.asset_index)
        .arg("--eltharnPreparedAssets")
        .arg(crate::launcher::downloader::prepared_manifest_path(
            paths,
            ELTHARN_ASSETS_COMPATIBILITY_VERSION,
        ))
        .arg("--resourcePackDir")
        .arg(paths.resourcepacks_dir())
        .arg("--version")
        .arg(ELTHARN_ASSETS_COMPATIBILITY_VERSION)
        .arg("--eltharnProtocolVersion")
        .arg(&params.protocol_version);
    if let Some(backend) = backend_executable {
        command.arg("--eltharnProtocolBackend").arg(backend);
    }
    command
        .spawn()
        .map_err(|e| format!("Failed to spawn Eltharn process: {e}"))
}
