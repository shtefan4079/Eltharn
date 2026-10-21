use crate::mth::{java_max_f64, java_min_f64};
use crate::{Axis, BlockPos, Direction, Vec3};

pub const EPSILON: f64 = 1.0e-7;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AABB {
    pub min_x: f64,
    pub min_y: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub max_z: f64,
}

impl AABB {
    pub fn new(x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64) -> Self {
        AABB {
            min_x: java_min_f64(x1, x2),
            min_y: java_min_f64(y1, y2),
            min_z: java_min_f64(z1, z2),
            max_x: java_max_f64(x1, x2),
            max_y: java_max_f64(y1, y2),
            max_z: java_max_f64(z1, z2),
        }
    }

    pub fn from_block(pos: BlockPos) -> Self {
        AABB::new(
            pos.x as f64,
            pos.y as f64,
            pos.z as f64,
            (pos.x + 1) as f64,
            (pos.y + 1) as f64,
            (pos.z + 1) as f64,
        )
    }

    pub fn from_vecs(begin: Vec3, end: Vec3) -> Self {
        AABB::new(begin.x, begin.y, begin.z, end.x, end.y, end.z)
    }

    pub fn unit_cube(pos: Vec3) -> Self {
        AABB::new(pos.x, pos.y, pos.z, pos.x + 1.0, pos.y + 1.0, pos.z + 1.0)
    }

    pub fn encapsulating_blocks(pos0: BlockPos, pos1: BlockPos) -> Self {
        let min_x = pos0.x.min(pos1.x) as f64;
        let min_y = pos0.y.min(pos1.y) as f64;
        let min_z = pos0.z.min(pos1.z) as f64;
        let max_x = (pos0.x.max(pos1.x) + 1) as f64;
        let max_y = (pos0.y.max(pos1.y) + 1) as f64;
        let max_z = (pos0.z.max(pos1.z) + 1) as f64;
        AABB {
            min_x,
            min_y,
            min_z,
            max_x,
            max_y,
            max_z,
        }
    }

    pub fn set_min_x(&self, min_x: f64) -> Self {
        AABB::new(
            min_x, self.min_y, self.min_z, self.max_x, self.max_y, self.max_z,
        )
    }

    pub fn set_min_y(&self, min_y: f64) -> Self {
        AABB::new(
            self.min_x, min_y, self.min_z, self.max_x, self.max_y, self.max_z,
        )
    }

    pub fn set_min_z(&self, min_z: f64) -> Self {
        AABB::new(
            self.min_x, self.min_y, min_z, self.max_x, self.max_y, self.max_z,
        )
    }

    pub fn set_max_x(&self, max_x: f64) -> Self {
        AABB::new(
            self.min_x, self.min_y, self.min_z, max_x, self.max_y, self.max_z,
        )
    }

    pub fn set_max_y(&self, max_y: f64) -> Self {
        AABB::new(
            self.min_x, self.min_y, self.min_z, self.max_x, max_y, self.max_z,
        )
    }

    pub fn set_max_z(&self, max_z: f64) -> Self {
        AABB::new(
            self.min_x, self.min_y, self.min_z, self.max_x, self.max_y, max_z,
        )
    }

    pub fn min(&self, axis: Axis) -> f64 {
        axis.choose_f64(self.min_x, self.min_y, self.min_z)
    }

    pub fn max(&self, axis: Axis) -> f64 {
        axis.choose_f64(self.max_x, self.max_y, self.max_z)
    }

    pub fn contract(&self, xa: f64, ya: f64, za: f64) -> Self {
        let mut min_x = self.min_x;
        let mut min_y = self.min_y;
        let mut min_z = self.min_z;
        let mut max_x = self.max_x;
        let mut max_y = self.max_y;
        let mut max_z = self.max_z;

        if xa < 0.0 {
            min_x -= xa;
        } else if xa > 0.0 {
            max_x -= xa;
        }

        if ya < 0.0 {
            min_y -= ya;
        } else if ya > 0.0 {
            max_y -= ya;
        }

        if za < 0.0 {
            min_z -= za;
        } else if za > 0.0 {
            max_z -= za;
        }

        AABB::new(min_x, min_y, min_z, max_x, max_y, max_z)
    }

