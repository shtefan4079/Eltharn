@group(0) @binding(0) var src_tex: texture_2d<f32>;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vs_full(@builtin(vertex_index) idx: u32) -> VsOut {
    var p = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    var out: VsOut;
    out.pos = vec4<f32>(p[idx], 0.0, 1.0);
    return out;
}

fn load_clamped(pos: vec4<f32>) -> vec4<f32> {
    let dim = vec2<i32>(textureDimensions(src_tex));
    let last: vec2<i32> = dim - vec2<i32>(1, 1);
    let xy: vec2<i32> = clamp(vec2<i32>(pos.xy), vec2<i32>(0, 0), last);
    return textureLoad(src_tex, xy, 0);
}

fn srgb_to_linear(x: f32) -> f32 {
    let c: f32 = clamp(x, 0.0, 1.0);
    if (c >= 0.04045) {
        return pow((c + 0.055) / 1.055, 2.4);
    }
    return c / 12.92;
}

fn linear_to_srgb(x: f32) -> f32 {
    let c: f32 = clamp(x, 0.0, 1.0);
    if (c >= 0.0031308) {
        return 1.055 * pow(c, 1.0 / 2.4) - 0.055;
    }
    return 12.92 * c;
}

@fragment
fn fs_decode(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let s: vec4<f32> = load_clamped(pos);
    let r: f32 = srgb_to_linear(s.r);
    let g: f32 = srgb_to_linear(s.g);
    let b: f32 = srgb_to_linear(s.b);
    return vec4<f32>(r, g, b, clamp(s.a, 0.0, 1.0));
}

@fragment
fn fs_encode(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let s: vec4<f32> = load_clamped(pos);
    let r: f32 = linear_to_srgb(s.r);
    let g: f32 = linear_to_srgb(s.g);
    let b: f32 = linear_to_srgb(s.b);
    return vec4<f32>(r, g, b, clamp(s.a, 0.0, 1.0));
}
