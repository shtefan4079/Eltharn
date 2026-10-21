use std::collections::HashMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoundSource {
    Master,
    Music,
    Records,
    Weather,
    Blocks,
    Hostile,
    Neutral,
    Players,
    Ambient,
    Voice,
    Ui,
}
impl SoundSource {
    pub const VALUES: [Self; 11] = [
        Self::Master,
        Self::Music,
        Self::Records,
        Self::Weather,
        Self::Blocks,
        Self::Hostile,
        Self::Neutral,
        Self::Players,
        Self::Ambient,
        Self::Voice,
        Self::Ui,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Master => "master",
            Self::Music => "music",
            Self::Records => "record",
            Self::Weather => "weather",
            Self::Blocks => "block",
            Self::Hostile => "hostile",
            Self::Neutral => "neutral",
            Self::Players => "player",
            Self::Ambient => "ambient",
            Self::Voice => "voice",
            Self::Ui => "ui",
        }
    }
    pub fn from_name(name: &str) -> Option<Self> {
        Self::VALUES
            .into_iter()
            .find(|source| source.name() == name)
    }
    pub fn options_key(self) -> String {
        format!("soundCategory_{}", self.name())
    }
}
#[derive(Clone, Debug)]
pub struct SoundSourceVolumes {
    volumes: HashMap<SoundSource, f32>,
}
impl Default for SoundSourceVolumes {
    fn default() -> Self {
        Self {
            volumes: SoundSource::VALUES
                .into_iter()
                .map(|source| (source, 1.0))
                .collect(),
        }
    }
}
impl SoundSourceVolumes {
    pub fn from_options(extra_options: &HashMap<String, String>) -> Self {
        let mut volumes = Self::default();
        for source in SoundSource::VALUES {
            if let Some(raw) = extra_options.get(&source.options_key()) {
                if let Some(value) = raw
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
                {
                    volumes.volumes.insert(source, value);
                }
            }
        }
        volumes
    }
    pub fn source_volume(&self, source: SoundSource) -> f32 {
        self.volumes.get(&source).copied().unwrap_or(1.0)
    }
    pub fn set_source_volume(&mut self, source: SoundSource, volume: f32) {
        self.volumes.insert(source, volume);
    }
    pub fn final_source_volume(&self, source: SoundSource) -> f32 {
        if source == SoundSource::Master {
            self.source_volume(source)
        } else {
            self.source_volume(source) * self.source_volume(SoundSource::Master)
        }
    }
}
#[derive(Clone, Debug)]
pub struct SourceGains {
    gains: HashMap<SoundSource, f32>,
}
impl Default for SourceGains {
    fn default() -> Self {
        Self {
            gains: SoundSource::VALUES
                .into_iter()
                .map(|source| (source, 1.0))
                .collect(),
        }
    }
}
impl SourceGains {
    pub fn gain(&self, source: SoundSource) -> f32 {
        self.gains.get(&source).copied().unwrap_or(1.0)
    }
    pub fn set_gain(&mut self, source: SoundSource, gain: f32) {
        self.gains.insert(source, gain.clamp(0.0, 1.0));
    }
}
pub fn calculate_volume(
    instance_volume: f32,
    source: SoundSource,
    volumes: &SoundSourceVolumes,
    gains: &SourceGains,
) -> f32 {
    instance_volume.clamp(0.0, 1.0)
        * volumes.final_source_volume(source).clamp(0.0, 1.0)
        * gains.gain(source)
}
pub fn calculate_pitch(instance_pitch: f32) -> f32 {
    instance_pitch.clamp(0.5, 2.0)
}
pub fn category_preview_sound(source: SoundSource) -> Option<&'static str> {
    match source {
        SoundSource::Records => Some("minecraft:block.note_block.guitar"),
        SoundSource::Weather => Some("minecraft:entity.lightning_bolt.thunder"),
        SoundSource::Blocks => Some("minecraft:block.grass.place"),
        SoundSource::Hostile => Some("minecraft:entity.zombie.ambient"),
        SoundSource::Neutral => Some("minecraft:entity.cow.ambient"),
        SoundSource::Players => Some("minecraft:entity.generic.eat"),
        SoundSource::Ambient => Some("minecraft:ambient.cave"),
        SoundSource::Ui => Some("minecraft:ui.button.click"),
        SoundSource::Master | SoundSource::Music | SoundSource::Voice => None,
    }
}
pub fn linear_attenuation_gain(distance: f32, max_distance: f32) -> f32 {
    if !(max_distance > 0.0) || !distance.is_finite() {
        return 1.0;
    }
    (1.0 - distance / max_distance).clamp(0.0, 1.0)
}
pub fn attenuation_distance(instance_volume: f32, sound_attenuation_distance: i32) -> f32 {
    instance_volume.max(1.0) * sound_attenuation_distance as f32
}
