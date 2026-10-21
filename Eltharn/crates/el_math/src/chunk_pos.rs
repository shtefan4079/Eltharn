use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub const ZERO: ChunkPos = ChunkPos { x: 0, z: 0 };
    pub const INVALID_CHUNK_POS: i64 = Self::pack_coords(1_875_066, 1_875_066);

    const COORD_MASK: u64 = 0xffff_ffff;
    const REGION_BITS: u32 = 5;
    const REGION_MASK: i32 = 31;
    const HASH_A: i32 = 1_664_525;
    const HASH_C: i32 = 1_013_904_223;
    const HASH_Z_XOR: i32 = -559_038_737;

    pub const fn new(x: i32, z: i32) -> Self {
        ChunkPos { x, z }
    }

    pub fn containing_block(block_x: i32, block_z: i32) -> Self {
        ChunkPos::new(
            block_to_section_coord(block_x),
            block_to_section_coord(block_z),
        )
    }

    pub fn unpack(key: i64) -> Self {
        ChunkPos::new(key as i32, (key >> 32) as i32)
    }

    pub fn min_from_region(region_x: i32, region_z: i32) -> Self {
        ChunkPos::new(
            region_x.wrapping_shl(Self::REGION_BITS),
            region_z.wrapping_shl(Self::REGION_BITS),
        )
    }

    pub fn max_from_region(region_x: i32, region_z: i32) -> Self {
        ChunkPos::new(
            region_x
                .wrapping_shl(Self::REGION_BITS)
                .wrapping_add(Self::REGION_MASK),
            region_z
                .wrapping_shl(Self::REGION_BITS)
                .wrapping_add(Self::REGION_MASK),
        )
    }

    pub const fn pack(self) -> i64 {
        Self::pack_coords(self.x, self.z)
    }

    pub const fn pack_coords(x: i32, z: i32) -> i64 {
        ((x as u32 as u64) | ((z as u32 as u64) << 32)) as i64
    }

    pub fn get_x(pos: i64) -> i32 {
        ((pos as u64) & Self::COORD_MASK) as u32 as i32
    }

    pub fn get_z(pos: i64) -> i32 {
        (((pos as u64) >> 32) & Self::COORD_MASK) as u32 as i32
    }

    pub fn java_hash_code(self) -> i32 {
        Self::hash(self.x, self.z)
    }

    pub fn hash(x: i32, z: i32) -> i32 {
        let x_transform = Self::HASH_A.wrapping_mul(x).wrapping_add(Self::HASH_C);
        let z_transform = Self::HASH_A
            .wrapping_mul(z ^ Self::HASH_Z_XOR)
            .wrapping_add(Self::HASH_C);
        x_transform ^ z_transform
    }

    pub fn middle_block_x(self) -> i32 {
        self.block_x(8)
    }

    pub fn middle_block_z(self) -> i32 {
        self.block_z(8)
    }

    pub fn min_block_x(self) -> i32 {
        section_to_block_coord(self.x)
    }

    pub fn min_block_z(self) -> i32 {
        section_to_block_coord(self.z)
    }

    pub fn max_block_x(self) -> i32 {
        self.block_x(15)
    }

    pub fn max_block_z(self) -> i32 {
        self.block_z(15)
    }

    pub fn region_x(self) -> i32 {
        self.x >> Self::REGION_BITS
    }

    pub fn region_z(self) -> i32 {
        self.z >> Self::REGION_BITS
    }

    pub fn region_x_from_packed(pos: i64) -> i32 {
        Self::get_x(pos) >> Self::REGION_BITS
    }

    pub fn region_z_from_packed(pos: i64) -> i32 {
        Self::get_z(pos) >> Self::REGION_BITS
    }

    pub fn region_local_x(self) -> i32 {
        self.x & Self::REGION_MASK
    }

    pub fn region_local_z(self) -> i32 {
        self.z & Self::REGION_MASK
    }

    pub fn block_x(self, offset: i32) -> i32 {
        section_to_block_coord_with_offset(self.x, offset)
    }

    pub fn block_z(self, offset: i32) -> i32 {
        section_to_block_coord_with_offset(self.z, offset)
    }

    pub fn block_at(self, x: i32, y: i32, z: i32) -> (i32, i32, i32) {
        (self.block_x(x), y, self.block_z(z))
    }

    pub fn middle_block_position(self, y: i32) -> (i32, i32, i32) {
        (self.middle_block_x(), y, self.middle_block_z())
    }

    pub fn contains_block(self, block_x: i32, block_z: i32) -> bool {
        block_x >= self.min_block_x()
            && block_z >= self.min_block_z()
            && block_x <= self.max_block_x()
            && block_z <= self.max_block_z()
    }

    pub fn world_position(self) -> (i32, i32, i32) {
        (self.min_block_x(), 0, self.min_block_z())
    }

    pub fn chessboard_distance(self, pos: ChunkPos) -> i32 {
        chessboard_distance(pos.x, pos.z, self.x, self.z)
    }

    pub fn chessboard_distance_to(self, x: i32, z: i32) -> i32 {
        chessboard_distance(x, z, self.x, self.z)
    }

    pub fn distance_squared(self, pos: ChunkPos) -> i32 {
        self.distance_squared_to(pos.x, pos.z)
    }

    pub fn distance_squared_packed(self, pos: i64) -> i32 {
        self.distance_squared_to(Self::get_x(pos), Self::get_z(pos))
    }

    pub fn distance_squared_to(self, x: i32, z: i32) -> i32 {
        let delta_x = x.wrapping_sub(self.x);
        let delta_z = z.wrapping_sub(self.z);
        delta_x
            .wrapping_mul(delta_x)
            .wrapping_add(delta_z.wrapping_mul(delta_z))
    }
}

impl fmt::Display for ChunkPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.x, self.z)
    }
}

pub fn block_to_section_coord(block_coord: i32) -> i32 {
    block_coord >> 4
}

pub fn section_to_block_coord(section_coord: i32) -> i32 {
    section_coord.wrapping_shl(4)
}

pub fn section_to_block_coord_with_offset(section_coord: i32, offset: i32) -> i32 {
    section_to_block_coord(section_coord).wrapping_add(offset)
}

fn chessboard_distance(x0: i32, z0: i32, x1: i32, z1: i32) -> i32 {
    std::cmp::max(
        java_abs_i32(x1.wrapping_sub(x0)),
        java_abs_i32(z1.wrapping_sub(z0)),
    )
}

fn java_abs_i32(value: i32) -> i32 {
    if value == i32::MIN {
        i32::MIN
    } else {
        value.abs()
    }
}
