#[cfg(not(target_arch = "wasm32"))]
use crate::resources::manager::ClientResourceManager;
use el_core::resource_location::ResourceLocation;
#[cfg(not(target_arch = "wasm32"))]
use rodio::source::Source;
#[cfg(not(target_arch = "wasm32"))]
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, SpatialSink};
#[cfg(not(target_arch = "wasm32"))]
use std::io::Cursor;
#[path = "audio/output.rs"]
mod output;
pub use output::{AudioOutputCatalog, AudioOutputSelection};
#[cfg(not(target_arch = "wasm32"))]
#[path = "audio/binaural.rs"]
mod binaural;
#[cfg(not(target_arch = "wasm32"))]
pub struct AudioEngine {
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    active: std::collections::HashMap<SoundHandle, ActiveSound>,
    next_handle: u64,
    delayed: Vec<DelayedSound>,
    listener_pos: glam::Vec3,
    listener_right: glam::Vec3,
    directional_audio_enabled: bool,
    pub is_enabled: bool,
    #[cfg(not(target_os = "android"))]
    outputs: output::NativeOutputs,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SoundHandle(u64);
impl SoundHandle {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayResult {
    Started(SoundHandle),
    StartedSilently,
    NotStarted,
}
#[derive(Clone, Copy, Debug)]
pub struct SoundPlayback {
    pub source: crate::client::sound_engine::SoundSource,
    pub instance_volume: f32,
    pub category_gain: f32,
    pub pitch: f32,
    pub position: Option<glam::Vec3>,
    pub attenuation_max_distance: f32,
}
#[cfg(not(target_arch = "wasm32"))]
struct ActiveSound {
    sink: SinkKind,
    event: ResourceLocation,
    source: crate::client::sound_engine::SoundSource,
    instance_volume: f32,
    category_gain: f32,
    world_position: Option<glam::Vec3>,
    attenuation_max_distance: f32,
}
#[cfg(not(target_arch = "wasm32"))]
enum SinkKind {
    Binaural(Sink, binaural::BinauralControl),
    Spatial(SpatialSink),
    Flat(Sink),
}
#[cfg(not(target_arch = "wasm32"))]
impl SinkKind {
    fn volume(&self) -> f32 {
        match self {
            Self::Binaural(sink, _) | Self::Flat(sink) => sink.volume(),
            Self::Spatial(sink) => sink.volume(),
        }
    }
    fn empty(&self) -> bool {
        match self {
            Self::Binaural(sink, _) | Self::Flat(sink) => sink.empty(),
            Self::Spatial(sink) => sink.empty(),
        }
    }
    fn stop(&self) {
        match self {
            Self::Binaural(sink, _) | Self::Flat(sink) => sink.stop(),
            Self::Spatial(sink) => sink.stop(),
        }
    }
    fn set_volume(&self, volume: f32) {
        match self {
            Self::Binaural(sink, _) | Self::Flat(sink) => sink.set_volume(volume),
            Self::Spatial(sink) => sink.set_volume(volume),
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
struct DelayedSound {
    remaining_ticks: u32,
    event: ResourceLocation,
    playback: SoundPlayback,
}
#[cfg(not(target_arch = "wasm32"))]
const EAR_OFFSET: f32 = 0.1;
#[cfg(not(target_arch = "wasm32"))]
const SPATIAL_PAN_RADIUS: f32 = 0.5;
#[cfg(not(target_arch = "wasm32"))]
impl AudioEngine {
    pub fn new() -> Self {
        log::info!("Initializing AudioEngine...");
        #[cfg(not(target_os = "android"))]
        {
            let mut engine = Self::silent();
            match engine.reload_output_preference("") {
                Ok(result) if result.running => {
                    log::info!("Audio output stream initialized successfully.")
                }
                Ok(result) => {
                    log::warn!(
                        "Audio output unavailable: {}",
                        result.error.as_deref().unwrap_or("no output device")
                    )
                }
                Err(error) => log::warn!("Audio output initialization failed: {error}"),
            }
            return engine;
        }
        #[cfg(target_os = "android")]
        match OutputStream::try_default() {
            Ok((stream, stream_handle)) => {
                log::info!("Audio output stream initialized successfully.");
                Self {
                    _stream: Some(stream),
                    stream_handle: Some(stream_handle),
                    active: std::collections::HashMap::new(),
                    next_handle: 0,
                    delayed: Vec::new(),
                    listener_pos: glam::Vec3::ZERO,
                    listener_right: glam::Vec3::new(1.0, 0.0, 0.0),
                    directional_audio_enabled: false,
                    is_enabled: true,
                }
            }
            Err(err) => {
                log::warn!(
                    "Failed to initialize audio output stream: {:?}. Running in silent fallback mode.",
                    err
                );
                Self::silent()
            }
        }
    }
    fn silent() -> Self {
        Self {
            _stream: None,
            stream_handle: None,
            active: std::collections::HashMap::new(),
            next_handle: 0,
            delayed: Vec::new(),
            listener_pos: glam::Vec3::ZERO,
            listener_right: glam::Vec3::new(1.0, 0.0, 0.0),
            directional_audio_enabled: false,
            is_enabled: false,
            #[cfg(not(target_os = "android"))]
            outputs: output::NativeOutputs::new(),
        }
    }
    pub fn tick(&mut self) {
        if !self.is_enabled {
            return;
        }
        self.active.retain(|_, sound| !sound.sink.empty());
    }
    pub const fn supports_directional_audio() -> bool {
        true
    }
    pub fn set_directional_audio_enabled(&mut self, enabled: bool) {
        if self.directional_audio_enabled == enabled {
            return;
        }
        self.stop_all();
        self.directional_audio_enabled = enabled;
    }
    pub const fn directional_audio_enabled(&self) -> bool {
        self.directional_audio_enabled
    }
    pub fn update_listener(&mut self, pos: glam::Vec3, yaw: f32) {
        if !self.is_enabled {
            return;
        }
        self.listener_pos = pos;
        let cos_y = yaw.cos();
        let sin_y = yaw.sin();
        self.listener_right = glam::Vec3::new(-cos_y, 0.0, -sin_y).normalize_or_zero();
        for sound in self.active.values() {
            match &sound.sink {
                SinkKind::Binaural(_, control) => {
                    if let Some(world_position) = sound.world_position {
                        control.set_direction(Self::listener_relative_direction(
                            self.listener_right,
                            self.listener_pos,
                            world_position,
                        ));
                    }
                }
                SinkKind::Spatial(sink) => {
                    let (left_ear, right_ear) = Self::ear_offsets(self.listener_right);
                    sink.set_left_ear_position(left_ear.into());
                    sink.set_right_ear_position(right_ear.into());
                    if let Some(world_position) = sound.world_position {
                        sink.set_emitter_position(
                            Self::pan_emitter_offset(
                                self.listener_right,
                                self.listener_pos,
                                world_position,
                            )
                            .into(),
                        );
                    }
                }
                SinkKind::Flat(_) => {}
            }
            sound
                .sink
                .set_volume(Self::attenuated_gain(self.listener_pos, sound).max(0.0));
        }
    }
    fn listener_relative_direction(
        listener_right: glam::Vec3,
        listener_pos: glam::Vec3,
        world_position: glam::Vec3,
    ) -> glam::Vec3 {
        let delta = world_position - listener_pos;
        let forward = glam::Vec3::new(listener_right.z, 0.0, -listener_right.x);
        glam::Vec3::new(delta.dot(listener_right), delta.y, delta.dot(forward)).normalize_or_zero()
    }
    fn pan_emitter_offset(
        listener_right: glam::Vec3,
        listener_pos: glam::Vec3,
        world_position: glam::Vec3,
    ) -> glam::Vec3 {
        let direction = (world_position - listener_pos).normalize_or_zero();
        if direction == glam::Vec3::ZERO {
            listener_right * SPATIAL_PAN_RADIUS
        } else {
            direction * SPATIAL_PAN_RADIUS
        }
    }
    fn ear_offsets(listener_right: glam::Vec3) -> (glam::Vec3, glam::Vec3) {
        (-listener_right * EAR_OFFSET, listener_right * EAR_OFFSET)
    }
    fn attenuated_gain(listener_pos: glam::Vec3, sound: &ActiveSound) -> f32 {
        match sound.world_position {
            Some(world_position) => {
                let distance = world_position.distance(listener_pos);
                sound.category_gain
                    * crate::client::sound_engine::linear_attenuation_gain(
                        distance,
                        sound.attenuation_max_distance,
                    )
            }
            None => sound.category_gain,
        }
    }
    pub fn play_sound(
        &mut self,
        resource_manager: &ClientResourceManager,
        sound_file: &ResourceLocation,
        playback: SoundPlayback,
    ) -> PlayResult {
        if !self.is_enabled {
            return PlayResult::NotStarted;
        }
        if playback.category_gain <= 0.0 {
            return PlayResult::StartedSilently;
        }
        let handle = match &self.stream_handle {
            Some(h) => h.clone(),
            None => return PlayResult::NotStarted,
        };
        let loaded_source = resource_manager.read_resource_bytes(sound_file).ok();
        if let Some(bytes) = loaded_source {
            let cursor = Cursor::new(bytes);
            if let Ok(decoded) = Decoder::new(cursor) {
                let speed_source = decoded.convert_samples::<f32>().speed(playback.pitch);
                if let Some(sink) = self.build_sink(&handle, speed_source, playback.position) {
                    self.next_handle += 1;
                    let sound_handle = SoundHandle(self.next_handle);
                    let sound = ActiveSound {
                        sink,
                        event: sound_file.clone(),
                        source: playback.source,
                        instance_volume: playback.instance_volume,
                        category_gain: playback.category_gain,
                        world_position: playback.position,
                        attenuation_max_distance: playback.attenuation_max_distance,
                    };
                    sound
                        .sink
                        .set_volume(Self::attenuated_gain(self.listener_pos, &sound).max(0.0));
                    self.active.insert(sound_handle, sound);
                    return PlayResult::Started(sound_handle);
                }
                return PlayResult::NotStarted;
            }
        }
        log::debug!("Sound not found, ignoring: {}", sound_file);
        PlayResult::NotStarted
    }
    pub fn play_delayed(
        &mut self,
        sound_file: &ResourceLocation,
        playback: SoundPlayback,
        delay_ticks: u32,
    ) {
        if !self.is_enabled {
            return;
        }
        self.delayed.push(DelayedSound {
            remaining_ticks: delay_ticks,
            event: sound_file.clone(),
            playback,
        });
    }
    pub fn tick_delayed(&mut self, resource_manager: &ClientResourceManager) {
        if !self.is_enabled || self.delayed.is_empty() {
            return;
        }
        let mut due = Vec::new();
        self.delayed.retain_mut(|queued| {
            if queued.remaining_ticks == 0 {
                due.push((queued.event.clone(), queued.playback));
                false
            } else {
                queued.remaining_ticks -= 1;
                true
            }
        });
        for (event, playback) in due {
            self.play_sound(resource_manager, &event, playback);
        }
    }
    pub fn is_active(&self, handle: SoundHandle) -> bool {
        self.active
            .get(&handle)
            .is_some_and(|sound| !sound.sink.empty())
    }
    pub fn playback_gain_and_listener_distance(
        &self,
        handle: SoundHandle,
    ) -> Option<(f32, Option<f32>)> {
        let sound = self.active.get(&handle)?;
        Some((
            sound.sink.volume(),
            sound.world_position.map(|p| p.distance(self.listener_pos)),
        ))
    }
    pub fn stop(&mut self, handle: SoundHandle) {
        if let Some(sound) = self.active.remove(&handle) {
            sound.sink.stop();
        }
    }
    pub fn stop_by(
        &mut self,
        event: Option<&ResourceLocation>,
        source: Option<crate::client::sound_engine::SoundSource>,
    ) {
        let mut stopped = Vec::new();
        for (handle, sound) in &self.active {
            let source_matches = source.is_none_or(|wanted| sound.source == wanted);
            let event_matches = event.is_none_or(|wanted| &sound.event == wanted);
            if source_matches && event_matches {
                stopped.push(*handle);
            }
        }
        for handle in stopped {
            self.stop(handle);
        }
    }
    pub fn stop_all(&mut self) {
        for (_, sound) in self.active.drain() {
            sound.sink.stop();
        }
        self.delayed.clear();
    }
    pub fn refresh_category_volume(
        &mut self,
        source: crate::client::sound_engine::SoundSource,
        volumes: &crate::client::sound_engine::SoundSourceVolumes,
        gains: &crate::client::sound_engine::SourceGains,
    ) {
        use crate::client::sound_engine::{SoundSource, calculate_volume};
        for sound in self.active.values_mut() {
            if source == sound.source || source == SoundSource::Master {
                sound.category_gain =
                    calculate_volume(sound.instance_volume, sound.source, volumes, gains);
                sound
                    .sink
                    .set_volume(Self::attenuated_gain(self.listener_pos, sound).max(0.0));
            }
        }
    }
    pub fn set_emitter_position(&mut self, handle: SoundHandle, position: glam::Vec3) {
        let listener_pos = self.listener_pos;
        let listener_right = self.listener_right;
        if let Some(sound) = self.active.get_mut(&handle) {
            if sound.world_position.is_none() {
                return;
            }
            sound.world_position = Some(position);
            match &sound.sink {
                SinkKind::Binaural(_, control) => control.set_direction(
                    Self::listener_relative_direction(listener_right, listener_pos, position),
                ),
                SinkKind::Spatial(sink) => sink.set_emitter_position(
                    Self::pan_emitter_offset(listener_right, listener_pos, position).into(),
                ),
                SinkKind::Flat(_) => {}
            }
            sound
                .sink
                .set_volume(Self::attenuated_gain(listener_pos, sound).max(0.0));
        }
    }
    pub fn set_sound_volume(&mut self, handle: SoundHandle, volume: f32) {
        if let Some(sound) = self.active.get(&handle) {
            sound.sink.set_volume(volume.max(0.0));
        }
    }
    fn build_sink<S>(
        &self,
        handle: &OutputStreamHandle,
        source: S,
        pos: Option<glam::Vec3>,
    ) -> Option<SinkKind>
    where
        S: Source<Item = f32> + Send + 'static,
    {
        if let Some(world_position) = pos {
            if self.directional_audio_enabled {
                let sink = Sink::try_new(handle).ok()?;
                let direction = Self::listener_relative_direction(
                    self.listener_right,
                    self.listener_pos,
                    world_position,
                );
                let (source, control) = binaural::BinauralSource::new(source, direction);
                sink.append(source);
                sink.play();
                Some(SinkKind::Binaural(sink, control))
            } else {
                let (left_ear, right_ear) = Self::ear_offsets(self.listener_right);
                let emitter = Self::pan_emitter_offset(
                    self.listener_right,
                    self.listener_pos,
                    world_position,
                );
                let sink =
                    SpatialSink::try_new(handle, emitter.into(), left_ear.into(), right_ear.into())
                        .ok()?;
                sink.append(source);
                sink.play();
                Some(SinkKind::Spatial(sink))
            }
        } else {
            let sink = Sink::try_new(handle).ok()?;
            sink.append(source);
            sink.play();
            Some(SinkKind::Flat(sink))
        }
    }
}
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(module = "/src/client/audio_web.js")]
extern "C" {
    fn play_sound_bytes_js(
        bytes: &[u8],
        volume: f32,
        pitch: f32,
        id: f64,
        positional: bool,
        x: f32,
        y: f32,
        z: f32,
    ) -> bool;
    fn update_listener_js(x: f32, y: f32, z: f32, yaw: f32);
    fn set_sound_position_js(id: f64, x: f32, y: f32, z: f32);
    fn set_sound_volume_js(id: f64, volume: f32);
    fn stop_sound_js(id: f64);
    fn is_sound_active_js(id: f64) -> bool;
    fn stop_all_sounds_js();
    fn set_directional_audio_js(enabled: bool);
}
#[cfg(target_arch = "wasm32")]
pub struct AudioEngine {
    pub is_enabled: bool,
    directional_audio_enabled: bool,
    next_handle: u64,
    active: std::collections::HashMap<SoundHandle, ActiveWebSound>,
    listener_pos: glam::Vec3,
}
#[cfg(target_arch = "wasm32")]
struct ActiveWebSound {
    event: ResourceLocation,
    source: crate::client::sound_engine::SoundSource,
    instance_volume: f32,
    category_gain: f32,
    world_position: Option<glam::Vec3>,
    attenuation_max_distance: f32,
}
#[cfg(target_arch = "wasm32")]
impl AudioEngine {
    pub fn new() -> Self {
        Self {
            is_enabled: true,
            directional_audio_enabled: false,
            next_handle: 0,
            active: std::collections::HashMap::new(),
            listener_pos: glam::Vec3::ZERO,
        }
    }
    pub const fn supports_directional_audio() -> bool {
        true
    }
    pub fn set_directional_audio_enabled(&mut self, enabled: bool) {
        if self.directional_audio_enabled == enabled {
            return;
        }
        self.stop_all();
        self.directional_audio_enabled = enabled;
        set_directional_audio_js(enabled);
    }
    pub const fn directional_audio_enabled(&self) -> bool {
        self.directional_audio_enabled
    }
    pub fn tick(&mut self) {
        self.active
            .retain(|handle, _| is_sound_active_js(handle.0 as f64));
    }
    pub fn update_listener(&mut self, pos: glam::Vec3, yaw: f32) {
        if !self.is_enabled {
            return;
        }
        self.listener_pos = pos;
        update_listener_js(pos.x, pos.y, pos.z, yaw);
        for (handle, sound) in &self.active {
            set_sound_volume_js(
                handle.0 as f64,
                Self::attenuated_gain(self.listener_pos, sound),
            );
        }
    }
    fn attenuated_gain(listener_pos: glam::Vec3, sound: &ActiveWebSound) -> f32 {
        let distance_gain = sound.world_position.map_or(1.0, |position| {
            crate::client::sound_engine::linear_attenuation_gain(
                position.distance(listener_pos),
                sound.attenuation_max_distance,
            )
        });
        (sound.category_gain * distance_gain).max(0.0)
    }
    pub fn play_sound(
        &mut self,
        resource_manager: &crate::resources::manager::ClientResourceManager,
        sound_file: &ResourceLocation,
        playback: SoundPlayback,
    ) -> PlayResult {
        if !self.is_enabled {
            return PlayResult::NotStarted;
        }
        if playback.category_gain <= 0.0 {
            return PlayResult::StartedSilently;
        }
        match resource_manager.read_resource_bytes(sound_file) {
            Ok(bytes) => {
                log::info!(
                    "[Audio] Playing sound: {} ({} bytes, gain={:.2})",
                    sound_file,
                    bytes.len(),
                    playback.category_gain
                );
                if self.next_handle >= 9_007_199_254_740_991 {
                    return PlayResult::NotStarted;
                }
                self.next_handle += 1;
                let sound = ActiveWebSound {
                    event: sound_file.clone(),
                    source: playback.source,
                    instance_volume: playback.instance_volume,
                    category_gain: playback.category_gain,
                    world_position: playback.position,
                    attenuation_max_distance: playback.attenuation_max_distance,
                };
                let position = playback.position.unwrap_or(glam::Vec3::ZERO);
                if play_sound_bytes_js(
                    &bytes,
                    Self::attenuated_gain(self.listener_pos, &sound),
                    playback.pitch,
                    self.next_handle as f64,
                    playback.position.is_some(),
                    position.x,
                    position.y,
                    position.z,
                ) {
                    let handle = SoundHandle(self.next_handle);
                    self.active.insert(handle, sound);
                    PlayResult::Started(handle)
                } else {
                    PlayResult::NotStarted
                }
            }
            Err(e) => {
                if sound_file.path.contains("music/") || sound_file.path.contains("records/") {
                    log::debug!(
                        "[Audio] Optional music track omitted in web bundle: {}",
                        sound_file
                    );
                } else {
                    log::warn!("[Audio] Sound file not found: {} -> {:?}", sound_file, e);
                }
                PlayResult::NotStarted
            }
        }
    }
    pub fn play_delayed(
        &mut self,
        _sound_file: &ResourceLocation,
        _playback: SoundPlayback,
        _delay_ticks: u32,
    ) {
    }
    pub fn tick_delayed(
        &mut self,
        _resource_manager: &crate::resources::manager::ClientResourceManager,
    ) {
    }
    pub fn is_active(&self, handle: SoundHandle) -> bool {
        self.active.contains_key(&handle) && is_sound_active_js(handle.0 as f64)
    }
    pub fn playback_gain_and_listener_distance(
        &self,
        handle: SoundHandle,
    ) -> Option<(f32, Option<f32>)> {
        let sound = self.active.get(&handle)?;
        Some((
            Self::attenuated_gain(self.listener_pos, sound),
            sound.world_position.map(|p| p.distance(self.listener_pos)),
        ))
    }
    pub fn stop(&mut self, handle: SoundHandle) {
        self.active.remove(&handle);
        stop_sound_js(handle.0 as f64);
    }
    pub fn stop_by(
        &mut self,
        event: Option<&ResourceLocation>,
        source: Option<crate::client::sound_engine::SoundSource>,
    ) {
        let handles: Vec<_> = self
            .active
            .iter()
            .filter(|(_, sound)| {
                event.is_none_or(|wanted| &sound.event == wanted)
                    && source.is_none_or(|wanted| sound.source == wanted)
            })
            .map(|(handle, _)| *handle)
            .collect();
        for handle in handles {
            self.stop(handle);
        }
    }
    pub fn stop_all(&mut self) {
        self.active.clear();
        stop_all_sounds_js();
    }
    pub fn refresh_category_volume(
        &mut self,
        source: crate::client::sound_engine::SoundSource,
        volumes: &crate::client::sound_engine::SoundSourceVolumes,
        gains: &crate::client::sound_engine::SourceGains,
    ) {
        use crate::client::sound_engine::{SoundSource, calculate_volume};
        for (handle, sound) in &mut self.active {
            if source == sound.source || source == SoundSource::Master {
                sound.category_gain =
                    calculate_volume(sound.instance_volume, sound.source, volumes, gains);
                set_sound_volume_js(
                    handle.0 as f64,
                    Self::attenuated_gain(self.listener_pos, sound),
                );
            }
        }
    }
    pub fn set_emitter_position(&mut self, handle: SoundHandle, position: glam::Vec3) {
        if let Some(sound) = self.active.get_mut(&handle) {
            if sound.world_position.is_none() {
                return;
            }
            sound.world_position = Some(position);
            set_sound_position_js(handle.0 as f64, position.x, position.y, position.z);
            set_sound_volume_js(
                handle.0 as f64,
                Self::attenuated_gain(self.listener_pos, sound),
            );
        }
    }
    pub fn set_sound_volume(&mut self, handle: SoundHandle, volume: f32) {
        if self.active.contains_key(&handle) {
            set_sound_volume_js(handle.0 as f64, volume.max(0.0));
        }
    }
}
