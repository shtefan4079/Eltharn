use crate::world::client_level::ClientLevel;
use el_math::block_pos::BlockPos;
use el_mod_content::scene_types::{
    world::{BlockCell, BlockDescriptor},
    world_region::{
        MAX_REGION_BYTES, MAX_REGION_PALETTE, RegionSnapshot, RegionStatus, RegionWatch,
    },
};
use std::collections::BTreeMap;
#[derive(Clone)]
pub(crate) struct RegionCache {
    revisions: Vec<Option<u64>>,
    pub snapshot: RegionSnapshot,
}
pub(crate) fn sample(
    level: &ClientLevel,
    watch: &RegionWatch,
    owner: &str,
    epoch: u64,
    next_sample: &mut u64,
    previous: Option<&RegionCache>,
) -> Result<RegionCache, String> {
    watch.validate()?;
    if watch.cells() == 0 {
        return Err("cannot sample cancelled region".into());
    }
    let mut revisions = Vec::new();
    let [x, _, z] = watch.origin;
    for cz in (z >> 4)..=((z + i32::from(watch.size[2]) - 1) >> 4) {
        for cx in (x >> 4)..=((x + i32::from(watch.size[0]) - 1) >> 4) {
            revisions.push(level.block_chunk_revision(cx, cz));
        }
    }
    if let Some(previous) =
        previous.filter(|old| old.snapshot.watch == *watch && old.revisions == revisions)
    {
        return Ok(previous.clone());
    }
    *next_sample = next_sample
        .checked_add(1)
        .ok_or("region sample sequence exhausted")?;
    let handle = format!("{owner}|{epoch}|{}|{}", watch.query, next_sample);
    let mut snapshot = RegionSnapshot {
        watch: watch.clone(),
        handle,
        status: RegionStatus::Ready,
        palette: Vec::new(),
        indices: Vec::with_capacity(watch.cells()),
    };
    let mut palette = BTreeMap::new();
    for dy in 0..i32::from(watch.size[1]) {
        for dz in 0..i32::from(watch.size[2]) {
            for dx in 0..i32::from(watch.size[0]) {
                let position = BlockPos::new(
                    watch.origin[0] + dx,
                    watch.origin[1] + dy,
                    watch.origin[2] + dz,
                );
                let cell = if !level.has_chunk_at(position) {
                    BlockCell::Unloaded
                } else if let Some(state) = level.get_block_state_ref(position) {
                    BlockCell::Loaded {
                        state: BlockDescriptor {
                            state_id: u32::try_from(state.id)
                                .map_err(|_| "block state ID overflow")?,
                            name: state.block.to_string(),
                            properties: state
                                .properties
                                .iter()
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect(),
                        },
                    }
                } else {
                    BlockCell::OutsideSections
                };
                let index = if let Some(&index) = palette.get(&cell) {
                    index
                } else {
                    if palette.len() >= MAX_REGION_PALETTE {
                        snapshot.status = RegionStatus::PaletteLimit;
                        snapshot.palette.clear();
                        snapshot.indices.clear();
                        return Ok(RegionCache {
                            revisions,
                            snapshot,
                        });
                    }
                    if let BlockCell::Loaded { state } = &cell {
                        state.validate()?;
                    }
                    let index = palette.len() as u16;
                    palette.insert(cell.clone(), index);
                    snapshot.palette.push(cell);
                    index
                };
                snapshot.indices.push(index);
            }
        }
    }
    if serde_json::to_vec(&snapshot)
        .map_err(|e| e.to_string())?
        .len()
        > MAX_REGION_BYTES
    {
        snapshot.status = RegionStatus::BudgetLimit;
        snapshot.palette.clear();
        snapshot.indices.clear();
    }
    snapshot.validate()?;
    Ok(RegionCache {
        revisions,
        snapshot,
    })
}
