pub mod aabb;
pub mod argb;
pub mod block_pos;
pub mod chunk_pos;
pub mod direction;
pub mod java_random;
pub mod mth;
pub mod noise;
pub mod random;
pub mod section_pos;
pub mod vec3;
pub mod vec3i;
pub mod voxel_shape;
pub use aabb::AABB;
pub use block_pos::BlockPos;
pub use chunk_pos::ChunkPos;
pub use direction::{Axis, AxisDirection, Direction, Plane};
pub use java_random::{JavaRandom, java_string_hash};
pub use random::{
    LegacyRandomSource, PositionalRandomFactory, RandomSource, Seed128bit,
    SingleThreadedRandomSource, ThreadSafeLegacyRandomSource, WorldgenRandom,
    XoroshiroRandomSource,
};
pub use section_pos::SectionPos;
pub use vec3::Vec3;
pub use vec3i::Vec3i;
pub use voxel_shape::VoxelShape;
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector2f {
    pub x: f32,
    pub y: f32,
}
impl Vector2f {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn x(&self) -> f32 {
        self.x
    }
    pub fn y(&self) -> f32 {
        self.y
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vector2i {
    pub x: i32,
    pub y: i32,
}
impl Vector2i {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    pub fn default() -> Self {
        Self { x: 0, y: 0 }
    }
}
pub trait Matrix3x2fc {
    fn transform_position(&self, pos: &mut Vector2f) -> Vector2f;
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix3x2f {
    pub m00: f32,
    pub m01: f32,
    pub m10: f32,
    pub m11: f32,
    pub m20: f32,
    pub m21: f32,
}
impl Matrix3x2f {
    pub fn new() -> Self {
        Self {
            m00: 1.0,
            m01: 0.0,
            m10: 0.0,
            m11: 1.0,
            m20: 0.0,
            m21: 0.0,
        }
    }
    pub fn invert(&self, _dest: Self) -> Self {
        let det = self.m00 * self.m11 - self.m01 * self.m10;
        if det.abs() < 1e-6 {
            return Self::new();
        }
        let inv_det = 1.0 / det;
        Self {
            m00: self.m11 * inv_det,
            m01: -self.m01 * inv_det,
            m10: -self.m10 * inv_det,
            m11: self.m00 * inv_det,
            m20: (self.m10 * self.m21 - self.m11 * self.m20) * inv_det,
            m21: (self.m01 * self.m20 - self.m00 * self.m21) * inv_det,
        }
    }
    pub fn scale(&self, sx: f32, sy: f32, _dest: Self) -> Self {
        Self {
            m00: self.m00 * sx,
            m01: self.m01 * sy,
            m10: self.m10 * sx,
            m11: self.m11 * sy,
            m20: self.m20 * sx,
            m21: self.m21 * sy,
        }
    }
}
impl Default for Matrix3x2f {
    fn default() -> Self {
        Self::new()
    }
}
impl Matrix3x2fc for Matrix3x2f {
    fn transform_position(&self, pos: &mut Vector2f) -> Vector2f {
        let x = pos.x * self.m00 + pos.y * self.m10 + self.m20;
        let y = pos.x * self.m01 + pos.y * self.m11 + self.m21;
        *pos = Vector2f::new(x, y);
        *pos
    }
}
