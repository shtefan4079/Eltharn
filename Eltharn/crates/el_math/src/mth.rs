pub const EPSILON: f32 = 1.0e-5;
const SIN_TABLE_SIZE: usize = 65_536;
const SIN_TABLE_MASK: i64 = 65_535;
const SIN_SCALE: f64 = 10_430.378_350_470_453;
const COS_OFFSET: f64 = 16_384.0;
fn java_sin_table() -> &'static [f32; SIN_TABLE_SIZE] {
    static TABLE: std::sync::OnceLock<Box<[f32; SIN_TABLE_SIZE]>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = Vec::with_capacity(SIN_TABLE_SIZE);
        for index in 0..SIN_TABLE_SIZE {
            table.push(((index as f64) / SIN_SCALE).sin() as f32);
        }
        table.into_boxed_slice().try_into().unwrap()
    })
}
#[inline]
fn java_sin_index(value: f32, offset: f64) -> usize {
    (((f64::from(value) * SIN_SCALE + offset) as i64) & SIN_TABLE_MASK) as usize
}
#[inline]
pub fn sin(value: f32) -> f32 {
    java_sin_table()[java_sin_index(value, 0.0)]
}
#[inline]
pub fn cos(value: f32) -> f32 {
    java_sin_table()[java_sin_index(value, COS_OFFSET)]
}
pub fn equal_f32(a: f32, b: f32) -> bool {
    (b - a).abs() < EPSILON
}
pub fn equal_f64(a: f64, b: f64) -> bool {
    (b - a).abs() < EPSILON as f64
}
pub fn floor_f32(value: f32) -> i32 {
    value.floor() as i32
}
pub fn floor_f64(value: f64) -> i32 {
    value.floor() as i32
}
pub fn lfloor_f64(value: f64) -> i64 {
    value.floor() as i64
}
pub fn clamp_i32(value: i32, min: i32, max: i32) -> i32 {
    std::cmp::min(std::cmp::max(value, min), max)
}
pub fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    std::cmp::min(std::cmp::max(value, min), max)
}
pub fn clamp_f32(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else {
        java_min_f32(value, max)
    }
}
pub fn clamp_f64(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else {
        java_min_f64(value, max)
    }
}
pub fn get_seed(x: i32, y: i32, z: i32) -> i64 {
    let x_part = x.wrapping_mul(3_129_871) as i64;
    let z_part = (z as i64).wrapping_mul(116_129_781);
    let seed = x_part ^ z_part ^ y as i64;
    seed.wrapping_mul(seed)
        .wrapping_mul(42_317_861)
        .wrapping_add(seed.wrapping_mul(11))
        >> 16
}
fn java_min_f32(left: f32, right: f32) -> f32 {
    if left.is_nan() || right.is_nan() {
        f32::NAN
    } else if left == 0.0 && right == 0.0 {
        if left.is_sign_negative() || right.is_sign_negative() {
            -0.0
        } else {
            0.0
        }
    } else if left <= right {
        left
    } else {
        right
    }
}
pub fn java_min_f64(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left == 0.0 && right == 0.0 {
        if left.is_sign_negative() || right.is_sign_negative() {
            -0.0
        } else {
            0.0
        }
    } else if left <= right {
        left
    } else {
        right
    }
}
pub fn java_max_f64(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left == 0.0 && right == 0.0 {
        if left.is_sign_positive() || right.is_sign_positive() {
            0.0
        } else {
            -0.0
        }
    } else if left >= right {
        left
    } else {
        right
    }
}
pub fn abs_f32(value: f32) -> f32 {
    value.abs()
}
pub fn abs_i32(value: i32) -> i32 {
    value.abs()
}
pub fn lerp_f32(alpha: f32, p0: f32, p1: f32) -> f32 {
    p0 + alpha * (p1 - p0)
}
pub fn lerp_f64(alpha: f64, p0: f64, p1: f64) -> f64 {
    p0 + alpha * (p1 - p0)
}
pub fn lerp_int(t: f32, start: i32, end: i32) -> i32 {
    start + floor_f32(t * (end - start) as f32)
}
