use super::super::tree::{CubeSpec, ModelTextureSize, cube_polygons};
use crate::render::baked_model::Vertex;
use glam::{Mat4, Quat, Vec3};
const TEXTURE_SIZE: ModelTextureSize = ModelTextureSize {
    width: 64.0,
    height: 32.0,
};
const SIN_45: f32 = std::f32::consts::FRAC_1_SQRT_2;
const TILT_RADIANS: f32 = std::f32::consts::PI / 3.0;
const INNER_GLASS_SCALE: f32 = 0.875;
const CUBE_SCALE: f32 = 0.765_625;
const END_CRYSTAL_BEAM_STEPS: usize = 8;
const END_CRYSTAL_BEAM_OUTER_RADIUS: f32 = 0.75;
const END_CRYSTAL_BEAM_INNER_RADIUS: f32 = 0.2;
pub fn java_end_crystal_y(age_in_ticks: f32) -> f32 {
    let mut bob = (age_in_ticks * 0.2).sin() / 2.0 + 0.5;
    bob = (bob * bob + bob) * 0.4;
    bob - 1.4
}
fn append_java_cube(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    cube: &CubeSpec,
    pixel_transform: Mat4,
    renderer_transform: Mat4,
) {
    for polygon in cube_polygons(cube, TEXTURE_SIZE) {
        let start = vertices.len() as u32;
        let normal = renderer_transform
            .transform_vector3(pixel_transform.transform_vector3(polygon.normal))
            .normalize_or_zero();
        for vertex in polygon.vertices {
            let model_position = pixel_transform.transform_point3(vertex.pos) / 16.0;
            let position = renderer_transform.transform_point3(model_position);
            vertices.push(Vertex {
                position: position.to_array(),
                tex_coords: vertex.uv,
                normal: normal.to_array(),
                color: [1.0, 1.0, 1.0, 1.0],
                light: [1.0, 1.0],
            });
        }
        indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
    }
}
fn crystal_part_rotations(age_in_ticks: f32) -> (Quat, Quat, Quat) {
    let animation_radians = (age_in_ticks * 3.0).to_radians();
    let tilt = Quat::from_axis_angle(Vec3::new(SIN_45, 0.0, SIN_45), TILT_RADIANS);
    let spin = Quat::from_rotation_y(animation_radians);
    let outer = spin * tilt;
    let nested = tilt * spin;
    (outer, nested, nested)
}
pub fn build_end_crystal_mesh(age_in_ticks: f32, shows_bottom: bool) -> (Vec<Vertex>, Vec<u32>) {
    let glass = CubeSpec::new(
        [0.0, 0.0],
        Vec3::new(-4.0, -4.0, -4.0),
        Vec3::splat(8.0),
        Vec3::ZERO,
        false,
    );
    let core_cube = CubeSpec::new(
        [32.0, 0.0],
        Vec3::new(-4.0, -4.0, -4.0),
        Vec3::splat(8.0),
        Vec3::ZERO,
        false,
    );
    let base = CubeSpec::new(
        [0.0, 16.0],
        Vec3::new(-6.0, 0.0, -6.0),
        Vec3::new(12.0, 4.0, 12.0),
        Vec3::ZERO,
        false,
    );
    let renderer_transform =
        Mat4::from_scale(Vec3::splat(2.0)) * Mat4::from_translation(Vec3::new(0.0, -0.5, 0.0));
    let (outer_rotation, inner_rotation, core_rotation) = crystal_part_rotations(age_in_ticks);
    let outer_y_pixels = 24.0 + java_end_crystal_y(age_in_ticks) * 16.0 / 2.0;
    let outer = Mat4::from_translation(Vec3::new(0.0, outer_y_pixels, 0.0))
        * Mat4::from_quat(outer_rotation);
    let inner =
        outer * Mat4::from_quat(inner_rotation) * Mat4::from_scale(Vec3::splat(INNER_GLASS_SCALE));
    let core_matrix =
        inner * Mat4::from_quat(core_rotation) * Mat4::from_scale(Vec3::splat(CUBE_SCALE));
    let mut vertices = Vec::with_capacity(if shows_bottom { 4 * 6 * 4 } else { 3 * 6 * 4 });
    let mut indices = Vec::with_capacity(if shows_bottom { 4 * 6 * 6 } else { 3 * 6 * 6 });
    if shows_bottom {
        append_java_cube(
            &mut vertices,
            &mut indices,
            &base,
            Mat4::IDENTITY,
            renderer_transform,
        );
    }
    append_java_cube(
        &mut vertices,
        &mut indices,
        &glass,
        outer,
        renderer_transform,
    );
    append_java_cube(
        &mut vertices,
        &mut indices,
        &glass,
        inner,
        renderer_transform,
    );
    append_java_cube(
        &mut vertices,
        &mut indices,
        &core_cube,
        core_matrix,
        renderer_transform,
    );
    (vertices, indices)
}
pub fn build_end_crystal_mesh_for_generic_remote_entity(
    age_in_ticks: f32,
    shows_bottom: bool,
    body_yaw_degrees: f32,
) -> (Vec<Vertex>, Vec<u32>) {
    let (mut vertices, indices) = build_end_crystal_mesh(age_in_ticks, shows_bottom);
    let compensation = Mat4::from_rotation_y(body_yaw_degrees.to_radians());
    for vertex in &mut vertices {
        let position = compensation.transform_point3(Vec3::from(vertex.position));
        let normal = compensation
            .transform_vector3(Vec3::from(vertex.normal))
            .normalize_or_zero();
        vertex.position = position.to_array();
        vertex.normal = normal.to_array();
    }
    (vertices, indices)
}
pub fn build_end_crystal_beam_mesh(
    delta_x: f32,
    delta_y: f32,
    delta_z: f32,
    time_in_ticks: f32,
    light: [f32; 2],
) -> (Vec<Vertex>, Vec<u32>) {
    let horizontal_length = (delta_x * delta_x + delta_z * delta_z).sqrt();
    let length = (delta_x * delta_x + delta_y * delta_y + delta_z * delta_z).sqrt();
    let beam_pose = Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0))
        * Mat4::from_rotation_y(-delta_z.atan2(delta_x) - std::f32::consts::FRAC_PI_2)
        * Mat4::from_rotation_x(-horizontal_length.atan2(delta_y) - std::f32::consts::FRAC_PI_2);
    let normal = beam_pose
        .transform_vector3(Vec3::NEG_Y)
        .normalize_or_zero()
        .to_array();
    let v0 = -time_in_ticks * 0.01;
    let v1 = length / 32.0 - time_in_ticks * 0.01;
    let mut vertices = Vec::with_capacity(END_CRYSTAL_BEAM_STEPS * 4);
    let mut indices = Vec::with_capacity(END_CRYSTAL_BEAM_STEPS * 6);
    let mut last_sin = 0.0;
    let mut last_cos = END_CRYSTAL_BEAM_OUTER_RADIUS;
    let mut last_u = 0.0;
    for step in 1..=END_CRYSTAL_BEAM_STEPS {
        let angle = step as f32 * std::f32::consts::TAU / END_CRYSTAL_BEAM_STEPS as f32;
        let sin = angle.sin() * END_CRYSTAL_BEAM_OUTER_RADIUS;
        let cos = angle.cos() * END_CRYSTAL_BEAM_OUTER_RADIUS;
        let u = step as f32 / END_CRYSTAL_BEAM_STEPS as f32;
        let start = vertices.len() as u32;
        for (position, tex_coords, color) in [
            (
                Vec3::new(
                    last_sin * END_CRYSTAL_BEAM_INNER_RADIUS,
                    last_cos * END_CRYSTAL_BEAM_INNER_RADIUS,
                    0.0,
                ),
                [last_u, v0],
                [0.0, 0.0, 0.0, 1.0],
            ),
            (
                Vec3::new(last_sin, last_cos, length),
                [last_u, v1],
                [1.0, 1.0, 1.0, 1.0],
            ),
            (Vec3::new(sin, cos, length), [u, v1], [1.0, 1.0, 1.0, 1.0]),
            (
                Vec3::new(
                    sin * END_CRYSTAL_BEAM_INNER_RADIUS,
                    cos * END_CRYSTAL_BEAM_INNER_RADIUS,
                    0.0,
                ),
                [u, v0],
                [0.0, 0.0, 0.0, 1.0],
            ),
        ] {
            vertices.push(Vertex {
                position: beam_pose.transform_point3(position).to_array(),
                tex_coords,
                normal,
                color,
                light,
            });
        }
        indices.extend_from_slice(&[start, start + 1, start + 2, start + 2, start + 3, start]);
        last_sin = sin;
        last_cos = cos;
        last_u = u;
    }
    (vertices, indices)
}
pub fn build_end_crystal_target_beam_mesh(
    beam_offset: Vec3,
    age_in_ticks: f32,
    light: [f32; 2],
) -> (Vec<Vertex>, Vec<u32>) {
    let crystal_y = java_end_crystal_y(age_in_ticks);
    let (mut vertices, indices) = build_end_crystal_beam_mesh(
        -beam_offset.x,
        -beam_offset.y + crystal_y,
        -beam_offset.z,
        age_in_ticks,
        light,
    );
    for vertex in &mut vertices {
        vertex.position = (Vec3::from(vertex.position) + beam_offset).to_array();
    }
    (vertices, indices)
}
