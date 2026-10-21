use super::super::tree::{CubeSpec, ModelTextureSize, cube_polygons};
use super::super::types::EntityModelAnimationState;
use crate::render::baked_model::Vertex;
use glam::{Mat4, Vec3};
const TEXTURE_SIZE: ModelTextureSize = ModelTextureSize {
    width: 32.0,
    height: 32.0,
};
const CROSS_TEXTURE_SIZE: ModelTextureSize = ModelTextureSize {
    width: 32.0,
    height: 32.0 * 0.8,
};
pub fn build_arrow_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_arrow_mesh_with_renderer_transform(Mat4::IDENTITY, 0.0)
}
pub fn java_arrow_shake_rotation_radians(shake: f32) -> f32 {
    if shake > 0.0 {
        -(shake * 3.0).sin() * shake * std::f32::consts::PI / 180.0
    } else {
        0.0
    }
}
#[allow(dead_code)]
pub fn build_arrow_mesh_for_java_render_state(
    y_rot_degrees: f32,
    x_rot_degrees: f32,
    shake: f32,
) -> (Vec<Vertex>, Vec<u32>) {
    let renderer_transform = Mat4::from_rotation_y((y_rot_degrees - 90.0).to_radians())
        * Mat4::from_rotation_z(x_rot_degrees.to_radians());
    build_arrow_mesh_with_renderer_transform(renderer_transform, shake)
}
pub fn build_arrow_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let generic_pass_compensation =
        Mat4::from_rotation_y((2.0 * state.body_rot - 90.0).to_radians())
            * Mat4::from_rotation_z(state.x_rot.to_radians());
    build_arrow_mesh_with_renderer_transform(generic_pass_compensation, state.arrow_shake)
}
fn build_arrow_mesh_with_renderer_transform(
    renderer_transform: Mat4,
    shake: f32,
) -> (Vec<Vertex>, Vec<u32>) {
    let root_transform = renderer_transform
        * Mat4::from_rotation_z(java_arrow_shake_rotation_radians(shake))
        * Mat4::from_scale(Vec3::splat(0.9));
    let back_transform = root_transform
        * Mat4::from_translation(Vec3::new(-11.0, 0.0, 0.0))
        * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_4)
        * Mat4::from_scale(Vec3::splat(0.8));
    let cross_1_transform = root_transform * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_4);
    let cross_2_transform =
        root_transform * Mat4::from_rotation_x(std::f32::consts::PI * 3.0 / 4.0);
    let back = CubeSpec::new(
        [0.0, 0.0],
        Vec3::new(0.0, -2.5, -2.5),
        Vec3::new(0.0, 5.0, 5.0),
        Vec3::ZERO,
        false,
    );
    let cross = CubeSpec::new(
        [0.0, 0.0],
        Vec3::new(-12.0, -2.0, 0.0),
        Vec3::new(16.0, 4.0, 0.0),
        Vec3::ZERO,
        false,
    );
    let mut vertices = Vec::with_capacity(18 * 4);
    let mut indices = Vec::with_capacity(18 * 6);
    append_java_cube(
        &mut vertices,
        &mut indices,
        &back,
        TEXTURE_SIZE,
        back_transform,
    );
    append_java_cube(
        &mut vertices,
        &mut indices,
        &cross,
        CROSS_TEXTURE_SIZE,
        cross_1_transform,
    );
    append_java_cube(
        &mut vertices,
        &mut indices,
        &cross,
        CROSS_TEXTURE_SIZE,
        cross_2_transform,
    );
    (vertices, indices)
}
fn append_java_cube(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    cube: &CubeSpec,
    texture: ModelTextureSize,
    pixel_transform: Mat4,
) {
    for polygon in cube_polygons(cube, texture) {
        let start = vertices.len() as u32;
        let normal = pixel_transform
            .transform_vector3(polygon.normal)
            .normalize_or_zero();
        for vertex in polygon.vertices {
            let position = pixel_transform.transform_point3(vertex.pos) / 16.0;
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
