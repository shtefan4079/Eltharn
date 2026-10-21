#![allow(dead_code)]
use crate::gameplay::game_mode::{Abilities, GameType};
use crate::gameplay::player::LocalPlayer;
use crate::network::local_connection::LocalConnection;
use crate::network::protocol_backend::RemotePacketSender as PacketSender;
use crate::world::client_level::{ChunkPos, ClientLevel};
use el_core::BlockState;
use el_core::item::ItemStack;
use el_core::registry::BuiltInRegistries;
use el_math::Direction;
use el_math::block_pos::BlockPos;
use el_nbt::Tag;
use el_protocol::packets::game::ContainerInput;
pub(crate) mod placement;
const ELTHARN_NOISY_DEBUG_ENV: &str = "ELTHARN_NOISY_DEBUG";
const RELEASE_USE_ITEM_ACTION: i32 = 5;
const STAB_ACTION: i32 = 7;
const MAIN_HAND: usize = 0;
const OFF_HAND: usize = 1;
const OFFHAND_SLOT: usize = 40;
const INVENTORY_SIZE: usize = 36;
fn item_slot_for_hand(hand: usize, active_hotbar_slot: usize) -> usize {
    if hand == OFF_HAND {
        OFFHAND_SLOT
    } else {
        active_hotbar_slot
    }
}
fn noisy_debug_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var(ELTHARN_NOISY_DEBUG_ENV)
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
            .unwrap_or(false)
    })
}
fn should_print_console_line(line: &str) -> bool {
    if noisy_debug_enabled()
        || std::env::var("ELTHARN_VERBOSE_LOGS")
            .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
            .unwrap_or(false)
        || matches!(
            std::env::var("ELTHARN_LOGS").as_deref(),
            Ok("all" | "trace" | "debug" | "info")
        )
    {
        return true;
    }
    if el_protocol_backend::diagnostics::console_enabled()
        && (line.starts_with("[MovementDebug]")
            || line.starts_with("[MoveDebug]")
            || line.starts_with("[SlideDebug]")
            || line.starts_with("[NetDebug]"))
    {
        return true;
    }
    if !matches!(
        std::env::var("ELTHARN_LOGS").as_deref(),
        Ok("error" | "warn")
    ) {
        return false;
    }
    let lower = line.to_ascii_lowercase();
    line.starts_with("[ERROR]")
        || lower.contains("error")
        || lower.contains("failed")
        || lower.contains("fatal")
        || lower.contains("disconnect")
        || lower.contains("kicked")
        || lower.contains("kick reason")
}
fn tag_compound_get<'a>(fields: &'a [(String, Tag)], key: &str) -> Option<&'a Tag> {
    fields
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
}
fn tag_string(value: &Tag) -> Option<&str> {
    match value {
        Tag::String(value) => Some(value.as_str()),
        _ => None,
    }
}
fn adventure_block_id_matches(value: &Tag, state: &BlockState) -> bool {
    let Some(identifier) = tag_string(value) else {
        return false;
    };
    let block_id = state.block.to_string();
    if let Some(tag_id) = identifier.strip_prefix('#') {
        return el_core::block_behaviour::is_java_26_2_vanilla_block_in_tag(&block_id, tag_id);
    }
    let identifier = if identifier.contains(':') {
        identifier.to_owned()
    } else {
        format!("minecraft:{identifier}")
    };
    identifier == block_id
}
fn adventure_blocks_match(value: &Tag, state: &BlockState) -> bool {
    match value {
        Tag::String(_) => adventure_block_id_matches(value, state),
        Tag::List(values) => values
            .iter()
            .any(|entry| adventure_block_id_matches(entry, state)),
        _ => false,
    }
}
fn adventure_property_value_matches(value: &Tag, actual: &str) -> bool {
    match value {
        Tag::String(expected) => expected == actual,
        Tag::Compound(fields) => {
            if let Some(expected) = tag_compound_get(fields, "value").and_then(tag_string) {
                return expected == actual;
            }
            if let Some(exact) = tag_compound_get(fields, "Exact") {
                return adventure_property_value_matches(exact, actual);
            }
            if let Some(range) = tag_compound_get(fields, "Range") {
                return adventure_property_value_matches(range, actual);
            }
            let min = tag_compound_get(fields, "min").and_then(tag_string);
            let max = tag_compound_get(fields, "max").and_then(tag_string);
            if min.is_none() && max.is_none() {
                return false;
            }
            let Ok(actual_number) = actual.parse::<i64>() else {
                return false;
            };
            if let Some(min) = min {
                let Ok(min_number) = min.parse::<i64>() else {
                    return false;
                };
                if actual_number < min_number {
                    return false;
                }
            }
            if let Some(max) = max {
                let Ok(max_number) = max.parse::<i64>() else {
                    return false;
                };
                if actual_number > max_number {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}
fn adventure_properties_match(value: &Tag, state: &BlockState) -> bool {
    match value {
        Tag::Compound(fields) => fields.iter().all(|(name, matcher)| {
            state
                .get_property(name)
                .is_some_and(|actual| adventure_property_value_matches(matcher, actual))
        }),
        Tag::List(entries) => entries.iter().all(|entry| {
            let Tag::Compound(fields) = entry else {
                return false;
            };
            let Some(name) = tag_compound_get(fields, "name").and_then(tag_string) else {
                return false;
            };
            let matcher = tag_compound_get(fields, "value_matcher")
                .or_else(|| tag_compound_get(fields, "value"));
            let Some(matcher) = matcher else {
                return false;
            };
            state
                .get_property(name)
                .is_some_and(|actual| adventure_property_value_matches(matcher, actual))
        }),
        _ => false,
    }
}
fn adventure_block_predicate_matches(value: &Tag, state: &BlockState) -> bool {
    let Tag::Compound(fields) = value else {
        return false;
    };
    if let Some(blocks) = tag_compound_get(fields, "blocks")
        && !adventure_blocks_match(blocks, state)
    {
        return false;
    }
    if let Some(properties) = tag_compound_get(fields, "properties")
        && !adventure_properties_match(properties, state)
    {
        return false;
    }
    if tag_compound_get(fields, "nbt").is_some() || tag_compound_get(fields, "components").is_some()
    {
        return false;
    }
    true
}
fn item_adventure_predicate_matches(
    stack: &ItemStack,
    component_name: &str,
    state: &BlockState,
) -> bool {
    let Some(el_core::item::DataComponent::Generic { value, .. }) =
        stack.components.get(component_name)
    else {
        return false;
    };
    let predicates: Vec<&Tag> = match value {
        Tag::List(entries) => entries.iter().collect(),
        Tag::Compound(fields) => {
            if let Some(Tag::List(entries)) = tag_compound_get(fields, "predicates") {
                entries.iter().collect()
            } else if tag_compound_get(fields, "predicates").is_some() {
                vec![tag_compound_get(fields, "predicates").unwrap()]
            } else {
                vec![value]
            }
        }
        _ => return false,
    };
    predicates
        .iter()
        .any(|predicate| adventure_block_predicate_matches(predicate, state))
}
pub(crate) fn item_can_break_block_in_adventure_mode(
    stack: &ItemStack,
    state: &BlockState,
) -> bool {
    item_adventure_predicate_matches(stack, "minecraft:can_break", state)
}
pub(crate) fn item_can_place_on_block_in_adventure_mode(
    stack: &ItemStack,
    state: &BlockState,
) -> bool {
    item_adventure_predicate_matches(stack, "minecraft:can_place_on", state)
}
pub(crate) fn item_can_use_on_block_in_adventure_mode(
    stack: &ItemStack,
    state: &BlockState,
) -> bool {
    item_can_break_block_in_adventure_mode(stack, state)
        || item_can_place_on_block_in_adventure_mode(stack, state)
}
macro_rules! println {
    ($($arg:tt)*) => {
        { let line = format!($($arg)*); if should_print_console_line(& line) {
        std::println!("{}", line); } }
    };
}
#[derive(Clone, Debug, PartialEq)]
pub struct MultiPlayerGameMode {
    pub local_player_mode: GameType,
    pub previous_local_player_mode: Option<GameType>,
    pub abilities: Abilities,
    pub destroy_block_pos: BlockPos,
    pub destroying_item: ItemStack,
    pub destroy_progress: f32,
    pub destroy_ticks: f32,
    pub destroy_delay: i32,
    pub is_destroying: bool,
    pub carried_index: i32,
    pub pending_place_sound: Option<(BlockPos, el_core::block_state::BlockState)>,
    pending_destroyed_blocks: Vec<(BlockPos, BlockState)>,
    pending_destroy_hit_sounds: Vec<(BlockPos, BlockState)>,
    pending_block_interaction_sounds: Vec<BlockInteractionSound>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockInteractionSound {
    pub pos: BlockPos,
    pub sound_event: &'static str,
    pub volume: f32,
    pub pitch_base: f32,
    pub pitch_random_range: f32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UseItemOutcome {
    pub consumes_action: bool,
    pub swing_source_client: bool,
    pub starts_using_item: bool,
}
impl UseItemOutcome {
    const PASS: Self = Self {
        consumes_action: false,
        swing_source_client: false,
        starts_using_item: false,
    };
    const CONSUME: Self = Self {
        consumes_action: true,
        swing_source_client: false,
        starts_using_item: true,
    };
    const SUCCESS: Self = Self {
        consumes_action: true,
        swing_source_client: true,
        starts_using_item: false,
    };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UseItemOnOutcome {
    pub consumes_action: bool,
    pub swing_source_client: bool,
    pub is_failure: bool,
}
impl UseItemOnOutcome {
    pub const PASS: Self = Self {
        consumes_action: false,
        swing_source_client: false,
        is_failure: false,
    };
    pub const FAIL: Self = Self {
        consumes_action: false,
        swing_source_client: false,
        is_failure: true,
    };
    pub const CONSUME: Self = Self {
        consumes_action: true,
        swing_source_client: false,
        is_failure: false,
    };
    pub const SUCCESS: Self = Self {
        consumes_action: true,
        swing_source_client: true,
        is_failure: false,
    };
}
impl Default for MultiPlayerGameMode {
    fn default() -> Self {
        let mut mode = Self {
            local_player_mode: GameType::DEFAULT_MODE,
            previous_local_player_mode: None,
            abilities: Abilities::default(),
            destroy_block_pos: BlockPos::new(-1, -1, -1),
            destroying_item: ItemStack::empty(),
            destroy_progress: 0.0,
            destroy_ticks: 0.0,
            destroy_delay: 0,
            is_destroying: false,
            carried_index: 0,
            pending_place_sound: None,
            pending_destroyed_blocks: Vec::new(),
            pending_destroy_hit_sounds: Vec::new(),
            pending_block_interaction_sounds: Vec::new(),
        };
        mode.local_player_mode
            .update_player_abilities(&mut mode.abilities);
        mode
    }
}
impl MultiPlayerGameMode {
    fn send_swing_for_hand(remote: &PacketSender, hand: usize) {
        if hand == OFF_HAND {
            remote.send_swing_off_hand();
        } else {
            remote.send_swing_main_hand();
        }
    }
    pub fn new() -> Self {
        Self::default()
    }
    pub fn abilities(&self) -> Abilities {
        self.abilities
    }
    pub fn set_flying(&mut self, flying: bool) {
        self.abilities.flying = flying && self.abilities.mayfly;
    }
    pub fn apply_server_abilities(&mut self, abilities: Abilities) {
        self.abilities = abilities;
    }
    pub fn local_player_mode(&self) -> GameType {
        self.local_player_mode
    }
    pub fn is_creative(&self) -> bool {
        self.local_player_mode.is_creative()
    }
    pub fn adjust_player(&mut self, _player: &mut LocalPlayer) {
        self.local_player_mode
            .update_player_abilities(&mut self.abilities);
    }
    pub fn set_local_mode_with_previous(
        &mut self,
        mode: GameType,
        previous_mode: Option<GameType>,
    ) {
        self.local_player_mode = mode;
        self.previous_local_player_mode = previous_mode;
        self.local_player_mode
            .update_player_abilities(&mut self.abilities);
    }
    pub fn set_local_mode(&mut self, mode: GameType) {
        if mode != self.local_player_mode {
            self.previous_local_player_mode = Some(self.local_player_mode);
        }
        self.local_player_mode = mode;
        self.local_player_mode
            .update_player_abilities(&mut self.abilities);
    }
    pub fn can_hurt_player(&self) -> bool {
        self.local_player_mode.is_survival()
    }
    fn block_action_restricted(
        &self,
        level: &ClientLevel,
        player: &LocalPlayer,
        pos: BlockPos,
        active_hotbar_slot: usize,
    ) -> bool {
        if !self.local_player_mode.is_block_placing_restricted() {
            return false;
        }
        if self.local_player_mode == GameType::Spectator {
            return true;
        }
        if self.abilities.may_build {
            return false;
        }
        let Some(stack) = player.inventory.get(active_hotbar_slot) else {
            return true;
        };
        if stack.is_empty() {
            return true;
        }
        let state = level.get_block_state(pos);
        !item_can_break_block_in_adventure_mode(stack, &state)
    }
    fn player_will_destroy_emits_local_break_event(state: &BlockState) -> bool {
        !(state.block.namespace == "minecraft"
            && matches!(state.block.path.as_str(), "fire" | "soul_fire"))
    }
    pub fn destroy_block(&mut self, pos: BlockPos, level: &mut ClientLevel) -> bool {
        let state = level.get_block_state(pos);
        if state.is_known_vanilla_air() {
            return false;
        }
        let target = crate::gameplay::player::correction::replacement_state_for_destroyed_block(
            &state, &level.air,
        );
        let changed = level.set_block(pos, target);
        if changed && Self::player_will_destroy_emits_local_break_event(&state) {
            self.pending_destroyed_blocks.push((pos, state));
        }
        changed
    }
    fn destroy_block_predicted(
        &mut self,
        pos: BlockPos,
        player_pos: glam::DVec3,
        level: &mut ClientLevel,
        sequence: u32,
    ) -> bool {
        let known_server_state = level.get_block_state(pos);
        level
            .block_state_prediction_handler
            .start_predicting_with_sequence(sequence);
        let changed = self.destroy_block(pos, level);
        if changed {
            level
                .block_state_prediction_handler
                .retain_known_server_state_at_sequence(
                    pos,
                    known_server_state,
                    player_pos,
                    sequence,
                );
            for (neighbor, old_neighbor) in level.take_neighbor_prediction_old_states() {
                level
                    .block_state_prediction_handler
                    .retain_known_server_state_at_sequence(
                        neighbor,
                        old_neighbor,
                        player_pos,
                        sequence,
                    );
            }
        }
        level.block_state_prediction_handler.close();
        changed
    }
    fn send_player_action(
        &self,
        remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
        action: i32,
        pos: BlockPos,
        direction: u8,
        sequence: u32,
    ) {
        if let Some(remote) = remote_conn {
            match action {
                0 => {
                    remote.send_start_destroy_block(pos.x, pos.y, pos.z, direction, sequence);
                }
                1 => remote.send_abort_destroy_block(pos.x, pos.y, pos.z, direction, sequence),
                2 => remote.send_stop_destroy_block(pos.x, pos.y, pos.z, direction, sequence),
                3 => remote.send_drop_all_items(),
                4 => remote.send_drop_item(),
                RELEASE_USE_ITEM_ACTION => remote.send_release_using_item(),
                STAB_ACTION => remote.send_stab(),
                _ => {}
            }
        }
        if let Some(local) = local_conn.as_mut() {
            let pkt = el_protocol::packets::game::ServerboundPlayerActionPacket {
                action,
                pos: el_protocol::packets::game::ProtoBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                direction: direction as i8,
                sequence: sequence as i32,
            };
            let _ = local.send_packet(&pkt);
        }
    }
    pub fn start_destroy_block(
        &mut self,
        pos: BlockPos,
        direction: el_math::Direction,
        player: &mut LocalPlayer,
        level: &mut ClientLevel,
        player_entity_id: i32,
        active_hotbar_slot: usize,
        remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
        sequence_counter: &mut u32,
    ) -> bool {
        if self.block_action_restricted(level, player, pos, active_hotbar_slot) {
            return false;
        }
        if !level.is_within_bounds(pos) {
            return false;
        }
        if self.abilities.instabuild {
            *sequence_counter = sequence_counter.wrapping_add(1);
            self.destroy_block_predicted(pos, player.exact_position(), level, *sequence_counter);
            level.destroy_block_progress(player_entity_id, [pos.x, pos.y, pos.z], -1);
            self.send_player_action(
                remote_conn,
                &mut *local_conn,
                0,
                pos,
                direction.data_3d() as u8,
                *sequence_counter,
            );
            self.destroy_delay = 5;
        } else if !self.is_destroying || !self.same_destroy_target(pos, player, active_hotbar_slot)
        {
            if self.is_destroying {
                self.send_player_action(
                    remote_conn,
                    &mut *local_conn,
                    1,
                    self.destroy_block_pos,
                    direction.data_3d() as u8,
                    0,
                );
            }
            let state = level.get_block_state(pos);
            let speed = player.get_destroy_progress(&state, active_hotbar_slot);
            let not_air = !state.is_known_vanilla_air();
            *sequence_counter = sequence_counter.wrapping_add(1);
            if not_air && speed >= 1.0 {
                self.destroy_block_predicted(
                    pos,
                    player.exact_position(),
                    level,
                    *sequence_counter,
                );
                level.destroy_block_progress(player_entity_id, [pos.x, pos.y, pos.z], -1);
                self.send_player_action(
                    remote_conn,
                    &mut *local_conn,
                    0,
                    pos,
                    direction.data_3d() as u8,
                    *sequence_counter,
                );
            } else {
                level
                    .block_state_prediction_handler
                    .start_predicting_with_sequence(*sequence_counter);
                self.is_destroying = true;
                self.destroy_block_pos = pos;
                self.destroying_item = player.inventory[active_hotbar_slot].clone();
                self.destroy_progress = 0.0;
                self.destroy_ticks = 0.0;
                level.destroy_block_progress(
                    player_entity_id,
                    [
                        self.destroy_block_pos.x,
                        self.destroy_block_pos.y,
                        self.destroy_block_pos.z,
                    ],
                    self.get_destroy_stage(),
                );
                self.send_player_action(
                    remote_conn,
                    &mut *local_conn,
                    0,
                    pos,
                    direction.data_3d() as u8,
                    *sequence_counter,
                );
                level.block_state_prediction_handler.close();
            }
        }
        true
    }
    pub fn stop_destroy_block(
        &mut self,
        level: &mut ClientLevel,
        player: &mut LocalPlayer,
        player_entity_id: i32,
        remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
    ) {
        if self.is_destroying {
            self.send_player_action(
                remote_conn,
                &mut *local_conn,
                1,
                self.destroy_block_pos,
                0,
                0,
            );
            self.is_destroying = false;
            self.destroy_progress = 0.0;
            player.reset_attack_strength_ticker();
            level.destroy_block_progress(
                player_entity_id,
                [
                    self.destroy_block_pos.x,
                    self.destroy_block_pos.y,
                    self.destroy_block_pos.z,
                ],
                -1,
            );
        }
    }
    pub fn continue_destroy_block(
        &mut self,
        pos: BlockPos,
        direction: el_math::Direction,
        player: &mut LocalPlayer,
        level: &mut ClientLevel,
        player_entity_id: i32,
        active_hotbar_slot: usize,
        remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
        sequence_counter: &mut u32,
    ) -> bool {
        self.ensure_has_sent_carried_item(remote_conn, &mut *local_conn, active_hotbar_slot);
        if self.destroy_delay > 0 {
            self.destroy_delay -= 1;
            return true;
        }
        if self.abilities.instabuild {
            if !level.is_within_bounds(pos) {
                return false;
            }
            *sequence_counter = sequence_counter.wrapping_add(1);
            self.destroy_delay = 5;
            self.destroy_block_predicted(pos, player.exact_position(), level, *sequence_counter);
            level.destroy_block_progress(player_entity_id, [pos.x, pos.y, pos.z], -1);
            self.send_player_action(
                remote_conn,
                &mut *local_conn,
                0,
                pos,
                direction.data_3d() as u8,
                *sequence_counter,
            );
            return true;
        }
        if self.same_destroy_target(pos, player, active_hotbar_slot) {
            let state = level.get_block_state(pos);
            if state.is_known_vanilla_air() {
                self.is_destroying = false;
                return false;
            }
            let speed = player.get_destroy_progress(&state, active_hotbar_slot);
            self.destroy_progress += speed;
            if self.destroy_ticks % 4.0 == 0.0 {
                self.pending_destroy_hit_sounds.push((pos, state.clone()));
            }
            self.destroy_ticks += 1.0;
            if self.destroy_progress >= 1.0 {
                *sequence_counter = sequence_counter.wrapping_add(1);
                self.is_destroying = false;
                self.destroy_block_predicted(
                    pos,
                    player.exact_position(),
                    level,
                    *sequence_counter,
                );
                self.send_player_action(
                    remote_conn,
                    &mut *local_conn,
                    2,
                    pos,
                    direction.data_3d() as u8,
                    *sequence_counter,
                );
                self.destroy_progress = 0.0;
                self.destroy_ticks = 0.0;
                self.destroy_delay = 5;
                level.destroy_block_progress(
                    player_entity_id,
                    [
                        self.destroy_block_pos.x,
                        self.destroy_block_pos.y,
                        self.destroy_block_pos.z,
                    ],
                    self.get_destroy_stage(),
                );
                return true;
            }
            level.destroy_block_progress(
                player_entity_id,
                [
                    self.destroy_block_pos.x,
                    self.destroy_block_pos.y,
                    self.destroy_block_pos.z,
                ],
                self.get_destroy_stage(),
            );
            return true;
        } else {
            return self.start_destroy_block(
                pos,
                direction,
                player,
                level,
                player_entity_id,
                active_hotbar_slot,
                remote_conn,
                &mut *local_conn,
                sequence_counter,
            );
        }
    }
    pub fn tick(
        &mut self,
        remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
        active_hotbar_slot: usize,
    ) {
        self.ensure_has_sent_carried_item(remote_conn, &mut *local_conn, active_hotbar_slot);
    }
    pub fn same_destroy_target(
        &self,
        pos: BlockPos,
        player: &LocalPlayer,
        active_hotbar_slot: usize,
    ) -> bool {
        let selected = &player.inventory[active_hotbar_slot];
        pos == self.destroy_block_pos
            && selected.item.location == self.destroying_item.item.location
            && selected.components == self.destroying_item.components
    }
    pub fn ensure_has_sent_carried_item(
        &mut self,
        remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
        active_hotbar_slot: usize,
    ) {
        let index = active_hotbar_slot as i32;
        if index != self.carried_index {
            self.carried_index = index;
            if let Some(remote) = remote_conn {
                remote.send_set_carried_item(index as u16);
            }
            if let Some(local) = local_conn.as_mut() {
                let pkt = el_protocol::packets::game::ServerboundSetCarriedItemPacket {
                    slot: index as i16,
                };
                let _ = local.send_packet(&pkt);
            }
        }
    }
    pub fn use_item_on(
        &mut self,
        player: &mut LocalPlayer,
        level: &mut ClientLevel,
        hand: usize,
        hit_result: &crate::gameplay::raycast::HitResult,
        remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
        sequence: u32,
        active_hotbar_slot: usize,
    ) -> UseItemOnOutcome {
        self.pending_block_interaction_sounds.clear();
        self.ensure_has_sent_carried_item(remote_conn, &mut *local_conn, active_hotbar_slot);
        let item_slot = item_slot_for_hand(hand, active_hotbar_slot);
        let pos = hit_result.pos;
        if !level.is_within_bounds(pos) {
            return UseItemOnOutcome::FAIL;
        }
        let mut outcome = if self.is_spectator() {
            UseItemOnOutcome::CONSUME
        } else {
            UseItemOnOutcome::PASS
        };
        let held_item_is_empty = player
            .inventory
            .get(item_slot)
            .is_none_or(ItemStack::is_empty);
        let have_something_in_our_hands = player
            .inventory
            .get(active_hotbar_slot)
            .is_some_and(|stack| !stack.is_empty())
            || player
                .inventory
                .get(OFFHAND_SLOT)
                .is_some_and(|stack| !stack.is_empty());
        let suppress_using_block = player.is_sneaking && have_something_in_our_hands;
        let block_outcome = if !self.is_spectator()
            && !suppress_using_block
            && block_interaction_consumes_action(
                &level.get_block_state(pos),
                level,
                hit_result,
                hand,
                !held_item_is_empty,
                self.abilities.may_build,
            ) {
            block_interaction_outcome(&level.get_block_state(pos), level, hit_result)
        } else {
            UseItemOnOutcome::PASS
        };
        let block_use_consumes_interaction = block_outcome.consumes_action;
        level
            .block_state_prediction_handler
            .start_predicting_with_sequence(sequence);
        if block_use_consumes_interaction && hand == MAIN_HAND {
            let clicked = level.get_block_state(pos);
            if placement::in_tag(&clicked, "minecraft:doors")
                || placement::in_tag(&clicked, "minecraft:trapdoors")
            {
                let opened = clicked
                    .get_property("open")
                    .is_some_and(|value| value == "true");
                let updated = canonicalize_block_state(
                    clicked.with_property("open", if opened { "false" } else { "true" }),
                );
                predict_block_write(level, pos, &updated, player, sequence);
                if let Some(sound) = placement::door_toggle_sound(&updated, pos, !opened) {
                    self.pending_block_interaction_sounds.push(sound);
                }
            }
        }
        if !held_item_is_empty
            && !block_use_consumes_interaction
            && !self.local_player_mode.is_block_placing_restricted()
        {
            let block_registry = el_core::registry::BuiltInRegistries::blocks();
            let item_location = player.inventory[item_slot].item.location.clone();
            let block = block_registry.get_by_location(&item_location);
            if block.location.path != "air" {
                outcome = UseItemOnOutcome::FAIL;
            }
            if block.location.path != "air"
                && let Some((target_pos, replacing_clicked)) =
                    placement::placement_target(level, block.default_state(), hit_result)
                && let Some(placement_state) = placement::state_for_context(
                    block.default_state().clone(),
                    level,
                    target_pos,
                    hit_result,
                    replacing_clicked,
                    player,
                )
            {
                let player_aabb = player.get_aabb();
                let collision_context = el_core::CollisionContext::placement_player(
                    player.is_sneaking,
                    player.exact_position().y,
                    player
                        .inventory
                        .get(active_hotbar_slot)
                        .filter(|stack| !stack.is_empty())
                        .map(|stack| stack.item.location.path.clone()),
                    player
                        .inventory
                        .get(36)
                        .filter(|stack| !stack.is_empty())
                        .map(|stack| stack.item.location.path.clone()),
                );
                let intersects_player = placement_state
                    .get_collision_shape_with_context(target_pos, &collision_context)
                    .to_aabbs()
                    .iter()
                    .any(|box_shape| {
                        player_aabb.intersects(&box_shape.move_by(
                            target_pos.x as f64,
                            target_pos.y as f64,
                            target_pos.z as f64,
                        ))
                    });
                if !intersects_player
                    && predict_block_write(level, target_pos, &placement_state, player, sequence)
                {
                    let mut placed_state = level.get_block_state(target_pos);
                    if placed_state.block == placement_state.block {
                        let component_state = placement::state_from_item_properties(
                            placed_state.clone(),
                            &player.inventory[item_slot],
                        );
                        if component_state != placed_state {
                            predict_block_write(
                                level,
                                target_pos,
                                &component_state,
                                player,
                                sequence,
                            );
                            placed_state = level.get_block_state(target_pos);
                        }
                    }
                    if placement::in_tag(&placed_state, "minecraft:doors") {
                        let upper = canonicalize_block_state(
                            placed_state.clone().with_property("half", "upper"),
                        );
                        predict_block_write(
                            level,
                            target_pos.relative(Direction::Up),
                            &upper,
                            player,
                            sequence,
                        );
                    }
                    if self.local_player_mode.is_survival() {
                        player.inventory[item_slot].count -= 1;
                    }
                    outcome = UseItemOnOutcome::SUCCESS;
                    self.pending_place_sound = Some((target_pos, placed_state));
                }
            }
        }
        if block_use_consumes_interaction {
            outcome = block_outcome;
        }
        let direction = hit_result.face.data_3d() as u8;
        let cursor_x = (hit_result.hit_pos.x - pos.x as f32).clamp(0.0, 1.0);
        let cursor_y = (hit_result.hit_pos.y - pos.y as f32).clamp(0.0, 1.0);
        let cursor_z = (hit_result.hit_pos.z - pos.z as f32).clamp(0.0, 1.0);
        if let Some(remote) = remote_conn {
            println!(
                "[MenuDebug] CLIENT_SEND_USE_ITEM_ON hand={} seq={} pos=[{},{},{}] face={} cursor=[{:.3},{:.3},{:.3}] carried_slot={}",
                hand,
                sequence,
                pos.x,
                pos.y,
                pos.z,
                direction,
                cursor_x,
                cursor_y,
                cursor_z,
                active_hotbar_slot
            );
            remote.send_use_item_on(
                hand as u8, pos.x, pos.y, pos.z, direction, cursor_x, cursor_y, cursor_z, false,
                sequence,
            );
            if outcome.swing_source_client {
                Self::send_swing_for_hand(remote, hand);
            }
        }
        if let Some(local) = local_conn.as_mut() {
            let pkt = el_protocol::packets::game::ServerboundUseItemOnPacket {
                hand: hand as i32,
                block_pos: el_protocol::packets::game::ProtoBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                direction: direction as i32,
                cursor_x,
                cursor_y,
                cursor_z,
                inside_block: false,
                world_border: false,
                sequence: sequence as i32,
            };
            let _ = local.send_packet(&pkt);
        }
        level.block_state_prediction_handler.close();
        outcome
    }
    pub fn use_item(
        &mut self,
        player: &mut LocalPlayer,
        level: &mut ClientLevel,
        hand: usize,
        remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
        sequence: u32,
        active_hotbar_slot: usize,
    ) -> UseItemOutcome {
        if self.is_spectator() {
            return UseItemOutcome::PASS;
        }
        self.ensure_has_sent_carried_item(remote_conn, &mut *local_conn, active_hotbar_slot);
        level
            .block_state_prediction_handler
            .start_predicting_with_sequence(sequence);
        let item_slot = item_slot_for_hand(hand, active_hotbar_slot);
        let item = &player.inventory[item_slot];
        let outcome = self.vanilla_use_item_outcome(item, player);
        if let Some(remote) = remote_conn {
            if outcome.consumes_action {
                println!(
                    "[MenuDebug] CLIENT_SEND_USE_ITEM hand={} seq={} yaw={:.2} pitch={:.2} carried_slot={} spectator=false",
                    hand,
                    sequence,
                    player.yaw.to_degrees(),
                    player.pitch.to_degrees(),
                    active_hotbar_slot
                );
                crate::gameplay::player::local_player::LocalPlayer::slide_probe_log(format!(
                    "[EatDebug] send_use_item hand={hand} seq={sequence} item={}",
                    player
                        .inventory
                        .get(item_slot)
                        .map(|stack| stack.item.location.to_string())
                        .unwrap_or_else(|| "<empty>".to_string())
                ));
            }
            remote.send_use_item(
                hand as u8,
                sequence,
                player.y_rot_degrees(),
                player.x_rot_degrees(),
            );
            if outcome.swing_source_client {
                Self::send_swing_for_hand(remote, hand);
            }
        } else if outcome.consumes_action {
            log::debug!(
                "Use item in hand {} for player {:?} has no local el_protocol packet",
                hand,
                player.position
            );
        }
        level.block_state_prediction_handler.close();
        outcome
    }
    fn vanilla_use_item_outcome(
        &self,
        stack: &el_core::item::ItemStack,
        player: &LocalPlayer,
    ) -> UseItemOutcome {
        if stack.is_empty() {
            return UseItemOutcome::PASS;
        }
        if stack.is_vanilla_bow_item() {
            return if self.vanilla_bow_has_projectile_like_java(player) {
                UseItemOutcome {
                    starts_using_item: true,
                    ..UseItemOutcome::CONSUME
                }
            } else {
                UseItemOutcome::PASS
            };
        }
        let path = stack.item.location.path.as_str();
        if stack.consumable_properties().is_some() {
            let can_consume = stack.food_properties().map_or(true, |food| {
                self.abilities.invulnerable || food.can_always_eat || player.food < 20
            });
            if !can_consume {
                return UseItemOutcome::PASS;
            }
            return UseItemOutcome {
                starts_using_item: stack
                    .consumable_properties()
                    .is_some_and(|consumable| consumable.consume_ticks() > 0),
                ..UseItemOutcome::CONSUME
            };
        }
        if let Some(el_core::item::DataComponent::Generic {
            value: el_nbt::Tag::Compound(fields),
            ..
        }) = stack.components.get("minecraft:equippable")
            && el_nbt::compound_get(fields, "swappable")
                .is_none_or(|value| matches!(value, el_nbt::Tag::Byte(value) if * value != 0))
        {
            return UseItemOutcome::SUCCESS;
        }
        if stack.components.get("minecraft:blocks_attacks").is_some()
            || stack.components.get("minecraft:kinetic_weapon").is_some()
        {
            return UseItemOutcome {
                starts_using_item: true,
                ..UseItemOutcome::CONSUME
            };
        }
        match path {
            "written_book" | "writable_book" | "wind_charge" | "lingering_potion"
            | "splash_potion" | "knowledge_book" | "snowball" | "experience_bottle"
            | "empty_map" | "ender_pearl" | "egg" | "fishing_rod" | "bucket" | "boat"
            | "chest_boat" | "bundle" | "white_bundle" | "orange_bundle" | "magenta_bundle"
            | "light_blue_bundle" | "yellow_bundle" | "lime_bundle" | "pink_bundle"
            | "gray_bundle" | "light_gray_bundle" | "cyan_bundle" | "purple_bundle"
            | "blue_bundle" | "brown_bundle" | "green_bundle" | "red_bundle" | "black_bundle" => {
                UseItemOutcome {
                    starts_using_item: path.ends_with("bundle"),
                    ..UseItemOutcome::SUCCESS
                }
            }
            "firework_rocket" if player.is_fall_flying => UseItemOutcome::SUCCESS,
            "trident" | "spyglass" => UseItemOutcome {
                starts_using_item: true,
                ..UseItemOutcome::CONSUME
            },
            "crossbow"
                if matches!(
                    stack.components.get("minecraft:charged_projectiles"),
                    Some(el_core::item::DataComponent::Generic { value :
                    el_nbt::Tag::List(items), .. }) if ! items.is_empty()
                ) =>
            {
                UseItemOutcome {
                    starts_using_item: false,
                    ..UseItemOutcome::CONSUME
                }
            }
            "crossbow" => {
                if self.vanilla_crossbow_has_projectile_like_java(player) {
                    UseItemOutcome {
                        starts_using_item: true,
                        ..UseItemOutcome::CONSUME
                    }
                } else {
                    UseItemOutcome::PASS
                }
            }
            "ender_eye" => UseItemOutcome {
                starts_using_item: true,
                ..UseItemOutcome::CONSUME
            },
            "goat_horn" if stack.components.get("minecraft:instrument").is_some() => {
                UseItemOutcome {
                    starts_using_item: true,
                    ..UseItemOutcome::CONSUME
                }
            }
            _ => UseItemOutcome::PASS,
        }
    }
    fn vanilla_bow_has_projectile_like_java(&self, player: &LocalPlayer) -> bool {
        player
            .inventory
            .get(OFFHAND_SLOT)
            .is_some_and(is_vanilla_bow_arrow)
            || player
                .inventory
                .iter()
                .take(INVENTORY_SIZE)
                .any(is_vanilla_bow_arrow)
            || self.abilities.instabuild
    }
    fn vanilla_crossbow_has_projectile_like_java(&self, player: &LocalPlayer) -> bool {
        player
            .inventory
            .get(OFFHAND_SLOT)
            .is_some_and(is_vanilla_crossbow_held_projectile)
            || player
                .inventory
                .iter()
                .take(INVENTORY_SIZE)
                .any(is_vanilla_bow_arrow)
            || self.abilities.instabuild
    }
    fn vanilla_use_item_can_consume(
        &self,
        stack: &el_core::item::ItemStack,
        player: &LocalPlayer,
    ) -> bool {
        self.vanilla_use_item_outcome(stack, player).consumes_action
    }
    pub fn attack(
        &mut self,
        player: &mut LocalPlayer,
        entity_id: i32,
        remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
        active_hotbar_slot: usize,
    ) {
        self.ensure_has_sent_carried_item(remote_conn, &mut *local_conn, active_hotbar_slot);
        if let Some(remote) = remote_conn {
            remote.send_attack(entity_id, player.input.key_presses.sneak);
            remote.send_swing_main_hand();
        }
        if let Some(local) = local_conn.as_mut() {
            let pkt = el_protocol::packets::game::ServerboundAttackPacket { entity_id };
            let _ = local.send_packet(&pkt);
        }
        player.reset_attack_strength_ticker();
    }
    pub fn spectate(
        &mut self,
        entity_id: i32,
        _remote_conn: Option<&PacketSender>,
        _local_conn: &mut Option<&mut LocalConnection>,
    ) {
        log::debug!("Spectate entity {}", entity_id);
    }
    pub fn spectator_no_action(
        &mut self,
        _remote_conn: Option<&PacketSender>,
        _local_conn: &mut Option<&mut LocalConnection>,
    ) {
        log::debug!("Spectator no action");
    }
    pub fn interact(
        &mut self,
        player: &mut LocalPlayer,
        entity_id: i32,
        hand: usize,
        relative_hit: glam::Vec3,
        remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
        active_hotbar_slot: usize,
    ) -> bool {
        self.ensure_has_sent_carried_item(remote_conn, &mut *local_conn, active_hotbar_slot);
        remote_conn.is_some_and(|remote| {
            remote.send_interact_entity(
                entity_id,
                hand as u8,
                relative_hit.x,
                relative_hit.y,
                relative_hit.z,
                player.input.key_presses.sneak,
            )
        })
    }
    pub fn handle_container_input(
        &mut self,
        container_id: i32,
        slot_num: i32,
        button_num: i32,
        container_input: ContainerInput,
        player: &mut LocalPlayer,
        _remote_conn: Option<&PacketSender>,
        _local_conn: &mut Option<&mut LocalConnection>,
    ) {
        log::debug!(
            "Container input for player {:?}, container_id={}, slot_num={}, button_num={}, input={:?}",
            player.position,
            container_id,
            slot_num,
            button_num,
            container_input
        );
    }
    pub fn handle_place_recipe(
        &mut self,
        container_id: i32,
        recipe_id: String,
        use_max_items: bool,
        _remote_conn: Option<&PacketSender>,
        _local_conn: &mut Option<&mut LocalConnection>,
    ) {
        log::debug!(
            "Place recipe {} in container {} (max_items={})",
            recipe_id,
            container_id,
            use_max_items
        );
    }
    pub fn handle_inventory_button_click(
        &mut self,
        container_id: i32,
        button_id: i32,
        _remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
    ) {
        if let Some(local) = local_conn.as_mut() {
            let pkt = el_protocol::packets::game::ServerboundContainerButtonClickPacket {
                container_id,
                button_id,
            };
            let _ = local.send_packet(&pkt);
        }
    }
    pub fn handle_creative_mode_item_add(
        &mut self,
        clicked: &ItemStack,
        slot: i32,
        _remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
    ) {
        if let Some(local) = local_conn.as_mut() {
            let pkt = el_protocol::packets::game::ServerboundSetCreativeModeSlotPacket {
                slot_num: slot as i16,
                item_stack: clicked.clone(),
            };
            let _ = local.send_packet(&pkt);
        }
    }
    pub fn handle_creative_mode_item_drop(
        &mut self,
        clicked: &ItemStack,
        _remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
    ) {
        if let Some(local) = local_conn.as_mut() {
            let pkt = el_protocol::packets::game::ServerboundSetCreativeModeSlotPacket {
                slot_num: -1,
                item_stack: clicked.clone(),
            };
            let _ = local.send_packet(&pkt);
        }
    }
    pub fn release_using_item(
        &mut self,
        _player: &mut LocalPlayer,
        remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
        active_hotbar_slot: usize,
    ) {
        self.ensure_has_sent_carried_item(remote_conn, &mut *local_conn, active_hotbar_slot);
        self.send_player_action(
            remote_conn,
            &mut *local_conn,
            RELEASE_USE_ITEM_ACTION,
            BlockPos::new(0, 0, 0),
            0,
            0,
        );
    }
    pub fn piercing_attack(
        &mut self,
        weapon: &ItemStack,
        remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
        active_hotbar_slot: usize,
    ) {
        self.ensure_has_sent_carried_item(remote_conn, &mut *local_conn, active_hotbar_slot);
        self.send_player_action(
            remote_conn,
            &mut *local_conn,
            STAB_ACTION,
            BlockPos::new(0, 0, 0),
            0,
            0,
        );
        log::debug!("Piercing attack with weapon {:?}", weapon.item.location);
    }
    pub fn has_experience(&self) -> bool {
        self.local_player_mode.is_survival()
    }
    pub fn has_miss_time(&self) -> bool {
        !self.local_player_mode.is_creative()
    }
    pub fn is_server_controlled_inventory(&self, _player: &LocalPlayer) -> bool {
        false
    }
    pub fn is_spectator(&self) -> bool {
        matches!(self.local_player_mode, GameType::Spectator)
    }
    pub fn get_previous_player_mode(&self) -> Option<GameType> {
        self.previous_local_player_mode
    }
    pub fn get_player_mode(&self) -> GameType {
        self.local_player_mode
    }
    pub fn is_destroying(&self) -> bool {
        self.is_destroying
    }
    pub fn get_destroy_stage(&self) -> i32 {
        if self.destroy_progress > 0.0 {
            (self.destroy_progress * 10.0) as i32
        } else {
            -1
        }
    }
    pub fn take_destroyed_blocks(&mut self) -> Vec<(BlockPos, BlockState)> {
        std::mem::take(&mut self.pending_destroyed_blocks)
    }
    pub fn take_destroy_hit_sounds(&mut self) -> Vec<(BlockPos, BlockState)> {
        std::mem::take(&mut self.pending_destroy_hit_sounds)
    }
    pub fn take_block_interaction_sounds(&mut self) -> Vec<BlockInteractionSound> {
        std::mem::take(&mut self.pending_block_interaction_sounds)
    }
    pub fn handle_pick_item_from_block(
        &mut self,
        pos: BlockPos,
        include_data: bool,
        _remote_conn: Option<&PacketSender>,
        _local_conn: &mut Option<&mut LocalConnection>,
    ) {
        log::debug!(
            "Pick item from block at {:?} (include_data={})",
            pos,
            include_data
        );
    }
    pub fn handle_pick_item_from_entity(
        &mut self,
        entity_id: i32,
        include_data: bool,
        _remote_conn: Option<&PacketSender>,
        _local_conn: &mut Option<&mut LocalConnection>,
    ) {
        log::debug!(
            "Pick item from entity {} (include_data={})",
            entity_id,
            include_data
        );
    }
    pub fn handle_slot_state_changed(
        &mut self,
        slot_id: i32,
        container_id: i32,
        new_state: bool,
        _remote_conn: Option<&PacketSender>,
        local_conn: &mut Option<&mut LocalConnection>,
    ) {
        if let Some(local) = local_conn.as_mut() {
            let pkt = el_protocol::packets::game::ServerboundContainerSlotStateChangedPacket {
                slot_id,
                container_id,
                new_state,
            };
            let _ = local.send_packet(&pkt);
        }
    }
}
pub(crate) fn is_vanilla_bow_arrow(stack: &ItemStack) -> bool {
    !stack.is_empty()
        && stack.item.location.namespace == "minecraft"
        && matches!(
            stack.item.location.path.as_str(),
            "arrow" | "tipped_arrow" | "spectral_arrow"
        )
}
pub(crate) fn is_vanilla_crossbow_held_projectile(stack: &ItemStack) -> bool {
    is_vanilla_bow_arrow(stack)
        || (!stack.is_empty()
            && stack.item.location.namespace == "minecraft"
            && stack.item.location.path == "firework_rocket")
}
fn block_use_without_item_consumes_action(state: &BlockState) -> bool {
    if el_core::chest::is_chest_block(state) {
        return true;
    }
    if state.block.namespace != "minecraft" {
        return false;
    }
    let path = state.block.path.as_str();
    if path == "respawn_anchor" {
        return state
            .get_property("charges")
            .and_then(|value| value.parse::<u8>().ok())
            .is_some_and(|charges| charges > 0);
    }
    if path == "composter" {
        return state
            .get_property("level")
            .is_some_and(|value| value == "8");
    }
    (placement::in_tag(state, "minecraft:doors") && path != "iron_door")
        || (placement::in_tag(state, "minecraft:trapdoors") && path != "iron_trapdoor")
        || placement::in_tag(state, "minecraft:fence_gates")
        || placement::in_tag(state, "minecraft:buttons")
        || matches!(
            path,
            "lever"
                | "repeater"
                | "comparator"
                | "daylight_detector"
                | "note_block"
                | "barrel"
                | "crafting_table"
                | "furnace"
                | "blast_furnace"
                | "smoker"
                | "brewing_stand"
                | "dispenser"
                | "dropper"
                | "hopper"
                | "beacon"
                | "enchanting_table"
                | "lectern"
                | "grindstone"
                | "smithing_table"
                | "stonecutter"
                | "cartography_table"
                | "loom"
                | "decorated_pot"
                | "bell"
                | "chiseled_bookshelf"
                | "crafter"
                | "anvil"
                | "chipped_anvil"
                | "damaged_anvil"
        )
        || path.ends_with("shulker_box")
}
fn block_interaction_consumes_action(
    state: &BlockState,
    level: &ClientLevel,
    hit: &crate::gameplay::raycast::HitResult,
    hand: usize,
    held_item: bool,
    may_build: bool,
) -> bool {
    if state.block.namespace != "minecraft" {
        return false;
    }
    let path = state.block.path.as_str();
    let has_block_entity = || {
        level
            .chunks
            .get(&ChunkPos::from_block(hit.pos))
            .is_some_and(|chunk| chunk.block_entities.contains_key(&hit.pos))
    };
    if path.ends_with("_shelf") && state.get_property("side_chain_part").is_some() {
        return hand == MAIN_HAND
            && held_item
            && has_block_entity()
            && state
                .get_property("facing")
                .is_some_and(|facing| facing == hit.face.name());
    }
    if hand != MAIN_HAND {
        return false;
    }
    match path {
        "repeater" | "comparator" | "daylight_detector" if !may_build => false,
        "decorated_pot" => has_block_entity(),
        "chiseled_bookshelf" => {
            has_block_entity()
                && state
                    .get_property("facing")
                    .is_some_and(|facing| facing == hit.face.name())
        }
        "bell" => {
            let Some(facing) = state
                .get_property("facing")
                .and_then(|v| Direction::from_name(v))
            else {
                return false;
            };
            hit.face.axis().is_horizontal()
                && f64::from(hit.hit_pos.y) - f64::from(hit.pos.y) <= f64::from(0.8124_f32)
                && match state.get_property("attachment").map(String::as_str) {
                    Some("floor") => facing.axis() == hit.face.axis(),
                    Some("single_wall" | "double_wall") => facing.axis() != hit.face.axis(),
                    Some("ceiling") => true,
                    _ => false,
                }
        }
        _ => block_use_without_item_consumes_action(state),
    }
}
fn block_interaction_outcome(
    state: &BlockState,
    level: &ClientLevel,
    hit: &crate::gameplay::raycast::HitResult,
) -> UseItemOnOutcome {
    let no_swing = match state.block.path.as_str() {
        "respawn_anchor" => true,
        "lectern" => state
            .get_property("has_book")
            .is_none_or(|value| value != "true"),
        "chiseled_bookshelf" => placement::selectable_slot(state, hit, 2, 3).is_some_and(|slot| {
            state
                .get_property(&format!("slot_{slot}_occupied"))
                .is_none_or(|value| value != "true")
        }),
        _ if placement::in_tag(state, "minecraft:buttons") => state
            .get_property("powered")
            .is_some_and(|value| value == "true"),
        _ => false,
    };
    let _ = level;
    if no_swing {
        UseItemOnOutcome::CONSUME
    } else {
        UseItemOnOutcome::SUCCESS
    }
}
fn predict_block_write(
    level: &mut ClientLevel,
    pos: BlockPos,
    state: &BlockState,
    player: &LocalPlayer,
    sequence: u32,
) -> bool {
    let old_state = level.get_block_state(pos);
    if !level.set_block(pos, state.clone()) {
        return false;
    }
    let player_pos = player.exact_position();
    level
        .block_state_prediction_handler
        .retain_known_server_state_at_sequence(pos, old_state.clone(), player_pos, sequence);
    for (neighbor, old_neighbor) in level.take_neighbor_prediction_old_states() {
        level
            .block_state_prediction_handler
            .retain_known_server_state_at_sequence(neighbor, old_neighbor, player_pos, sequence);
    }
    if let Some(chunk) = level.chunks.get_mut(&ChunkPos::from_block(pos)) {
        crate::client::state::sync_special_block_entity(
            std::sync::Arc::make_mut(chunk),
            pos,
            &old_state,
            state,
        );
    }
    true
}
fn canonicalize_block_state(state: BlockState) -> BlockState {
    BuiltInRegistries::get_id_by_state(state.block.as_ref(), &state.properties)
        .and_then(BuiltInRegistries::get_state_by_id)
        .cloned()
        .unwrap_or(state)
}
