use crate::block_state::BlockState;
use crate::collision_context::CollisionContext;
use crate::resource_location::ResourceLocation;
use el_math::VoxelShape;
use el_math::block_pos::BlockPos;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Property {
    Boolean { name: String },
    Integer { name: String, min: i32, max: i32 },
    Enum { name: String, values: Vec<String> },
}

impl Property {
    pub fn name(&self) -> &str {
        match self {
            Property::Boolean { name } => name,
            Property::Integer { name, .. } => name,
            Property::Enum { name, .. } => name,
        }
    }

    pub fn values(&self) -> Vec<String> {
        match self {
            Property::Boolean { .. } => vec!["true".to_string(), "false".to_string()],
            Property::Integer { min, max, .. } => (*min..=*max).map(|v| v.to_string()).collect(),
            Property::Enum { values, .. } => values.clone(),
        }
    }

    pub fn validate(&self, value: &str) -> bool {
        match self {
            Property::Boolean { .. } => value == "true" || value == "false",
            Property::Integer { min, max, .. } => {
                if let Ok(v) = value.parse::<i32>() {
                    v >= *min && v <= *max
                } else {
                    false
                }
            }
            Property::Enum { values, .. } => values.iter().any(|v| v == value),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub location: ResourceLocation,
    pub default_state_id: usize,
    pub properties: Vec<Property>,
}

impl Block {
    pub fn default_state(&self) -> &'static BlockState {
        crate::registry::BuiltInRegistries::get_state_by_id(self.default_state_id)
            .expect("default block state must be registered")
    }

    pub fn get_collision_shape(&self, state: &BlockState) -> VoxelShape {
        crate::block_behaviour::get_collision_shape(self, state)
    }

    pub fn get_collision_shape_with_context(
        &self,
        state: &BlockState,
        pos: BlockPos,
        context: &CollisionContext,
    ) -> VoxelShape {
        crate::block_behaviour::get_collision_shape_with_context(self, state, pos, context)
    }

    pub fn get_visual_shape_with_context(
        &self,
        state: &BlockState,
        pos: BlockPos,
        context: &CollisionContext,
    ) -> VoxelShape {
        crate::block_behaviour::get_visual_shape_with_context(self, state, pos, context)
    }

    pub fn get_occlusion_shape(&self, state: &BlockState) -> VoxelShape {
        crate::block_behaviour::get_occlusion_shape(self, state)
    }

    pub fn has_large_collision_shape(&self, state: &BlockState) -> bool {
        crate::block_behaviour::has_large_collision_shape(self, state)
    }

    pub fn get_interaction_shape(&self, state: &BlockState) -> VoxelShape {
        crate::block_behaviour::get_interaction_shape(self, state)
    }

    pub fn get_outline_shape(&self, state: &BlockState) -> VoxelShape {
        crate::block_behaviour::get_outline_shape(self, state)
    }

    pub fn get_outline_shape_with_context(
        &self,
        state: &BlockState,
        holding_item: Option<&str>,
    ) -> VoxelShape {
        crate::block_behaviour::get_outline_shape_with_context(self, state, holding_item)
    }

    pub fn get_outline_shape_with_collision_context_at(
        &self,
        state: &BlockState,
        pos: BlockPos,
        context: &CollisionContext,
    ) -> VoxelShape {
        crate::block_behaviour::get_outline_shape_with_collision_context_at(
            self, state, pos, context,
        )
    }

    pub fn get_interaction_override_shape(&self, state: &BlockState) -> VoxelShape {
        crate::block_behaviour::get_interaction_override_shape(self, state)
    }

    pub fn is_random_ticking(&self, state: &BlockState) -> bool {
        crate::block_behaviour::is_random_ticking(self, state)
    }

    pub fn is_replaceable(&self, state: &BlockState) -> bool {
        crate::block_behaviour::is_replaceable(self, state)
    }

    pub fn destroy_speed(&self, state: &BlockState) -> f32 {
        crate::block_behaviour::destroy_speed(self, state)
    }

    pub fn get_friction(&self, state: &BlockState) -> f32 {
        crate::block_behaviour::friction(self, state)
    }

    pub fn get_signal(&self, state: &BlockState, direction: el_math::Direction) -> i32 {
        crate::block_behaviour::get_signal(self, state, direction)
    }

    pub fn get_direct_signal(&self, state: &BlockState, direction: el_math::Direction) -> i32 {
        crate::block_behaviour::get_direct_signal(self, state, direction)
    }

    pub fn is_signal_source(&self, state: &BlockState) -> bool {
        crate::block_behaviour::is_signal_source(self, state)
    }

    pub fn own_signal(&self, state: &BlockState) -> i32 {
        crate::block_behaviour::own_signal(self, state)
    }

    pub fn is_redstone_conductor(&self, state: &BlockState) -> bool {
        crate::block_behaviour::is_redstone_conductor(self, state)
    }
}
