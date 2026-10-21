use super::AudioEngine;
use std::fmt;
#[cfg(all(
    feature = "native",
    not(any(target_arch = "wasm32", target_os = "android"))
))]
pub const MAX_OUTPUT_DEVICES: usize = 128;
#[cfg(all(
    feature = "native",
    not(any(target_arch = "wasm32", target_os = "android"))
))]
pub const MAX_OUTPUT_NAME_BYTES: usize = 512;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioOutputDevice {
    pub handle: String,
    pub name: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioOutputCatalog {
    pub supported: bool,
    pub revision: u64,
    pub devices: Vec<AudioOutputDevice>,
    pub preferred: String,
    pub actual: Option<String>,
    pub truncated: bool,
    pub error: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioOutputSelection {
    Default,
    Device(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioOutputSwitch {
    pub preferred: String,
    pub actual: Option<String>,
    pub fallback: bool,
    pub running: bool,
    pub error: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioOutputError {
    Unsupported,
    StaleCatalog,
    UnknownHandle,
    Catalog(String),
    InvalidPreference,
    IdentityExhausted,
}
impl fmt::Display for AudioOutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => {
                formatter.write_str("output selection is unsupported on this backend")
            }
            Self::StaleCatalog => {
                formatter.write_str("audio output catalog changed; refresh the selection")
            }
            Self::UnknownHandle => {
                formatter.write_str("audio output handle is not in this engine's catalog")
            }
            Self::Catalog(error) => write!(formatter, "audio output catalog: {error}"),
            Self::InvalidPreference => formatter.write_str("invalid audio output preference"),
            Self::IdentityExhausted => {
                formatter.write_str("audio output catalog identity exhausted")
            }
        }
    }
}
impl std::error::Error for AudioOutputError {}
#[cfg(any(target_arch = "wasm32", target_os = "android"))]
impl AudioEngine {
    pub fn output_catalog(&mut self) -> AudioOutputCatalog {
        AudioOutputCatalog {
            supported: false,
            revision: 0,
            devices: Vec::new(),
            preferred: String::new(),
            actual: None,
            truncated: false,
            error: Some(AudioOutputError::Unsupported.to_string()),
        }
    }
    pub fn select_output(
        &mut self,
        _revision: u64,
        _selection: AudioOutputSelection,
    ) -> Result<AudioOutputSwitch, AudioOutputError> {
        Err(AudioOutputError::Unsupported)
    }
    pub fn reload_output_preference(
        &mut self,
        _preferred: &str,
    ) -> Result<AudioOutputSwitch, AudioOutputError> {
        Err(AudioOutputError::Unsupported)
    }
}
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
mod native {
    use super::*;
    use rodio::cpal::traits::{DeviceTrait, HostTrait};
    use rodio::{OutputStream, cpal};
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);
    #[derive(Clone)]
    struct CachedDevice {
        token: String,
        name: String,
        #[cfg(not(target_os = "linux"))]
        device: cpal::Device,
    }
    impl CachedDevice {
        fn reopen(self) -> Result<cpal::Device, String> {
            #[cfg(target_os = "linux")]
            {
                cpal::default_host()
                    .devices()
                    .map_err(|error| error.to_string())?
                    .find(|device| device.name().is_ok_and(|name| name == self.name))
                    .ok_or_else(|| "selected audio output is no longer available".to_owned())
            }
            #[cfg(not(target_os = "linux"))]
            {
                Ok(self.device)
            }
        }
    }
    enum OutputCandidate {
        SystemDefault,
        Catalog(CachedDevice),
    }
    fn open_output_candidate(
        choice: OutputCandidate,
    ) -> Result<(OutputStream, rodio::OutputStreamHandle, Option<String>), String> {
        let device = match choice {
            OutputCandidate::SystemDefault => cpal::default_host()
                .default_output_device()
                .ok_or_else(|| "no default output device available".to_owned())?,
            OutputCandidate::Catalog(device) => device.reopen()?,
        };
        let actual = device
            .name()
            .ok()
            .filter(|name| valid_name(name) && !name.is_empty());
        OutputStream::try_from_device(&device)
            .map(|(stream, handle)| (stream, handle, actual))
            .map_err(|error| error.to_string())
    }
    pub(in crate::client::audio) struct NativeOutputs {
        owner: Option<u64>,
        revision: u64,
        devices: Vec<CachedDevice>,
        preferred: String,
        actual: Option<String>,
        truncated: bool,
    }
    impl NativeOutputs {
        pub(in crate::client::audio) fn new() -> Self {
            Self {
                owner: NEXT_OWNER
                    .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                        value.checked_add(1)
                    })
                    .ok(),
                revision: 0,
                devices: Vec::new(),
                preferred: String::new(),
                actual: None,
                truncated: false,
            }
        }
        fn advance(&mut self) -> Result<(), AudioOutputError> {
            self.owner.ok_or(AudioOutputError::IdentityExhausted)?;
            self.revision = self
                .revision
                .checked_add(1)
                .ok_or(AudioOutputError::IdentityExhausted)?;
            self.devices.clear();
            self.truncated = false;
            Ok(())
        }
        fn refresh(&mut self) -> Result<(), AudioOutputError> {
            self.advance()?;
            #[cfg(target_os = "linux")]
            {
                let hints = alsa::device_name::HintIter::new_str(None, "pcm").map_err(|error| {
                    AudioOutputError::Catalog(error.to_string().chars().take(512).collect())
                })?;
                for (index, hint) in hints.enumerate() {
                    if hint.direction == Some(alsa::Direction::Capture) {
                        continue;
                    }
                    let Some(name) = hint.name else { continue };
                    if name == "null" {
                        continue;
                    }
                    if !valid_name(&name) || name.is_empty() {
                        self.truncated = true;
                        continue;
                    }
                    if self.devices.len() == MAX_OUTPUT_DEVICES {
                        self.truncated = true;
                        break;
                    }
                    self.devices.push(CachedDevice {
                        token: format!(
                            "audio-output/{}/{}/{}",
                            self.owner.unwrap(),
                            self.revision,
                            index
                        ),
                        name,
                    });
                }
                Ok(())
            }
            #[cfg(not(target_os = "linux"))]
            {
                let devices = cpal::default_host().devices().map_err(|error| {
                    AudioOutputError::Catalog(error.to_string().chars().take(512).collect())
                })?;
                for (index, device) in devices.enumerate().take(MAX_OUTPUT_DEVICES + 1) {
                    if index == MAX_OUTPUT_DEVICES {
                        self.truncated = true;
                        break;
                    }
                    if !device
                        .supported_output_configs()
                        .is_ok_and(|mut configs| configs.next().is_some())
                    {
                        continue;
                    }
                    let Ok(name) = device.name() else {
                        continue;
                    };
                    if !valid_name(&name) || name.is_empty() {
                        self.truncated = true;
                        continue;
                    }
                    self.devices.push(CachedDevice {
                        token: format!(
                            "audio-output/{}/{}/{}",
                            self.owner.unwrap(),
                            self.revision,
                            index
                        ),
                        name,
                        #[cfg(not(target_os = "linux"))]
                        device,
                    });
                }
                Ok(())
            }
        }
        fn snapshot(&self, error: Option<String>) -> AudioOutputCatalog {
            AudioOutputCatalog {
                supported: self.owner.is_some(),
                revision: self.revision,
                devices: self
                    .devices
                    .iter()
                    .map(|device| AudioOutputDevice {
                        handle: device.token.clone(),
                        name: device.name.clone(),
                    })
                    .collect(),
                preferred: self.preferred.clone(),
                actual: self.actual.clone(),
                truncated: self.truncated,
                error,
            }
        }
        fn resolve(
            &self,
            revision: u64,
            selection: AudioOutputSelection,
        ) -> Result<Option<CachedDevice>, AudioOutputError> {
            if revision == 0 || revision != self.revision {
                return Err(AudioOutputError::StaleCatalog);
            }
            match selection {
                AudioOutputSelection::Default => Ok(None),
                AudioOutputSelection::Device(token) => self
                    .devices
                    .iter()
                    .find(|device| device.token == token)
                    .cloned()
                    .map(Some)
                    .ok_or(AudioOutputError::UnknownHandle),
            }
        }
    }
    fn valid_name(name: &str) -> bool {
        name.len() <= MAX_OUTPUT_NAME_BYTES && !name.chars().any(char::is_control)
    }
    struct Candidate<D> {
        device: D,
        name: Option<String>,
        preferred: bool,
    }
    struct Opened<S> {
        stream: Option<S>,
        actual: Option<String>,
        fallback: bool,
        error: Option<String>,
    }
    fn open_first<D, S>(
        candidates: impl IntoIterator<Item = Candidate<D>>,
        mut open: impl FnMut(D) -> Result<S, String>,
    ) -> Opened<S> {
        let mut last_error = "no output device available".to_owned();
        for candidate in candidates {
            match open(candidate.device) {
                Ok(stream) => {
                    return Opened {
                        stream: Some(stream),
                        actual: candidate.name,
                        fallback: !candidate.preferred,
                        error: None,
                    };
                }
                Err(error) => last_error = error.chars().take(512).collect(),
            }
        }
        Opened {
            stream: None,
            actual: None,
            fallback: false,
            error: Some(last_error),
        }
    }
    impl AudioEngine {
        pub fn output_catalog(&mut self) -> AudioOutputCatalog {
            let error = self.outputs.refresh().err().map(|error| error.to_string());
            self.outputs.snapshot(error)
        }
        pub fn select_output(
            &mut self,
            revision: u64,
            selection: AudioOutputSelection,
        ) -> Result<AudioOutputSwitch, AudioOutputError> {
            let selected = self.outputs.resolve(revision, selection)?;
            let preferred = selected
                .as_ref()
                .map_or(String::new(), |device| device.name.clone());
            Ok(self.reload_output(selected, preferred))
        }
        pub fn reload_output_preference(
            &mut self,
            preferred: &str,
        ) -> Result<AudioOutputSwitch, AudioOutputError> {
            if !valid_name(preferred) {
                return Err(AudioOutputError::InvalidPreference);
            }
            if !preferred.is_empty()
                && let Err(error) = self.outputs.refresh()
            {
                if !matches!(error, AudioOutputError::Catalog(_)) {
                    return Err(error);
                }
            }
            let selected = self
                .outputs
                .devices
                .iter()
                .find(|device| device.name == preferred && !preferred.is_empty())
                .cloned();
            Ok(self.reload_output(selected, preferred.to_owned()))
        }
        fn begin_output_reload(&mut self, preferred: String) {
            self.stop_all();
            self.stream_handle = None;
            self._stream = None;
            self.is_enabled = false;
            self.outputs.preferred = preferred;
            self.outputs.actual = None;
        }
        fn reload_output(
            &mut self,
            selected: Option<CachedDevice>,
            preferred: String,
        ) -> AudioOutputSwitch {
            let mut candidates = Vec::with_capacity(self.outputs.devices.len() + 2);
            if let Some(device) = selected {
                candidates.push(Candidate {
                    name: Some(device.name.clone()),
                    device: OutputCandidate::Catalog(device),
                    preferred: true,
                });
            }
            candidates.push(Candidate {
                device: OutputCandidate::SystemDefault,
                name: None,
                preferred: preferred.is_empty(),
            });
            candidates.extend(
                self.outputs
                    .devices
                    .iter()
                    .cloned()
                    .map(|device| Candidate {
                        name: Some(device.name.clone()),
                        device: OutputCandidate::Catalog(device),
                        preferred: false,
                    }),
            );
            self.begin_output_reload(preferred.clone());
            let mut opened = open_first(candidates, open_output_candidate);
            if opened.stream.is_none() && self.outputs.devices.is_empty() {
                if let Err(err) = self.outputs.refresh() {
                    log::warn!("Audio output device refresh failed: {err}");
                }
                opened = open_first(
                    self.outputs
                        .devices
                        .iter()
                        .cloned()
                        .map(|device| Candidate {
                            name: Some(device.name.clone()),
                            device: OutputCandidate::Catalog(device),
                            preferred: false,
                        }),
                    open_output_candidate,
                );
            }
            if let Some((stream, handle, actual)) = opened.stream {
                opened.actual = actual;
                self._stream = Some(stream);
                self.stream_handle = Some(handle);
                self.listener_pos = glam::Vec3::ZERO;
                self.listener_right = glam::Vec3::X;
                self.is_enabled = true;
            }
            self.outputs.actual = opened.actual.clone();
            AudioOutputSwitch {
                preferred,
                actual: opened.actual,
                fallback: opened.fallback,
                running: self.is_enabled,
                error: opened.error,
            }
        }
    }
}
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
pub(in crate::client::audio) use native::NativeOutputs;
