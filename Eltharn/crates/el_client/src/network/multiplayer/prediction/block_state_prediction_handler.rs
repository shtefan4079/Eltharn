use crate::world::client_level::ClientLevel;
use el_core::block_state::BlockState;
use el_math::block_pos::BlockPos;
use glam::DVec3;
use std::collections::HashMap;
#[derive(Debug, Clone)]
pub struct ServerVerifiedState {
    pub sequence: u32,
    pub block_state: BlockState,
    pub player_pos: DVec3,
}
#[derive(Debug, Clone)]
pub struct ResolvedServerVerifiedState {
    pub pos: BlockPos,
    pub sequence: u32,
    pub block_state: BlockState,
    pub player_pos: DVec3,
}
pub struct BlockStatePredictionHandler {
    pub server_verified_states: HashMap<BlockPos, ServerVerifiedState>,
    pub current_sequence_nr: u32,
    pub is_predicting: bool,
}
impl Default for BlockStatePredictionHandler {
    fn default() -> Self {
        Self::new()
    }
}
impl BlockStatePredictionHandler {
    pub fn new() -> Self {
        Self {
            server_verified_states: HashMap::new(),
            current_sequence_nr: 0,
            is_predicting: false,
        }
    }
    pub fn retain_known_server_state(
        &mut self,
        pos: BlockPos,
        state: BlockState,
        player_pos: DVec3,
    ) {
        self.retain_known_server_state_at_sequence(
            pos,
            state,
            player_pos,
            self.current_sequence_nr,
        );
    }
    pub fn retain_known_server_state_at_sequence(
        &mut self,
        pos: BlockPos,
        state: BlockState,
        player_pos: DVec3,
        sequence: u32,
    ) {
        self.server_verified_states
            .entry(pos)
            .and_modify(|s| s.sequence = sequence)
            .or_insert(ServerVerifiedState {
                sequence,
                block_state: state,
                player_pos,
            });
    }
    pub fn update_known_server_state(&mut self, pos: BlockPos, block_state: BlockState) -> bool {
        if let Some(state) = self.server_verified_states.get_mut(&pos) {
            state.block_state = block_state;
            true
        } else {
            false
        }
    }
    pub fn take_predictions_up_to(&mut self, sequence: u32) -> Vec<ResolvedServerVerifiedState> {
        let mut to_remove = Vec::new();
        for (pos, state) in &self.server_verified_states {
            if state.sequence <= sequence {
                to_remove.push(*pos);
            }
        }
        let mut resolved = Vec::with_capacity(to_remove.len());
        for pos in to_remove {
            if let Some(state) = self.server_verified_states.remove(&pos) {
                resolved.push(ResolvedServerVerifiedState {
                    pos,
                    sequence: state.sequence,
                    block_state: state.block_state,
                    player_pos: state.player_pos,
                });
            }
        }
        resolved
    }
    pub fn end_predictions_up_to(&mut self, sequence: u32, client_level: &mut ClientLevel) {
        for state in self.take_predictions_up_to(sequence) {
            let client_block_state = client_level.get_block_state(state.pos);
            if client_block_state != state.block_state {
                client_level.set_block_server_verified(state.pos, state.block_state);
            }
        }
    }
    pub fn start_predicting(&mut self) -> u32 {
        self.current_sequence_nr = self.current_sequence_nr.wrapping_add(1);
        self.is_predicting = true;
        self.current_sequence_nr
    }
    pub fn start_predicting_with_sequence(&mut self, sequence: u32) -> u32 {
        self.current_sequence_nr = sequence;
        self.is_predicting = true;
        sequence
    }
    pub fn close(&mut self) {
        self.is_predicting = false;
    }
    pub fn current_sequence(&self) -> u32 {
        self.current_sequence_nr
    }
    pub fn is_predicting(&self) -> bool {
        self.is_predicting
    }
    pub fn prediction_sequence(&self, pos: BlockPos) -> Option<u32> {
        self.server_verified_states
            .get(&pos)
            .map(|state| state.sequence)
    }
    pub fn clear(&mut self) {
        self.server_verified_states.clear();
        self.is_predicting = false;
    }
    pub fn retain_positions(&mut self, mut retain: impl FnMut(BlockPos) -> bool) {
        self.server_verified_states.retain(|pos, _| retain(*pos));
    }
}
