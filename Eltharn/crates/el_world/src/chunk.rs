use crate::paletted_container::PalettedContainer;
use el_core::block_state::BlockState;
use el_core::resource_location::ResourceLocation;
use el_math::block_pos::BlockPos;
use std::collections::HashMap;
pub const CHUNK_WIDTH: i32 = 16;
pub const SECTION_HEIGHT: i32 = 16;
pub const SECTION_VOLUME: usize = 16 * 16 * 16;
pub const BIOME_SECTION_VOLUME: usize = 4 * 4 * 4;
pub const LIGHT_LAYER_BYTES: usize = SECTION_VOLUME / 2;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HeightmapType {
    WorldSurfaceWg,
    WorldSurface,
    OceanFloorWg,
    OceanFloor,
    MotionBlocking,
    MotionBlockingNoLeaves,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataLayer {
    data: Box<[u8; LIGHT_LAYER_BYTES]>,
}
impl DataLayer {
    pub fn zero() -> Self {
        Self {
            data: Box::new([0; LIGHT_LAYER_BYTES]),
        }
    }
    pub fn filled(value: u8) -> Self {
        let nibble = value.min(15);
        Self {
            data: Box::new([nibble | (nibble << 4); LIGHT_LAYER_BYTES]),
        }
    }
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let data: [u8; LIGHT_LAYER_BYTES] = bytes.try_into().ok()?;
        Some(Self {
            data: Box::new(data),
        })
    }
    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        let index = ((y & 15) << 8 | (z & 15) << 4 | (x & 15)) as usize;
        let byte = self.data[index >> 1];
        if index & 1 == 0 { byte & 15 } else { byte >> 4 }
    }
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, z: i32, value: u8) {
        let index = ((y & 15) << 8 | (z & 15) << 4 | (x & 15)) as usize;
        let value = value.min(15);
        let byte = &mut self.data[index >> 1];
        if index & 1 == 0 {
            *byte = (*byte & 0xf0) | value;
        } else {
            *byte = (*byte & 0x0f) | (value << 4);
        }
    }
    pub fn as_bytes(&self) -> &[u8] {
        self.data.as_slice()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockEntityData {
    pub kind: String,
    pub nbt: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct ChunkSection {
    pub y_index: i32,
    pub non_empty_block_count: u16,
    pub fluid_count: u16,
    pub ticking_block_count: u16,
    pub ticking_fluid_count: u16,
    pub blocks: PalettedContainer<BlockState>,
    pub biomes: PalettedContainer<ResourceLocation>,
}
impl ChunkSection {
    pub fn new(y_index: i32, default_block: BlockState) -> Self {
        Self {
            y_index,
            non_empty_block_count: 0,
            fluid_count: 0,
            ticking_block_count: 0,
            ticking_fluid_count: 0,
            blocks: PalettedContainer::new(SECTION_VOLUME, default_block),
            biomes: PalettedContainer::new_with_minimum_bits(
                BIOME_SECTION_VOLUME,
                ResourceLocation::new("minecraft", "plains"),
                1,
            ),
        }
    }
    #[inline]
    fn block_index(x: i32, y: i32, z: i32) -> usize {
        ((y & 15) << 8 | (z & 15) << 4 | (x & 15)) as usize
    }
    #[inline]
    fn biome_index(x: i32, y: i32, z: i32) -> usize {
        ((y & 3) << 4 | (z & 3) << 2 | (x & 3)) as usize
    }
    pub fn get_block_state(&self, x: i32, y: i32, z: i32) -> BlockState {
        self.blocks.get(Self::block_index(x, y, z))
    }
    #[inline]
    pub fn get_block_state_ref(&self, x: i32, y: i32, z: i32) -> &BlockState {
        self.blocks.get_ref(Self::block_index(x, y, z))
    }
    pub fn set_block_state(&mut self, x: i32, y: i32, z: i32, state: BlockState) -> BlockState {
        let old = self
            .blocks
            .get_and_set(Self::block_index(x, y, z), state.clone());
        self.update_counts_for_change(&old, &state);
        old
    }
    pub fn get_biome(&self, quart_x: i32, quart_y: i32, quart_z: i32) -> ResourceLocation {
        self.biomes
            .get(Self::biome_index(quart_x, quart_y, quart_z))
    }
    pub fn set_biome(&mut self, quart_x: i32, quart_y: i32, quart_z: i32, biome: ResourceLocation) {
        self.biomes
            .set(Self::biome_index(quart_x, quart_y, quart_z), biome);
    }
    pub fn has_only_air(&self) -> bool {
        self.non_empty_block_count == 0
    }
    pub fn has_fluid(&self) -> bool {
        self.fluid_count != 0
    }
    pub fn recalc_block_counts(&mut self) {
        let mut non_empty = 0usize;
        let mut fluids = 0usize;
        let mut ticking_blocks = 0usize;
        self.blocks.count(|state, count| {
            if !is_air(state) {
                non_empty += count;
                if state.is_random_ticking() {
                    ticking_blocks += count;
                }
            }
            if !state.get_fluid_state().is_empty() {
                fluids += count;
            }
        });
        self.non_empty_block_count = non_empty.min(u16::MAX as usize) as u16;
        self.fluid_count = fluids.min(u16::MAX as usize) as u16;
        self.ticking_block_count = ticking_blocks.min(u16::MAX as usize) as u16;
        self.ticking_fluid_count = 0;
    }
    fn update_counts_for_change(&mut self, old: &BlockState, new: &BlockState) {
        let old_air = is_air(old);
        let new_air = is_air(new);
        match (old_air, new_air) {
            (true, false) => {
                self.non_empty_block_count = self.non_empty_block_count.saturating_add(1);
            }
            (false, true) => {
                self.non_empty_block_count = self.non_empty_block_count.saturating_sub(1);
            }
            _ => {}
        }
        let old_fluid = !old.get_fluid_state().is_empty();
        let new_fluid = !new.get_fluid_state().is_empty();
        match (old_fluid, new_fluid) {
            (false, true) => self.fluid_count = self.fluid_count.saturating_add(1),
            (true, false) => self.fluid_count = self.fluid_count.saturating_sub(1),
            _ => {}
        }
        let old_ticking = !old_air && old.is_random_ticking();
        let new_ticking = !new_air && new.is_random_ticking();
        match (old_ticking, new_ticking) {
            (false, true) => {
                self.ticking_block_count = self.ticking_block_count.saturating_add(1);
            }
            (true, false) => {
                self.ticking_block_count = self.ticking_block_count.saturating_sub(1);
            }
            _ => {}
        }
    }
}
#[derive(Clone, Debug)]
pub struct Chunk {
    pub x: i32,
    pub z: i32,
    pub sections: Vec<ChunkSection>,
    pub min_y_section: i32,
    pub heightmaps: HashMap<HeightmapType, Vec<u64>>,
    pub block_entities: HashMap<BlockPos, BlockEntityData>,
    pub sky_light: Vec<Option<DataLayer>>,
    pub block_light: Vec<Option<DataLayer>>,
    pub is_light_correct: bool,
}
impl Chunk {
    pub fn new(
        x: i32,
        z: i32,
        min_y_section: i32,
        max_y_section: i32,
        default_block: BlockState,
    ) -> Self {
        let sections = (min_y_section..=max_y_section)
            .map(|y| ChunkSection::new(y, default_block.clone()))
            .collect::<Vec<_>>();
        let light_section_count = sections.len() + 2;
        Self {
            x,
            z,
            sections,
            min_y_section,
            heightmaps: HashMap::new(),
            block_entities: HashMap::new(),
            sky_light: vec![None; light_section_count],
            block_light: vec![None; light_section_count],
            is_light_correct: false,
        }
    }
    pub fn get_block_state(&self, pos: &BlockPos) -> Option<BlockState> {
        let index = (pos.y >> 4) - self.min_y_section;
        self.sections
            .get(index as usize)
            .map(|section| section.get_block_state(pos.x, pos.y, pos.z))
    }
    pub fn set_block_state(&mut self, pos: &BlockPos, state: BlockState) -> bool {
        let index = (pos.y >> 4) - self.min_y_section;
        let Some(section) = self.sections.get_mut(index as usize) else {
            return false;
        };
        section.set_block_state(pos.x, pos.y, pos.z, state);
        true
    }
    pub fn get_biome(&self, pos: &BlockPos) -> Option<ResourceLocation> {
        let index = (pos.y >> 4) - self.min_y_section;
        self.sections
            .get(index as usize)
            .map(|section| section.get_biome(pos.x >> 2, pos.y >> 2, pos.z >> 2))
    }
    pub fn set_light_layer(
        &mut self,
        sky: bool,
        light_section_index: usize,
        layer: Option<DataLayer>,
    ) {
        let layers = if sky {
            &mut self.sky_light
        } else {
            &mut self.block_light
        };
        if let Some(slot) = layers.get_mut(light_section_index) {
            *slot = layer;
        }
    }
    pub fn update_light_for_air(&mut self, pos: &BlockPos) {
        let light_section_index = ((pos.y >> 4) - self.min_y_section + 1) as usize;
        let neighbors = [
            BlockPos::new(pos.x + 1, pos.y, pos.z),
            BlockPos::new(pos.x - 1, pos.y, pos.z),
            BlockPos::new(pos.x, pos.y + 1, pos.z),
            BlockPos::new(pos.x, pos.y - 1, pos.z),
            BlockPos::new(pos.x, pos.y, pos.z + 1),
            BlockPos::new(pos.x, pos.y, pos.z - 1),
        ];
        let max_sky = neighbors
            .iter()
            .map(|p| self.brightness(true, p))
            .max()
            .unwrap_or(15);
        let max_block = neighbors
            .iter()
            .map(|p| self.brightness(false, p))
            .max()
            .unwrap_or(0);
        if let Some(Some(layer)) = self.sky_light.get_mut(light_section_index) {
            layer.set(pos.x, pos.y, pos.z, max_sky);
        }
        if let Some(Some(layer)) = self.block_light.get_mut(light_section_index) {
            layer.set(pos.x, pos.y, pos.z, max_block);
        }
    }
    pub fn brightness(&self, sky: bool, pos: &BlockPos) -> u8 {
        let light_index = ((pos.y >> 4) - self.min_y_section + 1) as usize;
        let layers = if sky {
            &self.sky_light
        } else {
            &self.block_light
        };
        layers
            .get(light_index)
            .and_then(Option::as_ref)
            .map(|layer| layer.get(pos.x, pos.y, pos.z))
            .unwrap_or(if sky { 15 } else { 0 })
    }
    pub fn height(&self, kind: HeightmapType, local_x: i32, local_z: i32) -> Option<i32> {
        let data = self.heightmaps.get(&kind)?;
        let world_height = self.sections.len() * 16;
        let bits = usize::BITS as usize - world_height.leading_zeros() as usize;
        let values_per_long = 64 / bits;
        let index = ((local_z & 15) * 16 + (local_x & 15)) as usize;
        let word = *data.get(index / values_per_long)?;
        let offset = (index % values_per_long) * bits;
        let value = ((word >> offset) & ((1u64 << bits) - 1)) as i32;
        Some(value + (self.min_y_section << 4))
    }
    pub fn non_air_block_count(&self) -> usize {
        self.sections
            .iter()
            .map(|section| section.non_empty_block_count as usize)
            .sum()
    }
}
fn is_air(state: &BlockState) -> bool {
    matches!(state.block.path.as_str(), "air" | "cave_air" | "void_air")
        || state.block.path.is_empty()
}
