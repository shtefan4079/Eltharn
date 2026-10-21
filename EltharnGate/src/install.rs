use std::{io, path::PathBuf};
pub(crate) fn install() -> Result<PathBuf, String> {
    let runtime =
        crate::bundled_runtime().ok_or("Installation requires the unified desktop executable")?;
    let paths = crate::launcher::paths::EltharnGatePaths::discover();
    paths.ensure_directories();
    let name = if cfg!(windows) {
        "eltharn.exe"
    } else {
        "eltharn"
    };
    let destination = paths.bin_dir().join(name);
    if destination.canonicalize().ok().as_ref() != Some(&runtime.executable) {
        let mut source =
            std::fs::File::open(&runtime.executable).map_err(|error| error.to_string())?;
        let mut pending =
            tempfile::NamedTempFile::new_in(paths.bin_dir()).map_err(|error| error.to_string())?;
        io::copy(&mut source, &mut pending).map_err(|error| error.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            pending
                .as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o755))
                .map_err(|error| error.to_string())?;
        }
        pending
            .as_file()
            .sync_all()
            .map_err(|error| error.to_string())?;
        pending
            .persist(&destination)
            .map_err(|error| error.error.to_string())?;
    }
    crate::launcher::executor::record_launched_pair(
        &paths,
        &crate::launcher::executor::LaunchParams {
            executable_path: destination.clone(),
            player_name: String::new(),
            uuid: String::new(),
            access_token: String::new(),
            protocol_version: runtime.manifest.minecraft_version.clone(),
            protocol_backend: Some(format!("builtin:{}", runtime.manifest.minecraft_version)),
            extension_request: None,
        },
    )?;
    Ok(destination)
}
