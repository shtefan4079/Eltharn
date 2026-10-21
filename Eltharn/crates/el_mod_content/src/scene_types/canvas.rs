use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

pub const MAX_CANVAS_PIXELS: usize = 4096;
pub const MAX_CANVAS_PATCH: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvasView {
    pub id: u32,
    pub width: u8,
    pub height: u8,
    pub revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rgba_base64: Option<String>,
}
impl CanvasView {
    pub fn validate(&self) -> Result<(), String> {
        super::id_ok(self.id)?;
        dimensions(self.width, self.height)?;
        if self.revision == 0 {
            return Err("invalid canvas revision".into());
        }
        if let Some(encoded) = &self.rgba_base64 {
            if encoded.len()
                > ((usize::from(self.width) * usize::from(self.height) * 4 + 2) / 3) * 4
            {
                return Err("canvas encoded size limit".into());
            }
            let bytes = STANDARD
                .decode(encoded)
                .map_err(|_| "invalid canvas base64")?;
            if bytes.len() != usize::from(self.width) * usize::from(self.height) * 4 {
                return Err("canvas pixel length mismatch".into());
            }
            if STANDARD.encode(&bytes) != *encoded {
                return Err("noncanonical canvas pixels".into());
            }
        }
        Ok(())
    }
    pub fn pixels(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        STANDARD
            .decode(
                self.rgba_base64
                    .as_ref()
                    .ok_or("canvas pixels unavailable")?,
            )
            .map_err(|_| "invalid canvas base64".into())
    }
}
fn dimensions(width: u8, height: u8) -> Result<(), String> {
    if width == 0
        || height == 0
        || width > 64
        || height > 64
        || usize::from(width) * usize::from(height) > MAX_CANVAS_PIXELS
    {
        Err("canvas dimensions must be 1..64 with at most4096pixels".into())
    } else {
        Ok(())
    }
}
fn rgba(argb: u32) -> [u8; 4] {
    [
        (argb >> 16) as u8,
        (argb >> 8) as u8,
        argb as u8,
        (argb >> 24) as u8,
    ]
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum CanvasCommand {
    Create {
        id: u32,
        width: u8,
        height: u8,
        color: u32,
    },
    Patch {
        id: u32,
        offset: u16,
        pixels: Vec<u32>,
    },
    Clear {
        id: u32,
        color: u32,
    },
    Remove {
        id: u32,
    },
}
impl CanvasCommand {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Create {
                id, width, height, ..
            } => {
                super::id_ok(*id)?;
                dimensions(*width, *height)
            }
            Self::Patch { id, pixels, .. } => {
                super::id_ok(*id)?;
                if pixels.is_empty() || pixels.len() > MAX_CANVAS_PATCH {
                    Err("canvas patch size must be1..64".into())
                } else {
                    Ok(())
                }
            }
            Self::Clear { id, .. } | Self::Remove { id } => super::id_ok(*id),
        }
    }
    pub fn apply(&self, canvases: &mut Vec<CanvasView>) -> Result<(), String> {
        self.validate()?;
        if let Self::Create {
            id,
            width,
            height,
            color,
        } = self
        {
            if !canvases.is_empty() {
                return Err("one canvas per actor".into());
            }
            let pixels = rgba(*color).repeat(usize::from(*width) * usize::from(*height));
            canvases.push(CanvasView {
                id: *id,
                width: *width,
                height: *height,
                revision: 1,
                rgba_base64: Some(STANDARD.encode(pixels)),
            });
            return Ok(());
        }
        let id = match self {
            Self::Patch { id, .. } | Self::Clear { id, .. } | Self::Remove { id } => *id,
            _ => unreachable!(),
        };
        let index = canvases
            .iter()
            .position(|c| c.id == id)
            .ok_or("unknown actor canvas")?;
        if matches!(self, Self::Remove { .. }) {
            canvases.remove(index);
            return Ok(());
        }
        let canvas = &mut canvases[index];
        let mut bytes = canvas.pixels()?;
        match self {
            Self::Patch { offset, pixels, .. } => {
                let start = usize::from(*offset) * 4;
                let end = start + pixels.len() * 4;
                let dst = bytes
                    .get_mut(start..end)
                    .ok_or("canvas patch outside pixels")?;
                for (out, pixel) in dst.chunks_exact_mut(4).zip(pixels) {
                    out.copy_from_slice(&rgba(*pixel));
                }
            }
            Self::Clear { color, .. } => {
                for pixel in bytes.chunks_exact_mut(4) {
                    pixel.copy_from_slice(&rgba(*color));
                }
            }
            _ => unreachable!(),
        }
        let encoded = STANDARD.encode(bytes);
        if canvas.rgba_base64.as_deref() != Some(&encoded) {
            canvas.revision = canvas
                .revision
                .checked_add(1)
                .ok_or("canvas revision exhausted")?;
            canvas.rgba_base64 = Some(encoded);
        }
        Ok(())
    }
}
