use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Color,
    ColorDepth,
    ColorDepthEnvironment,
}

impl Profile {
    pub fn has_depth(self) -> bool {
        matches!(self, Self::ColorDepth | Self::ColorDepthEnvironment)
    }

    pub fn has_environment(self) -> bool {
        matches!(self, Self::ColorDepthEnvironment)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Color => "post_color_v1",
            Self::ColorDepth => "post_color_depth_v1",
            Self::ColorDepthEnvironment => "post_color_depth_environment_v1",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    pub source: String,
    pub vertex_entry: String,
    pub fragment_entry: String,
    pub has_params: bool,
}

#[derive(Clone, Debug)]
pub struct Pass {
    pub id: String,
    pub program: String,
    pub input: String,
    pub sampler: String,
    pub output: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureLifetime {
    pub first_write_step: usize,
    pub last_use_step: usize,
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub id: String,
    pub capabilities: Vec<String>,
    pub min_texture_dimension: u32,
    pub params: Option<Vec<u8>>,
    pub textures: Vec<String>,
    pub samplers: Vec<String>,
    pub passes: Vec<Pass>,
    pub final_output: String,
    pub texture_lifetimes: BTreeMap<String, TextureLifetime>,
}

#[derive(Clone, Debug)]
pub struct ValidatedPack {
    pub id: String,
    pub content_id: String,
    pub profile: Profile,
    pub programs: BTreeMap<String, Program>,
    pub variants: Vec<Variant>,
}

pub fn validate_environment_bytes(bytes: &[u8; 80]) -> Result<(), String> {
    let word = |offset: usize| -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("4 bytes"))
    };
    for offset in [16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56] {
        if !f32::from_bits(word(offset)).is_finite() {
            return Err(format!("nonfinite environment word at offset {offset}"));
        }
    }
    if word(60) != 0 {
        return Err("environment sky_light_color padding must be zero".into());
    }
    if word(64) >= 8 {
        return Err("environment moon phase out of range".into());
    }
    if bytes[68..80].iter().any(|byte| *byte != 0) {
        return Err("environment moon_phase padding must be zero".into());
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct FrameData {
    pub view_projection: [[f32; 4]; 4],
    pub inverse_view_projection: [[f32; 4]; 4],
    pub camera: [f32; 4],
    pub viewport: [f32; 4],
    pub planes: [f32; 4],
}

impl FrameData {
    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        let values = self
            .view_projection
            .iter()
            .flatten()
            .chain(self.inverse_view_projection.iter().flatten())
            .chain(self.camera.iter())
            .chain(self.viewport.iter())
            .chain(self.planes.iter());
        let mut out = Vec::with_capacity(176);
        for value in values {
            if !value.is_finite() {
                return Err("nonfinite camera uniform".into());
            }
            out.extend_from_slice(&value.to_le_bytes());
        }
        if self.camera[3] != 0.0
            || self.planes[2..] != [0.0, 0.0]
            || self.planes[0] <= 0.0
            || self.planes[1] <= self.planes[0]
            || self.viewport.iter().any(|v| *v <= 0.0)
        {
            return Err("invalid camera metadata".into());
        }
        Ok(out)
    }
}
