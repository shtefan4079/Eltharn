

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


fn linear_fog_value(vertex_distance: f32, fog_start: f32, fog_end: f32) -> f32 {
    if (vertex_distance <= fog_start) {
        return 0.0;
    }
    if (vertex_distance >= fog_end) {
        return 1.0;
    }
    return (vertex_distance - fog_start) / (fog_end - fog_start);
}

fn total_fog_value(spherical: f32, cylindrical: f32) -> f32 {
    return max(
        linear_fog_value(spherical, camera.fog_distances.x, camera.fog_distances.y),
        linear_fog_value(cylindrical, camera.fog_distances.z, camera.fog_distances.w),
    );
}



fn apply_fog(in_color: vec4<f32>, spherical: f32, cylindrical: f32) -> vec4<f32> {
    let fog_value = total_fog_value(spherical, cylindrical);
    return vec4<f32>(
        mix(in_color.rgb, camera.fog_color.rgb, fog_value * camera.fog_color.a),
        in_color.a,
    );
}

fn fog_spherical_distance(world_position: vec3<f32>) -> f32 {
    return length(world_position - camera.fog_camera_position.xyz);
}

fn fog_cylindrical_distance(world_position: vec3<f32>) -> f32 {
    let relative = world_position - camera.fog_camera_position.xyz;
    return max(length(relative.xz), abs(relative.y));
}

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
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,




    @location(3) lightmap: vec3<f32>,


    @location(4) fog_spherical: f32,
    @location(5) fog_cylindrical: f32,
};






struct TerrainVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) fog_spherical: f32,
    @location(4) fog_cylindrical: f32,
};

fn terrain_directional_lighting(normal: vec3<f32>) -> f32 {












    if (normal.y > 0.5) {
        return camera.cardinal_lighting.y;
    }
    if (normal.y < -0.5) {
        return camera.cardinal_lighting.x;
    }
    if (abs(normal.z) > 0.5) {
        return camera.cardinal_lighting.z;
    }
    if (abs(normal.x) > 0.5) {
        return camera.cardinal_lighting.w;
    }
    return 1.0;
}











fn java_entity_diffuse_lighting(normal: vec3<f32>) -> f32 {
    let minecraft_light_power = 0.6;
    let minecraft_ambient_light = 0.4;

    if (dot(normal, normal) < 1e-6) {
        return 1.0;
    }
    let unit_normal = normalize(normal);
    let light = max(
        vec2<f32>(0.0),
        vec2<f32>(
            dot(camera.diffuse_light_0.xyz, unit_normal),
            dot(camera.diffuse_light_1.xyz, unit_normal),
        ),
    );
    return min(
        1.0,
        (light.x + light.y) * minecraft_light_power + minecraft_ambient_light,
    );
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.normal = model.normal;
    out.color = model.color;





    var light_uv = model.light;
    if (all(camera.lightmap_factors.zw >= vec2<f32>(0.0))) {
        light_uv = camera.lightmap_factors.zw;
    }
    out.lightmap = sample_java_lightmap(light_uv);
    out.fog_spherical = fog_spherical_distance(model.position);
    out.fog_cylindrical = fog_cylindrical_distance(model.position);
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}

@vertex
fn vs_terrain(
    model: VertexInput,
) -> TerrainVertexOutput {
    var out: TerrainVertexOutput;
    out.tex_coords = model.tex_coords;
    out.normal = model.normal;
    let lightmap = sample_java_lightmap(model.light);
    let directional = terrain_directional_lighting(model.normal);




    out.vertex_color = vec4<f32>(model.color.rgb * directional * lightmap, model.color.a);
    out.fog_spherical = fog_spherical_distance(model.position);
    out.fog_cylindrical = fog_cylindrical_distance(model.position);
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}











