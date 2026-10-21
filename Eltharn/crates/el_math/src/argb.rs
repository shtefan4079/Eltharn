use std::sync::LazyLock;

static SRGB_TO_LINEAR: LazyLock<[u16; 256]> = LazyLock::new(|| {
    let mut table = [0u16; 256];
    for i in 0..256 {
        let v = i as f32 / 255.0;
        let val = compute_srgb_to_linear(v);
        table[i] = (val * 1023.0).round() as u16;
    }
    table
});

static LINEAR_TO_SRGB: LazyLock<[u8; 1024]> = LazyLock::new(|| {
    let mut table = [0u8; 1024];
    for i in 0..1024 {
        let v = i as f32 / 1023.0;
        let val = compute_linear_to_srgb(v);
        table[i] = (val * 255.0).round() as u8;
    }
    table
});

fn compute_srgb_to_linear(value: f32) -> f32 {
    if value < 0.04045 {
        value / 12.92
    } else {
        ((value as f64 + 0.055) / 1.055).powf(2.4) as f32
    }
}

fn compute_linear_to_srgb(value: f32) -> f32 {
    if value < 0.0031308 {
        value * 12.92
    } else {
        (1.055 * (value as f64).powf(1.0 / 2.4) - 0.055) as f32
    }
}

pub fn srgb_to_linear_channel(value: i32) -> f32 {
    SRGB_TO_LINEAR[value as usize] as f32 / 1023.0
}

pub fn linear_to_srgb_channel(value: f32) -> i32 {
    let idx = crate::mth::floor_f32(value * 1023.0);
    let idx = idx.clamp(0, 1023) as usize;
    LINEAR_TO_SRGB[idx] as i32
}

pub fn mean_linear(c0: i32, c1: i32, c2: i32, c3: i32) -> i32 {
    let a = (alpha(c0) + alpha(c1) + alpha(c2) + alpha(c3)) / 4;
    let r = linear_channel_mean(red(c0), red(c1), red(c2), red(c3));
    let g = linear_channel_mean(green(c0), green(c1), green(c2), green(c3));
    let b = linear_channel_mean(blue(c0), blue(c1), blue(c2), blue(c3));
    color(a, r, g, b)
}

fn linear_channel_mean(v0: i32, v1: i32, v2: i32, v3: i32) -> i32 {
    let mean = (SRGB_TO_LINEAR[v0 as usize] as i32
        + SRGB_TO_LINEAR[v1 as usize] as i32
        + SRGB_TO_LINEAR[v2 as usize] as i32
        + SRGB_TO_LINEAR[v3 as usize] as i32)
        / 4;
    LINEAR_TO_SRGB[mean as usize] as i32
}

pub const fn alpha(color_val: i32) -> i32 {
    ((color_val as u32) >> 24) as i32
}

pub const fn red(color_val: i32) -> i32 {
    ((color_val >> 16) & 255) as i32
}

pub const fn green(color_val: i32) -> i32 {
    ((color_val >> 8) & 255) as i32
}

pub const fn blue(color_val: i32) -> i32 {
    (color_val & 255) as i32
}

pub const fn color(a: i32, r: i32, g: i32, b: i32) -> i32 {
    (((a & 255) << 24) | ((r & 255) << 16) | ((g & 255) << 8) | (b & 255)) as i32
}

pub const fn color_rgb(r: i32, g: i32, b: i32) -> i32 {
    color(255, r, g, b)
}

pub fn color_from_vec3(vec: crate::vec3::Vec3) -> i32 {
    let r = as_8_bit_channel(vec.x as f32);
    let g = as_8_bit_channel(vec.y as f32);
    let b = as_8_bit_channel(vec.z as f32);
    color_rgb(r, g, b)
}

pub fn multiply(c0: i32, c1: i32) -> i32 {
    if c0 == -1 {
        c1
    } else if c1 == -1 {
        c0
    } else {
        let a = alpha(c0) * alpha(c1) / 255;
        let r = red(c0) * red(c1) / 255;
        let g = green(c0) * green(c1) / 255;
        let b = blue(c0) * blue(c1) / 255;
        color(a, r, g, b)
    }
}

pub fn add_rgb(c0: i32, c1: i32) -> i32 {
    let a = alpha(c0);
    let r = (red(c0) + red(c1)).min(255);
    let g = (green(c0) + green(c1)).min(255);
    let b = (blue(c0) + blue(c1)).min(255);
    color(a, r, g, b)
}

