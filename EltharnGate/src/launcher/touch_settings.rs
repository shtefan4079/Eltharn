use super::types::LauncherConfig;
use el_launcher_core::touch_controls::{TOUCH_CONTROL_PROFILE_FILE, TouchControlProfile};
use std::{io::Write, path::Path};

fn save_json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("Settings file has no parent directory")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let mut pending = tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    pending
        .write_all(&bytes)
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

pub fn save_launcher_config(path: &Path, config: &LauncherConfig) -> Result<(), String> {
    save_json(path, config)
}

pub fn save_profile(root: &Path, profile: &TouchControlProfile) -> Result<(), String> {
    profile.validate().map_err(|error| error.to_string())?;
    save_json(&root.join(TOUCH_CONTROL_PROFILE_FILE), profile)
}
