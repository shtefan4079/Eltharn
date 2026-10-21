use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::hash::Hash;
#[derive(Debug)]
pub struct RemoteMeshQueue<K: Eq + Hash + Copy + Debug> {
    queue: VecDeque<K>,
    queued: HashSet<K>,
    urgent: HashSet<K>,
    pending: HashMap<K, u64>,
    pending_urgent: HashMap<K, u64>,
    revisions: HashMap<K, u64>,
    next_revision: u64,
}
impl<K: Eq + Hash + Copy + Debug> RemoteMeshQueue<K> {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            queued: HashSet::new(),
            urgent: HashSet::new(),
            pending: HashMap::new(),
            pending_urgent: HashMap::new(),
            revisions: HashMap::new(),
            next_revision: 0,
        }
    }
    pub fn queue(&mut self, key: K, front: bool) -> Option<u64> {
        self.next_revision = self.next_revision.wrapping_add(1);
        let revision = self.next_revision;
        self.revisions.insert(key, revision);
        if self.queued.contains(&key) {
            if front {
                self.urgent.insert(key);
                if let Some(index) = self.queue.iter().position(|queued| *queued == key) {
                    self.queue.remove(index);
                    self.queue.push_front(key);
                }
            }
            return Some(revision);
        }
        self.queued.insert(key);
        if front {
            self.urgent.insert(key);
            self.queue.push_front(key);
        } else {
            self.queue.push_back(key);
        }
        Some(revision)
    }
    pub fn requeue(&mut self, key: K, front: bool) {
        self.queued.insert(key);
        if front {
            self.queue.push_front(key);
        } else {
            self.queue.push_back(key);
        }
    }
    pub fn pop(&mut self) -> Option<K> {
        let key = self.queue.pop_front()?;
        self.queued.remove(&key);
        Some(key)
    }
    pub fn clear(&mut self) {
        self.queue.clear();
        self.queued.clear();
        self.urgent.clear();
        self.pending.clear();
        self.pending_urgent.clear();
        self.revisions.clear();
    }
    pub fn len(&self) -> usize {
        self.queue.len()
    }
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = K> + '_ {
        self.queue.iter().copied()
    }
    pub fn is_urgent(&self, key: K) -> bool {
        self.urgent.contains(&key)
    }
    pub fn clear_urgent(&mut self, key: K) {
        self.urgent.remove(&key);
    }
    pub fn is_pending_in_lane(&self, key: K, urgent: bool) -> bool {
        if urgent {
            self.pending_urgent.contains_key(&key)
        } else {
            self.pending.contains_key(&key)
        }
    }
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
    pub fn urgent_len(&self) -> usize {
        self.urgent.len()
    }
    pub fn revision(&self, key: K) -> Option<u64> {
        self.revisions.get(&key).copied()
    }
    pub fn reorder_front(&mut self, selected: &[K]) {
        if selected.is_empty() {
            return;
        }
        let selected_keys: HashSet<K> = selected.iter().copied().collect();
        let mut reordered = VecDeque::with_capacity(self.queue.len());
        reordered.extend(selected.iter().copied());
        reordered.extend(
            self.queue
                .drain(..)
                .filter(|key| !selected_keys.contains(key)),
        );
        self.queue = reordered;
    }
    pub fn requeue_all(&mut self, keys: impl Iterator<Item = K>) {
        self.queue.clear();
        self.queued.clear();
        for key in keys {
            self.queue.push_back(key);
            self.queued.insert(key);
        }
        self.next_revision = self.next_revision.wrapping_add(1);
    }
    pub fn retain_all(&mut self, mut keep: impl FnMut(K) -> bool) {
        self.queue.retain(|key| keep(*key));
        self.queued.retain(|key| keep(*key));
        self.urgent.retain(|key| keep(*key));
        self.pending.retain(|key, _| keep(*key));
        self.pending_urgent.retain(|key, _| keep(*key));
        self.revisions.retain(|key, _| keep(*key));
    }
    pub fn retain_queue(&mut self, mut keep: impl FnMut(K) -> bool) {
        self.queue.retain(|key| keep(*key));
        self.queued.retain(|key| keep(*key));
        self.urgent.retain(|key| keep(*key));
        self.pending.retain(|key, _| keep(*key));
        self.pending_urgent.retain(|key, _| keep(*key));
    }
    pub fn set_pending(&mut self, key: K, revision: u64, urgent: bool) {
        if urgent {
            self.pending_urgent.insert(key, revision);
        } else {
            self.pending.insert(key, revision);
        }
    }
    pub fn clear_pending(&mut self, key: K, urgent: bool) {
        if urgent {
            self.pending_urgent.remove(&key);
        } else {
            self.pending.remove(&key);
        }
    }
    pub fn in_flight_len(&self) -> usize {
        self.pending.len() + self.pending_urgent.len()
    }
    pub fn drop_pending_and_urgent(&mut self) {
        self.pending.clear();
        self.pending_urgent.clear();
        self.urgent.clear();
    }
    pub fn bump_revision_counter(&mut self) {
        self.next_revision = self.next_revision.wrapping_add(1);
    }
    pub fn confirm_completion(&mut self, key: K, revision: u64, urgent: bool) -> bool {
        let lane = if urgent {
            &mut self.pending_urgent
        } else {
            &mut self.pending
        };
        if lane.get(&key).copied() != Some(revision) {
            return false;
        }
        lane.remove(&key);
        true
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshBatchClass {
    Urgent,
    Initial,
    Recompile,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshBatchCandidate<K: Copy> {
    pub key: K,
    pub queue_index: usize,
    pub distance_sq: f32,
    pub class: MeshBatchClass,
}
pub fn collect_mesh_batch_candidates<K: Eq + Hash + Copy + Debug>(
    queue: &RemoteMeshQueue<K>,
    urgent_only: bool,
    is_within_distance: impl Fn(K) -> bool,
    distance_sq: impl Fn(K) -> f32,
    is_pending_normal: impl Fn(K) -> bool,
    is_compiled_or_meshed: impl Fn(K) -> bool,
) -> Vec<MeshBatchCandidate<K>> {
    let mut candidates = Vec::with_capacity(queue.len());
    for (queue_index, key) in queue.iter().enumerate() {
        if !is_within_distance(key) {
            continue;
        }
        let distance = distance_sq(key);
        let class = if queue.is_urgent(key) {
            MeshBatchClass::Urgent
        } else {
            if urgent_only || is_pending_normal(key) {
                continue;
            }
            if is_compiled_or_meshed(key) {
                MeshBatchClass::Recompile
            } else {
                MeshBatchClass::Initial
            }
        };
        candidates.push(MeshBatchCandidate {
            key,
            queue_index,
            distance_sq: distance,
            class,
        });
    }
    candidates
}
use super::mesh_plan;
use super::mesh_result;
use super::mesh_upload;
#[derive(Debug)]
pub struct MeshPlannedBatch<K: Copy> {
    pub ordered_keys: Vec<K>,
}
#[derive(Debug)]
pub struct MeshCommitBatch<R> {
    pub commits: Vec<R>,
    pub deferred_urgent: Option<R>,
    pub deferred_normal: Option<R>,
}
pub fn plan_mesh_batch<K: Eq + Hash + Copy + Debug>(
    candidates: Vec<mesh_plan::PlanCandidate<K>>,
    urgent_only: bool,
    limit: usize,
    urgent_limit: usize,
    normal_limit: usize,
    recompile_quota: &mut usize,
) -> MeshPlannedBatch<K> {
    MeshPlannedBatch {
        ordered_keys: mesh_plan::select_mesh_batch_like_java(
            candidates,
            urgent_only,
            mesh_plan::PlanLimits {
                limit,
                urgent_limit,
                normal_limit,
            },
            recompile_quota,
        ),
    }
}
pub fn collect_mesh_commit_batch<K: Copy, R>(
    deferred_urgent: Option<R>,
    deferred_normal: Option<R>,
    next_urgent: impl FnMut() -> Option<R>,
    next_normal: impl FnMut() -> Option<R>,
    meta: impl Fn(&R) -> mesh_result::MeshResultMeta<K>,
    mut will_upload: impl FnMut(&R, &mesh_result::MeshResultMeta<K>) -> bool,
    mut should_yield_for_time: impl FnMut() -> bool,
    budget: &mut mesh_upload::UploadBudget,
) -> MeshCommitBatch<R> {
    let urgent = mesh_result::collect_lane_commits(
        deferred_urgent,
        next_urgent,
        &meta,
        &mut will_upload,
        &mut should_yield_for_time,
        |upload, index_count| budget.try_take(mesh_upload::UploadLane::Urgent, upload, index_count),
    );
    let normal = mesh_result::collect_lane_commits(
        deferred_normal,
        next_normal,
        meta,
        will_upload,
        should_yield_for_time,
        |upload, index_count| budget.try_take(mesh_upload::UploadLane::Normal, upload, index_count),
    );
    let mut commits = urgent.committed;
    commits.extend(normal.committed);
    MeshCommitBatch {
        commits,
        deferred_urgent: urgent.deferred,
        deferred_normal: normal.deferred,
    }
}
pub fn drain_mesh_completions<K: Copy>(
    completions: impl Iterator<Item = mesh_result::MeshCompletion<K>>,
    active_generation: u64,
    on_urgent_result: impl FnMut(u64),
    confirm_completion: impl FnMut(K, u64, bool),
) {
    mesh_result::drain_completions(
        completions,
        active_generation,
        on_urgent_result,
        confirm_completion,
    );
}
