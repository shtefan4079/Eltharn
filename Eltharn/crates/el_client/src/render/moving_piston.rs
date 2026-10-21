use el_core::{
    block_state::BlockState, registry::BuiltInRegistries, resource_location::ResourceLocation,
};
use el_math::{Direction, block_pos::BlockPos};
use el_nbt::Tag;
use glam::Vec3;
use std::collections::HashMap;
#[derive(Clone, Debug, PartialEq)]
pub struct MovingPistonBlockEntity {
    pub moved_state: BlockState,
    pub direction: Direction,
    pub extending: bool,
    pub is_source_piston: bool,
    pub progress: f32,
    pub progress_o: f32,
    pub death_ticks: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovingPistonClientTick {
    Keep,
    Remove,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MovingPistonRenderBlock {
    pub block_pos: BlockPos,
    pub state: BlockState,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MovingPistonRenderPlan {
    pub block_entity_pos: BlockPos,
    pub translated_block: MovingPistonRenderBlock,
    pub translated_offset: Vec3,
    pub base: Option<MovingPistonRenderBlock>,
}
impl MovingPistonBlockEntity {
    pub fn new(
        moved_state: BlockState,
        direction: Direction,
        extending: bool,
        is_source_piston: bool,
    ) -> Self {
        Self {
            moved_state,
            direction,
            extending,
            is_source_piston,
            progress: 0.0,
            progress_o: 0.0,
            death_ticks: 0,
        }
    }
    pub fn from_block_entity_nbt(nbt: &[u8]) -> Option<Self> {
        let Ok(Tag::Compound(root)) = el_nbt::read_any_compound(nbt) else {
            return None;
        };
        let moved_state = el_nbt::compound_get(&root, "blockState")
            .and_then(decode_java_block_state_codec)
            .unwrap_or_else(canonical_air_state);
        let direction = el_nbt::compound_get(&root, "facing")
            .and_then(tag_byte)
            .map(|id| Direction::from_3d_data_value(i32::from(id)))
            .unwrap_or(Direction::Down);
        let progress = el_nbt::compound_get(&root, "progress")
            .and_then(tag_float)
            .unwrap_or(0.0);
        let extending = el_nbt::compound_get(&root, "extending")
            .and_then(tag_boolean)
            .unwrap_or(false);
        let is_source_piston = el_nbt::compound_get(&root, "source")
            .and_then(tag_boolean)
            .unwrap_or(false);
        Some(Self {
            moved_state,
            direction,
            extending,
            is_source_piston,
            progress,
            progress_o: progress,
            death_ticks: 0,
        })
    }
    pub fn progress_at(&self, mut partial_ticks: f32) -> f32 {
        if partial_ticks > 1.0 {
            partial_ticks = 1.0;
        }
        self.progress_o + (self.progress - self.progress_o) * partial_ticks
    }
    pub fn movement_direction(&self) -> Direction {
        if self.extending {
            self.direction
        } else {
            self.direction.opposite()
        }
    }
    pub fn extended_progress(&self, progress: f32) -> f32 {
        if self.extending {
            progress - 1.0
        } else {
            1.0 - progress
        }
    }
    pub fn offset_at(&self, partial_ticks: f32) -> Vec3 {
        let extended_progress = self.extended_progress(self.progress_at(partial_ticks));
        Vec3::new(
            self.direction.step_x() as f32 * extended_progress,
            self.direction.step_y() as f32 * extended_progress,
            self.direction.step_z() as f32 * extended_progress,
        )
    }
    pub fn client_tick(&mut self) -> MovingPistonClientTick {
        self.progress_o = self.progress;
        if self.progress_o >= 1.0 {
            if self.death_ticks < 5 {
                self.death_ticks += 1;
                MovingPistonClientTick::Keep
            } else {
                MovingPistonClientTick::Remove
            }
        } else {
            self.progress = (self.progress + 0.5).min(1.0);
            MovingPistonClientTick::Keep
        }
    }
    pub fn settled_state(&self) -> BlockState {
        if self.moved_state.properties.contains_key("waterlogged") {
            canonical_state_with_property(&self.moved_state, "waterlogged", "false")
                .expect("a registered waterlogged block must support false")
        } else {
            self.moved_state.clone()
        }
    }
    pub fn render_plan(
        &self,
        block_entity_pos: BlockPos,
        partial_ticks: f32,
    ) -> Option<MovingPistonRenderPlan> {
        if self.moved_state.is_known_vanilla_air() {
            return None;
        }
        let progress = self.progress_at(partial_ticks);
        let movement_direction = self.movement_direction();
        let render_state_pos = block_entity_pos.relative(movement_direction.opposite());
        let translated_offset = self.offset_at(partial_ticks);
        if is_vanilla_block(&self.moved_state, "piston_head") && progress <= 4.0 {
            let state = canonical_state_with_property(
                &self.moved_state,
                "short",
                if progress <= 0.5 { "true" } else { "false" },
            )?;
            return Some(MovingPistonRenderPlan {
                block_entity_pos,
                translated_block: MovingPistonRenderBlock {
                    block_pos: render_state_pos,
                    state,
                },
                translated_offset,
                base: None,
            });
        }
        if self.is_source_piston && !self.extending {
            let facing = self
                .moved_state
                .get_property("facing")
                .and_then(|value| Direction::from_name(value))?;
            let piston_type = if is_vanilla_block(&self.moved_state, "sticky_piston") {
                "sticky"
            } else {
                "normal"
            };
            let head = canonical_state(
                "minecraft:piston_head",
                &[
                    ("type", piston_type),
                    ("facing", facing.name()),
                    ("short", if progress >= 0.5 { "true" } else { "false" }),
                ],
            )?;
            let base = canonical_state_with_property(&self.moved_state, "extended", "true")?;
            return Some(MovingPistonRenderPlan {
                block_entity_pos,
                translated_block: MovingPistonRenderBlock {
                    block_pos: render_state_pos,
                    state: head,
                },
                translated_offset,
                base: Some(MovingPistonRenderBlock {
                    block_pos: render_state_pos.relative(movement_direction),
                    state: base,
                }),
            });
        }
        Some(MovingPistonRenderPlan {
            block_entity_pos,
            translated_block: MovingPistonRenderBlock {
                block_pos: render_state_pos,
                state: self.moved_state.clone(),
            },
            translated_offset,
            base: None,
        })
    }
}
fn decode_java_block_state_codec(tag: &Tag) -> Option<BlockState> {
    let Tag::Compound(fields) = tag else {
        return None;
    };
    let Tag::String(name) = el_nbt::compound_get(fields, "Name")? else {
        return None;
    };
    let block = ResourceLocation::try_from_string(name)?;
    let mut properties = canonical_default_state(&block)?.properties;
    if let Some(Tag::Compound(encoded_properties)) = el_nbt::compound_get(fields, "Properties") {
        for (key, value) in encoded_properties {
            if !properties.contains_key(key) {
                continue;
            }
            let Tag::String(value) = value else {
                return None;
            };
            properties.insert(key.clone(), value.clone());
        }
    }
    canonical_state_from_location(&block, properties)
}
fn canonical_air_state() -> BlockState {
    canonical_state("minecraft:air", &[])
        .expect("vanilla minecraft:air default block state must be registered")
}
fn canonical_default_state(block: &ResourceLocation) -> Option<BlockState> {
    BuiltInRegistries::all_block_states()
        .iter()
        .any(|state| state.block.as_ref() == block)
        .then(|| {
            BuiltInRegistries::blocks()
                .get_by_location(block)
                .default_state()
                .clone()
        })
}
fn canonical_state(block_name: &str, changes: &[(&str, &str)]) -> Option<BlockState> {
    let block = ResourceLocation::try_from_string(block_name)?;
    let mut properties = canonical_default_state(&block)?.properties;
    for (key, value) in changes {
        if !properties.contains_key(*key) {
            return None;
        }
        properties.insert((*key).to_string(), (*value).to_string());
    }
    canonical_state_from_location(&block, properties)
}
fn canonical_state_with_property(state: &BlockState, key: &str, value: &str) -> Option<BlockState> {
    let mut properties = state.properties.clone();
    if !properties.contains_key(key) {
        return None;
    }
    properties.insert(key.to_string(), value.to_string());
    canonical_state_from_location(state.block.as_ref(), properties)
}
fn canonical_state_from_location(
    block: &ResourceLocation,
    properties: HashMap<String, String>,
) -> Option<BlockState> {
    let id = BuiltInRegistries::get_id_by_state(block, &properties)?;
    BuiltInRegistries::get_state_by_id(id).cloned()
}
fn is_vanilla_block(state: &BlockState, path: &str) -> bool {
    state.block.namespace == "minecraft" && state.block.path == path
}
fn tag_byte(tag: &Tag) -> Option<i8> {
    match tag {
        Tag::Byte(value) => Some(*value),
        _ => None,
    }
}
fn tag_float(tag: &Tag) -> Option<f32> {
    match tag {
        Tag::Float(value) => Some(*value),
        _ => None,
    }
}
fn tag_boolean(tag: &Tag) -> Option<bool> {
    match tag {
        Tag::Byte(value) => Some(*value != 0),
        _ => None,
    }
}
