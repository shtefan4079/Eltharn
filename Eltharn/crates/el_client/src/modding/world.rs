use crate::world::client_level::ClientLevel;
use el_math::block_pos::BlockPos;
use el_mod_content::scene_types::world::{
    BlockCell, BlockDescriptor, BlockWatch, WorldBlocksSnapshot,
};
pub fn sample(level: &ClientLevel, watch: &BlockWatch) -> Result<WorldBlocksSnapshot, String> {
    watch.validate()?;
    let cells = watch
        .positions
        .iter()
        .map(|&[x, y, z]| {
            let position = BlockPos::new(x, y, z);
            if !level.has_chunk_at(position) {
                return Ok(BlockCell::Unloaded);
            }
            let Some(state) = level.get_block_state_ref(position) else {
                return Ok(BlockCell::OutsideSections);
            };
            Ok(BlockCell::Loaded {
                state: BlockDescriptor {
                    state_id: u32::try_from(state.id).map_err(|_| "block state ID overflow")?,
                    name: state.block.to_string(),
                    properties: state
                        .properties
                        .iter()
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect(),
                },
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let snapshot = WorldBlocksSnapshot {
        watch: watch.clone(),
        cells,
    };
    snapshot.validate()?;
    Ok(snapshot)
}
