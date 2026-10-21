struct CameraUniform {
    view_proj: mat4x4<f32>,
    lightmap_factors: vec4<f32>,
    screen_size: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) normal: vec4<f32>,
    @location(3) line_width: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;









    let line_pos_start = camera.view_proj * vec4<f32>(model.position, 1.0);


    let line_pos_end = camera.view_proj * vec4<f32>(model.position + model.normal.xyz, 1.0);
    let ndc1 = line_pos_start.xyz / line_pos_start.w;
    let ndc2 = line_pos_end.xyz / line_pos_end.w;
    let line_screen_direction = normalize(
        (ndc2.xy - ndc1.xy) * camera.screen_size.xy
    );
    var line_offset = vec2<f32>(-line_screen_direction.y, line_screen_direction.x)
        * model.line_width / camera.screen_size.xy;
    if line_offset.x < 0.0 {
        line_offset = -line_offset;
    }
    let even_vertex = (vertex_index % 2u) == 0u;
    let ndc = select(
        ndc1 - vec3<f32>(line_offset, 0.0),
        ndc1 + vec3<f32>(line_offset, 0.0),
        even_vertex,
    );
    out.clip_position = vec4<f32>(ndc * line_pos_start.w, line_pos_start.w);
    out.color = model.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