@vertex
fn vs_moving_block(
    model: VertexInput,
) -> TerrainVertexOutput {
    var out: TerrainVertexOutput;
    out.tex_coords = model.tex_coords;
    let lightmap = sample_java_lightmap(model.light);
    let directional = terrain_directional_lighting(model.normal);
    out.vertex_color = vec4<f32>(
        model.color.rgb * directional * lightmap,
        model.color.a,
    );
    out.fog_spherical = fog_spherical_distance(model.position);
    out.fog_cylindrical = fog_cylindrical_distance(model.position);
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;









fn terrain_atlas_pixel_size() -> vec2<f32> {
    let size = textureDimensions(t_diffuse, 0);
    return 1.0 / vec2<f32>(f32(size.x), f32(size.y));
}

fn sample_terrain_nearest(uv_in: vec2<f32>) -> vec4<f32> {
    let pixel_size = terrain_atlas_pixel_size();
    let du = dpdx(uv_in);
    let dv = dpdy(uv_in);
    let texel_screen_size = sqrt(du * du + dv * dv);

    let uv_texel_coords = uv_in / pixel_size;
    let texel_center = round(uv_texel_coords) - vec2<f32>(0.5);
    var texel_offset = uv_texel_coords - texel_center;

    texel_offset = (texel_offset - vec2<f32>(0.5)) * pixel_size / texel_screen_size + vec2<f32>(0.5);
    texel_offset = clamp(texel_offset, vec2<f32>(0.0), vec2<f32>(1.0));

    let uv = (texel_center + texel_offset) * pixel_size;
    return textureSampleGrad(t_diffuse, s_diffuse, uv, du, dv);
}











fn java_lightmap_brightness(level: f32) -> f32 {
    return level / (4.0 - 3.0 * level);
}





fn java_lightmap_not_gamma(color: vec3<f32>) -> vec3<f32> {
    let max_component = max(max(color.x, color.y), color.z);
    let max_inverted = 1.0 - max_component;
    let max_scaled = 1.0 - max_inverted * max_inverted * max_inverted * max_inverted;
    return color * (max_scaled / max_component);
}

fn java_lightmap_texel(block_level: f32, sky_level: f32) -> vec3<f32> {




    let block_brightness = java_lightmap_brightness(block_level) * camera.lightmap_factors.y;
    let sky_brightness = java_lightmap_brightness(sky_level) * camera.lightmap_factors.x;

    let night_vision_color = camera.lightmap_night_vision_color.rgb * camera.lightmap_effects.x;
    var color = max(camera.lightmap_ambient_color.rgb, night_vision_color);
    color += camera.lightmap_sky_light_color.rgb * sky_brightness;

    let block_parabolic = (2.0 * block_level - 1.0) * (2.0 * block_level - 1.0);
    let block_light_color = mix(
        camera.lightmap_block_light_tint.rgb,
        vec3<f32>(1.0),
        0.9 * block_parabolic,
    );
    color += block_light_color * block_brightness;

    color = mix(
        color,
        color * vec3<f32>(0.7, 0.6, 0.6),
        camera.lightmap_effects.z,
    );
    color -= vec3<f32>(camera.lightmap_effects.y);
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    color = mix(
        color,
        java_lightmap_not_gamma(color),
        camera.lightmap_effects.w,
    );




    return floor(color * 255.0 + vec3<f32>(0.5)) / 255.0;
}

fn sample_java_lightmap(packed_uv: vec2<f32>) -> vec3<f32> {




    let cell = clamp(packed_uv * 16.0, vec2<f32>(0.0), vec2<f32>(15.0));
    let lower = floor(cell);
    let upper = min(lower + vec2<f32>(1.0), vec2<f32>(15.0));
    let fraction = fract(cell);

    let c00 = java_lightmap_texel(lower.x / 15.0, lower.y / 15.0);
    let c10 = java_lightmap_texel(upper.x / 15.0, lower.y / 15.0);
    let c01 = java_lightmap_texel(lower.x / 15.0, upper.y / 15.0);
    let c11 = java_lightmap_texel(upper.x / 15.0, upper.y / 15.0);
    return mix(mix(c00, c10, fraction.x), mix(c01, c11, fraction.x), fraction.y);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color = sample_terrain_nearest(in.tex_coords);




    let tinted_color = object_color * in.color;



    if (tinted_color.a < 0.5) {
        discard;
    }


    let lighting = terrain_directional_lighting(in.normal);




    let lightmap = in.lightmap;










    return vec4<f32>(tinted_color.xyz * lighting * lightmap, 1.0);
}



@fragment
fn fs_particle(in: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(t_diffuse, s_diffuse, in.tex_coords) * in.color;
    if (sampled.a < 0.1) {
        discard;
    }
    let lit = vec4<f32>(sampled.rgb * in.lightmap, sampled.a);
    return apply_fog(lit, in.fog_spherical, in.fog_cylindrical);
}




@fragment
fn fs_weather(in: VertexOutput) -> @location(0) vec4<f32> {




    let sampled = textureSample(t_diffuse, s_diffuse, in.tex_coords) * in.color;
    let lit = vec4<f32>(sampled.rgb * in.lightmap, sampled.a);
    if (lit.a < 0.1) {
        discard;
    }
    return apply_fog(lit, in.fog_spherical, in.fog_cylindrical);
}





@fragment
fn fs_item_frame(in: VertexOutput) -> @location(0) vec4<f32> {


    let sampled = sample_terrain_nearest(in.tex_coords);
    if (sampled.a < 0.1) { discard; }
    let tinted = sampled * in.color;
    let lit_rgb = tinted.rgb * java_entity_diffuse_lighting(in.normal);
    let overlaid = mix(camera.overlay_color.rgb, lit_rgb, camera.overlay_color.a);
    return apply_fog(vec4<f32>(overlaid * in.lightmap, tinted.a), in.fog_spherical, in.fog_cylindrical);
}

@fragment
fn fs_entity_solid(in: VertexOutput) -> @location(0) vec4<f32> {

    let tinted_color = sample_terrain_nearest(in.tex_coords) * in.color;
    let lit_rgb = tinted_color.rgb * java_entity_diffuse_lighting(in.normal);
    let overlaid = mix(camera.overlay_color.rgb, lit_rgb, camera.overlay_color.a);
    return apply_fog(vec4<f32>(overlaid * in.lightmap, tinted_color.a), in.fog_spherical, in.fog_cylindrical);
}

@fragment
fn fs_entity(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color = sample_terrain_nearest(in.tex_coords);
    let tinted_color = object_color * in.color;


    if (tinted_color.a < 0.1) {
        discard;
    }
    let lighting = java_entity_diffuse_lighting(in.normal);



    let lit_rgb = tinted_color.xyz * lighting;
    let overlaid = mix(camera.overlay_color.rgb, lit_rgb, camera.overlay_color.a);
    let lit = vec4<f32>(overlaid * in.lightmap, 1.0);
    return apply_fog(lit, in.fog_spherical, in.fog_cylindrical);
}

@fragment
fn fs_terrain(in: TerrainVertexOutput) -> @location(0) vec4<f32> {
    let tinted_color = sample_terrain_nearest(in.tex_coords) * in.vertex_color;


    if (tinted_color.a < 0.5) {
        discard;
    }
    return apply_fog(
        vec4<f32>(tinted_color.rgb, 1.0),
        in.fog_spherical,
        in.fog_cylindrical,
    );
}






@fragment
fn fs_moving_block_solid(in: TerrainVertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords) * in.vertex_color;
    return apply_fog(color, in.fog_spherical, in.fog_cylindrical);
}

