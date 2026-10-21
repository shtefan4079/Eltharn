use super::ClientSession;
use crate::gameplay::player::correction;
use el_core::block_state::BlockState;
use el_math::block_pos::BlockPos;
use glam::DVec3;
pub struct PredictedLocalBlockOutcome {
    pub level_changed: bool,
    pub previous_state: Option<BlockState>,
    pub chest_updates: Vec<(BlockPos, BlockState)>,
}
pub(crate) fn apply_predicted_local_block(
    session: &mut ClientSession,
    pos: BlockPos,
    target: &BlockState,
) -> PredictedLocalBlockOutcome {
    let previous_state = session
        .level
        .as_ref()
        .map(|level| level.get_block_state(pos));
    let Some(previous) = previous_state.clone() else {
        return PredictedLocalBlockOutcome {
            level_changed: false,
            previous_state: None,
            chest_updates: Vec::new(),
        };
    };
    if previous == *target {
        return PredictedLocalBlockOutcome {
            level_changed: false,
            previous_state: Some(previous),
            chest_updates: Vec::new(),
        };
    }
    if session.remote_connection.is_some()
        && let Some(level) = session.level.as_mut()
    {
        let sequence = session.remote_action_sequence;
        let player_pos = session.player.exact_position();
        level
            .block_state_prediction_handler
            .start_predicting_with_sequence(sequence);
        level
            .block_state_prediction_handler
            .retain_known_server_state_at_sequence(pos, previous.clone(), player_pos, sequence);
        level.block_state_prediction_handler.close();
    }
    let mut level_changed = false;
    let mut chest_updates = Vec::new();
    if let Some(level) = session.level.as_mut() {
        level_changed = level.set_block(pos, target.clone());
        chest_updates = level.take_chest_neighbor_shape_updates();
    }
    PredictedLocalBlockOutcome {
        level_changed,
        previous_state: Some(previous),
        chest_updates,
    }
}
pub struct BlockAckCorrection {
    pub pos: BlockPos,
    pub server_state: BlockState,
    pub prediction_sequence: u32,
    pub retained_player_pos: DVec3,
    pub previous_state: BlockState,
    pub corrected: bool,
    pub snapped: bool,
    pub chest_updates: Vec<(BlockPos, BlockState)>,
}
pub(crate) fn take_block_predictions(
    session: &mut ClientSession,
    sequence: u32,
) -> Vec<
    crate::network::multiplayer::prediction::block_state_prediction_handler::ResolvedServerVerifiedState,
>{
    session
        .level
        .as_mut()
        .map(|level| {
            level
                .block_state_prediction_handler
                .take_predictions_up_to(sequence)
        })
        .unwrap_or_default()
}
pub(crate) fn reconcile_taken_prediction(
    session: &mut ClientSession,
    taken: &crate::network::multiplayer::prediction::block_state_prediction_handler::ResolvedServerVerifiedState,
) -> BlockAckCorrection {
    let current = session
        .level
        .as_ref()
        .map(|level| level.get_block_state(taken.pos));
    let Some(previous_state) = current else {
        return BlockAckCorrection {
            pos: taken.pos,
            server_state: taken.block_state.clone(),
            prediction_sequence: taken.sequence,
            retained_player_pos: taken.player_pos,
            previous_state: taken.block_state.clone(),
            corrected: false,
            snapped: false,
            chest_updates: Vec::new(),
        };
    };
    if previous_state == taken.block_state {
        return BlockAckCorrection {
            pos: taken.pos,
            server_state: taken.block_state.clone(),
            prediction_sequence: taken.sequence,
            retained_player_pos: taken.player_pos,
            previous_state,
            corrected: false,
            snapped: false,
            chest_updates: Vec::new(),
        };
    }
    let mut snapped = false;
    let mut chest_updates = Vec::new();
    if let Some(level) = session.level.as_mut() {
        snapped = correction::reconcile_predicted_block(
            level,
            &mut session.player,
            taken.pos,
            &taken.block_state,
            taken.player_pos,
        );
        chest_updates = level.take_chest_neighbor_shape_updates();
    }
    BlockAckCorrection {
        pos: taken.pos,
        server_state: taken.block_state.clone(),
        prediction_sequence: taken.sequence,
        retained_player_pos: taken.player_pos,
        previous_state,
        corrected: true,
        snapped,
        chest_updates,
    }
}
pub(crate) fn resolve_block_ack(
    session: &mut ClientSession,
    sequence: u32,
) -> Vec<BlockAckCorrection> {
    let taken = take_block_predictions(session, sequence);
    taken
        .iter()
        .map(|prediction| reconcile_taken_prediction(session, prediction))
        .collect()
}
pub(crate) fn drain_level_chest_updates(
    session: &mut ClientSession,
) -> Vec<(BlockPos, BlockState)> {
    session
        .level
        .as_mut()
        .map(|level| level.take_chest_neighbor_shape_updates())
        .unwrap_or_default()
}
