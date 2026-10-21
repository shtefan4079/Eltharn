use std::{path::PathBuf, time::Instant};

pub struct Capture {
    output: PathBuf,
    started: Instant,
    requested: bool,
}

impl Capture {
    pub fn from_env() -> Result<Option<Self>, String> {
        let Some(output) = std::env::var_os("ELTHARN_GATE_UI_CAPTURE") else {
            return Ok(None);
        };
        let root = PathBuf::from(
            std::env::var_os("ELTHARN_GATE_DATA_DIR")
                .ok_or("UI capture requires an explicit isolated ELTHARN_GATE_DATA_DIR")?,
        );
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        if dirs::data_dir().is_some_and(|p| {
            ["eltharn_gate", "ronner"]
                .iter()
                .any(|name| p.join(name).canonicalize().ok().as_ref() == Some(&root))
        }) || std::env::var_os("ELTHARN_GATE_DEV_ROOT").is_some()
            || std::env::var_os("RONNER_DEV_ROOT").is_some()
            || root.join("config/accounts.json").symlink_metadata().is_ok()
        {
            return Err("UI capture requires a separate data directory without accounts or developer routing".into());
        }
        let output = PathBuf::from(output);
        if !output.is_absolute() || output.exists() || output.with_extension("json").exists() {
            return Err("UI capture needs a new absolute output path".into());
        }
        Ok(Some(Self {
            output,
            started: Instant::now(),
            requested: false,
        }))
    }

    pub fn ready(&mut self) -> bool {
        if self.started.elapsed().as_secs() > 15 {
            eprintln!("UI capture timed out");
            std::process::exit(1);
        }
        let delay = std::env::var("ELTHARN_GATE_UI_CAPTURE_DELAY_MS")
            .ok()
            .and_then(|value| value.parse::<u128>().ok())
            .unwrap_or(1600)
            .clamp(1600, 12000);
        if !self.requested && self.started.elapsed().as_millis() >= delay {
            self.requested = true;
            true
        } else {
            false
        }
    }

    pub fn save(&self, screenshot: &iced::window::Screenshot) -> Result<(), String> {
        use std::io::Write;
        let size = screenshot.size;
        if screenshot.rgba.len() != size.width as usize * size.height as usize * 4 {
            return Err("Invalid screenshot byte count".into());
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.output)
            .map_err(|e| e.to_string())?;
        file.write_all(&screenshot.rgba)
            .map_err(|e| e.to_string())?;
        let metadata = serde_json::json!({"width":size.width,"height":size.height,"format":"RGBA8","scale_factor":screenshot.scale_factor});
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.output.with_extension("json"))
            .map_err(|e| e.to_string())?;
        file.write_all(metadata.to_string().as_bytes())
            .map_err(|e| e.to_string())
    }
}