@fragment
fn fs_moving_block_cutout(in: TerrainVertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords) * in.vertex_color;
    if (color.a < 0.5) {
        discard;
    }
    return apply_fog(color, in.fog_spherical, in.fog_cylindrical);
}

@fragment
fn fs_moving_block_translucent(in: TerrainVertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords) * in.vertex_color;
    if (color.a < 0.1) {
        discard;
    }
    return apply_fog(color, in.fog_spherical, in.fog_cylindrical);
}

fn apply_translucent_lighting(in: VertexOutput, object_color: vec4<f32>) -> vec4<f32> {
    let tinted_color = object_color * in.color;


    if (tinted_color.a < 0.1) {
        discard;
    }

    let lighting = terrain_directional_lighting(in.normal);



    let lightmap = in.lightmap;



    let lit_rgb = tinted_color.xyz * lighting;
    let overlaid = mix(camera.overlay_color.rgb, lit_rgb, camera.overlay_color.a);
    return vec4<f32>(overlaid * lightmap, tinted_color.a);
}

fn apply_terrain_translucent_lighting(
    in: TerrainVertexOutput,
    object_color: vec4<f32>,
) -> vec4<f32> {
    let tinted_color = object_color * in.vertex_color;
    if (tinted_color.a < 0.1) {
        discard;
    }

    return apply_fog(tinted_color, in.fog_spherical, in.fog_cylindrical);
}

