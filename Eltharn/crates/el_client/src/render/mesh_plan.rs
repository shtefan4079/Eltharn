pub const MAX_RECOMPILE_QUOTA: usize = 2;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanClass {
    Urgent,
    Initial,
    Recompile,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanCandidate<K: Copy> {
    pub key: K,
    pub queue_index: usize,
    pub distance_sq: f32,
    pub class: PlanClass,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanLimits {
    pub limit: usize,
    pub urgent_limit: usize,
    pub normal_limit: usize,
}
pub fn select_mesh_batch_like_java<K: Copy>(
    candidates: Vec<PlanCandidate<K>>,
    urgent_only: bool,
    limits: PlanLimits,
    recompile_quota: &mut usize,
) -> Vec<K> {
    if limits.limit == 0 {
        return Vec::new();
    }
    let mut urgent = Vec::new();
    let mut initial = Vec::new();
    let mut recompile = Vec::new();
    for candidate in candidates {
        match candidate.class {
            PlanClass::Urgent => urgent.push(candidate),
            PlanClass::Initial => initial.push(candidate),
            PlanClass::Recompile => recompile.push(candidate),
        }
    }
    let nearest_first = |left: &PlanCandidate<K>, right: &PlanCandidate<K>| {
        left.distance_sq
            .total_cmp(&right.distance_sq)
            .then_with(|| left.queue_index.cmp(&right.queue_index))
    };
    let retain_nearest = |items: &mut Vec<PlanCandidate<K>>, keep: usize| {
        if keep == 0 {
            items.clear();
            return;
        }
        if items.len() > keep {
            items.select_nth_unstable_by(keep - 1, nearest_first);
            items.truncate(keep);
        }
        items.sort_unstable_by(nearest_first);
    };
    retain_nearest(&mut urgent, limits.limit.min(limits.urgent_limit));
    if !urgent_only && limits.normal_limit > 0 {
        retain_nearest(&mut initial, limits.normal_limit.min(limits.limit));
        retain_nearest(&mut recompile, limits.normal_limit.min(limits.limit));
    } else {
        initial.clear();
        recompile.clear();
    }
    let mut selected = urgent
        .into_iter()
        .take(limits.limit.min(limits.urgent_limit))
        .map(|candidate| candidate.key)
        .collect::<Vec<_>>();
    if urgent_only || selected.len() == limits.limit || limits.normal_limit == 0 {
        return selected;
    }
    let normal_budget = limits.normal_limit.min(limits.limit - selected.len());
    let mut selected_normal = 0usize;
    let mut initial_index = 0usize;
    let mut recompile_index = 0usize;
    while selected_normal < normal_budget {
        let next_initial = initial.get(initial_index);
        let next_recompile = recompile.get(recompile_index);
        let choose_initial = next_recompile.is_none()
            || next_initial.is_some_and(|initial| {
                *recompile_quota == 0
                    || next_recompile
                        .is_some_and(|recompile| recompile.distance_sq >= initial.distance_sq)
            });
        let selected_key = if choose_initial {
            let Some(candidate) = next_initial else {
                break;
            };
            initial_index += 1;
            *recompile_quota = MAX_RECOMPILE_QUOTA;
            candidate.key
        } else {
            let Some(candidate) = next_recompile else {
                break;
            };
            recompile_index += 1;
            *recompile_quota = recompile_quota.saturating_sub(1);
            candidate.key
        };
        selected.push(selected_key);
        selected_normal += 1;
    }
    selected
}
