@group(0) @binding(0) var dtex: texture_depth_2d;

@vertex
fn vs_full(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var p = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(p[idx], 0.0, 1.0);
}

@fragment
fn fs_copy(@builtin(position) pos: vec4<f32>) -> @builtin(frag_depth) f32 {
    let dim = vec2<i32>(textureDimensions(dtex));
    let last: vec2<i32> = dim - vec2<i32>(1, 1);
    let xy: vec2<i32> = clamp(vec2<i32>(pos.xy), vec2<i32>(0, 0), last);
    return textureLoad(dtex, xy, 0);
}
