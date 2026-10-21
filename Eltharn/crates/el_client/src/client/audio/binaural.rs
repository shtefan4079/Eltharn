use glam::{Vec2, Vec3};
use rodio::Source;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
const RATE: u32 = 44_100;
const TAPS: usize = 128;
const TRANSITION: usize = 128;
type Kernel = [Vec2; TAPS];
struct Measurement {
    elevation: f32,
    azimuth: f32,
    kernel: Kernel,
}
fn measurements() -> &'static [Measurement] {
    static DATA: OnceLock<Vec<Measurement>> = OnceLock::new();
    DATA.get_or_init(|| {
        let bytes = include_bytes!("hrtf_data/kemar.bin");
        assert_eq!(&bytes[..8], b"ROHRIR01");
        let count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        assert_eq!(bytes.len(), 12 + count * (4 + TAPS * 4));
        bytes[12..]
            .chunks_exact(4 + TAPS * 4)
            .map(|record| {
                let mut kernel = [Vec2::ZERO; TAPS];
                for (tap, samples) in kernel.iter_mut().zip(record[4..].chunks_exact(4)) {
                    *tap = Vec2::new(
                        i16::from_le_bytes([samples[0], samples[1]]) as f32,
                        i16::from_le_bytes([samples[2], samples[3]]) as f32,
                    ) / 32768.;
                }
                Measurement {
                    elevation: i16::from_le_bytes([record[0], record[1]]) as f32,
                    azimuth: u16::from_le_bytes([record[2], record[3]]) as f32,
                    kernel,
                }
            })
            .collect()
    })
}
fn kernel_for(direction: Vec3) -> Kernel {
    let direction = direction.try_normalize().unwrap_or(Vec3::Z);
    let elevation = direction.y.asin().to_degrees().clamp(-40., 90.);
    let azimuth = direction.x.atan2(direction.z).to_degrees().abs();
    let lower = (elevation / 10.).floor() * 10.;
    let upper = (lower + 10.).min(90.);
    let mix = (elevation - lower) / 10.;
    let data = measurements();
    let mut result = [Vec2::ZERO; TAPS];
    for (elevation, weight) in [(lower, 1. - mix), (upper, mix)] {
        if weight == 0. {
            continue;
        }
        let ring_start = data.partition_point(|point| point.elevation < elevation);
        let ring_end = data.partition_point(|point| point.elevation <= elevation);
        let ring = &data[ring_start..ring_end];
        let high = ring
            .partition_point(|point| point.azimuth < azimuth)
            .min(ring.len() - 1);
        let low = high.saturating_sub(1);
        let a = &ring[low];
        let b = &ring[high];
        let t = if a.azimuth == b.azimuth {
            0.
        } else {
            ((azimuth - a.azimuth) / (b.azimuth - a.azimuth)).clamp(0., 1.)
        };
        for (index, tap) in result.iter_mut().enumerate() {
            *tap += a.kernel[index].lerp(b.kernel[index], t) * weight;
        }
    }
    if direction.x < 0. {
        for tap in &mut result {
            *tap = Vec2::new(tap.y, tap.x);
        }
    }
    result
}
#[derive(Clone)]
pub(super) struct BinauralControl(Arc<Mutex<(Vec3, Kernel)>>);
impl BinauralControl {
    pub(super) fn set_direction(&self, direction: Vec3) {
        let direction = direction.try_normalize().unwrap_or(Vec3::Z);
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if state.0.distance_squared(direction) > 1e-8 {
            *state = (direction, kernel_for(direction));
        }
    }
}
struct Mono<S>(S);
impl<S: Source<Item = f32>> Iterator for Mono<S> {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let channels = self.0.channels().max(1);
        let mut value = self.0.next()?;
        for _ in 1..channels {
            value += self.0.next().unwrap_or(0.);
        }
        Some(value / channels as f32)
    }
}
impl<S: Source<Item = f32>> Source for Mono<S> {
    fn current_frame_len(&self) -> Option<usize> {
        self.0
            .current_frame_len()
            .map(|count| count / self.0.channels().max(1) as usize)
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        self.0.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.0.total_duration()
    }
}
pub(super) struct BinauralSource<S: Source<Item = f32>> {
    source: rodio::source::UniformSourceIterator<Mono<S>, f32>,
    control: BinauralControl,
    current: Kernel,
    previous: Kernel,
    history: [f32; TAPS * 2],
    cursor: usize,
    frame: usize,
    right_sample: Option<f32>,
    tail: usize,
    heard_input: bool,
    transitioning: bool,
}
impl<S: Source<Item = f32>> BinauralSource<S> {
    pub(super) fn new(source: S, direction: Vec3) -> (Self, BinauralControl) {
        let direction = direction.try_normalize().unwrap_or(Vec3::Z);
        let kernel = kernel_for(direction);
        let control = BinauralControl(Arc::new(Mutex::new((direction, kernel))));
        (
            Self {
                source: rodio::source::UniformSourceIterator::new(Mono(source), 1, RATE),
                control: control.clone(),
                current: kernel,
                previous: kernel,
                history: [0.; TAPS * 2],
                cursor: 0,
                frame: 0,
                right_sample: None,
                tail: TAPS - 1,
                heard_input: false,
                transitioning: false,
            },
            control,
        )
    }
}
impl<S: Source<Item = f32>> Iterator for BinauralSource<S> {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if let Some(right) = self.right_sample.take() {
            return Some(right);
        }
        let sample = if let Some(sample) = self.source.next() {
            self.heard_input = true;
            sample
        } else if self.heard_input && self.tail > 0 {
            self.tail -= 1;
            0.
        } else {
            return None;
        };
        if self.frame == 0 {
            self.previous = self.current;
            if let Ok(state) = self.control.0.try_lock() {
                self.current = state.1;
            }
            self.transitioning = self.previous != self.current;
        }
        self.cursor = (self.cursor + TAPS - 1) % TAPS;
        self.history[self.cursor] = sample;
        self.history[self.cursor + TAPS] = sample;
        let history = &self.history[self.cursor..self.cursor + TAPS];
        let mut value = Vec2::ZERO;
        let mut old = Vec2::ZERO;
        for (index, sample) in history.iter().enumerate() {
            value += self.current[index] * *sample;
            if self.transitioning {
                old += self.previous[index] * *sample;
            }
        }
        if self.transitioning {
            value = old.lerp(value, (self.frame + 1) as f32 / TRANSITION as f32);
        }
        self.frame = (self.frame + 1) % TRANSITION;
        self.right_sample = Some(value.y);
        Some(value.x)
    }
}
impl<S: Source<Item = f32>> Source for BinauralSource<S> {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        RATE
    }
    fn total_duration(&self) -> Option<Duration> {
        self.source
            .total_duration()
            .map(|duration| duration + Duration::from_secs_f64((TAPS - 1) as f64 / RATE as f64))
    }
}
