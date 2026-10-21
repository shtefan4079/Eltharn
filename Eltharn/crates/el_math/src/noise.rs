use crate::random::{LegacyRandomSource, RandomSource, WorldgenRandom};
use std::sync::OnceLock;
const GRADIENT: [[i32; 3]; 16] = [
    [1, 1, 0],
    [-1, 1, 0],
    [1, -1, 0],
    [-1, -1, 0],
    [1, 0, 1],
    [-1, 0, 1],
    [1, 0, -1],
    [-1, 0, -1],
    [0, 1, 1],
    [0, -1, 1],
    [0, 1, -1],
    [0, -1, -1],
    [1, 1, 0],
    [0, -1, 1],
    [-1, 1, 0],
    [0, -1, -1],
];
#[inline]
fn dot(g: [i32; 3], x: f64, y: f64, z: f64) -> f64 {
    (g[0] as f64) * x + (g[1] as f64) * y + (g[2] as f64) * z
}
#[inline]
fn grad_dot(hash: i32, x: f64, y: f64, z: f64) -> f64 {
    dot(GRADIENT[(hash & 15) as usize], x, y, z)
}
#[inline]
fn floor(v: f64) -> i32 {
    v.floor() as i32
}
#[inline]
fn smoothstep(x: f64) -> f64 {
    x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
}
#[inline]
fn lerp(alpha: f64, p0: f64, p1: f64) -> f64 {
    p0 + alpha * (p1 - p0)
}
#[inline]
fn lerp2(alpha1: f64, alpha2: f64, x00: f64, x10: f64, x01: f64, x11: f64) -> f64 {
    lerp(alpha2, lerp(alpha1, x00, x10), lerp(alpha1, x01, x11))
}
#[inline]
fn lerp3(
    alpha1: f64,
    alpha2: f64,
    alpha3: f64,
    x000: f64,
    x100: f64,
    x010: f64,
    x110: f64,
    x001: f64,
    x101: f64,
    x011: f64,
    x111: f64,
) -> f64 {
    lerp(
        alpha3,
        lerp2(alpha1, alpha2, x000, x100, x010, x110),
        lerp2(alpha1, alpha2, x001, x101, x011, x111),
    )
}
#[inline]
fn wrap(x: f64) -> f64 {
    x - (x / 3.3554432E7 + 0.5).floor() * 3.3554432E7
}
pub struct SimplexNoise {
    p: [u8; 256],
    pub xo: f64,
    pub yo: f64,
    pub zo: f64,
}
impl SimplexNoise {
    pub fn new(random: &mut dyn RandomSource) -> Self {
        let xo = random.next_double() * 256.0;
        let yo = random.next_double() * 256.0;
        let zo = random.next_double() * 256.0;
        let mut p = [0u8; 256];
        for (index, value) in p.iter_mut().enumerate() {
            *value = index as u8;
        }
        for index in 0..256 {
            let offset = random.next_int_bound(256 - index as i32) as usize;
            p.swap(index, index + offset);
        }
        Self { p, xo, yo, zo }
    }
    #[inline]
    fn p(&self, value: i32) -> i32 {
        self.p[(value & 0xFF) as usize] as i32
    }
    #[inline]
    fn corner_noise(index: usize, x: f64, y: f64, z: f64, base: f64) -> f64 {
        let mut value = base - x * x - y * y - z * z;
        if value < 0.0 {
            0.0
        } else {
            value *= value;
            value * value * dot(GRADIENT[index], x, y, z)
        }
    }
    pub fn get_value_2d(&self, xin: f64, yin: f64) -> f64 {
        let sqrt_3 = 3.0_f64.sqrt();
        let f2 = 0.5 * (sqrt_3 - 1.0);
        let g2 = (3.0 - sqrt_3) / 6.0;
        let skew = (xin + yin) * f2;
        let i = floor(xin + skew);
        let j = floor(yin + skew);
        let unskew = (i + j) as f64 * g2;
        let x0 = xin - (i as f64 - unskew);
        let y0 = yin - (j as f64 - unskew);
        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
        let x1 = x0 - i1 as f64 + g2;
        let y1 = y0 - j1 as f64 + g2;
        let x2 = x0 - 1.0 + 2.0 * g2;
        let y2 = y0 - 1.0 + 2.0 * g2;
        let ii = i & 0xFF;
        let jj = j & 0xFF;
        let gi0 = (self.p(ii + self.p(jj)) % 12) as usize;
        let gi1 = (self.p(ii + i1 + self.p(jj + j1)) % 12) as usize;
        let gi2 = (self.p(ii + 1 + self.p(jj + 1)) % 12) as usize;
        70.0 * (Self::corner_noise(gi0, x0, y0, 0.0, 0.5)
            + Self::corner_noise(gi1, x1, y1, 0.0, 0.5)
            + Self::corner_noise(gi2, x2, y2, 0.0, 0.5))
    }
}
pub fn biome_info_noise(x: f64, z: f64) -> f64 {
    static BIOME_INFO_NOISE: OnceLock<SimplexNoise> = OnceLock::new();
    BIOME_INFO_NOISE
        .get_or_init(|| {
            let mut random = WorldgenRandom::new(Box::new(LegacyRandomSource::new(2345)));
            SimplexNoise::new(&mut random)
        })
        .get_value_2d(x, z)
}
pub struct PerlinSimplexNoise2d {
    levels: Vec<Option<SimplexNoise>>,
    highest_freq_input_factor: f64,
    highest_freq_value_factor: f64,
}
impl PerlinSimplexNoise2d {
    pub fn new(seed: i64, octaves: &[i32]) -> Self {
        let low = -*octaves.iter().min().expect("at least one octave");
        let high = *octaves.iter().max().expect("at least one octave");
        let count = low + high + 1;
        let mut random = WorldgenRandom::new(Box::new(LegacyRandomSource::new(seed)));
        let zero = SimplexNoise::new(&mut random);
        let zero_index = high;
        let mut levels: Vec<Option<SimplexNoise>> = (0..count).map(|_| None).collect();
        if octaves.contains(&0) {
            levels[zero_index as usize] = Some(zero);
        }
        for index in (high + 1)..count {
            let octave = zero_index - index;
            if octaves.contains(&octave) {
                levels[index as usize] = Some(SimplexNoise::new(&mut random));
            } else {
                random.consume_count(262);
            }
        }
        Self {
            levels,
            highest_freq_input_factor: 2.0_f64.powi(high),
            highest_freq_value_factor: 1.0 / (2.0_f64.powi(count) - 1.0),
        }
    }
    pub fn value(&self, x: f64, z: f64) -> f64 {
        let mut value = 0.0;
        let mut input_factor = self.highest_freq_input_factor;
        let mut value_factor = self.highest_freq_value_factor;
        for level in &self.levels {
            if let Some(noise) = level {
                value += noise.get_value_2d(x * input_factor, z * input_factor) * value_factor;
            }
            input_factor /= 2.0;
            value_factor *= 2.0;
        }
        value
    }
}
pub fn biome_temperature_noise(x: f64, z: f64) -> f64 {
    static NOISE: OnceLock<PerlinSimplexNoise2d> = OnceLock::new();
    NOISE
        .get_or_init(|| PerlinSimplexNoise2d::new(1234, &[0]))
        .value(x / 8.0, z / 8.0)
}
pub fn biome_frozen_temperature_noise(x: f64, z: f64) -> f64 {
    static NOISE: OnceLock<PerlinSimplexNoise2d> = OnceLock::new();
    NOISE
        .get_or_init(|| PerlinSimplexNoise2d::new(3456, &[-2, -1, 0]))
        .value(x * 0.05, z * 0.05)
}
pub struct ImprovedNoise {
    p: [u8; 256],
    pub xo: f64,
    pub yo: f64,
    pub zo: f64,
}
impl ImprovedNoise {
    pub fn new(random: &mut dyn RandomSource) -> Self {
        let xo = random.next_double() * 256.0;
        let yo = random.next_double() * 256.0;
        let zo = random.next_double() * 256.0;
        let mut p = [0u8; 256];
        for i in 0..256 {
            p[i] = i as u8;
        }
        for i in 0..256 {
            let offset = random.next_int_bound(256 - i as i32) as usize;
            let tmp = p[i];
            p[i] = p[i + offset];
            p[i + offset] = tmp;
        }
        Self { p, xo, yo, zo }
    }
    #[inline]
    fn p(&self, x: i32) -> i32 {
        self.p[(x & 0xFF) as usize] as i32
    }
    pub fn noise(&self, x: f64, y: f64, z: f64) -> f64 {
        self.noise_scaled(x, y, z, 0.0, 0.0)
    }
    pub fn noise_scaled(&self, _x: f64, _y: f64, _z: f64, y_scale: f64, y_fudge: f64) -> f64 {
        let x = _x + self.xo;
        let y = _y + self.yo;
        let z = _z + self.zo;
        let xf = floor(x);
        let yf = floor(y);
        let zf = floor(z);
        let xr = x - xf as f64;
        let yr = y - yf as f64;
        let zr = z - zf as f64;
        let yr_fudge = if y_scale != 0.0 {
            let fudge_limit = if y_fudge >= 0.0 && y_fudge < yr {
                y_fudge
            } else {
                yr
            };
            ((fudge_limit / y_scale + 1.0E-7).floor()) * y_scale
        } else {
            0.0
        };
        self.sample_and_lerp(xf, yf, zf, xr, yr - yr_fudge, zr, yr)
    }
    fn sample_and_lerp(
        &self,
        x: i32,
        y: i32,
        z: i32,
        xr: f64,
        yr: f64,
        zr: f64,
        yr_original: f64,
    ) -> f64 {
        let x0 = self.p(x);
        let x1 = self.p(x + 1);
        let xy00 = self.p(x0 + y);
        let xy01 = self.p(x0 + y + 1);
        let xy10 = self.p(x1 + y);
        let xy11 = self.p(x1 + y + 1);
        let d000 = grad_dot(self.p(xy00 + z), xr, yr, zr);
        let d100 = grad_dot(self.p(xy10 + z), xr - 1.0, yr, zr);
        let d010 = grad_dot(self.p(xy01 + z), xr, yr - 1.0, zr);
        let d110 = grad_dot(self.p(xy11 + z), xr - 1.0, yr - 1.0, zr);
        let d001 = grad_dot(self.p(xy00 + z + 1), xr, yr, zr - 1.0);
        let d101 = grad_dot(self.p(xy10 + z + 1), xr - 1.0, yr, zr - 1.0);
        let d011 = grad_dot(self.p(xy01 + z + 1), xr, yr - 1.0, zr - 1.0);
        let d111 = grad_dot(self.p(xy11 + z + 1), xr - 1.0, yr - 1.0, zr - 1.0);
        let x_alpha = smoothstep(xr);
        let y_alpha = smoothstep(yr_original);
        let z_alpha = smoothstep(zr);
        lerp3(
            x_alpha, y_alpha, z_alpha, d000, d100, d010, d110, d001, d101, d011, d111,
        )
    }
}
pub struct PerlinNoise {
    noise_levels: Vec<Option<ImprovedNoise>>,
    amplitudes: Vec<f64>,
    lowest_freq_value_factor: f64,
    lowest_freq_input_factor: f64,
    max_value: f64,
}
impl PerlinNoise {
    pub fn create(random: &mut dyn RandomSource, first_octave: i32, amplitudes: &[f64]) -> Self {
        let mut noise_levels = Vec::with_capacity(amplitudes.len());
        let positional = random.fork_positional();
        for i in 0..amplitudes.len() {
            if amplitudes[i] != 0.0 {
                let octave = first_octave + i as i32;
                let mut rng = positional.from_hash_of(&format!("octave_{}", octave));
                noise_levels.push(Some(ImprovedNoise::new(rng.as_mut())));
            } else {
                noise_levels.push(None);
            }
        }
        let zero_octave_index = -first_octave;
        let lowest_freq_input_factor = 2.0f64.powi(-zero_octave_index);
        let lowest_freq_value_factor =
            2.0f64.powi(amplitudes.len() as i32 - 1) / (2.0f64.powi(amplitudes.len() as i32) - 1.0);
        let mut slf = Self {
            noise_levels,
            amplitudes: amplitudes.to_vec(),
            lowest_freq_value_factor,
            lowest_freq_input_factor,
            max_value: 0.0,
        };
        slf.max_value = slf.edge_value(2.0);
        slf
    }
    pub fn max_value(&self) -> f64 {
        self.max_value
    }
    fn edge_value(&self, noise_value: f64) -> f64 {
        let mut value = 0.0;
        let mut value_factor = self.lowest_freq_value_factor;
        for i in 0..self.noise_levels.len() {
            if self.noise_levels[i].is_some() {
                value += self.amplitudes[i] * noise_value * value_factor;
            }
            value_factor /= 2.0;
        }
        value
    }
    pub fn get_value(&self, x: f64, y: f64, z: f64) -> f64 {
        self.get_value_scaled(x, y, z, 0.0, 0.0)
    }
    pub fn get_value_scaled(&self, x: f64, y: f64, z: f64, y_scale: f64, y_fudge: f64) -> f64 {
        let mut value = 0.0;
        let mut factor = self.lowest_freq_input_factor;
        let mut value_factor = self.lowest_freq_value_factor;
        for i in 0..self.noise_levels.len() {
            if let Some(ref noise) = self.noise_levels[i] {
                let noise_val = noise.noise_scaled(
                    wrap(x * factor),
                    wrap(y * factor),
                    wrap(z * factor),
                    y_scale * factor,
                    y_fudge * factor,
                );
                value += self.amplitudes[i] * noise_val * value_factor;
            }
            factor *= 2.0;
            value_factor /= 2.0;
        }
        value
    }
}
pub struct NormalNoise {
    value_factor: f64,
    first: PerlinNoise,
    second: PerlinNoise,
    max_value: f64,
}
impl NormalNoise {
    pub fn create(random: &mut dyn RandomSource, first_octave: i32, amplitudes: &[f64]) -> Self {
        let first = PerlinNoise::create(random, first_octave, amplitudes);
        let second = PerlinNoise::create(random, first_octave, amplitudes);
        let mut min_octave = i32::MAX;
        let mut max_octave = i32::MIN;
        for i in 0..amplitudes.len() {
            if amplitudes[i] != 0.0 {
                min_octave = min_octave.min(i as i32);
                max_octave = max_octave.max(i as i32);
            }
        }
        let value_factor = 0.16666666666666666 / expected_deviation(max_octave - min_octave);
        let max_value = (first.max_value() + second.max_value()) * value_factor;
        Self {
            value_factor,
            first,
            second,
            max_value,
        }
    }
    pub fn max_value(&self) -> f64 {
        self.max_value
    }
    pub fn get_value(&self, x: f64, y: f64, z: f64) -> f64 {
        let x2 = x * 1.0181268882175227;
        let y2 = y * 1.0181268882175227;
        let z2 = z * 1.0181268882175227;
        (self.first.get_value(x, y, z) + self.second.get_value(x2, y2, z2)) * self.value_factor
    }
}
fn expected_deviation(octave_span: i32) -> f64 {
    0.1 * (1.0 + 1.0 / (octave_span + 1) as f64)
}
