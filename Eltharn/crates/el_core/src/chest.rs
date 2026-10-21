use crate::block_state::BlockState;
use crate::registry::BuiltInRegistries;
use crate::resource_location::ResourceLocation;
use el_math::block_pos::BlockPos;
use el_math::{Axis, Direction};
use std::collections::HashMap;
pub const UPDATE_SHAPE_ORDER: [Direction; 6] = [
    Direction::West,
    Direction::East,
    Direction::North,
    Direction::South,
    Direction::Down,
    Direction::Up,
];
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChestFamily {
    Chest,
    TrappedChest,
    EnderChest,
    Copper,
}
impl ChestFamily {
    pub fn from_block(block: &ResourceLocation) -> Option<Self> {
        if block.namespace != "minecraft" {
            return None;
        }
        match block.path.as_str() {
            "chest" => Some(Self::Chest),
            "trapped_chest" => Some(Self::TrappedChest),
            "ender_chest" => Some(Self::EnderChest),
            "copper_chest"
            | "exposed_copper_chest"
            | "weathered_copper_chest"
            | "oxidized_copper_chest"
            | "waxed_copper_chest"
            | "waxed_exposed_copper_chest"
            | "waxed_weathered_copper_chest"
            | "waxed_oxidized_copper_chest" => Some(Self::Copper),
            _ => None,
        }
    }
    pub fn has_chest_type_property(self) -> bool {
        !matches!(self, Self::EnderChest)
    }
    pub fn can_combine(self) -> bool {
        self.has_chest_type_property()
    }
}
pub fn chest_family(state: &BlockState) -> Option<ChestFamily> {
    ChestFamily::from_block(&state.block)
}
pub fn is_chest_block(state: &BlockState) -> bool {
    chest_family(state).is_some()
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChestType {
    Single,
    Left,
    Right,
}
impl ChestType {
    pub fn opposite(self) -> Self {
        match self {
            Self::Single => Self::Single,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
    pub fn serialized_name(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "single" => Some(Self::Single),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DoubleBlockType {
    Single,
    First,
    Second,
}
pub fn chest_effective_properties(state: &BlockState) -> HashMap<String, String> {
    let mut properties = state.get_block().default_state().properties.clone();
    properties.extend(state.properties.clone());
    properties
}
pub fn chest_type(state: &BlockState) -> Option<ChestType> {
    chest_effective_properties(state)
        .get("type")
        .map(String::as_str)
        .and_then(ChestType::from_name)
}
pub fn chest_facing(state: &BlockState) -> Option<Direction> {
    chest_effective_properties(state)
        .get("facing")
        .and_then(|value| Direction::from_name(value))
        .filter(|direction| direction.axis() != Axis::Y)
}
pub fn chest_connected_direction(facing: Direction, chest_type: ChestType) -> Direction {
    if chest_type == ChestType::Left {
        facing.clockwise()
    } else {
        facing.counter_clockwise()
    }
}
pub fn chest_connected_direction_of(state: &BlockState) -> Option<Direction> {
    Some(chest_connected_direction(
        chest_facing(state)?,
        chest_type(state)?,
    ))
}
pub fn chest_connected_block_pos(pos: BlockPos, state: &BlockState) -> Option<BlockPos> {
    Some(pos.relative(chest_connected_direction_of(state)?))
}
pub fn chest_can_connect_to(current_block: &ResourceLocation, neighbour: &BlockState) -> bool {
    match ChestFamily::from_block(current_block) {
        Some(ChestFamily::Chest) | Some(ChestFamily::TrappedChest) => {
            *current_block == *neighbour.block && chest_type(neighbour).is_some()
        }
        Some(ChestFamily::Copper) => {
            chest_family(neighbour) == Some(ChestFamily::Copper) && chest_type(neighbour).is_some()
        }
        Some(ChestFamily::EnderChest) | None => false,
    }
}
pub fn chest_state_can_connect_to(current: &BlockState, neighbour: &BlockState) -> bool {
    chest_can_connect_to(&current.block, neighbour)
}
pub fn canonical_state_with_properties(
    block: &ResourceLocation,
    properties: &HashMap<String, String>,
) -> Option<BlockState> {
    let id = BuiltInRegistries::get_id_by_state(block, properties)?;
    BuiltInRegistries::get_state_by_id(id).cloned()
}
pub fn canonical_state_with_chest_type(
    state: &BlockState,
    chest_type: ChestType,
) -> Option<BlockState> {
    let mut properties = chest_effective_properties(state);
    properties.insert("type".to_string(), chest_type.serialized_name().to_string());
    canonical_state_with_properties(&state.block, &properties)
}
pub fn get_block_type(state: &BlockState) -> Option<DoubleBlockType> {
    Some(double_block_type(chest_type(state)?))
}
pub fn double_block_type(chest_type: ChestType) -> DoubleBlockType {
    match chest_type {
        ChestType::Single => DoubleBlockType::Single,
        ChestType::Right => DoubleBlockType::First,
        ChestType::Left => DoubleBlockType::Second,
    }
}
pub fn chest_update_shape(
    state: &BlockState,
    direction_to_neighbour: Direction,
    neighbour_state: &BlockState,
) -> BlockState {
    let Some(state_type) = chest_type(state) else {
        return state.clone();
    };
    let Some(facing) = chest_facing(state) else {
        return state.clone();
    };
    if chest_state_can_connect_to(state, neighbour_state)
        && direction_to_neighbour.axis() != Axis::Y
    {
        let Some(neighbour_type) = chest_type(neighbour_state) else {
            return state.clone();
        };
        let Some(neighbour_facing) = chest_facing(neighbour_state) else {
            return state.clone();
        };
        if state_type == ChestType::Single
            && neighbour_type != ChestType::Single
            && facing == neighbour_facing
            && chest_connected_direction(neighbour_facing, neighbour_type)
                == direction_to_neighbour.opposite()
        {
            return canonical_state_with_chest_type(state, neighbour_type.opposite())
                .unwrap_or_else(|| state.clone());
        }
    } else if chest_connected_direction(facing, state_type) == direction_to_neighbour {
        return canonical_state_with_chest_type(state, ChestType::Single)
            .unwrap_or_else(|| state.clone());
    }
    state.clone()
}
pub fn copper_chest_update_shape(
    state_after_super: BlockState,
    direction_to_neighbour: Direction,
    neighbour_state: &BlockState,
) -> BlockState {
    if chest_state_can_connect_to(&state_after_super, neighbour_state) {
        if let (Some(state_type), Some(facing)) = (
            chest_type(&state_after_super),
            chest_facing(&state_after_super),
        ) {
            if state_type != ChestType::Single
                && chest_connected_direction(facing, state_type) == direction_to_neighbour
            {
                let properties = chest_effective_properties(&state_after_super);
                return canonical_state_with_properties(&neighbour_state.block, &properties)
                    .unwrap_or(state_after_super);
            }
        }
    }
    state_after_super
}
pub fn chest_state_after_neighbour_update(
    state: &BlockState,
    direction_to_neighbour: Direction,
    neighbour_state: &BlockState,
) -> Option<BlockState> {
    match chest_family(state) {
        Some(ChestFamily::Chest) | Some(ChestFamily::TrappedChest) => Some(chest_update_shape(
            state,
            direction_to_neighbour,
            neighbour_state,
        )),
        Some(ChestFamily::Copper) => Some(copper_chest_update_shape(
            chest_update_shape(state, direction_to_neighbour, neighbour_state),
            direction_to_neighbour,
            neighbour_state,
        )),
        Some(ChestFamily::EnderChest) | None => None,
    }
}
pub trait ChestLevelView {
    fn block_state_at(&self, pos: BlockPos) -> Option<BlockState>;
}
impl<F> ChestLevelView for F
where
    F: Fn(BlockPos) -> Option<BlockState>,
{
    fn block_state_at(&self, pos: BlockPos) -> Option<BlockState> {
        self(pos)
    }
}
pub fn candidate_partner_facing<L: ChestLevelView + ?Sized>(
    level: &L,
    current_block: &ResourceLocation,
    pos: BlockPos,
    neighbour_direction: Direction,
) -> Option<Direction> {
    let neighbour = level.block_state_at(pos.relative(neighbour_direction))?;
    if chest_can_connect_to(current_block, &neighbour)
        && chest_type(&neighbour)? == ChestType::Single
    {
        chest_facing(&neighbour)
    } else {
        None
    }
}
pub fn chest_type_for_placement<L: ChestLevelView + ?Sized>(
    level: &L,
    current_block: &ResourceLocation,
    pos: BlockPos,
    facing_direction: Direction,
) -> ChestType {
    if Some(facing_direction)
        == candidate_partner_facing(level, current_block, pos, facing_direction.clockwise())
    {
        ChestType::Left
    } else if Some(facing_direction)
        == candidate_partner_facing(
            level,
            current_block,
            pos,
            facing_direction.counter_clockwise(),
        )
    {
        ChestType::Right
    } else {
        ChestType::Single
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChestPlacementContext {
    pub clicked_pos: BlockPos,
    pub clicked_face: Direction,
    pub horizontal_direction: Direction,
    pub secondary_use_active: bool,
    pub replaced_fluid_is_water: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CopperChestVariant {
    weather_ordinal: u8,
    waxed: bool,
    unwaxed_path: &'static str,
}
fn copper_chest_variant(state: &BlockState) -> Option<CopperChestVariant> {
    if chest_family(state) != Some(ChestFamily::Copper) {
        return None;
    }
    match state.block.path.as_str() {
        "copper_chest" => Some(CopperChestVariant {
            weather_ordinal: 0,
            waxed: false,
            unwaxed_path: "copper_chest",
        }),
        "exposed_copper_chest" => Some(CopperChestVariant {
            weather_ordinal: 1,
            waxed: false,
            unwaxed_path: "exposed_copper_chest",
        }),
        "weathered_copper_chest" => Some(CopperChestVariant {
            weather_ordinal: 2,
            waxed: false,
            unwaxed_path: "weathered_copper_chest",
        }),
        "oxidized_copper_chest" => Some(CopperChestVariant {
            weather_ordinal: 3,
            waxed: false,
            unwaxed_path: "oxidized_copper_chest",
        }),
        "waxed_copper_chest" => Some(CopperChestVariant {
            weather_ordinal: 0,
            waxed: true,
            unwaxed_path: "copper_chest",
        }),
        "waxed_exposed_copper_chest" => Some(CopperChestVariant {
            weather_ordinal: 1,
            waxed: true,
            unwaxed_path: "exposed_copper_chest",
        }),
        "waxed_weathered_copper_chest" => Some(CopperChestVariant {
            weather_ordinal: 2,
            waxed: true,
            unwaxed_path: "weathered_copper_chest",
        }),
        "waxed_oxidized_copper_chest" => Some(CopperChestVariant {
            weather_ordinal: 3,
            waxed: true,
            unwaxed_path: "oxidized_copper_chest",
        }),
        _ => None,
    }
}
fn with_chest_block_path(state: &BlockState, block_path: &str) -> Option<BlockState> {
    canonical_state_with_properties(
        &ResourceLocation::new("minecraft", block_path),
        &chest_effective_properties(state),
    )
}
fn least_oxidized_connected_copper_chest<L: ChestLevelView + ?Sized>(
    level: &L,
    state: BlockState,
    pos: BlockPos,
) -> BlockState {
    let Some(placed_type) = chest_type(&state) else {
        return state;
    };
    if placed_type == ChestType::Single {
        return state;
    }
    let Some(connected_pos) = chest_connected_block_pos(pos, &state) else {
        return state;
    };
    let Some(connected_state) = level.block_state_at(connected_pos) else {
        return state;
    };
    let (Some(placed_variant), Some(connected_variant)) = (
        copper_chest_variant(&state),
        copper_chest_variant(&connected_state),
    ) else {
        return state;
    };
    let wax_mismatch = placed_variant.waxed != connected_variant.waxed;
    let updated_state = if wax_mismatch {
        with_chest_block_path(&state, placed_variant.unwaxed_path).unwrap_or_else(|| state.clone())
    } else {
        state.clone()
    };
    let connected_predicted_state = if wax_mismatch {
        with_chest_block_path(&connected_state, connected_variant.unwaxed_path)
            .unwrap_or(connected_state)
    } else {
        connected_state
    };
    let least_oxidized_path = if placed_variant.weather_ordinal <= connected_variant.weather_ordinal
    {
        updated_state.block.path.as_str()
    } else {
        connected_predicted_state.block.path.as_str()
    };
    with_chest_block_path(&updated_state, least_oxidized_path).unwrap_or(updated_state)
}
pub fn state_for_placement<L: ChestLevelView + ?Sized>(
    level: &L,
    block: &ResourceLocation,
    context: ChestPlacementContext,
) -> Option<BlockState> {
    let family = ChestFamily::from_block(block)?;
    let default_state = default_state_of(block)?;
    let mut properties = default_state.properties.clone();
    if !family.has_chest_type_property() {
        properties.insert(
            "facing".to_string(),
            context.horizontal_direction.opposite().name().to_string(),
        );
        properties.insert(
            "waterlogged".to_string(),
            context.replaced_fluid_is_water.to_string(),
        );
        return canonical_state_with_properties(block, &properties);
    }
    let mut placed_type = ChestType::Single;
    let mut facing_direction = context.horizontal_direction.opposite();
    if context.clicked_face.axis() != Axis::Y && context.secondary_use_active {
        let neighbour_facing = candidate_partner_facing(
            level,
            block,
            context.clicked_pos,
            context.clicked_face.opposite(),
        );
        if let Some(neighbour_facing) = neighbour_facing {
            if neighbour_facing.axis() != context.clicked_face.axis() {
                facing_direction = neighbour_facing;
                placed_type =
                    if neighbour_facing.counter_clockwise() == context.clicked_face.opposite() {
                        ChestType::Right
                    } else {
                        ChestType::Left
                    };
            }
        }
    }
    if placed_type == ChestType::Single && !context.secondary_use_active {
        placed_type = chest_type_for_placement(level, block, context.clicked_pos, facing_direction);
    }
    properties.insert("facing".to_string(), facing_direction.name().to_string());
    properties.insert(
        "type".to_string(),
        placed_type.serialized_name().to_string(),
    );
    properties.insert(
        "waterlogged".to_string(),
        context.replaced_fluid_is_water.to_string(),
    );
    let state = canonical_state_with_properties(block, &properties)?;
    if family == ChestFamily::Copper {
        Some(least_oxidized_connected_copper_chest(
            level,
            state,
            context.clicked_pos,
        ))
    } else {
        Some(state)
    }
}
pub fn is_blocked_chest_by_block<L: ChestLevelView + ?Sized>(level: &L, pos: BlockPos) -> bool {
    level
        .block_state_at(pos.above())
        .map(|state| state.is_redstone_conductor())
        .unwrap_or(false)
}
pub fn is_chest_blocked_at<L: ChestLevelView + ?Sized>(
    level: &L,
    pos: BlockPos,
    cat_sitting_on_chest: bool,
) -> bool {
    is_blocked_chest_by_block(level, pos) || cat_sitting_on_chest
}
fn default_state_of(block: &ResourceLocation) -> Option<BlockState> {
    let registered = BuiltInRegistries::blocks().get_by_location(block);
    if registered.location != *block {
        return None;
    }
    Some(registered.default_state().clone())
}
