use crate::CollisionContext;
use crate::resource_location::ResourceLocation;
use el_math::VoxelShape;
use el_math::block_pos::BlockPos;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockState {
    pub id: usize,
    pub block: Arc<ResourceLocation>,
    pub properties: HashMap<String, String>,
}

impl BlockState {
    pub fn new(block: Arc<ResourceLocation>) -> Self {
        BlockState {
            id: 0,
            block,
            properties: HashMap::new(),
        }
    }

    pub fn with_id(mut self, id: usize) -> Self {
        self.id = id;
        self
    }

    pub fn with_property(mut self, key: &str, value: &str) -> Self {
        self.properties.insert(key.to_string(), value.to_string());
        self
    }

    pub fn get_property(&self, key: &str) -> Option<&String> {
        self.properties.get(key)
    }

    pub fn is_known_vanilla_air(&self) -> bool {
        self.block.namespace == "minecraft"
            && matches!(self.block.path.as_str(), "air" | "cave_air" | "void_air")
    }

    pub fn get_block(&self) -> &crate::block::Block {
        &**crate::registry::BuiltInRegistries::blocks().get_by_location(&self.block)
    }

    pub fn get_collision_shape(&self) -> VoxelShape {
        self.get_block().get_collision_shape(self)
    }

    pub fn get_collision_shape_with_context(
        &self,
        pos: BlockPos,
        context: &CollisionContext,
    ) -> VoxelShape {
        self.get_block()
            .get_collision_shape_with_context(self, pos, context)
    }

    pub fn get_visual_shape_with_context(
        &self,
        pos: BlockPos,
        context: &CollisionContext,
    ) -> VoxelShape {
        self.get_block()
            .get_visual_shape_with_context(self, pos, context)
    }

    pub fn get_occlusion_shape(&self) -> VoxelShape {
        self.get_block().get_occlusion_shape(self)
    }

    pub fn has_large_collision_shape(&self) -> bool {
        self.get_block().has_large_collision_shape(self)
    }

    pub fn get_interaction_shape(&self) -> VoxelShape {
        self.get_block().get_interaction_shape(self)
    }

    pub fn get_outline_shape(&self) -> VoxelShape {
        self.get_block().get_outline_shape(self)
    }

    pub fn get_outline_shape_with_context(&self, holding_item: Option<&str>) -> VoxelShape {
        self.get_block()
            .get_outline_shape_with_context(self, holding_item)
    }

    pub fn get_outline_shape_with_collision_context_at(
        &self,
        pos: BlockPos,
        context: &CollisionContext,
    ) -> VoxelShape {
        self.get_block()
            .get_outline_shape_with_collision_context_at(self, pos, context)
    }

    pub fn get_interaction_override_shape(&self) -> VoxelShape {
        self.get_block().get_interaction_override_shape(self)
    }

    pub fn is_random_ticking(&self) -> bool {
        self.get_block().is_random_ticking(self)
    }

    pub fn ignited_by_lava(&self) -> bool {
        crate::block_behaviour::java_26_2_vanilla_block_ignited_by_lava(&self.block.to_string())
            .unwrap_or(false)
    }

    pub fn blocks_motion(&self) -> bool {
        crate::block_behaviour::java_26_2_vanilla_block_fluid_support_properties(self)
            .map(|properties| properties.blocks_motion)
            .unwrap_or(false)
    }

    pub fn is_replaceable(&self) -> bool {
        self.get_block().is_replaceable(self)
    }

    pub fn destroy_speed(&self) -> f32 {
        self.get_block().destroy_speed(self)
    }

    pub fn get_friction(&self) -> f32 {
        self.get_block().get_friction(self)
    }

    pub fn get_signal(&self, direction: el_math::Direction) -> i32 {
        self.get_block().get_signal(self, direction)
    }

    pub fn get_direct_signal(&self, direction: el_math::Direction) -> i32 {
        self.get_block().get_direct_signal(self, direction)
    }

    pub fn is_signal_source(&self) -> bool {
        self.get_block().is_signal_source(self)
    }

    pub fn own_signal(&self) -> i32 {
        self.get_block().own_signal(self)
    }

    pub fn is_redstone_conductor(&self) -> bool {
        self.get_block().is_redstone_conductor(self)
    }

    pub fn get_fluid_state(&self) -> crate::fluid_state::FluidState {
        crate::fluid_state::FluidState::from_block_state(self)
    }
}
