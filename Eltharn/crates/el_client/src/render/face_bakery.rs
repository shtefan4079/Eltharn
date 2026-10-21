use crate::render::face_info::FaceInfo;
use el_math::Direction;
use glam::{Mat4, Vec3};

pub struct FaceBakery;

impl FaceBakery {
    const BLOCK_MIDDLE: Vec3 = Vec3::new(0.5, 0.5, 0.5);

    pub fn default_face_uv(from: Vec3, to: Vec3, facing: Direction) -> [f32; 4] {
        match facing {
            Direction::Down => [from.x, 16.0 - to.z, to.x, 16.0 - from.z],
            Direction::Up => [from.x, from.z, to.x, to.z],
            Direction::North => [16.0 - to.x, 16.0 - to.y, 16.0 - from.x, 16.0 - from.y],
            Direction::South => [from.x, 16.0 - to.y, to.x, 16.0 - from.y],
            Direction::West => [from.z, 16.0 - to.y, to.z, 16.0 - from.y],
            Direction::East => [16.0 - to.z, 16.0 - to.y, 16.0 - from.z, 16.0 - from.y],
        }
    }

    pub fn bake_quad(
        from: Vec3,
        to: Vec3,
        uvs: [f32; 4],
        uv_rotation: u32,
        uv_transform: Mat4,
        facing: Direction,
        rotation: Mat4,
        element_rotation: Option<(Vec3, Mat4)>,
    ) -> ([Vec3; 4], [[f32; 2]; 4], Direction) {
        let mut vertex_positions = [Vec3::ZERO; 4];
        let mut vertex_uvs = [[0.0; 2]; 4];
        let face_info = FaceInfo::from_facing(facing);

        for i in 0..4 {
            Self::bake_vertex(
                i,
                face_info,
                uvs,
                uv_rotation,
                uv_transform,
                from,
                to,
                rotation,
                element_rotation,
                &mut vertex_positions,
                &mut vertex_uvs,
            );
        }

        let final_direction = Self::calculate_facing(&vertex_positions).unwrap_or(Direction::Up);

        if element_rotation.is_none() {
            Self::recalculate_winding(&mut vertex_positions, &mut vertex_uvs, final_direction);
        } else {
        }

        (vertex_positions, vertex_uvs, final_direction)
    }

    fn bake_vertex(
        index: usize,
        face_info: &FaceInfo,
        uvs: [f32; 4],
        uv_rotation: u32,
        uv_transform: Mat4,
        from: Vec3,
        to: Vec3,
        rotation: Mat4,
        element_rotation: Option<(Vec3, Mat4)>,
        position_output: &mut [Vec3; 4],
        uv_output: &mut [[f32; 2]; 4],
    ) {
        let vertex_info = face_info.get_vertex_info(index);
        let mut vertex = vertex_info.select(from, to) / 16.0;

        if let Some((origin, transform)) = element_rotation {
            Self::rotate_vertex_by(&mut vertex, origin, transform);
        }

        if rotation != Mat4::IDENTITY {
            Self::rotate_vertex_by(&mut vertex, Self::BLOCK_MIDDLE, rotation);
        }

        let raw_u = Self::get_u(uvs, uv_rotation, index);
        let raw_v = Self::get_v(uvs, uv_rotation, index);

        let (transformed_u, transformed_v) = if uv_transform == Mat4::IDENTITY {
            (raw_u, raw_v)
        } else {
            let transformed_uv = uv_transform.transform_point3(Vec3::new(
                Self::corner_to_center(raw_u),
                Self::corner_to_center(raw_v),
                0.0,
            ));
            (
                Self::center_to_corner(transformed_uv.x),
                Self::center_to_corner(transformed_uv.y),
            )
        };

        position_output[index] = vertex;
        uv_output[index] = [transformed_u, transformed_v];
    }

    fn corner_to_center(value: f32) -> f32 {
        value - 0.5
    }

    fn center_to_corner(value: f32) -> f32 {
        value + 0.5
    }

    fn rotate_vertex_by(vertex: &mut Vec3, origin: Vec3, transformation: Mat4) {
        *vertex -= origin;
        *vertex = transformation.transform_point3(*vertex);
        *vertex += origin;
    }

