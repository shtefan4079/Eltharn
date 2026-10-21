struct CameraUniform {
    view_proj: mat4x4<f32>,
    lightmap_factors: vec4<f32>,
    screen_size: vec4<f32>,
    lightmap_effects: vec4<f32>,
    lightmap_block_light_tint: vec4<f32>,
    lightmap_sky_light_color: vec4<f32>,
    lightmap_ambient_color: vec4<f32>,
    lightmap_night_vision_color: vec4<f32>,
    diffuse_light_0: vec4<f32>,
    diffuse_light_1: vec4<f32>,
    cardinal_lighting: vec4<f32>,
    fog_color: vec4<f32>,
    fog_distances: vec4<f32>,
    fog_camera_position: vec4<f32>,
    overlay_color: vec4<f32>,
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

struct VertexOutput {



    @builtin(position) @invariant clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) fog_distances: vec2<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.tex_coords = model.tex_coords;
    out.color = model.color;
    let relative = model.position - camera.fog_camera_position.xyz;
    out.fog_distances = vec2<f32>(length(relative), max(length(relative.xz), abs(relative.y)));
    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@group(2) @binding(0)
var glint_texture: texture_2d<f32>;
@group(2) @binding(1)
var glint_sampler: sampler;

struct ItemGlintUniform {
    time_millis: f32,
    strength: f32,
    uv_scale: f32,



    use_model_color: f32,
};

@group(3) @binding(0)
var<uniform> uniforms: ItemGlintUniform;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {




    let angle = 0.17453292;
    let cosine = cos(angle);
    let sine = sin(angle);
    let rotated = vec2<f32>(
        cosine * in.tex_coords.x - sine * in.tex_coords.y,
        sine * in.tex_coords.x + cosine * in.tex_coords.y,
    );
    let offset = vec2<f32>(
        -fract(uniforms.time_millis / 110000.0),
        fract(uniforms.time_millis / 30000.0),
    );
    let glint = textureSample(glint_texture, glint_sampler, rotated * uniforms.uv_scale + offset);
    if (glint.a < 0.1) {
        discard;
    }
    let model_color = select(vec4<f32>(1.0), in.color, uniforms.use_model_color > 0.5);
    let spherical = clamp((in.fog_distances.x - camera.fog_distances.x) / max(camera.fog_distances.y - camera.fog_distances.x, 0.00001), 0.0, 1.0);
    let cylindrical = clamp((in.fog_distances.y - camera.fog_distances.z) / max(camera.fog_distances.w - camera.fog_distances.z, 0.00001), 0.0, 1.0);


    let fade = (1.0 - max(spherical, cylindrical)) * uniforms.strength;
    return vec4<f32>(glint.rgb * model_color.rgb * fade, glint.a * model_color.a);
}
