use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    const PACKED_X_LENGTH: i32 = 26;
    const PACKED_Z_LENGTH: i32 = 26;
    const PACKED_Y_LENGTH: i32 = 64 - Self::PACKED_X_LENGTH - Self::PACKED_Z_LENGTH;
    const PACKED_X_MASK: i64 = (1 << Self::PACKED_X_LENGTH) - 1;
    const PACKED_Y_MASK: i64 = (1 << Self::PACKED_Y_LENGTH) - 1;
    const PACKED_Z_MASK: i64 = (1 << Self::PACKED_Z_LENGTH) - 1;
    const Z_OFFSET: i32 = Self::PACKED_Y_LENGTH;
    const X_OFFSET: i32 = Self::PACKED_Y_LENGTH + Self::PACKED_Z_LENGTH;

    pub const ZERO: BlockPos = BlockPos { x: 0, y: 0, z: 0 };

    pub fn new(x: i32, y: i32, z: i32) -> Self {
        BlockPos { x, y, z }
    }

    pub fn from_long(packed: i64) -> Self {
        let x = (packed << (64 - Self::X_OFFSET - Self::PACKED_X_LENGTH)
            >> (64 - Self::PACKED_X_LENGTH)) as i32;
        let y = (packed << (64 - Self::PACKED_Y_LENGTH) >> (64 - Self::PACKED_Y_LENGTH)) as i32;
        let z = (packed << (64 - Self::Z_OFFSET - Self::PACKED_Z_LENGTH)
            >> (64 - Self::PACKED_Z_LENGTH)) as i32;
        BlockPos { x, y, z }
    }

    pub fn as_long(&self) -> i64 {
        let mut res = 0i64;
        res |= (self.x as i64 & Self::PACKED_X_MASK) << Self::X_OFFSET;
        res |= (self.y as i64 & Self::PACKED_Y_MASK) << 0;
        res |= (self.z as i64 & Self::PACKED_Z_MASK) << Self::Z_OFFSET;
        res
    }

    pub fn add(&self, x: i32, y: i32, z: i32) -> Self {
        BlockPos::new(
            self.x.wrapping_add(x),
            self.y.wrapping_add(y),
            self.z.wrapping_add(z),
        )
    }

    pub fn above(&self) -> Self {
        self.add(0, 1, 0)
    }

    pub fn below(&self) -> Self {
        self.add(0, -1, 0)
    }

    pub fn relative(&self, direction: crate::direction::Direction) -> Self {
        self.add(direction.step_x(), direction.step_y(), direction.step_z())
    }

    pub fn dist_sq(&self, other: &BlockPos) -> f64 {
        let dx = (self.x - other.x) as f64;
        let dy = (self.y - other.y) as f64;
        let dz = (self.z - other.z) as f64;
        dx * dx + dy * dy + dz * dz
    }
}

impl fmt::Display for BlockPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BlockPos{{x={}, y={}, z={}}}", self.x, self.y, self.z)
    }
}