    pub fn expand_towards(&self, delta: Vec3) -> Self {
        self.expand_towards_xyz(delta.x, delta.y, delta.z)
    }

    pub fn expand_towards_xyz(&self, xa: f64, ya: f64, za: f64) -> Self {
        let mut min_x = self.min_x;
        let mut min_y = self.min_y;
        let mut min_z = self.min_z;
        let mut max_x = self.max_x;
        let mut max_y = self.max_y;
        let mut max_z = self.max_z;

        if xa < 0.0 {
            min_x += xa;
        } else if xa > 0.0 {
            max_x += xa;
        }

        if ya < 0.0 {
            min_y += ya;
        } else if ya > 0.0 {
            max_y += ya;
        }

        if za < 0.0 {
            min_z += za;
        } else if za > 0.0 {
            max_z += za;
        }

        AABB::new(min_x, min_y, min_z, max_x, max_y, max_z)
    }

    pub fn inflate(&self, x: f64, y: f64, z: f64) -> Self {
        let min_x = self.min_x - x;
        let min_y = self.min_y - y;
        let min_z = self.min_z - z;
        let max_x = self.max_x + x;
        let max_y = self.max_y + y;
        let max_z = self.max_z + z;
        AABB::new(min_x, min_y, min_z, max_x, max_y, max_z)
    }

    pub fn inflate_all(&self, amount: f64) -> Self {
        self.inflate(amount, amount, amount)
    }

    pub fn intersect(&self, other: &Self) -> Self {
        let min_x = java_max_f64(self.min_x, other.min_x);
        let min_y = java_max_f64(self.min_y, other.min_y);
        let min_z = java_max_f64(self.min_z, other.min_z);
        let max_x = java_min_f64(self.max_x, other.max_x);
        let max_y = java_min_f64(self.max_y, other.max_y);
        let max_z = java_min_f64(self.max_z, other.max_z);
        AABB::new(min_x, min_y, min_z, max_x, max_y, max_z)
    }

    pub fn minmax(&self, other: &Self) -> Self {
        let min_x = java_min_f64(self.min_x, other.min_x);
        let min_y = java_min_f64(self.min_y, other.min_y);
        let min_z = java_min_f64(self.min_z, other.min_z);
        let max_x = java_max_f64(self.max_x, other.max_x);
        let max_y = java_max_f64(self.max_y, other.max_y);
        let max_z = java_max_f64(self.max_z, other.max_z);
        AABB::new(min_x, min_y, min_z, max_x, max_y, max_z)
    }

    pub fn move_by(&self, xa: f64, ya: f64, za: f64) -> Self {
        AABB::new(
            self.min_x + xa,
            self.min_y + ya,
            self.min_z + za,
            self.max_x + xa,
            self.max_y + ya,
            self.max_z + za,
        )
    }

    pub fn move_by_block(&self, pos: BlockPos) -> Self {
        AABB::new(
            self.min_x + pos.x as f64,
            self.min_y + pos.y as f64,
            self.min_z + pos.z as f64,
            self.max_x + pos.x as f64,
            self.max_y + pos.y as f64,
            self.max_z + pos.z as f64,
        )
    }

    pub fn move_by_vec(&self, pos: Vec3) -> Self {
        self.move_by(pos.x, pos.y, pos.z)
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.intersects_xyz(
            other.min_x,
            other.min_y,
            other.min_z,
            other.max_x,
            other.max_y,
            other.max_z,
        )
    }

    pub fn intersects_xyz(
        &self,
        min_x: f64,
        min_y: f64,
        min_z: f64,
        max_x: f64,
        max_y: f64,
        max_z: f64,
    ) -> bool {
        self.min_x < max_x
            && self.max_x > min_x
            && self.min_y < max_y
            && self.max_y > min_y
            && self.min_z < max_z
            && self.max_z > min_z
    }

    pub fn intersects_vecs(&self, min: Vec3, max: Vec3) -> bool {
        self.intersects_xyz(
            java_min_f64(min.x, max.x),
            java_min_f64(min.y, max.y),
            java_min_f64(min.z, max.z),
            java_max_f64(min.x, max.x),
            java_max_f64(min.y, max.y),
            java_max_f64(min.z, max.z),
        )
    }