pub fn subtract_rgb(c0: i32, c1: i32) -> i32 {
    let a = alpha(c0);
    let r = (red(c0) - red(c1)).max(0);
    let g = (green(c0) - green(c1)).max(0);
    let b = (blue(c0) - blue(c1)).max(0);
    color(a, r, g, b)
}

pub fn multiply_alpha(col: i32, alpha_val: f32) -> i32 {
    if col == 0 || alpha_val <= 0.0 {
        0
    } else if alpha_val >= 1.0 {
        col
    } else {
        let a = alpha_float(col) * alpha_val;
        color_with_alpha_float(a, col)
    }
}

pub fn scale_rgb(col: i32, factor: f32) -> i32 {
    scale_rgb_channels(col, factor, factor, factor)
}

pub fn scale_rgb_channels(col: i32, r_factor: f32, g_factor: f32, b_factor: f32) -> i32 {
    let a = alpha(col);
    let r = ((red(col) as f32 * r_factor) as i32).clamp(0, 255);
    let g = ((green(col) as f32 * g_factor) as i32).clamp(0, 255);
    let b = ((blue(col) as f32 * b_factor) as i32).clamp(0, 255);
    color(a, r, g, b)
}

pub fn scale_rgb_int(col: i32, scale: i32) -> i32 {
    let a = alpha(col);
    let r = (((red(col) as i64 * scale as i64) / 255) as i32).clamp(0, 255);
    let g = (((green(col) as i64 * scale as i64) / 255) as i32).clamp(0, 255);
    let b = (((blue(col) as i64 * scale as i64) / 255) as i32).clamp(0, 255);
    color(a, r, g, b)
}

pub fn greyscale(col: i32) -> i32 {
    let r = red(col) as f32 * 0.3;
    let g = green(col) as f32 * 0.59;
    let b = blue(col) as f32 * 0.11;
    let gray_val = (r + g + b) as i32;
    color(alpha(col), gray_val, gray_val, gray_val)
}

pub fn alpha_blend(col0: i32, col1: i32) -> i32 {
    let a0 = alpha(col0);
    let a1 = alpha(col1);
    if a1 == 255 {
        col1
    } else if a1 == 0 {
        col0
    } else {
        let a = a1 + a0 * (255 - a1) / 255;
        let r = alpha_blend_channel(a, a1, red(col0), red(col1));
        let g = alpha_blend_channel(a, a1, green(col0), green(col1));
        let b = alpha_blend_channel(a, a1, blue(col0), blue(col1));
        color(a, r, g, b)
    }
}

fn alpha_blend_channel(a: i32, a1: i32, c0: i32, c1: i32) -> i32 {
    (c1 * a1 + c0 * (a - a1)) / a
}

pub fn srgb_lerp(t: f32, col0: i32, col1: i32) -> i32 {
    let a = crate::mth::lerp_int(t, alpha(col0), alpha(col1));
    let r = crate::mth::lerp_int(t, red(col0), red(col1));
    let g = crate::mth::lerp_int(t, green(col0), green(col1));
    let b = crate::mth::lerp_int(t, blue(col0), blue(col1));
    color(a, r, g, b)
}

pub fn linear_lerp(t: f32, col0: i32, col1: i32) -> i32 {
    let a = crate::mth::lerp_int(t, alpha(col0), alpha(col1));
    let r = LINEAR_TO_SRGB[crate::mth::lerp_int(
        t,
        SRGB_TO_LINEAR[red(col0) as usize] as i32,
        SRGB_TO_LINEAR[red(col1) as usize] as i32,
    ) as usize] as i32;
    let g = LINEAR_TO_SRGB[crate::mth::lerp_int(
        t,
        SRGB_TO_LINEAR[green(col0) as usize] as i32,
        SRGB_TO_LINEAR[green(col1) as usize] as i32,
    ) as usize] as i32;
    let b = LINEAR_TO_SRGB[crate::mth::lerp_int(
        t,
        SRGB_TO_LINEAR[blue(col0) as usize] as i32,
        SRGB_TO_LINEAR[blue(col1) as usize] as i32,
    ) as usize] as i32;
    color(a, r, g, b)
}

pub const fn opaque(color_val: i32) -> i32 {
    color_val | -16777216
}

pub const fn transparent(color_val: i32) -> i32 {
    color_val & 16777215
}

pub const fn color_with_alpha(a: i32, rgb: i32) -> i32 {
    ((a & 255) << 24) | (rgb & 16777215)
}

