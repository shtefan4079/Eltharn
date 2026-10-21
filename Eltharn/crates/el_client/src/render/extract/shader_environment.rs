use crate::render::state::level::LevelRenderState;
use crate::world::client_level::ClientLevel;
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderEnvironmentFrameV1 {
    pub game_time: i64,
    pub day_time: i64,
    pub partial_tick: f32,
    pub rain_level: f32,
    pub thunder_level: f32,
    pub sky_light_factor: f32,
    pub sun_angle_radians: f32,
    pub moon_angle_radians: f32,
    pub star_brightness: f32,
    pub rain_brightness: f32,
    pub sky_light_color: [f32; 3],
    pub moon_phase: u32,
}
impl ShaderEnvironmentFrameV1 {
    pub fn extract(frame: &LevelRenderState, level: &ClientLevel) -> Result<Self, String> {
        if !frame.is_in_world || !(0.0..=1.0).contains(&frame.partial_tick) {
            return Err("shader environment requires a world frame and valid partial tick".into());
        }
        let sky = &frame.sky_render_state;
        let environment = Self {
            game_time: frame.game_time,
            day_time: frame.day_time,
            partial_tick: frame.partial_tick,
            rain_level: level.rain_level(frame.partial_tick),
            thunder_level: level.thunder_level(frame.partial_tick),
            sky_light_factor: level.sky_light_factor(frame.partial_tick),
            sun_angle_radians: sky.sun_angle,
            moon_angle_radians: sky.moon_angle,
            star_brightness: sky.star_brightness,
            rain_brightness: sky.rain_brightness,
            sky_light_color: level.sky_light_color(frame.partial_tick),
            moon_phase: sky.moon_phase,
        };
        environment.validate()?;
        Ok(environment)
    }
    fn validate(&self) -> Result<(), String> {
        let values = [
            self.partial_tick,
            self.rain_level,
            self.thunder_level,
            self.sky_light_factor,
            self.sun_angle_radians,
            self.moon_angle_radians,
            self.star_brightness,
            self.rain_brightness,
            self.sky_light_color[0],
            self.sky_light_color[1],
            self.sky_light_color[2],
        ];
        if values.iter().any(|value| !value.is_finite()) || self.moon_phase >= 8 {
            return Err("invalid shader environment frame".into());
        }
        Ok(())
    }
    pub fn bytes(&self) -> Result<[u8; 80], String> {
        self.validate()?;
        let mut out = [0_u8; 80];
        for (index, word) in [
            self.game_time as u32,
            ((self.game_time as u64) >> 32) as u32,
            self.day_time as u32,
            ((self.day_time as u64) >> 32) as u32,
        ]
        .into_iter()
        .enumerate()
        {
            out[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        for (index, value) in [
            self.partial_tick,
            self.rain_level,
            self.thunder_level,
            self.sky_light_factor,
            self.sun_angle_radians,
            self.moon_angle_radians,
            self.star_brightness,
            self.rain_brightness,
            self.sky_light_color[0],
            self.sky_light_color[1],
            self.sky_light_color[2],
            0.0,
        ]
        .into_iter()
        .enumerate()
        {
            let start = 16 + index * 4;
            out[start..start + 4].copy_from_slice(&value.to_le_bytes());
        }
        out[64..68].copy_from_slice(&self.moon_phase.to_le_bytes());
        Ok(out)
    }
}
