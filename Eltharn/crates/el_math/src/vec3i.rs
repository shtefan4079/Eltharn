use crate::{Axis, Direction};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vec3i {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Vec3i {
    pub const ZERO: Vec3i = Vec3i { x: 0, y: 0, z: 0 };

    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Vec3i { x, y, z }
    }

    pub fn java_hash_code(self) -> i32 {
        self.y
            .wrapping_add(self.z.wrapping_mul(31))
            .wrapping_mul(31)
            .wrapping_add(self.x)
    }

    pub fn compare_to(self, pos: Self) -> i32 {
        if self.y == pos.y {
            if self.z == pos.z {
                self.x.wrapping_sub(pos.x)
            } else {
                self.z.wrapping_sub(pos.z)
            }
        } else {
            self.y.wrapping_sub(pos.y)
        }
    }

    pub fn offset(self, x: i32, y: i32, z: i32) -> Self {
        if x == 0 && y == 0 && z == 0 {
            self
        } else {
            Vec3i::new(
                self.x.wrapping_add(x),
                self.y.wrapping_add(y),
                self.z.wrapping_add(z),
            )
        }
    }

    pub fn offset_vec(self, vec: Self) -> Self {
        self.offset(vec.x, vec.y, vec.z)
    }

    pub fn subtract(self, vec: Self) -> Self {
        self.offset(
            vec.x.wrapping_neg(),
            vec.y.wrapping_neg(),
            vec.z.wrapping_neg(),
        )
    }

    pub fn multiply(self, scale: i32) -> Self {
        if scale == 1 {
            self
        } else if scale == 0 {
            Vec3i::ZERO
        } else {
            Vec3i::new(
                self.x.wrapping_mul(scale),
                self.y.wrapping_mul(scale),
                self.z.wrapping_mul(scale),
            )
        }
    }

    pub fn multiply_components(self, x_scale: i32, y_scale: i32, z_scale: i32) -> Self {
        Vec3i::new(
            self.x.wrapping_mul(x_scale),
            self.y.wrapping_mul(y_scale),
            self.z.wrapping_mul(z_scale),
        )
    }

    pub fn above(self) -> Self {
        self.above_steps(1)
    }

    pub fn above_steps(self, steps: i32) -> Self {
        self.relative(Direction::Up, steps)
    }

    pub fn below(self) -> Self {
        self.below_steps(1)
    }

    pub fn below_steps(self, steps: i32) -> Self {
        self.relative(Direction::Down, steps)
    }

    pub fn north(self) -> Self {
        self.north_steps(1)
    }

    pub fn north_steps(self, steps: i32) -> Self {
        self.relative(Direction::North, steps)
    }

    pub fn south(self) -> Self {
        self.south_steps(1)
    }

    pub fn south_steps(self, steps: i32) -> Self {
        self.relative(Direction::South, steps)
    }

    pub fn west(self) -> Self {
        self.west_steps(1)
    }

    pub fn west_steps(self, steps: i32) -> Self {
        self.relative(Direction::West, steps)
    }

    pub fn east(self) -> Self {
        self.east_steps(1)
    }

    pub fn east_steps(self, steps: i32) -> Self {
        self.relative(Direction::East, steps)
    }

    pub fn relative(self, direction: Direction, steps: i32) -> Self {
        if steps == 0 {
            self
        } else {
            Vec3i::new(
                self.x.wrapping_add(direction.step_x().wrapping_mul(steps)),
                self.y.wrapping_add(direction.step_y().wrapping_mul(steps)),
                self.z.wrapping_add(direction.step_z().wrapping_mul(steps)),
            )
        }
    }

    pub fn relative_axis(self, axis: Axis, steps: i32) -> Self {
        if steps == 0 {
            self
        } else {
            let x_step = if axis == Axis::X { steps } else { 0 };
            let y_step = if axis == Axis::Y { steps } else { 0 };
            let z_step = if axis == Axis::Z { steps } else { 0 };
            Vec3i::new(
                self.x.wrapping_add(x_step),
                self.y.wrapping_add(y_step),
                self.z.wrapping_add(z_step),
            )
        }
    }

    pub fn cross(self, up_vector: Self) -> Self {
        Vec3i::new(
            self.y
                .wrapping_mul(up_vector.z)
                .wrapping_sub(self.z.wrapping_mul(up_vector.y)),
            self.z
                .wrapping_mul(up_vector.x)
                .wrapping_sub(self.x.wrapping_mul(up_vector.z)),
            self.x
                .wrapping_mul(up_vector.y)
                .wrapping_sub(self.y.wrapping_mul(up_vector.x)),
        )
    }

    pub fn closer_than(self, pos: Self, distance: f64) -> bool {
        self.dist_sqr(pos) < distance * distance
    }

    pub fn dist_sqr(self, pos: Self) -> f64 {
        self.dist_to_low_corner_sqr(pos.x as f64, pos.y as f64, pos.z as f64)
    }

    pub fn dist_to_center_sqr(self, x: f64, y: f64, z: f64) -> f64 {
        let dx = self.x as f64 + 0.5 - x;
        let dy = self.y as f64 + 0.5 - y;
        let dz = self.z as f64 + 0.5 - z;
        dx * dx + dy * dy + dz * dz
    }

    pub fn dist_to_low_corner_sqr(self, x: f64, y: f64, z: f64) -> f64 {
        let dx = self.x as f64 - x;
        let dy = self.y as f64 - y;
        let dz = self.z as f64 - z;
        dx * dx + dy * dy + dz * dz
    }

    pub fn dist_manhattan(self, pos: Self) -> i32 {
        let xd = java_abs_i32(pos.x.wrapping_sub(self.x)) as f32;
        let yd = java_abs_i32(pos.y.wrapping_sub(self.y)) as f32;
        let zd = java_abs_i32(pos.z.wrapping_sub(self.z)) as f32;
        (xd + yd + zd) as i32
    }

    pub fn dist_chessboard(self, pos: Self) -> i32 {
        let xd = java_abs_i32(self.x.wrapping_sub(pos.x));
        let yd = java_abs_i32(self.y.wrapping_sub(pos.y));
        let zd = java_abs_i32(self.z.wrapping_sub(pos.z));
        std::cmp::max(std::cmp::max(xd, yd), zd)
    }

    pub fn get(self, axis: Axis) -> i32 {
        axis.choose_i32(self.x, self.y, self.z)
    }

    pub fn to_short_string(self) -> String {
        format!("{}, {}, {}", self.x, self.y, self.z)
    }
}

impl fmt::Display for Vec3i {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Vec3i{{x={}, y={}, z={}}}", self.x, self.y, self.z)
    }
}

fn java_abs_i32(value: i32) -> i32 {
    if value == i32::MIN {
        i32::MIN
    } else {
        value.abs()
    }
}
