use crate::{BlockPos, ChunkPos, Direction, Vec3i};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SectionPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl SectionPos {
    pub const SECTION_BITS: i32 = 4;
    pub const SECTION_SIZE: i32 = 16;
    pub const SECTION_BLOCK_COUNT: i32 = 4096;
    pub const SECTION_MASK: i32 = 15;
    pub const SECTION_HALF_SIZE: i32 = 8;
    pub const SECTION_MAX_INDEX: i32 = 15;

    const PACKED_X_MASK: i64 = 4_194_303;
    const PACKED_Y_MASK: i64 = 1_048_575;
    const PACKED_Z_MASK: i64 = 4_194_303;
    const Y_OFFSET: u32 = 0;
    const Z_OFFSET: u32 = 20;
    const X_OFFSET: u32 = 42;

    pub fn new(x: i32, y: i32, z: i32) -> Self {
        SectionPos { x, y, z }
    }

    pub fn from_block_pos(pos: BlockPos) -> Self {
        SectionPos::new(
            block_to_section_coord(pos.x),
            block_to_section_coord(pos.y),
            block_to_section_coord(pos.z),
        )
    }

    pub fn from_chunk_pos(pos: ChunkPos, section_y: i32) -> Self {
        SectionPos::new(pos.x, section_y, pos.z)
    }

    pub fn from_long(section_node: i64) -> Self {
        SectionPos::new(
            Self::x_from_long(section_node),
            Self::y_from_long(section_node),
            Self::z_from_long(section_node),
        )
    }

    pub fn offset_long(section_node: i64, direction: Direction) -> i64 {
        Self::offset_long_by(
            section_node,
            direction.step_x(),
            direction.step_y(),
            direction.step_z(),
        )
    }

    pub fn offset_long_by(section_node: i64, step_x: i32, step_y: i32, step_z: i32) -> i64 {
        Self::as_long_coords(
            Self::x_from_long(section_node).wrapping_add(step_x),
            Self::y_from_long(section_node).wrapping_add(step_y),
            Self::z_from_long(section_node).wrapping_add(step_z),
        )
    }

    pub fn pos_to_section_coord(pos: f64) -> i32 {
        block_to_section_coord(crate::mth::floor_f64(pos))
    }

    pub fn block_to_section_coord_f64(coord: f64) -> i32 {
        crate::mth::floor_f64(coord) >> Self::SECTION_BITS
    }

    pub fn section_relative(block_coord: i32) -> i32 {
        block_coord & Self::SECTION_MASK
    }

    pub fn section_relative_pos(pos: BlockPos) -> i16 {
        Self::section_relative_pos_coords(pos.x, pos.y, pos.z)
    }

    pub fn section_relative_pos_coords(x: i32, y: i32, z: i32) -> i16 {
        let relative_x = Self::section_relative(x);
        let relative_y = Self::section_relative(y);
        let relative_z = Self::section_relative(z);
        ((relative_x << 8) | (relative_z << 4) | relative_y) as i16
    }

    pub fn section_relative_x(relative: i16) -> i32 {
        ((relative as u16) >> 8 & 15) as i32
    }

    pub fn section_relative_y(relative: i16) -> i32 {
        ((relative as u16) & 15) as i32
    }

    pub fn section_relative_z(relative: i16) -> i32 {
        ((relative as u16) >> 4 & 15) as i32
    }

    pub fn relative_to_block_x(self, relative: i16) -> i32 {
        self.min_block_x()
            .wrapping_add(Self::section_relative_x(relative))
    }

    pub fn relative_to_block_y(self, relative: i16) -> i32 {
        self.min_block_y()
            .wrapping_add(Self::section_relative_y(relative))
    }

    pub fn relative_to_block_z(self, relative: i16) -> i32 {
        self.min_block_z()
            .wrapping_add(Self::section_relative_z(relative))
    }

    pub fn relative_to_block_pos(self, relative: i16) -> BlockPos {
        BlockPos::new(
            self.relative_to_block_x(relative),
            self.relative_to_block_y(relative),
            self.relative_to_block_z(relative),
        )
    }

    pub fn x_from_long(section_node: i64) -> i32 {
        (section_node.wrapping_shl(0) >> 42) as i32
    }

    pub fn y_from_long(section_node: i64) -> i32 {
        (section_node.wrapping_shl(44) >> 44) as i32
    }

    pub fn z_from_long(section_node: i64) -> i32 {
        (section_node.wrapping_shl(22) >> 42) as i32
    }

    pub fn min_block_x(self) -> i32 {
        section_to_block_coord(self.x)
    }

    pub fn min_block_y(self) -> i32 {
        section_to_block_coord(self.y)
    }

    pub fn min_block_z(self) -> i32 {
        section_to_block_coord(self.z)
    }

    pub fn max_block_x(self) -> i32 {
        section_to_block_coord_with_offset(self.x, 15)
    }

    pub fn max_block_y(self) -> i32 {
        section_to_block_coord_with_offset(self.y, 15)
    }

    pub fn max_block_z(self) -> i32 {
        section_to_block_coord_with_offset(self.z, 15)
    }

    pub fn block_to_section(block_node: i64) -> i64 {
        Self::as_long_coords(
            block_to_section_coord(BlockPos::from_long(block_node).x),
            block_to_section_coord(BlockPos::from_long(block_node).y),
            block_to_section_coord(BlockPos::from_long(block_node).z),
        )
    }

    pub fn zero_node(section_node: i64) -> i64 {
        section_node & -1_048_576
    }

    pub fn zero_node_from_xz(x: i32, z: i32) -> i64 {
        Self::zero_node(Self::as_long_coords(x, 0, z))
    }

    pub fn section_to_chunk(section_node: i64) -> i64 {
        ChunkPos::pack_coords(
            Self::x_from_long(section_node),
            Self::z_from_long(section_node),
        )
    }

    pub fn origin(self) -> BlockPos {
        BlockPos::new(
            section_to_block_coord(self.x),
            section_to_block_coord(self.y),
            section_to_block_coord(self.z),
        )
    }

    pub fn center(self) -> BlockPos {
        self.origin().add(8, 8, 8)
    }

    pub fn chunk(self) -> ChunkPos {
        ChunkPos::new(self.x, self.z)
    }

    pub fn as_long_block_pos(pos: BlockPos) -> i64 {
        Self::as_long_coords(
            block_to_section_coord(pos.x),
            block_to_section_coord(pos.y),
            block_to_section_coord(pos.z),
        )
    }

    pub fn as_long(self) -> i64 {
        Self::as_long_coords(self.x, self.y, self.z)
    }

    pub fn as_long_coords(x: i32, y: i32, z: i32) -> i64 {
        let mut node = 0_i64;
        node |= ((x as i64) & Self::PACKED_X_MASK).wrapping_shl(Self::X_OFFSET);
        node |= ((y as i64) & Self::PACKED_Y_MASK).wrapping_shl(Self::Y_OFFSET);
        node | ((z as i64) & Self::PACKED_Z_MASK).wrapping_shl(Self::Z_OFFSET)
    }

    pub fn offset(self, x: i32, y: i32, z: i32) -> Self {
        if x == 0 && y == 0 && z == 0 {
            self
        } else {
            SectionPos::new(
                self.x.wrapping_add(x),
                self.y.wrapping_add(y),
                self.z.wrapping_add(z),
            )
        }
    }

    pub fn as_vec3i(self) -> Vec3i {
        Vec3i::new(self.x, self.y, self.z)
    }
}

pub fn block_to_section_coord(block_coord: i32) -> i32 {
    block_coord >> SectionPos::SECTION_BITS
}

pub fn section_to_block_coord(section_coord: i32) -> i32 {
    section_coord.wrapping_shl(SectionPos::SECTION_BITS as u32)
}

pub fn section_to_block_coord_with_offset(section_coord: i32, offset: i32) -> i32 {
    section_to_block_coord(section_coord).wrapping_add(offset)
}
