struct CameraUniform {
    view_proj: mat4x4<f32>,
    lightmap_factors: vec4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct CloudInfo {
    color: vec4<f32>,
    offset: vec3<f32>,
    cloud_fog_end: f32,
    cell_size: vec3<f32>,
    _pad2: f32,
};
@group(2) @binding(0)
var<uniform> cloud_info: CloudInfo;

@group(2) @binding(1)
var<storage, read> cloud_faces: array<u32>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) vertex_distance: f32,
};

const FLAG_MASK_DIR: u32 = 7u;
const FLAG_INSIDE_FACE: u32 = 16u;
const FLAG_USE_TOP_COLOR: u32 = 32u;
const FLAG_EXTRA_Z: u32 = 64u;
const FLAG_EXTRA_X: u32 = 128u;

fn extract_signed_byte(val: u32) -> i32 {
    if ((val & 0x80u) != 0u) {
        return i32(val | 0xffffff00u);
    } else {
        return i32(val);
    }
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_id: u32,
) -> VertexOutput {
    var vertices = array<vec3<f32>, 24>(

        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(1.0, 0.0, 1.0),
        vec3<f32>(0.0, 0.0, 1.0),
        vec3<f32>(0.0, 0.0, 0.0),

        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 1.0, 1.0),
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(1.0, 1.0, 0.0),

        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(1.0, 1.0, 0.0),
        vec3<f32>(1.0, 0.0, 0.0),

        vec3<f32>(1.0, 0.0, 1.0),
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(0.0, 1.0, 1.0),
        vec3<f32>(0.0, 0.0, 1.0),

        vec3<f32>(0.0, 0.0, 1.0),
        vec3<f32>(0.0, 1.0, 1.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, 0.0),

        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(1.0, 1.0, 0.0),
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(1.0, 0.0, 1.0)
    );

    var faceColors = array<vec4<f32>, 6>(
        vec4<f32>(0.7, 0.7, 0.7, 1.0),
        vec4<f32>(1.0, 1.0, 1.0, 1.0),
        vec4<f32>(0.8, 0.8, 0.8, 1.0),
        vec4<f32>(0.8, 0.8, 0.8, 1.0),
        vec4<f32>(0.9, 0.9, 0.9, 1.0),
        vec4<f32>(0.9, 0.9, 0.9, 1.0)
    );

    let quad_vertex = vertex_id % 4u;
    let face_index = vertex_id / 4u;

    let packed = cloud_faces[face_index];
    let raw_x = packed & 0xffu;
    let raw_z = (packed >> 8u) & 0xffu;
    let dir_and_flags = (packed >> 16u) & 0xffu;

    var cell_x = extract_signed_byte(raw_x);
    var cell_z = extract_signed_byte(raw_z);

    let direction = dir_and_flags & FLAG_MASK_DIR;
    let is_inside_face = (dir_and_flags & FLAG_INSIDE_FACE) == FLAG_INSIDE_FACE;
    let use_top_color = (dir_and_flags & FLAG_USE_TOP_COLOR) == FLAG_USE_TOP_COLOR;

    cell_x = (cell_x << 1u) | i32((dir_and_flags & FLAG_EXTRA_X) >> 7u);
    cell_z = (cell_z << 1u) | i32((dir_and_flags & FLAG_EXTRA_Z) >> 6u);

    var idx = direction * 4u;
    if (is_inside_face) {
        idx = idx + (3u - quad_vertex);
    } else {
        idx = idx + quad_vertex;
    }

    let face_vertex = vertices[idx];
    let pos = (face_vertex * cloud_info.cell_size) + (vec3<f32>(f32(cell_x), 0.0, f32(cell_z)) * cloud_info.cell_size) + cloud_info.offset;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(pos, 1.0);
    out.vertex_distance = length(pos);

    var color_base: vec4<f32>;
    if (use_top_color) {
        color_base = faceColors[1];
    } else {
        color_base = faceColors[direction];
    }



    out.color = color_base * cloud_info.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = in.color;

    let fog_val = clamp(in.vertex_distance / cloud_info.cloud_fog_end, 0.0, 1.0);
    color.a = color.a * (1.0 - fog_val);
    return color;
}
