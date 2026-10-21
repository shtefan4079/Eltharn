use super::mesh_revision;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshResultMeta<K: Copy> {
    pub key: K,
    pub generation: u64,
    pub revision: u64,
    pub urgent: bool,
    pub cancelled: bool,
    pub index_count: usize,
    pub has_geometry: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshCompletion<K: Copy> {
    pub generation: u64,
    pub revision: u64,
    pub key: K,
    pub urgent: bool,
}
#[derive(Debug)]
pub struct LaneIntake<R> {
    pub committed: Vec<R>,
    pub deferred: Option<R>,
}
pub fn collect_lane_commits<K: Copy, R>(
    deferred: Option<R>,
    mut next: impl FnMut() -> Option<R>,
    meta: impl Fn(&R) -> MeshResultMeta<K>,
    mut will_upload: impl FnMut(&R, &MeshResultMeta<K>) -> bool,
    mut should_yield_for_time: impl FnMut() -> bool,
    mut try_take_budget: impl FnMut(bool, usize) -> bool,
) -> LaneIntake<R> {
    let mut committed = Vec::new();
    let mut deferred_out: Option<R> = None;
    let mut current = deferred.or_else(|| next());
    while let Some(result) = current.take() {
        let meta = meta(&result);
        let upload = will_upload(&result, &meta);
        if upload && should_yield_for_time() {
            deferred_out = Some(result);
            break;
        }
        if !try_take_budget(upload, meta.index_count) {
            deferred_out = Some(result);
            break;
        }
        committed.push(result);
        current = next();
    }
    LaneIntake {
        committed,
        deferred: deferred_out,
    }
}
pub fn drain_completions<K: Copy>(
    completions: impl Iterator<Item = MeshCompletion<K>>,
    active_generation: u64,
    mut on_urgent_result: impl FnMut(u64),
    mut confirm_completion: impl FnMut(K, u64, bool),
) {
    for completion in completions {
        if !mesh_revision::is_generation_current(active_generation, completion.generation) {
            continue;
        }
        if completion.urgent {
            on_urgent_result(completion.generation);
        }
        confirm_completion(completion.key, completion.revision, completion.urgent);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitVerdict {
    Upload,
    RetireCancelled,
    DropStale,
    EvictUnloaded,
    RetireEmpty,
}
pub fn classify_commit<K: Copy>(
    active_generation: u64,
    current_revision: Option<u64>,
    chunk_loaded: bool,
    result: &MeshResultMeta<K>,
) -> CommitVerdict {
    if result.cancelled {
        return CommitVerdict::RetireCancelled;
    }
    if !mesh_revision::is_result_fresh(
        active_generation,
        current_revision,
        result.generation,
        result.revision,
    ) {
        return CommitVerdict::DropStale;
    }
    if !chunk_loaded {
        return CommitVerdict::EvictUnloaded;
    }
    if !result.has_geometry {
        return CommitVerdict::RetireEmpty;
    }
    CommitVerdict::Upload
}
pub fn result_will_upload<K: Copy>(
    active_generation: u64,
    current_revision: Option<u64>,
    chunk_loaded: bool,
    result: &MeshResultMeta<K>,
) -> bool {
    !result.cancelled
        && mesh_revision::is_result_fresh(
            active_generation,
            current_revision,
            result.generation,
            result.revision,
        )
        && chunk_loaded
        && result.has_geometry
}
pub trait MeshCommitSink<K: Copy, R> {
    fn on_urgent_result(&mut self, generation: u64);
    fn confirm_completion(&mut self, key: K, revision: u64, urgent: bool);
    fn current_revision(&self, key: K) -> Option<u64>;
    fn chunk_loaded(&self, key: K) -> bool;
    fn commit_one(&mut self, result: R, meta: MeshResultMeta<K>, verdict: CommitVerdict);
}
pub fn drive_mesh_commits<K: Copy, R, S: MeshCommitSink<K, R>>(
    sink: &mut S,
    results: Vec<R>,
    meta: impl Fn(&R) -> MeshResultMeta<K>,
    active_generation: u64,
) {
    for result in results {
        let result_meta = meta(&result);
        if result_meta.urgent {
            sink.on_urgent_result(result_meta.generation);
        }
        sink.confirm_completion(result_meta.key, result_meta.revision, result_meta.urgent);
        let revision = sink.current_revision(result_meta.key);
        let loaded = sink.chunk_loaded(result_meta.key);
        let verdict = classify_commit(active_generation, revision, loaded, &result_meta);
        sink.commit_one(result, result_meta, verdict);
    }
}
