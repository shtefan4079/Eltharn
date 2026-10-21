pub const DISPLAY_TIME_MILLIS: f64 = 3000.0;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoundPlayedAt {
    pub location: glam::Vec3,
    pub time_millis: u64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Subtitle {
    pub text: String,
    pub range: f32,
    played_at: Vec<SoundPlayedAt>,
}
impl Subtitle {
    fn new(text: String, range: f32, location: glam::Vec3, now_millis: u64) -> Self {
        Self {
            text,
            range,
            played_at: vec![SoundPlayedAt {
                location,
                time_millis: now_millis,
            }],
        }
    }
    fn refresh(&mut self, location: glam::Vec3, now_millis: u64) {
        self.played_at.retain(|played| played.location != location);
        self.played_at.push(SoundPlayedAt {
            location,
            time_millis: now_millis,
        });
    }
    pub fn closest(&self, position: glam::Vec3) -> Option<SoundPlayedAt> {
        if self.played_at.len() == 1 {
            return self.played_at.first().copied();
        }
        self.played_at.iter().copied().min_by(|left, right| {
            let left = left.location.distance(position);
            let right = right.location.distance(position);
            left.total_cmp(&right)
        })
    }
    pub fn is_audible_from(&self, camera: glam::Vec3) -> bool {
        if self.range.is_infinite() {
            return true;
        }
        let Some(closest) = self.closest(camera) else {
            return false;
        };
        camera.distance_squared(closest.location) < self.range * self.range
    }
    fn purge_old_instances(&mut self, max_age_millis: f64, now_millis: u64) {
        self.played_at.retain(|played| {
            (now_millis.saturating_sub(played.time_millis)) as f64 <= max_age_millis
        });
    }
    pub fn is_still_active(&self) -> bool {
        !self.played_at.is_empty()
    }
}
#[derive(Clone, Debug, Default)]
pub struct SubtitleOverlay {
    subtitles: Vec<Subtitle>,
}
impl SubtitleOverlay {
    pub fn on_play_sound(
        &mut self,
        subtitle: Option<&str>,
        range: f32,
        location: glam::Vec3,
        now_millis: u64,
    ) {
        let Some(subtitle) = subtitle else {
            return;
        };
        if let Some(existing) = self
            .subtitles
            .iter_mut()
            .find(|candidate| candidate.text == subtitle)
        {
            existing.refresh(location, now_millis);
            return;
        }
        self.subtitles.push(Subtitle::new(
            subtitle.to_owned(),
            range,
            location,
            now_millis,
        ));
    }
    pub fn audible(
        &mut self,
        listener: glam::Vec3,
        display_time_multiplier: f64,
        now_millis: u64,
    ) -> Vec<Subtitle> {
        let max_age = DISPLAY_TIME_MILLIS * display_time_multiplier;
        let mut audible = Vec::new();
        for subtitle in &mut self.subtitles {
            if !subtitle.is_audible_from(listener) {
                continue;
            }
            subtitle.purge_old_instances(max_age, now_millis);
            if subtitle.is_still_active() {
                audible.push(subtitle.clone());
            }
        }
        self.subtitles.retain(Subtitle::is_still_active);
        audible
    }
    pub fn clear(&mut self) {
        self.subtitles.clear();
    }
    pub fn is_empty(&self) -> bool {
        self.subtitles.is_empty()
    }
}
