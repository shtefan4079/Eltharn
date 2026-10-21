







struct GuiUniforms {
    screen_size: vec2<f32>,


    glint_time_millis: f32,
    glint_strength: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: GuiUniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,




    @location(2) marker: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) source_uv: vec2<f32>,
    @location(1) item_uv: vec2<f32>,
    @location(2) source_alpha: f32,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let ndc_x = (model.position.x / uniforms.screen_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (model.position.y / uniforms.screen_size.y) * 2.0;
    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.source_uv = model.tex_coords;



    out.item_uv = model.marker.xy * model.marker.w;
    out.source_alpha = model.marker.z;
    return out;
}



@group(1) @binding(0)
var source_texture: texture_2d<f32>;
@group(1) @binding(1)
var source_sampler: sampler;

@group(2) @binding(0)
var glint_texture: texture_2d<f32>;
@group(2) @binding(1)
var glint_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let source = textureSample(source_texture, source_sampler, in.source_uv);
    let silhouette_alpha = source.a * in.source_alpha;
    if (silhouette_alpha < 0.1) {
        discard;
    }



    let angle = 0.17453292;
    let cosine = cos(angle);
    let sine = sin(angle);
    let rotated = vec2<f32>(
        cosine * in.item_uv.x - sine * in.item_uv.y,
        sine * in.item_uv.x + cosine * in.item_uv.y,
    );
    let offset = vec2<f32>(
        -fract(uniforms.glint_time_millis / 110000.0),
        fract(uniforms.glint_time_millis / 30000.0),
    );
    let glint = textureSample(glint_texture, glint_sampler, rotated * 8.0 + offset);
    if (glint.a < 0.1) {
        discard;
    }



    return vec4<f32>(glint.rgb * uniforms.glint_strength, glint.a * silhouette_alpha);
}
