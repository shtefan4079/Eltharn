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
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) fog_spherical: f32,
    @location(3) fog_cylindrical: f32,
};


fn linear_fog_value(vertex_distance: f32, fog_start: f32, fog_end: f32) -> f32 {
    if (vertex_distance <= fog_start) {
        return 0.0;
    }
    if (vertex_distance >= fog_end) {
        return 1.0;
    }
    return (vertex_distance - fog_start) / (fog_end - fog_start);
}

fn apply_fog(in_color: vec4<f32>, spherical: f32, cylindrical: f32) -> vec4<f32> {
    let fog_value = max(
        linear_fog_value(spherical, camera.fog_distances.x, camera.fog_distances.y),
        linear_fog_value(cylindrical, camera.fog_distances.z, camera.fog_distances.w),
    );
    return vec4<f32>(
        mix(in_color.rgb, camera.fog_color.rgb, fog_value * camera.fog_color.a),
        in_color.a,
    );
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.tex_coords = model.tex_coords;
    out.color = model.color;
    let relative = model.position - camera.fog_camera_position.xyz;
    out.fog_spherical = length(relative);
    out.fog_cylindrical = max(length(relative.xz), abs(relative.y));
    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;




@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords) * in.color;
    if (color.a < 0.1) {
        discard;
    }
    return apply_fog(color, in.fog_spherical, in.fog_cylindrical);
}
