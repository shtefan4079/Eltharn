use std::{io::Write, path::Path};
pub fn save(path: &Path, accounts: &[el_launcher_core::StoredAccountV1]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("Account store has no parent directory")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    let bytes =
        el_launcher_core::encode_legacy_accounts(accounts).map_err(|error| error.to_string())?;
    let mut pending = tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        pending
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|error| error.to_string())?;
    }
    pending
        .write_all(bytes.as_bytes())
        .map_err(|error| error.to_string())?;
    pending
        .as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    pending
        .persist(path)
        .map_err(|error| error.error.to_string())?;
    Ok(())
}