    fn get_u(uvs: [f32; 4], rotation: u32, index: usize) -> f32 {
        let steps = (rotation / 90) as usize;
        let rotated_index = (index + steps) % 4;
        let u = if rotated_index != 0 && rotated_index != 1 {
            uvs[2]
        } else {
            uvs[0]
        };
        u / 16.0
    }

    fn get_v(uvs: [f32; 4], rotation: u32, index: usize) -> f32 {
        let steps = (rotation / 90) as usize;
        let rotated_index = (index + steps) % 4;
        let v = if rotated_index != 0 && rotated_index != 3 {
            uvs[3]
        } else {
            uvs[1]
        };
        v / 16.0
    }

    fn calculate_facing(positions: &[Vec3; 4]) -> Option<Direction> {
        let v0 = positions[0];
        let v1 = positions[1];
        let v2 = positions[2];

        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let normal = edge1.cross(edge2).normalize_or_zero();

        Self::find_closest_direction(normal)
    }

    fn find_closest_direction(normal: Vec3) -> Option<Direction> {
        if !normal.is_finite() || normal.length_squared() < 0.0001 {
            return None;
        }

        let mut best = None;
        let mut closest_product = 0.0;

        for &candidate in &[
            Direction::Down,
            Direction::Up,
            Direction::North,
            Direction::South,
            Direction::West,
            Direction::East,
        ] {
            let candidate_vec = match candidate {
                Direction::Down => Vec3::new(0.0, -1.0, 0.0),
                Direction::Up => Vec3::new(0.0, 1.0, 0.0),
                Direction::North => Vec3::new(0.0, 0.0, -1.0),
                Direction::South => Vec3::new(0.0, 0.0, 1.0),
                Direction::West => Vec3::new(-1.0, 0.0, 0.0),
                Direction::East => Vec3::new(1.0, 0.0, 0.0),
            };

            let product = normal.dot(candidate_vec);
            if product >= 0.0 && product > closest_product {
                closest_product = product;
                best = Some(candidate);
            }
        }

        best
    }

    fn recalculate_winding(
        positions: &mut [Vec3; 4],
        uvs: &mut [[f32; 2]; 4],
        direction: Direction,
    ) {
        let mut min_x = 999.0_f32;
        let mut min_y = 999.0_f32;
        let mut min_z = 999.0_f32;
        let mut max_x = -999.0_f32;
        let mut max_y = -999.0_f32;
        let mut max_z = -999.0_f32;

        for pos in positions.iter() {
            if pos.x < min_x {
                min_x = pos.x;
            }
            if pos.y < min_y {
                min_y = pos.y;
            }
            if pos.z < min_z {
                min_z = pos.z;
            }
            if pos.x > max_x {
                max_x = pos.x;
            }
            if pos.y > max_y {
                max_y = pos.y;
            }
            if pos.z > max_z {
                max_z = pos.z;
            }
        }

        let info = FaceInfo::from_facing(direction);

        for vertex in 0..4 {
            let vert_info = info.get_vertex_info(vertex);
            let new_x = vert_info
                .x_face
                .select_f32(min_x, min_y, min_z, max_x, max_y, max_z);
            let new_y = vert_info
                .y_face
                .select_f32(min_x, min_y, min_z, max_x, max_y, max_z);
            let new_z = vert_info
                .z_face
                .select_f32(min_x, min_y, min_z, max_x, max_y, max_z);

            let vertex_to_swap = Self::find_vertex(positions, vertex, new_x, new_y, new_z);
            if vertex_to_swap == -1 {
                continue;
            }

            let swap_idx = vertex_to_swap as usize;
            if swap_idx != vertex {
                positions.swap(swap_idx, vertex);
                uvs.swap(swap_idx, vertex);
            }
        }
    }

    fn find_vertex(positions: &[Vec3; 4], start: usize, x: f32, y: f32, z: f32) -> i32 {
        for i in start..4 {
            let pos = positions[i];
            if (pos.x - x).abs() < 1e-4 && (pos.y - y).abs() < 1e-4 && (pos.z - z).abs() < 1e-4 {
                return i as i32;
            }
        }
        -1
    }
}
