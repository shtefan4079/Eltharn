use el_protocol_backend::{
    BackendToClientMessage, ClientToBackendMessage, ELTHARN_PROTOCOL_BACKEND_IPC_VERSION,
    ProtocolBackendManifest,
};
use std::path::Path;
pub(crate) fn validate_manifest(
    manifest: &ProtocolBackendManifest,
    requested_version: &str,
) -> Result<(), String> {
    if requested_version.trim().is_empty()
        || manifest.minecraft_version != requested_version
        || manifest.ipc_version != ELTHARN_PROTOCOL_BACKEND_IPC_VERSION
        || manifest.assets_compatibility.as_deref()
            != Some(el_launcher_core::ELTHARN_CLIENT_ASSETS_COMPATIBILITY)
        || manifest.protocol_version <= 0
        || manifest.backend_name.trim().is_empty()
        || manifest.protocol_source.trim().is_empty()
    {
        return Err("Packaged backend descriptor does not match the requested contract".into());
    }
    Ok(())
}
pub(crate) fn probe_manifest(
    executable: &Path,
    requested_version: &str,
) -> Result<ProtocolBackendManifest, String> {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        process::{Command, Stdio},
        sync::mpsc,
        thread,
        time::Duration,
    };
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Packaged backend could not start: {error}"))?;
    let result = (|| {
        let mut stdin = child.stdin.take().ok_or("Missing backend probe input")?;
        let stdout = child.stdout.take().ok_or("Missing backend probe output")?;
        let hello = el_protocol_backend::encode_json_line(&ClientToBackendMessage::Hello {
            ipc_version: ELTHARN_PROTOCOL_BACKEND_IPC_VERSION,
            requested_minecraft_version: requested_version.to_owned(),
        })
        .map_err(|error| error.to_string())?;
        stdin.write_all(&hello).map_err(|error| error.to_string())?;
        stdin.flush().map_err(|error| error.to_string())?;
        let (tx, rx) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("eltharn-backend-descriptor".into())
            .spawn(move || {
                let mut reader = BufReader::new(stdout).take(65_537);
                let mut line = Vec::new();
                let result = reader
                    .read_until(b'\n', &mut line)
                    .map_err(|e| e.to_string());
                let result = result.and_then(|_| {
                    if line.len() > 65_536 {
                        return Err("Backend descriptor exceeds 64 KiB".to_owned());
                    }
                    el_protocol_backend::decode_json_line::<BackendToClientMessage>(&line)
                        .map_err(|error| error.to_string())
                });
                let _ = tx.send(result);
            })
            .map_err(|error| error.to_string())?;
        let message = rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "Packaged backend descriptor timed out".to_owned())??;
        let BackendToClientMessage::Hello { manifest } = message else {
            return Err("Packaged backend did not return an IPC Hello".into());
        };
        validate_manifest(&manifest, requested_version)?;
        Ok(manifest)
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}
#[cfg(target_os = "android")]
static SELECTED: std::sync::OnceLock<(std::path::PathBuf, ProtocolBackendManifest)> =
    std::sync::OnceLock::new();
#[cfg(target_os = "android")]
pub(crate) fn select(executable: std::path::PathBuf, manifest: ProtocolBackendManifest) {
    let _ = SELECTED.set((executable, manifest));
}
#[cfg(target_os = "android")]
pub(crate) fn wire_protocol(executable: &Path, requested_version: &str) -> Option<i32> {
    let (selected, manifest) = SELECTED.get()?;
    (selected == executable && validate_manifest(manifest, requested_version).is_ok())
        .then_some(manifest.protocol_version)
}
