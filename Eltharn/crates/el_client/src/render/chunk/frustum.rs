#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    planes: [glam::Vec4; 6],
}
impl Frustum {
    pub fn from_view_projection(matrix: glam::Mat4) -> Self {
        let rows = matrix.transpose();
        let row0 = rows.x_axis;
        let row1 = rows.y_axis;
        let row2 = rows.z_axis;
        let row3 = rows.w_axis;
        Self {
            planes: [
                normalize_plane(row3 + row0),
                normalize_plane(row3 - row0),
                normalize_plane(row3 + row1),
                normalize_plane(row3 - row1),
                normalize_plane(row2),
                normalize_plane(row3 - row2),
            ],
        }
    }
    pub fn contains_aabb(self, min: glam::Vec3, max: glam::Vec3) -> bool {
        self.planes.iter().all(|plane| {
            let point = glam::Vec3::new(
                if plane.x >= 0.0 { max.x } else { min.x },
                if plane.y >= 0.0 { max.y } else { min.y },
                if plane.z >= 0.0 { max.z } else { min.z },
            );
            plane.truncate().dot(point) + plane.w >= 0.0
        })
    }
    pub fn contains_section(self, chunk_x: i32, section_y: i32, chunk_z: i32) -> bool {
        let min = glam::Vec3::new(
            (chunk_x << 4) as f32,
            (section_y << 4) as f32,
            (chunk_z << 4) as f32,
        );
        self.contains_aabb(min, min + glam::Vec3::splat(16.0))
    }
}
fn normalize_plane(plane: glam::Vec4) -> glam::Vec4 {
    let length = plane.truncate().length();
    if length == 0.0 { plane } else { plane / length }
}
