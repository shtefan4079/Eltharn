use el_math::VoxelShape;
use el_math::block_pos::BlockPos;
#[derive(Clone, Debug, PartialEq)]
pub struct CollisionContext {
    descending: bool,
    placement: bool,
    entity_bottom: Option<f64>,
    held_item_path: Option<String>,
    feet_item_path: Option<String>,
    always_collide_with_fluid: bool,
    entity_kind: CollisionEntityKind,
    fall_distance: f32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CollisionEntityKind {
    #[default]
    None,
    Entity,
    Living,
    Player,
    FallingBlock,
    PowderSnowWalkableMob,
}
impl CollisionContext {
    pub fn empty() -> Self {
        Self {
            descending: false,
            placement: false,
            entity_bottom: None,
            held_item_path: None,
            feet_item_path: None,
            always_collide_with_fluid: false,
            entity_kind: CollisionEntityKind::None,
            fall_distance: 0.0,
        }
    }
    pub fn empty_with_fluid_collisions() -> Self {
        Self {
            always_collide_with_fluid: true,
            ..Self::empty()
        }
    }
    pub fn position_context(y: f64) -> Self {
        Self {
            entity_bottom: Some(y),
            ..Self::empty()
        }
    }
    pub fn player(
        descending: bool,
        entity_bottom: f64,
        held_item_path: Option<String>,
        feet_item_path: Option<String>,
        fall_distance: f32,
    ) -> Self {
        Self {
            descending,
            placement: false,
            entity_bottom: Some(entity_bottom),
            held_item_path,
            feet_item_path,
            always_collide_with_fluid: false,
            entity_kind: CollisionEntityKind::Player,
            fall_distance,
        }
    }
    pub fn entity(
        entity_kind: CollisionEntityKind,
        descending: bool,
        entity_bottom: f64,
        held_item_path: Option<String>,
        feet_item_path: Option<String>,
        always_collide_with_fluid: bool,
        fall_distance: f32,
    ) -> Self {
        Self {
            descending,
            placement: false,
            entity_bottom: Some(entity_bottom),
            held_item_path,
            feet_item_path,
            always_collide_with_fluid,
            entity_kind,
            fall_distance,
        }
    }
    pub fn placement_player(
        descending: bool,
        entity_bottom: f64,
        held_item_path: Option<String>,
        feet_item_path: Option<String>,
    ) -> Self {
        Self {
            descending,
            placement: true,
            entity_bottom: Some(entity_bottom),
            held_item_path,
            feet_item_path,
            always_collide_with_fluid: false,
            entity_kind: CollisionEntityKind::Player,
            fall_distance: 0.0,
        }
    }
    pub fn is_descending(&self) -> bool {
        self.descending
    }
    pub fn is_placement(&self) -> bool {
        self.placement
    }
    pub fn always_collide_with_fluid(&self) -> bool {
        self.always_collide_with_fluid
    }
    pub fn entity_kind(&self) -> CollisionEntityKind {
        self.entity_kind
    }
    pub fn entity_bottom(&self) -> Option<f64> {
        self.entity_bottom
    }
    pub fn fall_distance(&self) -> f32 {
        self.fall_distance
    }
    pub fn is_above(&self, shape: &VoxelShape, pos: BlockPos, default_value: bool) -> bool {
        match self.entity_bottom {
            Some(entity_bottom) => entity_bottom > pos.y as f64 + shape.bounds().max_y - 1.0e-5,
            None => default_value,
        }
    }
    pub fn is_holding_item_path(&self, item_path: &str) -> bool {
        self.held_item_path
            .as_deref()
            .map(|held| held == item_path || held == format!("minecraft:{item_path}"))
            .unwrap_or(false)
    }
    pub fn with_held_item_path(mut self, held_item_path: Option<String>) -> Self {
        self.held_item_path = held_item_path;
        self
    }
    pub fn feet_item_path(&self) -> Option<&str> {
        self.feet_item_path.as_deref()
    }
    pub fn can_entity_walk_on_powder_snow(&self) -> bool {
        if self.entity_kind == CollisionEntityKind::PowderSnowWalkableMob {
            return true;
        }
        self.feet_item_path
            .as_deref()
            .map(|item| item == "leather_boots" || item == "minecraft:leather_boots")
            .unwrap_or(false)
    }
    pub fn is_falling_block(&self) -> bool {
        self.entity_kind == CollisionEntityKind::FallingBlock
    }
    pub fn has_entity(&self) -> bool {
        self.entity_kind != CollisionEntityKind::None
    }
}
