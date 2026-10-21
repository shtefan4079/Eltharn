struct Shadow { view_projection:mat4x4<f32>, light_direction:vec4<f32>, map:vec4<f32> }
@group(0) @binding(0) var<uniform> shadow:Shadow;
@group(0) @binding(1) var atlas:texture_2d<f32>;
@group(0) @binding(2) var linear:sampler;
struct Out { @builtin(position) position:vec4<f32>, @location(0) uv:vec2<f32>, @location(1) alpha:f32 }
@vertex fn vs(@location(0) position:vec3<f32>,@location(1) uv:vec2<f32>,@location(3) color:vec4<f32>)->Out {
    var out:Out;out.position=shadow.view_projection*vec4<f32>(position,1.);out.uv=uv;out.alpha=color.a;return out;
}



fn sample_terrain_nearest(uv_in:vec2<f32>)->vec4<f32> {
    let pixel_size = 1.0 / vec2<f32>(textureDimensions(atlas, 0));
    let du = dpdx(uv_in);
    let dv = dpdy(uv_in);
    let texel_screen_size = sqrt(du * du + dv * dv);
    let uv_texel_coords = uv_in / pixel_size;
    let texel_center = round(uv_texel_coords) - vec2<f32>(0.5);
    var texel_offset = uv_texel_coords - texel_center;
    texel_offset = (texel_offset - vec2<f32>(0.5)) * pixel_size / texel_screen_size + vec2<f32>(0.5);
    texel_offset = clamp(texel_offset, vec2<f32>(0.0), vec2<f32>(1.0));
    return textureSampleGrad(atlas, linear, (texel_center + texel_offset) * pixel_size, du, dv);
}
@fragment fn fs(in:Out) {


    if sample_terrain_nearest(in.uv).a*in.alpha<0.5 { discard; }
}
