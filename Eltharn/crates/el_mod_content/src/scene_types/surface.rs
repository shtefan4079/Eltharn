use super::world::BlockDescriptor;
use serde::{Deserialize, Serialize};
pub const MAX_SURFACE_BYTES: usize = 48 * 1024;
pub const MAX_SURFACE_PAGE: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceWatch {
    pub version: u32,
    pub query: u32,
    pub origin: [i32; 2],
    pub size: [u8; 2],
}
impl SurfaceWatch {
    pub fn columns(&self) -> usize {
        usize::from(self.size[0]) * usize::from(self.size[1])
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.query == 0 {
            return Err("unsupported surface query".into());
        }
        super::blk_ok([self.origin[0], 0, self.origin[1]])?;
        if self.size == [0, 0] {
            return Ok(());
        }
        if self.size.iter().any(|v| !(1..=16).contains(v)) {
            return Err("surface extent exceeds 16 per axis".into());
        }
        super::blk_ok([
            self.origin[0] + i32::from(self.size[0]) - 1,
            0,
            self.origin[1] + i32::from(self.size[1]) - 1,
        ])
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum SurfaceColumn {
    Pending,
    Unloaded,
    Empty,
    Loaded { y: i32, state: BlockDescriptor },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceStatus {
    Sampling,
    Ready,
    BudgetLimit,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceSnapshot {
    pub watch: SurfaceWatch,
    pub handle: String,
    pub status: SurfaceStatus,
    pub columns: Vec<SurfaceColumn>,
}
impl SurfaceSnapshot {
    pub fn budget_limit(&mut self) {
        self.status = SurfaceStatus::BudgetLimit;
        self.columns.clear();
    }
    pub fn validate(&self) -> Result<(), String> {
        self.watch.validate()?;
        if self.watch.columns() == 0
            || self.handle.is_empty()
            || self.handle.len() > 256
            || self.handle.contains('\0')
        {
            return Err("invalid surface identity".into());
        }
        if self.status == SurfaceStatus::BudgetLimit {
            if !self.columns.is_empty() {
                return Err("over-budget surface cannot carry columns".into());
            }
        } else {
            if self.columns.len() != self.watch.columns() {
                return Err("surface column count mismatch".into());
            }
            let pending = self
                .columns
                .iter()
                .any(|c| matches!(c, SurfaceColumn::Pending));
            if (self.status == SurfaceStatus::Sampling) != pending {
                return Err("surface progress mismatch".into());
            }
            for column in &self.columns {
                if let SurfaceColumn::Loaded { y, state } = column {
                    super::blk_ok([0, *y, 0])?;
                    state.validate()?;
                }
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > MAX_SURFACE_BYTES {
            return Err("surface byte limit".into());
        }
        Ok(())
    }
}
