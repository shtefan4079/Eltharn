use super::world::BlockCell;
use serde::{Deserialize, Serialize};

pub const MAX_REGION_CELLS: usize = 4096;
pub const MAX_REGION_PALETTE: usize = 128;
pub const MAX_REGION_BYTES: usize = 48 * 1024;
pub const MAX_REGION_PAGE: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionWatch {
    pub version: u32,
    pub query: u32,
    pub origin: [i32; 3],
    pub size: [u8; 3],
}
impl RegionWatch {
    pub fn cells(&self) -> usize {
        self.size.iter().map(|v| usize::from(*v)).product()
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.query == 0 {
            return Err("unsupported region subscription".into());
        }
        super::blk_ok(self.origin)?;
        if self.size == [0; 3] {
            return Ok(());
        }
        if self.size.iter().any(|v| !(1..=16).contains(v)) || self.cells() > MAX_REGION_CELLS {
            return Err("region extent exceeds 16 per axis".into());
        }
        let end = std::array::from_fn(|i| self.origin[i] + i32::from(self.size[i]) - 1);
        super::blk_ok(end)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionStatus {
    Ready,
    PaletteLimit,
    BudgetLimit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionSnapshot {
    pub watch: RegionWatch,
    pub handle: String,
    pub status: RegionStatus,
    pub palette: Vec<BlockCell>,
    pub indices: Vec<u16>,
}
impl RegionSnapshot {
    pub fn validate(&self) -> Result<(), String> {
        self.watch.validate()?;
        if self.watch.cells() == 0
            || self.handle.is_empty()
            || self.handle.len() > 256
            || self.handle.contains('\0')
        {
            return Err("invalid region sample identity".into());
        }
        if self.palette.len() > MAX_REGION_PALETTE {
            return Err("region palette limit".into());
        }
        if self.status == RegionStatus::Ready {
            if self.indices.len() != self.watch.cells()
                || self.palette.is_empty()
                || self
                    .indices
                    .iter()
                    .any(|&i| usize::from(i) >= self.palette.len())
            {
                return Err("invalid region palette indices".into());
            }
        } else if !self.indices.is_empty() || !self.palette.is_empty() {
            return Err("unavailable region cannot carry partial terrain".into());
        }
        for cell in &self.palette {
            if let BlockCell::Loaded { state } = cell {
                state.validate()?;
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > MAX_REGION_BYTES {
            return Err("region snapshot byte limit".into());
        }
        Ok(())
    }
}
