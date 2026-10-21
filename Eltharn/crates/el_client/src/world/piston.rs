use crate::render::moving_piston::MovingPistonBlockEntity;
use crate::world::client_level::{ChunkPos, ClientLevel};
use el_core::block_state::BlockState;
use el_core::registry::BuiltInRegistries;
use el_core::resource_location::ResourceLocation;
use el_math::block_pos::BlockPos;
use el_math::direction::Direction;
use std::collections::HashMap;
#[path = "piston_push_reactions.rs"]
mod piston_push_reactions;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PistonPushReaction {
    Normal,
    Destroy,
    Block,
    PushOnly,
}
fn canonical_default_state(path: &str) -> BlockState {
    BuiltInRegistries::blocks()
        .get_by_location(&ResourceLocation::new("minecraft", path))
        .default_state()
        .clone()
}
pub fn push_reaction(block_path: &str) -> PistonPushReaction {
    let entries = piston_push_reactions::ENTRIES;
    entries
        .binary_search_by_key(&block_path, |&(path, _)| path)
        .map(|index| entries[index].1)
        .unwrap_or(PistonPushReaction::Normal)
}
fn air_state() -> BlockState {
    canonical_default_state("air")
}
fn canonical_state_with_properties(state: BlockState, changes: &[(&str, &str)]) -> BlockState {
    let mut properties = state.properties;
    for (name, value) in changes {
        properties.insert((*name).to_string(), (*value).to_string());
    }
    let id = BuiltInRegistries::get_id_by_state(&state.block, &properties)
        .expect("source-backed piston property tuple must be registered");
    BuiltInRegistries::get_state_by_id(id)
        .expect("resolved piston state ID must exist")
        .clone()
}
fn moving_piston_state(direction: Direction, sticky: bool) -> BlockState {
    canonical_state_with_properties(
        canonical_default_state("moving_piston"),
        &[
            ("facing", direction.name()),
            ("type", if sticky { "sticky" } else { "normal" }),
        ],
    )
}
fn piston_head_state(direction: Direction, sticky: bool) -> BlockState {
    canonical_state_with_properties(
        canonical_default_state("piston_head"),
        &[
            ("facing", direction.name()),
            ("type", if sticky { "sticky" } else { "normal" }),
        ],
    )
}
fn state_is_air(state: &BlockState) -> bool {
    matches!(
        state.block.path.as_str(),
        "air" | "cave_air" | "void_air" | ""
    )
}
fn relative_times(mut pos: BlockPos, direction: Direction, count: i32) -> BlockPos {
    for _ in 0..count.max(0) {
        pos = pos.relative(direction);
    }
    pos
}
struct Overlay<'a> {
    base: &'a ClientLevel,
    writes: Vec<(BlockPos, BlockState)>,
    latest: HashMap<BlockPos, BlockState>,
}
impl<'a> Overlay<'a> {
    fn new(base: &'a ClientLevel) -> Self {
        Self {
            base,
            writes: Vec::new(),
            latest: HashMap::new(),
        }
    }
    fn get(&self, pos: BlockPos) -> BlockState {
        match self.latest.get(&pos) {
            Some(state) => state.clone(),
            None => self.base.get_block_state(pos),
        }
    }
    fn set(&mut self, pos: BlockPos, state: BlockState) {
        self.writes.push((pos, state.clone()));
        self.latest.insert(pos, state);
    }
    fn has_block_entity(&self, pos: BlockPos) -> bool {
        has_vanilla_block_entity(&self.get(pos))
            || self
                .base
                .chunks
                .get(&ChunkPos::from_block(pos))
                .is_some_and(|chunk| chunk.block_entities.contains_key(&pos))
    }
}
fn has_vanilla_block_entity(state: &BlockState) -> bool {
    if state.block.namespace != "minecraft" {
        return false;
    }
    let block_id = state.block.to_string();
    if [
        "all_signs",
        "banners",
        "shulker_boxes",
        "copper_chests",
        "copper_golem_statues",
    ]
    .iter()
    .any(|tag| el_core::block_behaviour::is_java_26_2_vanilla_block_in_tag(&block_id, tag))
    {
        return true;
    }
    matches!(
        state.block.path.as_str(),
        "furnace"
            | "chest"
            | "trapped_chest"
            | "ender_chest"
            | "jukebox"
            | "dispenser"
            | "dropper"
            | "spawner"
            | "creaking_heart"
            | "moving_piston"
            | "brewing_stand"
            | "enchanting_table"
            | "end_portal"
            | "beacon"
            | "skeleton_skull"
            | "skeleton_wall_skull"
            | "creeper_head"
            | "creeper_wall_head"
            | "dragon_head"
            | "dragon_wall_head"
            | "zombie_head"
            | "zombie_wall_head"
            | "wither_skeleton_skull"
            | "wither_skeleton_wall_skull"
            | "player_head"
            | "player_wall_head"
            | "piglin_head"
            | "piglin_wall_head"
            | "daylight_detector"
            | "hopper"
            | "comparator"
            | "structure_block"
            | "end_gateway"
            | "command_block"
            | "chain_command_block"
            | "repeating_command_block"
            | "conduit"
            | "barrel"
            | "smoker"
            | "blast_furnace"
            | "lectern"
            | "bell"
            | "jigsaw"
            | "campfire"
            | "soul_campfire"
            | "bee_nest"
            | "beehive"
            | "sculk_sensor"
            | "calibrated_sculk_sensor"
            | "sculk_catalyst"
            | "sculk_shrieker"
            | "chiseled_bookshelf"
            | "acacia_shelf"
            | "bamboo_shelf"
            | "birch_shelf"
            | "cherry_shelf"
            | "crimson_shelf"
            | "dark_oak_shelf"
            | "jungle_shelf"
            | "mangrove_shelf"
            | "oak_shelf"
            | "pale_oak_shelf"
            | "spruce_shelf"
            | "warped_shelf"
            | "suspicious_sand"
            | "suspicious_gravel"
            | "decorated_pot"
            | "crafter"
            | "trial_spawner"
            | "vault"
            | "test_block"
            | "test_instance_block"
            | "potent_sulfur"
    )
}
pub struct PistonEventOutcome {
    pub changed_positions: Vec<(BlockPos, BlockState)>,
    pub entities: Vec<(BlockPos, MovingPistonBlockEntity)>,
    pub finalized_entities: Vec<BlockPos>,
}
pub fn client_block_event_like_java(
    level: &ClientLevel,
    pos: BlockPos,
    state_at_pos: &BlockState,
    action_id: i32,
    action_parameter: i32,
    moving_entities: &HashMap<BlockPos, MovingPistonBlockEntity>,
) -> Option<PistonEventOutcome> {
    let direction = state_at_pos
        .get_property("facing")
        .and_then(|value| Direction::from_name(value))
        .unwrap_or(Direction::Down);
    let extending = action_id == 0;
    let is_sticky = state_at_pos.block.path == "sticky_piston";
    let mut overlay = Overlay::new(level);
    let mut entities = Vec::new();
    let mut finalized_entities = Vec::new();
    if extending {
        if !move_blocks_like_java(&mut overlay, &mut entities, pos, direction, true, is_sticky) {
            return None;
        }
        overlay.set(
            pos,
            canonical_state_with_properties(state_at_pos.clone(), &[("extended", "true")]),
        );
    } else if action_id == 1 || action_id == 2 {
        if !move_blocks_retract_tail_like_java(
            &mut overlay,
            &mut entities,
            pos,
            direction,
            is_sticky,
            action_id,
            action_parameter,
            moving_entities,
            &mut finalized_entities,
        ) {
            return None;
        }
    } else {
        return None;
    }
    let changed_positions = overlay.writes.clone();
    Some(PistonEventOutcome {
        changed_positions,
        entities,
        finalized_entities,
    })
}
fn move_blocks_like_java(
    overlay: &mut Overlay<'_>,
    entities: &mut Vec<(BlockPos, MovingPistonBlockEntity)>,
    piston_pos: BlockPos,
    direction: Direction,
    extending: bool,
    is_sticky: bool,
) -> bool {
    let arm_pos = piston_pos.relative(direction);
    if !extending && overlay.get(arm_pos).block.path == "piston_head" {
        overlay.set(arm_pos, air_state());
    }
    let mut resolver = PistonStructureResolver::new(overlay, piston_pos, direction, extending);
    if !resolver.resolve() {
        return false;
    }
    let to_push = resolver.to_push.clone();
    let to_destroy = resolver.to_destroy.clone();
    let mut delete_after_move: HashMap<BlockPos, BlockState> = HashMap::new();
    let mut to_push_shapes: Vec<BlockState> = Vec::with_capacity(to_push.len());
    for pos in &to_push {
        let state = overlay.get(*pos);
        to_push_shapes.push(state.clone());
        delete_after_move.insert(*pos, state);
    }
    let push_direction = if extending {
        direction
    } else {
        direction.opposite()
    };
    for pos in to_destroy.iter().rev() {
        overlay.set(*pos, air_state());
    }
    for index in (0..to_push.len()).rev() {
        let source = to_push[index];
        let target = source.relative(push_direction);
        delete_after_move.remove(&target);
        let actual_state = moving_piston_state(direction, false);
        overlay.set(target, actual_state);
        entities.push((
            target,
            MovingPistonBlockEntity::new(
                to_push_shapes[index].clone(),
                direction,
                extending,
                false,
            ),
        ));
    }
    if extending {
        delete_after_move.remove(&arm_pos);
        let head_state = piston_head_state(direction, is_sticky);
        let arm_moving_state = moving_piston_state(direction, is_sticky);
        overlay.set(arm_pos, arm_moving_state);
        entities.push((
            arm_pos,
            MovingPistonBlockEntity::new(head_state, direction, true, true),
        ));
    }
    let air = air_state();
    for pos in delete_after_move.keys() {
        overlay.set(*pos, air.clone());
    }
    true
}
fn move_blocks_retract_tail_like_java(
    overlay: &mut Overlay<'_>,
    entities: &mut Vec<(BlockPos, MovingPistonBlockEntity)>,
    pos: BlockPos,
    direction: Direction,
    is_sticky: bool,
    action_id: i32,
    action_parameter: i32,
    moving_entities: &HashMap<BlockPos, MovingPistonBlockEntity>,
    finalized_entities: &mut Vec<BlockPos>,
) -> bool {
    let arm_pos = pos.relative(direction);
    finalize_existing_moving_piece_like_java(overlay, moving_entities, arm_pos, finalized_entities);
    let moving_piston_state = moving_piston_state(direction, is_sticky);
    overlay.set(pos, moving_piston_state);
    let base_facing = Direction::from_3d_data_value(action_parameter & 7);
    let base_state = canonical_state_with_properties(
        canonical_default_state(if is_sticky { "sticky_piston" } else { "piston" }),
        &[("facing", base_facing.name())],
    );
    entities.push((
        pos,
        MovingPistonBlockEntity::new(base_state, direction, false, true),
    ));
    if is_sticky {
        let two_pos = pos.relative(direction).relative(direction);
        let moving_two = overlay.get(two_pos);
        let piston_piece = moving_two.block.path == "moving_piston"
            && moving_entities
                .get(&two_pos)
                .is_some_and(|entity| entity.direction == direction && entity.extending);
        if piston_piece {
            finalize_existing_moving_piece_like_java(
                overlay,
                moving_entities,
                two_pos,
                finalized_entities,
            );
        }
        if !piston_piece {
            let pull_allowed = action_id == 1
                && !state_is_air(&moving_two)
                && is_pushable_like_java(
                    overlay,
                    &moving_two,
                    two_pos,
                    direction.opposite(),
                    false,
                    direction,
                )
                && (push_reaction(&moving_two.block.path) == PistonPushReaction::Normal
                    || moving_two.block.path == "piston"
                    || moving_two.block.path == "sticky_piston");
            if !pull_allowed {
                overlay.set(arm_pos, air_state());
            } else {
                move_blocks_like_java(overlay, entities, pos, direction, false, is_sticky);
            }
        }
    } else {
        overlay.set(arm_pos, air_state());
    }
    true
}
fn finalize_existing_moving_piece_like_java(
    overlay: &mut Overlay<'_>,
    moving_entities: &HashMap<BlockPos, MovingPistonBlockEntity>,
    pos: BlockPos,
    finalized_entities: &mut Vec<BlockPos>,
) {
    let Some(entity) = moving_entities.get(&pos) else {
        return;
    };
    finalized_entities.push(pos);
    if overlay.get(pos).block.path == "moving_piston" {
        overlay.set(
            pos,
            if entity.is_source_piston {
                air_state()
            } else {
                entity.moved_state.clone()
            },
        );
    }
}
fn is_pushable_like_java(
    overlay: &Overlay<'_>,
    state: &BlockState,
    pos: BlockPos,
    direction: Direction,
    allow_destroyable: bool,
    connection_direction: Direction,
) -> bool {
    let path = state.block.path.as_str();
    if state_is_air(state) {
        return true;
    }
    if matches!(
        path,
        "obsidian" | "crying_obsidian" | "respawn_anchor" | "reinforced_deepslate"
    ) {
        return false;
    }
    if !matches!(path, "piston" | "sticky_piston") {
        if el_core::block_behaviour::destroy_speed(state.get_block(), state) == -1.0 {
            return false;
        }
        return match push_reaction(path) {
            PistonPushReaction::Block => false,
            PistonPushReaction::Destroy => allow_destroyable,
            PistonPushReaction::PushOnly => direction == connection_direction,
            PistonPushReaction::Normal => !overlay.has_block_entity(pos),
        };
    }
    if state.get_property("extended").map(String::as_str) == Some("true") {
        return false;
    }
    !overlay.has_block_entity(pos)
}
struct PistonStructureResolver<'a> {
    overlay: &'a Overlay<'a>,
    piston_pos: BlockPos,
    start_pos: BlockPos,
    push_direction: Direction,
    piston_direction: Direction,
    extending: bool,
    to_push: Vec<BlockPos>,
    to_destroy: Vec<BlockPos>,
}
const MAX_PUSH_DEPTH: usize = 12;
impl<'a> PistonStructureResolver<'a> {
    fn new(
        overlay: &'a Overlay<'a>,
        piston_pos: BlockPos,
        direction: Direction,
        extending: bool,
    ) -> Self {
        let push_direction = if extending {
            direction
        } else {
            direction.opposite()
        };
        let start_pos = if extending {
            piston_pos.relative(direction)
        } else {
            relative_times(piston_pos.relative(direction), direction, 1)
        };
        Self {
            overlay,
            piston_pos,
            start_pos,
            push_direction,
            piston_direction: direction,
            extending,
            to_push: Vec::new(),
            to_destroy: Vec::new(),
        }
    }
    fn resolve(&mut self) -> bool {
        self.to_push.clear();
        self.to_destroy.clear();
        let next_state = self.overlay.get(self.start_pos);
        if !is_pushable_like_java(
            self.overlay,
            &next_state,
            self.start_pos,
            self.push_direction,
            false,
            self.piston_direction,
        ) {
            if self.extending
                && push_reaction(&next_state.block.path) == PistonPushReaction::Destroy
            {
                self.to_destroy.push(self.start_pos);
                return true;
            }
            return false;
        }
        if !self.add_block_line(self.start_pos, self.push_direction) {
            return false;
        }
        let mut index = 0;
        while index < self.to_push.len() {
            let pos = self.to_push[index];
            if is_sticky_state(&self.overlay.get(pos)) && !self.add_branching_blocks(pos) {
                return false;
            }
            index += 1;
        }
        true
    }
    fn add_block_line(&mut self, start: BlockPos, direction: Direction) -> bool {
        let mut next_state = self.overlay.get(start);
        if state_is_air(&next_state) {
            return true;
        }
        if !is_pushable_like_java(
            self.overlay,
            &next_state,
            start,
            self.push_direction,
            false,
            direction,
        ) {
            return true;
        }
        if start == self.piston_pos {
            return true;
        }
        if self.to_push.contains(&start) {
            return true;
        }
        let mut block_count = 1usize;
        if block_count + self.to_push.len() > MAX_PUSH_DEPTH {
            return false;
        }
        while is_sticky_state(&next_state) {
            let pos = relative_times(start, self.push_direction.opposite(), block_count as i32);
            let previous_state = next_state.clone();
            next_state = self.overlay.get(pos);
            if state_is_air(&next_state)
                || !can_stick_to_each_other(&previous_state, &next_state)
                || !is_pushable_like_java(
                    self.overlay,
                    &next_state,
                    pos,
                    self.push_direction,
                    false,
                    self.push_direction.opposite(),
                )
                || pos == self.piston_pos
            {
                break;
            }
            block_count += 1;
            if block_count + self.to_push.len() > MAX_PUSH_DEPTH {
                return false;
            }
        }
        let mut blocks_added = 0usize;
        for offset in (0..block_count).rev() {
            self.to_push.push(relative_times(
                start,
                self.push_direction.opposite(),
                offset as i32,
            ));
            blocks_added += 1;
        }
        let mut i = 1i32;
        loop {
            let pos = relative_times(start, self.push_direction, i);
            if let Some(collision_index) = self.to_push.iter().position(|p| *p == pos) {
                self.reorder_list_at_collision(blocks_added, collision_index);
                for j in 0..=collision_index + blocks_added {
                    let block_pos = self.to_push[j];
                    if is_sticky_state(&self.overlay.get(block_pos))
                        && !self.add_branching_blocks(block_pos)
                    {
                        return false;
                    }
                }
                return true;
            }
            next_state = self.overlay.get(pos);
            if state_is_air(&next_state) {
                return true;
            }
            if !is_pushable_like_java(
                self.overlay,
                &next_state,
                pos,
                self.push_direction,
                true,
                self.push_direction,
            ) || pos == self.piston_pos
            {
                return false;
            }
            if push_reaction(&next_state.block.path) == PistonPushReaction::Destroy {
                self.to_destroy.push(pos);
                return true;
            }
            if self.to_push.len() >= MAX_PUSH_DEPTH {
                return false;
            }
            self.to_push.push(pos);
            blocks_added += 1;
            i += 1;
        }
    }
    fn reorder_list_at_collision(&mut self, blocks_added: usize, collision_pos: usize) {
        let total = self.to_push.len();
        let head: Vec<BlockPos> = self.to_push[..collision_pos].to_vec();
        let last_line_added: Vec<BlockPos> = self.to_push[total - blocks_added..total].to_vec();
        let collision_to_line: Vec<BlockPos> =
            self.to_push[collision_pos..total - blocks_added].to_vec();
        self.to_push.clear();
        self.to_push.extend_from_slice(&head);
        self.to_push.extend_from_slice(&last_line_added);
        self.to_push.extend_from_slice(&collision_to_line);
    }
    fn add_branching_blocks(&mut self, from_pos: BlockPos) -> bool {
        let from_state = self.overlay.get(from_pos);
        for direction in Direction::VALUES {
            if direction.axis() == self.push_direction.axis() {
                continue;
            }
            let neighbour_pos = from_pos.relative(direction);
            let neighbour_state = self.overlay.get(neighbour_pos);
            if can_stick_to_each_other(&neighbour_state, &from_state)
                && !self.add_block_line(neighbour_pos, direction)
            {
                return false;
            }
        }
        true
    }
}
fn is_sticky_state(state: &BlockState) -> bool {
    matches!(state.block.path.as_str(), "slime_block" | "honey_block")
}
fn can_stick_to_each_other(first: &BlockState, second: &BlockState) -> bool {
    if first.block.path == "honey_block" && second.block.path == "slime_block" {
        return false;
    }
    if first.block.path == "slime_block" && second.block.path == "honey_block" {
        return false;
    }
    is_sticky_state(first) || is_sticky_state(second)
}
