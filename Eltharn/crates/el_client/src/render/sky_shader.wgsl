struct CameraUniform {
    view_proj: mat4x4<f32>,
    lightmap_factors: vec4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct SkyUniforms {
    model_matrix: mat4x4<f32>,
    color_modifier: vec4<f32>,
    fog_color: vec4<f32>,
    fog_params: vec4<f32>,
};
@group(2) @binding(0)
var<uniform> sky_uniforms: SkyUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) color: vec4<f32>,
    @location(4) light: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) view_space_pos: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.color = model.color * sky_uniforms.color_modifier;
    out.view_space_pos = model.position;
    let world_pos = sky_uniforms.model_matrix * vec4<f32>(model.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;
    return out;
}

fn linear_fog_value(dist: f32, start: f32, end: f32) -> f32 {
    if (dist <= start) {
        return 0.0;
    }
    if (dist >= end) {
        return 1.0;
    }
    return (dist - start) / (end - start);
}

@fragment
fn fs_untextured(in: VertexOutput) -> @location(0) vec4<f32> {
    var final_color = in.color;


    if (sky_uniforms.fog_params.z > 0.5) {
        let spherical_dist = length(in.view_space_pos);
        let cylindrical_dist = max(length(in.view_space_pos.xz), abs(in.view_space_pos.y));
        let fog_start = sky_uniforms.fog_params.x;
        let fog_end = sky_uniforms.fog_params.y;

        let spherical_fog = linear_fog_value(spherical_dist, fog_start, fog_end);
        let cylindrical_fog = select(0.0, 1.0, cylindrical_dist >= fog_end);
        let fog_factor = max(spherical_fog, cylindrical_fog);

        final_color = vec4<f32>(mix(final_color.rgb, sky_uniforms.fog_color.rgb, fog_factor * sky_uniforms.fog_color.a), final_color.a);
    }

    return final_color;
}

@fragment
fn fs_textured(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    if (object_color.a < 0.1) {
        discard;
    }
    return object_color * in.color;
}