@fragment
fn fs_translucent(in: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {




    let object_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    if (object_color.a < 0.1) { discard; }
    let color = object_color * in.color;
    let normal = select(-in.normal, in.normal, front_facing);
    let directional = java_entity_diffuse_lighting(normal);
    let overlaid = mix(camera.overlay_color.rgb, color.rgb * directional, camera.overlay_color.a);
    return apply_fog(vec4<f32>(overlaid * in.lightmap, color.a), in.fog_spherical, in.fog_cylindrical);
}













@fragment
fn fs_entity_translucent(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool,
) -> @location(0) vec4<f32> {
    let object_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);


    if (object_color.a < 0.1) {
        discard;
    }



    let face_normal = select(-in.normal, in.normal, front_facing);
    let directional = java_entity_diffuse_lighting(face_normal);
    let tinted_color = object_color * in.color;





    let lit_rgb = tinted_color.rgb * directional;
    let overlaid = mix(camera.overlay_color.rgb, lit_rgb, camera.overlay_color.a);
    return vec4<f32>(
        overlaid * in.lightmap,
        tinted_color.a,
    );
}





@fragment
fn fs_entity_cutout_direct(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool,
) -> @location(0) vec4<f32> {
    let object_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    if (object_color.a < 0.1) {
        discard;
    }
    let face_normal = select(-in.normal, in.normal, front_facing);
    let directional = java_entity_diffuse_lighting(face_normal);
    let tinted_color = object_color * in.color;
    let lit_rgb = tinted_color.rgb * directional;
    let overlaid = mix(camera.overlay_color.rgb, lit_rgb, camera.overlay_color.a);
    return apply_fog(
        vec4<f32>(overlaid * in.lightmap, tinted_color.a),
        in.fog_spherical,
        in.fog_cylindrical,
    );
}






@fragment
fn fs_entity_translucent_cull_direct(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    if (object_color.a < 0.1) {
        discard;
    }
    let directional = java_entity_diffuse_lighting(in.normal);
    let tinted_color = object_color * in.color;
    let lit_rgb = tinted_color.rgb * directional;
    let overlaid = mix(camera.overlay_color.rgb, lit_rgb, camera.overlay_color.a);
    return apply_fog(
        vec4<f32>(overlaid * in.lightmap, tinted_color.a),
        in.fog_spherical,
        in.fog_cylindrical,
    );
}





@fragment
fn fs_breeze_wind(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    if (object_color.a < 0.1) {
        discard;
    }
    let tinted_color = object_color * in.color;
    return apply_fog(
        vec4<f32>(tinted_color.rgb * in.lightmap, tinted_color.a),
        in.fog_spherical,
        in.fog_cylindrical,
    );
}



@fragment
fn fs_map(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords) * in.color;
    if (color.a < 0.1) {
        discard;
    }
    return apply_fog(
        vec4<f32>(color.rgb * in.lightmap, color.a),
        in.fog_spherical,
        in.fog_cylindrical,
    );
}

@fragment
fn fs_terrain_translucent(in: TerrainVertexOutput) -> @location(0) vec4<f32> {



    return apply_terrain_translucent_lighting(in, sample_terrain_nearest(in.tex_coords));
}

@fragment
fn fs_unlit(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    if (object_color.a < 0.1) {
        discard;
    }
    let tinted_color = object_color * in.color;
    return tinted_color;
}





@fragment
fn fs_armor(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool,
) -> @location(0) vec4<f32> {
    let object_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    if (object_color.a < 0.1) {
        discard;
    }
    let face_normal = select(-in.normal, in.normal, front_facing);
    let directional = java_entity_diffuse_lighting(face_normal);
    let tinted_color = object_color * in.color;
    return apply_fog(
        vec4<f32>(tinted_color.rgb * directional * in.lightmap, tinted_color.a),
        in.fog_spherical,
        in.fog_cylindrical,
    );
}



@fragment
fn fs_cape(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords) * in.color;
    return apply_fog(
        vec4<f32>(color.rgb * java_entity_diffuse_lighting(in.normal) * in.lightmap, color.a),
        in.fog_spherical, in.fog_cylindrical,
    );
}