    pub fn intersects_block(&self, pos: BlockPos) -> bool {
        self.intersects_xyz(
            pos.x as f64,
            pos.y as f64,
            pos.z as f64,
            (pos.x + 1) as f64,
            (pos.y + 1) as f64,
            (pos.z + 1) as f64,
        )
    }

    pub fn contains(&self, vec: Vec3) -> bool {
        self.contains_xyz(vec.x, vec.y, vec.z)
    }

    pub fn contains_xyz(&self, x: f64, y: f64, z: f64) -> bool {
        x >= self.min_x
            && x < self.max_x
            && y >= self.min_y
            && y < self.max_y
            && z >= self.min_z
            && z < self.max_z
    }

    pub fn get_size(&self) -> f64 {
        let xs = self.get_x_size();
        let ys = self.get_y_size();
        let zs = self.get_z_size();
        (xs + ys + zs) / 3.0
    }

    pub fn get_x_size(&self) -> f64 {
        self.max_x - self.min_x
    }

    pub fn get_y_size(&self) -> f64 {
        self.max_y - self.min_y
    }

    pub fn get_z_size(&self) -> f64 {
        self.max_z - self.min_z
    }

    pub fn deflate(&self, x: f64, y: f64, z: f64) -> Self {
        self.inflate(-x, -y, -z)
    }

    pub fn deflate_all(&self, amount: f64) -> Self {
        self.inflate(-amount, -amount, -amount)
    }

    pub fn clip(&self, from: Vec3, to: Vec3) -> Option<Vec3> {
        Self::clip_many(std::iter::once(*self), from, to)
    }

    pub fn clip_static(
        min_x: f64,
        min_y: f64,
        min_z: f64,
        max_x: f64,
        max_y: f64,
        max_z: f64,
        from: Vec3,
        to: Vec3,
    ) -> Option<Vec3> {
        Self::clip_many(
            std::iter::once(Self {
                min_x,
                min_y,
                min_z,
                max_x,
                max_y,
                max_z,
            }),
            from,
            to,
        )
    }

    pub fn clip_many<I>(aabbs: I, from: Vec3, to: Vec3) -> Option<Vec3>
    where
        I: IntoIterator<Item = AABB>,
    {
        let mut scale_reference = 1.0f64;
        let mut direction: Option<Direction> = None;
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let dz = to.z - from.z;

        for aabb in aabbs {
            direction = clip_aabb(aabb, from, &mut scale_reference, direction, dx, dy, dz);
        }

        if direction.is_none() {
            None
        } else {
            Some(from.add(
                scale_reference * dx,
                scale_reference * dy,
                scale_reference * dz,
            ))
        }
    }

    pub fn distance_to_sq_vec(&self, point: Vec3) -> f64 {
        let dx = java_max_f64(
            java_max_f64(self.min_x - point.x, point.x - self.max_x),
            0.0,
        );
        let dy = java_max_f64(
            java_max_f64(self.min_y - point.y, point.y - self.max_y),
            0.0,
        );
        let dz = java_max_f64(
            java_max_f64(self.min_z - point.z, point.z - self.max_z),
            0.0,
        );
        dx * dx + dy * dy + dz * dz
    }

    pub fn distance_to_sq_aabb(&self, other: &Self) -> f64 {
        let dx = java_max_f64(
            java_max_f64(self.min_x - other.max_x, other.min_x - self.max_x),
            0.0,
        );
        let dy = java_max_f64(
            java_max_f64(self.min_y - other.max_y, other.min_y - self.max_y),
            0.0,
        );
        let dz = java_max_f64(
            java_max_f64(self.min_z - other.max_z, other.min_z - self.max_z),
            0.0,
        );
        dx * dx + dy * dy + dz * dz
    }

    pub fn has_nan(&self) -> bool {
        self.min_x.is_nan()
            || self.min_y.is_nan()
            || self.min_z.is_nan()
            || self.max_x.is_nan()
            || self.max_y.is_nan()
            || self.max_z.is_nan()
    }

