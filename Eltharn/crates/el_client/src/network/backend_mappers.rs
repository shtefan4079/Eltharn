#![allow(dead_code)]
use el_math::BlockPos;
use el_nbt::Tag;
pub fn map_backend_nbt_to_nbt(tag: el_protocol_backend::BackendNbtTag) -> Tag {
    match tag {
        el_protocol_backend::BackendNbtTag::End => Tag::End,
        el_protocol_backend::BackendNbtTag::Byte(v) => Tag::Byte(v),
        el_protocol_backend::BackendNbtTag::Short(v) => Tag::Short(v),
        el_protocol_backend::BackendNbtTag::Int(v) => Tag::Int(v),
        el_protocol_backend::BackendNbtTag::Long(v) => Tag::Long(v),
        el_protocol_backend::BackendNbtTag::Float(v) => Tag::Float(v),
        el_protocol_backend::BackendNbtTag::Double(v) => Tag::Double(v),
        el_protocol_backend::BackendNbtTag::ByteArray(v) => Tag::ByteArray(v),
        el_protocol_backend::BackendNbtTag::String(v) => Tag::String(v),
        el_protocol_backend::BackendNbtTag::List(v) => {
            Tag::List(v.into_iter().map(map_backend_nbt_to_nbt).collect())
        }
        el_protocol_backend::BackendNbtTag::Compound(v) => Tag::Compound(
            v.into_iter()
                .map(|(k, t)| (k, map_backend_nbt_to_nbt(t)))
                .collect(),
        ),
        el_protocol_backend::BackendNbtTag::IntArray(v) => Tag::IntArray(v),
        el_protocol_backend::BackendNbtTag::LongArray(v) => Tag::LongArray(v),
    }
}
pub fn map_backend_item_stack(
    b_stack: el_protocol_backend::BackendItemStack,
) -> el_core::ItemStack {
    let tag = map_backend_nbt_to_nbt(b_stack.tag);
    el_core::ItemStack::from_nbt(&tag).unwrap_or_else(el_core::ItemStack::empty)
}
pub fn map_nbt_to_backend_nbt(tag: Tag) -> el_protocol_backend::BackendNbtTag {
    match tag {
        Tag::End => el_protocol_backend::BackendNbtTag::End,
        Tag::Byte(v) => el_protocol_backend::BackendNbtTag::Byte(v),
        Tag::Short(v) => el_protocol_backend::BackendNbtTag::Short(v),
        Tag::Int(v) => el_protocol_backend::BackendNbtTag::Int(v),
        Tag::Long(v) => el_protocol_backend::BackendNbtTag::Long(v),
        Tag::Float(v) => el_protocol_backend::BackendNbtTag::Float(v),
        Tag::Double(v) => el_protocol_backend::BackendNbtTag::Double(v),
        Tag::ByteArray(v) => el_protocol_backend::BackendNbtTag::ByteArray(v),
        Tag::String(v) => el_protocol_backend::BackendNbtTag::String(v),
        Tag::List(v) => el_protocol_backend::BackendNbtTag::List(
            v.into_iter().map(map_nbt_to_backend_nbt).collect(),
        ),
        Tag::Compound(v) => el_protocol_backend::BackendNbtTag::Compound(
            v.into_iter()
                .map(|(k, t)| (k, map_nbt_to_backend_nbt(t)))
                .collect(),
        ),
        Tag::IntArray(v) => el_protocol_backend::BackendNbtTag::IntArray(v),
        Tag::LongArray(v) => el_protocol_backend::BackendNbtTag::LongArray(v),
    }
}
pub fn map_item_stack_to_backend(
    stack: &el_core::ItemStack,
) -> el_protocol_backend::BackendItemStack {
    el_protocol_backend::BackendItemStack {
        tag: map_nbt_to_backend_nbt(stack.to_nbt()),
    }
}
pub fn map_backend_blocks_container(
    container: el_protocol_backend::BackendPalettedContainer<usize>,
) -> el_world::PalettedContainer<el_core::BlockState> {
    let palette = container
        .palette
        .into_iter()
        .map(|id| {
            el_core::registry::BuiltInRegistries::get_state_by_id(id)
                .cloned()
                .unwrap_or_else(|| {
                    let air = el_core::resource_location::ResourceLocation::new("minecraft", "air");
                    el_core::BlockState::new(std::sync::Arc::new(air))
                })
        })
        .collect::<Vec<_>>();
    let default_val =
        el_core::registry::BuiltInRegistries::get_state_by_id(container.default_value)
            .cloned()
            .unwrap_or_else(|| {
                let air = el_core::resource_location::ResourceLocation::new("minecraft", "air");
                el_core::BlockState::new(std::sync::Arc::new(air))
            });
    el_world::PalettedContainer::from_raw_with_minimum_bits(
        palette,
        container.storage,
        container.bits_per_entry,
        container.capacity,
        default_val,
        4,
    )
}
pub fn map_backend_biomes_container(
    container: el_protocol_backend::BackendPalettedContainer<String>,
) -> el_world::PalettedContainer<el_core::resource_location::ResourceLocation> {
    let palette = container
        .palette
        .into_iter()
        .map(|s| el_core::resource_location::ResourceLocation::from_string(&s))
        .collect::<Vec<_>>();
    let default_val =
        el_core::resource_location::ResourceLocation::from_string(&container.default_value);
    el_world::PalettedContainer::from_raw_with_minimum_bits(
        palette,
        container.storage,
        container.bits_per_entry,
        container.capacity,
        default_val,
        1,
    )
}
pub fn map_heightmap_type_from_string(s: &str) -> Option<el_world::chunk::HeightmapType> {
    match s {
        "world_surface_wg" => Some(el_world::chunk::HeightmapType::WorldSurfaceWg),
        "world_surface" => Some(el_world::chunk::HeightmapType::WorldSurface),
        "ocean_floor_wg" => Some(el_world::chunk::HeightmapType::OceanFloorWg),
        "ocean_floor" => Some(el_world::chunk::HeightmapType::OceanFloor),
        "motion_blocking" => Some(el_world::chunk::HeightmapType::MotionBlocking),
        "motion_blocking_no_leaves" => Some(el_world::chunk::HeightmapType::MotionBlockingNoLeaves),
        _ => None,
    }
}
pub fn map_backend_chunk(b_chunk: el_protocol_backend::BackendChunk) -> el_world::chunk::Chunk {
    let sections = b_chunk
        .sections
        .into_iter()
        .map(|sec| el_world::chunk::ChunkSection {
            y_index: sec.y_index,
            non_empty_block_count: sec.non_empty_block_count,
            fluid_count: sec.fluid_count,
            ticking_block_count: sec.ticking_block_count,
            ticking_fluid_count: sec.ticking_fluid_count,
            blocks: map_backend_blocks_container(sec.blocks),
            biomes: map_backend_biomes_container(sec.biomes),
        })
        .collect::<Vec<_>>();
    let mut heightmaps = std::collections::HashMap::new();
    for (k, v) in b_chunk.heightmaps {
        if let Some(h) = map_heightmap_type_from_string(&k) {
            heightmaps.insert(h, v);
        }
    }
    let mut block_entities = std::collections::HashMap::new();
    for entry in b_chunk.block_entities {
        let pos = entry.pos;
        let data = entry.data;
        block_entities.insert(
            BlockPos::new(pos.x, pos.y, pos.z),
            el_world::chunk::BlockEntityData {
                kind: data.kind,
                nbt: data.nbt,
            },
        );
    }
    let sky_light = b_chunk
        .sky_light
        .into_iter()
        .map(|opt| opt.and_then(|bytes| el_world::chunk::DataLayer::from_bytes(&bytes)))
        .collect::<Vec<_>>();
    let block_light = b_chunk
        .block_light
        .into_iter()
        .map(|opt| opt.and_then(|bytes| el_world::chunk::DataLayer::from_bytes(&bytes)))
        .collect::<Vec<_>>();
    el_world::chunk::Chunk {
        x: b_chunk.x,
        z: b_chunk.z,
        sections,
        min_y_section: b_chunk.min_y_section,
        heightmaps,
        block_entities,
        sky_light,
        block_light,
        is_light_correct: b_chunk.is_light_correct,
    }
}
