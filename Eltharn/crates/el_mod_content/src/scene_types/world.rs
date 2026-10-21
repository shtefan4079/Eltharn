use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_BLOCK_POSITIONS: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockWatch {
    pub version: u32,
    pub query: u32,
    pub positions: Vec<[i32; 3]>,
}
impl BlockWatch {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.query == 0 || self.positions.len() > MAX_BLOCK_POSITIONS {
            return Err("unsupported or oversized sparse block subscription".into());
        }
        for position in &self.positions {
            super::blk_ok(*position)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockDescriptor {
    pub state_id: u32,
    pub name: String,
    pub properties: BTreeMap<String, String>,
}
impl BlockDescriptor {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty()
            || self.name.len() > 256
            || self.name.contains('\0')
            || !self.name.contains(':')
            || self.properties.len() > 32
            || self.properties.iter().any(|(key, value)| {
                key.is_empty()
                    || key.len() > 128
                    || value.len() > 128
                    || key.contains('\0')
                    || value.contains('\0')
            })
        {
            return Err("invalid loaded block descriptor".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlockCell {
    Loaded { state: BlockDescriptor },
    Unloaded,
    OutsideSections,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldBlocksSnapshot {
    pub watch: BlockWatch,
    pub cells: Vec<BlockCell>,
}
impl WorldBlocksSnapshot {
    pub fn validate(&self) -> Result<(), String> {
        self.watch.validate()?;
        if self.cells.len() != self.watch.positions.len() {
            return Err("sparse block result length mismatch".into());
        }
        for cell in &self.cells {
            if let BlockCell::Loaded { state } = cell {
                state.validate()?;
            }
        }
        Ok(())
    }
}
