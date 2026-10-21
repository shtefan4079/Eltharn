#![allow(dead_code)]
use el_core::resource_location::ResourceLocation;
use el_math::random::{LegacyRandomSource, RandomSource};
use std::collections::HashMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundType {
    File,
    Event,
}
impl SoundType {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "file" => Some(Self::File),
            "event" => Some(Self::Event),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Sound {
    pub location: ResourceLocation,
    pub volume: f32,
    pub pitch: f32,
    pub weight: u32,
    pub sound_type: SoundType,
    pub stream: bool,
    pub preload: bool,
    pub attenuation_distance: i32,
}
impl Sound {
    pub fn file_path(&self) -> ResourceLocation {
        ResourceLocation::new(
            &self.location.namespace,
            &format!("sounds/{}.ogg", self.location.path),
        )
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct SoundEventRegistration {
    pub sounds: Vec<Sound>,
    pub replace: bool,
    pub subtitle: Option<String>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WeighedSoundEvents {
    sounds: Vec<Sound>,
    subtitle: Option<String>,
}
impl WeighedSoundEvents {
    pub fn total_weight(&self) -> u32 {
        self.sounds.iter().map(|sound| sound.weight).sum()
    }
    pub fn choose(&self, random_value: u32) -> Option<&Sound> {
        let weight = self.total_weight();
        if self.sounds.is_empty() || weight == 0 {
            return None;
        }
        let mut index = (random_value % weight) as i64;
        for sound in &self.sounds {
            index -= i64::from(sound.weight);
            if index < 0 {
                return Some(sound);
            }
        }
        None
    }
    pub fn subtitle(&self) -> Option<&str> {
        self.subtitle.as_deref()
    }
    pub fn sounds(&self) -> &[Sound] {
        &self.sounds
    }
}
#[derive(Clone, Debug, Default)]
pub struct SoundRegistry {
    events: HashMap<ResourceLocation, WeighedSoundEvents>,
}
impl SoundRegistry {
    pub fn apply_namespace(
        &mut self,
        namespace: &str,
        registrations: HashMap<String, SoundEventRegistration>,
    ) {
        let mut keys = registrations.keys().cloned().collect::<Vec<_>>();
        keys.sort();
        for key in keys {
            let registration = &registrations[&key];
            let id = ResourceLocation::new(namespace, &key);
            let entry = self.events.entry(id).or_insert_with(|| WeighedSoundEvents {
                sounds: Vec::new(),
                subtitle: registration.subtitle.clone(),
            });
            if registration.replace {
                entry.sounds.clear();
                entry.subtitle = registration.subtitle.clone();
            }
            entry.sounds.extend(registration.sounds.iter().cloned());
        }
    }
    pub fn load(
        resource_manager: &crate::resources::manager::ClientResourceManager,
        namespaces: &[String],
    ) -> Self {
        let mut registry = Self::default();
        for namespace in namespaces {
            match resource_manager.read_sound_metadata_stack(namespace) {
                Ok(stack) => {
                    let count = stack.len();
                    for source in stack {
                        if let Some(registrations) = parse_sounds_json(&source) {
                            registry.apply_namespace(namespace, registrations);
                        }
                    }
                    log::info!(
                        "Loaded {count} stacked sounds.json document(s) for namespace {namespace}"
                    );
                }
                Err(error) => {
                    let id = ResourceLocation::new(namespace, "sounds");
                    if let Ok(source) = resource_manager.read_sound_metadata(&id) {
                        if let Some(registrations) = parse_sounds_json(&source) {
                            registry.apply_namespace(namespace, registrations);
                        }
                    } else {
                        log::debug!("no sounds.json for {namespace}: {error:?}");
                    }
                }
            }
        }
        log::info!("Sound registry holds {} events", registry.len());
        registry
    }
    pub fn get(&self, event_id: &ResourceLocation) -> Option<&WeighedSoundEvents> {
        self.events.get(event_id)
    }
    pub fn len(&self) -> usize {
        self.events.len()
    }
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
    pub fn resolve_file(
        &self,
        event_id: &ResourceLocation,
        random_value: u32,
        depth_limit: u32,
    ) -> Option<ResourceLocation> {
        let sound = self.get(event_id)?.choose(random_value)?;
        match sound.sound_type {
            SoundType::File => Some(sound.file_path()),
            SoundType::Event => {
                if depth_limit == 0 {
                    return None;
                }
                self.resolve_file(&sound.location, random_value, depth_limit - 1)
            }
        }
    }
    pub fn resolve_seeded_sound(
        &self,
        event_id: &ResourceLocation,
        seed: u64,
        depth_limit: u32,
    ) -> Option<Sound> {
        let mut random = LegacyRandomSource::new(seed as i64);
        self.resolve_sound_with_random(event_id, &mut random, depth_limit)
    }
    pub fn resolve_sound_with_random(
        &self,
        event_id: &ResourceLocation,
        random: &mut impl RandomSource,
        depth_limit: u32,
    ) -> Option<Sound> {
        let event = self.get(event_id)?;
        let weight = self.event_weight(event_id, depth_limit)?;
        if weight == 0 {
            return None;
        }
        let mut index = random.next_int_bound(i32::try_from(weight).ok()?);
        for sound in event.sounds() {
            let weight = match sound.sound_type {
                SoundType::File => sound.weight,
                SoundType::Event => {
                    if depth_limit == 0 {
                        0
                    } else {
                        self.event_weight(&sound.location, depth_limit - 1)
                            .unwrap_or(0)
                    }
                }
            };
            index -= i32::try_from(weight).ok()?;
            if index >= 0 {
                continue;
            }
            return match sound.sound_type {
                SoundType::File => Some(sound.clone()),
                SoundType::Event => {
                    let mut resolved = self.resolve_sound_with_random(
                        &sound.location,
                        random,
                        depth_limit.checked_sub(1)?,
                    )?;
                    resolved.volume *= sound.volume;
                    resolved.pitch *= sound.pitch;
                    resolved.stream |= sound.stream;
                    resolved.weight = sound.weight;
                    Some(resolved)
                }
            };
        }
        None
    }
    fn event_weight(&self, event_id: &ResourceLocation, depth_limit: u32) -> Option<u32> {
        let event = self.get(event_id)?;
        event.sounds().iter().try_fold(0_u32, |sum, sound| {
            let weight = match sound.sound_type {
                SoundType::File => sound.weight,
                SoundType::Event => {
                    if depth_limit == 0 {
                        0
                    } else {
                        self.event_weight(&sound.location, depth_limit - 1)
                            .unwrap_or(0)
                    }
                }
            };
            sum.checked_add(weight)
        })
    }
}
pub fn parse_sounds_json(source: &str) -> Option<HashMap<String, SoundEventRegistration>> {
    let value: serde_json::Value = serde_json::from_str(source).ok()?;
    let object = value.as_object()?;
    let mut registrations = HashMap::new();
    for (event, entry) in object {
        if let Some(registration) = parse_registration(entry) {
            registrations.insert(event.clone(), registration);
        }
    }
    Some(registrations)
}
fn parse_registration(entry: &serde_json::Value) -> Option<SoundEventRegistration> {
    let object = entry.as_object()?;
    let replace = object
        .get("replace")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let subtitle = object
        .get("subtitle")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let mut sounds = Vec::new();
    if let Some(array) = object.get("sounds").and_then(serde_json::Value::as_array) {
        for element in array {
            if let Some(sound) = parse_sound(element) {
                sounds.push(sound);
            }
        }
    }
    Some(SoundEventRegistration {
        sounds,
        replace,
        subtitle,
    })
}
fn parse_sound(element: &serde_json::Value) -> Option<Sound> {
    if let Some(name) = element.as_str() {
        return Some(Sound {
            location: parse_identifier(name)?,
            volume: 1.0,
            pitch: 1.0,
            weight: 1,
            sound_type: SoundType::File,
            stream: false,
            preload: false,
            attenuation_distance: 16,
        });
    }
    let object = element.as_object()?;
    let location = parse_identifier(object.get("name")?.as_str()?)?;
    let sound_type = match object.get("type").and_then(serde_json::Value::as_str) {
        Some(name) => SoundType::from_name(name)?,
        None => SoundType::File,
    };
    let volume = read_f32(object.get("volume"), 1.0);
    let pitch = read_f32(object.get("pitch"), 1.0);
    let weight = object
        .get("weight")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1);
    if !(volume > 0.0) || !(pitch > 0.0) || weight <= 0 {
        return None;
    }
    Some(Sound {
        location,
        volume,
        pitch,
        weight: weight as u32,
        sound_type,
        stream: object
            .get("stream")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        preload: object
            .get("preload")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        attenuation_distance: object
            .get("attenuation_distance")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(16) as i32,
    })
}
fn read_f32(value: Option<&serde_json::Value>, default: f32) -> f32 {
    value
        .and_then(serde_json::Value::as_f64)
        .map(|value| value as f32)
        .unwrap_or(default)
}
fn parse_identifier(value: &str) -> Option<ResourceLocation> {
    match value.split_once(':') {
        Some((namespace, path)) => (!namespace.is_empty() && !path.is_empty())
            .then(|| ResourceLocation::new(namespace, path)),
        None => (!value.is_empty()).then(|| ResourceLocation::new("minecraft", value)),
    }
}
