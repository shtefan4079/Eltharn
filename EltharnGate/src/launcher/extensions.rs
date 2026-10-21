use el_extension_manager::{native_store::NativeStore, package::PackageInfo};
use el_launcher_core::extensions::{ExtensionPreferences, ExtensionProfile, ShaderSelection};
use iced::{Element, Task};
use std::path::PathBuf;
mod catalog;
mod folders;
mod view;
use catalog::PackageKind;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CatalogTab {
    #[default]
    Mods,
    ShaderPacks,
}
impl CatalogTab {
    fn kind(self) -> PackageKind {
        match self {
            Self::Mods => PackageKind::Mod,
            Self::ShaderPacks => PackageKind::Shaderpack,
        }
    }
}
#[derive(Clone, Debug)]
pub enum ExtMessage {
    OpenFolder(CatalogTab),
    FoldersSynced(Result<folders::Snapshot, String>),
    Tab(CatalogTab),
    Search(String),
    Mods(bool),
    Shaders(bool),
    ToggleMod(usize, bool),
    SelectShader(usize),
    ClearShader,
}
#[derive(Clone, Debug)]
pub struct ExtPanel {
    pub prefs: Option<ExtensionPreferences>,
    pub catalog: Vec<PackageInfo>,
    pub error: Option<String>,
    pub busy: bool,
    tab: CatalogTab,
    search: String,
    root: PathBuf,
    folder_fingerprint: Option<folders::Fingerprint>,
    last_folder_check: std::time::Instant,
}
impl ExtPanel {
    pub fn new(root: PathBuf) -> Self {
        let store = NativeStore::new(root.clone());
        let (prefs, mut error) = match store.preferences() {
            Ok(p) => (Some(p), None),
            Err(e) => (None, Some(e)),
        };
        match store.catalog() {
            Ok(list) => Self {
                prefs,
                catalog: list,
                error,
                busy: false,
                tab: CatalogTab::default(),
                search: String::new(),
                root,
                folder_fingerprint: None,
                last_folder_check: std::time::Instant::now(),
            },
            Err(e) => {
                if error.is_none() {
                    error = Some(e);
                }
                Self {
                    prefs,
                    catalog: Vec::new(),
                    error,
                    busy: false,
                    tab: CatalogTab::default(),
                    search: String::new(),
                    root,
                    folder_fingerprint: None,
                    last_folder_check: std::time::Instant::now(),
                }
            }
        }
    }
    pub fn profile(&self, recovery: bool) -> Result<ExtensionProfile, String> {
        if self.busy {
            return Err("Extensions are busy. Try again in a moment.".to_owned());
        }
        let prefs = self
            .prefs
            .as_ref()
            .ok_or_else(|| "Extensions are unavailable.".to_owned())?;
        if recovery {
            return prefs.base_recovery();
        }
        if let Some(e) = &self.error {
            return Err(e.clone());
        }
        prefs
            .profiles
            .iter()
            .find(|p| p.id == prefs.active_profile)
            .cloned()
            .ok_or_else(|| "The active profile is missing.".to_owned())
    }
    pub fn sync_folders(&mut self) -> Task<ExtMessage> {
        if self.busy {
            return Task::none();
        }
        let Some(expected) = self.prefs.clone() else {
            return Task::none();
        };
        self.busy = true;
        self.last_folder_check = std::time::Instant::now();
        self.folder_fingerprint = folders::fingerprint(&self.root).ok();
        let root = self.root.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || folders::sync(root, expected))
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| result)
            },
            ExtMessage::FoldersSynced,
        )
    }
    pub fn observe_folders(&mut self) -> Task<ExtMessage> {
        if self.busy || self.last_folder_check.elapsed() < std::time::Duration::from_secs(2) {
            return Task::none();
        }
        self.last_folder_check = std::time::Instant::now();
        match folders::fingerprint(&self.root) {
            Ok(current) if self.folder_fingerprint.as_ref() == Some(&current) => Task::none(),
            Ok(_) => self.sync_folders(),
            Err(error) => {
                self.folder_fingerprint = None;
                self.error = Some(error);
                Task::none()
            }
        }
    }
    fn commit(&mut self, next: ExtensionPreferences) {
        let store = NativeStore::new(self.root.clone());
        let result = self
            .prefs
            .as_ref()
            .ok_or("Preferences unavailable".to_owned())
            .and_then(|previous| store.compare_and_save_preferences(previous, &next));
        match result {
            Ok(()) => {
                self.prefs = Some(next);
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
    }
    pub fn update(&mut self, msg: ExtMessage) -> Task<ExtMessage> {
        self.update_core(msg)
    }
    fn update_core(&mut self, msg: ExtMessage) -> Task<ExtMessage> {
        if self.busy
            && !matches!(
                &msg,
                ExtMessage::FoldersSynced(_)
                    | ExtMessage::Tab(_)
                    | ExtMessage::Search(_)
                    | ExtMessage::OpenFolder(_)
            )
        {
            return Task::none();
        }
        match msg {
            ExtMessage::OpenFolder(tab) => {
                let path = folders::folder(&self.root, tab);
                if let Err(error) = std::fs::create_dir_all(&path)
                    .map_err(|error| error.to_string())
                    .and_then(|()| super::open_folder(&path))
                {
                    self.error = Some(error);
                }
                Task::none()
            }
            ExtMessage::FoldersSynced(result) => {
                self.busy = false;
                match result {
                    Ok(snapshot) => {
                        self.prefs = Some(snapshot.prefs);
                        self.catalog = snapshot.catalog;
                        self.folder_fingerprint = Some(snapshot.fingerprint);
                        self.error = None;
                    }
                    Err(error) => {
                        if let Ok(prefs) = NativeStore::new(self.root.clone()).preferences() {
                            self.prefs = Some(prefs);
                        }
                        self.error = Some(error);
                    }
                }
                Task::none()
            }
            ExtMessage::Tab(tab) => {
                self.tab = tab;
                Task::none()
            }
            ExtMessage::Search(search) => {
                self.search = search.chars().take(256).collect();
                Task::none()
            }
            ExtMessage::Mods(on) | ExtMessage::Shaders(on) => {
                if self.busy {
                    return Task::none();
                }
                let mut next = match self.prefs.clone() {
                    Some(p) => p,
                    None => return Task::none(),
                };
                let id = next.active_profile.clone();
                let Some(a) = next.profiles.iter_mut().find(|p| p.id == id) else {
                    return Task::none();
                };
                if matches!(msg, ExtMessage::Mods(_)) {
                    a.flags.mods_enabled = on;
                } else {
                    a.flags.shaders_enabled = on;
                }
                self.commit(next);
                Task::none()
            }
            ExtMessage::ToggleMod(i, on) => {
                if self.busy {
                    return Task::none();
                }
                let pin = match self.catalog.get(i) {
                    Some(c) if PackageKind::of(c) == Some(PackageKind::Mod) => c.pin.clone(),
                    _ => return Task::none(),
                };
                let mut next = match self.prefs.clone() {
                    Some(p) => p,
                    None => return Task::none(),
                };
                let id = next.active_profile.clone();
                let Some(a) = next.profiles.iter_mut().find(|p| p.id == id) else {
                    return Task::none();
                };
                if on {
                    a.mods.retain(|m| m.id != pin.id);
                    if !a.mods.iter().any(|m| m.content_id == pin.content_id) {
                        a.mods.push(pin);
                    }
                } else {
                    a.mods.retain(|m| m.content_id != pin.content_id);
                }
                self.commit(next);
                Task::none()
            }
            ExtMessage::SelectShader(i) => {
                if self.busy {
                    return Task::none();
                }
                let pin = match self.catalog.get(i) {
                    Some(c) if PackageKind::of(c) == Some(PackageKind::Shaderpack) => c.pin.clone(),
                    _ => return Task::none(),
                };
                let mut next = match self.prefs.clone() {
                    Some(p) => p,
                    None => return Task::none(),
                };
                let id = next.active_profile.clone();
                let Some(a) = next.profiles.iter_mut().find(|p| p.id == id) else {
                    return Task::none();
                };
                a.shader = Some(ShaderSelection {
                    package: pin,
                    settings: serde_json::json!({}),
                });
                self.commit(next);
                Task::none()
            }
            ExtMessage::ClearShader => {
                if self.busy {
                    return Task::none();
                }
                let mut next = match self.prefs.clone() {
                    Some(p) => p,
                    None => return Task::none(),
                };
                let id = next.active_profile.clone();
                let Some(a) = next.profiles.iter_mut().find(|p| p.id == id) else {
                    return Task::none();
                };
                a.shader = None;
                self.commit(next);
                Task::none()
            }
        }
    }
    pub fn view(&self) -> Element<'_, ExtMessage> {
        view::management(self)
    }
}