    pub fn get_center(&self) -> Vec3 {
        Vec3::new(
            self.min_x + 0.5 * (self.max_x - self.min_x),
            self.min_y + 0.5 * (self.max_y - self.min_y),
            self.min_z + 0.5 * (self.max_z - self.min_z),
        )
    }

    pub fn get_bottom_center(&self) -> Vec3 {
        Vec3::new(
            self.min_x + 0.5 * (self.max_x - self.min_x),
            self.min_y,
            self.min_z + 0.5 * (self.max_z - self.min_z),
        )
    }

    pub fn get_min_pos(&self) -> Vec3 {
        Vec3::new(self.min_x, self.min_y, self.min_z)
    }

    pub fn get_max_pos(&self) -> Vec3 {
        Vec3::new(self.max_x, self.max_y, self.max_z)
    }

    pub fn of_size(center: Vec3, size_x: f64, size_y: f64, size_z: f64) -> Self {
        AABB::new(
            center.x - size_x / 2.0,
            center.y - size_y / 2.0,
            center.z - size_z / 2.0,
            center.x + size_x / 2.0,
            center.y + size_y / 2.0,
            center.z + size_z / 2.0,
        )
    }
}

fn clip_aabb(
    aabb: AABB,
    from: Vec3,
    scale_reference: &mut f64,
    mut direction: Option<Direction>,
    dx: f64,
    dy: f64,
    dz: f64,
) -> Option<Direction> {
    if dx > 1.0E-7 {
        direction = clip_point(
            scale_reference,
            direction,
            dx,
            dy,
            dz,
            aabb.min_x,
            aabb.min_y,
            aabb.max_y,
            aabb.min_z,
            aabb.max_z,
            Direction::West,
            from.x,
            from.y,
            from.z,
        );
    } else if dx < -1.0E-7 {
        direction = clip_point(
            scale_reference,
            direction,
            dx,
            dy,
            dz,
            aabb.max_x,
            aabb.min_y,
            aabb.max_y,
            aabb.min_z,
            aabb.max_z,
            Direction::East,
            from.x,
            from.y,
            from.z,
        );
    }

    if dy > 1.0E-7 {
        direction = clip_point(
            scale_reference,
            direction,
            dy,
            dz,
            dx,
            aabb.min_y,
            aabb.min_z,
            aabb.max_z,
            aabb.min_x,
            aabb.max_x,
            Direction::Down,
            from.y,
            from.z,
            from.x,
        );
    } else if dy < -1.0E-7 {
        direction = clip_point(
            scale_reference,
            direction,
            dy,
            dz,
            dx,
            aabb.max_y,
            aabb.min_z,
            aabb.max_z,
            aabb.min_x,
            aabb.max_x,
            Direction::Up,
            from.y,
            from.z,
            from.x,
        );
    }

    if dz > 1.0E-7 {
        direction = clip_point(
            scale_reference,
            direction,
            dz,
            dx,
            dy,
            aabb.min_z,
            aabb.min_x,
            aabb.max_x,
            aabb.min_y,
            aabb.max_y,
            Direction::North,
            from.z,
            from.x,
            from.y,
        );
    } else if dz < -1.0E-7 {
        direction = clip_point(
            scale_reference,
            direction,
            dz,
            dx,
            dy,
            aabb.max_z,
            aabb.min_x,
            aabb.max_x,
            aabb.min_y,
            aabb.max_y,
            Direction::South,
            from.z,
            from.x,
            from.y,
        );
    }

    direction
}

fn clip_point(
    scale_reference: &mut f64,
    direction: Option<Direction>,
    da: f64,
    db: f64,
    dc: f64,
    point: f64,
    min_b: f64,
    max_b: f64,
    min_c: f64,
    max_c: f64,
    new_direction: Direction,
    from_a: f64,
    from_b: f64,
    from_c: f64,
) -> Option<Direction> {
    let s = (point - from_a) / da;
    let pb = from_b + s * db;
    let pc = from_c + s * dc;
    if 0.0 < s
        && s < *scale_reference
        && min_b - 1.0E-7 < pb
        && pb < max_b + 1.0E-7
        && min_c - 1.0E-7 < pc
        && pc < max_c + 1.0E-7
    {
        *scale_reference = s;
        Some(new_direction)
    } else {
        direction
    }
}
