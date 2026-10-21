use el_core::block_state::BlockState;
use el_core::registry::BuiltInRegistries;
use el_math::block_pos::BlockPos;
use el_protocol::packets::game::{BlockEntityInfo, LightUpdatePacketData};
use el_protocol::{BufferError, PacketBuffer};
use el_world::chunk::{BIOME_SECTION_VOLUME, BlockEntityData, Chunk, DataLayer, SECTION_VOLUME};
use el_world::paletted_container::PalettedContainer;
const MAX_BLOCK_INDIRECT_BITS: usize = 8;
const MAX_BIOME_INDIRECT_BITS: usize = 3;
fn storage_word_count(capacity: usize, bits: usize) -> usize {
    if bits == 0 {
        0
    } else {
        capacity.div_ceil(64 / bits)
    }
}
fn read_storage(
    input: &mut PacketBuffer,
    capacity: usize,
    bits: usize,
) -> Result<Vec<u64>, BufferError> {
    (0..storage_word_count(capacity, bits))
        .map(|_| input.read_i64().map(|word| word as u64))
        .collect()
}
fn read_registry_state(id: i32) -> Result<BlockState, BufferError> {
    if id < 0 {
        return Err(BufferError::InvalidEnumId {
            name: "block state",
            id,
        });
    }
    BuiltInRegistries::block_states()
        .get_state_by_id(id as usize)
        .cloned()
        .ok_or(BufferError::InvalidEnumId {
            name: "block state",
            id,
        })
}
fn read_block_states(
    input: &mut PacketBuffer,
    default_state: &BlockState,
) -> Result<PalettedContainer<BlockState>, BufferError> {
    let wire_bits = usize::from(input.read_u8()?);
    let palette = if wire_bits == 0 {
        vec![read_registry_state(input.read_var_i32()?)?]
    } else if wire_bits <= MAX_BLOCK_INDIRECT_BITS {
        let palette_len = input.read_var_i32()?;
        if palette_len < 0 {
            return Err(BufferError::NegativeLength);
        }
        if palette_len as usize > (1usize << wire_bits.max(4)) {
            return Err(BufferError::CollectionTooLong {
                actual: palette_len as usize,
                max: 1usize << wire_bits.max(4),
            });
        }
        (0..palette_len)
            .map(|_| input.read_var_i32().and_then(read_registry_state))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        BuiltInRegistries::all_block_states().to_vec()
    };
    let storage_bits = match wire_bits {
        0 => 0,
        1..=4 => 4,
        bits if bits > MAX_BLOCK_INDIRECT_BITS => {
            let state_count = BuiltInRegistries::all_block_states().len();
            usize::BITS as usize - state_count.saturating_sub(1).leading_zeros() as usize
        }
        bits => bits,
    };
    let storage = read_storage(input, SECTION_VOLUME, storage_bits)?;
    let expected_words = storage_word_count(SECTION_VOLUME, storage_bits);
    if storage.len() != expected_words {
        log::warn!(
            "[ChunkDecode] block-state storage truncated: expected {expected_words} longs, got {}",
            storage.len()
        );
        return Err(BufferError::NotEnoughBytes {
            expected: expected_words,
            remaining: storage.len(),
        });
    }
    Ok(PalettedContainer::from_raw(
        palette,
        storage,
        storage_bits,
        SECTION_VOLUME,
        default_state.clone(),
    ))
}
fn skip_biomes(input: &mut PacketBuffer) -> Result<(), BufferError> {
    let bits = usize::from(input.read_u8()?);
    if bits == 0 {
        let _single_registry_id = input.read_var_i32()?;
    } else if bits <= MAX_BIOME_INDIRECT_BITS {
        let palette_len = input.read_var_i32()?;
        if palette_len < 0 {
            return Err(BufferError::NegativeLength);
        }
        if palette_len as usize > 1usize << bits {
            return Err(BufferError::CollectionTooLong {
                actual: palette_len as usize,
                max: 1usize << bits,
            });
        }
        for _ in 0..palette_len {
            let _registry_id = input.read_var_i32()?;
        }
    }
    let _storage = read_storage(input, BIOME_SECTION_VOLUME, bits)?;
    Ok(())
}
pub fn decode_chunk_sections(
    x: i32,
    z: i32,
    min_y_section: i32,
    max_y_section: i32,
    section_data: &[u8],
) -> Result<Chunk, BufferError> {
    let default_block = match BuiltInRegistries::block_states()
        .get_state_by_id(0)
        .cloned()
    {
        Some(state) => state,
        None => {
            log::error!("[ChunkDecode] air block state 0 is missing from the block-state registry");
            return Err(BufferError::InvalidEnumId {
                name: "block state",
                id: 0,
            });
        }
    };
    let mut chunk = Chunk::new(x, z, min_y_section, max_y_section, default_block.clone());
    let mut input = PacketBuffer::from_bytes(section_data.to_vec());
    for section in &mut chunk.sections {
        let non_empty_block_count = input.read_i16()?;
        let fluid_count = input.read_i16()?;
        section.blocks = read_block_states(&mut input, &default_block)?;
        skip_biomes(&mut input)?;
        section.non_empty_block_count = non_empty_block_count.max(0) as u16;
        section.fluid_count = fluid_count.max(0) as u16;
        let mut ticking_block_count = 0u16;
        section.blocks.count(|state, count| {
            if !state.is_known_vanilla_air() && state.is_random_ticking() {
                ticking_block_count =
                    ticking_block_count.saturating_add(count.min(u16::MAX as usize) as u16);
            }
        });
        section.ticking_block_count = ticking_block_count;
        section.ticking_fluid_count = 0;
    }
    if input.remaining() != 0 {
        log::warn!(
            "[ChunkDecode] chunk [{x}, {z}] retained {} trailing section bytes",
            input.remaining()
        );
    }
    Ok(chunk)
}
fn mask_indices(mask: &[i64]) -> impl Iterator<Item = usize> + '_ {
    mask.iter().enumerate().flat_map(|(word_index, word)| {
        let mut word = *word as u64;
        std::iter::from_fn(move || {
            if word == 0 {
                return None;
            }
            let bit = word.trailing_zeros() as usize;
            word &= word - 1;
            Some(word_index * 64 + bit)
        })
    })
}
fn mask_bit_count(mask: &[i64]) -> usize {
    mask.iter()
        .map(|word| (*word as u64).count_ones() as usize)
        .sum()
}
pub fn apply_light_data(chunk: &mut Chunk, data: &LightUpdatePacketData) {
    let mut complete = true;
    for index in mask_indices(&data.empty_sky_y_mask) {
        chunk.set_light_layer(true, index, Some(DataLayer::zero()));
    }
    let sky_count = mask_bit_count(&data.sky_y_mask);
    if sky_count != data.sky_updates.len() {
        log::warn!(
            "[ChunkDecode] sky light mask has {sky_count} sections but packet carries {} updates",
            data.sky_updates.len()
        );
        complete = false;
    } else {
        for (index, bytes) in mask_indices(&data.sky_y_mask).zip(&data.sky_updates) {
            chunk.set_light_layer(true, index, DataLayer::from_bytes(bytes));
        }
    }
    for index in mask_indices(&data.empty_block_y_mask) {
        chunk.set_light_layer(false, index, Some(DataLayer::zero()));
    }
    let block_count = mask_bit_count(&data.block_y_mask);
    if block_count != data.block_updates.len() {
        log::warn!(
            "[ChunkDecode] block light mask has {block_count} sections but packet carries {} updates",
            data.block_updates.len()
        );
        complete = false;
    } else {
        for (index, bytes) in mask_indices(&data.block_y_mask).zip(&data.block_updates) {
            chunk.set_light_layer(false, index, DataLayer::from_bytes(bytes));
        }
    }
    chunk.is_light_correct = complete;
}
pub fn install_block_entities(
    chunk: &mut Chunk,
    chunk_x: i32,
    chunk_z: i32,
    entries: &[BlockEntityInfo],
) {
    for info in entries {
        let packed_xz = u8::from_ne_bytes(info.packed_xz.to_ne_bytes());
        let pos = BlockPos::new(
            (chunk_x << 4) + i32::from(packed_xz >> 4),
            i32::from(info.y),
            (chunk_z << 4) + i32::from(packed_xz & 15),
        );
        let kind = chunk
            .get_block_state(&pos)
            .map(|state| state.block.to_string())
            .unwrap_or_else(|| format!("block_entity_registry_id:{}", info.block_entity_type_id));
        let mut nbt = Vec::new();
        if let Some(tag) = &info.tag {
            nbt.push(tag.get_id());
            if let Err(error) = el_nbt::io::write_payload(&mut nbt, tag) {
                log::warn!("[ChunkDecode] invalid block entity NBT at {pos:?}: {error}");
                nbt.clear();
            }
        }
        chunk
            .block_entities
            .insert(pos, BlockEntityData { kind, nbt });
    }
}
