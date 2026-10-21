use crate::world::client_level::{ChunkPos, ClientLevel};
use el_mod_content::scene_types::{surface::*, world::BlockDescriptor};
pub(crate) const SCAN_BUDGET: usize = 2048;
#[derive(Clone)]
pub(crate) struct SurfaceCache {
    world: u64,
    epoch: u64,
    owner: String,
    revisions: Vec<Option<u64>>,
    column: usize,
    next_y: Option<i32>,
    pub snapshot: SurfaceSnapshot,
}
pub(crate) fn sample(
    level: &ClientLevel,
    watch: &SurfaceWatch,
    owner: &str,
    epoch: u64,
    next_sample: &mut u64,
    previous: Option<&SurfaceCache>,
    budget: usize,
) -> Result<(SurfaceCache, usize), String> {
    watch.validate()?;
    if watch.columns() == 0 {
        return Err("cannot sample cancelled surface".into());
    }
    let mut revisions = Vec::new();
    for z in (watch.origin[1] >> 4)..=((watch.origin[1] + i32::from(watch.size[1]) - 1) >> 4) {
        for x in (watch.origin[0] >> 4)..=((watch.origin[0] + i32::from(watch.size[0]) - 1) >> 4) {
            revisions.push(level.block_chunk_revision(x, z));
        }
    }
    let previous = previous.filter(|p| {
        p.world == level.extension_identity
            && p.epoch == epoch
            && p.owner == owner
            && p.snapshot.watch == *watch
            && p.revisions == revisions
    });
    let mut cache = previous.cloned().unwrap_or_else(|| SurfaceCache {
        world: level.extension_identity,
        epoch,
        owner: owner.into(),
        revisions,
        column: 0,
        next_y: None,
        snapshot: SurfaceSnapshot {
            watch: watch.clone(),
            handle: String::new(),
            status: SurfaceStatus::Sampling,
            columns: vec![SurfaceColumn::Pending; watch.columns()],
        },
    });
    if cache.snapshot.status != SurfaceStatus::Sampling {
        return Ok((cache, 0));
    }
    let mut reads = 0;
    let mut changed = previous.is_none();
    while cache.column < watch.columns() && reads < budget.min(SCAN_BUDGET) {
        let index = cache.column;
        let x = watch.origin[0] + (index % usize::from(watch.size[0])) as i32;
        let z = watch.origin[1] + (index / usize::from(watch.size[0])) as i32;
        let chunk = level.chunks.get(&ChunkPos::new(x >> 4, z >> 4));
        let result = if let Some(chunk) = chunk {
            let min_y = chunk.min_y_section * 16;
            let y = cache
                .next_y
                .unwrap_or(min_y + chunk.sections.len() as i32 * 16 - 1);
            if y < min_y {
                Some(SurfaceColumn::Empty)
            } else {
                reads += 1;
                let state = chunk
                    .get_block_state(&el_math::block_pos::BlockPos::new(x, y, z))
                    .ok_or("surface scan section missing")?;
                cache.next_y = Some(y - 1);
                if state.is_known_vanilla_air() {
                    None
                } else {
                    let descriptor = BlockDescriptor {
                        state_id: u32::try_from(state.id).map_err(|_| "state ID overflow")?,
                        name: state.block.to_string(),
                        properties: state
                            .properties
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect(),
                    };
                    descriptor.validate()?;
                    Some(SurfaceColumn::Loaded {
                        y,
                        state: descriptor,
                    })
                }
            }
        } else {
            Some(SurfaceColumn::Unloaded)
        };
        if let Some(result) = result {
            cache.snapshot.columns[index] = result;
            cache.column += 1;
            cache.next_y = None;
            changed = true;
        }
    }
    if cache.column == watch.columns() {
        cache.snapshot.status = SurfaceStatus::Ready;
    }
    if changed {
        *next_sample = next_sample
            .checked_add(1)
            .ok_or("surface sequence overflow")?;
        cache.snapshot.handle = format!("{owner}|{epoch}|{}|{}", watch.query, next_sample);
        if serde_json::to_vec(&cache.snapshot)
            .map_err(|e| e.to_string())?
            .len()
            > MAX_SURFACE_BYTES
        {
            cache.snapshot.budget_limit();
        }
    }
    cache.snapshot.validate()?;
    Ok((cache, reads))
}
