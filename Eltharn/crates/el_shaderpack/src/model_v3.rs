use std::collections::BTreeMap;

pub const GRAPH_V3_VERSION: u64 = 3;

pub const MAX_V3_PASSES: usize = 32;
pub const MAX_V3_RESOURCES: usize = 64;
pub const MAX_V3_INPUTS_PER_PASS: usize = 8;
pub const MAX_V3_OUTPUTS_PER_PASS: usize = 4;
pub const MAX_V3_SAMPLERS_PER_PASS: usize = 4;
pub const MAX_V3_FINAL_OUTPUTS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StageV3 {
    Geometry,
    Shadow,
    Lighting,
    Post,
    Compute,
}

impl StageV3 {
    pub fn as_str(self) -> &'static str {
        match self {
            StageV3::Geometry => "geometry",
            StageV3::Shadow => "shadow",
            StageV3::Lighting => "lighting",
            StageV3::Post => "post",
            StageV3::Compute => "compute",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "geometry" => Some(StageV3::Geometry),
            "shadow" => Some(StageV3::Shadow),
            "lighting" => Some(StageV3::Lighting),
            "post" => Some(StageV3::Post),
            "compute" => Some(StageV3::Compute),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextureFormatV3 {
    Rgba8Unorm,
    Rgba16Float,
    Depth32Float,
}

impl TextureFormatV3 {
    pub fn as_str(self) -> &'static str {
        match self {
            TextureFormatV3::Rgba8Unorm => "rgba8unorm",
            TextureFormatV3::Rgba16Float => "rgba16float",
            TextureFormatV3::Depth32Float => "depth32float",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "rgba8unorm" => Some(TextureFormatV3::Rgba8Unorm),
            "rgba16float" => Some(TextureFormatV3::Rgba16Float),
            "depth32float" => Some(TextureFormatV3::Depth32Float),
            _ => None,
        }
    }

    pub fn bytes_per_texel(self) -> u64 {
        match self {
            TextureFormatV3::Rgba8Unorm => 4,
            TextureFormatV3::Rgba16Float => 8,
            TextureFormatV3::Depth32Float => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextureUsageV3 {
    ColorTarget,
    DepthTarget,
    Storage,
}

impl TextureUsageV3 {
    pub fn as_str(self) -> &'static str {
        match self {
            TextureUsageV3::ColorTarget => "color_target",
            TextureUsageV3::DepthTarget => "depth_target",
            TextureUsageV3::Storage => "storage",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "color_target" => Some(TextureUsageV3::ColorTarget),
            "depth_target" => Some(TextureUsageV3::DepthTarget),
            "storage" => Some(TextureUsageV3::Storage),
            _ => None,
        }
    }
}

pub fn format_usage_compatible(format: TextureFormatV3, usage: TextureUsageV3) -> bool {
    match (format, usage) {
        (TextureFormatV3::Rgba8Unorm, TextureUsageV3::ColorTarget) => true,
        (TextureFormatV3::Rgba8Unorm, TextureUsageV3::Storage) => true,
        (TextureFormatV3::Rgba16Float, TextureUsageV3::ColorTarget) => true,
        (TextureFormatV3::Rgba16Float, TextureUsageV3::Storage) => true,
        (TextureFormatV3::Depth32Float, TextureUsageV3::DepthTarget) => true,
        _ => false,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceV3 {
    Texture {
        id: String,
        format: TextureFormatV3,
        usage: TextureUsageV3,
        samples: u32,
    },
    Buffer {
        id: String,
        size_bytes: u64,
        usage: TextureUsageV3,
    },
    Sampler {
        id: String,
    },
}

impl ResourceV3 {
    pub fn id(&self) -> &str {
        match self {
            ResourceV3::Texture { id, .. }
            | ResourceV3::Buffer { id, .. }
            | ResourceV3::Sampler { id } => id,
        }
    }

    pub fn is_texture(&self) -> bool {
        matches!(self, ResourceV3::Texture { .. })
    }

    pub fn is_buffer(&self) -> bool {
        matches!(self, ResourceV3::Buffer { .. })
    }

    pub fn is_sampler(&self) -> bool {
        matches!(self, ResourceV3::Sampler { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassV3 {
    pub id: String,
    pub stage: StageV3,
    pub inputs: Vec<String>,
    pub samplers: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceLifetimeV3 {
    pub first_write_step: usize,
    pub last_use_step: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedGraphV3 {
    pub version: u64,
    pub min_texture_dimension: u32,
    pub passes: Vec<PassV3>,
    pub resources: BTreeMap<String, ResourceV3>,
    pub final_outputs: Vec<String>,
    pub lifetimes: BTreeMap<String, ResourceLifetimeV3>,
}

impl ValidatedGraphV3 {
    pub fn logical_bytes(&self, width: u64, height: u64) -> Option<u64> {
        let mut total: u64 = 0;
        for resource in self.resources.values() {
            let cost = match resource {
                ResourceV3::Texture { format, .. } => width
                    .checked_mul(height)?
                    .checked_mul(format.bytes_per_texel())?,
                ResourceV3::Buffer { size_bytes, .. } => *size_bytes,
                ResourceV3::Sampler { .. } => 0,
            };
            total = total.checked_add(cost)?;
        }
        Some(total)
    }
}
