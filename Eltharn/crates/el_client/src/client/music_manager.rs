#![allow(dead_code)]
use el_core::resource_location::ResourceLocation;
pub fn music_sound_language_key(sound_file: &ResourceLocation) -> Option<String> {
    let path = sound_file
        .path
        .strip_prefix("sounds/")?
        .strip_suffix(".ogg")?;
    (!path.is_empty()).then(|| path.replace('/', "."))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Music {
    pub sound: &'static str,
    pub min_delay: i32,
    pub max_delay: i32,
    pub replace_current_music: bool,
}
impl Music {
    const fn game(sound: &'static str) -> Self {
        Self {
            sound,
            min_delay: 12_000,
            max_delay: 24_000,
            replace_current_music: false,
        }
    }
}
pub mod musics {
    use super::Music;
    pub const MENU: Music = Music {
        sound: "minecraft:music.menu",
        min_delay: 20,
        max_delay: 600,
        replace_current_music: true,
    };
    pub const CREATIVE: Music = Music {
        sound: "minecraft:music.creative",
        min_delay: 12_000,
        max_delay: 24_000,
        replace_current_music: false,
    };
    pub const CREDITS: Music = Music {
        sound: "minecraft:music.credits",
        min_delay: 0,
        max_delay: 0,
        replace_current_music: true,
    };
    pub const END_BOSS: Music = Music {
        sound: "minecraft:music.dragon",
        min_delay: 0,
        max_delay: 0,
        replace_current_music: true,
    };
    pub const END: Music = Music {
        sound: "minecraft:music.end",
        min_delay: 6_000,
        max_delay: 24_000,
        replace_current_music: true,
    };
    pub const UNDER_WATER: Music = Music::game("minecraft:music.under_water");
    pub const GAME: Music = Music::game("minecraft:music.game");
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MusicFrequency {
    #[default]
    Default,
    Frequent,
    Constant,
}
impl MusicFrequency {
    fn max_frequency(self) -> i32 {
        match self {
            Self::Default => 20 * 1200,
            Self::Frequent => 10 * 1200,
            Self::Constant => 0,
        }
    }
    fn next_song_delay(
        self,
        music: Option<Music>,
        random_in_range: &mut impl FnMut(i32, i32) -> i32,
    ) -> i32 {
        let Some(music) = music else {
            return self.max_frequency();
        };
        if self == Self::Constant {
            return 100;
        }
        let min = music.min_delay.min(self.max_frequency());
        let max = music.max_delay.min(self.max_frequency());
        random_in_range(min, max)
    }
    pub fn from_options_value(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some("FREQUENT") => Self::Frequent,
            Some("CONSTANT") => Self::Constant,
            _ => Self::Default,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackgroundMusic {
    pub default_music: Option<Music>,
    pub creative_music: Option<Music>,
    pub underwater_music: Option<Music>,
}
impl BackgroundMusic {
    pub const OVERWORLD: Self = Self {
        default_music: Some(musics::GAME),
        creative_music: Some(musics::CREATIVE),
        underwater_music: None,
    };
    pub fn select(self, is_creative: bool, is_underwater: bool) -> Option<Music> {
        if is_underwater && self.underwater_music.is_some() {
            return self.underwater_music;
        }
        if is_creative && self.creative_music.is_some() {
            return self.creative_music;
        }
        self.default_music
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct MusicSituation {
    pub in_level: bool,
    pub is_creative: bool,
    pub is_underwater: bool,
    pub end_boss_fight: bool,
    pub background_music: Option<BackgroundMusic>,
}
pub fn end_boss_music_active(dimension: Option<&str>, overlay_requests_music: bool) -> bool {
    dimension == Some("minecraft:the_end") && overlay_requests_music
}
pub fn situational_music(situation: MusicSituation) -> Option<Music> {
    if !situation.in_level {
        return Some(musics::MENU);
    }
    if situation.end_boss_fight {
        return Some(musics::END_BOSS);
    }
    situation
        .background_music
        .unwrap_or(BackgroundMusic::OVERWORLD)
        .select(situation.is_creative, situation.is_underwater)
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FadeStep {
    Continue(f32),
    Reached,
    Stop,
}
pub fn fade_step(current_gain: f32, target_volume: f32) -> FadeStep {
    if current_gain == target_volume {
        return FadeStep::Reached;
    }
    let mut gain = if current_gain < target_volume {
        let stepped = current_gain + current_gain.clamp(5.0e-4, 0.005);
        if stepped > target_volume {
            target_volume
        } else {
            stepped
        }
    } else {
        let approached = 0.03 * target_volume + 0.97 * current_gain;
        if (approached - target_volume).abs() < 1.0e-4 || approached < target_volume {
            target_volume
        } else {
            approached
        }
    };
    gain = gain.clamp(0.0, 1.0);
    if gain <= 1.0e-4 {
        FadeStep::Stop
    } else {
        FadeStep::Continue(gain)
    }
}
#[derive(Debug)]
pub struct MusicManager {
    current: Option<PlayingMusic>,
    frequency: MusicFrequency,
    current_gain: f32,
    next_song_delay: i32,
}
#[derive(Clone, Copy, Debug)]
struct PlayingMusic {
    music: Music,
    handle: crate::client::audio::SoundHandle,
}
impl Default for MusicManager {
    fn default() -> Self {
        Self {
            current: None,
            frequency: MusicFrequency::Default,
            current_gain: 1.0,
            next_song_delay: 100,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MusicAction {
    Nothing,
    SetGain(f32),
    Stop,
    Start(Music),
}
impl MusicManager {
    pub fn frequency(&self) -> MusicFrequency {
        self.frequency
    }
    pub fn set_frequency(
        &mut self,
        frequency: MusicFrequency,
        situational: Option<Music>,
        random_in_range: &mut impl FnMut(i32, i32) -> i32,
    ) {
        self.frequency = frequency;
        self.next_song_delay = frequency.next_song_delay(situational, random_in_range);
    }
    pub fn is_playing(&self, music: Music) -> bool {
        self.current
            .is_some_and(|playing| playing.music.sound == music.sound)
    }
    #[allow(dead_code)]
    pub fn current_sound(&self) -> Option<ResourceLocation> {
        self.current
            .map(|playing| ResourceLocation::from_string(playing.music.sound))
    }
    pub fn on_started(&mut self, music: Music, handle: crate::client::audio::SoundHandle) {
        self.current = Some(PlayingMusic { music, handle });
        self.next_song_delay = i32::MAX;
    }
    pub fn on_start_failed(&mut self) {
        self.current = None;
    }
    pub fn current_handle(&self) -> Option<crate::client::audio::SoundHandle> {
        self.current.map(|playing| playing.handle)
    }
    pub fn on_stopped(
        &mut self,
        situational: Option<Music>,
        random_in_range: &mut impl FnMut(i32, i32) -> i32,
    ) {
        self.current = None;
        self.next_song_delay = self
            .frequency
            .next_song_delay(situational, random_in_range)
            .saturating_add(100);
    }
    pub fn tick(
        &mut self,
        situational: Option<Music>,
        music_volume: f32,
        still_active: bool,
        level_loading: bool,
        random_in_range: &mut impl FnMut(i32, i32) -> i32,
    ) -> MusicAction {
        if self.current.is_some() && self.current_gain != music_volume {
            match fade_step(self.current_gain, music_volume) {
                FadeStep::Stop => {
                    self.current_gain = music_volume.clamp(0.0, 1.0);
                    self.on_stopped(situational, random_in_range);
                    return MusicAction::Stop;
                }
                FadeStep::Continue(gain) => {
                    self.current_gain = gain;
                    return MusicAction::SetGain(gain);
                }
                FadeStep::Reached => {}
            }
        }
        let Some(music) = situational else {
            self.next_song_delay = self.next_song_delay.max(100);
            return MusicAction::Nothing;
        };
        if let Some(playing) = self.current {
            if music.replace_current_music && music.sound != playing.music.sound {
                self.next_song_delay = random_in_range(0, music.min_delay / 2);
                self.current = None;
                return MusicAction::Stop;
            }
            if !still_active {
                self.current = None;
                self.next_song_delay = self
                    .next_song_delay
                    .min(self.frequency.next_song_delay(Some(music), random_in_range));
            }
        }
        self.next_song_delay = self.next_song_delay.min(music.max_delay);
        if self.current.is_none() && !level_loading {
            self.next_song_delay -= 1;
            if self.next_song_delay <= 0 {
                return MusicAction::Start(music);
            }
        }
        MusicAction::Nothing
    }
}
