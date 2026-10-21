#![allow(dead_code)]
use std::fs;
use std::path::PathBuf;
fn configured_path(names: &[&str]) -> Option<PathBuf> {
    names.iter().find_map(|name| {
        std::env::var_os(name)
            .filter(|value| !value.to_string_lossy().trim().is_empty())
            .map(PathBuf::from)
    })
}
fn default_data_root(data_dir: Option<PathBuf>, home_dir: Option<PathBuf>) -> PathBuf {
    let (current, legacy) = if let Some(directory) = data_dir {
        (directory.join("eltharn_gate"), directory.join("ronner"))
    } else if let Some(directory) = home_dir {
        (directory.join(".eltharn_gate"), directory.join(".ronner"))
    } else {
        (PathBuf::from("./.eltharn_gate"), PathBuf::from("./.ronner"))
    };
    if !current.exists() && legacy.is_dir() {
        legacy
    } else {
        current
    }
}
#[derive(Debug, Clone)]
pub struct EltharnGatePaths {
    pub root: PathBuf,
}
impl EltharnGatePaths {
    pub fn discover() -> Self {
        let root = configured_path(&[
            "ELTHARN_GATE_DATA_DIR",
            "ELTHARN_GATE_HOME",
            "RONNER_DATA_DIR",
            "RONNER_HOME",
        ])
        .unwrap_or_else(|| default_data_root(dirs::data_dir(), dirs::home_dir()));
        let paths = Self { root };
        paths.ensure_directories();
        paths
    }
    pub fn ensure_directories(&self) {
        let dirs_to_create = [
            self.root.clone(),
            self.assets_dir(),
            self.asset_indexes_dir(),
            self.asset_objects_dir(),
            self.bin_dir(),
            self.resourcepacks_dir(),
            self.config_dir(),
            self.protocols_dir(),
            self.logs_dir(),
        ];
        for dir in dirs_to_create {
            let _ = fs::create_dir_all(&dir);
        }
    }
    pub fn assets_dir(&self) -> PathBuf {
        self.root.join("assets")
    }
    pub fn asset_indexes_dir(&self) -> PathBuf {
        self.assets_dir().join("indexes")
    }
    pub fn asset_objects_dir(&self) -> PathBuf {
        self.assets_dir().join("objects")
    }
    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }
    pub fn launch_pair_receipt_path(&self, protocol_version: &str) -> Option<PathBuf> {
        el_launcher_core::ProtocolId::new(protocol_version.to_owned()).ok()?;
        let mut file_name = String::from(".eltharn_launch_pair_");
        for ch in protocol_version.chars() {
            if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
                file_name.push(ch);
            } else {
                file_name.push('_');
            }
        }
        file_name.push_str(".json");
        Some(self.bin_dir().join(file_name))
    }
    pub fn launch_pair_receipt_read_path(&self, protocol_version: &str) -> Option<PathBuf> {
        let current = self.launch_pair_receipt_path(protocol_version)?;
        let legacy_name = current.file_name()?.to_str()?.replacen(
            ".eltharn_launch_pair_",
            ".rustore_launch_pair_",
            1,
        );
        let legacy = self.bin_dir().join(legacy_name);
        Some(if !current.exists() && legacy.is_file() {
            legacy
        } else {
            current
        })
    }
    pub fn resourcepacks_dir(&self) -> PathBuf {
        self.root.join("resourcepacks")
    }
    pub fn skins_dir(&self) -> PathBuf {
        self.root.join("skins")
    }
    pub fn skins_heads_dir(&self) -> PathBuf {
        self.skins_dir().join("heads")
    }
    pub fn config_dir(&self) -> PathBuf {
        self.root.join("config")
    }
    pub fn config_file(&self) -> PathBuf {
        self.config_dir().join("launcher_config.json")
    }
    pub fn accounts_file(&self) -> PathBuf {
        self.config_dir().join("accounts.json")
    }
    pub fn protocols_dir(&self) -> PathBuf {
        self.root.join("protocols")
    }
    pub fn backends_dir(&self) -> PathBuf {
        self.root.join("backends")
    }
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }
    pub fn eltharn_gate_log_file(&self) -> PathBuf {
        self.logs_dir().join("eltharn_gate.log")
    }
    pub fn crash_log_file(&self) -> PathBuf {
        self.logs_dir().join("crash.log")
    }
    pub fn find_eltharn_executable(&self) -> Option<PathBuf> {
        if let Some(runtime) = crate::bundled_runtime() {
            return Some(runtime.executable.clone());
        }
        self.find_eltharn_executable_with_dev_root(
            configured_path(&["ELTHARN_GATE_DEV_ROOT", "RONNER_DEV_ROOT"]).as_deref(),
        )
    }
    fn find_eltharn_executable_with_dev_root(
        &self,
        dev_root: Option<&std::path::Path>,
    ) -> Option<PathBuf> {
        #[cfg(windows)]
        let names = ["eltharn.exe", "eltharn_client.exe"];
        #[cfg(not(windows))]
        let names = ["eltharn", "eltharn_client"];
        if let Some(root) = dev_root {
            return ["release", "debug"]
                .into_iter()
                .flat_map(|profile| names.map(|name| root.join("target").join(profile).join(name)))
                .find(|path| path.is_file());
        }
        names
            .into_iter()
            .map(|name| self.bin_dir().join(name))
            .find(|path| path.is_file())
            .or_else(|| {
                let exe = std::env::current_exe().ok()?;
                let parent = exe.parent()?;
                names
                    .into_iter()
                    .map(|name| parent.join(name))
                    .find(|path| path.is_file())
            })
    }
    pub fn find_protocol_backend(&self, protocol: &str) -> Option<PathBuf> {
        if crate::bundled_runtime()
            .is_some_and(|runtime| runtime.manifest.minecraft_version == protocol)
        {
            return Some(PathBuf::from(format!("builtin:{protocol}")));
        }
        self.find_protocol_backend_with_dev_root(
            protocol,
            configured_path(&["ELTHARN_GATE_DEV_ROOT", "RONNER_DEV_ROOT"]).as_deref(),
        )
    }
    fn find_protocol_backend_with_dev_root(
        &self,
        protocol: &str,
        dev_root: Option<&std::path::Path>,
    ) -> Option<PathBuf> {
        el_launcher_core::ProtocolId::new(protocol.to_owned()).ok()?;
        #[cfg(windows)]
        let names = [
            "eltharn-protocol.exe".to_string(),
            format!("eltharn-protocol-{protocol}.exe"),
            "eltharn_azalea_protocol_backend.exe".to_string(),
        ];
        #[cfg(not(windows))]
        let names = [
            "eltharn-protocol".to_string(),
            format!("eltharn-protocol-{protocol}"),
            "eltharn_azalea_protocol_backend".to_string(),
        ];
        if let Some(root) = dev_root {
            let versioned = root
                .join("protocol_backends")
                .join(format!("azalea_{}", protocol.replace('.', "_")));
            for profile in ["release", "debug"] {
                for name in &names {
                    let candidate = versioned.join("target").join(profile).join(name);
                    if candidate.is_file() && candidate.parent()?.join("manifest.json").is_file() {
                        return Some(candidate);
                    }
                }
            }
            return None;
        }
        for directory in [
            self.protocols_dir().join(protocol),
            self.backends_dir().join(protocol),
        ] {
            if directory.join("manifest.json").is_file() {
                return Some(directory);
            }
        }
        None
    }
}
