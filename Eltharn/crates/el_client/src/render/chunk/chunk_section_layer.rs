#![allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChunkSectionLayer {
    Solid,
    Cutout,
    Translucent,
}
impl ChunkSectionLayer {
    pub const VALUES: [Self; 3] = [Self::Solid, Self::Cutout, Self::Translucent];
    pub const fn buffer_size(self) -> usize {
        match self {
            Self::Solid | Self::Cutout => 4 * 1024 * 1024,
            Self::Translucent => 768 * 1024,
        }
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Solid => "solid",
            Self::Cutout => "cutout",
            Self::Translucent => "translucent",
        }
    }
    pub const fn translucent(self) -> bool {
        matches!(self, Self::Translucent)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChunkSectionLayerGroup {
    Opaque,
    Translucent,
}
impl ChunkSectionLayerGroup {
    pub const fn layers(self) -> &'static [ChunkSectionLayer] {
        match self {
            Self::Opaque => &[ChunkSectionLayer::Solid, ChunkSectionLayer::Cutout],
            Self::Translucent => &[ChunkSectionLayer::Translucent],
        }
    }
}
