use crate::render::baked_model::Vertex;
use crate::render::model::player::player_model::{
    PlayerModelParts, generate_cape_mesh_with_jacket,
};
use el_mod_content::scene_types::avatar::{CAPE_ROWS, CapeRows};
use glam::Vec3;
pub type InterpolatedCapeRows = [[f32; 3]; CAPE_ROWS];
pub const MAX_CAPE_VERTICES: usize = 264;
pub const MAX_CAPE_INDICES: usize = 396;
pub fn interpolate(
    previous: &CapeRows,
    current: &CapeRows,
    partial_tick: f32,
) -> InterpolatedCapeRows {
    let t = if partial_tick.is_finite() {
        partial_tick.clamp(0.0, 1.0)
    } else {
        0.0
    };
    std::array::from_fn(|row| {
        std::array::from_fn(|axis| {
            previous[row][axis] as f32 + (current[row][axis] - previous[row][axis]) as f32 * t
        })
    })
}
pub fn generate(
    parts: &PlayerModelParts,
    cape_flap: f32,
    cape_lean: f32,
    cape_lean2: f32,
    jacket_offset: bool,
    rows: Option<&InterpolatedCapeRows>,
) -> (Vec<Vertex>, Vec<u32>) {
    let vanilla =
        generate_cape_mesh_with_jacket(parts, cape_flap, cape_lean, cape_lean2, jacket_offset);
    let Some(rows) = rows.filter(|r| {
        r.iter().flatten().any(|v| *v != 0.0)
            && r.iter()
                .flatten()
                .all(|v| v.is_finite() && v.abs() <= 8000.0)
            && r[0] == [0.0; 3]
    }) else {
        return vanilla;
    };
    let (base, base_indices) = vanilla;
    debug_assert_eq!(base.len(), 24);
    let axis_x = -Vec3::from_array(base[8].normal);
    let axis_y = Vec3::from_array(base[0].normal);
    let axis_z = Vec3::from_array(base[12].normal);
    let top = Vec3::from_array(base[0].position).dot(axis_y);
    let bottom = Vec3::from_array(base[4].position).dot(axis_y);
    let height = top - bottom;
    if !height.is_finite() || height <= 0.0 {
        return (base, base_indices);
    }
    let shift = |row: usize| {
        let r = rows[row];
        (axis_x * r[0] - axis_y * r[1] - axis_z * r[2]) / 16000.0
    };
    let mut vertices = Vec::with_capacity(MAX_CAPE_VERTICES);
    let mut indices = Vec::with_capacity(MAX_CAPE_INDICES);
    for (face_index, face) in base.chunks_exact(4).enumerate() {
        if face_index < 2 {
            let row = if face_index == 0 { 0 } else { CAPE_ROWS - 1 };
            let quad: [Vertex; 4] = std::array::from_fn(|i| {
                let mut v = face[i];
                v.position = (Vec3::from_array(v.position) + shift(row)).to_array();
                v
            });
            append_quad(quad, &mut vertices, &mut indices);
            continue;
        }
        let is_top: [bool; 4] = std::array::from_fn(|i| {
            Vec3::from_array(face[i].position).dot(axis_y) > (top + bottom) * 0.5
        });
        let mates: [usize; 4] = std::array::from_fn(|i| {
            let p = Vec3::from_array(face[i].position);
            (0..4)
                .filter(|j| is_top[i] != is_top[*j])
                .min_by(|a, b| {
                    let planar_distance = |j: usize| {
                        let d = Vec3::from_array(face[j].position) - p;
                        d.dot(axis_x).powi(2) + d.dot(axis_z).powi(2)
                    };
                    planar_distance(*a).total_cmp(&planar_distance(*b))
                })
                .expect("vanilla cape face has two top and two bottom corners")
        });
        for band in 0..CAPE_ROWS - 1 {
            let quad = std::array::from_fn(|i| {
                let row = band + usize::from(!is_top[i]);
                let (a, b) = if is_top[i] {
                    (face[i], face[mates[i]])
                } else {
                    (face[mates[i]], face[i])
                };
                let t = row as f32 / (CAPE_ROWS - 1) as f32;
                let mut v = a;
                v.position = (Vec3::from_array(a.position).lerp(Vec3::from_array(b.position), t)
                    + shift(row))
                .to_array();
                v.tex_coords = std::array::from_fn(|axis| {
                    a.tex_coords[axis] + (b.tex_coords[axis] - a.tex_coords[axis]) * t
                });
                v
            });
            append_quad(quad, &mut vertices, &mut indices);
        }
    }
    (vertices, indices)
}
fn append_quad(mut quad: [Vertex; 4], vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>) {
    let p = quad.map(|v| Vec3::from_array(v.position));
    let cross = (p[1] - p[0]).cross(p[2] - p[0]);
    if !cross.is_finite() || cross.length_squared() < 1.0e-16 {
        return;
    }
    let normal = cross.normalize().to_array();
    for v in &mut quad {
        v.normal = normal;
    }
    let first = vertices.len() as u32;
    vertices.extend(quad);
    indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
}