pub fn color_with_alpha_float(a: f32, rgb: i32) -> i32 {
    ((as_8_bit_channel(a) & 255) << 24) | (rgb & 16777215)
}

pub fn white_float(a: f32) -> i32 {
    (as_8_bit_channel(a) << 24) | 16777215
}

pub const fn white_int(a: i32) -> i32 {
    ((a & 255) << 24) | 16777215
}

pub fn black_float(a: f32) -> i32 {
    as_8_bit_channel(a) << 24
}

pub const fn black_int(a: i32) -> i32 {
    (a & 255) << 24
}

pub fn gray(a: f32) -> i32 {
    let g = as_8_bit_channel(a);
    color_rgb(g, g, g)
}

pub fn color_from_float(a: f32, r: f32, g: f32, b: f32) -> i32 {
    color(
        as_8_bit_channel(a),
        as_8_bit_channel(r),
        as_8_bit_channel(g),
        as_8_bit_channel(b),
    )
}

pub fn vector3f_from_rgb24(color_val: i32) -> [f32; 3] {
    [
        red_float(color_val),
        green_float(color_val),
        blue_float(color_val),
    ]
}

pub fn vector4f_from_argb32(color_val: i32) -> [f32; 4] {
    [
        red_float(color_val),
        green_float(color_val),
        blue_float(color_val),
        alpha_float(color_val),
    ]
}

pub fn average(col0: i32, col1: i32) -> i32 {
    let a = (alpha(col0) + alpha(col1)) / 2;
    let r = (red(col0) + red(col1)) / 2;
    let g = (green(col0) + green(col1)) / 2;
    let b = (blue(col0) + blue(col1)) / 2;
    color(a, r, g, b)
}

pub fn as_8_bit_channel(val: f32) -> i32 {
    crate::mth::floor_f32(val * 255.0)
}

pub fn alpha_float(color_val: i32) -> f32 {
    from_8_bit_channel(alpha(color_val))
}

pub fn red_float(color_val: i32) -> f32 {
    from_8_bit_channel(red(color_val))
}

pub fn green_float(color_val: i32) -> f32 {
    from_8_bit_channel(green(color_val))
}

pub fn blue_float(color_val: i32) -> f32 {
    from_8_bit_channel(blue(color_val))
}

fn from_8_bit_channel(val: i32) -> f32 {
    val as f32 / 255.0
}

pub fn to_abgr(color_val: i32) -> i32 {
    let color_u32 = color_val as u32;
    ((color_u32 & 0xFF00FF00) | ((color_u32 & 0x00FF0000) >> 16) | ((color_u32 & 0x000000FF) << 16))
        as i32
}

pub fn from_abgr(color_val: i32) -> i32 {
    to_abgr(color_val)
}

pub fn set_brightness(color_val: i32, brightness: f32) -> i32 {
    let r = red(color_val);
    let g = green(color_val);
    let b = blue(color_val);
    let a = alpha(color_val);

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = (max - min) as f32;

    let saturation = if max != 0 { delta / max as f32 } else { 0.0 };

    let hue = if saturation == 0.0 {
        0.0
    } else {
        let r_dist = (max - r) as f32 / delta;
        let g_dist = (max - g) as f32 / delta;
        let b_dist = (max - b) as f32 / delta;

        let mut h = if r == max {
            b_dist - g_dist
        } else if g == max {
            2.0 + r_dist - b_dist
        } else {
            4.0 + g_dist - r_dist
        };

        h /= 6.0;
        if h < 0.0 {
            h += 1.0;
        }
        h
    };

    if saturation == 0.0 {
        let gray = (brightness * 255.0).round() as i32;
        color(a, gray, gray, gray)
    } else {
        let h_val = (hue - hue.floor()) * 6.0;
        let f = h_val - h_val.floor();
        let p = brightness * (1.0 - saturation);
        let q = brightness * (1.0 - saturation * f);
        let t = brightness * (1.0 - saturation * (1.0 - f));

        let (r_f, g_f, b_f) = match h_val.floor() as i32 {
            0 => (brightness, t, p),
            1 => (q, brightness, p),
            2 => (p, brightness, t),
            3 => (p, q, brightness),
            4 => (t, p, brightness),
            5 => (brightness, p, q),
            _ => (brightness, p, q),
        };

        let r_out = (r_f * 255.0).round() as i32;
        let g_out = (g_f * 255.0).round() as i32;
        let b_out = (b_f * 255.0).round() as i32;

        color(a, r_out, g_out, b_out)
    }
}
