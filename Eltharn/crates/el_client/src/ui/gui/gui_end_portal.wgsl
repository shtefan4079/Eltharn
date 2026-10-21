

struct GuiUniforms {
    screen_size: vec2<f32>,
    glint_time_millis: f32,
    glint_strength: f32,
};
@group(0) @binding(0) var<uniform> uniforms: GuiUniforms;
@group(1) @binding(0) var sky: texture_2d<f32>;
@group(1) @binding(1) var sky_sampler: sampler;
@group(2) @binding(0) var portal: texture_2d<f32>;
@group(2) @binding(1) var portal_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
};
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) projected: vec4<f32>,
    @location(1) game_time: f32,
};
@vertex fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let ndc = vec2<f32>(input.position.x / uniforms.screen_size.x * 2.0 - 1.0,
        1.0 - input.position.y / uniforms.screen_size.y * 2.0);
    output.position = vec4<f32>(ndc, 0.0, 1.0);

    output.projected = vec4<f32>(ndc * 0.5 + vec2<f32>(1.0), 0.0, 1.0);
    output.game_time = input.color.x;
    return output;
}

const COLORS = array<vec3<f32>, 15>(
    vec3<f32>(0.022087, 0.098399, 0.110818),
    vec3<f32>(0.011892, 0.095924, 0.089485),
    vec3<f32>(0.027636, 0.101689, 0.100326),
    vec3<f32>(0.046564, 0.109883, 0.114838),
    vec3<f32>(0.064901, 0.117696, 0.097189),
    vec3<f32>(0.063761, 0.086895, 0.123646),
    vec3<f32>(0.084817, 0.111994, 0.166380),
    vec3<f32>(0.097489, 0.154120, 0.091064),
    vec3<f32>(0.106152, 0.131144, 0.195191),
    vec3<f32>(0.097721, 0.110188, 0.187229),
    vec3<f32>(0.133516, 0.138278, 0.148582),
    vec3<f32>(0.070006, 0.243332, 0.235792),
    vec3<f32>(0.196766, 0.142899, 0.214696),
    vec3<f32>(0.047281, 0.315338, 0.321970),
    vec3<f32>(0.204675, 0.390010, 0.302066),
);

fn end_portal_layer(layer: f32, time: f32) -> mat4x4<f32> {
    let translate = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 17.0 / layer),
        vec4<f32>(0.0, 1.0, 0.0, (2.0 + layer / 1.5) * time * 1.5),
        vec4<f32>(0.0, 0.0, 1.0, 0.0), vec4<f32>(0.0, 0.0, 0.0, 1.0));
    let angle = radians((layer * layer * 4321.0 + layer * 9.0) * 2.0);
    let s = (4.5 - layer / 4.0) * 2.0;

    let rotation_scale = mat4x4<f32>(
        vec4<f32>(cos(angle) * s, sin(angle) * s, 0.0, 0.0),
        vec4<f32>(-sin(angle) * s, cos(angle) * s, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0), vec4<f32>(0.0, 0.0, 0.0, 1.0));
    let scale_translate = mat4x4<f32>(
        vec4<f32>(0.5, 0.0, 0.0, 0.25), vec4<f32>(0.0, 0.5, 0.0, 0.25),
        vec4<f32>(0.0, 0.0, 1.0, 0.0), vec4<f32>(0.0, 0.0, 0.0, 1.0));
    return rotation_scale * translate * scale_translate;
}

@fragment fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(sky, sky_sampler, input.projected.xy / input.projected.w).rgb * COLORS[0];

    var colors = COLORS;
    for (var i = 0u; i < 15u; i = i + 1u) {
        let projected = input.projected * end_portal_layer(f32(i + 1u), input.game_time);
        color += textureSample(portal, portal_sampler, projected.xy / projected.w).rgb * colors[i];
    }
    return vec4<f32>(color, 1.0);
}
