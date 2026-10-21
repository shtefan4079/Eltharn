use crate::world::client_level::{
    RAIN_SUNRISE_MULTIPLIER, THUNDER_SUNRISE_MULTIPLIER, argb_to_rgba, multiply_argb, srgb_lerp,
};
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct SkyRenderState {
    pub should_render_dark_disc: bool,
    pub sun_angle: f32,
    pub moon_angle: f32,
    pub star_angle: f32,
    pub rain_brightness: f32,
    pub star_brightness: f32,
    pub sunrise_and_sunset_color: [f32; 4],
    pub sky_color: [f32; 3],
    pub fog_color: [f32; 3],
    pub day_time: i64,
    pub moon_phase: u32,
    pub view_distance_blocks: f32,
}
impl Default for SkyRenderState {
    fn default() -> Self {
        Self {
            should_render_dark_disc: false,
            sun_angle: 0.0,
            moon_angle: 0.0,
            star_angle: 0.0,
            rain_brightness: 1.0,
            star_brightness: 0.0,
            sunrise_and_sunset_color: [0.0, 0.0, 0.0, 0.0],
            sky_color: [0.1, 0.5, 0.8],
            fog_color: [0.75, 0.85, 1.0],
            day_time: 0,
            moon_phase: 0,
            view_distance_blocks: 192.0,
        }
    }
}
#[allow(dead_code)]
impl SkyRenderState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    #[inline]
    pub fn is_sunrise_visible(&self) -> bool {
        self.sunrise_and_sunset_color[3] > 0.001
    }
    #[inline]
    pub fn are_stars_visible(&self) -> bool {
        self.star_brightness > 0.0
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Keyframe<T> {
    pub ticks: i32,
    pub value: T,
}
#[derive(Debug, Clone)]
pub struct KeyframeTrackSampler<T: 'static> {
    keyframes: &'static [Keyframe<T>],
    period: i32,
}
impl<T: Copy + 'static> KeyframeTrackSampler<T> {
    pub const fn new(keyframes: &'static [Keyframe<T>], period: i32) -> Self {
        Self { keyframes, period }
    }
    pub fn sample(&self, ticks: f32, lerp: impl Fn(f32, T, T) -> T) -> T {
        if self.keyframes.is_empty() {
            panic!("KeyframeTrackSampler: no keyframes");
        }
        if self.keyframes.len() == 1 {
            return self.keyframes[0].value;
        }
        let looped_ticks = ticks.rem_euclid(self.period as f32);
        let first = &self.keyframes[0];
        let last = &self.keyframes[self.keyframes.len() - 1];
        if looped_ticks < first.ticks as f32 {
            let from_ticks = last.ticks as f32 - self.period as f32;
            let to_ticks = first.ticks as f32;
            let alpha = (looped_ticks - from_ticks) / (to_ticks - from_ticks);
            return lerp(alpha.clamp(0.0, 1.0), last.value, first.value);
        }
        if looped_ticks >= last.ticks as f32 {
            let from_ticks = last.ticks as f32;
            let to_ticks = first.ticks as f32 + self.period as f32;
            let alpha = (looped_ticks - from_ticks) / (to_ticks - from_ticks);
            return lerp(alpha.clamp(0.0, 1.0), last.value, first.value);
        }
        for i in 0..(self.keyframes.len() - 1) {
            let from = &self.keyframes[i];
            let to = &self.keyframes[i + 1];
            if looped_ticks >= from.ticks as f32 && looped_ticks < to.ticks as f32 {
                let alpha =
                    (looped_ticks - from.ticks as f32) / (to.ticks as f32 - from.ticks as f32);
                return lerp(alpha.clamp(0.0, 1.0), from.value, to.value);
            }
        }
        last.value
    }
}
pub const SUNRISE_SUNSET_KEYFRAMES: &[Keyframe<u32>] = &[
    Keyframe {
        ticks: 71,
        value: 1609540403,
    },
    Keyframe {
        ticks: 310,
        value: 703969843,
    },
    Keyframe {
        ticks: 565,
        value: 117167155,
    },
    Keyframe {
        ticks: 730,
        value: 16770355,
    },
    Keyframe {
        ticks: 11270,
        value: 16770355,
    },
    Keyframe {
        ticks: 11397,
        value: 83679283,
    },
    Keyframe {
        ticks: 11522,
        value: 268028723,
    },
    Keyframe {
        ticks: 11690,
        value: 703969843,
    },
    Keyframe {
        ticks: 11929,
        value: 1609540403,
    },
    Keyframe {
        ticks: 12243,
        value: 2984740659,
    },
    Keyframe {
        ticks: 12358,
        value: 3437526579,
    },
    Keyframe {
        ticks: 12512,
        value: 3923800627,
    },
    Keyframe {
        ticks: 12613,
        value: 4141706035,
    },
    Keyframe {
        ticks: 12732,
        value: 4275725107,
    },
    Keyframe {
        ticks: 12841,
        value: 4275526707,
    },
    Keyframe {
        ticks: 13035,
        value: 3973206323,
    },
    Keyframe {
        ticks: 13252,
        value: 3251390259,
    },
    Keyframe {
        ticks: 13775,
        value: 918435635,
    },
    Keyframe {
        ticks: 13888,
        value: 532362547,
    },
    Keyframe {
        ticks: 14039,
        value: 163001139,
    },
    Keyframe {
        ticks: 14192,
        value: 11744051,
    },
    Keyframe {
        ticks: 21807,
        value: 11678515,
    },
    Keyframe {
        ticks: 21961,
        value: 163001139,
    },
    Keyframe {
        ticks: 22112,
        value: 532362547,
    },
    Keyframe {
        ticks: 22225,
        value: 918435635,
    },
    Keyframe {
        ticks: 22748,
        value: 3251390259,
    },
    Keyframe {
        ticks: 22965,
        value: 3973206323,
    },
    Keyframe {
        ticks: 23159,
        value: 4275526707,
    },
    Keyframe {
        ticks: 23272,
        value: 4275725107,
    },
    Keyframe {
        ticks: 23488,
        value: 3923800627,
    },
    Keyframe {
        ticks: 23642,
        value: 3437526579,
    },
    Keyframe {
        ticks: 23757,
        value: 2984740659,
    },
];
pub const STAR_BRIGHTNESS_KEYFRAMES: &[Keyframe<f32>] = &[
    Keyframe {
        ticks: 92,
        value: 0.037,
    },
    Keyframe {
        ticks: 627,
        value: 0.0,
    },
    Keyframe {
        ticks: 11373,
        value: 0.0,
    },
    Keyframe {
        ticks: 11732,
        value: 0.016,
    },
    Keyframe {
        ticks: 11959,
        value: 0.044,
    },
    Keyframe {
        ticks: 12399,
        value: 0.143,
    },
    Keyframe {
        ticks: 12729,
        value: 0.258,
    },
    Keyframe {
        ticks: 13228,
        value: 0.5,
    },
    Keyframe {
        ticks: 22772,
        value: 0.5,
    },
    Keyframe {
        ticks: 23032,
        value: 0.364,
    },
    Keyframe {
        ticks: 23356,
        value: 0.225,
    },
    Keyframe {
        ticks: 23758,
        value: 0.101,
    },
];
#[derive(Debug, Clone, Copy)]
pub struct CubicCurve {
    a: f32,
    b: f32,
    c: f32,
}
impl CubicCurve {
    pub fn new(v1: f32, v2: f32) -> Self {
        Self {
            a: 3.0 * v1 - 3.0 * v2 + 1.0,
            b: -6.0 * v1 + 3.0 * v2,
            c: 3.0 * v1,
        }
    }
    pub fn sample(&self, t: f32) -> f32 {
        ((self.a * t + self.b) * t + self.c) * t
    }
    pub fn sample_gradient(&self, t: f32) -> f32 {
        (3.0 * self.a * t + 2.0 * self.b) * t + self.c
    }
}
#[derive(Debug, Clone, Copy)]
pub struct CubicBezier {
    x_curve: CubicCurve,
    y_curve: CubicCurve,
}
impl CubicBezier {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self {
            x_curve: CubicCurve::new(x1, x2),
            y_curve: CubicCurve::new(y1, y2),
        }
    }
    pub fn apply(&self, x: f32) -> f32 {
        self.y_curve.sample(self.solve_t(x))
    }
    pub fn solve_t(&self, x: f32) -> f32 {
        let mut t = x;
        for _ in 0..4 {
            let error = self.x_curve.sample(t) - x;
            if error.abs() < 1e-5 {
                return t;
            }
            let gradient = self.x_curve.sample_gradient(t);
            if gradient < 1e-5 {
                break;
            }
            t -= (error / gradient).clamp(-0.25, 0.25);
        }
        self.solve_t_bisect(x, t)
    }
    fn solve_t_bisect(&self, x: f32, initial_t: f32) -> f32 {
        let mut t0 = 0.0;
        let mut t1 = 1.0;
        let mut t = initial_t;
        while t0 < t1 {
            let error = self.x_curve.sample(t) - x;
            if error.abs() < 1e-5 {
                return t;
            }
            if error < 0.0 {
                t0 = t;
            } else {
                t1 = t;
            }
            t = (t1 + t0) / 2.0;
            if (t1 - t0) < 1e-6 {
                break;
            }
        }
        t
    }
}
#[inline]
pub fn color_lerp(alpha: f32, c0: u32, c1: u32) -> u32 {
    srgb_lerp(alpha, c0, c1)
}
#[inline]
pub fn float_lerp(alpha: f32, f0: f32, f1: f32) -> f32 {
    f0 + alpha * (f1 - f0)
}
pub fn apply_sunrise_weather(color: u32, rain_level: f32, thunder_level: f32) -> u32 {
    let thunder = thunder_level.clamp(0.0, 1.0);
    let rain = (rain_level - thunder).clamp(0.0, 1.0);
    let mut color = color;
    if rain > 0.0 {
        color = srgb_lerp(rain, color, multiply_argb(color, RAIN_SUNRISE_MULTIPLIER));
    }
    if thunder > 0.0 {
        color = srgb_lerp(
            thunder,
            color,
            multiply_argb(color, THUNDER_SUNRISE_MULTIPLIER),
        );
    }
    color
}
#[derive(Debug, Clone)]
pub struct SkyTimeline {
    sunrise_sunset_sampler: KeyframeTrackSampler<u32>,
    star_brightness_sampler: KeyframeTrackSampler<f32>,
}
impl Default for SkyTimeline {
    fn default() -> Self {
        Self::new()
    }
}
impl SkyTimeline {
    pub const fn new() -> Self {
        Self {
            sunrise_sunset_sampler: KeyframeTrackSampler::new(SUNRISE_SUNSET_KEYFRAMES, 24000),
            star_brightness_sampler: KeyframeTrackSampler::new(STAR_BRIGHTNESS_KEYFRAMES, 24000),
        }
    }
    pub fn celestial_angle(&self, day_time: i64, partial_tick: f32) -> f32 {
        let exact_ticks = day_time as f32 + partial_tick;
        let alpha = (exact_ticks - 6000.0).rem_euclid(24000.0) / 24000.0;
        let bezier = CubicBezier::new(0.362, 0.241, 1.0 - 0.362, 1.0 - 0.241);
        bezier.apply(alpha)
    }
    pub fn sun_angle(&self, day_time: i64, partial_tick: f32) -> f32 {
        self.celestial_angle(day_time, partial_tick) * std::f32::consts::TAU
    }
    pub fn moon_angle(&self, day_time: i64, partial_tick: f32) -> f32 {
        self.sun_angle(day_time, partial_tick) + std::f32::consts::PI
    }
    pub fn star_angle(&self, day_time: i64, partial_tick: f32) -> f32 {
        self.sun_angle(day_time, partial_tick)
    }
    pub fn sunrise_sunset_color(
        &self,
        day_time: i64,
        partial_tick: f32,
        rain_level: f32,
        thunder_level: f32,
    ) -> [f32; 4] {
        let exact_ticks = day_time as f32 + partial_tick;
        let ss_color_packed = apply_sunrise_weather(
            self.sunrise_sunset_sampler.sample(exact_ticks, color_lerp),
            rain_level,
            thunder_level,
        );
        argb_to_rgba(ss_color_packed)
    }
    pub fn raw_star_brightness(&self, day_time: i64, partial_tick: f32) -> f32 {
        let exact_ticks = day_time as f32 + partial_tick;
        self.star_brightness_sampler.sample(exact_ticks, float_lerp)
    }
    pub fn star_brightness(&self, day_time: i64, partial_tick: f32, rain_level: f32) -> f32 {
        self.raw_star_brightness(day_time, partial_tick) * (1.0 - rain_level)
    }
    pub const fn rain_brightness(rain_level: f32) -> f32 {
        1.0 - rain_level
    }
    pub fn extract_sky_render_state(
        &self,
        day_time: i64,
        partial_tick: f32,
        rain_level: f32,
        thunder_level: f32,
        sky_color: [f32; 3],
        fog_color: [f32; 3],
        should_render_dark_disc: bool,
        moon_phase: u32,
        view_distance_blocks: f32,
    ) -> SkyRenderState {
        SkyRenderState {
            should_render_dark_disc,
            sun_angle: self.sun_angle(day_time, partial_tick),
            moon_angle: self.moon_angle(day_time, partial_tick),
            star_angle: self.star_angle(day_time, partial_tick),
            rain_brightness: Self::rain_brightness(rain_level),
            star_brightness: self.star_brightness(day_time, partial_tick, rain_level),
            sunrise_and_sunset_color: self.sunrise_sunset_color(
                day_time,
                partial_tick,
                rain_level,
                thunder_level,
            ),
            sky_color,
            fog_color,
            day_time,
            moon_phase,
            view_distance_blocks,
        }
    }
}
