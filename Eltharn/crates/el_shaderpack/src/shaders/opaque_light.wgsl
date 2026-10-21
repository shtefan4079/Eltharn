struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) color: vec4<f32>,
    @location(4) light: vec2<f32>,
};

struct GeometryVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) light_uv: vec2<f32>,
};

@vertex
fn vs_geometry(model: VertexInput) -> GeometryVertexOutput {
    var out: GeometryVertexOutput;
    out.tex_coords = model.tex_coords;
    out.normal = model.normal;
    out.color = model.color;
    out.light_uv = model.light;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct GeometryFragment {
    @location(0) normal: vec4<f32>,
    @location(1) albedo: vec4<f32>,
    @location(2) light_uv: vec2<f32>,
};



fn sample_terrain_nearest(uv_in: vec2<f32>) -> vec4<f32> {
    let pixel_size = 1.0 / vec2<f32>(textureDimensions(t_diffuse, 0));
    let du = dpdx(uv_in);
    let dv = dpdy(uv_in);
    let texel_screen_size = sqrt(du * du + dv * dv);
    let uv_texel_coords = uv_in / pixel_size;
    let texel_center = round(uv_texel_coords) - vec2<f32>(0.5);
    var texel_offset = uv_texel_coords - texel_center;
    texel_offset = (texel_offset - vec2<f32>(0.5)) * pixel_size / texel_screen_size + vec2<f32>(0.5);
    texel_offset = clamp(texel_offset, vec2<f32>(0.0), vec2<f32>(1.0));
    return textureSampleGrad(t_diffuse, s_diffuse, (texel_center + texel_offset) * pixel_size, du, dv);
}

@fragment
fn fs_geometry_normal(in: GeometryVertexOutput) -> GeometryFragment {



    let sampled = sample_terrain_nearest(in.tex_coords) * in.color;
    if (sampled.a < 0.5) {
        discard;
    }


    var output: GeometryFragment;
    output.albedo = vec4<f32>(sampled.rgb, 1.0);
    output.light_uv = in.light_uv;
    output.normal = vec4<f32>(0.5, 1.0, 0.5, 1.0);
    if (dot(in.normal, in.normal) >= 1e-6) {
        output.normal = vec4<f32>(normalize(in.normal) * 0.5 + vec3<f32>(0.5), 1.0);
    }
    return output;
}
