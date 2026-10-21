struct CompositeUniforms {
    texel_size: vec2<f32>,
    blur_radius: f32,
    underwater_amount: f32,
    underwater_color: vec4<f32>,
    blur_direction: vec2<f32>,
    _padding: vec2<f32>,
};

@group(0) @binding(0)
var scene_texture: texture_2d<f32>;
@group(0) @binding(1)
var scene_sampler: sampler;
@group(0) @binding(2)
var<uniform> uniforms: CompositeUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(3.0, 1.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 2.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(2.0, 0.0),
    );

    var out: VertexOutput;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = uvs[vertex_index];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let actual_radius = round(uniforms.blur_radius);
    var blurred = vec4<f32>(0.0);
    var samples = 0.0;
    var offset = -actual_radius + 0.5;
    loop {
        if (offset > actual_radius) {
            break;
        }
        let uv_offset = uniforms.texel_size * uniforms.blur_direction * offset;
        blurred = blurred + textureSample(scene_texture, scene_sampler, in.uv + uv_offset);
        samples = samples + 1.0;
        offset = offset + 2.0;
    }
    let endpoint_offset = uniforms.texel_size * uniforms.blur_direction * actual_radius;
    blurred = blurred + textureSample(scene_texture, scene_sampler, in.uv + endpoint_offset) * 0.5;
    return blurred / (samples + 0.5);
}
