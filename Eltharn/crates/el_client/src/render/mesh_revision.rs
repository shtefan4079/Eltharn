#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerSendVerdict {
    Send,
    CompleteLightlyAndDrop,
    Drop,
}
pub fn is_generation_current(active_generation: u64, candidate_generation: u64) -> bool {
    active_generation == candidate_generation
}
pub fn is_revision_current(current_revision: Option<u64>, candidate_revision: u64) -> bool {
    current_revision == Some(candidate_revision)
}
pub fn is_result_fresh(
    active_generation: u64,
    current_revision: Option<u64>,
    result_generation: u64,
    result_revision: u64,
) -> bool {
    is_generation_current(active_generation, result_generation)
        && is_revision_current(current_revision, result_revision)
}
pub fn is_superseded(pending_revision: u64, current_revision: u64) -> bool {
    pending_revision != current_revision
}
pub fn worker_send_verdict(
    generation_fence: u64,
    compile_revision: u64,
    result_generation: u64,
    result_revision: u64,
) -> WorkerSendVerdict {
    if generation_fence != result_generation {
        return WorkerSendVerdict::Drop;
    }
    if is_superseded(result_revision, compile_revision) {
        return WorkerSendVerdict::CompleteLightlyAndDrop;
    }
    WorkerSendVerdict::Send
}
pub fn should_count_urgent_result(active_generation: u64, result_generation: u64) -> bool {
    active_generation == result_generation
}
