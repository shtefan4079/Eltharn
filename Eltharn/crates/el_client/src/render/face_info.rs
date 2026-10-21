use el_math::Direction;

#[derive(Debug, Clone, Copy)]
pub enum Extent {
    MinX,
    MinY,
    MinZ,
    MaxX,
    MaxY,
    MaxZ,
}

impl Extent {
    pub fn select(&self, min: glam::Vec3, max: glam::Vec3) -> f32 {
        match self {
            Extent::MinX => min.x,
            Extent::MinY => min.y,
            Extent::MinZ => min.z,
            Extent::MaxX => max.x,
            Extent::MaxY => max.y,
            Extent::MaxZ => max.z,
        }
    }

    pub fn select_f32(
        &self,
        min_x: f32,
        min_y: f32,
        min_z: f32,
        max_x: f32,
        max_y: f32,
        max_z: f32,
    ) -> f32 {
        match self {
            Extent::MinX => min_x,
            Extent::MinY => min_y,
            Extent::MinZ => min_z,
            Extent::MaxX => max_x,
            Extent::MaxY => max_y,
            Extent::MaxZ => max_z,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VertexInfo {
    pub x_face: Extent,
    pub y_face: Extent,
    pub z_face: Extent,
}

impl VertexInfo {
    pub const fn new(x_face: Extent, y_face: Extent, z_face: Extent) -> Self {
        Self {
            x_face,
            y_face,
            z_face,
        }
    }

    pub fn select(&self, min: glam::Vec3, max: glam::Vec3) -> glam::Vec3 {
        glam::Vec3::new(
            self.x_face.select(min, max),
            self.y_face.select(min, max),
            self.z_face.select(min, max),
        )
    }
}

#[derive(Debug, Clone)]
pub struct FaceInfo {
    pub infos: [VertexInfo; 4],
}

impl FaceInfo {
    pub const DOWN: Self = Self {
        infos: [
            VertexInfo::new(Extent::MinX, Extent::MinY, Extent::MaxZ),
            VertexInfo::new(Extent::MinX, Extent::MinY, Extent::MinZ),
            VertexInfo::new(Extent::MaxX, Extent::MinY, Extent::MinZ),
            VertexInfo::new(Extent::MaxX, Extent::MinY, Extent::MaxZ),
        ],
    };
    pub const UP: Self = Self {
        infos: [
            VertexInfo::new(Extent::MinX, Extent::MaxY, Extent::MinZ),
            VertexInfo::new(Extent::MinX, Extent::MaxY, Extent::MaxZ),
            VertexInfo::new(Extent::MaxX, Extent::MaxY, Extent::MaxZ),
            VertexInfo::new(Extent::MaxX, Extent::MaxY, Extent::MinZ),
        ],
    };
    pub const NORTH: Self = Self {
        infos: [
            VertexInfo::new(Extent::MaxX, Extent::MaxY, Extent::MinZ),
            VertexInfo::new(Extent::MaxX, Extent::MinY, Extent::MinZ),
            VertexInfo::new(Extent::MinX, Extent::MinY, Extent::MinZ),
            VertexInfo::new(Extent::MinX, Extent::MaxY, Extent::MinZ),
        ],
    };
    pub const SOUTH: Self = Self {
        infos: [
            VertexInfo::new(Extent::MinX, Extent::MaxY, Extent::MaxZ),
            VertexInfo::new(Extent::MinX, Extent::MinY, Extent::MaxZ),
            VertexInfo::new(Extent::MaxX, Extent::MinY, Extent::MaxZ),
            VertexInfo::new(Extent::MaxX, Extent::MaxY, Extent::MaxZ),
        ],
    };
    pub const WEST: Self = Self {
        infos: [
            VertexInfo::new(Extent::MinX, Extent::MaxY, Extent::MinZ),
            VertexInfo::new(Extent::MinX, Extent::MinY, Extent::MinZ),
            VertexInfo::new(Extent::MinX, Extent::MinY, Extent::MaxZ),
            VertexInfo::new(Extent::MinX, Extent::MaxY, Extent::MaxZ),
        ],
    };
    pub const EAST: Self = Self {
        infos: [
            VertexInfo::new(Extent::MaxX, Extent::MaxY, Extent::MaxZ),
            VertexInfo::new(Extent::MaxX, Extent::MinY, Extent::MaxZ),
            VertexInfo::new(Extent::MaxX, Extent::MinY, Extent::MinZ),
            VertexInfo::new(Extent::MaxX, Extent::MaxY, Extent::MinZ),
        ],
    };

    pub fn from_facing(direction: Direction) -> &'static Self {
        match direction {
            Direction::Down => &Self::DOWN,
            Direction::Up => &Self::UP,
            Direction::North => &Self::NORTH,
            Direction::South => &Self::SOUTH,
            Direction::West => &Self::WEST,
            Direction::East => &Self::EAST,
        }
    }

    pub fn get_vertex_info(&self, index: usize) -> &VertexInfo {
        &self.infos[index]
    }
}
